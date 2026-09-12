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
    /// movzx r32, [mem]  (0F B7 /r): zero-extend a 16-bit word into the full
    /// 32-bit register. Used by rev16 so the 16-bit rotate/store operate on a
    /// cleanly zero-extended value.
    pub fn mov_load16(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(false, rd, 0, base));
        }
        self.b(0x0F);
        self.b(0xB7);
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

    /// movsx r64 <- [mem (base+disp)] (byte load, sign-extend to 64)
    pub fn movsx_byte_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(true, rd, 0, base)); // REX.W: write the full 64-bit r64
        } else {
            self.b(0x48);
        }
        self.b(0x0F);
        self.b(0xBE);
        self.emit_mem(rd, base, disp);
    }

    /// movsx r64 <- [mem (base+disp)] (16-bit load, sign-extend to 64)
    pub fn movsx_word_mem(&mut self, rd: u8, base: u8, disp: i32) {
        if rd >= 8 || base >= 8 {
            self.b(rex(true, rd, 0, base)); // REX.W: write the full 64-bit r64
        } else {
            self.b(0x48);
        }
        self.b(0x0F);
        self.b(0xBF);
        self.emit_mem(rd, base, disp);
    }

    /// movzx r64 <- byte [base + idx*1 + disp]  (zero-extend indexed byte load).
    /// Used by TBL/TBX: table byte at a runtime index. `idx` is a 64-bit reg.
    /// ModRM rm=base(r), SIB scale=0 index=idx base=base, then disp32.
    pub fn movzx_byte_mem_idxd(&mut self, rd: u8, base: u8, idx: u8, disp: i32) {
        if rd >= 8 || base >= 8 || idx >= 8 {
            self.b(rex(false, rd, idx, base));
        }
        self.b(0x0F);
        self.b(0xB6);
        // modrm reg=rd, rm=100 (SIB follows); SIB scale=0, index=idx, base=base.
        self.b(modrm(2, rd & 7, 0b100));
        self.b((idx & 7) << 3 | (base & 7));
        self.u32(disp as u32);
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
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xEF);
        self.b(modrm(3, dst & 7, src & 7));
    }

    /// PCLMULQDQ dst, src, imm — carry-less (polynomial) multiply of a selectable
    /// 64-bit half of dst and src -> 128-bit result (66 0F 3A 44 /r ib). imm bits
    /// 1:0 select the src dst half, bits 5:4 the src src half (0=low 64, 1=high 64).
    pub fn pclmulq(&mut self, dst: u8, src: u8, imm: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x3A);
        self.b(0x44);
        self.b(modrm(3, dst & 7, src & 7));
        self.b(imm);
    }

    /// Set xmm register to all-ones (128 bits): pxor x,x + pcmpeqd x,x (66 0F 76 /r).
    pub fn movdqu_ones(&mut self, xmm: u8) {
        // pxor xmm, xmm
        self.b(0x66);
        if xmm >= 8 {
            self.b(rex(false, xmm, 0, xmm));
        }
        self.b(0x0F);
        self.b(0xEF);
        self.b(modrm(3, xmm & 7, xmm & 7));
        // pcmpeqd xmm, xmm
        self.b(0x66);
        if xmm >= 8 {
            self.b(rex(false, xmm, 0, xmm));
        }
        self.b(0x0F);
        self.b(0x76);
        self.b(modrm(3, xmm & 7, xmm & 7));
    }

    /// pand xmm, xmm : 128-bit bitwise AND (66 0F DB /r). ModRM: reg=DST, rm=SRC
    pub fn pand(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xDB);
        self.b(modrm(3, dst & 7, src & 7));
    }

    /// por xmm, xmm : 128-bit bitwise OR (66 0F EB). reg=DST, rm=SRC
    pub fn por(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xEB);
        self.b(modrm(3, dst & 7, src & 7));
    }

    /// pandn xmm, xmm : dst = ~dst & src (66 0F DF /r). Confirmed encoding:
        /// `pandn xmm1, xmm0` = 66 0F DF C8 (modrm reg=xmm1=DST, rm=xmm0=SRC).
        pub fn pandn(&mut self, dst: u8, src: u8) {
            self.b(0x66);
            if dst >= 8 || src >= 8 {
                self.b(rex(false, dst, 0, src));
            }
            self.b(0x0F);
            self.b(0xDF);
            self.b(modrm(3, dst & 7, src & 7));
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
    // PSUBD xmm, xmm/m128 (4x32-bit subtract): 66 0F FA /r.
    #[allow(dead_code)]
    pub fn psubd(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xFA);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // PADDB xmm, xmm/m128 : 16x8-bit lane add
    pub fn paddb(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xFC);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // PSUBB xmm, xmm/m128 : 16x8-bit lane sub
    pub fn psubb(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xF8);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // PADDW xmm, xmm/m128 : 8x16-bit lane add (66 0F FD)
    pub fn paddw(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xFD);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // PSUBW xmm, xmm/m128 : 8x16-bit lane sub (66 0F F9)
    pub fn psubw(&mut self, dst: u8, src: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xF9);
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
    /// MULSS (SSE): `F3 0F 59 /r` — dst *= src (scalar single). reg=DST, rm=SR
    pub fn mulss(&mut self, dst: u8, src: u8) {
        self.b(0xF3);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x59);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// DIVSS (SSE): `F3 0F 5E /r` — dst /= src (scalar single). reg=DST, rm=SR
    pub fn divss(&mut self, dst: u8, src: u8) {
        self.b(0xF3);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x5E);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// ADDSS (SSE): `F3 0F 58 /r` — dst += src (scalar single). reg=DST, rm=SR
    pub fn addss(&mut self, dst: u8, src: u8) {
        self.b(0xF3);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x58);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// SUBSS (SSE): `F3 0F 5C /r` — dst -= src (scalar single). reg=DST, rm=SR
    pub fn subss(&mut self, dst: u8, src: u8) {
        self.b(0xF3);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x5C);
        self.b(modrm(3, dst & 7, src & 7));
    }
    /// MOVAPS (SSE): `0F 28 /r` — dst = src (128-bit aligned move).
    pub fn subsd(&mut self, dst: u8, src: u8) {
        self.sd(0x5C, dst, src);
    }
    /// DIVSD (SSE2): `F2 0F 5E /r` — dst /= src (scalar double).
    pub fn divsd(&mut self, dst: u8, src: u8) {
        self.sd(0x5E, dst, src);
    }
    /// SQRTSD (SSE2): `F2 0F 51 /r` — dst = sqrt(src) (scalar double).
    pub fn sqrtsd(&mut self, dst: u8, src: u8) {
        self.sd(0x51, dst, src);
    }
    /// ROUNDSD (SSE4.1): `66 0F 3A 0B /r ib` — dst = round(src) with mode imm[1:0]:
    /// 00=nearest-even, 01=floor(-inf), 10=ceil(+inf), 11=trunc-toward-zero.
    /// Matches AArch64 `frintm` (toward -inf, mode 1) / `frintp` (2) / `frintz` (3).
    pub fn roundsd(&mut self, dst: u8, src: u8, mode: u8) {
        self.b(0x66);
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0x3A);
        self.b(0x0B);
        self.b(modrm(3, dst & 7, src & 7));
        self.b(mode & 0x0f);
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
    /// div r64 — unsigned divide RDX:RAX by r64; quotient RAX, remainder RDX. 48 F7 /6.
    pub fn div_r64(&mut self, divr: u8) {
        self.b(0x48);
        self.b(0xF7);
        self.b(modrm(3, 6, divr & 7)); // /6 (group-3 DIV; /0 would be TEST)
    }
    /// div r32 — unsigned divide EDX:EAX by r32; quotient EAX, remainder EDX. F7 /6.
    pub fn div_r32(&mut self, divr: u8) {
        self.b(0xF7);
        self.b(modrm(3, 6, divr & 7));
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
    /// mul r/m64 — UNSIGNED one-operand multiply: RDX:RAX = RAX * rm. 49? 48 F7 /4.
    /// (umulh uses this; result high half lands in RDX.)
    pub fn mul_high_r64(&mut self, rm: u8) {
        if rm >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0xF7);
        self.b(modrm(3, 4, rm & 7));
    }
    /// imul r/m64 — SIGNED one-operand multiply: RDX:RAX = RAX * rm. 48 F7 /5.
    /// (smulh uses this; high half in RDX.)
    pub fn imul_high_r64(&mut self, rm: u8) {
        if rm >= 8 {
            self.b(0x49);
        } else {
            self.b(0x48);
        }
        self.b(0xF7);
        self.b(modrm(3, 5, rm & 7));
    }
    /// movsxd r64, r/m32 — sign-extend a 32-bit operand into r64. 48 63 /r.
        pub fn movsxd_r64_r32(&mut self, rd: u8, rs: u8) {
            self.b(0x48);
            self.b(0x63);
            self.b(modrm(3, rd & 7, rs & 7));
        }
        /// movq xmm, r64  (66 48 0F 6E /r) — move a GPR's bits into the low 8B of an XMM.
        pub fn movq_xmm_r64(&mut self, xmm: u8, r64: u8) {
            self.b(0x66);
            self.b(rex(true, xmm, 0, r64)); // W=1 (64-bit), r=xmm ext, b=r64 exts
            self.b(0x0F);
            self.b(0x6E);
            self.b(modrm(3, xmm & 7, r64 & 7));
        }
        /// movq r64, xmm  (66 48 0F 7E /r) — move an XMM's low 8B bits into a GPR.
        pub fn movq_r64_xmm(&mut self, r64: u8, xmm: u8) {
            self.b(0x66);
            self.b(rex(true, r64, 0, xmm)); // W=1, modrm.reg=r64 (bit3), rm=xmm (bit3)
            self.b(0x0F);
            self.b(0x7E);
            self.b(modrm(3, r64 & 7, xmm & 7));
        }
        /// comisd xmm_a, xmm_b  (66 0F 2F /r) — signed compare; sets CF/ZF (CF=1 if a<b).
        pub fn comisd(&mut self, a: u8, b: u8) {
            self.b(0x66);
            self.b(rex(false, a, 0, b)); // r=a (high xmm), b=b (high xmm)
            self.b(0x0F);
            self.b(0x2F);
            self.b(modrm(3, a & 7, b & 7));
        }
        /// comiss xmm_a, xmm_b  (0F 2F /r) — single-precision order compare.
    pub fn comiss(&mut self, a: u8, b: u8) {
        self.b(rex(false, a, 0, b));
        self.b(0x0F);
        self.b(0x2F);
        self.b(modrm(3, a & 7, b & 7));
    }
    // SETcc r/m8: `0F 9?cc /r` — store 1 (cc true) or 0 into the 8-bit reg.
    // cc (x86): 4=sete, 7=seta, 3=setae (0x90+cc). reg field of ModRM is /0.
    pub fn setcc_rm8(&mut self, cc: u8, reg: u8) {
        if reg >= 8 {
            self.b(rex(false, 0, 0, reg));
        }
        self.b(0x0F);
        self.b(0x90 + (cc & 0xf));
        self.b(modrm(3, 0, reg & 7));
    }
    // MOVZX r32, r8: `0F B6 /r` — zero-extend a byte register into a 32-bit reg.
    pub fn movzx_r32_r8(&mut self, dst: u8, src: u8) {
        if dst >= 8 || src >= 8 {
            self.b(rex(false, dst, 0, src));
        }
        self.b(0x0F);
        self.b(0xB6);
        self.b(modrm(3, dst & 7, src & 7));
    }
    // MAXSD xmm, xmm : F2 0F 5F /r (max; a = max(a,b) on doubles)
    pub fn maxsd(&mut self, a: u8, b: u8) {
        self.b(0xF2);
        self.b(rex(false, a, 0, b));
        self.b(0x0F);
        self.b(0x5F);
        self.b(modrm(3, a & 7, b & 7));
    }
    // MINSD xmm, xmm : F2 0F 5D /r (min on doubles)
    pub fn minsd(&mut self, a: u8, b: u8) {
        self.b(0xF2);
        self.b(rex(false, a, 0, b));
        self.b(0x0F);
        self.b(0x5D);
        self.b(modrm(3, a & 7, b & 7));
    }
    // MAXSS xmm, xmm : F3 0F 5F /r (max on singles)
    pub fn maxss(&mut self, a: u8, b: u8) {
        self.b(0xF3);
        self.b(rex(false, a, 0, b));
        self.b(0x0F);
        self.b(0x5F);
        self.b(modrm(3, a & 7, b & 7));
    }
    // MINSS xmm, xmm : F3 0F 5D /r (min on singles)
    pub fn minss(&mut self, a: u8, b: u8) {
        self.b(0xF3);
        self.b(rex(false, a, 0, b));
        self.b(0x0F);
        self.b(0x5D);
        self.b(modrm(3, a & 7, b & 7));
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
        // movd r/m32, xmm: 66 0F 7E /r with reg-field=xmm(src), rm-field=GPR(dst).
        self.b(modrm(3, xmm & 7, rd & 7));
    }
    /// movd xmm, r32 (66 0F 6E /r) — copy low 32 bits of a GPR to xmm
    pub fn movd_xmm_r32(&mut self, xmm: u8, rs: u8) {
        self.b(0x66);
        self.b(0x0F);
        self.b(0x6E);
        // movd xmm, r/m32: 66 0F 6E /r with reg.field=xmm(dst), rm-field=GPR(src).
        self.b(modrm(3, xmm & 7, rs & 7));
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
    /// add r32 <- r32 (32-bit: NO REX.W). Sets N/Z/C/V from the 32-bit result
    /// (SF=bit31, OF=32-bit signed overflow) exactly as AArch64 `adds w…` needs,
    /// and clears the upper 32 bits of rd (x86-64 semantics for 32-bit writes).
    pub fn add_rr32(&mut self, rd: u8, rs: u8) {
        self.binop32(0x01, rd, rs);
    }
    /// sub r32 <- r32 (32-bit, no REX.W) — `subs w…` / `cmp w…` flag semantics.
    pub fn sub_rr32(&mut self, rd: u8, rs: u8) {
        self.binop32(0x29, rd, rs);
    }

    fn binop32(&mut self, op: u8, rd: u8, rs: u8) {
        if rd >= 8 || rs >= 8 {
            self.b(rex(false, rs, 0, rd)); // REX with W=0 for high regs
        }
        self.b(op);
        self.b(modrm(3, rs & 7, rd & 7));
    }
    /// adc r64 <- r64 + CF  (opcode 11: reg=src, rm=dest)
    pub fn adc_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x11, rd, rs);
    }
    /// sbb r64 <- r64 - CF  (opcode 19)
    pub fn sbb_rr64(&mut self, rd: u8, rs: u8) {
        self.binop(0x19, rd, rs);
    }
    /// cmc (F5): complement the carry flag
    pub fn cmc(&mut self) {
        self.b(0xF5);
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
    /// neg r64 (0x48/49 F7 /3) — sets flags (CF/OF), fine before load_nzcv.
    /// rd>=8 needs REX.B (0x49, not 0x48): without it `neg r10` operates on
    /// RDX (rm = rd&7), silently not negating the intended register.
    pub fn neg_r64(&mut self, rd: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xF7);
        self.b(modrm(3, 3, rd & 7));
    }
    /// not r64 (0x48/49 F7 /2) — sets flags.
    pub fn not_r64(&mut self, rd: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xF7);
        self.b(modrm(3, 2, rd & 7));
    }
    /// and r64, imm32
    pub fn and_ri64(&mut self, rd: u8, imm: u32) {
        self.ari_imm(4, rd, imm);
    }
    /// Zero-extend `rd`'s low 32 bits into the full 64-bit register — the
    /// canonical x86 idiom `mov rD, rD` (32-bit, NO REX.W) which clears the
    /// upper 32 bits.
    ///
    /// DO NOT use `and r64, 0xffffffff` for this: REX.W-and-imm32 sign-extends
    /// the immediate, so `and rax, 0xffffffff` is a NO-OP, not a mask. That
    /// wrong idiom silently leaked uninitialized high bits into every W-register
    /// write (a latent bug found via glibc-CRT register corruption).
    pub fn zero_ext_r32(&mut self, rd: u8) {
        let rex = 0x40
            | if rd >= 8 {
                // REX.R (reg) + REX.B (rm) for high registers.
                0x04 | 0x01
            } else {
                0
            };
        if rex != 0x40 {
            self.b(rex);
        }
        self.b(0x89);
        self.b(modrm(3, rd & 7, rd & 7)); // mov r32, r32 (reg=rm=rd)
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

    /// add r32, imm32 (81 /0, NO REX.W) — 32-bit flag semantics + upper-clear.
    pub fn add_ri32(&mut self, rd: u8, imm: u32) {
        self.ari_imm32(0, rd, imm);
    }
    /// sub r32, imm32 (81 /5, no REX.W).
    pub fn sub_ri32(&mut self, rd: u8, imm: u32) {
        self.ari_imm32(5, rd, imm);
    }
    fn ari_imm32(&mut self, op: u8, rd: u8, imm: u32) {
        if rd >= 8 {
            self.b(0x41);
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

    /// shl r64, imm8  (encoding 48/49 C1 /4 ib). rd>=8 needs REX.B (0x49).
    pub fn shl_ri8(&mut self, rd: u8, imm: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xC1);
        self.b(modrm(3, 4, rd & 7));
        self.b(imm);
    }
    /// shr r64, imm8  (encoding 48/49 C1 /5 ib)
    pub fn shr_ri8(&mut self, rd: u8, imm: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xC1);
        self.b(modrm(3, 5, rd & 7));
        self.b(imm);
    }
    /// sar r64, imm8  (encoding 48/49 C1 /7 ib) — arithmetic (sign) shift right
    pub fn sar_ri8(&mut self, rd: u8, imm: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xC1);
        self.b(modrm(3, 7, rd & 7));
        self.b(imm);
    }
    /// sar r32, imm8  (C1 /7 ib, no REX.W) — 32-bit arithmetic shift, sign
    /// taken from bit31. Used for `asr Wd` where the guest operand is a
    /// zero-extended 32-bit value: a 64-bit `sar` would read bit63 (=0) as the
    /// sign and turn `asr w,#1` of 0x80000000 into 0x40000000, not 0xc0000000.
    pub fn sar32_ri8(&mut self, rd: u8, imm: u8) {
        if rd & 8 != 0 {
            self.b(0x41);
        }
        self.b(0xC1);
        self.b(modrm(3, 7, rd & 7));
        self.b(imm);
    }

    /// ror r64, imm8  (48/49 C1 /1 ib) — rotate right
    pub fn ror_ri8(&mut self, rd: u8, imm: u8) {
        self.b(if rd >= 8 { 0x49 } else { 0x48 });
        self.b(0xC1);
        self.b(modrm(3, 1, rd & 7));
        self.b(imm);
    }
    /// ROR r32, imm8 : C1 /1 ib (32-bit rotate right by imm). No REX.W.
    pub fn ror32_ri8(&mut self, rd: u8, imm: u8) {
        if rd & 8 != 0 {
            self.b(0x41);
        }
        self.b(0xC1);
        self.b(modrm(3, 1, rd & 7));
        self.b(imm);
    }
    /// ROL r16, imm8 : 66 C1 /0 ib (16-bit rotate LEFT by imm). The 0x66 prefix
    /// makes this operate on the low 16 bits only, so a pre-existing high half
    /// of the register is untouched. Used for rev16, which needs a 16-bit
    /// byte-swap = rotate-left-by-8 within each halfword.
    pub fn rol16_ri8(&mut self, rd: u8, imm: u8) {
        self.b(0x66); // 16-bit operand-size prefix
        if rd & 8 != 0 {
            self.b(0x41); // REX.B for high register
        }
        self.b(0xC1);
        self.b(modrm(3, 0, rd & 7));
        self.b(imm);
    }
    /// bswap r32  (0F C8+rd): reverse byte order of the low 32 bits.
    pub fn bswap_r32(&mut self, rd: u8) {
        self.b(0x0F);
        self.b(0xC8 | (rd & 7));
    }
    /// bswap r64  (48 0F C8+rd): reverse byte order of the whole 64 bits.
    pub fn bswap_r64(&mut self, rd: u8) {
        self.b(0x48);
        self.b(0x0F);
        self.b(0xC8 | (rd & 7));
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
    /// jne short (75 rel8) — used to skip a fixed-length (e.g. 1-byte `ret`)
    /// block when a condition holds. `disp` is the signed rel8 (jump target =
    /// address after this 2-byte instruction + disp).
    pub fn jne_rel8(&mut self, disp: u8) {
        self.b(0x75);
        self.b(disp);
    }
    /// jz short (74 rel8) — same shape as `jne_rel8`.
    pub fn jz_rel8(&mut self, disp: u8) {
        self.b(0x74);
        self.b(disp);
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
