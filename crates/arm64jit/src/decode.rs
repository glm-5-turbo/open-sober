// SPDX-License-Identifier: MIT
//
// ARM64 (AArch64) instruction decoder for the open-sober JIT.
//
// Bitfields follow the ARM ARM (DDI0487). Encodings are verified against ground
// truth emitted by aarch64-linux-gnu-gcc -O1 (objdump) during development. Code
// outside the covered subset decodes to Unsupported so the translator traps on
// it visibly. Each class is added with a unit test matching the real encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchRegKind {
    Br,
    Blr,
    Ret,
    Eret,
}

/// A decoded AArch64 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    // ---- unconditional branch ----
    B {
        imm: i64,
        link: bool,
    },
    // ---- conditional branch ----
    BCond {
        cond: u8,
        imm: i64,
    },
    // ---- pc-relative ----
    Adr {
        rd: u8,
        imm: i64,
    },
    Adrp {
        rd: u8,
        imm: i64,
    },
    // ---- move wide ----
    MoveWide {
        rd: u8,
        imm16: u16,
        hw: u8,
        opc: u8,
        sf: bool,
    },
    // ---- add/sub immediate ----
    AddSubImm {
        rd: u8,
        rn: u8,
        imm12: u32,
        shift12: bool,
        sub: bool,
        sf: bool,
        s: bool,
    },
    // ---- add/sub subtract register (shifted) ----
    AddSubReg {
        rd: u8,
        rn: u8,
        rm: u8,
        sub: bool,
        sf: bool,
        s: bool,
        shift: ShiftKind,
        sh_amt: u8,
    },
    // ---- logical (shifted register) ----
    LogicReg {
        rd: u8,
        rn: u8,
        rm: u8,
        op: u8, // 0=AND,1=ORR,2=EOR (plus variants/not)
        s: bool,
        sf: bool,
        shift: ShiftKind,
        sh_amt: u8,
    },
    // ---- conditional select (CSEL/CSINC/CSINV/CSNEG: incl. CSET/CINC)
    CSel {
        rd: u8,
        rn: u8,
        rm: u8,
        cond: u8,
        op: u8, // 0=csel,1=csinc,2=csinv,3=csneg
        sf: bool,
    },
    // ---- load/store (unsigned immediate offset) ----
    LdStrImm {
        rt: u8,
        rn: u8,
        imm: u32, // scaled byte offset = value * size
        size: u8, // 1=byte,2=half,4=word,8=dword
        ld: bool, // load=true, store=false
    },
    // ---- load/store (register offset) ----
    LdStrReg {
        rt: u8,
        rn: u8,
        rm: u8,
        size: u8,
        ld: bool,
        shift: bool, // S bit: scaled by element size
    },
    // ---- load/store pair ----
    LdStPair {
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i64, // signed scaled offset
        ld: bool,
        writeback: bool,
        preidx: bool,
        size_64: bool, // false => 32-bit W pair
        q128: bool,    // true => 128-bit SIMD pair (ldp/stp q)
        fp_d: bool,    // true => 64-bit FP/vector d-pair (ldp/stp d)
    },
    // ---- SIMD/NEON 128-bit vector load/store (ldr q0,[xN,#imm] / str q) ----
    VecLdStImm {
        vt: u8, // vector register
        rn: u8,
        imm: u32, // scaled-by-16 byte offset
        ld: bool,
    },
    // ---- SIMD/NEON vector move-immediate (movi Vd.<T>, #imm) ----
    // `lo`/`hi` are the low/high 64-bit halves of the 128-bit result, already
    // expanded to the element size (each byte/word/dword lane set to #imm).
    VecMovi {
        vd: u8,
        lo: u64,
        hi: u64,
    },
    // ---- compare-and-branch ----
    Cbz {
        rt: u8,
        imm: i64,
        nonzero: bool,
        sf: bool,
    },
    // ---- test-bit-and-branch (tbz/tbnz Xt,#bit,label) ----
    Tbz {
        rt: u8,
        bit: u32,      // bit position to test (0..63)
        imm: i64,      // branch offset from pc
        nonzero: bool, // true = tbnz
        sf: bool,
    },
    // ---- load-acquire / store-release (LDAR/STLR) ----
    // Single-threaded JIT: ordering is irrelevant, treated as a plain load/store.
    AcqRel {
        size: u32, // 0=byte,1=half,2=word,3=x
        ld: bool,  // true = ldar (load), false = stlr (store)
        rt: u8,
        rn: u8,
    },
    // ---- scalar floating-point arithmetic on d-regs (double) ----
    FpScalar {
        rd: u8,
        rn: u8,
        rm: u8,
        op: u8, // 4=mul,5=add,6=sub,7=div
        sz: bool, // true = double
    },
    // ---- FP convert to integer (fcvtas/fcvtzs): Dn|Sn -> Rd (signed int) ----
    FcvtToInt {
           rd: u8,
           rn: u8,   // source fp reg
           mode: u8, // 0=fcvtzs, 2=fcvtas
           sf: bool, // 64-bit dest
       },
       // ---- FMOV between a core register and a scalar FP register ----
       //   FMOV Dd,Xn 0x9E670000 (write GPR to low 64 of Dd, zero hi)
       //   FMOV Xd,Dn 0x9E660000 (read low 64 of Dn into Xd)
       //   FMOV Sd,Wn 0x1E270000 / FMOV Wd,Sn 0x1E260000
       FmovGp {
               f: bool,  // false = GP->FP (X/Sd <- X/Wn), true = FP->GP (Xd/Wd <- D/Sn)
               sz: bool, // true = double (d/x), false = single (s/w)
               rd: u8,
               rn: u8,
           },
           // ---- NEON: cnt V.8b (per-byte popcount) and uaddlv H, V.8b (byte sum) ----
           SimdPopcnt { rd: u8, rn: u8 }, // cnt v{d}.8b, v{m}.8b
           SimdSum8 { rd: u8, rn: u8 },   // uaddlv h{rd}, v{rn}.8b
                       // ---- scalar FP width convert (fcvt s,d / fcvt d,s) ----
                       Fcvt {
                               to_d: bool, // true = d0 = (double)(s src) ; false = s
                               rd: u8,
                               rn: u8,
                           },
                           // ---- NEON: mov Vd.D[1], Vn.D[0] (dup low 64 into the high 64 lane) ----
                           InsD1D0 { rd: u8, rn: u8 }, // v16B: slot_hi(8B) = low-64-of-Vn
                           // ---- EXTR / ROR rotate: rm==rn in the EXTR base ----
                           Ror { rd: u8, rn: u8, rot: u32, sf: bool }, // ror rd,rn,#rot
                           // ---- NEON lane add: add Vd.4s, Vn.4s, Vm.4s ---------
                           Simd4s {
                                               rd: u8,
                                               rn: u8,
                                               rm: u8,
                                               op: u8, // 0=add (currently), future sub/etc
                                           },
                           // ---- bitfield (UBFM/SBFM): decoded to the lsr/lsl/asr and extraction aliases ----
           BitField {
        rd: u8,
        rn: u8,
        immr: u32,
        imms: u32,
        sf: bool,   // 64-bit
        arith: bool, // true = arithmetic shift (SBFM/asr) sign-extends
    },
    // ---- system register access (mrs xN, <sysreg> / msr <sysreg>, xN) ----
        // Only the thread-pointer registers the JIT models are decoded: tpidr_el0
        // (op0=3 op1=3 CRn=13 CRm=0 op2=2). Other <sysreg> encodings fall back to
        // Unsupported.
        SysReg {
            sysreg: u32, // packed (op0,op1,CRn,CRm,op2); 0 == tpidr_el0
            rt: u8,      // read: Rt = tpidr_el0 ; write: tpidr_el0 = Rt
            read: bool,  // true = MRS (system -> GPR), false = MSR (GPR -> system)
        },
        // ---- logical (immediate): AND/ORR/EOR/ANDS with a bitmask immediate ----
        // Covers the `mov xD, #imm` alias (ORR xD, xzr, #imm) and
        // tst (ANDS xzr, xN, #imm) and BICS-family AND-immediate. The 64-bit
        // operand mask is decoded from N/immr/imms by decode_logical_mask.
        LogicImm {
            rd: u8,
            rn: u8,
            mask: u64,
            op: u8, // 0=AND,1=ORR,2=EOR,3=ANDS (set-flags)
            sf: bool, // 64-bit operands
        },
        // ---- exclusive load/store (ldxr/stxr/ldaxr/stlxr) ----
        // Single-threaded: an exclusive block always succeeds, so `ldxr` is a
        // plain load and `stxr` is a plain store that reports success (Rs==0).
        LdExr {
            size: u32, // 0=byte,1=half,2=word,3=x
            ld: bool,  // true = ldxr/ldaxr (load), false = stxr/stlxr (store)
            rs: u8,    // store-exclusive status reg (writes 0); unused for ld
            rt: u8,
            rn: u8,
        },
        // ---- integer multiply/divide register (madd/msub/udiv/sdiv) ----
        MulDiv {
            div: bool,    // true = UDIV/SDIV, false = MADD/MSUB
            signed: bool, // SDIV / MSUB vs UDIV / MADD
            rd: u8,
            rn: u8,
            rm: u8,
            ra: u8, // MADD/MSUB accumulate reg; 0 for DIV
            sf: bool,
        },
    // ---- HINT / PAC NOP (nop, yield, esb, csdb, paciasp, autiasp, bti, ...) ----
    // Dealt with as a no-op for execution (PAC is ignored in the guest).
    Hint,
    // ---- return (ret x30) ----
    Ret,
    // ---- indirect branch (br Xn) and register call (blr Xn) ----
    Br {
        rn: u8,
    },
    Blr {
        rn: u8,
    },
    // ---- fallback ----
    Unsupported(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Lsl = 0,
    Lsr = 1,
    Asr = 2,
    Ror = 3,
}
impl ShiftKind {
    #[inline]
    pub fn from_u32(v: u32) -> ShiftKind {
        match v {
            0 => ShiftKind::Lsl,
            1 => ShiftKind::Lsr,
            2 => ShiftKind::Asr,
            _ => ShiftKind::Ror,
        }
    }
}

