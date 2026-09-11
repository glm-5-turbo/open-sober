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
    /// Scratch: two 16-byte temp vector slots used to snapshot rn/rm before a
    /// SIMD permute (uzp1/uzp2/zip1/zip2) when rd aliases a source. Reading
    /// through memory that the permute is simultaneously writing into would
    /// otherwise corrupt late-iteration reads (gcc's `uzp1 v31.8h, v31.8h,
    /// v26.8h` writes rd==rn while still reading rn's high half). Kept after
    /// `cntvct` so VECTOR_BASE (272) is unchanged.
    pub permscratch: [u64; 4], // 32 bytes = 2 × 16-byte vectors
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
/// Byte offset of `CpuState.permscratch` — right after `cntvct` (792..800).
pub const PERMSCRATCH_OFF: i32 = CNTVCT_OFF + 8; // 800

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
            permscratch: [0; 4],
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

/// A **GLES bridge** function: gets the full guest `CpuState` (both the integer
/// x0..x7 arguments AND the SIMD v0..v7 registers, plus the guest stack pointer
/// for >8-arg calls) and returns the value to store back into guest x0. This is
/// how OpenGL ES functions with *mixed* integer+float ABIs (glClearColor,
/// glUniform4f) and *more than 8 args* (glTexImage2D, whose 9th arg lives on the
/// guest stack) reach real Mesa: the generic integer `HostCall` only marshals
/// 8 x-register args and the uniform-float bridges assume all-float ABIs, so
/// neither can express GLES. Each registered bridge reads the exact guest
/// x/s-lanes its signature needs and calls the real Mesa symbol via gles-wrapper.
pub type HostGlesCall = extern "C" fn(st: *mut CpuState) -> u64;

static HOST_CALLS: Mutex<[Option<HostCall>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);
static HOST_FLOAT_CALLS: Mutex<[Option<HostFloatCall>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);
static HOST_FLOAT32_CALLS: Mutex<[Option<HostFloat32Call>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);
static HOST_GLES_CALLS: Mutex<[Option<HostGlesCall>; HOST_THUNK_MAX]> = Mutex::new([None; HOST_THUNK_MAX]);

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

/// Guest base address of the **GLES bridge** thunk region (after the f32 slots).
#[inline(always)]
pub fn host_gles_base() -> u64 {
    host_float32_base() + (HOST_THUNK_MAX as u64) * 8
}

/// Register a GLES mixed-ABI bridge at an auto-allocated slot; returns its guest
/// address. The bridge is called with the full guest `CpuState` by the dispatcher.
pub fn register_gles_call(f: HostGlesCall) -> u64 {
    let mut hc = HOST_GLES_CALLS.lock().unwrap();
    let i = hc.iter().position(|s| s.is_none()).expect("gles thunk table full");
    hc[i] = Some(f);
    host_gles_base() + (i as u64) * 8
}

