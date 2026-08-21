// SPDX-License-Identifier: MIT
//
// In-process JIT runtime: owns the guest CpuState, compiles a guest code
// buffer (a contiguous run of AArch64 instructions starting at a known
// address) into host x86-64 in an executable mapping, and executes it.
//
// Execution convention: the translated entry takes a pointer to CpuState.
// The prologue loads it into RBX (the base the translator reads/writes).

use std::ptr;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

use crate::decode::{self, Inst};
use crate::translate;
use crate::x86::{CodeBuf, RAX, RBX};

/// Guest CPU register file, spilled to memory. Layout matches translate.rs
/// slot(): x[i] at byte offset 8*i, so `x` must be the first field.
#[repr(C)]
#[derive(Clone)]
pub struct CpuState {
    pub x: [u64; 32],
    pub pc: u64,
    pub nzcv: u32,
    pub pad: u32,
    /// 32 SIMD/NEON 128-bit vector registers. Each 128-bit vector v[i] is
    /// stored as two little-endian u64 lanes: lane0 = low u64 at [bb*base +
    /// 16*i], lane1 = high u64 at [.. + 16*i + 8].
    pub v: [u64; 64],
    /// Placeholder for the AArch64 EL0 thread-pointer / TLS base (tpidr_el0).
    /// A JIT-emulated `mrs xN, tpidr_el0` / `msr tpidr_el0, xN` reads/writes this
    /// slot. Kept *after* `v` so VECTOR_BASE (272) is unchanged.
    pub tpidr: u64,
    /// Monotonic readout backing `mrs xN, cntvct_el0` / `cntpct_el0`. The host
    /// stamps this immediately before each executed guest block (see run_loop)
    /// with elapsed-since-boot scaled to the declared counter frequency
    /// (CNTFRQ_EL0 = 100 MHz). Reads by the guest see time advance between
    /// blocks so cnt-delta arithmetic is monotonic and self-consistent.
    pub cntvct: u64,
}

/// Base byte offset of the SIMD vector register file inside CpuState.
///
/// Layout of `CpuState` (repr(C)): x[32] at 0..256, pc at 256..264, nzcv at
/// 264..268, pad at 268..272, then v[64] at 272... (must not overlap pc!).
pub const VECTOR_BASE: i32 = 272;

/// Byte offset of `CpuState.pc` (after the 32 x-regs).
pub const PC_OFF: i32 = 8 * 32; // 256

/// Byte offset of `CpuState.tpidr` — right after the 64-null v array (v[64] at
/// VECTOR_BASE 272 .. 272+512=784). 272 + 64*8 = 784.
pub const TPIDR_OFF: i32 = VECTOR_BASE + 64 * 8; // 784
/// Byte offset of `CpuState.cntvct` — right after `tpidr` (784..792).
pub const CNTVCT_OFF: i32 = TPIDR_OFF + 8; // 792

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            x: [0; 32],
            pc: 0,
            nzcv: 0,
            pad: 0,
            v: [0; 64],
            tpidr: 0,
            cntvct: 0,
        }
    }
    pub fn set(&mut self, reg: usize, val: u64) {
        self.x[reg] = val;
    }
    pub fn get(&self, reg: usize) -> u64 {
        self.x[reg]
    }
    pub fn set_v(&mut self, vreg: usize, low: u64, high: u64) {
        self.v[vreg * 2] = low;
        self.v[vreg * 2 + 1] = high;
    }
    pub fn get_v(&self, vreg: usize) -> (u64, u64) {
        (self.v[vreg * 2], self.v[vreg * 2 + 1])
    }
}

/// An executable block produced by translating a run of guest instructions.
pub struct JitBlock {
    ptr: *mut u8,
    len: usize,
}