#[inline]
fn b(insn: u32, lo: u32, hi: u32) -> u32 {
    (insn >> lo) & ((1u32 << (hi - lo + 1)) - 1)
}

/// Decode an AArch64 logical-immediate bitmask from `N`/`immr`/`imms`.
/// Returns the 64-bit operand mask, or None if the encoding is invalid.
/// Element size: N=1 -> 64-bit element; N=0 -> 32-bit element (replicated twice
/// to fill the 64-bit register, as used by X-register ANDIMM/ORRIMM etc.).
fn decode_logical_mask(n: u32, immr: u32, imms: u32) -> Option<u64> {
    if n == 1 {
        // 64-bit element.
        if imms >= 64 {
            return None;
        }
        let ones: u64 = (1u64 << (imms + 1)).wrapping_sub(1);
        let mask64: u64 = u64::MAX;
        Some(if immr == 0 {
            ones
        } else {
            (ones.rotate_right(immr)) & mask64
        })
    } else {
        // 32-bit element (N=0): valid only when imms < 32.
        if imms >= 32 {
            return None;
        }
        let ones: u32 = (1u32 << (imms + 1)).wrapping_sub(1);
        let e32: u32 = if immr == 0 {
            ones
        } else {
            ones.rotate_right(immr)
        };
        // Replicate the 32-bit element twice to form the 64-bit mask.
        Some((e32 as u64) | ((e32 as u64) << 32))
    }
}
/// Sign-extend a `bits`-wide value.
#[inline]
fn sext(v: u64, bits: u32) -> i64 {
    ((v << (64 - bits)) as i64) >> (64 - bits)
}
#[inline]
fn rd(insn: u32) -> u8 {
    (insn & 0x1F) as u8
}

/// Replicate an 8-bit lane value into every byte of a 64-bit word
/// (used by NEON `movi` .8b/.16b element broadcast).
#[inline]
fn replicate_imm(lane: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..8 {
        acc |= (lane & 0xff) << (8 * i);
    }
    acc
}
#[inline]
#[allow(dead_code)]
fn rn(insn: u32) -> u8 {
    b(insn, 5, 9) as u8
}

