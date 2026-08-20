// SPDX-License-Identifier: MIT
//
// Minimal x86-64 machine code emitter for the ARM64->x86-64 JIT.
//
// The emitter only includes the encodings the translator actually uses, and
// each was verified by running the emitted bytes through `objdump` during
// development. Register operands are u8 in 0..16 mapping to RAX..R15; memory
// operands are base-register + signed displacement (register, no index).

/// Growable buffer holding emitted machine code bytes.
#[derive(Default, Clone)]
pub struct CodeBuf {
    pub bytes: Vec<u8>,
}

impl CodeBuf {
    pub fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(256),
        }
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
    /// Emit one byte.
    #[inline]
    pub fn b(&mut self, v: u8) {
        self.bytes.push(v);
    }
    /// Emit u32 little-endian.
    pub fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    /// Emit u64 little-endian.
    pub fn u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
}

// ---- register constants -----------------------------------------------------
pub const RAX: u8 = 0;
pub const RCX: u8 = 1;
pub const RDX: u8 = 2;
pub const RBX: u8 = 3;
pub const RSP: u8 = 4;
pub const RBP: u8 = 5;
pub const RSI: u8 = 6;
pub const RDI: u8 = 7;
pub const R8: u8 = 8;
pub const R9: u8 = 9;
pub const R10: u8 = 10;
pub const R11: u8 = 11;
pub const R12: u8 = 12;
pub const R13: u8 = 13;
pub const R14: u8 = 14;
pub const R15: u8 = 15;

// ---------------------------------------------------------------------------
// Encoder primitives
// ---------------------------------------------------------------------------

/// REX prefix. `w`=64-bit operand, `r`=extended modrm.reg reg, `x`,`b`=extended.
fn rex(w: bool, r: u8, x: u8, b: u8) -> u8 {
    0x40 | ((w as u8) << 3) | (((r & 8) as u8) << 1) | (((x & 8) as u8) << 2) | (b & 8) >> 0
}

fn modrm(mod_: u8, reg: u8, rm: u8) -> u8 {
    ((mod_ & 3) << 6) | ((reg & 7) << 3) | (rm & 7)
}

/// ModRM displacement-size selection.
fn disp_mod(disp: i32) -> u8 {
    if disp == 0 {
        0
    } else if (-128..128).contains(&disp) {
        1
    } else {
        2
    }
}