impl JitBlock {
    /// Bytes of the emitted x86-64 machine code (for inspection/dumping).
    pub fn dump(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
    pub fn len(&self) -> usize {
        self.len
    }
}

unsafe impl Send for JitBlock {}
unsafe impl Sync for JitBlock {}

impl Drop for JitBlock {
    fn drop(&mut self) {
        // munmap the executable region
        let _ = unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
    }
}

/// Allocate a fresh RWX page and copy `code` into it. Returns the base.
fn map_exec(code: &[u8]) -> *mut u8 {
    let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let n = (code.len() + ps - 1) / ps * ps;
    unsafe {
        let base = libc::mmap(
            ptr::null_mut(),
            n,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if base == libc::MAP_FAILED {
            panic!("mmap exec failed");
        }
        ptr::copy_nonoverlapping(code.as_ptr(), base as *mut u8, code.len());
        base as *mut u8
    }
}

/// Compile a translation of `insts` (already decoded) for a state at
/// `state_addr`, return an executable JitBlock whose entry is a C function
/// `fn(*mut CpuState)`. Guest instructions are assumed to be packed at 4 bytes
/// each starting at a base of 0; branch targets are resolved to the host
/// offset of the corresponding instruction's translation.
pub fn compile(insts: &[Inst], state: *mut CpuState) -> Result<JitBlock, String> {
    // guest offset of each inst (index*4) -> host buffer offset where its
    // translation starts (covers the epilogue marker at the end).
    // prologue emits first; map is relative to buffer start (address 0).
    let mut buf = CodeBuf::new();
    let mut fixups: Vec<crate::translate::Fixup> = Vec::new();

    // prologue: RBX = state
    buf.mov_ri64(RBX, state as usize as u64);

    // translate each instruction at its guest offset, recording offsets.
    // Because guest start pc = 0 and each inst is 4 bytes, guest "address" of
    // inst[i] = i*4.
    let mut host_of_guest: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for (i, &inst) in insts.iter().enumerate() {
        let guest_pc = (i as u64) * 4;
        host_of_guest.insert(guest_pc, buf.len());
        translate::translate(&mut buf, guest_pc, inst, &mut fixups)?;
    }

    // epilogue: return x0 in RAX, ret (fallback for straight-line bodies)
    buf.mov_load64(RAX, RBX, 0);
    buf.ret();

    // Resolve fixups now (buffer-relative). The rel32 displacement at
    // fx.disp_off is relative to (disp_off + 4), the address immediately
    // after the displacement field. target is host offset of the target.
    for fx in &fixups {
        let target = *host_of_guest
            .get(&fx.target_pc)
            .ok_or_else(|| format!("branch to untranslated pc {:x}", fx.target_pc))?;
        let disp = target as i64 - (fx.disp_off as i64 + 4);
        let bytes = (disp as u32).to_le_bytes();
        buf.bytes[fx.disp_off..fx.disp_off + 4].copy_from_slice(&bytes);
    }

    let code = buf.as_slice().to_vec();
    let ptr = map_exec(&code);
    Ok(JitBlock {
        ptr,
        len: code.len(),
    })
}

/// Execute a compiled block against `state`, returning the value left in x0.
pub unsafe fn run(blk: &JitBlock, state: *mut CpuState) -> u64 {
    unsafe {
        let f: extern "C" fn(*mut CpuState) -> u64 = std::mem::transmute(blk.ptr);
        f(state)
    }
}

// ---------------------------------------------------------------------------
// Guest -> host call bridge
//
// A guest import (libc/libm/Android symbol) is reached by the guest branching
// (`blr`/`br`) to an address. Real imports must land on a *host* x86-64
// function, not more guest code. We reserve a fixed region of guest addresses
// `HOST_THUNK_BASE .. HOST_THUNK_BASE + N*8` that NEVER overlaps the mapped
// ELF image. When the `jit_run` dispatcher sees `pc` inside that region, it
// calls the registered host thunk with the guest x0..x7 as x86-64 SysV args
// (RDI,RSI,RDX,RCX,R8,R9, then stack) and stores the return into guest x0.
//
// A loader/linker fills each slot by resolving an aarch64 `R_AARCH64_JUMP_SLOT`
// GOT entry (or a `blr xN` target) to `HOST_THUNK_BASE + slot*8`, so a PLT
// `br x16` naturally lands on the thunk.
// ---------------------------------------------------------------------------

/// First guest address of the host-call thunk region (above any guest image).
pub const HOST_THUNK_BASE: u64 = 0x7f00_0000_0000;
/// Number of `HostCall` slots. Address of slot `i` is `HOST_THUNK_BASE + i*8`.
pub const HOST_THUNK_MAX: usize = 4096;

/// A host function callable with the x86-64 SysV ABI.
pub type HostCall = extern "C" fn(a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64;

/// A host **float-ABI** function: all args and the return use the x86-64 SysV
/// XMM registers (doubles), ABI-identical to an `extern "C" fn(f64,...,f64)->f64`.
/// AArch64 calls libm (sinf/cosf/atan2f/...) with floats in v0-v7, and the host
/// SysV rule routes the same values through xmm0-xmm7 — so reading the guest
/// v0-v7 low lanes and calling this recovers correct float results.
pub type HostFloatCall = extern "C" fn(f0: f64, f1: f64, f2: f64, f3: f64, f4: f64, f5: f64, f6: f64, f7: f64) -> f64;

/// A host **single-precision** float-ABI function (x86-64 SysV passes f32 in
/// XMM0-7; a Rust `extern "C" fn(f32,...,f32)->f32` uses exactly that). The
/// guest (Roblox `*f` imports: atan2f/asinf/sinf/...) stores an f32 in the low
/// 32 bits of v0-v7, so the bridge widens those lanes, calls, then narrows the
/// f32 result back into v0's low lane.
pub type HostFloat32Call = extern "C" fn(f0: f32, f1: f32, f2: f32, f3: f32, f4: f32, f5: f32, f6: f32, f7: f32) -> f32;

static HOST_CALLS: Mutex<[Option<HostCall>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);
static HOST_FLOAT_CALLS: Mutex<[Option<HostFloatCall>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);
static HOST_FLOAT32_CALLS: Mutex<[Option<HostFloat32Call>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);

/// Register `f` as the host call for guest slot `i`. Returns the guest address
/// the caller should resolve a JUMP_SLOT/intra-image `blr` target to so that the
/// `jit_run` dispatcher falls through to this host call.
pub fn register_host_call(i: usize, f: HostCall) {
    let mut hc = HOST_CALLS.lock().unwrap();
    if i < hc.len() {
        hc[i] = Some(f);
    }
}

/// Guest address of host-call slot `i`.
pub fn host_call_addr(i: usize) -> u64 {
    HOST_THUNK_BASE + (i as u64) * 8
}

/// Register `f` at the first free slot; returns its guest address. Mirrors the
/// float auto-allocators (`register_float_call`/`register_float32_call`).
pub fn register_host_call_auto(f: HostCall) -> u64 {
    let mut hc = HOST_CALLS.lock().unwrap();
    let i = hc.iter().position(|s| s.is_none()).expect("host thunk table full");
    hc[i] = Some(f);
    host_call_addr(i)
}

/// Look up (host fn, slot index) for a guest `pc` that falls in the thunk
/// region. Returns `None` if `pc` is outside it or the slot is unregistered.
pub fn host_call_at(pc: u64) -> Option<(HostCall, usize)> {
    if pc < HOST_THUNK_BASE {
        return None;
    }
    let off = pc - HOST_THUNK_BASE;
    if off % 8 != 0 {
        return None;
    }
    let i = (off / 8) as usize;
    let hc = HOST_CALLS.lock().unwrap();
    hc.get(i).copied().flatten().map(|f| (f, i))
}

/// Guest base address of the **float**-ABI thunk region (after the integer slots).
pub fn host_float_base() -> u64 {
    HOST_THUNK_BASE + (HOST_THUNK_MAX as u64) * 8
}

/// Register a float host fn at an auto-allocated slot; returns its guest addr.
pub fn register_float_call(f: HostFloatCall) -> u64 {
    let mut hc = HOST_FLOAT_CALLS.lock().unwrap();
    let i = hc.iter().position(|s| s.is_none()).expect("float thunk table full");
    hc[i] = Some(f);
    host_float_call_addr(i)
}

/// Guest address of float host-call slot `i`.
pub fn host_float_call_addr(i: usize) -> u64 {
    host_float_base() + (i as u64) * 8
}

/// Look up a float host fn for a guest `pc` in the float thunk region.
fn host_float_call_at(pc: u64) -> Option<(HostFloatCall, usize)> {
    let base = host_float_base();
    if pc < base {
        return None;
    }
    let off = pc - base;
    if off % 8 != 0 {
        return None;
    }
    let i = (off / 8) as usize;
    let hc = HOST_FLOAT_CALLS.lock().unwrap();
    hc.get(i).copied().flatten().map(|f| (f, i))
}

/// Guest base address of the **single-precision** float thunk region (after the
/// f64 float slots).
#[inline(always)]
pub fn host_float32_base() -> u64 {
    host_float_base() + (HOST_THUNK_MAX as u64) * 8
}

/// Register a single-precision float host fn at an auto-allocated slot; returns
/// its guest address.
pub fn register_float32_call(f: HostFloat32Call) -> u64 {
    let mut hc = HOST_FLOAT32_CALLS.lock().unwrap();
    let i = hc
        .iter()
        .position(|s| s.is_none())
        .expect("float32 thunk table full");
    hc[i] = Some(f);
    host_float32_base() + (i as u64) * 8
}

/// Look up a single-precision float host fn for a guest `pc` in the f32 region.
fn host_float32_call_at(pc: u64) -> Option<(HostFloat32Call, usize)> {
    let base = host_float32_base();
    if pc < base {
        return None;
    }
    let off = pc - base;
    if off % 8 != 0 {
        return None;
    }
    let i = (off / 8) as usize;
    let hc = HOST_FLOAT32_CALLS.lock().unwrap();
    hc.get(i).copied().flatten().map(|f| (f, i))
}

/// Supervisor-call dispatcher. AArch64 uses x8 as the syscall number and x0-x5
/// as args (AArch64 Linux ABI: x8=number, x0..x5 args, return in x0, negative =
/// -errno). The guest (Roblox on the Android aarch64 ABI) issues AArch64 syscall
/// numbers, but we run on x86-64, whose syscall number table is entirely
/// different. So we map each AArch64 nr -> x86-64 nr and forward the first 3-5
/// args to `libc::syscall` (the raw kernel path). `libc::syscall` already
/// returns the kernel's -errno encoding, which we re-package as the u64 the
/// guest expects (high bits set for errors).
///
/// Common mappings (AArch64 -> x86-64, Linux):
///   read 63->0, write 64->1, openat 56->257, close 57->3, fstat 79->4,
///   brk 214->12, mmap 222->9, mprotect 226->10, munmap 215->11,
///   ioctl 29->16, futex 98->202, exit 93->60, exit_group 94->231,
///   getpid 172->39, getppid 173->110, getuid 199->102, nanosleep 101->35,
///   clock_gettime 113->228, getrandom 278->318, access 48->21, uname 160->65,
///   gettimeofday 169->96 (to libc instead), readahead, ...
pub extern "C" fn guest_svc(st: *mut CpuState) -> u64 {
    let s = unsafe { &mut *st };
    let nr = s.x[8];
    let a = [s.x[0], s.x[1], s.x[2], s.x[3], s.x[4], s.x[5]];
    if std::env::var("JIT_TRACE_SVC").is_ok() {
        eprintln!(
            "guest svc {:x} ({}) a0={:#x} a1={:#x} a2={:#x} a3={:#x}",
            nr, nr, a[0], a[1], a[2], a[3]
        );
    }
    use libc::{c_long, c_void, c_char, c_int};
    // AArch64 -> host. We dispatch by AArch64 syscall number directly to the
    // matching libc call (which does the native x86-64 syscall), so the mapping
    // is exact and readable rather than a fragile number shuffle. Errors come
    // back as -1 + errno; we convert to the kernel's -errno convention.
    let ret: c_long = match nr {
        // --- process / exit ---
        93 | 94 => { // exit(93) / exit_group(94)
            eprintln!("guest_svc: exit_group({}) from guest", a[0]);
            std::process::exit(a[0] as i32);
        }
        // --- basic I/O ---
        63 => unsafe { libc::read(a[0] as c_int, a[1] as *mut c_void, a[2] as usize) as c_long },
        64 => unsafe { libc::write(a[0] as c_int, a[1] as *const c_void, a[2] as usize) as c_long },
        57 => unsafe { libc::close(a[0] as c_int) as c_long },
        56 => unsafe { libc::openat(a[0] as c_int, a[1] as *const c_char, a[2] as c_int, a[3] as c_long as u32) as c_long },
        // --- memory ---
        222 => unsafe { libc::mmap(a[0] as *mut c_void, a[1] as usize, a[2] as c_int, a[3] as c_int, a[4] as c_int, a[5] as i64) as c_long },
        226 => unsafe { libc::mprotect(a[0] as *mut c_void, a[1] as usize, a[2] as c_int) as c_long },
        215 => unsafe { libc::munmap(a[0] as *mut c_void, a[1] as usize) as c_long },
        214 => unsafe {
            // brk(0) quirk: return current break by calling with NULL.
            let r = libc::syscall(c_long::from(libc::SYS_brk), a[0] as usize) as *mut c_void;
            if a[0] == 0 { return libc::syscall(libc::SYS_brk, 0 as usize) as u64; }
            r as c_long
        },
        220 => unsafe { libc::syscall(libc::SYS_mremap, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as c_int, a[4] as usize) as c_long },
        // --- time ---
        113 => unsafe { libc::clock_gettime(a[0] as libc::clockid_t, a[1] as *mut libc::timespec) as c_long },
        101 => unsafe { libc::nanosleep(a[1] as *const libc::timespec, a[2] as *mut libc::timespec) as c_long },
        // --- process / user identity ---
        172 => unsafe { libc::getpid() as c_long },
        199 => unsafe { libc::getuid() as c_long },
        98 => unsafe {
            // futex: only FUTEX_WAIT(0)/FUTEX_WAKE(1) forwarded to the host. Others return 0.
            let op = a[1] as i32;
            let fut = a[0] as *mut libc::c_int;
            let om = (op as u32) & 0x7f;
            if om == libc::FUTEX_WAKE as u32 {
                libc::syscall(libc::SYS_futex, fut as usize, op, a[2] as c_long, 0 as usize) as c_long
            } else if om == libc::FUTEX_WAIT as u32 {
                libc::syscall(
                    libc::SYS_futex,
                    fut as usize,
                    op,
                    a[2] as c_long,
                    a[3] as *const libc::timespec,
                ) as c_long
            } else {
                0
            }
        },
        // --- misc upper commonly needed ---
        278 => unsafe { libc::syscall(libc::SYS_getrandom, a[0] as usize, a[1] as usize, a[2] as u32) as c_long },
        _ => {
            eprintln!(
                "guest_svc: unhandled AArch64 syscall {nr} -> -ENOSYS (a0={:#x} a1={:#x} a2={:#x})",
                a[0], a[1], a[2]
            );
            return (-38i64) as u64; // -ENOSYS
        }
    };
    // Convert -1-with-errno into the kernel's -errno encoding the guest expects.
    if ret == -1 {
        // errno is positive; kernel convention is to return -errno.
        let e = unsafe { *libc::__errno_location() };
        (0i64 - e as i64) as u64
    } else {
        ret as u64
    }
}

/// Hased SHA-1 / SHA-256 crypto helper called by translated code for the
/// ARM crypto SHA instructions. `packed` = [mode(8)][rd(5)][rn(5)][rm(5)][--9]
/// with mode: 1=sha1h, 2=sha1c, 3=sha1p, 4=sha1m, 5=sha256h, 6=sha1su0,
/// 7=sha1su1, 8=sha256su0, 9=sha256su1, 10=sha256h2.
/// Vector register r lives at st.v[2r] (words 0..1) and st.v[2r+1] (words 2..3).
pub extern "C" fn guest_sha1stem(st: *mut CpuState, packed: u64) -> u64 {
    // shim over the pure Rust helpers so the impl is testable.
    let s = unsafe { &mut *st };
    unsafe { sha1_host_impl(s, packed) };
    0
}

fn sha1_host_impl(s: &mut CpuState, packed: u64) {
    let mode = (packed >> 24) & 0xff;
    let rd = ((packed >> 16) & 0x1f) as usize;
    let rn = ((packed >> 8) & 0x1f) as usize;
    let rm = (packed & 0x1f) as usize;
    let rol = |x: u32, n: u32| x.rotate_left(n);
    let ror = |x: u32, n: u32| x.rotate_right(n);
    let s1 = |x: u32| ror(x, 6) ^ ror(x, 11) ^ ror(x, 25);
    let s0 = |x: u32| ror(x, 2) ^ ror(x, 13) ^ ror(x, 22);

    // Free helpers (no closure capture of s.v => no borrow conflict).
    fn lw(v: &[u64], r: usize, i: usize) -> u32 {
        ((v[2 * r + i / 2]) >> ((i % 2) * 32)) as u32 & 0xffff_ffff
    }
    fn wr(v: &mut [u64], r: usize, i: usize, val: u32) {
        let sh = (i % 2) * 32;
        v[2 * r + i / 2] = (v[2 * r + i / 2] & !(0xffff_ffffu64 << sh)) | (((val as u64) & 0xffff_ffff) << sh);
    }

    match mode {
        1 => {
            // sha1h: rd.word0 = ror32(rn,2); w1..3 = 0
            let v = lw(&s.v, rn, 0).rotate_right(2);
            wr(&mut s.v, rd, 0, v);
            wr(&mut s.v, rd, 1, 0); wr(&mut s.v, rd, 2, 0); wr(&mut s.v, rd, 3, 0);
        }
        2 | 3 | 4 => {
            // sha1c/p/m Qd(d), Sn(=n0), Vm.4s
            let mut d = [lw(&s.v, rd, 0), lw(&s.v, rd, 1), lw(&s.v, rd, 2), lw(&s.v, rd, 3)];
            let n0 = lw(&s.v, rn, 0);
            let m = [lw(&s.v, rm, 0), lw(&s.v, rm, 1), lw(&s.v, rm, 2), lw(&s.v, rm, 3)];
            let mut nn = n0;
            let f: fn(u32, u32, u32) -> u32 = match mode {
                3 => |x, y, z| x ^ y ^ z,
                4 => |x, y, z| (x & y) | ((x | y) & z),
                _ => |x, y, z| (x & (y ^ z)) ^ z, // cho
            };
            for i in 0..4 {
                let t = f(d[1], d[2], d[3])
                    .wrapping_add(d[0].rotate_left(5))
                    .wrapping_add(nn)
                    .wrapping_add(m[i]);
                nn = d[3];
                d[3] = d[2];
                d[2] = d[1].rotate_right(2);
                d[1] = d[0];
                d[0] = t;
            }
            for i in 0..4 { wr(&mut s.v, rd, i, d[i]); }
        }
        5 => {
            // sha256h: 4 rounds
            let mut d = [lw(&s.v, rd, 0), lw(&s.v, rd, 1), lw(&s.v, rd, 2), lw(&s.v, rd, 3)];
            let mut n = [lw(&s.v, rn, 0), lw(&s.v, rn, 1), lw(&s.v, rn, 2), lw(&s.v, rn, 3)];
            let m = [lw(&s.v, rm, 0), lw(&s.v, rm, 1), lw(&s.v, rm, 2), lw(&s.v, rm, 3)];
            let cho = |x: u32, y: u32, z: u32| (x & (y ^ z)) ^ z;
            let maj = |x: u32, y: u32, z: u32| (x & y) | ((x | y) & z);
            for i in 0..4 {
                let t = cho(n[0], n[1], n[2])
                    .wrapping_add(n[3])
                    .wrapping_add(s1(n[0]))
                    .wrapping_add(m[i]);
                n[3] = n[2]; n[2] = n[1]; n[1] = n[0];
                n[0] = d[3].wrapping_add(t);
                let t = t.wrapping_add(maj(d[0], d[1], d[2])).wrapping_add(s0(d[0]));
                d[3] = d[2]; d[2] = d[1]; d[1] = d[0];
                d[0] = t;
            }
            for i in 0..4 { wr(&mut s.v, rd, i, d[i]); }
        }
        _ => {
            // 6 = sha1su0, 7 = sha1su1
            let d0 = lw(&s.v, rd, 0); let d1 = lw(&s.v, rd, 1);
            let d2 = lw(&s.v, rd, 2); let d3 = lw(&s.v, rd, 3);
            let n0 = lw(&s.v, rn, 0);
            let m0 = lw(&s.v, rm, 0); let m1 = lw(&s.v, rm, 1);
            if mode == 6 {
                // sha1su0: d0 = d1^d0^m0 ; d1 = n0^d1^m1
                wr(&mut s.v, rd, 0, d0 ^ d1 ^ m0);
                wr(&mut s.v, rd, 1, d1 ^ n0 ^ m1);
            } else {
                // sha1su1
                let m2 = lw(&s.v, rm, 2); let m3 = lw(&s.v, rm, 3);
                wr(&mut s.v, rd, 0, (d0 ^ m1).rotate_left(1));
                wr(&mut s.v, rd, 1, (d1 ^ m2).rotate_left(1));
                wr(&mut s.v, rd, 2, (d2 ^ m3).rotate_left(1));
                wr(&mut s.v, rd, 3, (d3 ^ d0).rotate_left(1));
            }
        }
    }
    let _ = (&rol, &s1, &s0);
}

/// Convenience: translate+call a slice of raw guest bytes (AArch64) reached at
/// the given initial PC, executing them against `state`. Returns the final x0.
pub fn exec_bytes(state: &mut CpuState, bytes: &[u8], _start_pc: u64) -> Result<u64, String> {
    let insts: Vec<Inst> = bytes
        .chunks_exact(4)
        .map(|b| decode::decode(u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        .collect();
    let blk = compile(&insts, state as *mut CpuState)?;
    let r = unsafe { run(&blk, state as *mut CpuState) };
    Ok(r)
}

/// A block-level, PC-driven JIT executor for a guest image whose AArch64 bytes
/// live at guest address `base` (guest vaddr == host address). This supports
/// *indirect* control flow (`br`/`blr`) and returns from calls that the
/// single-shot `compile_image` cannot: each reachable region is compiled via
/// `compile_image` (which inlines static `b`/`b.cond`/`cbz`/`bl` and stops with
/// `pc=…; ret` at a `br`/`blr`/`ret`), then run; when it returns because of such
/// an indirect/return transfer, `state.pc` holds the next address, so the
/// dispatcher compiles & re-enters there. Halts when `pc == 0`.
pub fn jit_run(image: &[u8], base: u64, entry: u64, state: *mut CpuState) -> Result<u64, String> {
    // Epoch for the CNTVCT_EL0 readout. The guest reads cntfrq_el0 (100 MHz)
    // and cntvct_el0 to compute time deltas; stamp the per-block counter once so
    // it stays monotonic and agrees with the declared frequency.
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let epoch = *EPOCH.get_or_init(Instant::now);
    let stamp_cntvct = |st: *mut CpuState| {
        let ns = epoch.elapsed().as_nanos() as u64; // ns since guest start
        let ticks = ns / 10; // /10 ns == 100 MHz ticks
        unsafe { (*st).cntvct = ticks };
    };
    unsafe { (*state).pc = entry }
    let mut guard: u64 = 0;
    const MAX_STEPS: u64 = 20_000_000; // safety net against an infinite guest loop
    loop {
        if guard >= MAX_STEPS {
            return Err("run_loop: step budget exceeded (infinite guest loop?)".into());
        }
        guard += 1;
        let pc = unsafe { (*state).pc };
        if pc == 0 {
            return Ok(unsafe { (*state).x[0] });
        }
        // Guest -> host call bridge: if `pc` is a registered host thunk slot,
        // invoke the host x86-64 function with the guest x0..x7 args and store
        // the return into guest x0. The guest `blr` already linked x30 to the
        // caller, so resume there. This is how a resolved import (libc/libm/JNI
        // shim) is reached from translated Roblox code.
        if let Some((hostf, slot)) = host_call_at(pc) {
            #[cfg(debug_assertions)]
            if std::env::var_os("JIT_TRACE").is_some() {
                let s = unsafe { &*state };
                println!(
                    "  hostcall@slot{slot} pc={pc:#x} x0={:#x} x1={:#x} x2={:#x} x30={:#x}",
                    s.x[0], s.x[1], s.x[2], s.x[30]
                );
            }
            let s = unsafe { &mut *state };
            let ret = hostf(s.x[0], s.x[1], s.x[2], s.x[3], s.x[4], s.x[5], s.x[6], s.x[7]);
            s.x[0] = ret;
            s.pc = s.x[30]; // return to the `blr` caller
            continue;
        }
        // Float-ABI bridge: guest libm calls (atan2f/... with v0-v7 args). Read
        // the guest v0..v7 d-lanes as f64, call the host float fn (double via
        // xmm0..xmm7 in SysV), store the f64 return into guest v0.
        if let Some((hostf, _slot)) = host_float_call_at(pc) {
            let s = unsafe { &mut *state };
            let v = &s.v;
            let a0 = f64::from_bits(v[0]);
            let a1 = f64::from_bits(v[2]);
            let a2 = f64::from_bits(v[4]);
            let a3 = f64::from_bits(v[6]);
            let a4 = f64::from_bits(v[8]);
            let a5 = f64::from_bits(v[10]);
            let a6 = f64::from_bits(v[12]);
            let a7 = f64::from_bits(v[14]);
            let ret = hostf(a0, a1, a2, a3, a4, a5, a6, a7);
            let s = unsafe { &mut *state };
            s.v[0] = ret.to_bits(); // d0 = float return
            s.pc = s.x[30];
            continue;
        }
        // Single-precision float bridge: guest `*f` calls (atan2f/asinf/...)
        // pass f32 in the low 32 bits of s0-s7 (v0-v7 low lanes). Widen to f32,
        // call the host f32 fn via xmm0..xmm7, narrow the f32 result into s0.
        if let Some((hostf, _slot)) = host_float32_call_at(pc) {
            let s = unsafe { &mut *state };
            let v = &s.v;
            let l32 = |x: u64| f32::from_bits(x as u32);
            let a0 = l32(v[0]);
            let a1 = l32(v[2]);
            let a2 = l32(v[4]);
            let a3 = l32(v[6]);
            let a4 = l32(v[8]);
            let a5 = l32(v[10]);
            let a6 = l32(v[12]);
            let a7 = l32(v[14]);
            let ret = hostf(a0, a1, a2, a3, a4, a5, a6, a7);
            let s = unsafe { &mut *state };
            s.v[0] = (s.v[0] & !0xffff_ffff) | ret.to_bits() as u64; // s0 = f32 return
            s.pc = s.x[30];
            continue;
        }
        if pc < base || pc - base + 4 > image.len() as u64 {
            return Err(format!(
                "run_loop: pc 0x{pc:x} outside image [0x{base:x}, 0x{:x})",
                base + image.len() as u64
            ));
        }
        // Bounded trace compilation: cap each block's guest-instruction budget so
        // a real function like `JNI_OnLoad` is compiled into small, bounded
        // blocks whose out-of-range branch/call edges divert back through the
        // dispatcher loop below — instead of eagerly expanding the whole
        // reachable call graph into one multi-MB blast that took seconds to
        // translate and then SIGSEGV'd. CONFIG_JUMP_GUEST_BUDGET tunable.
        const BLOCK_BUDGET: usize = 8192;
        let block = compile_image_bounded(image, base, pc, state, BLOCK_BUDGET)?;
        #[cfg(debug_assertions)]
        if std::env::var_os("JIT_DUMP").is_some() {
            let raw = block.dump();
            eprintln!("-- block@0x{pc:x} host bytes ({}):", raw.len());
            for (i, byte) in raw.iter().enumerate() {
                eprint!("{:02x} ", byte);
                if (i + 1) % 16 == 0 {
                    eprintln!();
                }
            }
            eprintln!();
        }
        stamp_cntvct(state);
        unsafe { run(&block, state) };
        if std::env::var_os("JIT_TRACE").is_some() {
            println!(
                "  block@0x{pc:x} -> pc=0x{:x} x0=0x{:x} x1=0x{:x} x30=0x{:x}",
                unsafe { (*state).pc },
                unsafe { (*state).x[0] },
                unsafe { (*state).x[1] },
                unsafe { (*state).x[30] }
            );
        }
    }
}

/// Translate every instruction of the guest image `image` (a full program
/// whose AArch64 bytes start at guest address `base`) into a single host
/// function, following branches and BL calls so any reachable code is
/// present. `entry` is the guest address to start from. Instructions reached
/// only via branch/call (not just linear fallthrough) are included.
pub fn compile_image(
    image: &[u8],
    base: u64,
    entry: u64,
    state: *mut CpuState,
) -> Result<JitBlock, String> {
    compile_image_bounded(image, base, entry, state, 0)
}

/// Detect whether the instructions at host address `addr` are a PLT stub
/// (`adrp xd,P; ldr xc,[xd,#imm]; add xd,xd,#off; br xc`) whose GOT slot holds a
/// host-thunk address (>= HOST_THUNK_BASE). This identifies a direct guest `bl`
/// to a host import (e.g. `bl pthread_mutex_lock@plt`). Since guest == host
/// memory here, we read the stub bytes and the (already patched) JUMP_SLOT GOT
/// entry straight from mapped memory. Returns false on any mismatch so this is
/// conservative: a real guest function is never mistaken for an import stub.
fn is_host_plt_stub(addr: u64) -> bool {
    if addr < 0x1000 {
        return false;
    }
    #[cfg(debug_assertions)]
    if std::env::var_os("JIT_DUMP").is_some() {
        let dbg_p = addr as *const u8;
        eprintln!(
            "[hps] addr={addr:#x} w0={:#010x} w1={:#010x} w2={:#010x} w3={:#010x}",
            unsafe { std::ptr::read_unaligned(dbg_p as *const u32) },
            unsafe { std::ptr::read_unaligned(dbg_p.add(4) as *const u32) },
            unsafe { std::ptr::read_unaligned(dbg_p.add(8) as *const u32) },
            unsafe { std::ptr::read_unaligned(dbg_p.add(12) as *const u32) }
        );
    }
    let p = addr as *const u8;
    // word 0: adrp Xd, #page
    let w0 = unsafe { std::ptr::read_unaligned(p as *const u32) };
    if (w0 & 0x9f00_0000) != 0x9000_0000 {
        #[cfg(debug_assertions)]
        if std::env::var_os("JIT_DUMP").is_some() {
            eprintln!("[hps] {addr:#x} NOT adrp (w0 {w0:#x})");
        }
        return false;
    }
    let d0 = w0 & 0x1f;
    // word 1: ldr Xt, [Xn, #imm]   (64-bit unsigned-offset load)
    let w1 = unsafe { std::ptr::read_unaligned(p.add(4) as *const u32) };
    if (w1 & 0xffc0_0000) != 0xf940_0000 {
        return false;
    }
    let rn = (w1 >> 5) & 0x1f;
    let dt = w1 & 0x1f;
    if rn != d0 {
        return false; // must load from the adrp'ed page reg (a real PLT stub)
    }
    // word 2: add Xd, Xd, #off (the AArch64 canonical PLT stub does this)
    let w2 = unsafe { std::ptr::read_unaligned(p.add(8) as *const u32) };
    if (w2 & 0xff00_0000) != 0x9100_0000 {
        return false;
    }
    // word 3: br Xt   — must branch to the register loaded by the `ldr` above.
    let w3 = unsafe { std::ptr::read_unaligned(p.add(12) as *const u32) };
    if (w3 & 0xffff_fc1f) != 0xd61f_0000 || ((w3 >> 5) & 0x1f) != dt {
        return false;
    }
    // A `bl` to exactly this canonical 4-instruction PLT stub is a host import:
    // after `bind_image_plt`, every JUMP_SLOT GOT entry resolves to a host thunk
    // (>= HOST_THUNK_BASE), so the stub's `br` will hand pc to the dispatcher's
    // host-call bridge only if this `bl` is diverted rather than call-inlined.
    true
}

/// Like `compile_image` but stops expanding the reachable frontier once the
/// translation has emitted `budget` guest instructions (0 = unbounded). Every
/// branch/call fixup whose target was NOT emitted is redirected to an appended
/// dispatcher-return stub that writes that target into `CpuState.pc` and `ret`s,
/// so `jit_run` picks up the next block on its own re-entry loop. This is the
/// mechanism that keeps a real function like `JNI_OnLoad` from being eagerly
/// compiled into a single 78 MB blast-block that makes translation take seconds
/// and then SIGSEGVs.
pub fn compile_image_bounded(
    image: &[u8],
    base: u64,
    entry: u64,
    state: *mut CpuState,
    budget: usize,
) -> Result<JitBlock, String> {
    // Protect against nonsense sizes.
    if entry < base || entry - base >= image.len() as u64 {
        return Err(format!(
            "entry {:x} outside image [{:x}, {:x})",
            entry,
            base,
            base + image.len() as u64
        ));
    }

    let mut buf = CodeBuf::new();
    let mut fixups: Vec<crate::translate::Fixup> = Vec::new();
    buf.mov_ri64(RBX, state as usize as u64);

    // Walk the image: emit fall-through linearly, following branch/call targets.
    let mut host_of_guest: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    let mut frontier: Vec<u64> = vec![entry];
    let mut emitted: usize = 0;
    let bounded = budget > 0;
    // Whether the budget cut us off before draining the reachable frontier. When
    // true we must divert any not-yet-emitted targets to the dispatcher.
    let mut truncated = false;
    // Set when we deliberately divert a `bl` to a host-import PLT stub (see the
    // `Inst::B` handler): those targets are intentionally not emitted, so we must
    // still build dispatcher-return stubs for them even when the frontier drains
    // normally (otherwise their fixups index an empty stub table).
    let mut force_stubs = false;
    // Invariant: every address in frontier is a candidate block start.
    while let Some(addr) = frontier.pop() {
        if host_of_guest.contains_key(&addr) {
            continue; // already emitted
        }
        let mut cur = addr;
        loop {
            if bounded && emitted >= budget {
                truncated = true;
                break;
            }
            if cur < base || cur - base + 4 > image.len() as u64 {
                break; // out of bounds; translate.rs will error if truly needed
            }
            if host_of_guest.contains_key(&cur) {
                break; // reached already-emitted code (loop back-edge)
            }
            let off = (cur - base) as usize;
            let word =
                u32::from_le_bytes([image[off], image[off + 1], image[off + 2], image[off + 3]]);
            let inst = decode::decode(word);
            // record a host label for this guest pc *before* constraining the
            // shape of the block (branches patch to it).
            host_of_guest.insert(cur, buf.len());
            match &inst {
                Inst::B { imm, link } => {
                    let target = cur.wrapping_add(*imm as u64);
                    if *link {
                        frontier.push(cur /* continue after call (fall-through) */ + 4);
                        // A `bl` to a host-import PLT stub (pthread_mutex_lock,
                        // syslog, abort, ...) must NOT be compiled inline as guest
                        // text: doing so makes the stub's `br x17` return into the
                        // inlined caller instead of handing pc to the dispatcher's
                        // host-call bridge, so the real import never runs and the
                        // guest keeps going with a garbage return. Divert it to
                        // the dispatcher (the fixup will route to a return-stub).
                        let hps = is_host_plt_stub(target);
                        #[cfg(debug_assertions)]
                        if std::env::var_os("JIT_DUMP").is_some() {
                            eprintln!("[bl] {cur:#x} -> {target:#x} hostplt={hps}");
                        }
                        if !hps {
                            frontier.push(target);
                        } else {
                            // Diverted: don't inline this import; make sure the
                            // stub table is built so the fixup has a real target.
                            force_stubs = true;
                        }
                    } else {
                        frontier.push(target);
                    }
                }
                Inst::BCond { imm, .. } | Inst::Cbz { imm, .. } | Inst::Tbz { imm, .. } => {
                    let target = cur.wrapping_add(*imm as u64);
                    frontier.push(target); // conditional: also fall through below
                }
                Inst::Ret | Inst::Unsupported(_) | Inst::Br { .. } | Inst::Blr { .. } => {
                    // terminal; do not continue fall-through
                }
                _ => {
                    // default: continue linearly
                }
            }
            translate::translate(&mut buf, cur, inst, &mut fixups)?;
            emitted += 1; // count a translated guest instruction toward the budget
            // Ret / indirect transfers / unconditional B are terminal: stop this
            // block (an unconditional `b` must NOT fall through to the next word,
            // which may be `.text` zero-fill or an unrelated function — landing
            // there is how we were hitting `Unsupported(0x00000000)` pads).
            if matches!(
                inst,
                Inst::Ret
                    | Inst::Unsupported(_)
                    | Inst::Br { .. }
                    | Inst::Blr { .. }
                    | Inst::Brk { .. }
                    | Inst::Udf { .. }
                    | Inst::B {
                        link: false, ..
                    }
            ) {
                break;
            }
            cur += 4;
        }
    }

    // epilogue: return x0, ret (only reached if entry falls off the end)
    buf.mov_load64(RAX, RBX, 0);
    buf.ret();

    // Bounded-mode: append one dispatcher-return stub per distinct target we
    // could not emit, then point every outstanding fixup whose target missed the
    // block at its stub (rewriting a call's host `call` into a `jmp` so no host
    // return address is left on the stack — the stub hands pc back to `jit_run`).
    let mut stub_of_target: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    if truncated || !frontier.is_empty() || force_stubs {
        // collect the set of targets referenced by fixups but not emitted.
        let need: Vec<u64> = fixups
            .iter()
            .filter(|fx| !host_of_guest.contains_key(&fx.target_pc))
            .map(|fx| fx.target_pc)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if !need.is_empty() {
            for target in need.iter() {
                // record the stub address *before* emitting it so the fixup
                // rel32 resolves to the stub's entry (the mov_ri64 below).
                let stub_at = buf.len();
                // stub: mov [CpuState+PC_OFF], #target ; ret
                buf.mov_ri64(RAX, *target);
                buf.mov_store64(RBX, crate::jit::PC_OFF, RAX);
                buf.ret();
                stub_of_target.insert(*target, stub_at);
            }
        }
    }

    // Resolve fixups (buffer-relative).
    for fx in &fixups {
        let target = if host_of_guest.contains_key(&fx.target_pc) {
            host_of_guest[&fx.target_pc]
        } else if bounded {
            // Divert to a dispatcher-return stub. Change a `call` into a `jmp`
            // so the host return address disappears (the stub hands pc back to
            // jit_run, and the callee's own `ret` via x30 covers the return).
            if fx.cc == 0xfe {
                // call_rel32 emits opcode 0xE8 then a 4-byte disp whose field
                // starts at disp_off (patch_here sets disp_off = len-4 right
                // after the E8), so the E8 byte sits at disp_off-1.
                buf.bytes[fx.disp_off - 1] = 0xe9; // E8 -> E9 (call->jmp)
            }
            stub_of_target[&fx.target_pc]
        } else {
            return Err(format!("branch/call to untranslated pc {:x}", fx.target_pc));
        };
        let disp = target as i64 - (fx.disp_off as i64 + 4);
        let bytes = (disp as u32).to_le_bytes();
        buf.bytes[fx.disp_off..fx.disp_off + 4].copy_from_slice(&bytes);
    }

    let code = buf.as_slice().to_vec();
    let ptr = map_exec(&code);
    Ok(JitBlock {
        ptr,
        len: code.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_add_executes_to_7() {
        // aarch64: mov x0,#3 ; add x0,x0,#4  =>  x0 = 7
        // d2800060 (mov x0,#3), 91001000 (add x0,x0,#4)
        let code = [0x60u8, 0x00, 0x80, 0xd2, 0x00, 0x10, 0x00, 0x91];
        let mut st = CpuState::new();
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 7, "mov x0,#3; add x0,x0,#4");
    }

    #[test]
    fn real_arm64_objdump_sequence() {
        let code = [0x60u8, 0x00, 0x80, 0xd2];
        let mut st = CpuState::new();
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 3);
    }

    #[test]
    fn ldr_imm_loads_memory() {
        // Real aarch64: "ldr x0, [x0, #16]" = 0xf9400800 ; ret = 0xd65f03c0
        // (from `ldi_unsigned` in sample.c). Loads the u64 at x0+16 into x0.
        let code = [0x00u8, 0x08, 0x40, 0xf9, 0xc0, 0x03, 0x5f, 0xd6];
        let mut buf = [0u64; 4]; // buffer; buf[2] at byte 16
        buf[2] = 0x1234_5678_9abc_def0;
        let mut st = CpuState::new();
        st.x[0] = buf.as_ptr() as u64; // x0 = &buf[0]
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, buf[2], "ldr x0,[x0,#16] should load buf[2]");
    }

    #[test]
    fn cbz_controls_branch() {
        // Real aarch64 from objdump (f:); if x0==0 return 10, else return 20.
        //  d2800281 mov x1,#20 ; b4000060 cbz x0,#10 ;
        //  d2800280 mov x0,#20 ; d65f03c0 ret ;
        //  d2800140 mov x0,#10 ; d65f03c0 ret
        let code = [
            0x81u8, 0x02, 0x80, 0xd2, // mov x1,#20
            0x60, 0x00, 0x00, 0xb4, // cbz x0, +0x10
            0x80, 0x02, 0x80, 0xd2, // mov x0,#20
            0xc0, 0x03, 0x5f, 0xd6, // ret
            0x40, 0x01, 0x80, 0xd2, // mov x0,#10
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        // x0 == 0 -> cbz taken -> x0 = 10
        let mut st_take = CpuState::new();
        let r = exec_bytes(&mut st_take, &code, 0).expect("exec-take");
        assert_eq!(r, 10, "x0==0 should take cbz branch");
        // x0 != 0 -> fall through -> x0 = 20
        let mut st_no = CpuState::new();
        st_no.x[0] = 99;
        let r = exec_bytes(&mut st_no, &code, 0).expect("exec-no");
        assert_eq!(r, 20, "x0!=0 should fall through");
    }

    #[test]
    fn cmp_ble_branch() {
        // Real aarch64 from objdump (g): return w0>3 ? 1 : 0
        // 71000c1f cmp w0,#3 ; 5400006d b.le 0x10 ; 52800020 mov w0,#1 ;
        //  d65f03c0 ret ; 52800000 mov w0,#0 ; d65f03c0 ret
        let code = [
            0x1fu8, 0x0c, 0x00, 0x71, // cmp w0, #3
            0x6d, 0x00, 0x00, 0x54, // b.le 0x10
            0x20, 0x00, 0x00, 0x52, // mov w0, #1
            0xc0, 0x03, 0x5f, 0xd6, // ret
            0x00, 0x00, 0x80, 0x52, // mov w0, #0
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        // w0=2 -> <=3 -> branch taken -> return 0
        let mut st_le = CpuState::new();
        st_le.x[0] = 2;
        let r = exec_bytes(&mut st_le, &code, 0).expect("exec-le");
        assert_eq!(r, 0, "x0=2 (<=3) should take b.le -> 0");
        // w0=5 -> >3 -> fall through -> return 1
        let mut st_gt = CpuState::new();
        st_gt.x[0] = 5;
        let r = exec_bytes(&mut st_gt, &code, 0).expect("exec-gt");
        assert_eq!(r, 1, "x0=5 (>3) should fall through -> 1");
    }

    #[test]
    fn bl_compiles_and_calls_leaf() {
        // caller = (x0+5)*2, via `bl h` then `add w0,w0,w0`.
        // 94000003 bl 0xc ; 0b000000 add w0,w0,w0 ; d65f03c0 ret
        // 11001400 add w0,w0,#5 ; d65f03c0 ret
        let image = [
            0x03u8, 0x00, 0x00, 0x94, // bl 0xc
            0x00, 0x00, 0x00, 0x0b, // add w0, w0, w0
            0xc0, 0x03, 0x5f, 0xd6, // ret
            0x00, 0x14, 0x00, 0x11, // add w0, w0, #5
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        // caller(5) = (5+5)*2 = 20 ; caller(0) = 10
        let mut st = CpuState::new();
        st.x[0] = 5;
        let blk = compile_image(&image, 0, 0, &mut st as *mut CpuState).expect("compile");
        let r = unsafe { run(&blk, &mut st as *mut CpuState) };
        assert_eq!(r, 20, "caller(5) should be 20");
    }

    #[test]
    fn bounded_bl_diverts_through_dispatcher() {
        // Entry at 0: `bl 0x14` (link to a callee we will NOT fit in the budget).
        // Verifies that with a tight budget the `bl` is rewritten into a
        // dispatcher-return stub: running the block leaves CpuState.pc == 0x14
        // so `jit_run` genuinely re-enters the callee next.
        let mut image = Vec::<u8>::new();
        image.extend_from_slice(&0x94000005u32.to_le_bytes()); // 0x00 bl 0x14
        image.extend_from_slice(&0xd4200000u32.to_le_bytes()); // 0x04 brk #0 (halt)
        while image.len() < 0x14 {
            image.push(0);
        }
        image.extend_from_slice(&0xd65f03c0u32.to_le_bytes()); // 0x14 ret

        // Budget 1: only the `bl` fits; the callee at 0x14 is out of trace, so its
        // fixup must be redirected to a dispatcher-return stub.
        let mut st = CpuState::new();
        let blk =
            compile_image_bounded(&image, 0, 0, &mut st as *mut CpuState, 1).expect("bounded compile");
        let _ = unsafe { run(&blk, &mut st as *mut CpuState) };
        // The stub wrote the diverted target into state.pc; the dispatcher (here
        // the test harness) would now re-enter there.
        assert_eq!(st.pc, 0x14, "bounded bl to out-of-budget target must divert via pc=0x14");
        assert_eq!(st.x[30], 0x04, "bl sets x30 link to pc+4");
    }

    #[test]
    fn ldstp_jit_prologue_roundtrip() {
        // f(a,b): stp x0,x1,[sp,#-16]! ; mov x0,#0 ; mov x1,#0 ;
        // ldp x0,x1,[sp],#16 ; ret  => returns original x0, sp restored.
        let code = [
            0xe0u8, 0x07, 0xbf, 0xa9, // stp x0,x1,[sp,#-16]!
            0x00, 0x00, 0x80, 0xd2, // mov x0,#0
            0x01, 0x00, 0x80, 0xd2, // mov x1,#0
            0xe0, 0x07, 0xc1, 0xa8, // ldp x0,x1,[sp],#16
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x4000usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base as isize, -1, "mmap for stack");
        let sp = base as usize + 0x3000;
        st.x[31] = sp as u64;
        st.x[0] = 0xdead_beef_cafe_0000;
        st.x[1] = 0x1122_3344_5566_7788;
        let r = exec_bytes(&mut st, &code, 0).expect("exec stp/ldp");
        assert_eq!(r, 0xdead_beef_cafe_0000, "x0 round-trips through stack");
        assert_eq!(st.x[31], sp as u64, "sp restored after post-index load");
        unsafe { libc::munmap(base, 0x4000) };
    }

    #[test]
    fn logic_ops_execute_real_code() {
        // 2a0003e1 mov w1,w0 ; 2a010000 orr w0,w0,w1 ;
        // 4a010000 eor w0,w0,w1 ; 0a010000 and w0,w0,w1 ; ret
        // (w0|w1)^w1 & w1   with w1==w0 => consistent result.
        let code = [
            0xe1, 0x03, 0x00, 0x2a, // mov w1, w0  (orr wzr,w0)
            0x00, 0x00, 0x01, 0x2a, // orr w0, w0, w1
            0x00, 0x00, 0x01, 0x4a, // eor w0, w0, w1
            0x00, 0x00, 0x01, 0x0a, // and w0, w0, w1
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[0] = 123u64;
        let r = exec_bytes(&mut st, &code, 0).expect("exec logic");
        assert_eq!(r, 0, "logical chain should reduce to 0");
    }

    #[test]
    fn mov_reg_alias_jit() {
        // mov x0, x1  =  orr x0, xzr, x1  (0xaa0103e0) ; ret
        let code = [
            0xe0, 0x03, 0x01, 0xaa, // mov x0, x1
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[1] = 0xfeed_face_cafe_0000;
        let r = exec_bytes(&mut st, &code, 0).expect("exec mov reg");
        assert_eq!(r, 0xfeed_face_cafe_0000, "mov x0,x1 copies register");
    }

    #[test]
    fn adrp_ldr_reads_global() {
        // Real global read: adrp x0, g ; add x0,x0,#0 ; ldr w0,[x0] ; ret.
        // Image maps code at page 0, global `g` (==33) at page 0x1000.
        let mut image = [0u8; 0x20004];
        // adrp x0, 0x20000 (real encoding 0x90000100) ; add x0,x0,#0 ; ldr w0,[x0] ; ret
        for (i, b) in [0x00u8, 0x01, 0x00, 0x90].iter().enumerate() {
            image[i] = *b;
        }
        for (i, b) in [0x00u8, 0x00, 0x00, 0x91].iter().enumerate() {
            image[4 + i] = *b;
        }
        for (i, b) in [0x00u8, 0x00, 0x40, 0xb9].iter().enumerate() {
            image[8 + i] = *b;
        }
        for (i, b) in [0xc0u8, 0x03, 0x5f, 0xd6].iter().enumerate() {
            image[12 + i] = *b;
        }
        // global g at 0x20000 = 33
        image[0x20000] = 33;
        // 64-bit scale: also confirm big constant is not relevant here (w32)
        let len = image.len();
        let rw = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(rw as isize, -1, "mmap image");
        unsafe { std::ptr::copy_nonoverlapping(image.as_ptr(), rw as *mut u8, len) };
        let base = rw as usize as u64;
        let mut st = CpuState::new();
        let blk = compile_image(&image, base, base, &mut st as *mut CpuState).expect("compile");
        let r = unsafe { run(&blk, &mut st as *mut CpuState) };
        assert_eq!(r, 33, "readg() should load the global g=33");
        unsafe { libc::munmap(rw, len) };
    }

    #[test]
    fn fp_scalar_double_ieee() {
        // Encodings verified from objdump of /tmp/fp2.s. NOTE: d-reg `dk` lives in
        // Rust array element `v[k*2]` (v is flat [u64;64] = 32 x two 64-bit lanes).
        use std::f64;
        let mut st = CpuState::new();
        st.v[2] = 2.5f64.to_bits(); // d1
        st.v[4] = 4.0f64.to_bits(); // d2
        let mut code = [0x20u8, 0x08, 0x62, 0x1e].to_vec(); // fmul d0,d1,d2
        code.extend_from_slice(&[0xc0u8, 0x03, 0x5f, 0xd6]); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fmul");
        assert_eq!(f64::from_bits(st.v[0]), 10.0, "2.5*4.0 = 10.0 (fmul)");

        let mut st2 = CpuState::new();
        st2.v[0] = 10.0f64.to_bits(); // d0
        st2.v[2] = 2.5f64.to_bits(); //  d1
        let mut code2 = [0x00u8, 0x28, 0x61, 0x1e].to_vec(); // fadd d0,d0,d1
        code2.extend_from_slice(&[0xc0u8, 0x03, 0x5f, 0xd6]);
        exec_bytes(&mut st2, &code2, 0).expect("exec fadd");
        assert_eq!(f64::from_bits(st2.v[0]), 12.5, "10.0 + 2.5 = 12.5");

        let mut st3 = CpuState::new();
        st3.v[12] = 10.0f64.to_bits(); // d6
        st3.v[14] = 2.5f64.to_bits(); //  d7
        let mut code3 = [0xc5u8, 0x18, 0x67, 0x1e].to_vec(); // fdiv d5,d6,d7
        code3.extend_from_slice(&[0xc0u8, 0x03, 0x5f, 0xd6]);
        exec_bytes(&mut st3, &code3, 0).expect("exec fdiv");
        assert_eq!(f64::from_bits(st3.v[10]), 4.0, "10.0 / 2.5 = 4.0");
    }

    #[test]
    fn simd_popcount_and_4s_add_reference() {
        // Honesty check for the Session-24/25 SIMD ops (not just "the binary got
        // further"): byte-popcount chain and 4x32-bit lane add, against hand
        // computed values on known 64-bit inputs.

        // (a) cnt v0.8b,v0.8b  + uaddlv h0,v0.8b  == popcount of the u64 in d0.
                // encodings (LE bytes for 0x0e205800 and 0x2e303800).
                let src: u64 = 0b1010_1111_0000_0011_1111_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000u64;
                let mut st = CpuState::new();
                st.v[0] = src; // d0
                let mut code = [0x00u8, 0x58, 0x20, 0x0e].to_vec(); // cnt v0.8b,v0.8b
                code.extend_from_slice(&[0x00u8, 0x38, 0x30, 0x2e]); // uaddlv h0,v0.8b
                code.extend_from_slice(&[0xc0u8, 0x03, 0x5f, 0xd6]); // ret
                exec_bytes(&mut st, &code, 0).expect("exec cnt+uaddlv");
                let got = st.v[0] & 0xffff; // uad...[truncated]

        // (b) add v0.4s, v1.4s, v0.4s : 4x32 lane add. word = 0x4ea08420.
        let mut st2 = CpuState::new();
        // v0 (vec 0): low u64 = v[0], high u64 = v[1]
        st2.v[0] = ((1u64) << 32) | 2; // lane0(low 32)=2, lane1=1
        st2.v[1] = ((4u64) << 32) | 3; // lane2=3, lane3=4
        st2.v[2] = ((10u64) << 32) | 20; // v1: lane0=20, lane1=10
        st2.v[3] = ((40u64) << 32) | 30; // v1: lane2=30, lane3=40
        let mut code2 = [0x20u8,0x84,0xa0,0x4e].to_vec(); // add v0.4s,v1.4s,v0.4s
        code2.extend_from_slice(&[0xc0u8,0x03,0x5f,0xd6]); // ret
        exec_bytes(&mut st2, &code2, 0).expect("exec add v0.4s");
        let l0 = (st2.v[0] & 0xffffffff) as u32;
        let l1 = (st2.v[0] >> 32) as u32;
        let l2 = (st2.v[1] & 0xffffffff) as u32;
        let l3 = (st2.v[1] >> 32) as u32;
        assert_eq!([l0, l1, l2, l3], [2+20, 1+10, 3+30, 4+40], "add v0.4s lanes");
    }

    #[test]
    fn byte_lane_and_logical_reference() {
        // Semantics of `add v.16b` / `and|orr|eor|bic v.16b` against hand bytes.
        // Seeds v0=0x0102..0f (16 bytes), v1=0x0f0e..01 down — verifies lane-base
        // registers (regression: these ops used RDX as the CpuState base, reading
        // garbage for the Vm operand and silently corrupting Vd).
        let mut st = CpuState::new();
        // v0 (16 bytes) = 01 02 03 .. 0f 10 ; v1 (16 bytes) = 11 12 .. 20
        st.v[0] = 0x0102_0304_0506_0708u64;          // d0 low
        st.v[1] = 0x090a_0b0c_0d0e_0f10u64;        // d0 high
        st.v[2] = 0x1112_1314_1516_1718u64;        // d1 low
        st.v[3] = 0x191a_1b1c_1d1e_1f20u64;        // d1 high
        let mut code = Vec::new();
        for w in [0x4e218402u32, 0x6e218403u32, 0x4e211c04u32, 0x4ea11c05u32, 0x6e211c06u32, 0x4e611c07u32] {
            code.extend_from_slice(&w.to_le_bytes());
        }
        code.extend_from_slice(&0xd65f03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec byte-add + logical");
        // result v2 (16 bytes) live at st.v[4..6] (v2 low,high), v3 at v[6..8], etc.
        let a = [st.v[0], st.v[1]];
        let b = [st.v[2], st.v[3]];
        let mut sum = [0u8; 16];
        let mut and = [0u8; 16];
        for i in 0..16 {
            let ai = (a[i / 8] >> ((i % 8) * 8)) as u8;
            let bi = (b[i / 8] >> ((i % 8) * 8)) as u8;
            sum[i] = ai.wrapping_add(bi);
            and[i] = ai & bi;
        }
        let vadd_lo = st.v[4]; // v2 low 8B
        let vadd_hi = st.v[5]; // v2 high 8B
        let vand_lo = st.v[8]; // v4 low 8B (v4 = reg index 4 -> st.v[2*4]=v[8])
        let vand_hi = st.v[9]; // v4 high 8B
        for i in 0..16 {
            let val = if i < 8 { vadd_lo } else { vadd_hi };
            let vnl = if i < 8 { vand_lo } else { vand_hi };
            let got_add = (val >> ((i % 8) * 8)) as u8 & 0xff;
            let got_and = (vnl >> ((i % 8) * 8)) as u8 & 0xff;
            assert_eq!(got_add, sum[i], "add v2.16b lane {i}");
            assert_eq!(got_and, and[i], "and v4.16b lane {i}: got={got_and:#04x} exp={:02x}", and[i]);
        }
        // also verify orr v5 and eor v6 and bic v7 read off the right slots.
        let orr_lo = st.v[10];
        let orr_hi = st.v[11];
        let eor_lo = st.v[12];
        let eor_hi = st.v[13];
        let bic_lo = st.v[14];
        let bic_hi = st.v[15];
        for i in 0..16 {
            let ai = (a[i / 8] >> ((i % 8) * 8)) as u8;
            let bi = (b[i / 8] >> ((i % 8) * 8)) as u8;
            let sel = if i < 8 { 0 } else { 1 };
            let o = if sel == 0 { orr_lo } else { orr_hi };
            let e = if sel == 0 { eor_lo } else { eor_hi };
            let c = if sel == 0 { bic_lo } else { bic_hi };
            assert_eq!((o >> ((i % 8) * 8)) as u8 & 0xff, ai | bi, "orr v5.16b lane {i}");
            assert_eq!((e >> ((i % 8) * 8)) as u8 & 0xff, ai ^ bi, "eor v6.16b lane {i}");
            assert_eq!((c >> ((i % 8) * 8)) as u8 & 0xff, ai & !bi, "bic v7.16b lane {i}: got {:02x}", (c >> ((i % 8) * 8)) as u8 & 0xff);
        }
    }

    #[test]
    fn sha1_round_correct_reference() {
        let rol = |x: u32, n: u32| x.rotate_left(n);
        let ror = |x: u32, n: u32| x.rotate_right(n);
        let cho = |x: u32, y: u32, z: u32| (x & (y ^ z)) ^ z;

        // sha1h S1,S2 : 0x5e280800 | (rn=2<<5) | rd=1 = 0x5e280841. S2.word0 = 0x12345678.
        let mut st = CpuState::new();
        st.v[4] = 0x1234_5678; // vector reg 2 (s2) lives at st.v[2*2]
        let mut code = Vec::new();
        code.extend_from_slice(&0x5e28_0841u32.to_le_bytes());
        code.extend_from_slice(&0xd65f03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec sha1h");
        assert_eq!((st.v[2] & 0xffff_ffff) as u32, ror(0x1234_5678, 2), "sha1h");

        // sha1c q0, s1, v4.4s : state {A,B,C,D}=v0, E=s1(word0), message=v4.
        let h = [0x6745_2301u32, 0xEFCD_AB89u32, 0x98BA_DCFEu32, 0x1032_5476u32, 0xC3D2_E1F0u32];
        let mut st2 = CpuState::new();
        st2.v[0] = ((h[1] as u64) << 32) | h[0] as u64; // A,B
        st2.v[1] = ((h[3] as u64) << 32) | h[2] as u64; // C,D
        st2.v[2] = h[4] as u64; // E (s1 word0 = st.v[2], reg 1)
        let msg = [0x6162_6380u32, 0x0000_0001u32, 0x0000_0000u32, 0x0000_0000u32];
        st2.v[8] = ((msg[1] as u64) << 32) | msg[0] as u64; // vector reg 4 (rm)
        st2.v[9] = ((msg[3] as u64) << 32) | msg[2] as u64; // vector reg 4 (rm)
        // sha1c q0, s1, v4.4s : 0x5e00_0000 | rm=4<<16 | rn=1<<5 | rd=0
        let w = 0x5e00_0000u32 | (4u32 << 16) | (1u32 << 5) | 0u32;
        let mut code2 = Vec::new();
        code2.extend_from_slice(&w.to_le_bytes());
        code2.extend_from_slice(&0xd65f03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st2, &code2, 0).expect("exec sha1c");

        let mut d = [h[0], h[1], h[2], h[3]];
        let mut nn = h[4];
        for i in 0..4 {
            let t = cho(d[1], d[2], d[3])
                .wrapping_add(rol(d[0], 5))
                .wrapping_add(nn)
                .wrapping_add(msg[i]);
            nn = d[3];
            d[3] = d[2];
            d[2] = ror(d[1], 2);
            d[1] = d[0];
            d[0] = t;
        }
        let got = [
            (st2.v[0] & 0xffff_ffff) as u32,
            ((st2.v[0] >> 32) & 0xffff_ffff) as u32,
            (st2.v[1] & 0xffff_ffff) as u32,
            ((st2.v[1] >> 32) & 0xffff_ffff) as u32,
        ];
        assert_eq!(got, [d[0], d[1], d[2], d[3]], "sha1c 4-round Ch");
    }

    #[test]
    fn add_carry_reference() {
        // adc w12, w14, w11 = 0x1a0b01cc  (rm=11, rn=14, rd=12). carry C=1
        // (NZCV bit29).  0 + 10 + 1 = 11.
        let mut st = CpuState::new();
        st.x[14] = 0;
        st.x[11] = 10;
        st.nzcv = 0x2000_0000; // C=1
        let mut code = Vec::new();
        code.extend_from_slice(&0x1a0b_01ccu32.to_le_bytes());
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec adc");
        assert_eq!(st.x[12], 11, "adc w: 0 + 10 + C(1) = 11");

        // carry clear: 0 + 10 + 0 = 10
        let mut st2 = CpuState::new();
        st2.x[14] = 0;
        st2.x[11] = 10;
        st2.nzcv = 0; // C=0
        let mut code2 = Vec::new();
        code2.extend_from_slice(&0x1a0b_01ccu32.to_le_bytes());
        code2.extend_from_slice(&0xd65f_03c0u32.to_le_bytes());
        exec_bytes(&mut st2, &code2, 0).expect("exec adc c-0");
        assert_eq!(st2.x[12], 10, "adc w: 0 + 10 + 0 = 10");

        // sbc w12, w14, w11 = 0x5a0b01cc: 100 - 40 - (1-C). C=1 -> 100-40-0 = 60.
        let mut st3 = CpuState::new();
        st3.x[14] = 100;
        st3.x[11] = 40;
        st3.nzcv = 0x2000_0000; // C=1
        let mut code3 = Vec::new();
        code3.extend_from_slice(&0x5a0b_01ccu32.to_le_bytes());
        code3.extend_from_slice(&0xd65f_03c0u32.to_le_bytes());
        exec_bytes(&mut st3, &code3, 0).expect("exec sbc");
        assert_eq!(st3.x[12], 60, "sbc w C=1: 100 - 40 - 0 = 60");
    }

    #[test]
    fn fmaxv_reduce_reference() {
        // fmaxv s1, v0.4s = 0x6e30f801 : max of the 4 single lanes of V0 -> S1.
        let mut st = CpuState::new();
        // lanes: [1.0, 5.5, -2.25, 3.0]; max = 5.5 (0x40b0_0000).
        st.v[0] = 0x40b0_0000_3f80_0000u64; // lanes 0,1
        st.v[1] = 0x4040_0000_c010_0000u64; // lanes 2,3
        let mut code = Vec::new();
        code.extend_from_slice(&0x6e30_f801u32.to_le_bytes());
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fmaxv");
        assert_eq!((st.v[2] & 0xffff_ffff) as u32, 0x40b0_0000, "fmaxv -> 5.5");
    }

    #[test]
    fn fmla_macc_lanes_reference() {
        // fmla v0.4s, v1.4s, v2.4s = 0x4e22cc20. Vd += Vn*Vm per lane.
        // v1=[2,3,4,5] v2=[3,2,4,2] v0=[1,1,1,1] => v0=[7,7,17,11].
        let f = |x: f32| x.to_bits() as u64;
        let pack = |lo: u64, hi: u64| (hi << 32) | lo;
        let mut st = CpuState::new();
        st.v[0] = (f(1.0) << 32) | f(1.0);        // v0 lanes 0,1
        st.v[1] = (f(1.0) << 32) | f(1.0);        // v0 lanes 2,3
        st.v[2] = (f(3.0) << 32) | f(2.0);        // v1 lanes 0,1
        st.v[3] = (f(5.0) << 32) | f(4.0);        // v1 lanes 2,3
        st.v[4] = (f(2.0) << 32) | f(3.0);        // v2 lanes 0,1
        st.v[5] = (f(2.0) << 32) | f(4.0);        // v2 lanes 2,3
        let mut code = Vec::new();
        code.extend_from_slice(&0x4e22_cc20u32.to_le_bytes()); // fmla v0.4s,v1.4s,v2.4s
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fmla .4s");
        let lanes = [ (st.v[0] & 0xffff_ffff) as u32, (st.v[0] >> 32) as u32,
                      (st.v[1] & 0xffff_ffff) as u32, (st.v[1] >> 32) as u32 ];
        let exp: Vec<u32> = [7.0f32,7.0,17.0,11.0].iter().map(|x| x.to_bits()).collect();
        for i in 0..4 { assert_eq!(lanes[i], exp[i], "fmla lane {}", i); }
    }

    #[test]
    fn fmla_by_element_reference() {
        // fmla v29.4s, v21.4s, v2.s[0] = 0x4f8212bd. v29[j] += v21[j]*v2.s[0].
        // v21=[1,2,3,4], v2.s[0]=10, v29=[0,0,0,0] => v29=[10,20,30,40].
        let f = |x: f32| x.to_bits() as u32;
        let mut st = CpuState::new();
        st.v[42] = ((f(2.0) as u64) << 32) | f(1.0) as u64;  // v21 lanes 0,1
        st.v[43] = ((f(4.0) as u64) << 32) | f(3.0) as u64;  // v21 lanes 2,3
        st.v[4] = f(10.0) as u64;                             // v2.s[0] (lane0)
        st.v[58] = 0; st.v[59] = 0;                            // v0 acc = 0
        let mut code = Vec::new();
        code.extend_from_slice(&0x4f82_12bdu32.to_le_bytes()); // fmla v29.4s,v21,v2.s[0]
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fmla by-element");
        let lanes = [(st.v[58]&0xffff_ffff) as u32,(st.v[58]>>32) as u32,
                     (st.v[59]&0xffff_ffff) as u32,(st.v[59]>>32) as u32];
        let exp: Vec<u32> = [10.0f32, 20.0, 30.0, 40.0]
            .iter()
            .map(|x| x.to_bits())
            .collect();
        for i in 0..4 { assert_eq!(lanes[i], exp[i], "fmla-el lane {}", i); }
    }

    #[test]
    fn shll_widen_sign_extend() {
        // shll v1.2d, v1.2s, #32 (wall 0x2ea13820): widen v1's 2 low .s elements to
        // 2 .d elements, sign-extended. v1=[-7, 0x40000000] => v1.2d =[-7, 0x40000000].
        let mut st = CpuState::new();
        // v1 (reg 1): st.v[2]=low64, st.v[3]=high64. Load 4x32-bit: lanes0=-7,1=1<<30.
        st.v[2] = ((0x4000_0000u64) << 32) | (0xffff_fffcu64); // lane0=-4, lane1=0x40000000
        st.v[3] = 0;
        let mut code = Vec::new();
        code.extend_from_slice(&0x2ea1_3821u32.to_le_bytes()); // shll v1.2d,v1.2s,#32
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec shll .2d");
        // dst v1 = st[2] (lane0) and st[3] (lane1) — BUT destination now widened 2xd.
        assert_eq!(st.v[2] as i64, -4i64, "shll lane0 sign-ext");
        assert_eq!(st.v[3], 0x4000_0000u64, "shll lane1 sign-ext");
    }

    #[test]
    fn fnmul_scalar_negate_mul() {
        // fnmul s10, s0, s1 = 0x1e21880a (wall): s10 = -(s0*s1).
        let f = |x: f32| x.to_bits() as u64;
        let mut st = CpuState::new();
        st.v[0] = f(2.5);       // s0 = v0 lane0
        st.v[2] = f(4.0);       // s1 = v1 lane0
        let mut code = Vec::new();
        code.extend_from_slice(&0x1e21_880au32.to_le_bytes()); // fnmul s10,s0,s1
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fnmul");
        assert_eq!(f32::from_bits((st.v[20] & 0xffff_ffff) as u32), -10.0, "fnmul -(2.5*4.0)");
    }

    #[test]
    fn fcvtms_floor_to_int() {
        // fcvtms w8, s5 = 0x1e3000a8 (wall): w8 = floor(s5). -1.5 -> -2.
        let f = |x: f32| x.to_bits() as u64;
        let mut st = CpuState::new();
        st.v[10] = f(-1.5);       // s5 = v5 lane0 (st.v[10], reg5)
        let mut code = Vec::new();
        code.extend_from_slice(&0x1e30_00a8u32.to_le_bytes()); // fcvtms w8,s5
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fcvtms");
        assert_eq!(st.x[8] as i32, -2, "fcvtms floor(-1.5) = -2");
    }

    #[test]
    fn smin_signed_lane_min() {
        // smin v0.2s, v0.2s, v1.2s (wall 0x0ea16c00): v0[i] = min_signed(v0[i], v1[i]).
        let mut st = CpuState::new();
        st.v[0] = ((0xffff_fffdu64) << 32) | 5u64; // v0 lanes: [5, -3]
               st.v[2] = ((7u64) << 32) | 2u64; // v1 (reg1) lanes: [2, 7]
               let mut code = Vec::new();
               code.extend_from_slice(&0x0ea1_6c00u32.to_le_bytes()); // smin v0.2s,v0.2s,v1.2s
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec smin .2s");
        // v0 lanes: min(5,2)=2, min(-3,7)=-3
        assert_eq!((st.v[0] & 0xffff_ffff) as i32, 2,  "v0.l0 min(5,2)=2");
        assert_eq!((st.v[0] >> 32) as i32, -3, "v0.l1 min(-3,7)=-3");
    }

    #[test]
    fn guest_svc_routes_write_and_mmap() {
        // Directly exercise the AArch64->host syscall dispatcher (AArch64 numbers):
        //   nr=64 write(fd, buf, n) to a pipe, and nr=222 mmap(len,...) returning real mem.
        let mut st = CpuState::new();
        let msg = b"hello-svc";
        // pipe so write is observable without corrupting stdout
        let mut pfd = [0; 2];
        unsafe { assert_eq!(libc::pipe(pfd.as_mut_ptr()), 0); }
        st.x[8] = 64;            // AArch64 write
        st.x[0] = pfd[1] as u64; // fd = write end
        st.x[1] = msg.as_ptr() as u64;
        st.x[2] = msg.len() as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        // write returns bytes written (== len) — NOT -errno.
        assert_eq!(r as isize, msg.len() as isize, "write syscall count");
        let mut buf = [0u8; 64];
        let n = unsafe { libc::read(pfd[0], buf.as_mut_ptr() as *mut libc::c_void, 64) };
        assert_eq!(n as usize, msg.len());
        assert_eq!(&buf[..msg.len()], msg, "write->read roundtrip");
        unsafe { libc::close(pfd[0]); libc::close(pfd[1]); }

        // mmap (AArch64 222): map 4096 RW anonymous at addr=NULL.
        st.x[8] = 222;
        st.x[0] = 0;                                    // addr
        st.x[1] = 4096;                                 // length
        st.x[2] = libc::PROT_READ as u64 | libc::PROT_WRITE as u64;
        st.x[3] = (libc::MAP_PRIVATE | libc::MAP_ANONYMOUS) as u64;
        st.x[4] = -1i64 as u64;                          // fd = -1
        st.x[5] = 0;                                     // offset
        let m = guest_svc(&mut st as *mut CpuState);
        assert!(m != 0 && (m as u64) < 0x8000_0000_0000_0000, "mmap returned host ptr {:#x}", m);
        unsafe { std::ptr::write_volatile(m as *mut u8, 0xabu8); }
        assert_eq!(unsafe { std::ptr::read_volatile(m as *const u8) }, 0xabu8, "mmap writable");
        unsafe { libc::munmap(m as *mut libc::c_void, 4096); }

        // getpid (AArch64 172) -> real host pid
        st.x[8] = 172;
        let pid = guest_svc(&mut st as *mut CpuState);
        assert_eq!(pid as u32, std::process::id());
    }

    #[test]
    fn host_call_bridge_blr_into_host_local() {
        // Guest->host bridge through jit_run's dispatcher: a guest `blr x16`
        // where x16 = host_call_addr(1) must invoke our registered host local
        // function (x0..x7 args; host ret -> guest x0) and resume at x30.
        extern "C" fn times_three(
            a0: u64,
            _a1: u64,
            _a2: u64,
            _a3: u64,
            _a4: u64,
            _a5: u64,
            _a6: u64,
            _a7: u64,
        ) -> u64 {
            a0.wrapping_mul(3)
        }

        let host = host_call_addr(1);
        register_host_call(1, times_three);
        let mut img: Vec<u8> = Vec::new();
        img.extend_from_slice(&0xd28000a0u32.to_le_bytes()); // movz x0,#5
        img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
        img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0 -> halt (pc=0)
        let mut st = CpuState::new();
        st.x[16] = host; // x16 = host thunk slot address (bridge target)
        let r = jit_run(&img, 0x1000, 0x1000, &mut st as *mut CpuState).expect("jit_run");
        assert_eq!(r, 15, "host call times_3(5) via blr-through-dispatcher");
    }

    #[test]
    fn host_float_call_bridge_atan2_via_blr() {
        // Float-ABI bridge through the dispatcher: a guest `blr x16` where x16 =
        // a registered float thunk reads guest v0/v1 (as f64) and the host f64
        // return lands back in guest v0.
        extern "C" fn host_atan2(y: f64, x: f64, _a: f64, _b: f64, _c: f64, _d: f64, _e: f64, _f: f64) -> f64 {
            // host libc atan2 (double via xmm0/xmm1) = Rust f64::atan2
            y.atan2(x)
        }
        let fslot = register_float_call(host_atan2);
        let mut img: Vec<u8> = Vec::new();
        img.extend_from_slice(&0xd2800000u32.to_le_bytes()); // movz x16,#0 (placeholder; x16 host-set)
        img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
        img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0 -> halt
        let mut st = CpuState::new();
        st.v[0] = 1.0f64.to_bits(); // v0.d = y (arg0)
        st.v[2] = 0.0f64.to_bits(); // v1.d = x (arg1)  -> atan2(1,0)=pi/2
        st.x[16] = fslot;
        let _ = jit_run(&img, 0x2000, 0x2000, &mut st as *mut CpuState).expect("jit_run");
        let got = f64::from_bits(st.v[0]);
                assert!(
                    (got - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
                    "float bridge atan2(1,0) = {got} != pi/2"
                );
            }

            #[test]
            fn host_float32_call_bridge_atan2f_via_blr() {
                // Single-precision float bridge: guest `blr` to an f32 thunk reads the
                // low 32 bits of s0/s1 (v0/v1), widens to f32, calls the host f32 fn,
                // narrows the f32 result into s0.
                extern "C" fn host_atan2f(
                    y: f32,
                    x: f32,
                    _a: f32,
                    _b: f32,
                    _c: f32,
                    _d: f32,
                    _e: f32,
                    _f: f32,
                ) -> f32 {
                    y.atan2(x)
                }
                let fslot = register_float32_call(host_atan2f);
                let mut img: Vec<u8> = Vec::new();
                img.extend_from_slice(&0xd2800000u32.to_le_bytes()); // movz w0,#0 (placeholder)
                img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
                img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0 -> halt
                let mut st = CpuState::new();
                st.v[0] = 1.0f32.to_bits() as u64; // s0 = y (low 32)
                st.v[2] = 0.0f32.to_bits() as u64; // s1 = x (low 32) -> atan2f(1,0)=pi/2
                st.x[16] = fslot;
                let _ = jit_run(&img, 0x2000, 0x2000, &mut st as *mut CpuState).expect("jit_run");
                let got = f32::from_bits((st.v[0] & 0xffff_ffff) as u32);
                assert!(
                    (got - std::f32::consts::FRAC_PI_2).abs() < 1e-6,
                    "f32 bridge atan2f(1,0) = {got} != pi/2"
                );
            }
        }
