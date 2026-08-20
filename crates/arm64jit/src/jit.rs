// SPDX-License-Identifier: MIT
//
// In-process JIT runtime: owns the guest CpuState, compiles a guest code
// buffer (a contiguous run of AArch64 instructions starting at a known
// address) into host x86-64 in an executable mapping, and executes it.
//
// Execution convention: the translated entry takes a pointer to CpuState.
// The prologue loads it into RBX (the base the translator reads/writes).

use std::ptr;

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
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            x: [0; 32],
            pc: 0,
            nzcv: 0,
            pad: 0,
        }
    }
    pub fn set(&mut self, reg: usize, val: u64) {
        self.x[reg] = val;
    }
    pub fn get(&self, reg: usize) -> u64 {
        self.x[reg]
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
    let mut host_of_guest: std::collections::HashMap<u64, usize> =
        std::collections::HashMap::new();
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
    Ok(JitBlock { ptr, len: code.len() })
}

/// Execute a compiled block against `state`, returning the value left in x0.
pub unsafe fn run(blk: &JitBlock, state: *mut CpuState) -> u64 { unsafe {
    let f: extern "C" fn(*mut CpuState) -> u64 = std::mem::transmute(blk.ptr);
    f(state)
}}

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
    }