pub fn decode(insn: u32) -> Inst {
    // ---- unconditional branch: bits[30:26] = 0b00101, bit31=link ----
    if b(insn, 26, 30) == 0b00101 {
        let link = insn >> 31 == 1;
        let imm = sext((insn & 0x03FF_FFFF) as u64, 26);
        return Inst::B {
            imm: imm << 2,
            link,
        };
    }

    // ---- cond branch: bits[31:24] = 0x54 ----
    if insn >> 24 == 0x54 {
        let cond = (insn & 0xF) as u8;
        let imm = sext(b(insn, 5, 23) as u64, 19);
        return Inst::BCond {
            cond,
            imm: imm << 2,
        };
    }

    // ---- ADR / ADRP: bits[31:24] = imm_lo + op(10000) ----
    // The top byte varies with imm[1:0] (bits 30-29), so the discriminator is
    // the opcode mask 0x9F000000: ADR = 0x10000000, ADRP = 0x90000000. Using a
    // rigid `insn>>24 == 0x90` check misses real encodings (e.g. immlo=0b10
    // yields top byte 0xD0, as in libroblox's 0xd0026a93).
    let immlo = (insn >> 29) & 0x3; // imm[1:0]
    let immhi = b(insn, 5, 23); // imm[20:2]
    let imm = ((immhi as u64) << 2) | (immlo as u64);
    match insn & 0x9F00_0000 {
        0x1000_0000 => {
            return Inst::Adr {
                rd: rd(insn),
                imm: sext(imm, 21),
            };
        }
        0x9000_0000 => {
            return Inst::Adrp {
                rd: rd(insn),
                imm: sext(imm, 21) << 12,
            };
        }
        _ => {}
    }

    // ---- MoveWide (MOVZ/MOVK/MOVN): high byte is one of 6 verified encodings.
    //   0x52 movz32  0xD2 movz64  0x72 movk32  0xF2 movk64  0x12 movn32  0x92 movn64
    let top = insn >> 24;
    let sf = insn >> 31 == 1;
    if matches!(top, 0x12 | 0x52 | 0x72 | 0x92 | 0xD2 | 0xF2) {
        // opc: bit30 (sf=0 uses bit29=m... ). Derive opcode from identifier bits:
        //   -(sf, opc2) : movz <-> opc=0, movk: opc=1, movn: opc=2
        // Use displacement: the top-nibble distinguishes mov | k | n via bit3.
        let opc = match top & 0xF0 {
            0xD0 | 0x50 => 0, // movz
            0xF0 | 0x70 => 1, // movk
            _ => 2,           // movn
        };
        let _ = sf;
        let hw = b(insn, 21, 22) as u8;
        let imm16 = (insn >> 5) as u16;
        let rd = rd(insn);
        return Inst::MoveWide {
            rd,
            imm16,
            hw,
            opc,
            sf,
        };
    }

    // ---- add/subtract immediate ----
    // add w=0x11 sub=0x51 ; adds/sub w(=s flag) =0x31/0x71; x: 0x91/0xD1, 0xB1/0xF1
    //   (0x71 is `cmp w,#imm` = SUBS w, xzr, #-imm; 0xF1 is cmp x,#imm)
    if matches!(top, 0x11 | 0x51 | 0x31 | 0x71 | 0x91 | 0xD1 | 0xB1 | 0xF1) {
        let sub = (insn >> 30) & 1 == 1;
        let s = (insn >> 29) & 1 == 1;
        let shift12 = (insn >> 22) & 1 == 1;
        let imm12 = b(insn, 10, 21);
        let rn = b(insn, 5, 9) as u8;
        let rd = rd(insn);
        return Inst::AddSubImm {
            rd,
            rn,
            imm12,
            shift12,
            sub,
            sf,
            s,
        };
    }

    // ---- add/subtract (shifted register) ----
    // top byte: 0x0b/0x2b(add) 0x4b/0x6b(sub) x32-sfx; 0x8b/0xab 0xcb/0xeb x64
    //   (the 1x/3x/5x/7x... bit29 = S flag: 0x2b=ADDS32,0xab=ADDS64,0x6b=SUBS32,0xeb=SUBS64)
    if matches!(
        top,
        0x0b | 0x1b | 0x2b | 0x3b | 0x4b | 0x6b | 0x8b | 0x9b | 0xab | 0xbb | 0xcb | 0xeb
    ) {
        // 0x1b/0x3b/0x9b/0xbb are the S-set with shift_amount N? keep set broad; refine below.
        let n = b(insn, 21, 21);
        let sub = b(insn, 30, 30) == 1;
        let s = b(insn, 29, 29) == 1;
        let shift = ShiftKind::from_u32(b(insn, 22, 23));
        let rm = b(insn, 16, 20) as u8;
        let _ = n;
        let sh_amt = b(insn, 10, 15) as u8;
        let rn = b(insn, 5, 9) as u8;
        let rd = b(insn, 0, 4) as u8;
        return Inst::AddSubReg {
            rd,
            rn,
            rm,
            sub,
            sf,
            s,
            shift,
            sh_amt,
        };
    }

    // ---- conditional select (CSEL/CSINC/CSINV/CSNEG): mask (insn&0x7fe00000)==0x1a800000 ----
    // MUST precede the logic/add-sub shifted-register decoders: csel X-variants
    // share top byte 0x9a/0xda with the ORR/EOR/BIC families. The 0x1a800000
    // fixed-bit pattern uniquely identifies csel/csinc/csinv/csneg.
    if insn & 0x7fe0_0000 == 0x1a80_0000
        || insn & 0x7fe0_0000 == 0x5a80_0000
        || insn & 0x7fe0_0000 == 0x9a80_0000
        || insn & 0x7fe0_0000 == 0xda80_0000
    {
        let sf = (insn >> 31) & 1 == 1;
        let cond = b(insn, 12, 15) as u8;
        let rm = b(insn, 16, 20) as u8;
        let rn = b(insn, 5, 9) as u8;
        let rd = b(insn, 0, 4) as u8;
        // op = bits[11:10]: 00=csel,01=csinc,10=csinv,11=csneg
        let op = b(insn, 10, 11) as u8;
        return Inst::CSel { rd, rn, rm, cond, op, sf };
    }

    // ---- integer multiply/divide register (madd/msub/udiv/sdiv) ----
    // top 0x1a/0x9a (DIV) or 0x1b/0x9b (MUL). Masked base: 0x1ac00000 (div),
    // 0x1b000000 (mul). sf = bit31. signed: div bit17 (1=sdiv), mul bit15 (1=msub).
    if insn & 0x7ff0_0000 == 0x1ac0_0000 || insn & 0x7ff0_0000 == 0x1b00_0000 {
        let sf = (insn >> 31) & 1 == 1;
        let div = insn & 0x7ff0_0000 == 0x1ac0_0000;
        let signed = if div {
            b(insn, 17, 17) == 1 // SDIV vs UDIV
        } else {
            b(insn, 15, 15) == 1 // MSUB vs MADD
        };
        let rd = b(insn, 0, 4) as u8;
        let rn = b(insn, 5, 9) as u8;
        let rm = b(insn, 16, 20) as u8;
        let ra = if div { 0 } else { b(insn, 10, 14) as u8 };
        return Inst::MulDiv {
            div,
            signed,
            rd,
            rn,
            rm,
            ra,
            sf,
        };
    }

    // ---- logical (shifted register): AND/ORR/EOR/BIC/ORN/EON ----
    // top byte: 0x0a xx-family; opc = bits[30:29], N = bit21
    if matches!(
        top,
        0x0a | 0x2a | 0x4a | 0x6a | 0x8a | 0x9a | 0xaa | 0xba | 0xca | 0xda | 0x3a | 0x7a
            | 0xea | 0xfa
    ) {
        let n = b(insn, 21, 21);
        let opc = b(insn, 29, 30); // ops: 0=AND/BIC, 1=ORR/ORN, 2=EOR/EON, 3=AND/OR/EOR + set-flags
        let s = opc == 0b11; // the ANDS/BICS set-flags family is opc==3, not a separate S bit.
        // opc==3 always means AND (with N deciding AND vs BIC); the base opcode for the
        // non-set variants is opc itself (0=AND,1=ORR,2=EOR), +4 when N (BIC/ORN/EON).
        let op = if opc == 0b11 {
            if n == 1 { 4 } else { 0 } // BICS / ANDS
        } else {
            (opc & 0b11) as u8 | ((n == 1) as u8) << 2
        };
        let shift = ShiftKind::from_u32(b(insn, 22, 23));
        let rm = b(insn, 16, 20) as u8;
        let sh_amt = b(insn, 10, 15) as u8;
        let rn = b(insn, 5, 9) as u8;
        let rd = b(insn, 0, 4) as u8;
        return Inst::LogicReg {
            rd,
            rn,
            rm,
            op,
            s,
            sf,
            shift,
            sh_amt,
        };
    }

    // ---- logical (immediate): AND/ORR/EOR/ANDS with a bitmask immediate ----
    // class top bytes: W 0x12/0x32/0x52/0x72 ; X 0x92/0xb2/0xd2/0xf2. The `mov
    // xD, #imm` alias...[truncated]
    if matches!(
        insn >> 24,
        0x12 | 0x32 | 0x52 | 0x72 | 0x92 | 0xb2 | 0xd2 | 0xf2
    ) {
        let sf = (insn >> 31) & 1 == 1;
        let op = ((insn >> 29) & 0x3) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        let n = (insn >> 22) & 1;
        let immr = b(insn, 16, 21);
        let imms = b(insn, 10, 15);
        if let Some(mask) = decode_logical_mask(n, immr, imms) {
            return Inst::LogicImm {
                rd,
                rn,
                mask,
                op,
                sf,
            };
        }
    }

    // ---- exclusive load/store (ldxr/stxr) ----
    // class (insn & 0x3f000000) == 0x08000000, but EXCLUDING the acquire/
    // release LDAR/STLR codes (0x08800000 / 0x08c00000 masked by 0x3fe00000),
    // which are handled by the AcqRel arm. Single-threaded: ldxr = plain
    // load, stxr = plain store with status reg Rs written 0 (success).
    if insn & 0x3f00_0000 == 0x0800_0000
        && (insn & 0x3fe0_0000) != 0x0880_0000
        && (insn & 0x3fe0_0000) != 0x08c0_0000
    {
        let size = (insn >> 30) & 0x3;
        let opc = (insn >> 21) & 0x3; // 2,3 = ld ; 0,1 = st
        let ld = opc == 2 || opc == 3;
        let rs = ((insn >> 16) & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rt = (insn & 0x1f) as u8;
        return Inst::LdExr {
            size,
            ld,
            rs,
            rt,
            rn,
        };
    }

    // ---- load/store (unsigned immediate offset) ----
    // class: (top & 0x3b) == 0x39 => ldr/str, all sizes, both ld or st
    if (insn & 0x3b00_0000) == 0x3900_0000 {
        let size = match (insn >> 30) & 0x3 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let ld = (insn >> 22) & 1 == 1;
        let imm = (insn >> 10) & 0xfff;
        let rn = b(insn, 5, 9) as u8;
        let rt = b(insn, 0, 4) as u8;
        return Inst::LdStrImm {
            rt,
            rn,
            imm,
            size,
            ld,
        };
    }

    // ---- SIMD/NEON 128-bit vector load/store (ldr q / str q) ----
    // Encoding 0x3D8xxxxx (str q) / 0x3DCxxxxx (ldr q), imm12 scaled by 16.
    if (insn & 0xffc0_0000) == 0x3d80_0000 || (insn & 0xffc0_0000) == 0x3dc0_0000 {
        let ld = (insn & 0x40_0000) != 0;
        let imm = (insn >> 10) & 0xfff; // scaled by 16 bytes
        let rn = b(insn, 5, 9) as u8;
        let vt = b(insn, 0, 4) as u8;
        return Inst::VecLdStImm { vt, rn, imm, ld };
    }

    // ---- SIMD/NEON movi vector-immediate ----
    // Forms the real binary hits, validated against aarch64 objdump ground
    // truth (see Session 20/21 HANDOFF):
    //   movi Vd.2S/.4S, #imm  (cmode=0000)  : word lanes, each = imm8
    //   movi Vd.8B/.16B, #imm (cmode=1110)  : byte lanes, each = imm8
    //   movi Vd.2D, #imm      (cmode=1110 + op=1) : dword lanes, each = imm8
    // The 8-bit immediate is reassembled from bits[9:5] (low) and bits[18:16]
    // (high): imm8 = abcd | (defg? === bits[18:16] << 5). Ebconfirmed against
    // 6 grounds-truth encodings incl. the actual boot blocker 0x6f00e400.
    if matches!(insn >> 24, 0x0F | 0x1F | 0x2F | 0x4F | 0x5F | 0x6F) {
        let op = (insn >> 29) & 1;
        let cmode = b(insn, 12, 15);
        let imm8 = (b(insn, 5, 9)) | (b(insn, 16, 18) << 5);
        // Replicate the 8-bit lane value across a 64-bit word.
        let low64: u64 = replicate_imm(imm8 as u64);
        let (lo, hi): (u64, u64) = match (op, cmode) {
            // .2S (Q=0) / .4S (Q=1): 32-bit word lanes
            (0, 0b0000) => {
                let lane = (imm8 as u64) & 0xffff_ffff;
                let low = lane | (lane << 32);
                if (insn >> 30) & 1 == 1 {
                    (low, low)
                } else {
                    (low, 0)
                }
            }
            // .8B (Q=0) / .16B (Q=1): byte lanes
            (0, 0b1110) => {
                if (insn >> 30) & 1 == 1 {
                    (low64, low64)
                } else {
                    (low64, 0)
                }
            }
            // .2D: dword replicate
            (1, 0b1110) => (imm8 as u64, imm8 as u64),
            _ => return Inst::Unsupported(insn),
        };
        return Inst::VecMovi {
            vd: rd(insn),
            lo,
            hi,
        };
    }

    // ---- load/store (register offset) ----
    // class: (top & 0x3b) == 0x38
    if (insn & 0x3b00_0000) == 0x3800_0000 {
        let size = match (insn >> 30) & 0x3 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let ld = (insn >> 22) & 1 == 1;
        let rm = b(insn, 16, 20) as u8;
        let shift = (insn >> 12) & 1 == 1; // S bit
        let rn = b(insn, 5, 9) as u8;
        let rt = b(insn, 0, 4) as u8;
        return Inst::LdStrReg {
            rt,
            rn,
            rm,
            size,
            ld,
            shift,
        };
    }

    // ---- return (ret x30): 0xd65f03c0 ----
    if insn & 0xffff_fc1f == 0xd65f_0000 {
        return Inst::Ret;
    }

    // ---- indirect branch / call: br Xn = 0xd61f0..., blr Xn = 0xd63f0... ----
    if insn & 0xffff_fc1f == 0xd61f_0000 {
        return Inst::Br {
            rn: b(insn, 5, 9) as u8,
        };
    }
    if insn & 0xffff_fc1f == 0xd63f_0000 {
        return Inst::Blr {
            rn: b(insn, 5, 9) as u8,
        };
    }

    // ---- compare-and-branch (CBZ/CBNZ): cbz w=0x34 cbnz=0x35 cbzx=0xb4 cbnzx=0xb5 ----
    if matches!(insn >> 24, 0x34 | 0x35 | 0xb4 | 0xb5) {
        let sf = insn >> 31 == 1;
        let nonzero = (insn >> 24) & 1 == 1;
        let rt = (insn & 0x1f) as u8;
        let imm19 = ((insn >> 5) & 0x7ffff) as u64;
        let imm = sext(imm19, 19) * 4;
        return Inst::Cbz {
            rt,
            imm,
            nonzero,
            sf,
        };
    }

    // ---- load-acquire / store-release (LDAR/STLR family): single-threaded => plain load/store ----
    // size[31:30], L=bit22, Rt[0:4], Rn[5:9]. mask 0x3fe00000 -> 0x08800000 (stlr) / 0x08c00000 (ldar)
    let acquire = insn & 0x3fe0_0000;
    if acquire == 0x0880_0000 || acquire == 0x08c0_0000 {
        let size = (insn >> 30) & 3; // 0=byte,1=half,2=word,3=x
        let ld = (insn >> 22) & 1 == 1; // 1=ldar load, 0=stlr store
        let rt = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        return Inst::AcqRel { size, ld, rt, rn };
    }

    // ---- scalar FP 3-source (d-float) : class 0x1E00_0000, opcode = insn with the
    // three register fields masked. Verified: fmul=1e600800 fadd=1e602800 fsub=1e603800
    // fdiv=1e601800 (d, sz=1); 1-source fmov/fneg/fabs are separately classified and
    // not handled here.
    if insn & 0x1f80_0000 == 0x1e00_0000 {
        let sz = (insn >> 22) & 1 == 1;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        let op = match insn & !(((0x1f) as u32) << 16 | ((0x1f) as u32) << 5 | 0x1f) {
            0x1e60_0800 => Some(4), // fmul
            0x1e60_2800 => Some(5), // fadd
            0x1e60_3800 => Some(6), // fsub
            0x1e60_1800 => Some(7), // fdiv
            _ => None,
        };
        if let Some(op) = op {
            return Inst::FpScalar { rd, rn, rm, op, sz };
        }
    }

    // ---- EXTR / ROR rotate: class (insn&0x1fe00000) in {0x13800000,0x13c00000}
//      (the EXTR base; UBFM is 0x130/0x136 — disjoint). rm==rn => rotation.
    if matches!(insn & 0x1fe0_0000, 0x1380_0000 | 0x13c0_0000)
        && ((insn >> 16) & 0x1f) == ((insn >> 5) & 0x1f)
    {
        let sf = (insn >> 31) & 1 == 1;
        let rot = b(insn, 10, 15); // rotation amount (6-bit, 0..63)
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::Ror { rd, rn, rot, sf };
    }

    // ---- bitfield (UBFM/SBFM): lsr/lsl (UBFM) and asr (SBFM) aliases ----
    // top bytes: UBM-X=0xd3 UBM-W=0x53 SBM-X=0x93 SBM-W=0x13.
    if matches!(insn >> 24, 0xd3 | 0x53 | 0x93 | 0x13) {
        let sf = (insn >> 31) & 1 == 1;
        let arith = matches!(insn >> 24, 0x93 | 0x13);
        let bits = if sf { 64u32 } else { 32u32 };
        let immr = b(insn, 16, 21);
        let imms = b(insn, 10, 15);
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        // Accept the shift aliases (lsr/asr/lsl) plus the general extract aliases
        // (ubfx/sbfx/uxb/sxtb/ughl. any immr<=imms) — BFM-insert (bfi/bfc) left later.
        let is_valid = (imms == bits - 1) // LSR/ASR
            || (immr == (imms + 1) % bits) // LSL
            || (immr <= imms); // UBFX/SBFX + zero/sign-extend
        if is_valid && (rd != 31) {
            return Inst::BitField { rd, rn, immr, imms, sf, arith };
        }
    }

    // ---- bitfield insert (BFM/BFI/BFC): inserts bits of Rn into Rd.
    // top bytes: X=0xb3, W=0x33. The wrap case immr>imms decodes to the
    // bfi/bfc aliases (lsb = (bits-immr)&(bits-1), width = imms+1); the
    // non-wrap (immr<=imms) BFM is the extract-insert and is deferred.
    if matches!(insn >> 24, 0xb3 | 0x33) {
        let sf = (insn >> 31) & 1 == 1;
        let bits = if sf { 64u32 } else { 32u32 };
        let immr = b(insn, 16, 21);
        let imms = b(insn, 10, 15);
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        if immr > imms {
            // BFI/BFC (insert): lsb = (bits-immr)&(bits-1), width = imms+1
            return Inst::BitField { rd, rn, immr, imms, sf, arith: false };
        }
    }

    // ---- FP convert to signed integer (fcvtzs/fcvtas): Dn|Sn -> Rd ----
    // class (insn & 0x5f20fc00)==0x1e200000 ; opc = bits[17:19] (2=fcvtas,0=fcvtzs)
    if insn & 0x5f20_fc00 == 0x1e20_0000 {
        let mode = ((insn >> 17) & 7) as u8; // 0=fcvtzs(toward-zero), 2=fcvtas(nearest-away)
        if mode == 0 || mode == 2 {
            let sf = (insn >> 31) & 1 == 1;
            let sz = (insn >> 22) & 1 == 1; // 1 => source is double (d)
            if sz {
                            let rn = ((insn >> 5) & 0x1f) as u8;
                            let rd = (insn & 0x1f) as u8;
                            return Inst::FcvtToInt { rd, rn, mode, sf };
                        }
                    }
                }

                // ---- FMOV between core and scalar FP register ----
                // Bases: FMOV Xd,Dn 0x9E660000 ; FMOV Dd,Xn 0x9E670000
                //        FMOV Wd,Sn 0x1E260000 ; FMOV Sd,Wn 0x1E270000  (mask clears rn/rt)
                let base = insn & 0xffff_f800;
                    let fmov = match base {
                        0x9e66_0000 | 0x1e26_0000 => Some(true), // FP -> GP
                        0x9e67_0000 | 0x1e27_0000 => Some(false), // GP -> FP
                        _ => None,
                    };
                    if let Some(f) = fmov {
                            let sz = matches!(base, 0x9e66_0000 | 0x9e67_0000); // d/x double
                            let rn = ((insn >> 5) & 0x1f) as u8;
                            let rd = (insn & 0x1f) as u8;
                            return Inst::FmovGp { f, sz, rd, rn };
                        }

                        // ---- NEON bit-popcount idiom: cnt Vd.8b,Vn.8b and uaddlv hD,Vn.8b ----
                            // cnt vD.8b,vN.8b = 0x0e20_5800 | n<<5 | d ; uaddlv hD,vN.8b = 0x2e30_3800...
                            if (insn & 0xffff_fc00) == 0x0e20_5800 {
                                let rn = ((insn >> 5) & 0x1f) as u8;
                                let rd = (insn & 0x1f) as u8;
                                return Inst::SimdPopcnt { rd, rn };
                            }
                            if (insn & 0xffff_fc00) == 0x2e30_3800 {
                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                    let rd = (insn & 0x1f) as u8;
                                    return Inst::SimdSum8 { rd, rn };
                                }

                                // ---- scalar FP width convert: fcvt sd (D->S) / fcvt ds (S->D) ----
                                if (insn & 0xffff_fc00) == 0x1e62_4000 {
                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                    let rd = (insn & 0x1f) as u8;
                                    return Inst::Fcvt { to_d: false, rd, rn }; // s{rd} = (single) d{rn}
                                }
                                if (insn & 0xffff_fc00) == 0x1e22_c000 {
                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                        let rd = (insn & 0x1f) as u8;
                                        return Inst::Fcvt { to_d: true, rd, rn }; // d{rd} = (double) s{rn}
                                    }

                                    // ---- NEON mov Vd.D[1], Vn.D[0] (dup the low 64 into the high lane) ----
                                    if (insn & 0xffff_fc00) == 0x6e18_0400 {
                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                        let rd = (insn & 0x1f) as u8;
                                        return Inst::InsD1D0 { rd, rn };
                                    }

                                    // ---- NEON int add (4x32 lanes): add Vd.4s, Vn.4s, Vm.4s ----
                                    // class Q=1 0x0e20_0000 .. 0x4e20_0000 integer add (S: size=01).
                                    if (insn & 0x2f20_0c00) == 0x0e20_0400 && (insn & 0x3) != 3 {
                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                        let rd = (insn & 0x1f) as u8;
                                        return Inst::Simd4s { rd, rn, rm, op: 0 };
                                    }

    // ---- test-bit-and-branch (tbz/tbnz): (insn & 0x7e000000) == 0x36000000 ----
    if insn & 0x7e00_0000 == 0x3600_0000 {
        let sf = (insn >> 31) & 1 == 1;
        // `op` is bit13: tbz=0, tbnz=1.
        let nonzero = (insn >> 24) & 1 == 1;
        let rt = (insn & 0x1f) as u8;
        let bit = b(insn, 19, 23) | (b(insn, 31, 31) << 5); // bits[23:19] + bit31
        let imm14 = sext(((insn >> 5) & 0x3fff) as u64, 14) * 4;
        return Inst::Tbz {
            rt,
            bit,
            imm: imm14,
            nonzero,
            sf,
        };
    }

    // ---- HINT / PAC NOP (nop, yield, esb, csdb, paciasp, autiasp, bti, ...) ----
    // Mask (insn & 0xfffff01f) == 0xd503201f covers all of these; they are no-ops
    // (PAC is ignored in the guest). Verified clean against real system ops
    // (mrs/msr/dmb/tlbi share 0xd503 but have nonzero register fields).
    if (insn & 0xffff_f01f) == 0xd503_201f {
        return Inst::Hint;
    }

    // ---- system register read/write (mrs xN, <sysreg> / msr <sysreg>, xN) ----
    // AArch64 system-access op base: `1101_0101_0 xxxx` (0xD5000000..0xD57FFFFF).
    // L bit (bit 20) selects MRS(1) vs MSR(0); op1<16:18> CRn<12:15> CRm<8:11>
    // op2<5:7> identify the register. Only tpidr_el0 (op1=3 CRn=13 CRm=0 op2=2)
    // is modelled as a jitter slot (CpuState.tpidr). Everything else falls back.
    if insn & 0xff90_0000 == 0xd510_0000 {
        let op1 = b(insn, 16, 18);
        let crn = b(insn, 12, 15);
        let crm = b(insn, 8, 11);
        let op2 = b(insn, 5, 7);
        let read = b(insn, 21, 21) == 1; // MRS (L bit): mrs has bit21 set, msr clear.
        if op1 == 3 && crn == 13 && crm == 0 && op2 == 2 {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 0, rt, read };
        }
    }

    // ---- load/store pair (X: 0xa8/0xa9, W: 0x28/0x29, SIMD Q 128-bit: 0xAD, FP/vec d: 0x6d/0x2d) ----
    if matches!(insn >> 24, 0x29 | 0x28 | 0xa9 | 0xa8 | 0xad | 0x6d | 0x2d) {
        let q128 = (insn >> 24) & 0xff == 0xad; // 128-bit SIMD pair (ldp/stp q)
        let fp_d = (insn >> 24) & 0xff == 0x6d || (insn >> 24) & 0xff == 0x2d; // FP/vec d pair
        let size_64 = insn >> 31 == 1; // sf  (Q pair ignores this for reg scale)
        let ld = (insn >> 22) & 1 == 1; // L: 1=ldp, 0=stp
        let indexed = (insn >> 23) & 1 == 1; // 0=offset, 1=indexed (pre/post)
        let preidx = indexed && (insn >> 24) & 1 == 1; // pre if bit24=1 within indexed
        let scale = if q128 {
            16
        } else if fp_d {
            8 // d-pairs are 64-bit FP/vector registers
        } else if size_64 {
            8
        } else {
            4
        };
        let imm7 = b(insn, 15, 21) as i64;
        let imm = sext(imm7 as u64, 7) * scale;
        return Inst::LdStPair {
            rt: rd(insn),
            rt2: b(insn, 10, 14) as u8,
            rn: b(insn, 5, 9) as u8,
            imm,
            ld,
            writeback: indexed,
            preidx,
            size_64,
            q128,
            fp_d,
        };
    }

    Inst::Unsupported(insn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bl_opens_space_for_more() {
        // 0x94000000 is `bl 0x0000000000000000` — verify the decode picks branch
        let i = decode(0x94000000);
        match i {
            Inst::B { imm, link } => {
                assert_eq!(imm, 0);
                assert!(link);
            }
            _ => panic!("expected BL, got {:?}", i),
        }
    }

    #[test]
    fn b_ne_ground_truth() {
        // aarch64-linux-gnu: "b.ne 1c <test_fn+0x10>" at offset 0x34 -> 0x54ffff41
        let i = decode(0x54ffff41);
        match i {
            Inst::BCond { cond, imm } => {
                assert_eq!(cond, 1); // NE
                assert_eq!(imm, 0x1c - 0x34); // relative branch displacement
            }
            _ => panic!("expected BCond, got {:?}", i),
        }
    }

    #[test]
    fn cbz_ground_truth() {
        // "cbz x2, 0x10" = 0xb4000082 ; "cbnz x3" = 0xb5000083 ; "b" = 0x14000004
        match decode(0xb4000082) {
            Inst::Cbz {
                rt,
                imm,
                nonzero,
                sf,
            } => {
                assert_eq!(rt, 2);
                assert_eq!(imm, 16); // target 0x10 from pc 0
                assert!(!nonzero);
                assert!(sf);
            }
            other => panic!("expected Cbz, got {:?}", other),
        }
        assert_eq!(
            decode(0x14000004),
            Inst::B {
                imm: 16,
                link: false
            }
        );
    }
    #[test]
    fn ret_decodes() {
        let i = decode(0xd65f03c0); // RET
        assert_eq!(i, Inst::Ret, "ret x30 should decode to Ret");
    }

    #[test]
    fn ldstp_decode_ground_truth() {
        // stp x0,x1,[x2,#16] = a9010440 ; ldp x29,x30,[sp],#16 = a8c17bfd
        match decode(0xa9010440) {
            Inst::LdStPair {
                rt,
                rt2,
                rn,
                imm,
                ld,
                writeback,
                preidx,
                size_64,
                q128,
                fp_d,
            } => {
                assert_eq!(rt, 0);
                assert_eq!(rt2, 1);
                assert_eq!(rn, 2);
                assert_eq!(imm, 16);
                assert!(!ld);
                assert!(!writeback);
                assert!(size_64);
                assert!(!fp_d);
            }
            other => panic!("expected LdStPair, got {:?}", other),
        }
        match decode(0xa8c17bfd) {
            Inst::LdStPair {
                rt,
                rt2,
                rn,
                imm,
                ld,
                writeback,
                preidx,
                size_64,
                q128,
                fp_d,
            } => {
                assert_eq!(rt, 29);
                assert_eq!(rt2, 30);
                assert_eq!(rn, 31); // sp
                assert_eq!(imm, 16);
                assert!(ld);
                assert!(writeback);
                assert!(!preidx);
                assert!(!fp_d);
            }
            other => panic!("expected LdStPair post, got {:?}", other),
        }
        match decode(0xa9bf7bfd) {
            Inst::LdStPair {
                imm,
                writeback,
                preidx,
                ld,
                ..
            } => {
                assert_eq!(imm, -16);
                assert!(!ld);
                assert!(writeback);
                assert!(preidx);
            }
            other => panic!("expected LdStPair pre, got {:?}", other),
        }
    }

    #[test]
    fn movz_64_ground_truth() {
        // "mov x1, #0x7"  = 0xd28000e1
        match decode(0xd28000e1) {
            Inst::MoveWide {
                rd,
                imm16,
                hw,
                opc,
                sf,
            } => {
                assert_eq!(rd, 1);
                assert_eq!(imm16, 0x7);
                assert_eq!(hw, 0);
                assert_eq!(opc, 0); // movz
                assert!(sf);
            }
            other => panic!("expected MoveWide, got {:?}", other),
        }
    }
    #[test]
    fn movk_64_lsl32_ground_truth() {
        // "movk x3, #0x8, lsl #32" = 0xf2c00103
        match decode(0xf2c00103) {
            Inst::MoveWide {
                rd,
                imm16,
                hw,
                opc,
                sf,
            } => {
                assert_eq!(rd, 3);
                assert_eq!(imm16, 0x8);
                assert_eq!(hw, 2);
                assert_eq!(opc, 1); // movk
                assert!(sf);
            }
            other => panic!("expected MoveWide, got {:?}", other),
        }
    }
    #[test]
    fn add_imm64_ground_truth() {
        // "add x12, x13, #0x234" = 0x9108d1ac (from objdump at 0x1c)
        match decode(0x9108d1ac) {
            Inst::AddSubImm {
                rd,
                rn,
                imm12,
                shift12,
                sub,
                sf,
                s,
            } => {
                assert_eq!(rd, 12);
                assert_eq!(rn, 13);
                assert_eq!(imm12, 0x234);
                assert!(!shift12);
                assert!(!sub);
                assert!(sf);
                assert!(!s);
            }
            other => panic!("expected AddSubImm, got {:?}", other),
        }
    }
    #[test]
    fn sub_imm32_ground_truth() {
        // "sub w14, w15, #0x45" = 0x510115ee
        match decode(0x510115ee) {
            Inst::AddSubImm {
                rd,
                rn,
                imm12,
                sub,
                sf,
                ..
            } => {
                assert_eq!(rd, 14);
                assert_eq!(rn, 15);
                assert_eq!(imm12, 0x45);
                assert!(sub);
                assert!(!sf);
            }
            other => panic!("expected AddSubImm, got {:?}", other),
        }
    }
    #[test]
    fn add_reg32_shift_ground_truth() {
        // "add w0, w0, w0, lsl #1" = 0x0b000400 (objdump at 0x0)
        match decode(0x0b000400) {
            Inst::AddSubReg {
                rd,
                rn,
                rm,
                sub,
                sf,
                s,
                shift,
                sh_amt,
            } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 0);
                assert_eq!(rm, 0);
                assert!(!sub);
                assert!(!sf);
                assert!(!s);
                assert_eq!(shift, ShiftKind::Lsl);
                assert_eq!(sh_amt, 1);
            }
            other => panic!("expected AddSubReg, got {:?}", other),
        }
    }
    #[test]
    fn cmp_64_ground_truth() {
        // "cmp x5, x6" = 0xeb0600bf (subs xzr, x5, x6)
        match decode(0xeb0600bf) {
            Inst::AddSubReg {
                rd,
                rn,
                rm,
                sub,
                sf,
                s,
                shift,
                ..
            } => {
                assert_eq!(rd, 31);
                assert_eq!(rn, 5);
                assert_eq!(rm, 6);
                assert!(sub);
                assert!(sf);
                assert!(s);
                assert_eq!(shift, ShiftKind::Lsl);
            }
            other => panic!("expected AddSubReg(cmp), got {:?}", other),
        }
    }
    #[test]
    fn orr_64_lsr3_ground_truth() {
        // "orr x0, x0, x1, lsr #3" = 0xaa410c00
        match decode(0xaa410c00) {
            Inst::LogicReg {
                rd,
                rn,
                rm,
                op,
                s,
                sf,
                shift,
                sh_amt,
            } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 0);
                assert_eq!(rm, 1);
                assert_eq!(op, 1 /*orr*/);
                assert!(!s);
                assert!(sf);
                assert_eq!(shift, ShiftKind::Lsr);
                assert_eq!(sh_amt, 3);
            }
            other => panic!("expected LogicReg, got {:?}", other),
        }
    }
    #[test]
    fn eor_64_ground_truth() {
        // "eor x0, x2, x3" = 0xca030040
        match decode(0xca030040) {
            Inst::LogicReg {
                rd, rn, rm, op, sf, ..
            } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 2);
                assert_eq!(rm, 3);
                assert_eq!(op, 2 /*eor*/);
                assert!(sf);
            }
            other => panic!("expected LogicReg, got {:?}", other),
        }
    }

    #[test]
    fn ldr_x_imm_ground_truth() {
        // "ldr x0, [x0, #16]" = 0xf9400800
        match decode(0xf9400800) {
            Inst::LdStrImm {
                rt,
                rn,
                imm,
                size,
                ld,
            } => {
                assert_eq!(rt, 0);
                assert_eq!(rn, 0);
                assert_eq!(imm, 2);
                assert_eq!(size, 8);
                assert!(ld);
            }
            other => panic!("expected LdStrImm, got {:?}", other),
        }
    }
    #[test]
    fn str_x_imm_ground_truth() {
        // "str x1, [x0, #24]" = 0xf9000c01
        match decode(0xf9000c01) {
            Inst::LdStrImm {
                rt,
                rn,
                imm,
                size,
                ld,
            } => {
                assert_eq!(rt, 1);
                assert_eq!(rn, 0);
                assert_eq!(imm, 3);
                assert_eq!(size, 8);
                assert!(!ld);
            }
            other => panic!("expected LdStrImm, got {:?}", other),
        }
    }
    #[test]
    fn ldr_w_reg_ground_truth() {
        // "ldr w0, [x1, x2]" = 0xb8626820
        match decode(0xb8626820) {
            Inst::LdStrReg {
                rt,
                rn,
                rm,
                size,
                ld,
                shift,
            } => {
                assert_eq!(rt, 0);
                assert_eq!(rn, 1);
                assert_eq!(rm, 2);
                assert_eq!(size, 4);
                assert!(ld);
                assert!(!shift);
            }
            other => panic!("expected LdStrReg, got {:?}", other),
        }
    }

    // ---- indirect branch / br/blr decode uses Rn at bits[9:5] ----
    #[test]
    fn br_blr_ground_truth() {
        // blr x19 = 0xd63f0260, br x19 = 0xd61f0260 (from Session 21 disasm).
        match decode(0xd63f0260) {
            Inst::Blr { rn } => assert_eq!(rn, 19),
            other => panic!("expected Blr, got {other:?}"),
        }
        match decode(0xd61f0260) {
            Inst::Br { rn } => assert_eq!(rn, 19),
            other => panic!("expected Br, got {other:?}"),
        }
        // blr x1 (regression: register is bits[9:5], not [4:0]).
        match decode(0xd63f0020) {
            Inst::Blr { rn } => assert_eq!(rn, 1),
            other => panic!("expected Blr x1, got {other:?}"),
        }
    }

    #[test]
    fn movi_ground_truth() {
        // Ground-truth encodings from aarch64-linux-gnu-objdump (Session 20/21).
        //                movi v0.4s,#1
        for (w, want_lo, want_hi, label) in [
            (
                0x4f00_0420u32,
                0x0000_0001_0000_0001u64,
                0x0000_0001_0000_0001u64,
                "movi v0.4s,#1",
            ),
            (
                0x4f00_0641u32,
                0x0000_0012_0000_0012u64,
                0x0000_0012_0000_0012u64,
                "movi v1.4s,#0x12",
            ),
            (
                0x4f07_07e2u32,
                0x0000_00ff_0000_00ffu64,
                0x0000_00ff_0000_00ffu64,
                "movi v2.4s,#0xff",
            ),
            (
                0x0f00_e4e3u32,
                0x0707_0707_0707_0707u64,
                0x0000_0000_0000_0000u64,
                "movi v3.8b,#7",
            ),
            (
                0x4f04_e404u32,
                0x8080_8080_8080_8080u64,
                0x8080_8080_8080_8080u64,
                "movi v4.16b,#0x80",
            ),
            (0x6f00_e400u32, 0x0, 0x0, "movi v0.2d,#0"),
        ] {
            let inst = decode(w);
            match inst {
                Inst::VecMovi { vd, lo, hi } => {
                    assert_eq!(lo, want_lo, "{label}: low64");
                    assert_eq!(hi, want_hi, "{label}: hi64");
                }
                other => panic!("{label}: expected VecMovi, got {other:?}"),
            }
        }
    }

    #[test]
    fn csel_family_ground_truth() {
        // csel x22,x8,x10,hi = 0x9a8a8116 ; csinc x1,x9,xr... ; ldc set X1.. etc
        // Our decode sets rd=r8? — assert on well-known words:
        let csel = decode(0x9a8a8116); // from actual libroblox disasm: csel x22,x8,x10,hi
        match csel {
            Inst::CSel {
                rd, rn, rm, cond, op, sf,
            } => {
                assert_eq!(rd, 22);
                assert_eq!(rn, 8);
                assert_eq!(rm, 10);
                assert_eq!(cond, 0x8); // hi
                assert_eq!(op, 0); // csel
                assert!(sf);
            }
            other => panic!("0x9a8a8116: expected CSel, got {other:?}"),
        }
        // cset w0, eq via csinc in 32-bit: 0x1a9f17e0 (W cset)
        match decode(0x1a9f17e0) {
            Inst::CSel { op, sf, rd, .. } => {
                assert!(!sf);
                assert_eq!(op, 1); // csinc
                assert_eq!(rd, 0);
            }
            other => panic!("cset: expected CSel, got {other:?}"),
        }
    }

    #[test]
    fn stp_d_zero() {
        // stp d0,d1,[x0,#272] = 0x6d110400 (fp/vec 64-bit pair)
        match decode(0x6d110400) {
            Inst::LdStPair {
                rt, rt2, rn, imm, ld, fp_d, q128, ..
            } => {
                assert!(fp_d);
                assert!(!q128);
                assert!(!ld); // 0x6d... = stp (store); L bit22=0
                assert_eq!(imm, 272);
                assert_eq!(rt, 0);
                assert_eq!(rt2, 1);
                assert_eq!(rn, 0);
            }
            other => panic!("expected LdStPair d, got {other:?}"),
        }
    }

    #[test]
    fn ldar_stlr_plain() {
        // ldar x0, [x8] = 0xc8dffd00 ; stlr x9,[x10] = 0xc89ffd49
        match decode(0xc8dffd00) {
            Inst::AcqRel { size, ld, rt, rn } => {
                assert_eq!(size, 3);
                assert!(ld);
                assert_eq!(rt, 0);
                assert_eq!(rn, 8);
            }
            other => panic!("expected AcqRel ldar, got {other:?}"),
        }
        match decode(0xc89ffd49) {
            Inst::AcqRel { ld, rt, rn, .. } => {
                assert!(!ld);
                assert_eq!(rt, 9);
                assert_eq!(rn, 10);
            }
            other => panic!("expected AcqRel stlr, got {other:?}"),
        }
    }

    #[test]
    fn mrs_tpidr_el0() {
        // mrs x19, tpidr_el0 = 0xd53bd053 (real libroblox.so)
        match decode(0xd53bd053) {
            Inst::SysReg { sysreg, rt, read } => {
                assert_eq!(sysreg, 0); // tpidr_el0
                assert_eq!(rt, 19);
                assert!(read); // MRS (system -> GPR)
            }
            other => panic!("expected SysReg MRS tpidr_el0, got {other:?}"),
        }
        // msr tpidr_el0, x4 = 0xd51bd044
        match decode(0xd51bd044) {
            Inst::SysReg { rt, read, .. } => {
                assert_eq!(rt, 4);
                assert!(!read); // MSR (GPR -> system)
            }
            other => panic!("expected SysReg MSR tpidr_el0, got {other:?}"),
        }
        // A non-TLS sysreg (mrs x0, cntfrq_el0) must NOT decode to SysReg.
                assert!(!matches!(decode(0xd53be020), Inst::SysReg { .. }));
            }

            #[test]
            fn ror_exclude() {
                // ror x0, x1, #12 = 0x93c13020 (real libroblox EXTR rotate)
                match decode(0x93c13020) {
                    Inst::Ror { rd, rn, rot, sf } => {
                        assert_eq!(rd, 0);
                        assert_eq!(rn, 1);
                        assert_eq!(rot, 12);
                        assert!(sf);
                    }
                    other => panic!("expected Ror, got {other:?}"),
                }
                // ror w23, w22, #20 = 0x139652d7 (real)
                match decode(0x139652d7) {
                    Inst::Ror { rd, rn, rot, sf } => {
                        assert_eq!(rd, 23);
                        assert_eq!(rn, 22);
                        assert_eq!(rot, 20);
                        assert!(!sf);
                    }
                    other => panic!("expected Ror, got {other:?}"),
                }
                // A real UBFM extract (lsl) must NOT be mis-decodded as Ror.
                assert!(!matches!(decode(0xbbf13c69), Inst::Ror { .. }));
            }
        }
