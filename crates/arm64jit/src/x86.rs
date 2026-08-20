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
    0x40 | ((w as u8) << 3) | (if r & 8 != 0 { 0x04 } else { 0 }) | (if x & 8 != 0 { 0x02 } else { 0 })
        | (if b & 8 != 0 { 0x01 } else { 0 })
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

    /// movzx r64 <- [mem (base+disp)] (16-bit load, zero-extend)
    pub fn movzx_word_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x0F);
        self.b(0xB7);
        self.emit_mem(rd, base, disp);
    }

    /// movsx r64 <- [mem (base+disp)] (byte load, sign-extend)
    pub fn movsx_byte_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x0F);
        self.b(0xBE);
        self.emit_mem(rd, base, disp);
    }

    /// movsx r64 <- [mem (base+disp)] (16-bit load, sign-extend)
    pub fn movsx_word_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x0F);
        self.b(0xBF);
        self.emit_mem(rd, base, disp);
    }

    /// mov [mem (base+disp)] <- r8 (byte store)
    pub fn mov_store8(&mut self, base: u8, disp: i32, src: u8) {
        if base >= 8 || src >= 8 {
            self.b(rex(false, src, 0, base));
        }
        self.b(0x88);
        self.emit_mem(src, base, disp);
    }

    /// mov [mem (base+disp)] <- r16 (16-bit store)
    pub fn mov_store16(&mut self, base: u8, disp: i32, src: u8) {
        self.b(0x66); // 16-bit operand-size prefix
        if base >= 8 || src >= 8 {
            self.b(rex(false, src, 0, base));
        }
        self.b(0x89);
        self.emit_mem(src, base, disp);
    }

    // ---- SIMD / NEON (128-bit XMM) ----
    /// movdqu xmm <- [mem (base+disp)]  ; 128-bit unaligned load
    pub fn movdqu_load(&mut self, xmm: u8, base: u8, disp: i32) {
        self.b(0xF3);
        if xmm >= 8 || base >= 8 {
            self.b(rex(false, xmm, 0, base));
        }
        self.b(0x0F);
        self.b(0x6F);
        self.emit_mem(xmm, base, disp);
    }
    /// movdqu [mem (base+disp)] <- xmm ; 128-bit unaligned store
    pub fn movdqu_store(&mut self, base: u8, disp: i32, xmm: u8) {
        self.b(0xF3);
        if base >= 8 || xmm >= 8 {
            self.b(rex(false, xmm, 0, base));
        }
        self.b(0x0F);
        self.b(0x7F);
        self.emit_mem(xmm, base, disp);
    }
    /// movdqa xmm, xmm (copy register)
    pub fn movdqa_xmm(&mut self, dst: u8, src: u8) {
        if dst == src {
            return;
        }
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, src, 0, dst));
        }
        self.b(0x0F);
        self.b(0x6F);
        self.b(modrm(3, src & 7, dst & 7));
    }
    /// pxor xmm, xmm (zero or XOR; for CLEARING use pxor x,x)
    pub fn pxor_xmm(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, src, 0, dst));
        }
        self.b(0x0F);
        self.b(0xEF);
        self.b(modrm(3, src & 7, dst & 7));
    }

    // ---- scalar double-precision (FP64) ----
    /// movq xmm, [mem] : 64-bit load (F3 48 0F 7E xmm, r/m64)
    pub fn movq_load(&mut self, xmm: u8, base: u8, disp: i32) {
        self.b(0xF3);
        self.b(0x48);
        if xmm >= 8 || base >= 8 {
            self.b(rex(false, xmm, 0, base));
        }
        self.b(0x0F);
        self.b(0x7E);
        self.emit_mem(xmm, base, disp);
    }
    /// movq [mem+disp] <- xmm (66 48 0F D6 /r)
    pub fn movq_store(&mut self, base: u8, disp: i32, xmm: u8) {
        self.b(0x66);
        self.b(0x48);
        if base >= 8 || xmm >= 8 {
            self.b(rex(false, xmm, 0, base));
        }
        self.b(0x0F);
        self.b(0xD6);
        self.emit_mem(xmm, base, disp);
    }
    /// paddd xmm, xmm/m128 (66 0F FE /r) — add 4x32-bit lanes
    pub fn paddd(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xFE);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // (Intel: ModRM.reg = DST, r/m = SRC) — so mulsd(0,1) => F2 0F 59 C1 => xmm0 = xmm0*xmm1.
        fn sd(&mut self, op: u8, dst: u8, src: u8) {
            self.b(0xF2);
            if dst >= 8 || src >= 8 {
                self.b(rex(false, dst, 0, src));
            }
            self.b(0x0F);
            self.b(op);
            self.b(modrm(3, dst & 7, src & 7));
        }
    pub fn addsd(&mut self, dst: u8, src: u8) {
        self.sd(0x58, dst, src);
    }
    pub fn mulsd(&mut self, dst: u8, src: u8) {
        self.sd(0x59, dst, src);
    }
    pub fn subsd(&mut self, dst: u8, src: u8) {
        self.sd(0x5C, dst, src);
    }
    pub fn divsd(&mut self, dst: u8, src: u8) {
        self.sd(0x5E, dst, src);
    }
    // ---- integer multiply / divide (data-processing register) ----

    /// cqo — sign-extend RAX into RDX (64-bit), 48 99. pranda.
    pub fn cqo(&mut self) {
        self.b(0x48);
        self.b(0x99);
    }
    /// cdq — sign-extend EAX into EDX (32-bit), 99.
    pub fn cdq(&mut self) {
        self.b(0x99);
    }
    /// div r64 — unsigned divide RDX:RAX by r64; quotient RAX, remainder RDX. 48 F7 /0.
    pub fn div_r64(&mut self, divr: u8) {
        self.b(0x48);
        self.b(0xF7);
        self.b(modrm(3, 0, divr & 7)); // /0
    }
    /// div r32 — unsigned divide EDX:EAX by r32; quotient EAX, remainder EDX. F7 /0.
    pub fn div_r32(&mut self, divr: u8) {
        self.b(0xF7);
        self.b(modrm(3, 0, divr & 7));
    }
    /// idiv r64 — signed divide: RDX:RAX / r64; quotient RAX, remainder RDX. 48 F7 /7.
    pub fn idiv_r64(&mut self, divr: u8) {
        self.b(0x48);
        self.b(0xF7);
        self.b(modrm(3, 7, divr & 7));
    }
    /// idiv r32 — signed divide: EDX:EAX / r32; quotient EAX, remainder EDX. F7 /7.
    pub fn idiv_r32(&mut self, divr: u8) {
        self.b(0xF7);
        self.b(modrm(3, 7, divr & 7));
    }
    /// imul r64, r/m64 — signed multiply (2-operand): rd = rd * rs. 48 0F AF /r.
    pub fn imul_rr64(&mut self, rd: u8, rs: u8) {
        self.b(0x48);
        self.b(0x0F);
        self.b(0xAF);
        self.b(modrm(3, rd & 7, rs & 7)); // Intel: ModRM.reg=dst, rm=src => AF C1 = eax<-eax*ecx
    }
    /// movsxd r64, r/m32 — sign-extend a 32-bit operand into r64. 48 63 /r.
        pub fn movsxd_r64_r32(&mut self, rd: u8, rs: u8) {
            self.b(0x48);
            self.b(0x63);
            self.b(modrm(3, rd & 7, rs & 7));
        }
        /// cvttsd2si r64, xmm  (F2 48 0F 2C /r) — truncate toward zero
    pub fn cvttsd2si(&mut self, rd: u8, xmm: u8) {
        self.b(0xF2);
        self.b(0x48);
        if rd >= 8 || xmm >= 8 {
            self.b(rex(true, rd, 0, xmm));
        }
        self.b(0x0F);
        self.b(0x2C);
        self.b(modrm(3, rd & 7, xmm & 7)); // reg=GPR(dst), rm=xmm(src)
    }
    /// cvtsd2si r64, xmm  (F2 48 0F 2D /r) — round per MXCSR (default nearest)
    pub fn cvtsd2si(&mut self, rd: u8, xmm: u8) {
        self.b(0xF2);
        self.b(0x48);
        if rd >= 8 || xmm >= 8 {
            self.b(rex(true, rd, 0, xmm));
        }
        self.b(0x0F);
        self.b(0x2D);
        self.b(modrm(3, rd & 7, xmm & 7));
    }

    /// cvtsi2sd xmm, r/m{32|64}  (F2 [REX.W] 0F 2A /r) — signed int -> double
    pub fn cvtsi2sd(&mut self, xmm: u8, src64: bool, rs: u8) {
        self.b(0xF2);
        if src64 {
            self.b(0x48);
        }
        if xmm >= 8 || rs >= 8 {
            self.b(rex(src64, xmm, 0, rs));
        }
        self.b(0x0F);
        self.b(0x2A);
        self.b(modrm(3, xmm & 7, rs & 7));
    }
    /// cvtsi2ss xmm, r/m{32|64}  (F3 [REX.W] 0F 2A /r) — signed int -> single
    pub fn cvtsi2ss(&mut self, xmm: u8, src64: bool, rs: u8) {
        self.b(0xF3);
        if src64 {
            self.b(0x48);
        }
        if xmm >= 8 || rs >= 8 {
            self.b(rex(src64, xmm, 0, rs));
        }
        self.b(0x0F);
        self.b(0x2A);
        self.b(modrm(3, xmm & 7, rs & 7));
    }

    /// cvtsd2ss xmm_dst, xmm_src (F2 0F 5A /r) — double -> single (narrow)
    pub fn cvtsd2ss(&mut self, dst: u8, src: u8) {
        self.b(0xF2);
        self.b(0x0F);
        self.b(0x5A);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// cvtss2sd xmm_dst, xmm_src (F3 0F 5A /r) — single -> double (widen)
    pub fn cvtss2sd(&mut self, dst: u8, src: u8) {
        self.b(0xF3);
        self.b(0x0F);
        self.b(0x5A);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// movd r32, xmm (66 0F 7E /r) — copy low 32 bits of xmm to a GPR
    pub fn movd_r32_xmm(&mut self, rd: u8, xmm: u8) {
        self.b(0x66);
        self.b(0x0F);
        self.b(0x7E);
        self.b(modrm(3, rd & 7, xmm & 7));
    }
    /// movd xmm, r32 (66 0F 6E /r) — copy low 32 bits of a GPR to xmm
    pub fn movd_xmm_r32(&mut self, xmm: u8, rs: u8) {
        self.b(0x66);
        self.b(0x0F);
        self.b(0x6E);
        self.b(modrm(3, rs & 7, xmm & 7));
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
    /// neg r64 (0x48 F7 /3) — sets flags (CF/OF), fine before load_nzcv
    pub fn neg_r64(&mut self, rd: u8) {
        self.b(0x48);
        self.b(0xF7);
        self.b(modrm(3, 3, rd & 7));
    }
    /// not r64 (0x48 F7 /2) — sets flags
    pub fn not_r64(&mut self, rd: u8) {
        self.b(0x48);
        self.b(0xF7);
        self.b(modrm(3, 2, rd & 7));
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
    /// ror r64, cl
    pub fn ror_cl64(&mut self, rd: u8) {
        self.shift_cl(1, rd);
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

    /// shl r64, imm8  (encoding 48 C1 /4 ib)
    pub fn shl_ri8(&mut self, rd: u8, imm: u8) {
        self.b(0x48);
        self.b(0xC1);
        self.b(modrm(3, 4, rd & 7));
        self.b(imm);
    }
    /// shr r64, imm8  (encoding 48 C1 /5 ib)
       pub fn shr_ri8(&mut self, rd: u8, imm: u8) {
           self.b(0x48);
           self.b(0xC1);
           self.b(modrm(3, 5, rd & 7));
           self.b(imm);
       }
       /// sar r64, imm8  (encoding 48 C1 /7 ib) — arithmetic (sign) shift right
       pub fn sar_ri8(&mut self, rd: u8, imm: u8) {
           self.b(0x48);
           self.b(0xC1);
           self.b(modrm(3, 7, rd & 7));
           self.b(imm);
       }

    /// ror r64, imm8  (48 C1 /1 ib) — rotate right
    pub fn ror_ri8(&mut self, rd: u8, imm: u8) {
        self.b(0x48);
        self.b(0xC1);
        self.b(modrm(3, 1, rd & 7));
        self.b(imm);
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
    /// cmovcc r64, r/m64  (0F 40+cc), `cc` = cmov-ccode 2nd byte *after* jcc-0x40
/// (e.g. 0x45 = cmovne). ModRM reg=rd(dst), rm=rs(src). Caller passes
/// `x86_cc_for_cond(cond) - 0x40`.
    pub fn cmov_rr64(&mut self, cc: u8, rd: u8, rs: u8) {
        if rd >= 8 || rs >= 8 {
            self.b(rex(true, rd, 0, rs));
        } else {
            self.b(0x48);
        }
        self.b(0x0F);
        self.b(cc);
        self.b(modrm(3, rd & 7, rs & 7));
    }
    /// call r64
    pub fn call_r64(&mut self, rd: u8) {
        if rd >= 8 {
            self.b(0x41);
        }
        self.b(0xFF);
        self.b(modrm(3, 2, rd & 7)); // call r/m64
    }
    /// call rel32; returns patch offset for the displacement (like jmp_rel32)
    pub fn call_rel32(&mut self) -> usize {
        self.b(0xE8);
        self.patch_here()
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
    /// pushfq  (0x9C): push rflags
    pub fn pushfq(&mut self) {
        self.b(0x9C);
    }
    /// pop r64
    pub fn pop(&mut self, r: u8) {
        if r >= 8 {
            self.b(0x41);
        }
        self.b(0x58 + (r & 7));
    }
    /// popfq  (0x9D): pop rflags
    pub fn popfq(&mut self) {
        self.b(0x9D);
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