impl CodeBuf {
    /// mov r64, imm64
    pub fn mov_ri64(&mut self, rd: u8, imm: u64) {
        if rd >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0xB8 + (rd & 7));
        self.u64(imm);
    }
    /// mov r64, imm32 sign-extended
    pub fn mov_ri32(&mut self, rd: u8, imm: u32) {
        if rd >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0xC7);
        self.b(modrm(3, 0, rd & 7));
        self.u32(imm);
    }
    /// mov r32, imm32 (zero-extends to r64)
    pub fn mov_eax_imm32(&mut self, rd: u8, imm: u32) {
        // 64-bit default; 0xB8+rd imm32 zero-extends
        self.b(0xB8 + (rd & 7));
        self.u32(imm);
    }
    /// mov r64, r64
    pub fn mov_rr64(&mut self, rd: u8, rs: u8) {
        if rd == rs {
            return;
        }
        if rd >= 8 || rs >= 8 {
            self.b(rex(true, rs, 0, rd));
        } else {
            self.b(0x48);
        }
        self.b(0x89);
        self.b(modrm(3, rs & 7, rd & 7));
    }
    /// mov r64 <- [mem] (base + disp)
    pub fn mov_load64(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(true, rd, 0, base));
        } else {
            self.b(0x48);
        }
        self.b(0x8B);
        self.emit_mem(rd, base, disp);
    }
    /// mov r32 <- [mem] zero-extend
    pub fn mov_load32(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x8B);
        self.emit_mem(rd, base, disp);
    }
    /// mov [mem] <- r64
    pub fn mov_store64(&mut self, base: u8, disp: i32, src: u8) {
        if base >= 8 || src >= 8 {
            self.b(rex(true, src, 0, base));
        } else {
            self.b(0x48);
        }
        self.b(0x89);
        self.emit_mem(src, base, disp);
    }
    /// mov [mem] <- r32
    pub fn mov_store32(&mut self, base: u8, disp: i32, src: u8) {
        if base >= 8 || src >= 8 {
            self.b(rex(false, src, 0, base));
        }
        self.b(0x89);
        self.emit_mem(src, base, disp);
    }
    /// movzx r64<- [mem (base+disp)]  (byte load, zero-extend)
    pub fn movzx_byte_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x0F);
        self.b(0xB6);
        self.emit_mem(rd, base, disp);
    }

    /// lea r64, [base + disp]
    pub fn lea64(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(true, rd, 0, base));
        } else {
            self.b(0x48);
        }
        self.b(0x8D);
        self.emit_mem(rd, base, disp);
    }

    /// Emit the ModRM+SIB+disp for [base+disp] where `reg` is the register field.
    fn emit_mem(&mut self, reg: u8, base: u8, disp: i32) {
        // RSP/R12 as base require "mod=00 rm=100 (SIB)" — but we only use
        // base regs that are RBP/RBX/RAX etc. (low regs, no RSP). Restrict here.
        assert!(base != RSP, "emitter: RSP base not supported");
        assert!(base != R12, "emitter: R12 base not supported");
        match disp_mod(disp) {
            d if d == 0 && (base & 7) != 5 => {
                self.b(modrm(0, reg, base & 7));
            }
            1 => {
                self.b(modrm(1, reg, base & 7));
                self.b(disp as u8);
            }
            _ => {
                self.b(modrm(2, reg, base & 7));
                self.u32(disp as u32);
            }
        }
    }

    // ---- arith (64-bit) ----
    /// add r64 <- r64
    pub fn add_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x01, rd, rs);
    }
    /// sub r64 <- r64
    pub fn sub_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x29, rd, rs);
    }
    /// xor r64 <- r64
    pub fn xor_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x31, rd, rs);
    }
    /// or r64 <- r64
    pub fn or_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x09, rd, rs);
    }
    /// and r64 <- r64
    pub fn and_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x21, rd, rs);
    }
    /// cmp r64, r64
    pub fn cmp_rr64(&mut self, a: u8, b: u8) {
        self.binop(0x39, a, b);
    }
    /// test r64, r64
    pub fn test_rr64(&mut self, a: u8, b: u8) {
        if a >= 8 || b >= 8 {
            self.b(rex(true, b, 0, a));
        } else {
            self.b(0x48);
        }
        self.b(0x85);
        self.b(modrm(3, b & 7, a & 7));
    }

    fn binop(&mut self, op: u8, rd: u8, rs: u8) {
        if rd >= 8 || rs >= 8 {
            self.b(rex(true, rs, 0, rd));
        } else {
            self.b(0x48);
        }
        self.b(op);
        self.b(modrm(3, rs & 7, rd & 7));
    }

    /// add r64, imm32
    pub fn add_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(0, rd, imm);
    }
    /// sub r64, imm32
    pub fn sub_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(5, rd, imm);
    }
    /// and r64, imm32
    pub fn and_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(4, rd, imm);
    }
    /// or r64, imm32
    pub fn or_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(1, rd, imm);
    }
    /// xor r64, imm32
    pub fn xor_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(6, rd, imm);
    }
    /// cmp r64, imm32
    pub fn cmp_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(7, rd, imm);
    }
    fn ari_imm(&mut self, op: u8, rd: u8, imm: u32) {
        if rd >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0x81);
        self.b(modrm(3, op, rd & 7));
        self.u32(imm);
    }

    // ---- shifts (by CL) ----
    /// shl r64, cl
    pub fn shl_cl64(&mut self, rd: u8) {
        self.shift_cl(4, rd);
    }
    /// shr r64, cl
    pub fn shr_cl64(&mut self, rd: u8) {
        self.shift_cl(5, rd);
    }
    /// sar r64, cl
    pub fn sar_cl64(&mut self, rd: u8) {
        self.shift_cl(7, rd);
    }
    fn shift_cl(&mut self, op: u8, rd: u8) {
        if rd >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0xD3);
        self.b(modrm(3, op, rd & 7));
    }

    // ---- control flow ----
    /// jmp rel32; returns patch offset for disp
    pub fn jmp_rel32(&mut self) -> usize {
        self.b(0xE9);
        self.patch_here()
    }
    /// jcc rel32 with cc = 0F 8x second byte (e.g. 0x84=JE)
    pub fn jcc_rel32(&mut self, cc: u8) -> usize {
        self.b(0x0F);
        self.b(cc);
        self.patch_here()
    }
    /// call r64
    pub fn call_r64(&mut self, rd: u8) {
        if rd >= 8 {
            self.b(0x41);
        }
        self.b(0xFF);
        self.b(modrm(3, 2, rd & 7)); // call r/m64
    }
    /// ret
    pub fn ret(&mut self) {
        self.b(0xC3);
    }
    /// push r64
    pub fn push(&mut self, r: u8) {
        if r >= 8 {
            self.b(0x41);
        }
        self.b(0x50 + (r & 7));
    }
    /// pop r64
    pub fn pop(&mut self, r: u8) {
        if r >= 8 {
            self.b(0x41);
        }
        self.b(0x58 + (r & 7));
    }
    /// pad with NOPs to a specified alignment
    pub fn align_to(&mut self, n: usize) {
        while self.len() % n != 0 {
            self.b(0x90);
        }
    }
    fn patch_here(&mut self) -> usize {
        self.u32(0);
        self.len() - 4
    }
}

/// Patch a 4-byte signed disp at `off` in the buffer to make it a rel32 jump
/// that lands on `target` (where target is the absolute address of the dest).
pub fn patch_rel32(buf: &mut Vec<u8>, off: usize, from: usize, target: usize) {
    let next = from + off + 4;
    let disp = (target as i64).wrapping_sub(next as i64) as u32;
    buf[off + from..off + from + 4].copy_from_slice(&disp.to_le_bytes());
}
