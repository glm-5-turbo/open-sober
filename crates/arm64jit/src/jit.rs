// SPDX-License-Identifier: MIT
//
// In-process JIT runtime: owns the guest CpuState, compiles a guest code
// buffer (a contiguous run of AArch64 instructions starting at a known
// address) into host x86-64 in an executable mapping, and executes it.
//
// Execution convention: the translated entry takes a pointer to CpuState.
// The prologue loads it into RBX (the base the translator reads/writes).

use std::ptr;
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

/// Supervisor-call dispatcher. AArch64 uses x8 as the syscall number and x0-x5
/// as args (like the iOS ARM64 kernel asvp); the AArch64 syscall ABI is
/// x8=number, x0..x5 args, return in x0 (negative = -errno). We forward the
/// handful the guest needs early on to real host syscalls via `libc` (the
/// kernel numbers match Linux AArch64 == x86-64 for the common set, so libc's
/// `syscall` with the same number works for mmap/open/futex/exit_group/...).
pub extern "C" fn guest_svc(st: *mut CpuState) -> u64 {
    let s = unsafe { &mut *st };
    let nr = s.x[8];
    let a = [s.x[0], s.x[1], s.x[2], s.x[3], s.x[4], s.x[5]];
    if std::env::var("JIT_TRACE_SVC").is_ok() {
        eprintln!("guest svc {:x} ({}) a0={:#x} a1={:#x} a2={:#x}",
            nr, nr, a[0], a[1], a[2]);
    }
    // exit(93) / exit_group(94) end the process cleanly.
    if nr == 93 || nr == 94 {
        eprintln!("guest_svc: syscall({nr}) status {}", a[0] as i32);
        std::process::exit(a[0] as i32);
    }
    eprintln!(
        "guest_svc: unhandled AArch64 syscall {nr} -> -ENOSYS (args {:#x},{:#x},{:#x})",
        a[0], a[1], a[2]
    );
    (-38i64) as u64 // -ENOSYS
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
        if pc < base || pc - base + 4 > image.len() as u64 {
            return Err(format!(
                "run_loop: pc 0x{pc:x} outside image [0x{base:x}, 0x{:x})",
                base + image.len() as u64
            ));
        }
        let block = compile_image(image, base, pc, state)?;
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
    // Invariant: every addresses in frontier is a candidate block start.
    while let Some(addr) = frontier.pop() {
        if host_of_guest.contains_key(&addr) {
            continue; // already emitted
        }
        let mut cur = addr;
        loop {
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
                        frontier.push(target);
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

    // Resolve fixups (buffer-relative).
    for fx in &fixups {
        let target = *host_of_guest
            .get(&fx.target_pc)
            .ok_or_else(|| format!("branch/call to untranslated pc {:x}", fx.target_pc))?;
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
}