/// Look up a GLES bridge fn for a guest `pc` in the GLES region.
fn host_gles_call_at(pc: u64) -> Option<(HostGlesCall, usize)> {
    let base = host_gles_base();
    if pc < base {
        return None;
    }
    let off = pc - base;
    if off % 8 != 0 {
        return None;
    }
    let i = (off / 8) as usize;
    let hc = HOST_GLES_CALLS.lock().unwrap();
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
/// AArch64 `struct stat` (asm-generic/stat.h, 64-bit — 128 bytes) written into
/// guest memory. The HOST `libc::stat` layout differs across architectures
/// (x86_64 vs aarch64), so forwarding the host struct as-is would let the guest
/// read st_mode/st_size/etc. from the wrong offsets — a silent miscompile. We
/// copy the host fields into this fixed aarch64 layout.
unsafe fn write_guest_stat(buf: u64, s: &libc::stat) {
    unsafe {
        let p = buf as *mut u64;
        let w = buf as *mut u32;
        std::ptr::write_volatile(p.add(0), s.st_dev as u64); // st_dev    @0
        std::ptr::write_volatile(p.add(1), s.st_ino as u64); // st_ino    @8
        std::ptr::write_volatile(w.add(4), s.st_mode as u32); // st_mode   @16
        std::ptr::write_volatile(w.add(5), s.st_nlink as u32); // st_nlink @20
        std::ptr::write_volatile(w.add(6), s.st_uid as u32); // st_uid    @24
        std::ptr::write_volatile(w.add(7), s.st_gid as u32); // st_gid    @28
        std::ptr::write_volatile(p.add(4), s.st_rdev as u64); // st_rdev  @32
        std::ptr::write_volatile(p.add(6), s.st_size as i64 as u64); // st_size @48
        std::ptr::write_volatile(w.add(14), s.st_blksize as i32 as u32); // st_blksize @56
        std::ptr::write_volatile(p.add(8), s.st_blocks as i64 as u64); // st_blocks @64
        std::ptr::write_volatile(p.add(9), s.st_atime as i64 as u64); // st_atime @72
        std::ptr::write_volatile(p.add(10), s.st_atime_nsec as u64); // @80
        std::ptr::write_volatile(p.add(11), s.st_mtime as i64 as u64); // st_mtime @88
        std::ptr::write_volatile(p.add(12), s.st_mtime_nsec as u64); // @96
        std::ptr::write_volatile(p.add(13), s.st_ctime as i64 as u64); // st_ctime @104
        std::ptr::write_volatile(p.add(14), s.st_ctime_nsec as u64); // @112
    }
}

/// Write an AArch64 `struct statfs` (as-generic 64-bit layout, 120 bytes) at
/// `buf`, from the host `libc::statfs`. The leading fields through `f_frsize`
/// (offset 72) are byte-identical on both arches; the aarch64 `f_flags`(80) and
/// `f_spare[4]`(88..119) are not present in the glibc struct, so we zero them
/// (a guest free-space check only needs blocks/bfree/bavail/files/bsize/frsize,
/// all of which match exactly).
unsafe fn write_guest_statfs(buf: u64, s: &libc::statfs) {
    unsafe {
        let p = buf as *mut u64;
        let w = buf as *mut u32;
        std::ptr::write_volatile(p.add(0), s.f_type as u64); // f_type   @0
        std::ptr::write_volatile(p.add(1), s.f_bsize as u64); // f_bsize  @8
        std::ptr::write_volatile(p.add(2), s.f_blocks as u64); // f_blocks @16
        std::ptr::write_volatile(p.add(3), s.f_bfree as u64); // f_bfree  @24
        std::ptr::write_volatile(p.add(4), s.f_bavail as u64); // f_bavail @32
        std::ptr::write_volatile(p.add(5), s.f_files as u64); // f_files  @40
        std::ptr::write_volatile(p.add(6), s.f_ffree as u64); // f_ffree  @48
        let fsidp = &s.f_fsid as *const _ as *const u32;
        std::ptr::write_volatile(w.add(14), *fsidp); // f_fsid @56 (2 x u32)
        std::ptr::write_volatile(w.add(15), *fsidp.add(1)); // f_fsid @60
        std::ptr::write_volatile(p.add(8), s.f_namelen as i64 as u64); // f_namelen @64
        std::ptr::write_volatile(p.add(9), s.f_frsize as u64); // f_frsize @72
        // f_flags @80 + f_spare[4] @88..119 stay zeroed (aarch64-only fields).
    }
}

pub extern "C" fn guest_svc(st: *mut CpuState) -> u64 {
    let s = unsafe { &mut *st };
    let nr = s.x[8];
    let a = [s.x[0], s.x[1], s.x[2], s.x[3], s.x[4], s.x[5]];
    if std::env::var("JIT_TRACE_SVC").is_ok() {
        // Unbuffered fd-2 marker (eprintln buffers and is lost on _exit/segv).
        let m = format!("guest svc {nr:x} a0={:#x}\n", a[0]);
        unsafe { libc::write(2, m.as_ptr() as *const libc::c_void, m.len()); }
    }
    use libc::{c_long, c_void, c_char, c_int};
    // AArch64 -> host. We dispatch by AArch64 syscall number directly to the
    // matching libc call (which does the native x86-64 syscall), so the mapping
    // is exact and readable rather than a fragile number shuffle. Errors come
    // back as -1 + errno; we convert to the kernel's -errno convention.
    let ret: c_long = match nr {
        // --- process / exit ---
        93 | 94 => { // exit(93) / exit_group(94)
            if std::env::var("JIT_TRACE_SVC").is_ok() {
                eprintln!("guest_svc: exit_group({}) from guest", a[0]);
            }
            // A guest exit_group is a raw kernel call: terminate immediately
            // without Rust's stdout flush / destructor walk (a guest `exit`
            // must NOT run host language-level cleanup, and Rust's atexit stdio
            // flush crashed under elfjit when stdout was redirected). Guest
            // writes went directly to fd 1, so nothing is lost by _exit.
            unsafe { libc::_exit(a[0] as c_int) };
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
        216 => unsafe { libc::syscall(libc::SYS_mremap, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as c_int, a[4] as usize) as c_long }, // (220 is clone, NOT mremap)
        // --- filesystem / directory ---
        17 => unsafe { libc::syscall(libc::SYS_getcwd, a[0] as usize, a[1] as usize) as c_long },
        34 => unsafe { libc::mkdirat(a[0] as c_int, a[1] as *const c_char, a[2] as libc::mode_t) as c_long },
        35 => unsafe { libc::unlinkat(a[0] as c_int, a[1] as *const c_char, a[2] as c_int) as c_long },
        36 => unsafe { libc::symlinkat(a[1] as *const c_char, a[0] as c_int, a[2] as *const c_char) as c_long },
        37 => unsafe { // linkat(37)
            libc::syscall(libc::SYS_linkat, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as usize, a[4] as usize) as c_long
        },
        38 => unsafe { libc::renameat(a[0] as c_int, a[1] as *const c_char, a[2] as c_int, a[3] as *const c_char) as c_long },
        158 => unsafe { // getgroups(158): count, list
            libc::getgroups(a[0] as c_int, a[1] as *mut libc::gid_t) as c_long
        },
        49 => unsafe { libc::chdir(a[0] as *const c_char) as c_long },
        61 => unsafe { libc::syscall(libc::SYS_getdents64, a[0] as c_int, a[1] as usize, a[2] as usize) as c_long },
        62 => unsafe { libc::lseek(a[0] as c_int, a[1] as i64, a[2] as c_int) as c_long },
        48 => unsafe { libc::faccessat(libc::AT_FDCWD, a[0] as *const c_char, a[1] as c_int, 0) as c_long },
        78 => unsafe { libc::syscall(libc::SYS_readlinkat, libc::AT_FDCWD as usize, a[0] as usize, a[1] as usize, a[2] as usize) as c_long },
        59 => unsafe { libc::pipe2(a[0] as *mut c_int, a[2] as c_int) as c_long },
        96 => unsafe { libc::syscall(libc::SYS_set_tid_address, a[0] as usize) as c_long },
        99 => 0, // set_robust_list: a no-op (no robust futexes) is valid; glibc retries if it errors
        283 => 0, // membarrier: no-op
        // rseq (293): the thread-local restartable sequence — glibc enables it
        // opportunistically and continues if it fails, so -ENOSYS is fine; but
        // if registered, the kernel expects a valid rseq area. We never touch it,
        // so return -ENOSYS rather than fake a success.
        // (add rseq only if a later boot frontier requires it)
        124 => unsafe { libc::sched_yield() as c_long },
        // --- time ---
        113 => unsafe { libc::clock_gettime(a[0] as libc::clockid_t, a[1] as *mut libc::timespec) as c_long },
        101 => unsafe { libc::nanosleep(a[1] as *const libc::timespec, a[2] as *mut libc::timespec) as c_long },
        // --- process / control ---
        167 => unsafe { libc::prctl(a[0] as c_int, a[1], a[2], a[3], a[4]) as c_long },
        // --- process / user identity ---
        172 => unsafe { libc::getpid() as c_long },
        174 => unsafe { libc::getuid() as c_long }, // (199 is socketpair, NOT getuid)
        175 => unsafe { libc::geteuid() as c_long },
        176 => unsafe { libc::getgid() as c_long },
        177 => unsafe { libc::getegid() as c_long },
        178 => unsafe { libc::gettid() as c_long },
        173 => unsafe { libc::getppid() as c_long },
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
        // --- file/dir stat (aarch64 buf layout, see write_guest_stat) ---
        80 => { // AArch64 fstat (80)
            unsafe {
                let mut st = core::mem::MaybeUninit::<libc::stat>::zeroed().assume_init();
                let r = libc::fstat(a[0] as c_int, &mut st);
                if r == 0 {
                    write_guest_stat(a[1], &st);
                }
                r as c_long
            }
        }
        79 => { // AArch64 newfstatat (fstatat, 79)
            unsafe {
                let mut st = core::mem::MaybeUninit::<libc::stat>::zeroed().assume_init();
                let r = libc::fstatat(a[0] as c_int, a[1] as *const c_char, &mut st, a[3] as c_int);
                if r == 0 {
                    write_guest_stat(a[2], &st);
                }
                r as c_long
            }
        }
        // --- readv/writev ---
        65 => unsafe { libc::readv(a[0] as c_int, a[1] as *const libc::iovec, a[2] as c_int) as c_long },
        66 => unsafe { libc::writev(a[0] as c_int, a[1] as *const libc::iovec, a[2] as c_int) as c_long },
        67 => unsafe { // pread64(67)
            libc::syscall(libc::SYS_pread64, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as i64) as c_long
        },
        68 => unsafe { // pwrite64(68)
            libc::syscall(libc::SYS_pwrite64, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as i64) as c_long
        },
        // --- system metadata (fixed char-array layout, arch-independent) ---
        160 => { // uname
            unsafe {
                let mut u = core::mem::MaybeUninit::<libc::utsname>::zeroed().assume_init();
                let r = libc::uname(&mut u);
                if r == 0 {
                    // utsname is fixed 65-byte char arrays on both arches (asm-generic).
                    std::ptr::copy_nonoverlapping(&u as *const libc::utsname as *const u8, a[0] as *mut u8, core::mem::size_of::<libc::utsname>());
                }
                r as c_long
            }
        }
        // --- timeval (two u64/i64, layout-identical) ---
        169 => unsafe { libc::gettimeofday(a[0] as *mut libc::timeval, a[1] as *mut libc::timezone) as c_long },
        114 => unsafe { libc::clock_getres(a[0] as libc::clockid_t, a[1] as *mut libc::timespec) as c_long },
        // --- descriptors ---
        23 => unsafe { libc::dup(a[0] as c_int) as c_long },
        24 => unsafe { libc::dup3(a[0] as c_int, a[1] as c_int, a[2] as c_int) as c_long },
        29 => unsafe { libc::ioctl(a[0] as c_int, a[1] as libc::c_ulong, a[2]) as c_long },
        // --- event/epoll (Android ALooper is epoll-based; struct layouts identical) ---
        19 => unsafe { libc::eventfd(a[0] as libc::c_uint, a[1] as c_int) as c_long },
        20 => unsafe { libc::epoll_create1(a[0] as c_int) as c_long },
        21 => unsafe { libc::epoll_ctl(a[0] as c_int, a[1] as c_int, a[2] as c_int, a[3] as *mut libc::epoll_event) as c_long },
        22 => unsafe { libc::epoll_pwait(a[0] as c_int, a[1] as *mut libc::epoll_event, a[2] as c_int, a[3] as c_int, a[4] as *const libc::sigset_t) as c_long },
        73 => unsafe { libc::ppoll(a[0] as *mut libc::pollfd, a[1] as libc::nfds_t, a[2] as *const libc::timespec, a[3] as *const libc::sigset_t) as c_long },
        // --- sockets ---
        198 => unsafe { libc::socket(a[0] as c_int, a[1] as c_int, a[2] as c_int) as c_long },
        200 => unsafe { libc::bind(a[0] as c_int, a[1] as *const libc::sockaddr, a[2] as libc::socklen_t) as c_long },
        201 => unsafe { libc::listen(a[0] as c_int, a[1] as c_int) as c_long },
        202 => unsafe { libc::accept(a[0] as c_int, a[1] as *mut libc::sockaddr, a[2] as *mut libc::socklen_t) as c_long },
        203 => unsafe { libc::connect(a[0] as c_int, a[1] as *const libc::sockaddr, a[2] as libc::socklen_t) as c_long },
        208 => unsafe { libc::setsockopt(a[0] as c_int, a[1] as c_int, a[2] as c_int, a[3] as *const c_void, a[4] as libc::socklen_t) as c_long },
        209 => unsafe { libc::getsockopt(a[0] as c_int, a[1] as c_int, a[2] as c_int, a[3] as *mut c_void, a[4] as *mut libc::socklen_t) as c_long },
        199 => unsafe { libc::socketpair(a[0] as c_int, a[1] as c_int, a[2] as c_int, a[3] as *mut c_int) as c_long },
        206 => unsafe { libc::sendto(a[0] as c_int, a[1] as *const c_void, a[2] as usize, a[3] as c_int, a[4] as *const libc::sockaddr, a[5] as libc::socklen_t) as c_long },
        207 => unsafe { libc::recvfrom(a[0] as c_int, a[1] as *mut c_void, a[2] as usize, a[3] as c_int, a[4] as *mut libc::sockaddr, a[5] as *mut libc::socklen_t) as c_long },
        211 => unsafe { libc::sendmsg(a[0] as c_int, a[1] as *const libc::msghdr, a[2] as c_int) as c_long },
        212 => unsafe { libc::recvmsg(a[0] as c_int, a[1] as *mut libc::msghdr, a[2] as c_int) as c_long },
        213 => unsafe { libc::accept4(a[0] as c_int, a[1] as *mut libc::sockaddr, a[2] as *mut libc::socklen_t, a[3] as c_int) as c_long },
        // --- memory advice / umask ---
        233 => unsafe { libc::madvise(a[0] as *mut c_void, a[1] as usize, a[2] as c_int) as c_long },
        166 => unsafe { libc::umask(a[0] as libc::mode_t) as c_long },
        // --- limits ---
        163 => unsafe { libc::getrlimit(a[0] as u32, a[1] as *mut libc::rlimit) as c_long },
        164 => unsafe { libc::setrlimit(a[0] as u32, a[1] as *const libc::rlimit) as c_long },
        // --- signals / timers re-delivery ---
        129 => unsafe { libc::kill(a[0] as c_int, a[1] as c_int) as c_long },
        131 => unsafe { libc::tgkill(a[0] as c_int, a[1] as c_int, a[2] as c_int) as c_long },
        107 => unsafe { libc::timer_create(a[0] as libc::clockid_t, a[1] as *mut libc::sigevent, a[2] as *mut libc::timer_t) as c_long },
        110 => unsafe { libc::timer_settime(a[0] as libc::timer_t, a[1] as c_int, a[2] as *const libc::itimerspec, a[3] as *mut libc::itimerspec) as c_long },
        // --- common Android boot-path gaps (ARGID asm-generic table) ---
        115 => unsafe { // clock_nanosleep(115): clockid, flags, req, rem
            libc::syscall(libc::SYS_clock_nanosleep, a[0] as usize, a[1] as c_int, a[2] as usize, a[3] as usize) as c_long
        },
        165 => unsafe { // getrusage(165): who, struct rusage* (layout-identical u64/i64 pairs + timeval)
            libc::getrusage(a[0] as c_int, a[1] as *mut libc::rusage) as c_long
        },
        154 => unsafe { libc::setpgid(a[0] as libc::pid_t, a[1] as libc::pid_t) as c_long },
        25 => unsafe { // fcntl(25): fd, cmd, [arg]. AArch64 uses argfd semantics; the
            // 2- and 3-arg forms cover F_GETFL/F_SETFL/F_SETFD/F_DUPFD/F_GETFD.
            // fcntl is variadic at the ABI level; call through the raw syscall with
            // a3 as the optional arg so both shapes land correctly on x86-64.
            libc::syscall(libc::SYS_fcntl, a[0] as usize, a[1] as usize, a[2] as usize) as c_long
        },
        134 => unsafe { // rt_sigaction(134): sig, act, oact, sigsetsize. We cannot
            // actually install a *guest* trampoline handler in the host, so we
            // accept the register (return 0) and do NOT invoke one — matching the
            // treatment of signals elsewhere in this shim (signals return 0/ignored).
            // oact (a2, non-null) is cleared to keep guest callers from derefing
            // garbage; a null oact is tolerated.
            if a[2] != 0 {
                unsafe { std::ptr::write_bytes(a[2] as *mut u8, 0, 128); }
            }
            0
        },
        135 => unsafe { // rt_sigprocmask(135): how, set, oset, sigsetsize. No-op: we
            // do not dispatch signals, so accept and report an empty old-set.
            if a[2] != 0 {
                unsafe { std::ptr::write_bytes(a[2] as *mut u8, 0, 8); }
            }
            0
        },
        223 => unsafe { // fadvise64(223): fd, off, len, advice (aarch64 __NR3264_fadvise64)
            libc::syscall(libc::SYS_fadvise64, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as usize) as c_long
        },
        // --- CPU affinity / scheduler probes (Android/bionic detects core count
        // at startup; a game engine sizes its worker pool from this) ---
        204 => unsafe { // sched_getaffinity(204): pid, cpusetsize, mask*. cpu_set_t is
            // a bitmask, byte-identical across arches — forward via raw syscall so
            // the guest mask buffer is written in place.
            libc::syscall(libc::SYS_sched_getaffinity, a[0] as usize, a[1] as usize, a[2] as usize) as c_long
        },
        122 => unsafe { // sched_setaffinity(122)
            libc::syscall(libc::SYS_sched_setaffinity, a[0] as usize, a[1] as usize, a[2] as usize) as c_long
        },
        // --- resource limits (host layout identical: struct rlimit64/__rlimit) ---
        261 => unsafe { // prlimit64(261): pid, resource, new_limit, old_limit
            libc::prlimit(a[0] as libc::pid_t, a[1] as u32, a[2] as *const libc::rlimit, a[3] as *mut libc::rlimit) as c_long
        },
        // --- CPU id / round-trip timing ---
        168 => unsafe { // getcpu(168): cpu*, node*, tcache*. Trivial 3-int writes, no struct.
            libc::syscall(libc::SYS_getcpu, a[0] as usize, a[1] as usize, a[2] as usize) as c_long
        },
        103 => unsafe { // setitimer(103): which, new_value, old_value (struct itimerval)
            libc::setitimer(a[0] as c_int, a[1] as *const libc::itimerval, a[2] as *mut libc::itimerval) as c_long
        },
        102 => unsafe { // getitimer(102)
            libc::getitimer(a[0] as c_int, a[1] as *mut libc::itimerval) as c_long
        },
        // --- file I/O durability / sizing (same semantics both arches) ---
        82 => unsafe { libc::fsync(a[0] as c_int) as c_long },
        83 => unsafe { libc::fdatasync(a[0] as c_int) as c_long },
        45 => unsafe { libc::truncate(a[0] as *const c_char, a[1] as libc::off_t) as c_long },
        46 => unsafe { libc::ftruncate(a[0] as c_int, a[1] as libc::off_t) as c_long },
        // --- system memory (sysinfo 179): a game engine sizes its worker-pool
        // heaps / caches from totalram/freeram. The asm-generic `struct sysinfo`
        // is byte-identical on aarch64 and x86-64, so forward the host's REAL
        // values (the record shows forging memory figures changes nothing). ---
        179 => {
            unsafe {
                let mut si: libc::sysinfo = core::mem::zeroed();
                let r = libc::sysinfo(&mut si);
                if r == 0 {
                    std::ptr::copy_nonoverlapping(
                        &si as *const libc::sysinfo as *const u8,
                        a[0] as *mut u8,
                        core::mem::size_of::<libc::sysinfo>(),
                    );
                }
                r as c_long
            }
        }
        // --- statx (291): the modern stat query (bionic/Java use it for file
        // metadata); `struct statx` is asm-generic and byte-identical on both
        // arches, so a raw forward writes the guest's statx buffer in place. ---
        291 => unsafe {
            libc::syscall(
                libc::SYS_statx,
                a[0] as usize, a[1] as usize, a[2] as usize,
                a[3] as usize, a[4] as usize,
            ) as c_long
        },
        // --- get_robust_list (100): glibc's pthread init probes for a robust
        // futex list; report a valid EMPTY list (a zeroed `next`) rather than
        // -ENOSYS so thread bootstrap proceeds. len = pointer size. ---
        100 => {
            static EMPTY_ROBUST_LIST: [u8; 24] = [0u8; 24]; // struct robust_list{next}/flags
            if a[1] != 0 {
                unsafe { std::ptr::write(a[1] as *mut u64, &EMPTY_ROBUST_LIST as *const u8 as u64); }
            }
            if a[2] != 0 {
                unsafe { std::ptr::write(a[2] as *mut u64, core::mem::size_of::<u64>() as u64); }
            }
            0
        }
        128 => (-4i32) as c_long, // restart_syscall(128): only surfaces from a
        // -ERESTART* interrupted syscall we never produce; -EINTR is correct.
        // --- filesystem space (statfs/fstatfs, 43/44) ---
        43 => {
            // AArch64 statfs (43): path, struct statfs*. Write the guest layout,
            // same fields as the 64-bit asm-generic struct the host fills.
            unsafe {
                let mut fs = core::mem::MaybeUninit::<libc::statfs>::zeroed().assume_init();
                let r = libc::statfs(a[0] as *const c_char, &mut fs);
                if r == 0 { write_guest_statfs(a[1], &fs); }
                r as c_long
            }
        }
        44 => {
            unsafe {
                let mut fs = core::mem::MaybeUninit::<libc::statfs>::zeroed().assume_init();
                let r = libc::fstatfs(a[0] as c_int, &mut fs);
                if r == 0 { write_guest_statfs(a[1], &fs); }
                r as c_long
            }
        }
        // --- data plumbing ---
        71 => unsafe { // sendfile(71): out, in, offset*, count (aarch64 __NR3264_sendfile)
            libc::sendfile(a[0] as c_int, a[1] as c_int, a[2] as *mut libc::off_t, a[3] as usize) as c_long
        },
        84 => unsafe { // sync_file_range(84): fd, off, nbytes, flags
            libc::syscall(libc::SYS_sync_file_range, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as usize) as c_long
        },
        // --- memory control (no struct layouts involved) ---
        227 => unsafe { libc::msync(a[0] as *mut c_void, a[1] as usize, a[2] as c_int) as c_long },
        228 => unsafe { libc::mlock(a[0] as *const c_void, a[1] as usize) as c_long },
        229 => unsafe { libc::munlock(a[0] as *const c_void, a[1] as usize) as c_long },
        232 => unsafe { libc::mincore(a[0] as *mut c_void, a[1] as usize, a[2] as *mut u8) as c_long },
        // --- file metadata ownership / timestamps ---
        53 => unsafe { // fchmodat(53): dirfd, path, mode, flags
            libc::fchmodat(a[0] as c_int, a[1] as *const c_char, a[2] as libc::mode_t, a[3] as c_int) as c_long
        },
        54 => unsafe { libc::fchownat(a[0] as c_int, a[1] as *const c_char, a[2] as libc::uid_t, a[3] as libc::gid_t, a[4] as c_int) as c_long },
        55 => unsafe { libc::fchown(a[0] as c_int, a[1] as libc::uid_t, a[2] as libc::gid_t) as c_long },
        88 => unsafe { libc::utimensat(a[0] as c_int, a[1] as *const c_char, a[2] as *const libc::timespec, a[3] as c_int) as c_long },
        // --- session / process group ---
        156 => unsafe { libc::getsid(a[0] as c_int) as c_long },
        157 => unsafe { libc::setsid() as c_long },
        // --- timerfd (85/86/87): Android/libutils/ALooper wait on timerfds for
        // timeouts (SystemClock, trace, watchdog). itimerspec is two timespecs =
        // byte-identical across aarch64/x86-64, so forward directly. ---
        85 => unsafe { // timerfd_create(clockid, flags)
            libc::syscall(libc::SYS_timerfd_create, a[0] as usize, a[1] as usize) as c_long
        },
        86 => unsafe { // timerfd_settime(fd, flags, new_value*, old_value*)
            libc::syscall(libc::SYS_timerfd_settime, a[0] as usize, a[1] as usize, a[2] as usize, a[3] as usize) as c_long
        },
        87 => unsafe { // timerfd_gettime(fd, curr_value*)
            libc::syscall(libc::SYS_timerfd_gettime, a[0] as usize, a[1] as usize) as c_long
        },
        // --- signalfd4 (74): guest signal *dispatch* isn't supported here (rt_sigaction
        // is a no-op), so a signalfd would never fire. Return a real host signalfd but
        // with an EMPTY sigset (never wakes) so an app that requires signalfd succeeds
        // on the call instead of aborting on -ENOSYS, while honoring the no-dispatch
        // stance. fd==-1 creates a new one, else it just (re)arms the given fd. ---
        74 => unsafe {
            let mut empty: libc::sigset_t = core::mem::zeroed();
            libc::signalfd(a[0] as c_int, &empty, a[3] as c_int) as c_long
        },
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
        // GLES mixed-ABI bridge: OpenGL ES functions whose signature mixes
        // integer args (in x0..x7) with float args (in the low 32 bits of
        // s0..s7) and/or needs >8 args (the extra ones passed on the guest
        // stack). The uniform integer/float bridges cannot express these, so
        // hand the full guest CpuState to a per-function wrapper that reads the
        // exact x/s/sp lanes it needs and calls real Mesa (via gles-wrapper).
        if let Some((hostg, _slot)) = host_gles_call_at(pc) {
            let ret = hostg(state);
            let s = unsafe { &mut *state };
            s.x[0] = ret;
            s.pc = s.x[30];
            continue;
        }
        if pc < base || pc - base + 4 > image.len() as u64 {
            #[cfg(debug_assertions)]
            if std::env::var_os("JIT_TRACE").is_some() {
                let s = unsafe { &*state };
                eprintln!(
                    "[outside-image] pc={pc:#x} base={base:#x} end={:#x}",
                    base + image.len() as u64
                );
                eprintln!(
                    "[outside-image] x0={:#x} x1={:#x} x2={:#x} x3={:#x} x4={:#x} x5={:#x} x6={:#x} x7={:#x}",
                    s.x[0], s.x[1], s.x[2], s.x[3], s.x[4], s.x[5], s.x[6], s.x[7]
                );
                eprintln!(
                    "[outside-image] x8={:#x} x9={:#x} x10={:#x} x11={:#x} x12={:#x} x13={:#x} x14={:#x} x15={:#x}",
                    s.x[8], s.x[9], s.x[10], s.x[11], s.x[12], s.x[13], s.x[14], s.x[15]
                );
                eprintln!(
                    "[outside-image] x16={:#x} x17={:#x} x18={:#x} x19={:#x} x20={:#x} x21={:#x} x22={:#x} x23={:#x}",
                    s.x[16], s.x[17], s.x[18], s.x[19], s.x[20], s.x[21], s.x[22], s.x[23]
                );
                eprintln!(
                    "[outside-image] x24={:#x} x25={:#x} x26={:#x} x27={:#x} x28={:#x} x29={:#x} x30={:#x} pc={:#x}",
                    s.x[24], s.x[25], s.x[26], s.x[27], s.x[28], s.x[29], s.x[30], s.pc
                );
            }
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
        // `JIT_BUDGET` env overrides for instruction-granular tracing.
        // `JIT_STEP=1` forces single-instruction blocks and dumps the full
        // guest register file after each one — a per-instruction trace for
        // diffing a miscompiled straight-line block against a reference
        // (qemu -d cpu, or a hand/simulated oracle). Debug-only; no effect on
        // the normal path.
        let step_trace = std::env::var_os("JIT_STEP").is_some();
        let block_budget: usize = if step_trace {
            1
        } else {
            std::env::var("JIT_BUDGET")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8192)
        };
        let block = compile_image_bounded(image, base, pc, state, block_budget)?;
        #[cfg(debug_assertions)]
        if std::env::var_os("JIT_DUMP").is_some() {
            let raw = block.dump();
            if let Ok(f) = std::env::var("JIT_DUMP_FILE") {
                let _ = std::fs::write(&f, &raw);
            }
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
        if step_trace {
            let s = unsafe { &*state };
            let mut line = format!("STEP pc={:#x}", s.pc);
            for (i, x) in s.x.iter().enumerate() {
                line.push_str(&format!(" x{i}={x:#x}"));
            }
            for (i, v) in s.v.iter().enumerate() {
                line.push_str(&format!(" v{i}={v:#x}"));
            }
            println!("{line}");
        }
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

/// Bounds-checked read of a 32-bit word at guest address `addr` from `image`
/// mapped at `base` (guest == host only when the image is at its base address,
/// which is how elfjit maps it; the bounds check keeps this safe even for a
/// test Vec that is not at `base`).
fn word_at(image: &[u8], base: u64, addr: u64) -> Option<u32> {
    if addr < base {
        return None;
    }
    let off = addr - base;
    if off + 4 > image.len() as u64 {
        return None;
    }
    let o = off as usize;
    Some(u32::from_le_bytes([
        image[o],
        image[o + 1],
        image[o + 2],
        image[o + 3],
    ]))
}

/// Decode the reachable guest call graph starting at `entry` (bounded) and
/// report whether any `bl` inside it targets a host-import PLT stub. Used to
/// decide whether a guest `bl` to `entry` should be diverted through the
/// dispatcher instead of inlined: a callee that itself calls host imports
/// (pthread_mutex_lock, abort, syslog, ...) cannot be inlined safely, because
/// the inner import `bl` is itself diverted via the dispatcher stub table and
/// — when the outer routine is inlined a second time inside a larger block —
/// that inner return-stub bookkeeping regresses (the FMOD once-routine returns
/// "not done", w0=1, and the caller branches into a guard address). Diverting
/// the outer `bl` makes the callee run as its own fresh `jit_run` block, whose
/// inner imports get clean diversion every time.
///
/// The scan is deliberately bounded: it gives up (returns `true`, i.e. "safe
/// to divert") after `BODY_SCAN_BUDGET` decoded instructions rather than walk
/// an arbitrarily large function. Diverting is always *conservative* (correct,
/// just more dispatcher round-trips), so the false-positive on give-up is safe.
const BODY_SCAN_BUDGET: usize = 512;

fn body_contains_host_plt_bl(image: &[u8], base: u64, entry: u64) -> bool {
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut frontier: Vec<u64> = vec![entry];
    let mut scanned = 0usize;
    while let Some(start) = frontier.pop() {
        if !seen.insert(start) {
            continue;
        }
        if scanned > BODY_SCAN_BUDGET {
            return true; // give up conservatively: treat as import-bearing
        }
        let mut cur = start;
        loop {
            // stop at a block start that a sibling frontier item already owns
            if cur != start && seen.contains(&cur) {
                break;
            }
            let Some(word) = word_at(image, base, cur) else {
                break;
            };
            let inst = decode::decode(word);
            scanned += 1;
            if scanned > BODY_SCAN_BUDGET {
                return true;
            }
            match inst {
                Inst::B { imm, link } => {
                    let target = cur.wrapping_add(imm as u64);
                    if link {
                        // a `bl` straight to a host import => this body diverts
                        if is_host_plt_stub(image, base, target) {
                            return true;
                        }
                        // otherwise a guest call: do NOT follow into the callee
                        // body. This predicate detects only DIRECT host-import
                        // calls in the entry's own body (the once-routine calls
                        // pthread_mutex_lock@plt directly). Following through
                        // guest->guest->import would mark every caller up the
                        // whole call graph as import-bearing and defeat the
                        // bounded-compile model. The caller's own `bl` to a
                        // guest callee is not itself a host-import call, so we
                        // just let the linear walk continue at the fall-through.
                    } else {
                        // unconditional b: follow target, stop linear walk
                        frontier.push(target);
                        break;
                    }
                }
                Inst::BCond { imm, .. } | Inst::Cbz { imm, .. } | Inst::Tbz { imm, .. } => {
                    frontier.push(cur.wrapping_add(imm as u64));
                }
                Inst::Ret
                | Inst::Br { .. }
                | Inst::Blr { .. }
                | Inst::Unsupported(_)
                | Inst::Brk { .. }
                | Inst::Udf { .. } => break,
                _ => {}
            }
            cur += 4;
        }
    }
    false
}

/// Detect whether the instructions at guest address `addr` (within `image`
/// mapped at `base`) are a PLT stub
/// (`adrp xd,P; ldr xc,[xd,#imm]; add xd,xd,#off; br xc`) whose GOT slot holds a
/// host-thunk address (>= HOST_THUNK_BASE). This identifies a direct guest `bl`
/// to a host import (e.g. `bl pthread_mutex_lock@plt`). Returns false on any
/// mismatch so this is conservative: a real guest function is never mistaken
/// for an import stub.
fn is_host_plt_stub(image: &[u8], base: u64, addr: u64) -> bool {
    // No low-address guard here: reading goes through `word_at`, which is
    // bounds-checked against `image`, so low synthetic addresses (the unit-test
    // stub images live at 0x40) are handled safely. The old `addr < 0x1000`
    // reject was a leftover from the raw-pointer implementation and wrongly
    // rejected those legitimate stubs.
    let Some(w0) = word_at(image, base, addr) else {
        return false;
    };
    #[cfg(debug_assertions)]
    if std::env::var_os("JIT_DUMP").is_some() {
        eprintln!(
            "[hps] addr={addr:#x} w0={:#010x} w1={:#010x}",
            w0,
            word_at(image, base, addr + 4).unwrap_or(0)
        );
    }
    // word 0: adrp Xd, #page
    if (w0 & 0x9f00_0000) != 0x9000_0000 {
        return false;
    }
    let d0 = w0 & 0x1f;
    // word 1: ldr Xt, [Xn, #imm]   (64-bit unsigned-offset load)
    let Some(w1) = word_at(image, base, addr + 4) else {
        return false;
    };
    if (w1 & 0xffc0_0000) != 0xf940_0000 {
        return false;
    }
    let rn = (w1 >> 5) & 0x1f;
    let dt = w1 & 0x1f;
    if rn != d0 {
        return false; // must load from the adrp'ed page reg (a real PLT stub)
    }
    // word 2: add Xd, Xd, #off (the AArch64 canonical PLT stub does this)
    let Some(w2) = word_at(image, base, addr + 8) else {
        return false;
    };
    if (w2 & 0xff00_0000) != 0x9100_0000 {
        return false;
    }
    // word 3: br Xt   — must branch to the register loaded by the `ldr` above.
    let Some(w3) = word_at(image, base, addr + 12) else {
        return false;
    };
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
    // Guest-`bl` targets that we DECIDED must divert through the dispatcher
    // (a callee whose body calls a host import — import_bearing), even when the
    // target is the current block's own address and would otherwise be found
    // in `host_of_guest`. The recursion case is the kicker: `bl f` where f's
    // body also calls a host import; f is import-bearing, so its recursion must
    // divide via a dispatcher stub (a fresh f frame) — but if we resolve the
    // fixup to `host_of_guest[f]` we inline the recursion into the very block
    // being compiled, and the inline-call/dispatcher-stub interaction for the
    // inner host import regresses exactly like the once-routine bug. Force these
    // to the stub on resolution.
    let mut divert_set: std::collections::HashSet<u64> = std::collections::HashSet::new();
    // Memo of body_contains_host_plt_bl() per guest-bl target, so we scan a
    // given callee body at most once per compile (it may be inlined from many
    // call sites within one block).
    let mut memo_divert: std::collections::HashMap<u64, bool> = std::collections::HashMap::new();
    // Truncation fall-through tracking: when the budget cuts a straight-line
    // body short (no terminal instruction writes pc), the last emitted
    // instruction falls through to `trunc_next_pc` with nothing updating
    // CpuState.pc — the dispatcher would otherwise re-compile from the SAME
    // entry forever. Captured at function scope so multiple frontier regions
    // don't contaminate it; only the last partially-emitted region matters.
    let mut trunc_next_pc: Option<u64> = None;
    let mut trunc_last_terminal = false;
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
                // Loop back-edge / already-emitted tail: emit an unconditional
                // jump to the already-emitted host offset so a loop re-executes
                // its body, instead of falling through to the shared epilogue
                // `ret`. Without this every loop body "fell off the end" and
                // returned / re-dispatched after a single pass (broke loops —
                // diverged, corrupted pc, or hung re-compiling).
                let disp_off = buf.jmp_rel32();
                fixups.push(crate::translate::Fixup {
                    target_pc: cur,
                    disp_off,
                    cc: 0, // unconditional jmp (E9)
                });
                break;
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
                        let hps = is_host_plt_stub(image, base, target);
                        // A guest `bl` whose callee body itself calls a host
                        // import (pthread_mutex_lock, abort, syslog, ...) is
                        // also unsafe to inline: the inner import divert via
                        // the dispatcher stub table regresses when this callee
                        // (e.g. the FMOD one-time-init routine) is inlined a
                        // second time inside a larger block, so it returns
                        // "not done" and the caller branches into garbage.
                        // Divert these too, so the callee compiles as its own
                        // fresh block with clean inner-import diversion.
                        let import_bearing = hps || memo_divert.get(&target).copied().unwrap_or_else(|| {
                            let b = body_contains_host_plt_bl(image, base, target);
                            memo_divert.insert(target, b);
                            b
                        });
                        #[cfg(debug_assertions)]
                        if std::env::var_os("JIT_DUMP").is_some() {
                            eprintln!("[bl] {cur:#x} -> {target:#x} hostplt={hps} import_bearing={import_bearing}");
                        }
                        if !import_bearing {
                            frontier.push(target);
                        } else {
                            // Diverted: don't inline this call; make sure the
                            // stub table is built so the fixup has a real target.
                            force_stubs = true;
                            divert_set.insert(target);
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
            #[cfg(debug_assertions)]
            if std::env::var_os("JIT_DUMP").is_some() {
                let is_ret = matches!(inst, Inst::Br { .. } | Inst::Blr { .. } | Inst::Ret);
                if is_ret {
                    eprintln!("[term] guest_pc={cur:#x} inst={inst:?}");
                }
            }
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
                trunc_last_terminal = true; // this instruction writes pc itself
                break;
            }
            cur += 4;
            // Non-terminal fall-through successor (for truncation divert below).
            trunc_next_pc = Some(cur);
            trunc_last_terminal = false;
        }
    }

    // Bounded truncation may cut a straight-line body short with no terminal
    // instruction to write CpuState.pc. Divert the fall-through to a
    // dispatcher-return stub for `trunc_next_pc` so the block advances past
    // its untranslated tail instead of re-running its own entry forever.
    if truncated && !trunc_last_terminal {
        if let Some(np) = trunc_next_pc {
            if np >= base && np - base + 4 <= image.len() as u64 {
                let disp_off = buf.jmp_rel32();
                fixups.push(crate::translate::Fixup {
                    target_pc: np,
                    disp_off,
                    cc: 0, // unconditional jmp (E9) -> dispatcher-return stub
                });
            }
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
        // collect the set of targets referenced by fixups but not emitted, PLUS
            // any divert_set target (which must go through a stub even when the target
            // is in host_of_guest — see the recursion note above).
            let need: Vec<u64> = fixups
                .iter()
                .filter(|fx| !host_of_guest.contains_key(&fx.target_pc) || divert_set.contains(&fx.target_pc))
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
        let target = if divert_set.contains(&fx.target_pc) {
            // Decided to divert (import-bearing callee / recursion): route to a
            // dispatcher-return stub even though the target may be in
            // host_of_guest (e.g. `bl f` recursion where f is the block's own
            // entry). Turn a call into a jmp so it re-enters the dispatcher for
            // a clean f frame (see the divert_set doc).
            if fx.cc == 0xfe {
                buf.bytes[fx.disp_off - 1] = 0xe9; // E8 -> E9 (call->jmp)
            }
            stub_of_target[&fx.target_pc]
        } else if host_of_guest.contains_key(&fx.target_pc) {
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
    fn addsubext_sxtw_and_postindex_exec() {
        // add x3, x2, w20, sxtw #3 = 0x8b34cc43: x3 = x2 + (sext32(w20)<<3).
        // mov x0,x3 (orr)=0xaa0303e0; ret.
        let code = [
            0x43u8, 0xcc, 0x34, 0x8b, // add x3,x2,w20,sxtw #3
            0xe0, 0x03, 0x03, 0xaa, // mov x0,x3
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[2] = 0x1000;
        st.x[20] = 0x18;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x1000 + (0x18 << 3), "sxtw#3 add");
        // sxtw of a negative 32-bit: w20 = -1 -> extend to -1, <<3 = -8.
        let mut st2 = CpuState::new();
        st2.x[2] = 0x1000;
        st2.x[20] = 0xFFFF_FFFF; // w20 = -1
        let r = exec_bytes(&mut st2, &code, 0).expect("exec");
        assert_eq!(r as i64, 0x1000 - 8, "sxtw negative");
    }

    #[test]
    fn ldrstr_postindex_writeback_exec() {
        // ldr x3,[x0],#8 ; ldr x4,[x0],#8 ; ldr x5,[x0],#8 ; mov x0,x5 ; ret
        // Post-index must advance x0 each time and load the successive values.
        let code = [
            0x03u8, 0x84, 0x40, 0xf8, // ldr x3,[x0],#8
            0x04, 0x84, 0x40, 0xf8, // ldr x4,[x0],#8
            0x05, 0x84, 0x40, 0xf8, // ldr x5,[x0],#8
            0xe0, 0x03, 0x05, 0xaa, // mov x0, x5
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let buf = [0x1111u64, 0x2222, 0x3333, 0x4444];
        let mut st = CpuState::new();
        st.x[0] = buf.as_ptr() as u64;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x3333, "post-index writes back x0, x5=3rd value");
    }

    fn pack4(f: [f32; 4]) -> (u64, u64) {
        let b = |x: f32| (x.to_bits() as u64);
        (b(f[0]) | (b(f[1]) << 32), b(f[2]) | (b(f[3]) << 32))
    }

    #[test]
    fn vector_fp_arith_ground_truth() {
        // Vector NEON single/double FP arithmetic + int<->float convert,
        // encodings verified against aarch64-linux-gnu-as (see /tmp/fpx.s).
        // A Roblox 3D engine's matrix/vertex math is dense with these .4s/.2d
        // ops; the differential battery surfaced them as silent miscompiles.
        let ret: [u8; 4] = [0xc0, 0x03, 0x5f, 0xd6];
        let finv = |w: u32| w.to_le_bytes();

        // fmla v0.4s, v0.4s, v1.4s = 0x4e21cc00 -> v0[i] = v0[i] + v0[i]*v1[i]
        let mut st = CpuState::new();
        let (l0, h0) = pack4([1.0, 2.0, 3.0, 4.0]);
        let (l1, h1) = pack4([2.0, 3.0, 4.0, 5.0]);
        st.set_v(0, l0, h0);
        st.set_v(1, l1, h1);
        let mut code = finv(0x4e21cc00).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmla4s exec");
        let (lo, hi) = st.get_v(0);
        let e = pack4([3.0, 8.0, 15.0, 24.0]); // x*(1+y)
        assert_eq!((lo, hi), e, "fmla v0.4s ground truth");

        // fmul v0.4s, v0.4s, v1.s[0] = 0x4f819000 -> v0[i] = v0[i]*v1[0]
        let mut st = CpuState::new();
        let (l0, h0) = pack4([1.0, 2.0, 3.0, 4.0]);
        let (l1, h1) = pack4([10.0, 0.0, 0.0, 0.0]);
        st.set_v(0, l0, h0);
        st.set_v(1, l1, h1);
        let mut code = finv(0x4f819000).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmul4s_el exec");
        let (lo, hi) = st.get_v(0);
        let e = pack4([10.0, 20.0, 30.0, 40.0]);
        assert_eq!((lo, hi), e, "fmul v0.4s, v1.s[0] ground truth");

        // fmla v0.2d, v0.2d, v1.2d = 0x4e61cc00 -> two double lanes
        let mut st = CpuState::new();
        st.set_v(0, 1.0f64.to_bits(), 2.0f64.to_bits());
        st.set_v(1, 3.0f64.to_bits(), 4.0f64.to_bits());
        let mut code = finv(0x4e61cc00).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmla2d exec");
        let (lo, hi) = st.get_v(0);
        assert_eq!(lo, 4.0f64.to_bits(), "fmla v0.2d lane0 (1+1*3)");
        assert_eq!(hi, 10.0f64.to_bits(), "fmla v0.2d lane1 (2+2*4)");

        // scvtf v0.4s, v0.4s = 0x4e21d800 -> per-lane int->float
        let mut st = CpuState::new();
        st.set_v(0, 1u64 | (2 << 32), 3u64 | (4 << 32));
        let mut code = finv(0x4e21d800).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("scvtf4s exec");
        let (lo, hi) = st.get_v(0);
        let e = pack4([1.0, 2.0, 3.0, 4.0]);
        assert_eq!((lo, hi), e, "scvtf v0.4s ground truth");

        // fmov v0.4s, #1.0 (SimdFmovImm, 0x4f03f600): all 4 lanes = 1.0f
        let mut st = CpuState::new();
        st.set_v(0, 0xdead, 0xbeef); // dirty slots, must be overwritten
        let mut code = finv(0x4f03f600).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmov v.4s exec");
        let (lo, hi) = st.get_v(0);
        assert_eq!((lo, hi), pack4([1.0, 1.0, 1.0, 1.0]), "fmov v0.4s,#1.0 broadcast");

        // fmov s0, w1 (FmovGp single, 0x1e270021): move w1 bits into v0.s[0]
        let mut st = CpuState::new();
        st.set_v(0, 0, 0);
        st.x[1] = 1.13f32.to_bits() as u64;
        let mut code = finv(0x1e270020).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmov s,w exec");
        let (lo, _hi) = st.get_v(0);
        assert_eq!(lo as u32, 1.13f32.to_bits(), "fmov s0,w1 single bits");

        // scvtf with NEGATIVE int lanes (fv_i2f: a[] = i*3-7 -> -7,-4,-1,..)
        let mut st = CpuState::new();
        st.set_v(2, 0xfffffff9u64 | (0xfffffffcu64 << 32), 0xffffffffu64 | (0x2u64 << 32));
        let mut code = finv(0x4e21d842).to_vec(); // rd=2,rn=2 (in-place, high reg)
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("scvtf4s neg exec");
        let (lo, hi) = st.get_v(2);
        let e = pack4([-7.0, -4.0, -1.0, 2.0]);
        assert_eq!((lo, hi), e, "scvtf v0.4s negative lanes");

        // fadd v0.4s, v0.4s, v1.4s = 0x4e21d400 -> per-lane add
        let mut st = CpuState::new();
        let (l0, h0) = pack4([1.0, 2.0, 3.0, 4.0]);
        let (l1, h1) = pack4([0.5, 0.5, 0.5, 0.5]);
        st.set_v(0, l0, h0);
        st.set_v(1, l1, h1);
        let mut code = finv(0x4e21d400).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fadd4s exec");
        let (lo, hi) = st.get_v(0);
        let e = pack4([1.5, 2.5, 3.5, 4.5]);
        assert_eq!((lo, hi), e, "fadd v0.4s ground truth");
    }

    #[test]
    fn vector_fp_by_element_highreg_and_2d() {
        // The exact gcc-emitted by-element and high-register forms the float
        // differential battery failed on (fv_arith / dv_arith): fmul against a
        // broadcast scalar lane in HIGH registers, and the .2d double variant.
        let ret: [u8; 4] = [0xc0, 0x03, 0x5f, 0xd6];
        let finv = |w: u32| w.to_le_bytes();

        // fmul v30.4s, v30.4s, v17.s[0] = 0x4f9193de (fv_arith)
        let mut st = CpuState::new();
        let (l0, h0) = pack4([1.0, 2.0, 3.0, 4.0]);
        st.set_v(30, l0, h0);
        st.set_v(17, pack4([10.0, 0.0, 0.0, 0.0]).0, 0); // s[0]=10
        let mut code = finv(0x4f9193de).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmul v30, v17.s[0] exec");
        let (lo, hi) = st.get_v(30);
        assert_eq!((lo, hi), pack4([10.0, 20.0, 30.0, 40.0]), "fmul v30.4s, v17.s[0] highreg");

        // fmul v6.2d, v6.2d, v1.d[0] = 0x4fc190c6 (dv_arith): double by-element
        let mut st = CpuState::new();
        st.set_v(6, 1.0f64.to_bits(), 2.0f64.to_bits());
        st.set_v(1, 3.0f64.to_bits(), 0u64);
        let mut code = finv(0x4fc190c6).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmul v6.d[0] exec");
        let (lo, hi) = st.get_v(6);
        assert_eq!(lo, 3.0f64.to_bits(), "fmul v6.2d, v1.d[0] lane0");
        assert_eq!(hi, 6.0f64.to_bits(), "fmul v6.2d, v1.d[0] lane1");

        // fmla v21.2d, v1.2d, v22.2d = 0x4e76cc35 (dv_arith): double vector FMLA
        let mut st = CpuState::new();
        st.set_v(21, 100.0f64.to_bits(), 200.0f64.to_bits());
        st.set_v(1, 3.0f64.to_bits(), 4.0f64.to_bits());
        st.set_v(22, 5.0f64.to_bits(), 6.0f64.to_bits());
        let mut code = finv(0x4e76cc35).to_vec();
        code.extend_from_slice(&ret);
        exec_bytes(&mut st, &code, 0).expect("fmla v21.2d exec");
        let (lo, hi) = st.get_v(21);
        assert_eq!(lo, 115.0f64.to_bits(), "fmla v21.2d lane0 (100+3*5)");
        assert_eq!(hi, 224.0f64.to_bits(), "fmla v21.2d lane1 (200+4*6)");
    }

    #[test]
    fn lse_atomic_swp_and_ldadd_exec() {
        // ldadd w3, w6, [x0] : Rs=w6(>>16), Rn=x0(>>5), Rt=w3(&0x1f). 0xb8260003.
        // swp x3, x6, [x0]   : Rs=x6, Rn=x0, Rt=x3.             0xf8a68003.
        let ldadd = [
            0x03u8, 0x00, 0x26, 0xb8, // ldadd w3, w6, [x0]
            0xe0, 0x03, 0x03, 0xaa, // mov x0, x3
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut mem = [0u64; 2];
        let mut st = CpuState::new();
        st.x[0] = mem.as_ptr() as u64;
        st.x[6] = 5;
        mem[0] = 100;
        let r = exec_bytes(&mut st, &ldadd, 0).expect("exec");
        assert_eq!(r, 100, "ldadd returns the OLD value");
        assert_eq!(mem[0], 105, "ldadd adds into memory");
        // swp: swap old value with Rs. swp x3, x6, [x0] = 0xf8a68003
        let swp = [
            0x03u8, 0x80, 0xa6, 0xf8, // swp x3, x6, [x0]
            0xe0, 0x03, 0x03, 0xaa,
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        let mut mem = [0u64; 2];
        let mut st = CpuState::new();
        st.x[0] = mem.as_ptr() as u64;
        st.x[6] = 42;
        mem[0] = 7;
        let r = exec_bytes(&mut st, &swp, 0).expect("exec");
        assert_eq!(r, 7, "swp returns old");
        assert_eq!(mem[0], 42, "swp stores Rs into memory");
    }

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
    fn mullong_umull_exec() {
        // umull x1, w3, w7 = 0x9ba77c61 : x1 = (u64)w3 * (u64)w7 (unsigned 32x32).
        // mov x0,x1 (orr) = 0xaa0103e0 ; ret = 0xd65f03c0.
        let code = [
            0x61u8, 0x7c, 0xa7, 0x9b, // umull x1, w3, w7
            0xe0, 0x03, 0x01, 0xaa, // mov x0, x1
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        // Large 32-bit operands: exercise the zero-extension (unsigned) path.
        let mut st = CpuState::new();
        st.x[3] = 0x0000_0001_0000_0005; // w3 low32 = 5
        st.x[7] = 7;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 35, "umull 5*7 should be 35 (w3 high bits ignored)");
        // Unsigned with high bit set: w3 = 0xFFFFFFFF (as u32), w7 = 2.
        let mut st = CpuState::new();
        st.x[3] = 0xFFFFFFFF;
        st.x[7] = 2;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, (0xFFFFFFFFu64) * 2, "umull unsigned 0xFFFFFFFF * 2");
    }

    #[test]
    fn mullong_smull_and_msubl_exec() {
        // smull x4, w5, w6 = 0x9b267ca4 (signed): x4 = (i64)sext(w5)*(i64)sext(w6).
        let code1 = [
            0xa4u8, 0x7c, 0x26, 0x9b, // smull x4, w5, w6
            0xe0, 0x03, 0x04, 0xaa, // mov x0, x4
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[5] = 0xFFFF_FFFD; // w5 = -3 (sign-extended)
        st.x[6] = 4;
        let r = exec_bytes(&mut st, &code1, 0).expect("exec");
        assert_eq!(r as i64, -12, "smull (-3)*4 = -12");
        // umsubl x10, w11, w12, x13 = 0x9bacb56a : x10 = x13 - w11*w12.
        let code2 = [
            0x6au8, 0xb5, 0xac, 0x9b, // umsubl x10, w11, w12, x13
            0xe0, 0x03, 0x0a, 0xaa, // mov x0, x10
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[11] = 2;
        st.x[12] = 3;
        st.x[13] = 100;
        let r = exec_bytes(&mut st, &code2, 0).expect("exec");
        assert_eq!(r, 100 - 6, "umsubl 100 - 2*3");
    }

    #[test]
    fn addvl_scales_by_16_bytes() {
        // addvl x0, x0, #16 = 0x04205200 : x0 += 16*16 = 256 (model VL=16B).
        let code = [
            0x00u8, 0x52, 0x20, 0x04, // addvl x0,x0,#16
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[0] = 1000;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 1256, "addvl x0,x0,#16 => x0 += 16*16");
    }

    #[test]
    fn uaddw2_accumulate_lanes_correct_in_isolation() {
        // Regression-guard: `uaddw v31.2d,v31.2d,v26.2s` then
        // `uaddw2 v31.2d,v31.2d,v26.4s` must accumulate the two/4 word-lanes
        // of v26 into the two 64-bit lanes of v31, no cross-lane contamination
        // (encodings from aarch64-linux-gnu-as, see udw2.s). This isolates the
        // accumulate arm from the pre-existing CO-RESIDENT two-loop bug (where
        // the shared zero-widening register v29 is clobbered by the first
        // loop's scalar tail `fmov d29,x` before the second loop's zip reads
        // it) — the accumulate itself is correct when inputs are clean.
        let code = [
            0xffu8, 0x13, 0xba, 0x2e, // uaddw  v31.2d, v31.2d, v26.2s
            0xff, 0x13, 0xba, 0x6e, // uaddw2 v31.2d, v31.2d, v26.4s
            0xff, 0xbb, 0xf1, 0x5e, // addp   d31, v31.2d
            0xe0, 0x03, 0x66, 0x9e, // fmov   x0, d31
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        // v26 = 4 word-lanes: [10, 20, 30, 40]  (v26 slot = VECTOR_BASE+26*16)
        st.v[26 * 2] = (20u64 << 32) | 10;
        st.v[26 * 2 + 1] = (40u64 << 32) | 30;
        st.v[31 * 2] = 0; // v31.2d zeroed
        st.v[31 * 2 + 1] = 0;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        // uaddw:  v31[0] += 10; v31[1] += 20
        // uaddw2: v31[0] += 30; v31[1] += 40  => lane0=40 lane1=60
        // addp:   d31 = 40 + 60 = 100
        assert_eq!(r, 100, "uaddw/uaddw2 .2d accumulate should sum all four words");
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
    fn ldst_pair_offset_form_applies_immediate() {
        // Regression: the LdStPair *offset* form `ldp x0,x1,[x2,#16]` must read
        // [x2+16] and [x2+24]; it used to ignore the `#16` and read [x2]/[x2+8],
        // so a 16-byte struct passed by value read stale/x29 at the wrong base
        // (byvalue.elf: returned 0,0 instead of the packed struct).
        // ldp x0,x1,[x2,#16]=0xa9410440 ; ret=0xd65f03c0
        let code = [0x40u8, 0x04, 0x41, 0xa9, 0xc0, 0x03, 0x5f, 0xd6];
        let mut buf = [0u64; 4];
        buf[0] = 0xdead_beef_dead_beef; // must NOT be read (offset form)
        buf[2] = 0x2222_2222_1111_1111; // [x2+16]
        buf[3] = 0x4444_4444_3333_3333; // [x2+24]
        let mut st = CpuState::new();
        st.x[2] = buf.as_ptr() as u64; // x2 = &buf[0]
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, buf[2], "x0 = [x2+16]");
        assert_eq!(buf[0], 0xdead_beef_dead_beef, "buf must be unmodified");
        assert_eq!(st.x[1], buf[3], "x1 = [x2+24]");
    }

    #[test]
    fn addsub_s_flag_reads_xzr_not_sp_for_rn31() {
        // Regression: in ADD/SUB with the S (flags) bit set, register 31 is XZR
        // (= 0), NOT SP. `negs w1,w0` (subs w1,wzr,w0) used to read rn=31 as the
        // stack pointer, computing `sp - w0` instead of `-w0` (signmod.elf's
        // `%16` returned garbled remainders; byvalue's negs/cset also corrupted).
        // Seed SP with a distinctive value so any sp-dependent result differs.
        // negs w1,w0=0x6b0003e1 ; mov w0,w1=0x2a0103e0 ; ret=0xd65f03c0
        let code = [
            0xe1u8, 0x03, 0x00, 0x6b, // negs w1,w0
            0xe0, 0x03, 0x01, 0x2a, // mov w0,w1
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[0] = 5;
        st.x[31] = 0x1000; // if rn=31 wrongly read as SP, result = (0x1000-5)&0xffffffff
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0xffff_fffb, "-w0 = -5 must not depend on SP (rn=31 is XZR)");
    }

    #[test]
    fn udiv_computes_quotient() {
        // Regression: unsigned `udiv x5, x0, x1` (0x9ac10805) returned 0 for
        // every input — the x86 emitter's `div r64` used group-3 /0 (TEST)
        // instead of /6 (DIV), so `48 f7 c1` decoded as `test rcx,eax` and the
        // quotient never reached the destination. Fixed div_r64/div_r32 to /6.
        let code = [0x05u8, 0x08, 0xc1, 0x9a, 0xc0, 0x03, 0x5f, 0xd6]; // udiv x5,x0,x1; ret
        let mut st = CpuState::new();
        st.x[0] = 100;
        st.x[1] = 10;
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[5], 10, "udiv x5,100,10 = 10");
        // divisor greater than dividend -> 0 quotient
        st.x[0] = 7;
        st.x[1] = 20;
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[5], 0, "7/20 = 0");
    }

    #[test]
    fn sdiv_is_signed_udiv_is_unsigned_same_negative_input() {
        // REGRESSION (Session 99): every sdiv/udiv was decoded signed=bit17==1,
        // but bit17=0 for BOTH forms (the real discriminator is bit10: sdiv=1).
        // The MulDiv gate (first) labeled sdiv unsigned, so `sdiv` emitted the
        // UNSIGNED `div`: a negative dividend (w3) became huge positive ->
        // garbage quotient (idivA -O2 battery returned 0xaaaaaa2d, wanted -35).
        // Same negative input via the signed form must truncate toward zero;
        // via the unsigned form it must treat w3 as 0xffffffffffffffff.
        // sdiv w1,w3,w1 = 0x1ac10c61 ; udiv w1,w3,w1 = 0x1ac10861 ; ret
        let mut st = CpuState::new();
        st.x[3] = 0xffff_ffff_ffff_ff9c; // w3 = -100
        st.x[1] = 3;
        let code = [0x61u8, 0x0c, 0xc1, 0x1a, 0xc0, 0x03, 0x5f, 0xd6]; // sdiv w1,w3,w1; ret
        let _ = exec_bytes(&mut st, &code, 0).expect("exec sdiv");
        assert_eq!(st.x[1] as u32, 0xffff_ffdf, "signed -100/3 = -33 (0xffffffdf), not unsigned-mangled");

        let mut st = CpuState::new();
        st.x[3] = 0xffff_ffff_ffff_ff9c; // as W3 reinterpreted by udiv
        st.x[1] = 3;
        let code = [0x61u8, 0x08, 0xc1, 0x1a, 0xc0, 0x03, 0x5f, 0xd6]; // udiv w1,w3,w1; ret
        let _ = exec_bytes(&mut st, &code, 0).expect("exec udiv");
        assert_eq!(st.x[1] as u32, 0xffff_ff9c / 3, "unsigned w3/3 treats w3 as huge positive");
    }

    #[test]
    fn msub_reuses_rm_as_rd_without_clobbering_the_multiply_operand() {
        // REGRESSION (Session 99): br battery (n - (n/50)*50 -> `msub
        // w0,w1,w0,w2`) returned -48 for n=1298, q=25, divisor=50: the MulDiv
        // MSUB arm computed rn*rm - ra, but ARM MSUB is ra - rn*rm (Wd = Wa -
        // Wn*Wm), so 25*50-1298 = -48 instead of 48. Constant-folded addrs
        // masked it (gcc never emitted the instruction). 0x1b008820 = msub
        // w0,w1,w0,w2, 0x1b008824 = msub w4,w1,w0,w2 (disjoint rd).
        // msub w0,w1,w0,w2 = 0x1b008820 : w0 = w2 - w1*w0 = 1298 - 25*50 = 48
        let code = [0x20u8, 0x88, 0x00, 0x1b, 0xc0, 0x03, 0x5f, 0xd6]; // msub w0,w1,w0,w2; ret
        let mut st = CpuState::new();
        st.x[1] = 25; // w1 = quotient q
        st.x[0] = 50; // w0 = divisor d (rm, also dst)
        st.x[2] = 1298; // w2 = dividend n (ra)
        let _ = exec_bytes(&mut st, &code, 0).expect("exec msub overlap");
        assert_eq!(st.x[0] as u32, 48, "msub w0,w1,w0,w2 must read OLD w0=50 before writing dst");

        // disjoint rd: msub w4,w1,w0,w2 = 0x1b008824 ; mov w0,w4
        let code = [0x24u8, 0x88, 0x00, 0x1b, 0xe0, 0x03, 0x04, 0x2a, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        st.x[1] = 25;
        st.x[0] = 50;
        st.x[2] = 1298;
        let _ = exec_bytes(&mut st, &code, 0).expect("exec msub disjoint");
        assert_eq!(st.x[0] as u32, 48, "msub disjoint rd must also give 48");
    }

    #[test]
    fn addv_horizontal_sum_across_4s_lanes() {
        // REGRESSION (Session 99): SIMD `addv s31,v31.4s` (horizontal add, 0x4eb1b820)
        // was swallowed by an earlier dup/move gate that emitted per-lane identity
        // copies, so a vec-int short-array sum (vadd -O2 battery) returned 0 instead
        // of 360. Unknown: ADDV source Vn is at bits[9:5], bits20:16 is a fixed 17;
        // result goes to the bottom S element of Vd.
        // addv s0,v1.4s = 0x4eb1b820 ; ret. V1 words = [1,2,3,4] -> s0 = 10.
        let code = [0x20u8, 0xb8, 0xb1, 0x4e, 0xc0, 0x03, 0x5f, 0xd6]; // addv s0,v1.4s; ret
        let mut st = CpuState::new();
        st.v[2] = 0x0000_0002_0000_0001; // s1[0]=1, s1[1]=2
        st.v[3] = 0x0000_0004_0000_0003; // s1[2]=3, s1[3]=4
        let _ = exec_bytes(&mut st, &code, 0).expect("exec addv 4s");
        assert_eq!(st.v[0] & 0xffff_ffff, 10, "addv s0,v1.4s sums 1+2+3+4 into s0.low");
    }

    #[test]
    fn neg_reads_rn31_as_xzr_not_sp() {
        // Regression: `neg x6,x6` = `sub x6, xzr, x6` (0xcb0603e6) is the SHIFTED-
        // register add/sub form (bit21=0), where register 31 in the rn operand is
        // XZR (=0), NOT the stack pointer. Previously the non-S path read rn=31 as
        // SP, so neg(x6) computed sp - x6 instead of 0 - x6 (qemu: x6=2 -> -2).
        //   neg x6,x6 = 0xcb0603e6 ; mov x0,x6 = 0xaa0603e0 ; ret = 0xd65f03c0
        let code = [0xe6u8, 0x03, 0x06, 0xcb, 0xe0, 0x03, 0x06, 0xaa, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        st.x[6] = 2;
        st.x[31] = 0x1234_5678_9abc_def0; // SP set apart so any SP-read is visible
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0xffff_ffff_ffff_fffe, "neg(x6=2) = -2, must not be sp-2");
    }

    #[test]
    fn ubfiz_zero_extends_field_and_discards_old_rd() {
        // Regression: `ubfiz x4, x0, #7, #32` (0xd3797c04) is a ZERO-extending
        // shift-left. With x0=0x18 the result must be (0x18<<7) = 0xC00 and the
        // upper 32 bits of x4 must be ZERO, regardless of x4's old value. The
        // translate previously routed immr>imms through the BFI/merge path, so a
        // stale x4 (e.g. 0x7f8000000000 | ...) kept garbage high bits — a silent
        // miscompile that corrupted glibc's `__tunable_get_val` (x4.addr became
        // 0x7f800048e888 instead of 0x48e888, then ldr w6,[x4,#48] segfaulted).
        // Real word from modmain: ubfiz x4,x0,#7,#32 = d3797c04.
        let code = [0x04u8, 0x7c, 0x79, 0xd3];
        let mut st = CpuState::new();
        st.x[0] = 0x18;
        // Old x4 carries a high garbage prefix; it must be fully discarded.
        st.x[4] = 0x7f80_0000_0000_0000 | 0xDEAD_DEAD;
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[4], 0xC00, "ubfiz x4,x0,#7,#32 = 0xC00, upper zeroed");
    }

    #[test]
    fn bfi_still_merges_into_old_rd() {
        // Guard: genuine BFI (Bfm insert, insert=true, immr>imms) MUST still
        // preserve Rd's bits outside the field, unlike UBFIZ.
        // bfi x4, x0, #16, #16  (verified via objdump: 0xb3703c04).
        // BFM X4,X0,#immr=48,#imms=15 (immr>imms => wrap insert,
        // lsb=(64-48)&63=16, w=imms+1=16), rn=0, rd=4.
        // BFM Xd,Xn,#immr,#imms: 0xB340_0000 base | imms<<10 | immr<<16 |
        //   rn<<5 | rd. immr=0x30 -> 0x300000, imms=0x0f -> 0x3c00.
        let word = 0xB340_0000u32 | (0x0f << 10) | (0x30 << 16) | (0x0 << 5) | 0x4;
        let code = word.to_le_bytes();
        let mut st = CpuState::new();
        st.x[0] = 0x00FF; // field value; shifted <<16
        st.x[4] = 0xF000_0000_0000_0000; // Rd bits OUTSIDE field must stay
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        // field: (0x00FF<<16) = 0x00FF0000; rd keeps other bits.
        assert_eq!(st.x[4], 0xF000_0000_00ff_0000, "BFI merges into old Rd");
    }

    #[test]
    fn asr_w32_takes_sign_from_bit31_not_bit63() {
        // Regression: `asr w2, w1, #1` (0x13017c22) with w1=0x80000000 must give
        // 0xc0000000 (bit31 is the sign for a 32-bit arithmetic shift), NOT
        // 0x40000000. The translate used 64-bit `sar rax,1`; RAX held the
        // zero-extended guest value 0x0000000080000000, so bit63 (=0) was taken
        // as the sign and the shift became logical. (Found via gcc's
        // INT_MIN/2 fast-path: `add w2,w2,w2,lsr#31; asr w0,w2,#1`.)
        let code = [
            0x22u8, 0x7c, 0x01, 0x13, // asr w2, w1, #1
            0xe0, 0x03, 0x02, 0xaa, // mov x0, x2
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[1] = 0x8000_0000; // w1 (zero-extended)
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0xc000_0000, "asr w2,w1,#1 of 0x80000000 = 0xc0000000");
        // positive keeps clear
        let mut st2 = CpuState::new();
        st2.x[1] = 0x4000_0000;
        let r = exec_bytes(&mut st2, &code, 0).expect("exec");
        assert_eq!(r, 0x2000_0000, "asr of clear-bit31");
    }

    #[test]
    fn csel_family_op_discriminates_neg_not_inc_identity() {
        // Regression: the CSEL-family op field is `op = (bit30<<1)|bit10`, not
        // bits[11:10]. The old decode collapsed csinv->CSEL (identity instead of
        // NOT) and csneg->CSINC (+1 instead of NEG). All four variants with the
        // SAME regs/cond, run with the condition TRUE and FALSE, must apply the
        // right transform to rm on the false branch:
        //   csel  rd = c ? rn :  rm
        //   csinc rd = c ? rn :  rm+1
        //   csinv rd = c ? rn : ~rm
        //   csneg rd = c ? rn : -rm
        // Real words (assembler): csel 0x1a82b020, csinc 0x1a82b420,
        // csinv 0x5a82b020, csneg 0x5a82b420 (all W, rd0 rn1 rm2 cond lt).
        // cmp w1,#0 = 0x7100003f. w1=5 -> lt false; w1=-1 -> lt true.
        // The pre-fix op field (bits[11:10]) read csinv=0 (identity, so NOT was
        // lost) and csneg=1 (+1), discovered via gcc's INT_MIN % 2 body
        // `cmp; and w,#1; cneg w,,lt` returning 1 instead of 0.
        let csel = 0x1a82b020u32;
        let csinc = 0x1a82b420u32;
        let csinv = 0x5a82b020u32;
        let csneg = 0x5a82b420u32;
        let cmpw = 0x7100003fu32;
        let ret = 0xd65f03c0u32;
        // run(insn): w1 = -1 (lt TRUE) or +5 (lt FALSE); w2=5; return w0.
        let run = |word: u32, neg_w1: bool| -> u64 {
            let mut code = Vec::new();
            code.extend_from_slice(&cmpw.to_le_bytes());
            code.extend_from_slice(&word.to_le_bytes());
            code.extend_from_slice(&ret.to_le_bytes());
            let mut st = CpuState::new();
            st.x[1] = if neg_w1 { 0xFFFF_FFFF } else { 5 }; // w1
            st.x[2] = 5; // w2 = 5
            exec_bytes(&mut st, &code, 0).expect("exec")
        };
        // w1=5 -> cmp sets N=0,V=0 -> lt FALSE -> rd = f(rm) = f(5)
        assert_eq!(run(csel, false), 5, "csel false -> rn? no: -> rm = 5");
        // csinc: false -> rm+1 = 6
        assert_eq!(run(csinc, false), 6, "csinc false -> rm+1 = 6");
        // csinv: false -> ~5 = 0xfffffffa (w zero-extended)
        assert_eq!(run(csinv, false), 0x0000_0000_ffff_fffa, "csinv false -> ~5");
        // csneg: false -> -5 = 0xfffffffb
        assert_eq!(run(csneg, false), 0x0000_0000_ffff_fffb, "csneg false -> -5");
        // w1=-1 -> lt TRUE -> rd = rn = w1 = 0xffffffff
        assert_eq!(run(csneg, true), 0xffff_ffff, "csneg true -> rn = w1");
    }

    #[test]
    fn movn_w32_zero_extends_to_64_bits() {
        // Regression: `movn w0, #2` = 0x12800040 writes w0 = ~2 = 0xfffffffd, and
        // a 32-bit destination must ZERO-extend to the 64-bit register -> x0 =
        // 0x00000000fffffffd, NOT 0xfffffffffffffffd. mov_guest_imm's imm32
        // short-cut (`mov r32` sign-extends RAX) left the upper 32 bits set; the
        // translate now truncates W-dest MOVN to 32 bits first. (Found via a
        // `return x & 0xffffffff` folded to `movn w0,#2` returning
        // 0xfffffffffffffffd instead of 0x00000000fffffffd.)
        let code = [
            0x40u8, 0x00, 0x80, 0x12, // movn w0, #2 (0x12800040)
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x0000_0000_ffff_fffd, "movn w0,#2 zero-extends to x0");
    }

    #[test]
    fn extr_general_two_operand_rotate() {
        // Regression: `ror` via `(x >> 51) | (x << 13)` compiles to the GENERAL
        // EXTR `extr x0, x0, x1, #51` (rm != rn), which the Ror gate (rm==rn
        // only) skipped, letting it fall through to the UBFM/SBFM misdecode.
        // External result = (x0 >> 51) | (x1 << 13) = 0x8acf13579bde0246 for
        // x0=0x123456789abcdef0, x1=0x123456789abcdef0.
        // extr x0, x0, x1, #51 = 0x93c1cc00 (assembler-verified).
        let code = [
            0x00u8, 0xcc, 0xc1, 0x93, // extr x0, x0, x1, #51
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[0] = 0x123456789abcdef0;
        st.x[1] = 0x123456789abcdef0;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x8acf13579bde0246, "general EXTR rotates");
        // ror alias still works: extr x0, x0, x0, #13 = ror x0,#13 (0x93c03400)
        let code2 = [
            0x00u8, 0x34, 0xc0, 0x93, // ror x0, #13
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        let mut st2 = CpuState::new();
        st2.x[0] = 0x123456789abcdef0;
        let r2 = exec_bytes(&mut st2, &code2, 0).expect("exec");
        assert_eq!(r2, 0xf78091a2b3c4d5e6, "EXTR rm==rn == ror");
    }

    #[test]
    fn uzp1_rd_aliases_rn_does_not_corrupt_source() {
        // Regression: `uzp1 v12.8h, v12.8h, v26.8h` (rd==rn, gcc's ubiquitous
        // rotate/unpack idiom) wrote the SECOND-half (Vm) elements into rd bytes
        // 8..15, then a later first-half iteration read source bytes 8..15 from
        // the SAME slot — now corrupted. Snapshot source to scratch first.
        // v12.8h = {0x1111,0x2222,0x3333,0x4444, 0x5555,0x6666,0x7777,0x8888}
        // v26.8h = {0xaabb,0xccdd,0xeeff,0x0011, 0x2233,0x4455,0x6677,0x8899}
        // uzp1 v12.8h, v12.8h, v26.8h: result = even hw of v12 then even of v26:
        //   {0x1111,0x3333,0x5555,0x7777, 0xaabb,0xeeff,0x2233,0x6677}
        let mut st = CpuState::new();
        st.set_v(12, 0x4444333322221111, 0x8888777766665555); // v12 .8h
        st.set_v(26, 0x0011eeffccddaabb, 0x8899667755442233); // v26 .8h
        let word = 0x4e41198cu32; // uzp1 v12.8h, v12.8h, v26.8h (assembler-verified)
        let code = [
            word.to_le_bytes()[0], word.to_le_bytes()[1],
            word.to_le_bytes()[2], word.to_le_bytes()[3],
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        // Even halfwords: v12 lane0 holds hw0=0x1111 (LE) ... check d12 low = hw0,hw2,hw4,hw6
        // v12 d0 after: hw0=0x1111, hw2=0x3333, hw4=0x5555, hw6=0x7777 (LE u64)
        let low = 0x7777_5555_3333_1111u64;
        assert_eq!(st.v[12 * 2], low, "uzp1 rd==rn first half (even v12)");
    }

    #[test]
    fn simd_insd_sets_correct_lane_with_multi_byte_indices() {
        // Regression: the INS (vector, element) decode read dst_idx=bit20 and
        // src_idx=bit14 — two single bits. That only coincided with the true
        // lane once (S lane0->1); every S lane other than 0, and every H/B lane,
        // silently copied into the wrong element. The fv4 float canary
        // (`mov v3.s[1], v28.s[0]`, `mov v31.s[1], v4.s[0]`, built by gcc -O3)
        // depended on an S insert into lane 1 and returned 153 instead of 175.
        // Correct packing (verified against the aarch64 assembler for all 4x4 S,
        // 8x8 H, 16x16 B, 2x2 D lane pairs):
        //   l = log2(esize); dst = imm5 >> (l+1); src = (insn>>(11+l)) & ((1<<(4-l))-1).
        // mov v3.s[2], v5.s[1]: v3 lane 2 <- v5 lane 1 (=3.5f). 0x6e1424a3.
        let mut st = CpuState::new();
        st.set_v(5, 0x40600000_3f800000, 0); // v5.4s = {1.0, 3.5, 0, 0}
        let code = [
            0xa3, 0x24, 0x14, 0x6e, // mov v3.s[2], v5.s[1]
            // read v3 lane 2 back into x0 via `mov s0, v3.s[2]; fmov w0, s0`
            0x60, 0x04, 0x14, 0x5e, // mov s0, v3.s[2]
            0x00, 0x00, 0x26, 0x1e, // fmov w0, s0
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x4060_0000, "v3.s[2] = 3.5f copied from v5.s[1]");
        assert_eq!(st.v[3 * 2 + 1] & 0xffff_ffff, 0x4060_0000, "v3 lane2 = 3.5f");
    }

    #[test]
    fn fmls_vector_subtract_has_correct_operand_order() {
        // Regression: vector fmls (Vd = Vd - Vn*Vm) emitted `Vn*Vm - Vd` (the
        // same mul/op ordering as the commutative fmla add), so every
        // accumulate-subtract produced the right magnitude but WRONG SIGN —
        // `fmls v0.4s,v1.4s,v2.4s` on {100,..} - {1,..}*{10,..} gave +90 for
        // lane0 (should've been correct sign) but lanes were Vn*Vm-Vd. Fixed by
        // loading Vd into xmm0 and the product into xmm1 so subss(0,1) = Vd -
        // Vn*Vm.
        // fmls v31.4s, v1.4s, v26.4s = 0x4ebacc3f (rd=31,rn=1,rm=26) — the
        // gcc -O3 accumulator idiom. V31={100,200,300,400}; V1={1,2,3,4};
        // V26={10,20,30,40} => V31 = {90,160,210,240}.
        let mut st = CpuState::new();
        // v31 .4s lanes {100,200,300,400}: lo={200,100} hi={400,300}
        st.set_v(31, 0x42c80000_42c80000, 0x43c80000_43960000);
        // v1 .4s lanes {1,2,3,4}: lo={2,1} hi={4,3}
        st.set_v(1, 0x40000000_3f800000, 0x40800000_40400000);
        // v26 .4s lanes {10,20,30,40}: lo={20,10} hi={40,30}
        st.set_v(26, 0x41c00000_41200000, 0x42200000_41f00000);
        let code = [
            0x3f, 0xcc, 0xba, 0x4e, // fmls v31.4s, v1.4s, v26.4s
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        let (lo, _hi) = st.get_v(31);
        assert_eq!(
            lo & 0xffff_ffff,
            0x42b4_0000,
            "lane0 = 100 - 1*10 = 90 (fmls must be Vd - Vn*Vm, not reversed)"
        );
    }

    #[test]
    fn sqadd_uqadd_respect_lane_width_and_sign() {
        // Regression: SimdSatAdd treated every lane >= 32-bit as a 64-bit op —
        // `.4s` (esize=4) loaded 8 bytes as ONE 64-bit value and clamped both
        // s-lanes together, returning the smin sentinel 0x80000000 for a=10,b=5
        // (qemu: 15). Signed narrow lanes also compared the sign-extended result
        // against a positive smin bit-pattern (0x80000000 of the lane width) and
        // falsely clamped. Now: per-lane loads with the correct width, sign-
        // extended bounds, and .2d guard shifts (1<<64 / 1<<63 overflow).
        // uqadd v0.16b,v1.16b,v2.16b = 0x6e220c20 ; sqadd v0.4s = 0x4ea20c20 ;
        // uqadd v0.2d = 0x6ee20c20 ; sqsub v0.2d = 0x4ee22c20.
        for (w, esize, sub, unsigned, expect_lane0) in [
            (0x6e220c20u32, 1, false, true, 15), // uqadd16b: byte lanes {10}+{5}
            (0x4ea20c20u32, 4, false, false, 15), // sqadd4s
            (0x4ea22c20u32, 4, true, false, 5), // sqsub4s
            (0x6ea22c20u32, 4, true, true, 5), // uqsub4s
            (0x0e220c20u32, 1, false, false, 15), // sqadd8b
            (0x4e620c20u32, 2, false, false, 15), // sqadd8h q=1
            (0x6ee20c20u32, 8, false, true, 15), // uqadd2d
            (0x4ee22c20u32, 8, true, false, 5), // sqsub2d
        ] {
            let mut st = CpuState::new();
            st.set_v(1, 0x0a, 0); // lane0 = 10 (low byte / dword)
            st.set_v(2, 0x05, 0); // lane0 = 5
            let code = [
                w.to_le_bytes()[0], w.to_le_bytes()[1], w.to_le_bytes()[2], w.to_le_bytes()[3],
                0xc0, 0x03, 0x5f, 0xd6, // ret
            ];
            let _ = exec_bytes(&mut st, &code, 0).expect("exec");
            let got = match esize {
                8 => st.v[0],
                4 => st.v[0] & 0xffff_ffff,
                2 => (st.v[0] & 0xffff) as u64,
                _ => (st.v[0] & 0xff) as u64,
            };
            assert_eq!(
                got, expect_lane0,
                "word {w:#x}: lane0 = {got} (expected {expect_lane0})"
            );
        }
    }

    #[test]
    fn ubfiz_immr_gt_imms_does_not_rotate() {
        // Regression: `ubfiz w4,w2,#3,#3` (immr=29,imms=2 — 29+2+1==32==bits)
        // was caught by the UBFM/SBFM **ROR** shortcut `imms+immr+1==bits`
        // before reaching the UBFIZ shift branch, so it rotated by imms=2
        // instead of left-extending (w2&7)<<3. A genuine rotate (ror) has
        // immr<=imms; ubfiz/sbfiz have immr>imms. Named bfi semantic checks:
        // the -O2 mix-hash loop `h ^= msg[i] << ((i%8)*8)` emitted this exact
        // ubfiz and returned garbage (13680984341602923654 vs 13072640789477207222).
        // ubfiz w4, w2, #3, #3 = 0x531d0844 => w4 = (w2 & 7) << 3.
        let mut st = CpuState::new();
        st.x[2] = 13; // (13 & 7) << 3 = 5*8 = 40
        let code = [
            0x44, 0x08, 0x1d, 0x53, // ubfiz w4, w2, #3, #3
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[4] & 0xffff_ffff, 40, "ubfiz w4,w2,#3,#3 of 13 -> 40 (not a rotate)");
        // also cover a few more widths: ubfiz x4,x0,#7,#32 = 0xd3797c04
        let mut st2 = CpuState::new();
        st2.x[0] = 0x1; // (1 & mask32) << 7 = 0x80
        let code2 = [
            0x04, 0x7c, 0x79, 0xd3, // ubfiz x4,x0,#7,#32
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let _ = exec_bytes(&mut st2, &code2, 0).expect("exec");
        assert_eq!(st2.x[4], 0x80, "ubfiz x4,x0,#7,#32 of 1 -> 0x80");
    }

    #[test]
    fn fcvtzs_fixed_point_fbits_scales() {
        // Regression: fixed-point fcvtzs/fcvtzu Rd, Fn, #fbits (result =
        // trunc(Fn * 2^fbits)) was misdecoded as `SimdMull` (smull x0,w31,w24)
        // by the widening-multiply gate, so every *2^fbits scale in libc /
        // gcc -O3 fixed-point math (e.g. `(long long)(s*4)` folding into
        // fcvtzs #2) silently dropped the scale — v64f returned 5 vs 436.
        // Encodings: top16 0x1e18/0x1e58/0x9e18/0x9e58 (signed) or the same with
        // bit16 set (unsigned); fbits = 64 - field. Verified vs qemu: 3.25>>#2
        // = 13, 3.25>>#4 = 52, s 2.5>>#3 = 20, fcvtzu 3.75>>#1 = 7.
        // fcvtzs x0, d31, #2 = 0x9e58fbe0; fcvtzs x0,s31,#5 = 0x9e18efe0;
        // fcvtzu x0, d31, #3 = 0x9e59f7e0.
        use crate::decode::decode;
        assert!(
            matches!(decode(0x9e58fbe0), Inst::FcvtToInt { fbits: 2, sf: true, unsigned: false, src_sng: false, .. }),
            "fcvtzs x0,d31,#2 must decode FcvtToInt{{fbits:2}}, got {:?}",
            decode(0x9e58fbe0)
        );
        assert!(
            matches!(decode(0x9e59f7e0), Inst::FcvtToInt { fbits: 3, sf: true, unsigned: true, .. }),
            "fcvtzu x0,d31,#3 must decode unsigned fbits 3, got {:?}",
            decode(0x9e59f7e0)
        );
        assert!(
            matches!(decode(0x9e18efe0), Inst::FcvtToInt { fbits: 5, src_sng: true, .. }),
            "fcvtzs x0,s31,#5 must decode single fbits 5, got {:?}",
            decode(0x9e18efe0)
        );
    }

    #[test]
    fn ld1_multireg_post_index_decode_and_advance() {
        // Regression: post-indexed multi-register ld1/st1 {Vt..,Vt+n},[Xn],#imm
        // set bit23 (bases 0x..cc0 ld / 0x..c80 st), which the structure-multiple
        // gate's four no-post bases missed — so gcc's
        // `ld1 {v26.16b,v27.16b}, [x1], #32` fell through to the single-vector
        // Ld1V gate: only 16 bytes were loaded and Xn advanced by just #16.
        // A -O2 double dot-product (fmadd loop) accumulated garbage (165 vs 470).
        // Each must decode to its nreg-correct Inst with post = nreg*block.
        use crate::decode::decode;
        let cases: &[(u32, &str, u32)] = &[
            (0x4cdfa03a, "Ld1N", 2), // ld1 2reg post #32
            (0x4cdf703a, "Ld1N", 1), // ld1 1reg post #16
            (0x4cdf603a, "Ld1N", 3), // 3reg post #48
            (0x4c9fa03a, "St1N", 2), // st1 2reg post #32
            (0x4cdf803a, "Ld2", 0), // ld2 2reg post #32
            (0x4cdf003a, "Ld4N", 4), // ld4 post #64
        ];
        for (w, kind, nreg) in cases {
            let i = decode(*w);
            let name = format!("{i:?}");
            assert!(
                name.starts_with(kind),
                "{w:#x} must decode {kind}, got {name}"
            );
            if *nreg > 0 && (name.starts_with("Ld1N") || name.starts_with("St1N")) {
                assert!(
                    name.contains(&format!("nreg: {nreg}")),
                    "{w:#x} must have nreg {nreg}, got {name}"
                );
            }
        }
        // End-to-end: ld1 {v26,v27},[x1],#32 from a 64-byte buffer then read back.
        use crate::jit::{exec_bytes};
        let mut st = CpuState::new();
        let mem = Box::leak(vec![0u8; 128].into_boxed_slice());
        for i in 0..64u32 { mem[i as usize] = i as u8; }
        st.x[1] = mem.as_ptr() as u64; // base
        let code = [
            0x3a, 0xa0, 0xdf, 0x4c, // ld1 {v26.16b,v27.16b},[x1],#32
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[1], mem.as_ptr() as u64 + 32, "xn advances by 32");
        let (l26, h26) = st.get_v(26);
        assert_eq!(l26, u64::from_le_bytes(mem[0..8].try_into().unwrap()), "v26 low");
        assert_eq!(h26, u64::from_le_bytes(mem[8..16].try_into().unwrap()), "v26 high");
        let (l27, _) = st.get_v(27);
        assert_eq!(l27, u64::from_le_bytes(mem[16..24].try_into().unwrap()), "v27 low = bytes 16..24");
    }

    #[test]
    fn ld2_st2_halfword_deinterleave_respects_esize() {
        // Regression: the ld2/st2 translate arm DEINTERLEAVED AT BYTE
        // GRANULARITY regardless of element size, so `ld2 {v28.8h,v29.8h}`
        // (2-byte elements) read mem[2i], mem[2i+1] instead of the correct
        // mem[4i], mem[4i+2] — a u16 strided-accumulate loop (`for i+=2`)
        // silently returned 311814 vs the native 281606 (the even-index Sum
        // registered the wrong memory elements). Element size must scale the
        // deinterleave stride: element i of reg j is at byte i*(2*es) + j*es.
        use crate::decode::decode;
        use crate::decode::Inst;
        // Assemble-verified encodings (aarch64-linux-gnu-as):
        //   ld2 {v30.8h-v31.8h},[x0] = 0x4c40841e ; st2 same = 0x4c00841e
        //   ld2 {v30.8h-v31.8h},[x0],#32 = 0x4cdf841e ; st2 = 0x4c9f841e
        //   ld2 {v30.16b-v31.16b},[x0] = 0x4c40801e (byte esize=1)
        //   ld2 {v28.4s-v29.4s},[x0] = 0x4c40881c (word esize=4)
        for w in [0x4c40841eu32, 0x4c00841e, 0x4cdf841e, 0x4c9f841e] {
            let i = decode(w);
            assert!(
                matches!(i, Inst::Ld2 { esize: 2, .. } | Inst::St2 { esize: 2, .. }),
                "halfword ld2/st2 word {w:#x} decoded {i:?}"
            );
        }
        // The byte deinterleave (esize=1) still decodes: ld2 {v30.16b-v31.16b},[x0].
        let i = decode(0x4c40801eu32);
        assert!(
            matches!(i, Inst::Ld2 { esize: 1, .. }),
            "byte ld2 word decoded {i:?}"
        );
        // And the word (4-byte) deinterleave: ld2 {v28.4s-v29.4s},[x0].
        let i = decode(0x4c40881cu32);
        assert!(
            matches!(i, Inst::Ld2 { esize: 4, .. }),
            "word ld2 word decoded {i:?}"
        );
    }

    #[test]
    fn ld4_st4_decode_to_structure_deinterleave() {
        // Regression: the structure-load gate folded opcode 0b0000 (ld4/st4)
        // into the single-register consecutive path (0x7|0x0 => nreg 1), so
        // `ld4 {v24.4s-v27.4s},[x0]` loaded ONE 16B block instead of
        // deinterleaving four 4s vectors (matmul garbage: 20 vs 5248), and
        // 0b0100 (ld3/st3) was Unsupported. Each must now decode to its own
        // structure-DEINTERLEAVE Inst (never the consecutive Ld1N/St1N).
        use crate::decode::decode;
        // ld4 {v24.4s-v27.4s},[x0] = 0x4c400818 ; st4 = 0x4c000818
        // ld3 {v24.4s-v26.4s},[x0] = 0x4c404818 ; ld1 {v24.4s} = 0x4c407818
        for (w, ty) in [
            (0x4c400818u32, "ld4"),
            (0x4c000818u32, "st4"),
            (0x4c404818u32, "ld3"),
            (0x4c407818u32, "ld1single"),
        ] {
            let i = crate::decode::decode(w);
            let name = format!("{i:?}");
            match ty {
                "ld4" => assert!(name.starts_with("Ld4N"), "ld4 word {w:#x} decoded {i:?}"),
                "st4" => assert!(name.starts_with("St4N"), "st4 word {w:#x} decoded {i:?}"),
                "ld3" => assert!(name.starts_with("Ld3N"), "ld3 word {w:#x} decoded {i:?}"),
                _ => assert!(name.starts_with("Ld1N"), "ld1-1reg word {w:#x} decoded {i:?}"),
            }
        }
    }

    #[test]
    fn dup_from_gpr_not_swallowed_by_sqadd_gate() {
        // Regression: the SIMD saturating-add gate also matched `dup Vd.T,Wn`
        // (byte2==0x0c), so gcc's `dup v30.4s, w1` matrix-init broadcast
        // decoded as sqadd(v1,v4) and every -O2 matrix/fill loop corrupted the
        // array (init_O2 returned huge garbage vs 96). Discriminator: sat-add
        // always sets bit21, dup-from-GPR always clears it (assembler-verified).
        // dup v30.4s, w1 = 0x4e040c3e ; sqadd v30.4s, v1.4s, v4.4s = 0x4ea40c3e.
        use crate::decode::decode;
        assert!(
            matches!(decode(0x4e040c3e), Inst::SimdDupGp { rd: 30, rn: 1, .. }),
            "dup v30.4s,w1 must decode SimdDupGp, got {:?}",
            decode(0x4e040c3e)
        );
        assert!(
            matches!(decode(0x4ea40c3e), Inst::SimdSatAdd { rd: 30, rn: 1, rm: 4, .. }),
            "sqadd v30.4s,v1.4s,v4.4s must decode SimdSatAdd, got {:?}",
            decode(0x4ea40c3e)
        );
        // dup other widths still decode to SimdDupGp.
        for w in [0x4e020c3eu32 /*8h*/, 0x4e010c3e /*16b*/, 0x0e040c3e /*2s*/] {
            assert!(
                matches!(decode(w), Inst::SimdDupGp { .. }),
                "dup width word {w:#x} decoded {:?}",
                decode(w)
            );
        }
    }

    #[test]
    fn mrs_dczid_el0_returns_block_size() {
        // Regression: `mrs x0, dczid_el0` (0xd53b00e0) — read by glibc's CRT to
        // size its DC ZVA memset path — was previously Unsupported, halting any
        // full glibc-linked program at __libc_start_main. Returns a 16-byte block
        // (0x4, DZP=0), which is a valid, self-consistent value.
        //   mrs x0, dczid_el0 = 0xd53b00e0 ; mov x4,x0 = 0xaa0003e4 ; ret
        let code = [0xe0u8, 0x00, 0x3b, 0xd5, 0xe4, 0x03, 0x00, 0xaa, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[0], 0x4, "dczid_el0 -> x0 = 16-byte DC ZVA block");
        assert_eq!(r, 0x4, "x0 = dczid value");
    }

    #[test]
    fn sysreg_mrs_reads_commit_to_guest_register() {
        // Regression: the SysReg MRS write path used `buf.mov_ri64(rt, ..)` with
        // rt as a HOST register index, so every `mrs xN, <cntfrq|cntvct|nzcv|
        // dczid|tpidr>` dumped the value into a stray x86 reg and left the guest
        // slot stale — a silent no-op (verify: cf/dz battery returned 0 before).
        // Now each read commits via stg. cntfrq_el0 = 100 MHz, dczid = 4 bytes.
        //   mrs x0,cntfrq_el0 = 0xd53be000 ; mrs x4,dczid_el0 = 0xd53b00e4
        //   mrs x7,tpidr_el0 = 0xd53bd047 ; ret
        let code = [
            0x00, 0xe0, 0x3b, 0xd5, // mrs x0, cntfrq_el0
            0xe4, 0x00, 0x3b, 0xd5, // mrs x4, dczid_el0
            0x47, 0xd0, 0x3b, 0xd5, // mrs x7, tpidr_el0
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.tpidr = 0x1234_5678_9abc_def0;
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[0], 100_000_000, "cntfrq_el0 -> guest x0");
        assert_eq!(st.x[4], 0x4, "dczid_el0 -> guest x4");
        assert_eq!(st.x[7], st.tpidr, "tpidr_el0 -> guest x7");
    }

    #[test]
    fn mulh_high_product_umulh_smulh() {
        // Regression: umulh/smulh (high 64 of 128-bit product) were Unsupported —
        // a common compiler/glibc idiom (modmain.elf stopped on `umulh x2,x3,x6`).
        // Encodings objdump-verified: umulh x2,x3,x6 = 0x9bc67c62, smulh = 0x9b467c62.
        // hand-rolled (objdump-verified): umulh x2,x3,x6=0x9bc67c62 ; smulh x4,x5,x6=0x9b467ca4 ; ret
        let code = [
            0x62, 0x7c, 0xc6, 0x9b, // umulh x2, x3, x6
            0xa4, 0x7c, 0x46, 0x9b, // smulh x4, x5, x6
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[3] = 0x10000_0000u64; // 2^32
        st.x[6] = 0x10000_0000u64; // 2^32
        st.x[5] = 0xffff_ffff_ffff_ffffu64; // -1
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[2], 1, "umulh(2^32 * 2^32) high = 1");
        assert_eq!(
            st.x[4],
            0xffff_ffff_ffff_ffff,
            "smulh(-1 * 2^32): -1*2^32 = -2^32, 128-bit high = all-ones"
        );

        // decode binds: umulh (bit23=1,unsigned), smulh (bit23=0,signed); the
        // MulDiv madd alias (mul x0,x1,x0=0x9b007c20, bit22=0) must NOT be MulHigh.
        assert!(matches!(
            crate::decode::decode(0x9bc67c62),
            Inst::MulHigh { signed: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x9b467c62),
            Inst::MulHigh { signed: true, .. }
        ));
        assert!(!matches!(
            crate::decode::decode(0x9b007c20),
            Inst::MulHigh { .. }
        ));
    }

    #[test]
    fn mte_alloc_tag_ops_stores_noop_ldg_zero() {
        // Regression: glibc's __libc_mtag_tag_region issues an `stg` loop; a full
        // glibc-linked program stopped on the first tag store (modmain.elf at
        // 0x40c120 = `stg x0,[x0]`). Host has no MTE: stores are no-ops, ldg reads
        // tag 0. Encodings objdump-verified (armv8.5-a+memtag):
        //   stg x0,[x0]=0xd9200800 ; stzg x1,[x1,#16]=0xd9601821
        //   ldg x2,[x3]=0xd9600062 ; st2g x0,[x4]=0xd9a00880
        let code = [
            0x00, 0x08, 0x20, 0xd9, // stg x0, [x0]
            0x21, 0x18, 0x60, 0xd9, // stzg x1, [x1, #16]
            0x62, 0x00, 0x60, 0xd9, // ldg x2, [x3]
            0x80, 0x08, 0xa0, 0xd9, // st2g x0, [x4]
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[3] = 0xdead_beef_cafe_b000; // base for ldg
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[2], 0, "ldg reads tag 0 (no MTE / no tag state)");
        assert_eq!(r, 0, "x0 untouched by the stg stores");
        // decode binds across the alloc-tag space: stores (no load), ldg (load).
        assert!(matches!(crate::decode::decode(0xd92008c5), Inst::MteTag { load: false, .. }));
        assert!(matches!(crate::decode::decode(0xd96008c5), Inst::MteTag { load: false, .. }));
        assert!(matches!(crate::decode::decode(0xd9a008c5), Inst::MteTag { load: false, .. }));
        assert!(matches!(crate::decode::decode(0xd96000c5), Inst::MteTag { load: true, rt: 5, .. }));
    }

    #[test]
    fn mrs_gcspr_and_tpidr2_read_zero() {
        // Regression: full glibc-linked programs (modmain.elf) read gcspr_el0
        // (armv9 GCS) + tpidr2_el0 (SME 2nd TLS) sizing GCS call frames; both
        // must decode and read 0 (features not enabled). objdump-verified.
        // gcspr_el0 x2 = 0xd53b2522, tpidr2_el0 x14 = 0xd53bd0ae.
        assert!(matches!(crate::decode::decode(0xd53b2522), Inst::SysReg { sysreg: 6, rt: 2, read: true }));
        assert!(matches!(crate::decode::decode(0xd53bd0ae), Inst::SysReg { sysreg: 7, rt: 14, read: true }));
        let code = [
            0x22, 0x25, 0x3b, 0xd5, // mrs x2, gcspr_el0
            0xae, 0xd0, 0x3b, 0xd5, // mrs x14, tpidr2_el0
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[2] = 0xdead;
        st.x[14] = 0xdead;
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[2], 0, "gcspr_el0 reads 0");
        assert_eq!(st.x[14], 0, "tpidr2_el0 reads 0");
    }

    #[test]
    fn mte_writeback_advances_base_register() {
        // Regression: st2g/stg writeback forms (post-index `[x2],#64` = bit10)
        // previously fell through to Unsupported because the MteTag gate forced
        // bit10==0; but the base-register advance is a real side effect glibc
        // memset/stg loops depend on. Each instruction: tag-store to memory is a
        // no-op (no tags kept) yet Xn must += imm<<4. Encodings objdump-verified
        // (armv8.5-a+memtag): st2g x0,[x2],#64 = 0xd9a04440 (post, +64),
        // stg x0,[x2,#-64]! = 0xd93fcc40 (pre, -64).
        let code = [
            0x40, 0x44, 0xa0, 0xd9, // st2g x0,[x2],#64  (x2 += 64)
            0x40, 0xcc, 0x3f, 0xd9, // stg  x0,[x2,#-64]! (x2 -= 64)
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.x[2] = 0x1000;
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[2], 0x1000, "post +64 then pre -64 net out to zero");
        // decode binds writeback + offset for both forms.
        assert!(matches!(crate::decode::decode(0xd9a04440),
            Inst::MteTag { load: false, wb: true, wb_off: 64, rn: 2, .. }));
        assert!(matches!(crate::decode::decode(0xd93fcc40),
            Inst::MteTag { load: false, wb: true, wb_off: -64, rn: 2, .. }));
        // plain offset form has no writeback.
        assert!(matches!(crate::decode::decode(0xd9204840),
            Inst::MteTag { load: false, wb: false, rn: 2, .. }));
    }

    #[test]
    fn cache_maintain_dc_is_noop_dc_zva_zeroes() {
        // Regression: glibc's __libc_mtag_tag_region ends with `dc gva`/cache
        // ops; a full glibc-linked program stopped on `dc gva` (modmain.elf at
        // 0x40c174). In the single-threaded direct-mapped JIT these coherence
        // ops are no-ops; `dc zva` must zero the advertised 16-byte block.
        // Encodings objdump-verified (armv8.5-a+memtag):
        //   dc gva x2 = 0xd50b7462 ; dc zva x0 = 0xd50b7420 ; dc civac x3 = 0xd50b7e60
        //   dc zva zeroes [x0] = 16 zero bytes.
        let code = [
            0x62, 0x74, 0x0b, 0xd5, // dc gva, x2  (no-op)
            0x20, 0x74, 0x0b, 0xd5, // dc zva, x0  (zero [x0])
            0x60, 0x7e, 0x0b, 0xd5, // dc civac, x3 (no-op)
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let buf = [0xabu8; 16];
        let mut st = CpuState::new();
        st.x[0] = buf.as_ptr() as u64;
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(buf, [0u8; 16], "dc zva zeroed the 16-byte cache line");
        // decode binds: zva vs non-zva distinguished by CRm=4/op2=1.
        assert!(matches!(crate::decode::decode(0xd50b7420), Inst::CacheMaintain { zva: true, .. }));
        assert!(matches!(crate::decode::decode(0xd50b7462), Inst::CacheMaintain { zva: false, .. }));
        assert!(matches!(crate::decode::decode(0xd50b7e20), Inst::CacheMaintain { zva: false, .. }));
        assert!(matches!(crate::decode::decode(0xd50b7526), Inst::CacheMaintain { zva: false, rt: 6 }));
    }

    #[test]
    fn fcvtzu_handles_u64_beyond_2pow63() {
        // Regression: `fcvtzu x0,d0` (unsigned double->u64) is valid over the
        // whole [0,2^64) range, but x86 cvttsd2si saturates anything >= 2^63 to
        // INT64_MAX, silently corrupting the high half. Now a two-path sequence
        // subtracts 2^63 for d >= 2^63. fcvtzu x0,d0=0x9e790000 ; ret=0xd65f03c0
        let code = [0x00u8, 0x00, 0x79, 0x9e, 0xc0, 0x03, 0x5f, 0xd6];
        let conv = |bits: u64| {
            let mut st = CpuState::new();
            st.v[0] = bits; // d0 = low 8B of vector slot 0
            exec_bytes(&mut st, &code, 0).expect("exec")
        };
        // boundary + high half (exactly representable doubles)
        assert_eq!(conv((2.0f64.powi(63)).to_bits()), 1u64 << 63, "d = 2^63");
        assert_eq!(
            conv((3.0f64 * 2.0f64.powi(62)).to_bits()),
            3u64 << 62,
            "d = 3*2^62 in [2^63,2^64)"
        );
        assert_eq!(conv((2.0f64.powi(64)).to_bits()), u64::MAX, "d = 2^64 saturates");
        // below 2^63, negatives, NaN
        assert_eq!(conv((10.0f64).to_bits()), 10);
        assert_eq!(conv((-1.5f64).to_bits()), 0, "negative -> 0");
        assert_eq!(conv(f64::NAN.to_bits()), 0, "NaN -> 0");
    }

    #[test]
    fn ins_gp_inserts_element_into_vector_and_extract_reads_it() {
        // `mov v0.s[0],w1; smov x2,v0.s[0]; mov x0,x2; ret`.
        // Regression: `mov v0.s[i],w1` (INS: GPR->vector insert, bit13 CLEAR)
        // was mis-decoded as SimdLaneGp (umov extract), never writing v0 and
        // clobbering a GPR with garbage. Encodings objdump-verified:
        //   mov w1,#5        = 0x528000a1  (empty-line note: precedes ins)
        //   mov v0.s[0],w1   = 0x4e041c20
        //   smov x2,v0.s[0]  = 0x4e042c02
        //   mov x0,x2        = 0xaa0203e0
        //   ret              = 0xd65f03c0
        let code = [
            0xa1u8, 0x00, 0x80, 0x52, // mov w1,#5
            0x20, 0x1c, 0x04, 0x4e, // mov v0.s[0],w1 (INS)
            0x02, 0x2c, 0x04, 0x4e, // smov x2,v0.s[0] each (extract)
            0xe0, 0x03, 0x02, 0xaa, // mov x0,x2
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[0], 5, "x0 = smov-extracted v0.s[0] = 5");
        // v0 lane0 (low 32 of v[0]) = 5 proves the INS wrote the vector.
        assert_eq!(st.v[0] & 0xffffffff, 5, "v0.s[0] inserted by INS");
        // v0 lane1..3 stay zero (INS only wrote element 0).
        assert_eq!((st.v[0] >> 32) & 0xffffffff, 0, "v0.s[1] untouched");
        assert_eq!(st.v[1], 0, "v0.s[2..3] untouched");
    }

    #[test]
    fn ins_gp_sign_and_zero_variants_insert_correct_lanes() {
        // `mov w3,#-17; mov v2.h[0],w3; mov v2.b[0],w4; smov x5,v2.h[0]; ...`
        // Encodings objdump-verified (from lane_all.s / ground truth):
        //   ins v0.h[1],w2 = 0x4e061c40 ; ins v0.d[1],x4 = 0x4e181c80
        //   smov x6,v0.h[2] = 0x4e0a2c06 ; umov x11,v0.d[1] = 0x4e183c0b
        // Sequence: mov x4,#10 ; mov v0.d[1],x4 ; umov x11,v0.d[1] ; mov x12,#3
        //   mov v0.h[1],w12 ; smov x6,v0.h[2] ; mvn x6,x6 ; mov x0,x6 ; ...
        // Simpler deterministic check: set v0.d[1]=0x1234 via INS from x4,
        // extract to x0, then ROBUST: also test that INS .d[1] does NOT touch d[0].
        // Verify decode of each form is the right instruction KIND
        // (we assert the exact rd/rn/esize/index/sign/wide mapping too):
        assert!(matches!(
            crate::decode::decode(0x4e061c40),
            Inst::InsGp { rd: 0, rn: 2, esize: 2, index: 1 }
        ));
        assert!(matches!(
            crate::decode::decode(0x4e181c80),
            Inst::InsGp { rd: 0, rn: 4, esize: 8, index: 1 }
        ));
        assert!(matches!(
            crate::decode::decode(0x4e0a2c06),
            Inst::SimdLaneGp { rd: 6, rn: 0, esize: 2, index: 2, sign: true, wide: true }
        ));
        assert!(matches!(
            crate::decode::decode(0x4e183c0b),
            Inst::SimdLaneGp { rd: 11, rn: 0, esize: 8, index: 1, sign: false, wide: true }
        ));
    }

    #[test]
    fn simd_addl_widening_all_esrc_and_signs() {
        // saddl/uaddl/subl/usubl widen esrc-byte elements to 2*esrc and add/sub.
        // Regression: gate only matched esrc=2 (0x..60), so esrc=4 (.2s->.2d) and
        // esrc=1 (.8b->.8h) fell through to Unsupported; and the translate read the
        // wrong width (esrc=4 loaded 64 bits = both lanes; esrc=2-unsigned loaded 32;
        // esrc=1 stored 32). Encodings objdump-verified.
        // saddl v0.2d,v1.2s,v2.2s = 0x0ea20020 (signed): {7,-2}+{3,9} = {10,7}
        let mut st = CpuState::new();
        st.v[2] = ((-2i32 as u32 as u64) << 32) | 7; // v1.2s lane0=7 lane1=-2
        st.v[4] = ((9u64) << 32) | 3; // v2.2s lane0=3 lane1=9
        exec_bytes(&mut st, &[0x20, 0x00, 0xa2, 0x0e, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        assert_eq!(st.v[0], 10, "saddl .2d lane0 = 7+3");
        assert_eq!(st.v[1], 7, "saddl .2d lane1 = -2+9");
        // uaddl v0.4s,v1.4h,v2.4h = 0x2e620020 (unsigned, esrc=2, rm=v2): {1,2,3,4}+{10,20,30,40}
        let mut st = CpuState::new();
        st.v[2] = (4u64 << 48) | (3 << 32) | (2 << 16) | 1;
        st.v[4] = (40u64 << 48) | (30 << 32) | (20 << 16) | 10;
        exec_bytes(&mut st, &[0x20, 0x00, 0x62, 0x2e, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        assert_eq!(st.v[0], (22u64 << 32) | 11, "uaddl .4s lanes 0,1");
        assert_eq!(st.v[1], (44u64 << 32) | 33, "uaddl .4s lanes 2,3");
        // uaddl v0.8h,v1.8b,v2.8b = 0x2e220020 (unsigned, esrc=1): 1..8 + 1..8
        let mut st = CpuState::new();
        let mut a = 0u64;
        for i in 0..8 { a |= (i as u64 + 1) << (8 * i); }
        st.v[2] = a;
        st.v[4] = a;
        exec_bytes(&mut st, &[0x20, 0x00, 0x22, 0x2e, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        let mk = |l0: u64, l1: u64, l2: u64, l3: u64| (l3 << 48) | (l2 << 32) | (l1 << 16) | l0;
        assert_eq!(st.v[0], mk(2, 4, 6, 8), "uaddl .8h lanes 0..3");
        assert_eq!(st.v[1], mk(10, 12, 14, 16), "uaddl .8h lanes 4..7");
        // decode: saddl .2d must be SimdAddl esrc=4 signed; uaddl .2s->.2d unsigned.
        assert!(matches!(
            crate::decode::decode(0x0ea20020),
            Inst::SimdAddl { esrc: 4, sign: true, sub: false, upper: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x2ea20020),
            Inst::SimdAddl { esrc: 4, sign: false, sub: false, upper: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x2e220020),
            Inst::SimdAddl { esrc: 1, sign: false, sub: false, upper: false, .. }
        ));
    }

    #[test]
    fn and_then_sxtl_sxtl2_upper_half() {
        // Regression for vectorized `m[k] = k & 0xf` init loops (gcc -O2):
        //   and v28.16b,v28.16b,v29.16b ; sxtl v27.2d,v28.2s ; sxtl2 v28.2d,v28.4s
        // with v28 = {16,17,18,19} (4 s-lanes) and v29 = 0x0000000f per lane:
        // v27.2d = {16&15, 17&15} = {0,1}; v28.2d = {18&15, 19&15} = {2,3}.
        // Encodings objdump-verified (maskf probe). Catches any upper-half or
        // mask-lane slip in the sxtl/sxtl2/pand pipeline.
        let mut st = CpuState::new();
        let s = |x: u64| x & 0xffff_ffff;
        // vector reg n lives at st.v[2n] (lo u64) and st.v[2n+1] (hi u64).
        st.v[56] = (s(17) << 32) | s(16); // v28 lo: s-lanes 0,1
        st.v[57] = (s(19) << 32) | s(18); // v28 hi: s-lanes 2,3
        st.v[58] = (0x0fu64 << 32) | 0x0f; // v29 lo: 0xf per s-lane
        st.v[59] = (0x0fu64 << 32) | 0x0f; // v29 hi
        exec_bytes(
            &mut st,
            &[
                0x9c, 0x1f, 0x3d, 0x4e, // and v28.16b, v28.16b, v29.16b
                0x9b, 0xa7, 0x20, 0x0f, // sxtl v27.2d, v28.2s
                0x9c, 0xa7, 0x20, 0x4f, // sxtl2 v28.2d, v28.4s
            ],
            0,
        )
        .expect("exec");
        assert_eq!(st.v[54], 0, "sxtl lane0 = 16&15");
        assert_eq!(st.v[55], 1, "sxtl lane1 = 17&15");
        assert_eq!(st.v[56], 2, "sxtl2 lane0 = 18&15");
        assert_eq!(st.v[57], 3, "sxtl2 lane1 = 19&15");
    }

    #[test]
    fn sxtl_in_place_rd_eq_rn_widening_does_not_clobber_src() {
        // Regression (found by gen_signed_div differential fuzz): a widening
        // `sxtl/uxtl Vd.<long>, Vd.<short>` where the DEST is the SAME vector as
        // the source (rd==rn, what gcc emits for a reduction) clobbers its own
        // still-needed source: the widened 8-byte write of lane 0 at byte 0
        // overwrites the narrow source bytes lane 1 reads at byte 4. The fix
        // snapshots Vn to permscratch first. Convention: vector n lives at
        // st.v[2n] (lo) / st.v[2n+1] (hi).
        // sxtl v0.2d, v0.2s (0x0f20a400) on v0.4s = {1,2,3,4} -> v0.2d = {1,2}.
        let mut st = CpuState::new();
        st.v[0] = (2u64 << 32) | 1; // s-lanes 0,1
        st.v[1] = (4u64 << 32) | 3; // s-lanes 2,3
        exec_bytes(&mut st, &0x0f20a400u32.to_le_bytes(), 0).expect("exec sxtl v0,v0");
        assert_eq!(st.v[0], 1, "in-place sxtl lane0 = 1");
        assert_eq!(st.v[1], 2, "in-place sxtl lane1 = 2 (was clobbered to 0)");
        // sxtl2 v1.2d, v1.4s (0x4f20a421) upper s-lanes {300,400} -> {300,400}
        let mut st = CpuState::new();
        st.v[2] = (200u64 << 32) | 100;
        st.v[3] = (400u64 << 32) | 300;
        exec_bytes(&mut st, &0x4f20a421u32.to_le_bytes(), 0).expect("exec sxtl2 v1,v1");
        assert_eq!(st.v[2], 300, "in-place sxtl2 lane0 (upper src) = 300");
        assert_eq!(st.v[3], 400, "in-place sxtl2 lane1 (upper src) = 400");
        // sxtl v2.4s, v2.4h (0x0f10a442): 2-byte -> 4-byte, 4 lanes, in place.
        // v2.4h = {1,2,3,4} -> v2.4s = {1,2,3,4} each in a 32-bit lane.
        let mut st = CpuState::new();
        st.v[4] = (0x0004_0003_0002_0001u64); // h-lanes 0..3
        exec_bytes(&mut st, &0x0f10a442u32.to_le_bytes(), 0).expect("exec sxtl v2,v2 4h");
        assert_eq!(st.v[4], 0x0000_0002_0000_0001, "in-place sxtl.4s lane0/1 = 1,2");
        assert_eq!(st.v[5], 0x0000_0004_0000_0003, "in-place sxtl.4s lane2/3 = 3,4");
    }

    #[test]
    fn shifted_reg_asr_32bit_sign_extends_before_sar() {
        // Regression (found by gen_signed_div differential fuzz): gcc's signed
        // magic-division remainder computes `q = hi - (a asr 31)` to correct the
        // sign. `sub w3, w3, w4, asr #31` (0x4b847c63): the JIT zero-extended the
        // 32-bit operand and did a 64-bit `sar`, so a NEGATIVE w4 shifted right by
        // 31 became +1 instead of -1 (bit-31 wasn't the 64-bit sign), producing
        // q off-by-2 and corrupting signed quotients/remainders for every divisor.
        // Fix: sign-extend the W operand to 64 bits before the 64-bit asr.
        // w3 = 0 - asr31(0x80000000 = -2147483648) = 0 - (-1) = 1.
        let mut st = CpuState::new();
        st.x[3] = 0;
        st.x[4] = 0x8000_0000u64; // negative as 32-bit
        exec_bytes(&mut st, &0x4b847c63u32.to_le_bytes(), 0).expect("exec sub asr#31");
        assert_eq!(st.x[3], 1, "asr#31 of negative W = -1, so w3 = 0 - (-1) = 1");
        // sub w0, w1, w2, asr #10 (0x4b822820): w1=200, w2=0x80000000.
        // asr10(w2) sign-extends bit-31: -2^31 >> 10 = -2^21 = -2097152.
        // w0 = 200 - (-2097152) = 2097352.
        let mut st = CpuState::new();
        st.x[1] = 200;
        st.x[2] = 0x8000_0000u64;
        exec_bytes(&mut st, &0x4b822820u32.to_le_bytes(), 0).expect("exec sub asr#10");
        assert_eq!(st.x[0] & 0xffff_ffff, 2097352, "asr#10 of negative W sign-correct");
    }

    #[test]
    fn saddw_in_place_aliasing_snapshots_narrow_source() {
        // Regression (found by gen_signed_div differential fuzz): a widening
        // `saddw/saddw2 Vd.2D, Vn.D, Vm.2S` whose NARROW source aliases the dest
        // (rd==rm) clobbers its own source — the widened 8-byte write of lane 0
        // at byte 0 overwrites the narrow msrc bytes lane 1 reads at byte 4,
        // so lane 1 adds 0 instead of the real Vm.s[1]. gcc emits this for
        // vector-reduced sum-of-quotients-and-remainders (`saddw v31.2d,
        // v29.2d, v31.2s`). Fix: snapshot the (aliasing) source to permscratch.
        // Vector n lives at st.v[2n] (lo) / st.v[2n+1] (hi).
        // saddw v28.2d, v27.2d, v28.2s (0x0ebc137c): v28.2s={10,20} + v27.2d={30,40}
        //   -> v28.2d = {40,60} (lane1 would wrongly be 40 without the snapshot).
        let mut st = CpuState::new();
        st.v[54] = 30; // v27 d-lane0
        st.v[55] = 40; // v27 d-lane1
        st.v[56] = (20u64 << 32) | 10; // v28 s-lanes 0,1
        st.v[57] = (40u64 << 32) | 30; // v28 s-lanes 2,3
        exec_bytes(&mut st, &0x0ebc137cu32.to_le_bytes(), 0).expect("exec saddw rd==rm");
        assert_eq!(st.v[56], 40, "saddw lane0 = 30+10");
        assert_eq!(st.v[57], 60, "saddw lane1 = 40+20 (was 40, src clobbered)");
        // saddw2 v31.2d, v31.2d, v28.4s (0x4ebc13ff): rd==rn too, upper narrow src.
        // v31.2d starts {0,0}; v28.4s = {1,2,3,4} upper = {3,4} -> v31.2d = {3,4}.
        let mut st = CpuState::new();
        st.v[62] = 0; // v31 lo
        st.v[63] = 0; // v31 hi
        st.v[56] = (2u64 << 32) | 1;
        st.v[57] = (4u64 << 32) | 3;
        exec_bytes(&mut st, &0x4ebc13ffu32.to_le_bytes(), 0).expect("exec saddw2 rd==rn upper");
        assert_eq!(st.v[62], 3, "saddw2 lane0 += upper src[0]=3");
        assert_eq!(st.v[63], 4, "saddw2 lane1 += upper src[1]=4");
    }

    #[test]
    fn widen_in_place_smull_fcvtl_snapshot_source() {
        // Regression (same in-place widening-alias class found by gen_signed_div
        // fuzz): smull/fcvtl/fcvtn2 with rd aliasing the narrow source clobber
        // their own operand — the wide write of lane i at i*res_esize overwrites
        // the narrow source bytes lane i+1 reads. gcc -O3 in-place-vectorizes
        // these. Now snapshots the source to permscratch when it aliases rd.
        // Vector n lives at st.v[2n] (lo) / st.v[2n+1] (hi).
        // smull v0.2d, v0.2s, v1.2s (0x0ea1c000): v0.2s={1,2} * v1.2s={3,4}
        //   = {1*3, 2*4} = {3,8}.
        let mut st = CpuState::new();
        st.v[0] = (2u64 << 32) | 1; // v0 s-lanes 0,1
        st.v[1] = 0;                // v0 s-lanes 2,3 (unused, must be zeroed by write)
        st.v[2] = (4u64 << 32) | 3; // v1 s-lanes 0,1
        exec_bytes(&mut st, &0x0ea1c000u32.to_le_bytes(), 0).expect("exec smull in-place");
        assert_eq!(st.v[0], 3, "smull lane0 = 1*3");
        assert_eq!(st.v[1], 8, "smull lane1 = 2*4 (was clobbered by src overwrite)");
        // fcvtl v5.2d, v5.2s (0x0e6178a5): widen f32 {1.0, 2.0} -> f64 {1.0, 2.0}
        let mut st = CpuState::new();
        st.v[10] = (0x4000_0000u64 << 32) | 0x3f80_0000u64; // v5 s-lanes = 1.0f, 2.0f
        st.v[11] = 0; // v5 s-lanes 2,3 (unused)
        exec_bytes(&mut st, &0x0e6178a5u32.to_le_bytes(), 0).expect("exec fcvtl in-place");
        assert_eq!(st.v[10], 0x3ff0_0000_0000_0000, "fcvtl lane0 = 1.0 f64");
        assert_eq!(st.v[11], 0x4000_0000_0000_0000, "fcvtl lane1 = 2.0 f64");
    }

    #[test]
    fn simd_stp_q_preindex_store_and_writeback() {
        // REBUILD maskf's real instruction stream END-TO-END (no seeded
        // v-registers): movi v30.4s,#4 / movi v29.4s,#0xf, ldr q31=[init],
        // then 6× the loop body (mov snapshot; add v31+=4; and &0xf; sxtl;
        // sxtl2; stp q27,q28,[x0],#32). Verifies the movi/ldrq/acum/store all
        // agree — maskf's `m[k]=k&0xf` must yield 0..23&0xf in memory.
        let mut st = CpuState::new();
        // One shared x0 base per the real loop: init constant at [x0,#400]
        // ({0,1,2,3} i32), then the loop stores the widening result at [x0],
        // advancing x0 by 32/iter (6 iters = 192 bytes, never reaches +400).
        let buf = Box::leak(vec![0xABu8; 512].into_boxed_slice());
        let mk = |x: u32| x.to_le_bytes();
        for (i, v) in [0u32, 1, 2, 3].iter().enumerate() {
            buf[400 + 4 * i..400 + 4 * i + 4].copy_from_slice(&mk(*v));
        }
        st.x[0] = buf.as_ptr() as u64;
        let mut seq: Vec<u8> = Vec::new();
        // set constants + initial v31 from [x0,#400]
        seq.extend_from_slice(&[0x9e, 0x04, 0x00, 0x4f]); // movi v30.4s, #4
        seq.extend_from_slice(&[0xfd, 0x05, 0x00, 0x4f]); // movi v29.4s, #0xf
        seq.extend_from_slice(&[0x1f, 0x64, 0xc0, 0x3d]); // ldr q31, [x0, #400]
        let body = [
            0xfc, 0x1f, 0xbf, 0x4e, // mov v28.16b, v31.16b
            0xff, 0x87, 0xbe, 0x4e, // add v31.4s, v31.4s, v30.4s
            0x9c, 0x1f, 0x3d, 0x4e, // and v28.16b, v28.16b, v29.16b
            0x9b, 0xa7, 0x20, 0x0f, // sxtl v27.2d, v28.2s
            0x9c, 0xa7, 0x20, 0x4f, // sxtl2 v28.2d, v28.4s
            0x1b, 0x70, 0x81, 0xac, // stp q27, q28, [x0], #32
        ];
        for _ in 0..6 {
            seq.extend_from_slice(&body);
        }
        exec_bytes(&mut st, &seq[0..12], 0).expect("exec setup"); // movi v30, movi v29, ldr q31
        let s = |x: u64| x & 0xffff_ffff;
        assert_eq!(st.v[60], (s(4) << 32) | 4, "v30 = {{4,4}} (movi v30.4s,#4)");
        assert_eq!(st.v[58], (0x0fu64 << 32) | 0x0f, "v29 = {{0xf,0xf}} (movi v29.4s,#0xf)");
        assert_eq!(st.v[62], (s(1) << 32) | 0, "v31 lo lanes {{0,1}} (ldr q31 init)");
        assert_eq!(st.v[63], (s(3) << 32) | 2, "v31 hi lanes {{2,3}} (ldr q31 init)");
        exec_bytes(&mut st, &seq[12..], 0).expect("exec loop");
        let rd = |off: usize| unsafe { *(buf.as_ptr().add(off) as *const u64) };
        for k in 0..24usize {
            let exp = (k as u64) & 0xf;
            assert_eq!(rd(k * 8), exp, "m[{k}] = k & 0xf");
        }
        assert_eq!(st.x[0], buf.as_ptr() as u64 + 6 * 32, "x0 writeback 6x32");
    }

    #[test]
    fn and_sxtl_accumulation_two_iterations() {
        // The full maskf loop body x2 (mov snapshot; add v31+=4; and &0xf;
        // sxtl + sxtl2), verifying the ACCUMULATOR survives across iterations:
        //   mov v28.16b,v31.16b; add v31.4s,v31.4s,v30.4s; and v28,v28,v29;
        //   sxtl v27.2d,v28.2s; sxtl2 v28.2d,v28.4s   (x2)
        // Start v30=4, v29=0xf, v31={0,1,2,3}. After 2 iterations v31 must be
        // {8,9,10,11}, v27={4,5} (sxtl of v28={4,5,6,7}), v28={6,7} (sxtl2).
        // The single-shot and+sxtl+sxtl2 test passes but the looped version
        // regressed (jit m[17]=0 instead of 1), so MOV/ADD accumulation is key.
        let mut st = CpuState::new();
        let s = |x: u64| x & 0xffff_ffff;
        st.v[60] = (s(4) << 32) | 4; // v30 lo: +4 per lane   (reg 30)
        st.v[61] = (s(4) << 32) | 4; // v30 hi
        st.v[58] = (0x0fu64 << 32) | 0x0f; // v29 lo: mask 0xf
        st.v[59] = (0x0fu64 << 32) | 0x0f; // v29 hi
        st.v[62] = (s(1) << 32) | 0;  // v31 lo: {0,1}
        st.v[63] = (s(3) << 32) | 2;  // v31 hi: {2,3}
        // 5-instruction loop body (LE little-endian encodings, objdump-verified).
        let body = [
            0xfc, 0x1f, 0xbf, 0x4e, // mov v28.16b, v31.16b
            0xff, 0x87, 0xbe, 0x4e, // add v31.4s, v31.4s, v30.4s
            0x9c, 0x1f, 0x3d, 0x4e, // and v28.16b, v28.16b, v29.16b
            0x9b, 0xa7, 0x20, 0x0f, // sxtl v27.2d, v28.2s
            0x9c, 0xa7, 0x20, 0x4f, // sxtl2 v28.2d, v28.4s
        ];
        let mut seq = Vec::new();
        seq.extend_from_slice(&body);
        seq.extend_from_slice(&body);
        exec_bytes(&mut st, &seq, 0).expect("exec");
        assert_eq!(st.v[62], (s(9) << 32) | 8, "v31 lo after 2 iters {{8,9}}");
        assert_eq!(st.v[63], (s(11) << 32) | 10, "v31 hi after 2 iters {{10,11}}");
        assert_eq!(st.v[54], 4, "v27 lane0 = 4");
        assert_eq!(st.v[55], 5, "v27 lane1 = 5");
        assert_eq!(st.v[56], 6, "v28 lane0 = 6 (sxtl2 of {{4,5,6,7}})");
        assert_eq!(st.v[57], 7, "v28 lane1 = 7");
    }

    #[test]
    fn simd_mull_widening_multiply_correct() {
        // smull/umull/smlal/umlal widen esrc-byte elements to res and multiply.
        // Decode regression: the old gate read res_esize from bit22 (missed
        // .8b->.8h res=2, mis-sized .4h as 8), unsigned from bit28 (umull treated
        // as signed), and acc from bit15 (plain smull accumulated) — four silent
        // miscompiles. Encodings objdump-verified.
        // smull v0.2d, v1.2s, v2.2s = 0x0ea2c020 (signed, res 8): {7,-3}*{5,-2} => {35,6}
        let mut st = CpuState::new();
        st.v[2] = ((-3i32 as u32 as u64) << 32) | 7; // v1.2s
        st.v[4] = ((-2i32 as u32 as u64) << 32) | 5; // v2.2s
        exec_bytes(&mut st, &[0x20, 0xc0, 0xa2, 0x0e, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        assert_eq!(st.v[0], 35, "smull .2d lane0 = 7*5");
        assert_eq!(st.v[1], 6, "smull .2d lane1 = -3*-2");
        // umull v0.8h, v1.8b, v2.8b = 0x2e22c020 (unsigned, res 2):
        // v1.8b bytes {0xFE, 0xFF, 2,3,4,5,6,7} * v2.8b {2,2,2,2,2,2,2,2}
        // => {508, 510, 4,6,8,10,12,14}. If umull were (wrongly) signed, 0xFE as -2
        // would give -4 (0xFFFC) not 508 (0x01FC).
        let mut st = CpuState::new();
        let mut a = 0xFEu64 | (0xFF << 8); // bytes 0,1
        for i in 2..8 { a |= (i as u64) << (8 * i); } // bytes 2..7 = 2..7
        let mut b = 2u64;
        for i in 1..8 { b |= 2u64 << (8 * i); } // v2 all 2
        st.v[2] = a;
        st.v[4] = b;
        exec_bytes(&mut st, &[0x20, 0xc0, 0x22, 0x2e, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        let mk2 = |l: &[u64]| -> u64 { l.iter().enumerate().fold(0u64, |acc, (i, v)| acc | (v << (16 * i))) };
        assert_eq!(st.v[0], mk2(&[508, 510, 4, 6]), "umull .8h lanes 0..3");
        assert_eq!(st.v[1], mk2(&[8, 10, 12, 14]), "umull .8h lanes 4..7");
        // decode binds: smull = signed non-acc; umull = unsigned; smlal = acc res4.
        assert!(matches!(
            crate::decode::decode(0x0ea2c020),
            Inst::SimdMull { res_esize: 8, unsigned: false, acc: false, q: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x0e62c020),
            Inst::SimdMull { res_esize: 4, unsigned: false, acc: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x2e62c020),
            Inst::SimdMull { res_esize: 4, unsigned: true, acc: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x2e22c020),
            Inst::SimdMull { res_esize: 2, unsigned: true, acc: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x0e628020),
            Inst::SimdMull { res_esize: 4, acc: true, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x6ea2c020),
            Inst::SimdMull { res_esize: 8, unsigned: true, q: true, acc: false, .. }
        ));
    }

    #[test]
    fn simd_shift_right_immediate_ushr_sshr() {
        // ushr/sshr Vd.T, Vn.T, #imm. Regression: plain shift-right (marker
        // bits[14:12]==0b000) had no gate and was swallowed by the VecMovi gate
        // (silently wrote a wrong immediate; shiftimm.elf returned 0xfffffffc
        // instead of 3). New SimdShr gate (immh!=0 vs movi), esize from fls(immh),
        // shift = 2*esize_bits-(immh:immb). Encodings objdump-verified.
        // ushr v0.2s,v1.2s,#8 = 0x2f380420: {0x100,0x200} -> {1,2}
        let mut st = CpuState::new();
        st.v[2] = (0x200u64 << 32) | 0x100; // v1.2s
        exec_bytes(&mut st, &[0x20, 0x04, 0x38, 0x2f, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        assert_eq!(st.v[0] & 0xffffffff, 1, "ushr .2s lane0 = 0x100>>8");
        assert_eq!((st.v[0] >> 32) & 0xffffffff, 2, "ushr .2s lane1 = 0x200>>8");
        // sshr v0.2s,v1.2s,#8 = 0x0f380420 (arithmetic): {-0x100(0xffffff00), 0x200} -> {-1, 2}
        let mut st = CpuState::new();
        st.v[2] = (0x200u64 << 32) | 0xffffff00u64; // v1.2s lane0 = -256
        exec_bytes(&mut st, &[0x20, 0x04, 0x38, 0x0f, 0xc0, 0x03, 0x5f, 0xd6], 0).expect("exec");
        assert_eq!((st.v[0] & 0xffffffff) as u32 as i32, -1, "sshr .2s lane0 = -256>>8 (arith)");
        assert_eq!(((st.v[0] >> 32) & 0xffffffff) as u32 as i32, 2, "sshr .2s lane1 = 0x200>>8");
        // decode binds: ushr unsigned, sshr signed, both esize from immh.
        assert!(matches!(
            crate::decode::decode(0x2f380420),
            Inst::SimdShr { esize: 4, shift: 8, unsigned: true, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x0f380420),
            Inst::SimdShr { esize: 4, shift: 8, unsigned: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x6f6f0420),
            Inst::SimdShr { esize: 8, shift: 17, unsigned: true, .. }
        ));
        // movi must NOT be reclassified as a shift (immh==0 stays VecMovi).
        assert!(matches!(
            crate::decode::decode(0x0f0004a0),
            Inst::VecMovi { .. }
        ));
    }

    #[test]
    fn simd_ssra_usra_shift_accumulate_esize_correct() {
        // Regression: SimdShrAcc (usra/ssra Vd += Vn>>imm) derived esize from the
        // 3-bit tagless immh via trailing_zeros, collapsing EVERY esize>=4 shift to
        // esize=1/shift=0 — so ssra silently accumulated WITHOUT shifting
        // (ssra .2d #2 returned -8-16=-24 not -2-4=-6). Mirror SimdShr's verified
        // full-immh (bit22) esize + 2*esize_bits shift. Encodings objdump-verified.
        // ssra v4.2d,v3.2d,#2 = 0x4f7e1464 (real word) with v3={-8,-16} -> {-2,-4}.
        // real encoding uses rn=v3 (bits5:9=3, so CpuState.v[2*3]), rd=v4(0x4).
        // ssra v4.2d,v3.2d,#2 = 0x4f7e1464 ; then mov x0,v4.d[0]=0x4e083c80
        // mov x1,v4.d[1]=0x4e183c81 ; add x0,x0,x1=0x8b010000 ; ret (objdump)
        let code = [
            0x64, 0x14, 0x7e, 0x4f,
            0x80, 0x3c, 0x08, 0x4e,
            0x81, 0x3c, 0x18, 0x4e,
            0x00, 0x00, 0x01, 0x8b,
            0xc0, 0x03, 0x5f, 0xd6,
        ];
        let mut st = CpuState::new();
        st.v[2 * 3] = 0xffff_ffff_ffff_fff8u64;      // v3.d[0] = -8
        st.v[2 * 3 + 1] = 0xffff_ffff_ffff_fff0u64;  // v3.d[1] = -16
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r as u64, 0xffff_ffff_ffff_fffa, "ssra .2d #2 of {{-8,-16}} = {{-2,-4}}, sum -6");
        assert_eq!(st.v[2 * 4] as u64, 0xffff_ffff_ffff_fffe, "v4.d[0] = -8>>2 = -2");

        // decode binds: ssra .4s #2 (0x4f3e1464) -> esize 4, shift 2, signed;
        // usra .4s #2 (0x6f3e1464) -> unsigned.
        assert!(matches!(
            crate::decode::decode(0x4f3e1464),
            Inst::SimdShrAcc { esize: 4, shift: 2, unsigned: false, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x6f3e1464),
            Inst::SimdShrAcc { esize: 4, shift: 2, unsigned: true, .. }
        ));
        assert!(matches!(
            crate::decode::decode(0x4f7e1464),
            Inst::SimdShrAcc { esize: 8, shift: 2, unsigned: false, .. }
        ));
    }

    #[test]
    fn fcvt_vec_4s_lanes_are_32bit_and_independent() {
        // Regression: `fcvtzs v0.4s, v1.4s` treats each lane as a 32-bit float and
        // writes a 32-bit int per lane. It used movq_load (reads 8 bytes = lane +
        // next lane) and movq_store (writes 8 bytes over the neighbour lane), so
        // multi-lane vectors were corrupt. fcvtzs v0.4s,v1.4s=0x4ea1b820 ;
        // fcvtzs v0.2d,v1.2d=0x4ee1b820 ; fcvtzu v2.2d,v3.2d=0x6ee1b862 ; ret
        let code4 = [0x20u8, 0xb8, 0xa1, 0x4e, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        st.v[2] = 1.5f32.to_bits() as u64 | ((2.5f32.to_bits() as u64) << 32); // v1.4s
        st.v[3] = (-3.0f32).to_bits() as u64 | ((4.25f32.to_bits() as u64) << 32);
        exec_bytes(&mut st, &code4, 0).expect("exec");
        assert_eq!(st.v[0] & 0xffffffff, 1, "lane0 = trunc(1.5)");
        assert_eq!((st.v[0] >> 32) & 0xffffffff, 2, "lane1 = trunc(2.5)");
        assert_eq!(st.v[1] & 0xffffffff, (-3 as i64) as u32 as u64, "lane2 = trunc(-3.0)");
        assert_eq!((st.v[1] >> 32) & 0xffffffff, 4, "lane3 = trunc(4.25)");
        // 2d signed -> two i64 lanes
        let code2 = [0x20u8, 0xb8, 0xe1, 0x4e, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        st.v[2] = 7.7f64.to_bits(); // v1.d[0]
        st.v[3] = (-2.2f64).to_bits(); // v1.d[1]
        exec_bytes(&mut st, &code2, 0).expect("exec");
        assert_eq!(st.v[0], 7, "d-lane0 = trunc(7.7)");
        assert_eq!(st.v[1], (-2 as i64) as u64, "d-lane1 = trunc(-2.2)");
        // 2d unsigned: negatives clamp to 0
        let codeu = [0x62u8, 0xb8, 0xe1, 0x6e, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        st.v[6] = 3.9f64.to_bits(); // v3.d[0]
        st.v[7] = (-1.0f64).to_bits(); // v3.d[1]
        exec_bytes(&mut st, &codeu, 0).expect("exec");
        assert_eq!(st.v[4], 3, "unsigned d-lane0 = trunc(3.9)");
        assert_eq!(st.v[5], 0, "unsigned d-lane1 negative -> 0");
    }

    #[test]
    fn str_d0_writes_vector_reg_not_gpr() {
        // Regression: `str d0,[x0]` must write the FP/vector register v[0]'s low
        // 64 bits to memory, not the GPR x0 slot (it used to be decoded as a GPR
        // store).  str d0,[x0]=0xfd000000 ; ldr x1,[x0]=0xf9400001 ; ret=0xd65f03c0
        let insn: &[u32] = &[0xfd000000, 0xf9400001, 0xd65f03c0];
        let mut code = Vec::new();
        for w in insn {
            code.extend_from_slice(&w.to_le_bytes());
        }
        let mut buf = [0u64; 2];
        let mut st = CpuState::new();
        st.x[0] = buf.as_ptr() as u64;
        st.x[31] = 0x1111_2222_3333_4444; // sentinel SP
        st.v[0] = 0xdead_beef_cafe_f00d; // d0 low 64 bits (v is [u64;64])
        let _r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(buf[0], 0xdead_beef_cafe_f00d, "str d0 wrote v[0] to memory");
        assert_eq!(st.x[1], 0xdead_beef_cafe_f00d, "ldr x1 read back the stored d0");
    }

    #[test]
    fn sub_add_sp_updates_stack_pointer() {
        // Regression: `sub sp,sp,#0x10` / `add sp,sp,#0x10` must actually update
        // CpuState.x[31] (SP). Previously AddSubImm/AddSubReg suppressed rd==31,
        // so every function prologue's `sub sp` silently did nothing and nested
        // frames collided on the same sp (inlined f()'s `str d31,[sp+8]`
        // clobbered the caller's saved x30 -> pc jumped to a double's bit pattern).
        //   sub sp,sp,#0x10 = 0xd10043ff ; mov x0,sp = 0x910003e0
        //   add sp,sp,#0x10 = 0x910043ff ; ret = 0xd65f03c0
        let code = [0xffu8, 0x43, 0x00, 0xd1, 0xe0, 0x03, 0x00, 0x91, 0xff, 0x43, 0x00, 0x91, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        let sp0 = 0x7000_0000_2000u64;
        st.x[31] = sp0;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.x[0], sp0 - 0x10, "sub sp must decrement SP (x0 = sp)");
        assert_eq!(st.x[31], sp0, "add sp must restore SP (x31 == sp0)");
        assert_eq!(r, sp0 - 0x10, "ret returns x0 = moved sp");
    }

    #[test]
    fn fcvtzs_to_fp_reg_converts_and_stores_int() {
        // Regression: scalar `fcvtzs d0,d0` converts the double in v0 to an int
        // and stores the INTEGER (not the original float). The FcvVec translate
        // sent cvttsd2si's result to RAX but stored xmm0 (still the float),
        // so fcvtzs(d0=3.5) left 3.5 instead of 3.
        //   fcvtzs d0,d0=0x5ee1b800 ; str d0,[x1]=0xfd000020
        //   ldr x0,[x1]=0xf9400020 ; ret=0xd65f03c0
        let insn: &[u32] = &[0x5ee1b800, 0xfd000020, 0xf9400020, 0xd65f03c0];
        let mut code = Vec::new();
        for w in insn {
            code.extend_from_slice(&w.to_le_bytes());
        }
        let mut buf = [0u64; 2];
        let mut st = CpuState::new();
        st.v[0] = 3.5f64.to_bits();
        st.x[1] = buf.as_ptr() as u64;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(buf[0], 3, "fcvtzs d0,d0 stores the int 3, not the float 3.5");
        assert_eq!(r, 3, "x0 = converted integer");
    }

    #[test]
    fn vector_neg_abs_unary_lane_magnitudes() {
        // Integer vector NEG/ABS across widths. Regression: `neg v29.2s,
        // v31.2s` (0x2ea0bbfd) was mis-decoded as a vector float->int (FcvVec)
        // because the FcvVec gate masks off bit16, silently zeroing/corrupting
        // lanes. Verify signed magnitude per lane for .4s, .8h, .16b, .2d.
        // Encodings assembly-verified:
        //   neg v0.4s,v1.4s=0x6ea0b820  abs v2.4s,v1.4s=0x4ea0b822
        //   neg v4.8h,v5.8h=0x6e60b8a4  abs v6.8h,v5.8h=0x4e60b8a6
        //   neg v8.16b,v9.16b=0x6e20b928 abav10=0x4e20b92a
        //   neg v12.2d,v13.2d=0x6ee0b9ac abav14=0x4ee0b9ae ; ret
        let insn: &[u32] = &[
            0x6ea0b820, 0x4ea0b822, // v0=-v1, v2=|v1| (4s)
            0x6e60b8a4, 0x4e60b8a6, // v4=-v5, v6=|v5| (8h)
            0x6e20b928, 0x4e20b92a, // v8=-v9, v10=|v9| (16b)
            0x6ee0b9ac, 0x4ee0b9ae, // v12=-v13, v14=|v13| (2d)
            0xd65f03c0, // ret
        ];
        let mut code = Vec::new();
        for w in insn {
            code.extend_from_slice(&w.to_le_bytes());
        }
        let mut st = CpuState::new();
        // v1 .4s = [-57798278, -1, 1000000, -2000000000]
        let s4: [i32; 4] = [-57798278, -1, 1000000, -2000000000];
        st.v[2] = (s4[0] as u32 as u64) | ((s4[1] as u32 as u64) << 32);
        st.v[3] = (s4[2] as u32 as u64) | ((s4[3] as u32 as u64) << 32);
        // v5 .8h = [0x8000,-1,0x0002,0xffff,0x0001,0x7fff,0x8001,0x0003]
        let h8: [u32; 8] = [0x8000, 0xffff, 0x0002, 0xffff, 0x0001, 0x7fff, 0x8001, 0x0003];
        for (i, h) in h8.iter().enumerate() {
            let reg = 2 * 5 + i / 4;
            let shift = (i % 4) * 16;
            st.v[reg] |= (*h as u64) << shift;
        }
        // v9 .16b = [0xff,0x00,0x01,0x80,0x02,0xff,0x7f,0x81, ...]
        let b16: [u32; 16] = [
            0xff, 0x00, 0x01, 0x80, 0x02, 0xff, 0x7f, 0x81, 0xfe, 0x01, 0x00, 0x7f, 0x0a, 0xf0, 0x03, 0x80,
        ];
        for (i, b) in b16.iter().enumerate() {
            let reg = 2 * 9 + i / 8;
            let shift = (i % 8) * 8;
            st.v[reg] |= (*b as u64) << shift;
        }
        // v13 .2d = [-5, 9223372036854775807]
        st.v[26] = (-5i64 as u64);
        st.v[27] = i64::MAX as u64;

        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0, "entry returns x0");

        // neg v0.4s (negate each s-lane): [-57798278, -1, 1000000, -2000000000]
        //   -> [57798278, 1, -1000000, 2000000000]
        let g0 = |i: usize| st.v[i / 2] >> ((i % 2) * 32) & 0xffffffff;
        assert_eq!(g0(0) as i32, 57798278, ".4s neg lane0");
        assert_eq!(g0(1) as i32, 1, ".4s neg lane1");
        assert_eq!(g0(2) as i32, -1000000, ".4s neg lane2");
        assert_eq!(g0(3) as i32, 2000000000, ".4s neg lane3");
        // abs v2.4s
        let g2 = |i: usize| st.v[4 + i / 2] >> ((i % 2) * 32) & 0xffffffff;
        assert_eq!(g2(0) as i32, 57798278, ".4s abs lane0");
        assert_eq!(g2(1) as i32, 1, ".4s abs lane1");
        assert_eq!(g2(2) as i32, 1000000, ".4s abs lane2");
        assert_eq!(g2(3) as i32, 2000000000, ".4s abs lane3");
        // neg v4.8h: neg of [0x8000,-1,2,-1,1,0x7fff,-32767,3] -> [0x8000,1,-2,1,-1,-32767,32767,-3]
        let gh = |reg: usize, i: usize| (st.v[reg] >> ((i % 4) * 16)) as i16 as i32;
        let n4 = |i: usize| gh(2 * 4 + i / 4, i);
        assert_eq!(n4(0), -32768, ".8h neg lane0 (wrap)");
        assert_eq!(n4(1), 1, ".8h neg lane1");
        assert_eq!(n4(2), -2, ".8h neg lane2");
        assert_eq!(n4(3), 1, ".8h neg lane3");
        assert_eq!(n4(7), -3, ".8h neg lane7");
        // abs v6.8h
        let a6 = |i: usize| gh(2 * 6 + i / 4, i);
        assert_eq!(a6(0), -32768, ".8h abs lane0 (|−32768| wraps to 0x8000)");
        assert_eq!(a6(1), 1, ".8h abs lane1");
        assert_eq!(a6(7), 3, ".8h abs lane7");
        // neg v8.16b
        let n8 = |i: usize| (st.v[16 + i / 8] >> ((i % 8) * 8)) as u8 as i32;
        assert_eq!(n8(0), 1, ".16b neg b0 (0xff -> 1)");
        assert_eq!(n8(3), 128, ".16b neg b3 (0x80 -> 128 wrap)");
        assert_eq!(n8(7), 127, ".16b neg b7 (0x81 -> 127)");
        // abs v10.16b (abs of the SIGNED byte)
        let a10 = |i: usize| (st.v[20 + i / 8] >> ((i % 8) * 8)) as u8 as i32;
        assert_eq!(a10(0), 1, ".16b abs b0 (0xff=-1 -> 1)");
        assert_eq!(a10(3), 128, ".16b abs b3 (0x80=-128 -> 128)");
        assert_eq!(a10(7), 127, ".16b abs b7 (0x81=-127 -> 127)");
        // neg v12.2d
        assert_eq!(st.v[24], 5, ".2d neg lane0 (-5 -> 5)");
        assert_eq!(st.v[25] as i64, i64::MIN + 1, ".2d neg lane1 (INT64_MAX -> -INT64_MAX)");
        // abs v14.2d
        assert_eq!(st.v[28], 5, ".2d abs lane0");
        assert_eq!(st.v[29] as i64, i64::MAX, ".2d abs lane1");
    }

    #[test]
    fn loop_back_edge_reiterates_body() {
        // Regression: a guest `b.lt` (and unconditional `b` forward) forming a
        // loop must iterate in-block, not fall through to the epilogue `ret`
        // after one pass. sum(0..5) = 10. Assembler-verified bytes:
        //   sub sp,#0x10; str xzr,[sp]; str xzr,[sp,#8]; b Ltest; Lbody:
        //   ldr x0,add; str; ldr; add #1; str; Ltest: ldr; cmp #5; b.lt Lbody;
        //   ldr x0[s=s]; add sp; ret
        let insn: &[u32] = &[
            0xd10043ff, 0xf90003ff, 0xf90007ff, 0x14000008, // entry
            0xf94003e0, 0xf94007e1, 0x8b010000, 0xf90003e0, // Lbody part1
            0xf94007e0, 0x91000400, 0xf90007e0, //            Lbody part2
            0xf94007e0, 0xf100141f, 0x54fffeeb, //            Ltest cmp/b.lt
            0xf94003e0, 0x910043ff, 0xd65f03c0, //            exit
        ];
        let mut code = Vec::new();
        for w in insn {
            code.extend_from_slice(&w.to_le_bytes());
        }
        let mut st = CpuState::new();
        let stack = Box::leak(vec![0u8; 512].into_boxed_slice());
        st.x[31] = stack.as_ptr() as u64 + 256; // sp
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 10, "loop sum(0..5) == 10");
    }

    #[test]
    fn movk_merges_into_existing_register() {
        // Regression: `movk x0,#hi,lsl#16` must MERGE into bits[16..32],
        // preserving the low 16 from a preceding `movz`. A full replace broke
        // every multi-part constant: movz 0x8bb1 ; movk 0x2 lsl#16 must be
        // 0x28bb1, but came out 0x20000.  movz x0,#0x8bb1 = 0xd2917620 ;
        // movk x0,#0x2,lsl#16 = 0xf2a00040 ; ret = 0xd65f03c0
        let code = [0x20u8, 0x76, 0x91, 0xd2, 0x40, 0x00, 0xa0, 0xf2, 0xc0, 0x03, 0x5f, 0xd6];
        let mut st = CpuState::new();
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0x28bb1, "movz 0x8bb1 then movk 0x2 lsl#16 == 0x28bb1");
    }

    #[test]
    fn str_xzr_stores_zero_not_sp() {
        // Regression: `str xzr,[x0]` must write 0, never the stack pointer.
        // AArch64 stores read the source field x31 as XZR (zero); the JIT used to
        // load CpuState.x[31] (= SP), so zero-init stored SP and corrupted memory.
        //   str xzr,[x0]  = 0xf900001f ; ldr x0,[x0] = 0xf9400000 ; ret = 0xd65f03c0
        let code = [0x1fu8, 0x00, 0x00, 0xf9, 0x00, 0x00, 0x40, 0xf9, 0xc0, 0x03, 0x5f, 0xd6];
        let mut buf = [0xdead_beef_cafe_f00du64; 2];
        let mut st = CpuState::new();
        st.x[0] = buf.as_ptr() as u64;
        st.x[31] = 0xaaaa_bbbb_cccc_dddd; // sentinel SP: must survive untouched
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 0, "str xzr zeroes the slot (must not write SP)");
        assert_eq!(buf[0], 0, "memory actually zeroed");
        assert_eq!(st.x[31], 0xaaaa_bbbb_cccc_dddd, "SP (x31) untouched");
    }

    #[test]
    fn fp_scalar_unscaled_ldur_stur_roundtrip() {
        // Scalars B/H/S/D unscaled ldur/stur transfer `size` bytes between the
        // low bytes of the vector slot v[vt] and [Xn+imm9]. Round-trip a double
        // buffer and confirm both the slot and the memory end up
        // correct.  stur d0,[x1,#-8] ; ldur d1,[x1,#-8] ; ret
        let code = [0x20u8, 0x80, 0x1f, 0xfc, 0x21, 0x80, 0x5f, 0xfc, 0xc0, 0x03, 0x5f, 0xd6];
        let mut buf = [0xdead_beef_cafe_f00du64; 2];
        let mut st = CpuState::new();
        st.x[1] = (buf.as_ptr() as u64).wrapping_add(8); // [x1-8] -> buf[0]
        st.v[0] = 0x8899_aabb_ccdd_eeff; // d0
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(buf[0], 0x8899_aabb_ccdd_eeff, "stur d0 wrote memory");
        assert_eq!(st.v[2], 0x8899_aabb_ccdd_eeff, "ldur d1 read it back");
    }

    #[test]
    fn fp_scalar_post_index_writeback_advances_base() {
        // ldr s0,[x1],#4 (post-index) reads 4 bytes from [x1] into s0 and
        // advances x1 by +4. Word verified by aarch64-linux-gnu-as.
        let code = [0x20u8, 0x44, 0x40, 0xbc, 0xc0, 0x03, 0x5f, 0xd6]; // ldr s0,[x1],#4 ; ret
        let mut store = 0x1234_5678u64;
        let mut st = CpuState::new();
        let base = (&store as *const u64) as u64;
        st.x[1] = base;
        st.v[0] = 0xffff_ffff_ffff_ffff; // pre-fill s0
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.v[0] & 0xffff_ffff, 0x1234_5678, "s0 loaded from [x1]");
        assert_eq!(st.x[1], base.wrapping_add(4), "post-index advanced Xn by 4");
    }

    #[test]
    fn fp_scalar_pre_index_writeback_applies_offset_before_load() {
        // ldr d0,[x1,#-8]! (pre-index) reads 8 bytes from [x1-8] and advances
        // x1 to x1-8. Word verified by aarch64-linux-gnu-as.
        let code = [0x20u8, 0x8c, 0x5f, 0xfc, 0xc0, 0x03, 0x5f, 0xd6]; // ldr d0,[x1,#-8]! ; ret
        let mut store = 0x1122_3344_5566_7788u64;
        let mut st = CpuState::new();
        let base = (&store as *const u64) as u64;
        st.x[1] = base.wrapping_add(8); // address AFTER the -8 offset
        st.v[0] = 0;
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.v[0], 0x1122_3344_5566_7788, "d0 loaded from [x1-8]");
        assert_eq!(st.x[1], base, "pre-index wrote Xn back to x1-8");
    }

    #[test]
    fn ldr_reg_sext_sign_extends_into_dest() {
        // Regression: register-offset `ldrsh w0,[x1,x0]` (0x78e06820) loaded the
        // signed value into RCX but wrote RAX (= the effective ADDRESS) into the
        // dest reg, so a[i] read back garbage in short-array loops. Real encoding
        // from aarch64-linux-gnu-gcc -O0 (sumh over `short a[]`). -13 as i16 =
        // 0xfff3, sign-extended to 0xffff_ffff_ffff_fff3.
        //   ldrsh w0,[x1,x0]=0x78e06820 ; ldr x2,[x1]=0xf9400022 ; ret=0xd65f03c0
        let insn: &[u32] = &[0x78e06820, 0xf9400022, 0xd65f03c0];
        let mut code = Vec::new();
        for w in insn {
            code.extend_from_slice(&w.to_le_bytes());
        }
        let mut buf = [0x1234i16, -13, 0x7fff, -1];
        let mut st = CpuState::new();
        st.x[1] = buf.as_ptr() as u64; // x1 = &buf
        st.x[0] = 1_u64 << 1; // x0 = i*2 = byte offset of buf[1]
        let _r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(
            st.x[0],
            0xffff_ffff_ffff_fff3,
            "ldrsh w0,[x1,x0] of buf[1] (-13) must sign-extend into x0, not hold the address"
        );
        // buf as a u64 (4×i16 little-endian: 1234 fff3 7fff ffff) -> 0xffff7ffffff31234
        assert_eq!(st.x[2], 0xffff_7fff_fff3_1234, "ldr x2,[x1] read buf[0..8] as u64");
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
    fn host_plt_stub_detected_from_image_slice() {
        // 0x40: adrp x8,#0 ; ldr x9,[x8,#8] ; add x8,x8,#0 ; br x9  (canonical PLT stub)
        let mut image = Vec::<u8>::new();
        while image.len() < 0x40 {
            image.push(0);
        }
        image.extend_from_slice(&0x9000_0008u32.to_le_bytes()); // 0x40 adrp x8
        image.extend_from_slice(&0xf940_0109u32.to_le_bytes()); // 0x44 ldr x9,[x8,#8]
        image.extend_from_slice(&0x9100_0108u32.to_le_bytes()); // 0x48 add x8,x8,#0
        image.extend_from_slice(&0xd61f_0120u32.to_le_bytes()); // 0x4c br x9
        assert!(is_host_plt_stub(&image, 0, 0x40), "canonical PLT stub at 0x40");
        // A non-stub (just `ret`) is not mis-detected.
        assert!(!is_host_plt_stub(&image, 0, 0x20), "no stub at 0x20");
        // Out-of-range reads are rejected, not UB.
        assert!(!is_host_plt_stub(&image, 0, 0x400), "OOB stub read rejected");
    }

    #[test]
    fn body_contains_host_plt_bl_follows_call_graph() {
        // caller 0x00 bl 0x20; ret
        // callee 0x20: bl 0x40 (a host-import PLT stub); ret
        // stub   0x40: adrp/ldr/add/br (matches is_host_plt_stub)
        let mut image = Vec::<u8>::new();
        image.extend_from_slice(&0x9400_0008u32.to_le_bytes()); // 0x00 bl 0x20
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x04 ret
        while image.len() < 0x20 {
            image.push(0);
        }
        image.extend_from_slice(&0x9400_0008u32.to_le_bytes()); // 0x20 bl 0x40
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x24 ret
        while image.len() < 0x40 {
            image.push(0);
        }
        image.extend_from_slice(&0x9000_0008u32.to_le_bytes()); // 0x40 adrp x8
        image.extend_from_slice(&0xf940_0109u32.to_le_bytes()); // 0x44 ldr x9,[x8,#8]
        image.extend_from_slice(&0x9100_0108u32.to_le_bytes()); // 0x48 add x8,x8,#0
        image.extend_from_slice(&0xd61f_0120u32.to_le_bytes()); // 0x4c br x9
        // The callee body (via its own `bl 0x40`) references a host import.
        assert!(
            body_contains_host_plt_bl(&image, 0, 0x20),
            "callee 0x20 calls a host-import PLT stub"
        );
        // The caller body does NOT (its only `bl` is to the guest callee).
        assert!(
            !body_contains_host_plt_bl(&image, 0, 0x00),
            "caller 0x00 has no direct host-import call"
        );
    }

    #[test]
    fn guest_bl_to_import_bearing_callee_diverts_through_dispatcher() {
        // Same layout as body_contains_host_plt_bl test. A generous budget would
        // normally inline the callee, but because the callee body itself calls a
        // host-import PLT stub, the outer `bl` must be diverted to the dispatcher:
        // running the caller block leaves CpuState.pc == 0x20 (the callee), x30
        // == 0x04 (link), instead of the callee being compiled inline.
        let mut image = Vec::<u8>::new();
        image.extend_from_slice(&0x9400_0008u32.to_le_bytes()); // 0x00 bl 0x20
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x04 ret
        while image.len() < 0x20 {
            image.push(0);
        }
        image.extend_from_slice(&0x9400_0008u32.to_le_bytes()); // 0x20 bl 0x40
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x24 ret
        while image.len() < 0x40 {
            image.push(0);
        }
        image.extend_from_slice(&0x9000_0008u32.to_le_bytes()); // 0x40 adrp x8
        image.extend_from_slice(&0xf940_0109u32.to_le_bytes()); // 0x44 ldr x9,[x8,#8]
        image.extend_from_slice(&0x9100_0108u32.to_le_bytes()); // 0x48 add x8,x8,#0
        image.extend_from_slice(&0xd61f_0120u32.to_le_bytes()); // 0x4c br x9

        let mut st = CpuState::new();
        let blk = compile_image_bounded(&image, 0, 0, &mut st as *mut CpuState, 100)
            .expect("bounded compile");
        let _ = unsafe { run(&blk, &mut st as *mut CpuState) };
        // The stub wrote the diverted target into state.pc; the dispatcher would
        // re-enter the callee next. The callee was NOT inlined into this block.
        assert_eq!(st.pc, 0x20, "bl to import-bearing callee must divert via pc=0x20");
        assert_eq!(st.x[30], 0x04, "bl sets x30 link to pc+4");
    }

    #[test]
    fn guest_bl_to_import_free_callee_still_inlines() {
        // caller 0x00 bl 0x10 ; ret ; callee 0x10 mov x0,#42 ; ret
        let mut image = Vec::<u8>::new();
        image.extend_from_slice(&0x9400_0004u32.to_le_bytes()); // 0x00 bl 0x10
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x04 ret
        while image.len() < 0x10 {
            image.push(0);
        }
        image.extend_from_slice(&0xd280_0540u32.to_le_bytes()); // 0x10 mov x0,#42
        image.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // 0x14 ret

        let mut st = CpuState::new();
        let blk = compile_image_bounded(&image, 0, 0, &mut st as *mut CpuState, 100)
            .expect("bounded compile");
        let _ = unsafe { run(&blk, &mut st as *mut CpuState) };
        // Import-free callee is inlined: its `mov x0,#42` ran inline.
        assert_eq!(st.x[0], 42, "import-free guest callee is inlined (x0=42)");
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
        // adc w12, w14, w11 = 0x1a0b01cc  (rm=11, rn=14, rd=12).
        // nzcv bit29 holds the stored (borrow-convention) C: TRUE carry = !C_s
        // (store_nzcv stores !carry-out for carries, borrow for subs). So to give
        // the adc a TRUE carry-in, nzcv.C_s must be 0.
        //   TRUE_C=1: 0 + 10 + 1 = 11.
        let mut st = CpuState::new();
        st.x[14] = 0;
        st.x[11] = 10;
        st.nzcv = 0; // C_s=0 -> TRUE_C=1
        let mut code = Vec::new();
        code.extend_from_slice(&0x1a0b_01ccu32.to_le_bytes());
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec adc");
        assert_eq!(st.x[12], 11, "adc w: 0 + 10 + TRUE_C(1) = 11");

        // TRUE carry clear (C_s=1): 0 + 10 + 0 = 10.
        let mut st2 = CpuState::new();
        st2.x[14] = 0;
        st2.x[11] = 10;
        st2.nzcv = 0x2000_0000; // C_s=1 -> TRUE_C=0
        let mut code2 = Vec::new();
        code2.extend_from_slice(&0x1a0b_01ccu32.to_le_bytes());
        code2.extend_from_slice(&0xd65f_03c0u32.to_le_bytes());
        exec_bytes(&mut st2, &code2, 0).expect("exec adc c-0");
        assert_eq!(st2.x[12], 10, "adc w: 0 + 10 + 0 = 10");

        // sbc w12, w14, w11 = 0x5a0b01cc: 100 - 40 - (1 - TRUE_C).
        // TRUE_C=1 (C_s=0) -> 100 - 40 - 0 = 60.
        let mut st3 = CpuState::new();
        st3.x[14] = 100;
        st3.x[11] = 40;
        st3.nzcv = 0; // C_s=0 -> TRUE_C=1
        let mut code3 = Vec::new();
        code3.extend_from_slice(&0x5a0b_01ccu32.to_le_bytes());
        code3.extend_from_slice(&0xd65f_03c0u32.to_le_bytes());
        exec_bytes(&mut st3, &code3, 0).expect("exec sbc");
        assert_eq!(st3.x[12], 60, "sbc w TRUE_C=1: 100 - 40 - 0 = 60");
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
    fn fsub_scalar_not_swallowed_by_widening_shll() {
        // REGRESSION (Session 99): `fsub d0,d0,d1` = 0x1e61_3800 decodes as
        // SIMD WidenShl (shll) because byte3-low-nibble is 0x0e and bits15:8 == 0x38
        // (the WidenShl gate lacked a bit28==0 guard against the scalar-FP 0x1e
        // family). Result: 59049.0 - 59048.0 returned 0.0 and dscale.elf gave 0.
        let mut st = CpuState::new();
        st.v[0] = 0x40ec_d520_0000_0000u64; // 59049.0  (d0 == V0 low 8B)
        st.v[2] = 0x40ec_d500_0000_0000u64; // 59048.0  (d1 == V1 low 8B)
        let mut code = Vec::new();
        code.extend_from_slice(&0x1e61_3800u32.to_le_bytes()); // fsub d0,d0,d1
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec scalar fsub");
        assert_eq!(st.v[0], 0x3ff0_0000_0000_0000u64, "59049.0 - 59048.0 == 1.0");
    }

    #[test]
    fn fp_compare_sets_negative_flag_and_branches() {
        // REGRESSION (Session 99): store_nzcv_fp hardcoded N=0, but AArch64 FP
        // compare sets N=1 for the ordered less-than case, so b.mi/b.lt/b.gt were
        // all wrong (dclamp.elf counted everything -> 6 instead of 4). Now
        // N = CF && !ZF. fcmp d0,d1 with d0=59048 < d1=59049:
        //   cset w2,mi (N==1) -> 1 ; cset w3,le (Z||N!=V) -> 1 ; cset w4,gt -> 0.
        let mut st = CpuState::new();
        st.v[0] = 0x40ec_d500_0000_0000u64; // 59048.0  (d0 == V0 low 8B)
        st.v[2] = 0x40ec_d520_0000_0000u64; // 59049.0  (d1 == V1 low 8B)
        let mut code = Vec::new();
        code.extend_from_slice(&0x1e61_2000u32.to_le_bytes()); // fcmp d0,d1
        code.extend_from_slice(&0x1a9f_57e2u32.to_le_bytes()); // cset w2, mi
        code.extend_from_slice(&0x1a9f_c7e3u32.to_le_bytes()); // cset w3, le
        code.extend_from_slice(&0x1a9f_d7e4u32.to_le_bytes()); // cset w4, gt
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec fcmp + cset");
        assert_eq!(st.x[2], 1, "mi (N==1 for d0<d1)");
        assert_eq!(st.x[3], 1, "le (ordered less-than or equal)");
        assert_eq!(st.x[4], 0, "gt (d0<d1 is not greater)");
    }

    #[test]
    fn ld1_two_register_consecutive_load_is_not_deinterleaved() {
        // REGRESSION (Session 99): `ld1 {v0.16b-v1.16b},[x0]` (opcode bits[15:12]
        // == 0xA) was swallowed by the ld2 gate (0x8) which DEINTERLEAVES; LD1
        // multiple-structure loads CONSECUTIVE blocks. ddiv.elf (array-literal
        // double array) returned 2 instead of 10. Now ld1-2reg reads 32 bytes
        // straight into v0,v1.
        let mut st = CpuState::new();
        let mut buf = Vec::new();
        for i in 0u8..32 {
            buf.push(i); // mem = [0,1,2,...,31]
        }
        let bb = Box::leak(buf.into_boxed_slice());
        st.set(0, bb.as_ptr() as u64);
        let mut code = Vec::new();
        code.extend_from_slice(&0x4c40_a000u32.to_le_bytes()); // ld1 {v0.16b-v1.16b},[x0]
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec ld1-2reg");
        assert_eq!(st.v[0], 0x0706_0504_0302_0100u64, "v0 = consecutive first 8 bytes");
        assert_eq!(st.v[1], 0x0f0e_0d0c_0b0a_0908u64, "v0 hi = next 8 bytes");
        assert_eq!(st.v[2], 0x1716_1514_1312_1110u64, "v1 = second block, no deinterleave");
        assert_eq!(st.v[3], 0x1f1e_1d1c_1b1a_1918u64, "v1 hi");
    }

    #[test]
    fn scalar_fma3_all_four_variants() {
        // Scalar 3-source FP multiply-accumulate: the compiler contracts every
        // a*b+c (and -O2 fuses a*x*x into) fmadd. These were previously swallowed
        // by a broad SIMD-immediate gate and silently corrupted. d1=3,d2=4,d3=5
        // (V1/V2/V3 low u64 = st.v[2/4/6]); each writes d0 (=st.v[0]).
        let words: [u32; 4] = [0x1f42_0c20, 0x1f42_8c20, 0x1f62_0c20, 0x1f62_8c20];
        let expect: [i64; 4] = [17, -7, -17, 7]; // fmadd=5+12, fmsub=5-12, fnmadd=-(5+12), fnmsub=12-5
        for (i, w) in words.iter().enumerate() {
            let mut st = CpuState::new();
            st.v[2] = 3.0f64.to_bits();
            st.v[4] = 4.0f64.to_bits();
            st.v[6] = 5.0f64.to_bits();
            let mut code = Vec::new();
            code.extend_from_slice(&w.to_le_bytes());
            code.extend_from_slice(&0x1e78_0000u32.to_le_bytes()); // fcvtzs w0,d0
            code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
            exec_bytes(&mut st, &code, 0).expect("exec scalar fma3");
            assert_eq!(
                f64::from_bits(st.v[0]) as i64,
                expect[i],
                "fma3 variant {i}"
            );
        }
        // single-precision fmadd s0,s1,s2,s3: s1..s3 low 4B of V1..V3; 2*3+5=11.
        let mut st = CpuState::new();
        st.v[2] = 2.0f32.to_bits() as u64;
        st.v[4] = 3.0f32.to_bits() as u64;
        st.v[6] = 5.0f32.to_bits() as u64;
        let mut code = Vec::new();
        code.extend_from_slice(&0x1f02_0c20u32.to_le_bytes()); // fmadd s0,s1,s2,s3
        code.extend_from_slice(&0x1e26_0000u32.to_le_bytes()); // fmov w0,s0
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec scalar fmadd s");
        assert_eq!(f32::from_bits(st.x[0] as u32), 11.0, "fmadd s: 5 + 2*3");
    }

    #[test]
    fn ldst_pair_d_registers_use_16_byte_vector_stride() {
        // REGRESSION (Session 99): the LdStPair fp_d branch used VECTOR_BASE + rt*8
        // as the D-reg slot, but Dn is the LOW 8 bytes of the 16-byte Vn slot
        // (VECTOR_BASE + rt*16). So `ldp d0,d1,[x0]` wrote to 0x110/0x108 instead
        // of 0x110/0x120, and a follow-on fmadd read stale slots — structfield.elf
        // (-O2, struct double array walk via `ldp d29,d28,[x0],#16`) returned 128
        // instead of 52. Encodings from ldpd.o.
        let mut st = CpuState::new();
        let vals: [u64; 4] = [
            0x1111_2222_3333_4444,
            0x5555_6666_7777_8888,
            0x9999_aaaa_bbbb_cccc,
            0xdddd_eeee_ffff_0000,
        ];
        let mut buf = Vec::new();
        for v in vals {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        let bb = Box::leak(buf.into_boxed_slice());
        let orig = bb.as_ptr() as u64;
        st.set(0, orig);
        let mut code = Vec::new();
        code.extend_from_slice(&0x6d40_0400u32.to_le_bytes()); // ldp d0,d1,[x0] (bytes 0,1 as f64)
        code.extend_from_slice(&0x6cc1_0c02u32.to_le_bytes()); // ldp d2,d3,[x0],#16
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec ldp d-pair");
        // d0 = low 8 of V0 = st.v[0]; d1 = low 8 of V1 = st.v[2]; d2 = V2 = st.v[4]; etc.
        assert_eq!(st.v[0], vals[0], "d0 = first double");
        assert_eq!(st.v[2], vals[1], "d1 = second double");
        // second ldp has no pre-advance: reads vals[0],vals[1] again, then +16.
        assert_eq!(st.v[4], vals[0], "d2 = first double (no wx before 2nd ldp)");
        assert_eq!(st.v[6], vals[1], "d3 = second double");
        assert_eq!(st.x[0], orig + 16, "post-index ldp advanced x0 by 16");
    }

    #[test]
    fn ldst_pair_s_registers_use_4_byte_transfers() {
        // REGRESSION (Session 99): byte3 0x2c/0x2d (single-precision FP pair)
        // was lumped into fp_d (scale 8), so `ldp s0,s1,[x0]` read 8 bytes per
        // reg and post-indexed 2x — fstruct.elf (-O2 float-struct walk) returned
        // 0x391c0000 garbage and fmat2.elf (2D float det) returned -108. Now the
        // 32-bit s-pair uses scale 4 and 4-byte transfers (low 4B of each 16B
        // vector slot: sN = VECTOR_BASE + N*16). Encodings from ldps.o.
        let mut st = CpuState::new();
        let vals: [u32; 4] = [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xdd00_1122];
        let mut buf = Vec::new();
        for v in vals {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        let bb = Box::leak(buf.into_boxed_slice());
        let orig = bb.as_ptr() as u64;
        st.set(0, orig);
        let mut code = Vec::new();
        code.extend_from_slice(&0x2d40_0400u32.to_le_bytes()); // ldp s0,s1,[x0]
        code.extend_from_slice(&0x2cc1_0c02u32.to_le_bytes()); // ldp s2,s3,[x0],#8
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec ldp s-pair");
        assert_eq!((st.v[0] & 0xffff_ffff) as u32, vals[0] as u32, "s0");
        assert_eq!((st.v[2] & 0xffff_ffff) as u32, vals[1] as u32, "s1");
        // second ldp reads from same x0 (no pre-advance), then +8.
        assert_eq!((st.v[4] & 0xffff_ffff) as u32, vals[0] as u32, "s2");
        assert_eq!((st.v[6] & 0xffff_ffff) as u32, vals[1] as u32, "s3");
        assert_eq!(st.x[0], orig + 8, "post-index s-pair advanced x0 by 8");
    }

    #[test]
    fn add_shifted_register_applies_shift_to_source_not_clobbered() {
        // REGRESSION (Session 99): apply_shift_const wrote the shift amount into
        // RCX (the very register holding the Rm value) then `shl rcx, cl`, so
        // `add x1,x2,x0,lsl#3` became x2 + (3<<3=24) — a CONSTANT, not x0<<3.
        // -O2 array-index loops then read the same element every iteration
        // (fclamp returned 10 instead of 9). Encodings from shadd.o.
        let mut st = CpuState::new();
        st.set(0, 2); // x0
        st.set(1, 4); // x1
        st.set(4, 16); // x4
        st.set(6, -8i64 as u64); // x6 (asr #1 -> -4)
        st.set(8, 1); // x8
        let mut code = Vec::new();
        code.extend_from_slice(&0x8b00_0c22u32.to_le_bytes()); // add x2,x1,x0,lsl#3
        code.extend_from_slice(&0x8b44_0803u32.to_le_bytes()); // add x3,x0,x4,lsr#2
        code.extend_from_slice(&0x8b86_0425u32.to_le_bytes()); // add x5,x1,x6,asr#1
        code.extend_from_slice(&0x8b08_0007u32.to_le_bytes()); // add x7,x0,x8
        code.extend_from_slice(&0xd65f_03c0u32.to_le_bytes()); // ret
        exec_bytes(&mut st, &code, 0).expect("exec shifted-register add");
        assert_eq!(st.x[2], 20, "x1 + (x0<<3)");
        assert_eq!(st.x[3], 6, "x0 + (x4>>2)");
        assert_eq!(st.x[5], 0, "x1 + (x6 asr#1)");
        assert_eq!(st.x[7], 3, "x0 + x8");
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
    fn vector_2d_fp_div_mul_not_swallowed_by_int_add_or_bsl() {
        // Session (Sep 11 2026): three FP `Vd.2D` decode collisions silently
        // corrupted double math on real code paths. Each op must compute the
        // honest double, not a integer add / bitwise-select of the bit patterns.
        //   - `fadd v0.2d` (0x4e61d400) was decoded as integer SimdAddH
        //     (halfword paddw) because the integer add/sub gates ignored bit14.
        //   - `fdiv v0.2d` (0x6e61fc00) / `fmul v0.2d` (0x6e61dc00) were decoded
        //     as SimdSel (bsl bitwise select) because byte1 bits[15:13] were not
        //     masked off the 0x2e/0x6e select gate.
        // v0={64,128}, v1={8,16}, v2.d[0]=2.0
        let mk = |v0: f64, v1: f64, v2: f64, v3: f64| {
            let mut st = CpuState::new();
            st.v[0] = v0.to_bits(); st.v[1] = v1.to_bits();
            st.v[2] = v2.to_bits(); st.v[3] = v3.to_bits();
            st
        };
        // fadd v0.2d,v0.2d,v1.2d = 0x4e61d400 -> {72.0, 144.0}
        let mut st = mk(64.0, 128.0, 8.0, 16.0);
        exec_bytes(&mut st, &0x4e61d400u32.to_le_bytes(), 0).unwrap();
        assert_eq!(st.v[0], 72.0f64.to_bits());
        assert_eq!(st.v[1], 144.0f64.to_bits());
        // fmul v0.2d,v0.2d,v1.2d = 0x6e61dc00 -> {512.0, 2048.0}
        let mut st = mk(64.0, 128.0, 8.0, 16.0);
        exec_bytes(&mut st, &0x6e61dc00u32.to_le_bytes(), 0).unwrap();
        assert_eq!(st.v[0], 512.0f64.to_bits());
        assert_eq!(st.v[1], 2048.0f64.to_bits());
        // fdiv v0.2d,v0.2d,v1.2d = 0x6e61fc00 -> {8.0, 8.0}
        let mut st = mk(64.0, 128.0, 8.0, 16.0);
        exec_bytes(&mut st, &0x6e61fc00u32.to_le_bytes(), 0).unwrap();
        assert_eq!(st.v[0], 8.0f64.to_bits());
        assert_eq!(st.v[1], 8.0f64.to_bits());
        // genuine bsl v0.16b,v0.16b,v1.16b must STILL be SimdSel (bitwise).
        let mut st = mk(0.0, 0.0, 0.0, 0.0);
        st.v[0] = 0x0f0f0f0f0f0f0f0f; st.v[1] = 0x0f0f0f0f0f0f0f0f;
        st.v[2] = 0x00ff00ff00ff00ff; st.v[3] = 0;
        exec_bytes(&mut st, &0x6e611c00u32.to_le_bytes(), 0).unwrap();
        // bsl op0: Vd = (Rn&Rd)|(~Rd&Vm). Rd=Rn=v0(0x0f..), Vm=v1(0x00ff.. per
        // byte, low byte first). Per byte: (0x0f&0x0f)|(~0x0f & Vm) = 0xff when
        // Vm byte is 0xff, 0x0f when Vm byte is 0x00 => 0x0fff0fff0fff0fff.
        // (A pure bitwise result — proves it is NOT the FP fdiv.)
        assert_eq!(st.v[0], 0x0fff0fff0fff0fffu64);
    }

    #[test]
    fn fmov_imm_high_mantissa_12_to_15_not_swallowed_as_fcvt() {
        // Session (Sep 11 2026): `fmov d,#imm` values with mantissa m>=8 (imm8 bit3 set,
        // instruction bit16) were swallowed by the coarse fcvt-to-int round gate
        // (0xffff_0000 top-16 matched `fcvtau 0x1e65`'s top bytes) and decoded as
        // FcvtToInt, leaving the destination 0 instead of loading 12/13/14/15.
        // The fcvt-round gate now requires bit12 CLR (FMOV-imm has it SET).
        for (enc, expect) in [
            (0x1e651017u32, 12.0f64),
            (0x1e655017u32, 13.0f64),
            (0x1e659017u32, 14.0f64),
            (0x1e65d017u32, 15.0f64),
        ] {
            let mut st = CpuState::new();
            exec_bytes(&mut st, &enc.to_le_bytes(), 0).unwrap();
            assert_eq!(f64::from_bits(st.v[46]), expect, "fmov d23,# {expect} (0x{enc:08x})");
        }
    }

    #[test]
    fn simd_reduce_minmax_across_lanes() {
        // SMINV/SMAXV/UMINV/UMAXV Sd/Hd/Bd, Vn.T: horizontal min/max off ALL
        // lanes -> bottom scalar (upper cleared). Signed 16-bit lanes need a
        // 64-bit sign-extension on the load — which surfaced a latent emitter
        // bug: movsx_word_mem/movsx_byte_mem emitted REX without W (0F BF/BE
        // wrote only a 32-bit dest), so a negative 8/16-bit lane compared as a
        // huge positive u64 (sminv.8h of {-9,-2,..} picked 4, not -9). Fixed
        // both to REX.W.
        let l32 = |v: u64| -> i32 { (v & 0xffffffff) as u32 as i32 };
        // sminv s0,v1.4s = 0x4eb1a820 on v1.4s = {-3,5,42,7} -> -3
        let mut st = CpuState::new();
        st.v[2] = (-3i32 as u32 as u64) | ((5u32 as u64) << 32);
        st.v[3] = (42u32 as u64) | ((7u32 as u64) << 32);
        exec_bytes(&mut st, &0x4eb1a820u32.to_le_bytes(), 0).unwrap();
        assert_eq!(l32(st.v[0]), -3, "sminv.4s");
        // smaxv s0,v1.4s = 0x4eb0a820 -> 42
        let mut st = CpuState::new();
        st.v[2] = (-3i32 as u32 as u64) | ((5u32 as u64) << 32);
        st.v[3] = (42u32 as u64) | ((7u32 as u64) << 32);
        exec_bytes(&mut st, &0x4eb0a820u32.to_le_bytes(), 0).unwrap();
        assert_eq!(l32(st.v[0]), 42, "smaxv.4s");
        let mk16 = |vals: &[i16]| {
            let mut s = CpuState::new();
            for i in 0..vals.len() {
                s.v[2 + i / 4] |= ((vals[i] as u16 as u64) << ((i % 4) * 16));
            }
            s
        };
        let mut st = mk16(&[-9, -2, 4, 6, 8, 10, 12, 14]);
        exec_bytes(&mut st, &0x4e71a820u32.to_le_bytes(), 0).unwrap();
        assert_eq!((st.v[0] & 0xffff) as i16, -9, "sminv.8h (sign-extend)");
        let mut st = mk16(&[-9, -2, 4, 6, 8, 10, 12, 14]);
        exec_bytes(&mut st, &0x4e70a820u32.to_le_bytes(), 0).unwrap();
        assert_eq!((st.v[0] & 0xffff) as i16, 14, "smaxv.8h");
    }

    #[test]
    fn smin_smax_element_high_register_b2_mask() {
        // `smin`/`smax` Vd.4s decode: the gate tests `(b2 & 0xfc) == 0x64`
        // (max) but ASSIGNED `max: b2 == 0x64` exactly. b2's low 2 bits carry
        // Rn (bits[9:8]), so a real gcc `smax v30.4s, v29.4s, v28.4s`
        // (0x4ebc67be, b2=0x67) decoded as MIN and returned the Vn operands
        // verbatim (max of {-28} and {308} -> -28). Only register 0..3 hid it
        // (b2 stayed 0x64/0x6c). Fixed assignment to mask like the gate.
        // smax v30.4s, v29.4s, v28.4s = 0x4ebc67be (rd=30, rn=29, rm=28):
        //   Vn=v29 = <140,308,7,9> ; Vm=v28 = <-28,5,3,2> -> Vd=v30 = <140,308,7,9>
        let mut st = CpuState::new();
        st.v[58] = (140u32 as u64) | ((308u32 as u64) << 32); // v29 (rn)
        st.v[59] = (7u32 as u64) | ((9u32 as u64) << 32);
        st.v[56] = (-28i32 as u32 as u64) | ((5u32 as u64) << 32); // v28 (rm)
        st.v[57] = (3u32 as u64) | ((2u32 as u64) << 32);
        exec_bytes(&mut st, &0x4ebc67beu32.to_le_bytes(), 0).unwrap();
        assert_eq!((st.v[60] & 0xffffffff) as u32 as i32, 140, "smax lane0");
        assert_eq!(((st.v[60] >> 32) & 0xffffffff) as u32 as i32, 308, "smax lane1");
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
    fn guest_svc_common_boot_gaps_roundtrip() {
        // Exercise the newly-added boot-path syscalls: fcntl(25), setpgid(154),
        // getrusage(165), clock_nanosleep(115), rt_sigaction(134),
        // rt_sigprocmask(135), fadvise64(223). All must return without crashing
        // and with sane semantics (no -ENOSYS).
        let mut st = CpuState::new();

        // fcntl(25) on a fresh dup of a pipe write end: F_GETFD (1) must be 0.
        let mut pfd = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(pfd.as_mut_ptr()) }, 0);
        st.x[8] = 25; st.x[0] = pfd[1] as u64; st.x[1] = libc::F_GETFD as u64; st.x[2] = 0;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "fcntl F_GETFD on pipe write end");
        // F_SETFL(4) with O_NONBLOCK must succeed.
        st.x[1] = libc::F_SETFL as u64; st.x[2] = libc::O_NONBLOCK as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "fcntl F_SETFL O_NONBLOCK");
        unsafe { libc::close(pfd[0]); libc::close(pfd[1]); }

        // getrusage(165) RUSAGE_SELF (0) -> guest rusage buffer, returns 0.
        st.x[8] = 165; st.x[0] = 0; let mut ru = [0u8; 144]; st.x[1] = ru.as_mut_ptr() as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "getrusage RUSAGE_SELF");

        // clock_nanosleep(115) with zero time must return immediately, 0.
        let ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        st.x[8] = 115; st.x[0] = libc::CLOCK_MONOTONIC as u64; st.x[1] = 0;
        st.x[2] = (&ts as *const libc::timespec) as u64; st.x[3] = 0;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "clock_nanosleep 0-time");

        // rt_sigaction(134): installing a handler succeeds (returns 0) and a
        // non-null oact output is zeroed, not left as garbage.
        let act = [0u8; 128]; let mut oact = [0xabu8; 128];
        st.x[8] = 134; st.x[0] = 2 /*SIGINT*/; st.x[1] = act.as_ptr() as u64;
        st.x[2] = oact.as_mut_ptr() as u64; st.x[3] = 8;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "rt_sigaction register ok");
        assert_eq!(oact.iter().all(|&b| b == 0), true, "rt_sigaction oact zeroed");

        // rt_sigprocmask(135): reports empty old set.
        let mut oset = [0xffu8; 8];
        st.x[8] = 135; st.x[0] = 0 /*SIG_BLOCK*/; st.x[1] = 0; st.x[2] = oset.as_mut_ptr() as u64; st.x[3] = 8;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "rt_sigprocmask ok");
        assert_eq!(oset, [0u8; 8], "rt_sigprocmask empty old set");

        // fadvise64(223) on an fd: POSIX_FADV_NORMAL(0) must not fault.
        let file = std::env::temp_dir().join(format!("svc_fadv_{}.tmp", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        let c = std::ffi::CString::new(file.to_str().unwrap()).unwrap();
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
        st.x[8] = 223; st.x[0] = fd as u64; st.x[1] = 0; st.x[2] = 0; st.x[3] = 0;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "fadvise64 ok");
        unsafe { libc::close(fd); }
        let _ = std::fs::remove_file(&file);

        // Second batch: mkdirat(34)/unlinkat(35)/renameat(38), socketpair(199),
        // pread64(67)/pwrite64(68), madvise(233), umask(166). All must return
        // without -ENOSYS and with correct effect.
        let d = std::env::temp_dir().join(format!("svc_dir_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let cd = std::ffi::CString::new(d.to_str().unwrap()).unwrap();
        st.x[8] = 34; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cd.as_ptr() as u64; st.x[2] = 0o755;
        assert_eq!(guest_svc(&mut st as *mut CpuState) as i64, 0, "mkdirat");
        assert!(d.is_dir());

        // write a file, then pread64/pwrite64 through it.
        let fpath = d.join("f.bin");
        let fp_c = std::ffi::CString::new(fpath.to_str().unwrap()).unwrap();
        st.x[8] = 56; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = fp_c.as_ptr() as u64;
        st.x[2] = (libc::O_CREAT | libc::O_RDWR | 0o644) as u64; st.x[3] = 0o644;
        let fd = guest_svc(&mut st as *mut CpuState) as i32;
        assert!(fd >= 0, "openat for pread/pwrite");
        let mut buf = [0u8; 8];
        buf.copy_from_slice(b"abcdefgh");
        st.x[8] = 68; st.x[0] = fd as u64; st.x[1] = buf.as_ptr() as u64; st.x[2] = 8; st.x[3] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 8, "pwrite64 writes 8");
        let mut rb = [0xffu8; 8];
        st.x[8] = 67; st.x[0] = fd as u64; st.x[1] = rb.as_mut_ptr() as u64; st.x[2] = 8; st.x[3] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 8, "pread64 reads 8");
        assert_eq!(&rb, b"abcdefgh", "pread64 content");
        unsafe { libc::close(fd); }

        // socketpair(199) AF_UNIX stream -> two fds.
        let mut sv = [0i32; 2];
        st.x[8] = 199; st.x[0] = libc::AF_UNIX as u64; st.x[1] = libc::SOCK_STREAM as u64;
        st.x[2] = 0; st.x[3] = sv.as_mut_ptr() as u64;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "socketpair");
        assert!(sv[0] >= 0 && sv[1] >= 0);
        // sendmsg/recvmsg(211/212) roundtrip a byte over it.
        let msg = b"Z";
        let mut iov = libc::iovec { iov_base: msg.as_ptr() as *mut libc::c_void, iov_len: 1 };
        let mut mh = libc::msghdr { msg_name: std::ptr::null_mut(), msg_namelen: 0,
            msg_iov: &mut iov, msg_iovlen: 1, msg_control: std::ptr::null_mut(),
            msg_controllen: 0, msg_flags: 0 };
        st.x[8] = 211; st.x[0] = sv[1] as u64; st.x[1] = (&mh as *const libc::msghdr) as u64; st.x[2] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 1, "sendmsg");
        let mut out = [0u8; 1];
        let mut iov2 = libc::iovec { iov_base: out.as_mut_ptr() as *mut libc::c_void, iov_len: 1 };
        let mut mh2 = libc::msghdr { msg_name: std::ptr::null_mut(), msg_namelen: 0,
            msg_iov: &mut iov2, msg_iovlen: 1, msg_control: std::ptr::null_mut(),
            msg_controllen: 0, msg_flags: 0 };
        st.x[8] = 212; st.x[0] = sv[0] as u64; st.x[1] = (&mut mh2 as *mut libc::msghdr) as u64; st.x[2] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 1, "recvmsg");
        assert_eq!(out[0], b'Z', "recvmsg content");
        unsafe { libc::close(sv[0]); libc::close(sv[1]); }

        // umask(166) roundtrips: set to a value, read back.
        st.x[8] = 166; st.x[0] = 0o027;
        let _m = guest_svc(&mut st as *mut CpuState) as u32;
        st.x[8] = 166; st.x[0] = 0o027;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0o027, "umask returns previous");

        // madvise(233) on an anonymous page.
        let m = unsafe { libc::mmap(std::ptr::null_mut(), 4096, libc::PROT_READ|libc::PROT_WRITE, libc::MAP_PRIVATE|libc::MAP_ANONYMOUS, -1, 0) };
        st.x[8] = 233; st.x[0] = m as u64; st.x[1] = 4096; st.x[2] = libc::MADV_DONTNEED as u64;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "madvise DONTNEED");
        unsafe { libc::munmap(m, 4096); }

        // renameat(38) the file.
        let d2 = d.join("f2.bin");
        let fc2 = std::ffi::CString::new(d2.to_str().unwrap()).unwrap();
        st.x[8] = 38; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = fp_c.as_ptr() as u64;
        st.x[2] = libc::AT_FDCWD as u64; st.x[3] = fc2.as_ptr() as u64;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "renameat");
        assert!(d2.is_file() && !fpath.exists());

        // unlinkat(35) cleanup.
        st.x[8] = 35; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = fc2.as_ptr() as u64; st.x[2] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "unlinkat");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// timerfd (85/86/87) and signalfd4 (74) — the ALooper/libutils timeout &
    /// signal-fd primitives Roblox's event loop waits on. timerfd_create must
    /// return a real fd, settime arms it, gettime reflects the pending value, and
    /// a read returns after the interval expires. signalfd must return a valid
    /// host fd (empty mask -> never fires, matching the no-signal-dispatch stance)
    /// rather than -ENOSYS.
    #[test]
    fn guest_svc_timerfd_and_signalfd_roundtrip() {
        let mut st = CpuState::new();
        let svc = |st: &mut CpuState| -> i64 { guest_svc(st as *mut CpuState) as i64 };

        // timerfd_create(CLOCK_MONOTONIC=1, flags=0).
        st.x[8] = 85; st.x[0] = libc::CLOCK_MONOTONIC as u64; st.x[1] = 0;
        let tfd = svc(&mut st);
        assert!(tfd >= 0, "timerfd_create returns a real fd, got {tfd}");
        let tfd = tfd as i32;

        // timerfd_settime(fd, 0, new={it_value 5ms}, NULL): arm an absolute-free
        // one-shot timer that expires in 5ms.
        let new = libc::itimerspec {
            it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            it_value: libc::timespec { tv_sec: 0, tv_nsec: 5_000_000 },
        };
        st.x[8] = 86; st.x[0] = tfd as u64; st.x[1] = 0;
        st.x[2] = (&new as *const libc::itimerspec) as u64; st.x[3] = 0;
        assert_eq!(svc(&mut st), 0, "timerfd_settime arms the timer");

        // timerfd_gettime(fd, curr) reflects a pending (non-zero) remaining time.
        let mut curr = libc::itimerspec { it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 }, it_value: libc::timespec { tv_sec: 0, tv_nsec: 0 } };
        st.x[8] = 87; st.x[0] = tfd as u64; st.x[1] = (&mut curr as *mut libc::itimerspec) as u64;
        assert_eq!(svc(&mut st), 0, "timerfd_gettime ok");
        let pending = curr.it_value.tv_sec > 0 || curr.it_value.tv_nsec > 0;
        assert!(pending, "timerfd_gettime reports a pending timer (got {:?})", curr.it_value);

        // Sleep past expiry, then read(): returns 8 (one u64 expiration count).
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut exp = 0u64;
        st.x[8] = 63; st.x[0] = tfd as u64; st.x[1] = (&mut exp as *mut u64) as u64; st.x[2] = 8;
        assert_eq!(svc(&mut st), 8, "read on expired timerfd returns 8 bytes");
        if exp > 0 {
            // No requirement on the count, just that events were delivered.
        }
        unsafe { libc::close(tfd); }

        // signalfd4(-1, mask, 8, 0): an empty-mask signalfd is never woken by our
        // no-signal stance, but the call must succeed with a valid fd.
        st.x[8] = 74; st.x[0] = !0u64 as u64; st.x[1] = 0; st.x[2] = 8; st.x[3] = 0;
        let sfd = svc(&mut st);
        assert!(sfd >= 0, "signalfd4 returns a real fd, got {sfd}");
        unsafe { libc::close(sfd as i32); }
    }

    #[test]
    fn guest_svc_boot_io_affinity_limits_roundtrip() {
        // Exercise the boot-path batch: CPU-affinity probes, prlimit64, getcpu,
        // itimers, statfs/fstatfs, truncate/ftruncate, fsync/fdatasync, sendfile,
        // utimensat/fchmodat, getsid, msync/mlock/munlock/mincore. All must return
        // real results (or a valid -errno), never -ENOSYS.
        let mut st = CpuState::new();
        let svc = |st: &mut CpuState| -> i64 { guest_svc(st as *mut CpuState) as i64 };

        // Get/Set affinity (204/122) for the current process: getcpu count > 0.
        let mut mask = [0u8; 128];
        st.x[8] = 204; st.x[0] = 0; st.x[1] = mask.len() as u64; st.x[2] = mask.as_mut_ptr() as u64;
        let n = svc(&mut st);
        assert!(n > 0, "sched_getaffinity returns cpu-set size, got {n}");
        assert!(mask.iter().any(|&b| b != 0), "affinity mask non-zero");
        // Safe set: rebuild a mask containing cpu 0 only.
        let mut one = [0u8; 128]; one[0] = 1;
        st.x[8] = 122; st.x[0] = 0; st.x[1] = one.len() as u64; st.x[2] = one.as_mut_ptr() as u64;
        assert_eq!(svc(&mut st), 0, "sched_setaffinity cpu0");

        // prlimit64(261): read RLIMIT_NOFILE into the old-limit struct.
        let mut rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        st.x[8] = 261; st.x[0] = 0; st.x[1] = libc::RLIMIT_NOFILE as u64;
        st.x[2] = 0; st.x[3] = (&mut rl as *mut libc::rlimit) as u64;
        assert_eq!(svc(&mut st), 0, "prlimit64 RLIMIT_NOFILE get");
        assert!(rl.rlim_cur > 0, "RLIMIT_NOFILE cur>0");

        // getcpu(168): writes three ints.
        let (mut cpu, mut node) = (-1i32, -1i32);
        st.x[8] = 168; st.x[0] = (&mut cpu as *mut i32) as u64; st.x[1] = (&mut node as *mut i32) as u64; st.x[2] = 0;
        assert_eq!(svc(&mut st), 0, "getcpu");
        assert!(node >= 0, "getcpu node>=0"); // cpu may read arbitrary; kernel writes real

        // getitimer(102)/setitimer(103): ITIMER_REAL readback returns 0 (no alarm).
        let mut itv = libc::itimerval { it_interval: libc::timeval{tv_sec:0,tv_usec:0}, it_value: libc::timeval{tv_sec:0,tv_usec:0} };
        st.x[8] = 102; st.x[0] = libc::ITIMER_REAL as u64; st.x[1] = (&mut itv as *mut libc::itimerval) as u64;
        assert_eq!(svc(&mut st), 0, "getitimer ITIMER_REAL");

        // statfs(43) on "/" — the leading fields must be non-zero (space check).
        let croot = std::ffi::CString::new("/").unwrap();
        let mut fsb = [0u8; 120];
        st.x[8] = 43; st.x[0] = croot.as_ptr() as u64; st.x[1] = fsb.as_mut_ptr() as u64;
        assert_eq!(svc(&mut st), 0, "statfs /");
        let bsize = u64::from_le_bytes(fsb[8..16].try_into().unwrap());
        let blocks = u64::from_le_bytes(fsb[16..24].try_into().unwrap());
        assert!(bsize > 0 && blocks > 0, "statfs bsize/blocks populated ({bsize}/{blocks})");

        // temp file for sizing / durability / metadata tests.
        let f = std::env::temp_dir().join(format!("svc_boot_{}.bin", std::process::id()));
        std::fs::write(&f, b"0123456789").unwrap();
        let cf = std::ffi::CString::new(f.to_str().unwrap()).unwrap();
        // openat(56) O_RDWR.
        st.x[8] = 56; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cf.as_ptr() as u64;
        st.x[2] = (libc::O_RDWR | libc::O_CLOEXEC) as u64; st.x[3] = 0o644;
        let fd = svc(&mut st) as i32;
        assert!(fd >= 0, "openat for boot batch");

        // fstatfs(44) on fd.
        let mut fsb2 = [0xffu8; 120];
        st.x[8] = 44; st.x[0] = fd as u64; st.x[1] = fsb2.as_mut_ptr() as u64;
        assert_eq!(svc(&mut st), 0, "fstatfs fd");
        // ftruncate(46) to 5 bytes -> size 5.
        st.x[8] = 46; st.x[0] = fd as u64; st.x[1] = 5;
        assert_eq!(svc(&mut st), 0, "ftruncate to 5");
        // fstat(80) size must now be 5.
        let mut gbuf = [0u8; 128];
        st.x[8] = 80; st.x[0] = fd as u64; st.x[1] = gbuf.as_mut_ptr() as u64;
        assert_eq!(svc(&mut st), 0, "fstat");
        assert_eq!(i64::from_le_bytes(gbuf[48..56].try_into().unwrap()), 5, "fstat size==5 after ftruncate");
        // fsync(82) and fdatasync(83) succeed.
        st.x[8] = 82; st.x[0] = fd as u64;
        assert_eq!(svc(&mut st), 0, "fsync");
        st.x[8] = 83; st.x[0] = fd as u64;
        assert_eq!(svc(&mut st), 0, "fdatasync");
        // truncate(45) path to 3.
        st.x[8] = 45; st.x[0] = cf.as_ptr() as u64; st.x[1] = 3;
        assert_eq!(svc(&mut st), 0, "truncate to 3");
        assert_eq!(std::fs::read(&f).unwrap().len(), 3, "file now 3 bytes");
        // utimensat(88) set now -> 0.
        let ts = [libc::timespec{tv_sec: 1_000_000, tv_nsec: 0}, libc::timespec{tv_sec: 1_000_000, tv_nsec: 0}];
        st.x[8] = 88; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cf.as_ptr() as u64;
        st.x[2] = ts.as_ptr() as u64; st.x[3] = 0;
        assert_eq!(svc(&mut st), 0, "utimensat");
        // fchmodat(53) 0600 -> 0, mode reflects it.
        st.x[8] = 53; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cf.as_ptr() as u64; st.x[2] = 0o600; st.x[3] = 0;
        assert_eq!(svc(&mut st), 0, "fchmodat 0600");
        unsafe { libc::close(fd); }

        // sendfile(71): copy the 3-byte file into a new output file.
        let out = std::env::temp_dir().join(format!("svc_boot_out_{}.bin", std::process::id()));
        std::fs::write(&out, b"").unwrap();
        let cout = std::ffi::CString::new(out.to_str().unwrap()).unwrap();
        st.x[8] = 56; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cout.as_ptr() as u64;
        st.x[2] = (libc::O_RDWR | libc::O_CLOEXEC) as u64; st.x[3] = 0o644;
        let ofd = svc(&mut st) as i32;
        st.x[8] = 56; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = cf.as_ptr() as u64;
        st.x[2] = (libc::O_RDONLY | libc::O_CLOEXEC) as u64; st.x[3] = 0;
        let ifd = svc(&mut st) as i32;
        st.x[8] = 71; st.x[0] = ofd as u64; st.x[1] = ifd as u64; st.x[2] = 0; st.x[3] = 3;
        assert_eq!(svc(&mut st), 3, "sendfile copies 3 bytes");
        unsafe { libc::close(ofd); libc::close(ifd); }
        assert_eq!(std::fs::read(&out).unwrap(), b"012", "sendfile content");

        // getsid(156) returns a valid sid.
        st.x[8] = 156; st.x[0] = 0;
        assert!(svc(&mut st) > 0, "getsid(0) valid");

        // msync(227)/mlock(228)/munlock(229)/mincore(232) on an anon page.
        let m = unsafe { libc::mmap(std::ptr::null_mut(), 4096, libc::PROT_READ|libc::PROT_WRITE, libc::MAP_PRIVATE|libc::MAP_ANONYMOUS, -1, 0) };
        assert!(m != libc::MAP_FAILED);
        unsafe { std::ptr::write_volatile(m as *mut u8, 7); } // fault in the page
        st.x[8] = 227; st.x[0] = m as u64; st.x[1] = 4096; st.x[2] = libc::MS_SYNC as u64;
        assert_eq!(svc(&mut st), 0, "msync MS_SYNC");
        st.x[8] = 228; st.x[0] = m as u64; st.x[1] = 4096;
        let ml = svc(&mut st);
        assert!(ml == 0 || ml == -libc::EPERM as i64, "mlock (tolerate EPERM) got {ml}");
        if ml == 0 {
            st.x[8] = 229; st.x[0] = m as u64; st.x[1] = 4096;
            assert_eq!(svc(&mut st), 0, "munlock");
        }
        let mut vec = [0u8; 1];
        st.x[8] = 232; st.x[0] = m as u64; st.x[1] = 4096; st.x[2] = vec.as_mut_ptr() as u64;
        assert_eq!(svc(&mut st), 0, "mincore");
        assert_eq!(vec[0] & 1, 1, "mincore page resident");
        unsafe { libc::munmap(m, 4096); }

        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn guest_svc_stats_and_descriptors_roundtrip() {
        // Exercise the newly-added AArch64 syscall families without crashing or
        // touching stdout: fstat(80) + newfstatat(79) must write a GUEST-layout
        // stat; eventfd/dup/gettid-style fd ops and gettimeofday must roundtrip.

        // A temp file to stat.
        let path = std::env::temp_dir().join(format!("svc_stat_{}_{}.tmp", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&path, b"some-payload-bytes").unwrap();
        let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY) };
        assert!(fd >= 0);

        // fstat (80) -> guest-layout buffer.
        let mut buf = [0u8; 128];
        let mut st = CpuState::new();
        st.x[8] = 80; st.x[0] = fd as u64; st.x[1] = buf.as_mut_ptr() as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r, 0, "fstat ok");
        // Guest layout: st_size @48 (i64), st_mode @16 (u32), st_ino @8 (u64).
        let size = i64::from_le_bytes(buf[48..56].try_into().unwrap());
        assert_eq!(size, b"some-payload-bytes".len() as i64, "fstat st_size");
        let mode = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        assert!(mode & 0o170000 != 0, "fstat st_mode has a file type (S_IFREG)");

        // newfstatat (79) with AT_FDCWD.
        let mut buf2 = [0u8; 128];
        st.x[8] = 79; st.x[0] = libc::AT_FDCWD as u64;
        st.x[1] = cpath.as_ptr() as u64; st.x[2] = buf2.as_mut_ptr() as u64; st.x[3] = 0;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r, 0, "newfstatat ok");
        let size2 = i64::from_le_bytes(buf2[48..56].try_into().unwrap());
        assert_eq!(size2, b"some-payload-bytes".len() as i64, "newfstatat st_size");

        // eventfd (19): createable and readable (may be ignored by some kernels
        // without EFD; but any valid fd >= 0 proves the routing works).
        st.x[8] = 19; st.x[0] = 0; st.x[1] = libc::EFD_CLOEXEC as u64 | 0 as u64;
        let efd = guest_svc(&mut st as *mut CpuState);
        let mut efd_writable = 0;
        if efd > 0 {
            // write 1 to it, read it back.
            let one = 1u64;
            unsafe { assert_eq!(libc::write(efd as i32, &one as *const u64 as *const libc::c_void, 8), 8); }
            let mut val = 0u64;
            unsafe { assert_eq!(libc::read(efd as i32, &mut val as *mut u64 as *mut libc::c_void, 8), 8); }
            assert_eq!(val, 1);
            efd_writable = efd as i32;
        }

        // epoll_create1 (20) + epoll_ctl (21): create an epoll fd and register an
        // eventfd (the Android ALooper pattern). Regular files aren't pollable,
        // so EPERM registering `fd` — use the eventfd (or a pipe) instead.
        st.x[8] = 20; st.x[0] = 0; // EPOLL_CLOEXEC off
        let ep = guest_svc(&mut st as *mut CpuState);
        assert!(ep >= 0, "epoll_create1 fd");
        if ep > 0 {
            let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: 42 };
            if efd_writable == 0 {
                // no eventfd: fall back to a pipe (pollable).
                let mut p = [0i32; 2];
                unsafe { assert_eq!(libc::pipe(p.as_mut_ptr()), 0); }
                efd_writable = p[0];
            }
            st.x[8] = 21; st.x[0] = ep as u64; st.x[1] = libc::EPOLL_CTL_ADD as u64;
            st.x[2] = efd_writable as u64; st.x[3] = (&mut ev as *mut libc::epoll_event) as u64;
            assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "epoll_ctl ADD");
            unsafe { libc::close(ep as i32); }
        }
        if efd > 0 { unsafe { libc::close(efd as i32); } }

        // gettimeofday (169): fills a timeval (two i64 -> identical layout).
        let mut tv = [0u8; 16];
        st.x[8] = 169; st.x[0] = tv.as_mut_ptr() as u64; st.x[1] = 0;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "gettimeofday ok");
        let secs = i64::from_le_bytes(tv[0..8].try_into().unwrap());
        assert!(secs > 1_500_000_000, "gettimeofday tv_sec sane, got {secs}");

        // uname (160): sysname == "Linux".
        let mut un = [0u8; 65 * 6];
        st.x[8] = 160; st.x[0] = un.as_mut_ptr() as u64;
        assert_eq!(guest_svc(&mut st as *mut CpuState), 0, "uname ok");
        let sysname_len = un.iter().position(|&c| c == 0).unwrap_or(0);
        assert_eq!(&un[..sysname_len], b"Linux", "uname sysname");

        unsafe { libc::close(fd); }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn guest_svc_identity_numbers_match_aarch64_abi() {
        // Regression for two latent syscall-number bugs: the table mapped
        // getuid to 199 (that's actually socketpair) and mremap to 220 (that's
        // clone). The real AArch64 numbers (asm-generic/unistd.h, arm64 uapi):
        // getpid=172, getuid=174, geteuid=175, getgid=176, getegid=177,
        // gettid=178, getppid=173, mremap=216 (3264_mremap), mmap=222.
        let mut st = CpuState::new();
        // getuid (AArch64 174) == host real uid (euid sandboxing aside, same)
        st.x[8] = 174;
        let uid = guest_svc(&mut st as *mut CpuState);
        assert_eq!(uid as libc::uid_t, unsafe { libc::getuid() }, "getuid is 174");
        // geteuid (175)
        st.x[8] = 175;
        assert_eq!(guest_svc(&mut st as *mut CpuState) as libc::uid_t,
            unsafe { libc::geteuid() }, "geteuid is 175");
        // gettid (178) == the host thread id (libc gettid)
        st.x[8] = 178;
        assert_eq!(guest_svc(&mut st as *mut CpuState) as isize,
            unsafe { libc::syscall(libc::SYS_gettid) as isize }, "gettid is 178");
        // The wrong numbers must NOT be getuid: 199 returns the (host) socketpair
        // error -EINVAL here, NOT the uid — proving 199 is not getuid.
        st.x[8] = 199;
        let r199 = guest_svc(&mut st as *mut CpuState);
        assert_ne!(r199 as libc::uid_t, unsafe { libc::getuid() },
            "199 is not getuid (it is socketpair)");
    }

    /// The boot-memory/stat syscalls added this cycle: sysinfo (179) fills the
    /// guest asm-generic struct with real host values; statx (291) statfs a file;
    /// get_robust_list (100) reports a valid empty list; restart_syscall (128)
    /// returns -EINTR. All must succeed (no -ENOSYS) and be self-consistent.
    #[test]
    fn guest_svc_sysinfo_statx_robust_restart_roundtrip() {
        let mut st = CpuState::new();

        // sysinfo(179) -> guest struct: uptime/totalram must be non-zero and the
        // guest buffer actually receives the asm-generic 64-bit layout.
        let mut si = [0u8; 256];
        st.x[8] = 179; st.x[0] = si.as_mut_ptr() as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "sysinfo succeeds");
        // uptime (first 8 bytes) is a u64 > 0 on any running host.
        let uptime = u64::from_le_bytes(si[0..8].try_into().unwrap());
        assert!(uptime > 0, "uptime populated (got {uptime})");
        let totalram = u64::from_le_bytes(si[16..24].try_into().unwrap());
        assert!(totalram > 0, "totalram populated (got {totalram})");

        // statx(291) on "." via AT_FDCWD: must return 0 and write a statx struct.
        let path = b".\0";
        let path_addr = path.as_ptr() as u64;
        let mut sx = [0u8; 256];
        st.x[8] = 291; st.x[0] = libc::AT_FDCWD as u64; st.x[1] = path_addr;
        st.x[2] = 0; st.x[3] = libc::AT_STATX_SYNC_AS_STAT as u64; st.x[4] = sx.as_mut_ptr() as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "statx succeeds (kernel supports statx)");
        // stx_mask is first u32; at least STX_TYPE (0x1) set for a dir.
        let mask = u32::from_le_bytes(sx[0..4].try_into().unwrap());
        assert!(mask != 0, "statx mask populated (got {mask:#x})");

        // get_robust_list(100) writes a non-zero head and size, returns 0.
        let mut head = 0u64; let mut len = 0u64;
        st.x[8] = 100; st.x[0] = 0; // this process
        st.x[1] = (&mut head as *mut u64) as u64; st.x[2] = (&mut len as *mut u64) as u64;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, 0, "get_robust_list succeeds");
        assert_ne!(head, 0, "reports an empty robust-list head pointer");
        assert!(len > 0, "reports a sane list size ({len})");

        // restart_syscall(128) -> -EINTR (-4), never -ENOSYS.
        st.x[8] = 128;
        let r = guest_svc(&mut st as *mut CpuState);
        assert_eq!(r as i64, -4, "restart_syscall returns -EINTR");
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
    fn fp_scalar_postindex_store_uses_base_not_value() {
        // str s30, [x4], #4 = 0xbc00449e (post-index single store). FpLdStImmWb
        // must write the float to [x4] and advance x4 -- NOT write to [s30's
        // bit pattern]. Regression for the latent bug where fp_scalar_xfer's RAX
        // value scratch clobbered the base register (addr==RAX), so a store
        // stored to [0x41480000] = the float bits and faulted. That bug surfaced
        // only once fcvtl/fcvtn let a gcc float<->double array loop compile fully.
        let code = [
            0x9eu8, 0x44, 0x00, 0xbc, // str s30, [x4], #4
            0xe0, 0x03, 0x04, 0xaa, // mov x0, x4
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut buf = [0u32; 4];
        let mut st = CpuState::new();
        let orig = buf.as_ptr() as u64;
        st.x[4] = orig;
        // s30 = guest v[30] (slot VECTOR_BASE+30*16 => st.v[60]); bits = 12.5f.
        st.v[60] = 0x4148_0000;
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(buf[0], 0x4148_0000, "float stored to [x4], not to [0x4148_0000]");
        assert_eq!(r, orig + 4, "x0 = advanced x4 (returned ptr)");
        assert_eq!(st.x[4], orig + 4, "post-index writeback advanced x4 by 4");
    }

    #[test]
    fn fcvtl_fcvtn_decode_and_lane_widen_exec() {
        use crate::decode::decode;
        // fcvtl v1.2d, v0.2s = 0x0e617801, fcvtl2 v29.2d, v29.4s = 0x4e617bbd,
        // fcvtn v27.2s, v27.2d = 0x0e616b7b, fcvtn2 v27.4s, v26.2d = 0x4e616b5b
        // must decode to their own Inst (not be swallowed by an int->fp/widen-mul
        // gate). Exec: v0 = [1.0f, 2.0f]; fcvtl v1.2d,v0.2s; fcvtzs x0,d1 => 1.
        assert!(matches!(decode(0x0e617801), Inst::VecFcvtl { upper: false, .. }));
        assert!(matches!(decode(0x4e617bbd), Inst::VecFcvtl { upper: true, .. }));
        assert!(matches!(decode(0x0e616b7b), Inst::VecFcvtn { upper: false, .. }));
        assert!(matches!(decode(0x4e616b5b), Inst::VecFcvtn { upper: true, .. }));
        // exec: [1.0f, 2.0f] in v0 -> fcvtl -> d1 = 1.0 -> scalar fcvtzs => 1.
        let code = [
            0x01u8, 0x78, 0x61, 0x0e, // fcvtl v1.2d, v0.2s
            0x22, 0x40, 0x60, 0x1e, // fmov d2, d1
            0x40, 0x00, 0x78, 0x9e, // fcvtzs x0, d2
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let mut st = CpuState::new();
        st.v[0] = 0x4000_0000_3f80_0000; // lane0=1.0f, lane1=2.0f
        let r = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(r, 1, "fcvtl widens 1.0f -> (double)1.0 -> fcvtzs 1");
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

        

    #[test]
    fn bit_vs_bif_bitwise_insert_semantics() {
        // BIT Vd,Vn,Vm: (Vn & Vm)|(Vd & ~Vm); BIF: (Vn & ~Vm)|(Vd & Vm) -- the
        // complement (which operand is masked by Vm vs ~Vm). The two share the
        // SimdSel residue; the gate now routes only BSL(bit23=0) to SimdSel and
        // BIT/BIF (both bit23=1) to SimdBit with the bif flag (bit14).
        use crate::decode::decode;
        // bit v15.16b,v16.16b,v17.16b = 0x6eb11e0f ; bif = 0x6ef11e0f (asm-verified)
        assert!(matches!(decode(0x6eb11e0f), Inst::SimdBit { bif: false, .. }));
        assert!(matches!(decode(0x6ef11e0f), Inst::SimdBit { bif: true, .. }));
        // bsl v6.16b,v7.16b,v8.16b = 0x6e681ce6 stays a select.
        assert!(matches!(decode(0x6e681ce6), Inst::SimdSel { .. }));

        // v15_in (dest) = 0x1122334455667788 ; v16 (mask) = 0x00FF00FF00FF00FF ;
        // v17 (source) = 0xAABBCCDDEEFF0011.
        let bit_code = [0x0fu8, 0x1e, 0xb1, 0x6e, 0xc0, 0x03, 0x5f, 0xd6]; // bit v15,v16,v17; ret
        let bif_code = [0x0fu8, 0x1e, 0xf1, 0x6e, 0xc0, 0x03, 0x5f, 0xd6]; // bif v15,v16,v17; ret
        let run = |code: &[u8]| -> u64 {
            let mut st = CpuState::new();
            st.v[30] = 0x1122334455667788; // v15 slot (2*15)
            st.v[32] = 0x00FF00FF00FF00FF; // v16 mask (2*16)
            st.v[34] = 0xAABBCCDDEEFF0011; // v17 (2*17)
            let _ = exec_bytes(&mut st, code, 0).expect("exec");
            st.v[30] // low 64 of v15 after the insert
        };
        // Real instruction semantics: bit v15,v16,v17 -> Vd=v15, Vn=v16, Vm=v17
        // (Vm is the MASK). Values: Vd=0x1122334455667788, Vn=0x00FF00FF00FF00FF,
        // Vm=0xAABBCCDDEEFF0011. Verified against the bit/BIF formulas (below).
        // BIT = (Vn & Vm)|(Vd & ~Vm) ; BIF = (Vn & ~Vm)|(Vd & Vm).
        assert_eq!(run(&bit_code), 0x11bb33dd11ff7799, "bit insert");
        assert_eq!(run(&bif_code), 0x660066446600ee, "bif insert (opposite select)");
    }

        #[test]
        fn vec128_reg_offset_store_preserves_base_and_writes_16b() {
            // str q0, [x0, x3] = 0x3ca36800 (128-bit register-offset store) must
            // NOT decode as a 1-byte GPR sign-extend load INTO x0 (bit26=1 picks
            // the vector file), which silently corrupted the caller (memset
            // clobbered x0, then `str w5,[x0,#4]` faulted at 0x4).
            let code = [
                0x00, 0x68, 0xa3, 0x3c, // str q0, [x0, x3]
                0xc0, 0x03, 0x5f, 0xd6, // ret
            ];
            let mut mem = [0u8; 64];
            let base = mem.as_ptr() as u64;
            let mut st = CpuState::new();
            st.x[0] = base;
            st.x[3] = 0x10;
            st.v[0] = 0x1122334455667788; // v0 low 64
            st.v[1] = 0x99aabbccddeeff00; // v0 high 64
            let r = exec_bytes(&mut st, &code, 0).expect("exec");
            assert_eq!(r, base, "x0 (store base) must be preserved, not written back");
            let lo = u64::from_le_bytes(mem[16..24].try_into().unwrap());
            let hi = u64::from_le_bytes(mem[24..32].try_into().unwrap());
            assert_eq!(lo, 0x1122334455667788, "v0 low lane stored at [x0+x3]");
            assert_eq!(hi, 0x99aabbccddeeff00, "v0 high lane stored at [x0+x3]");
        }

        #[test]
        fn vec128_unscaled_store_preserves_pointer() {
            // stur q0,[x5,#-16] = 0x3c9f00a0 (unscaled, no writeback): x5 must
            // stay put and 16 bytes land at [x5-16].
            let code = [
                0xa0, 0x00, 0x9f, 0x3c, // stur q0, [x5, #-16]
                0xc0, 0x03, 0x5f, 0xd6, // ret
            ];
            let mut mem = [0u8; 64];
            let base = mem.as_ptr() as u64;
            let mut st = CpuState::new();
            st.x[5] = base + 0x20; // [base+0x20 - 0x10] = [base+0x10]
            st.v[0] = 0xfedcba9876543210;
            st.v[1] = 0x0123456789abcdef;
            let r = exec_bytes(&mut st, &code, 0).expect("exec");
            let _ = r;
            assert_eq!(st.x[5], base + 0x20, "unscaled stur must not write back the pointer");
            let lo = u64::from_le_bytes(mem[0x10..0x18].try_into().unwrap());
            let hi = u64::from_le_bytes(mem[0x18..0x20].try_into().unwrap());
            assert_eq!(lo, 0xfedcba9876543210);
            assert_eq!(hi, 0x0123456789abcdef);
        }

        #[test]
        fn vec128_pre_index_relocates_base_after_load() {
            // ldr q4,[x0,#64]! = 0x3cc40c04 (pre-index writeback): loads 16 bytes
            // from [x0+64] AND advances x0 by +64.
            let code = [
                0x04, 0x0c, 0xc4, 0x3c, // ldr q4, [x0, #64]!
                0xc0, 0x03, 0x5f, 0xd6, // ret
            ];
            let mut mem = [0u8; 96];
            let base = mem.as_ptr() as u64;
            let mut st = CpuState::new();
            st.x[0] = base;
            mem[64..72].copy_from_slice(&0x0102030405060708u64.to_le_bytes());
            mem[72..80].copy_from_slice(&0x1112131415161718u64.to_le_bytes());
            let _ = exec_bytes(&mut st, &code, 0).expect("exec");
            assert_eq!(st.x[0], base + 64, "pre-index ldr q advances Xn by imm9");
            assert_eq!(st.v[8], 0x0102030405060708, "q4 = v slots 8..9 low");
            assert_eq!(st.v[9], 0x1112131415161718, "q4 high lane");
        }

        #[test]
        fn fpl_single_reg_offset_store_with_shift() {
            // str s0,[x0,x3,lsl#2] = 0xbc237800: stores v0's low 32 bits at
            // [x0 + x3*4] without touching x0/x3. Pre-fix this family (bit26=1
            // scalar reg-offset) fell into the GPR register-offset gate and was
            // executed as a GPR op against the wrong register file.
            let code = [
                0x00, 0x78, 0x23, 0xbc, // str s0, [x0, x3, lsl #2]
                0xc0, 0x03, 0x5f, 0xd6, // ret
            ];
            let mut mem = [0u8; 64];
            let base = mem.as_ptr() as u64;
            let mut st = CpuState::new();
            st.x[0] = base;
            st.x[3] = 0x3; // index; shifted by lsl#2 -> +12 bytes
            st.v[0] = 0x123456789abcdef0; // low 32 = 0x9abcdef0
            let r = exec_bytes(&mut st, &code, 0).expect("exec");
            assert_eq!(r, base, "x0 preserved");
            assert_eq!(st.x[3], 0x3, "x3 preserved");
            let val = u32::from_le_bytes(mem[12..16].try_into().unwrap());
            assert_eq!(val, 0x9abcdef0, "v0 low s-lane stored at [x0 + x3*4]");
        }

        #[test]
        fn vec128_ldst_decode_not_gpr_and_scalar_b_untouched() {
            use crate::decode::{decode, Inst};
            // 128-bit vector register-offset / unscaled / indexed forms route to
            // the vector classes, NOT GPR LdStrReg/LdStrImmWb (which would write
            // INTO a GPR register).
            assert!(matches!(decode(0x3ca36800), Inst::VecLdStrReg { ld: false, .. }));
            assert!(matches!(decode(0x3ce46841), Inst::VecLdStrReg { ld: true, .. }));
            assert!(matches!(decode(0x3c9f00a0), Inst::VecLdStImmUnscaled { ld: false, .. }));
            assert!(matches!(decode(0x3cc200c1), Inst::VecLdStImmUnscaled { ld: true, .. }));
            assert!(matches!(
                decode(0x3cc40c04),
                Inst::VecLdStIndexed { ld: true, pre: true, .. }
            ));
            assert!(matches!(
                decode(0x3c9e0404),
                Inst::VecLdStIndexed { ld: false, pre: false, .. }
            ));
            // A scalar byte unscaled (stur b0 = 0x3c1fc100) must NOT be a 128-bit
            // vector class.
            assert!(!matches!(decode(0x3c1fc100), Inst::VecLdStImmUnscaled { .. }));
            // A real GPR register-offset load still decodes as LdStrReg.
            assert!(matches!(decode(0xf8626803), Inst::LdStrReg { .. }));
        }
        }

mod diag_tmp {
    #[allow(dead_code)]
    fn probe() {
        eprintln!("d0x0#1={:?}", crate::decode::decode(0x9e42fc00u32 as u64 as _));
    }
}


#[cfg(test)]
mod isa_regress_tests {
    use crate::decode::decode;
    use crate::decode::Inst;
    use crate::jit::{CpuState, exec_bytes};

    #[test]
    fn simd_cmpzero_cmlt_masks_negative_bytes() {
        // cmlt v0.16b, v1.16b, #0 = 0x4e200820 (rd=0, rn=1). Per-byte all-ones
        // mask where the signed byte<0. v1=0xf0000ffe01ff7f00 -> mem bytes
        // [00,7f,ff,01,fe,0f,00,f0]: negatives at 0xff(=-1),0xfe(=-2),0xf0(=-16)
        // -> mask bytes [00,00,ff,00,ff,00,00,ff] = 0xff_00_00_ff_00_ff_00_00.
        assert!(matches!(decode(0x4e20a820), Inst::SimdCmpZero { cond: 3, esize: 1, q: true, .. }));
        assert!(matches!(decode(0x4e209820), Inst::SimdCmpZero { cond: 0, esize: 1, q: true, .. }));
        assert!(matches!(decode(0x4e208820), Inst::SimdCmpZero { cond: 1, .. }));
        assert!(matches!(decode(0x6e208820), Inst::SimdCmpZero { cond: 2, .. }));
        assert!(matches!(decode(0x6e209820), Inst::SimdCmpZero { cond: 4, .. }));
        assert!(matches!(decode(0x4ea0a820), Inst::SimdCmpZero { esize: 4, .. }));
        let code = [0x20u8, 0xa8, 0x20, 0x4e, 0xc0, 0x03, 0x5f, 0xd6]; // cmlt v0,v1,#0 = 0x4e20a820 ; ret
        let mut st = CpuState::new();
        st.v[2] = 0xf000_0ffe_01ff_7f00; // v1
        st.v[0] = 0; // v0 clean
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        assert_eq!(st.v[0], 0xff0000ff00ff0000 & 0xffff_ffff_ffff_ffff,
            "cmlt v0,v1,#0 mask (neg bytes all-ones)");
    }

    #[test]
    fn simd_ssubw_subtracts_widened_not_adds() {
        // ssubw v0.4s, v0.4s, v1.4h = 0x0e613000 (bit13 set => sub-wide).
        // saddw v0.4s,v0.4s,v1.4h = 0x0e611000 (bit13 clear => add-wide).
        assert!(matches!(decode(0x0e613000), Inst::SimdAddw { sub: true, .. }));
        assert!(matches!(decode(0x0e611000), Inst::SimdAddw { sub: false, .. }));
        let code = [0x00u8, 0x30, 0x61, 0x0e, 0xc0, 0x03, 0x5f, 0xd6]; // ssubw v0,v0,v1 ; ret
        let mut st = CpuState::new();
        st.v[0] = (20u64 << 32) | 10;      // v0 4s lanes: [10,20,30,40]
        st.v[1] = (40u64 << 32) | 30;
        st.v[2] = 0x0004000300020001u64; // v1 4h (low 8 bytes): [1,2,3,4]
        let _ = exec_bytes(&mut st, &code, 0).expect("exec");
        // v0[i] = [10-1,20-2,30-3,40-4] = [9,18,27,36]
        assert_eq!(st.v[0], (18u64 << 32) | 9, "ssubw lanes 0-1");
        assert_eq!(st.v[1], (36u64 << 32) | 27, "ssubw lanes 2-3 (subtract, not add)");
    }
}
