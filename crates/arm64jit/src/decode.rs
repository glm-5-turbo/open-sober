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
    // ---- scalar 1-source FP (no rn/rm): d-dst<-f(d-src) ----
    FpUnary {
        rd: u8,
        rn: u8,
        op: u8, // 0=fsqrt, 1=frintm(toward -inf), 2=frintp(+inf), 3=frintz(toward 0)
        sz: bool, // true = double
    },
    // ---- scalar FP absolute difference: fabd Dd, Dn, Dm = |dn - dm| ----
    // Gate (insn & 0xffe0_fc00)==0x7ee0_d400 (scalar double; verified vs real
    // 0x7ee1d503 and compiler 0x7ee1d400). Disjoint from fadd/fmul/fdiv/fcmp.
    Fabd { rd: u8, rn: u8, rm: u8 },
    // ---- FP convert to integer (fcvtas/fcvtzs): Dn|Sn -> Rd (signed int) ----
    FcvtToInt {
            rd: u8,
            rn: u8,        // source fp reg
            mode: u8,      // 0=fcvtzs, 2=fcvtas, 1=fcvtzu, 3=fcvtzu(nearest? unused)
            sf: bool,      // 64-bit dest
            unsigned: bool, // fcvtzu: convert to unsigned int (clamp/-to-0 semantics)
        },
        // ---- FP convert from signed integer (scvtf: Wn|Xn -> Sd|Dd) ----
        Scvtf {
            rd: u8,        // destination FP reg
            rn: u8,        // source integer reg
            to_double: bool, // true => Dd (double), false => Sd (single)
            sf: bool,      // true => 64-bit source reg (Rn), false => 32-bit (Wn)
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
           // ---- FMOV scalar immediate (fmov Dd, #imm / fmov Sd, #imm) ----
           // Encodes an 8-bit vfp-immediate (imm3:imm5) into a concrete IEEE-754
           // value (`value_bits` already decoded by decode_fmov_imm). `f64` => Dd.
           FmovImm {
               rd: u8,          // destination 64-bit (d) or 32-bit (s) FP reg
               f64: bool,       // true => double (8B) result, false => single (4B)
               value_bits: u64, // IEEE-754 bits (f64 `value` for f64, low 32 for f32)
       },
       // ---- scalar FP register-to-register move (fmov Dd,Dn / fmov Sd,Sn) ----
       FmovFp {
           rd: u8,          // destination FP reg
           rn: u8,          // source FP reg
           sz: bool,        // true = double (8B move), false = single (4B)
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
                           // ---- scalar FP compare to NZCV (fcmp Dn, Dm / fcmp Dn, #0.0) ----
                           Fcmp {
                               rn: u8, // first operand (source fp reg / d-reg)
                               rm: u8, // second fp reg (0 for the #0.0 form)
                           },
    // ---- scalar FP conditional select: fcsel Dd, Dn, Dm, <cond> ----
        FcsSel {
            rd: u8,   // destination FP reg
            rn: u8,   // "if-true" FP reg
            rm: u8,   // "else" FP reg
            cond: u8, // AArch64 condition code (0-14)
            sz: bool, // true = double (8B), false = single (4B)
        },
        // ---- NEON: mov Vd.D[1], Vn.D[0] (dup low 64 into the high 64 lane) ----
                           InsD1D0 { rd: u8, rn: u8 }, // v16B: slot_hi(8B) = low-64-of-Vn
                           // ---- EXTR / ROR rotate: rm==rn in the EXTR base ----
                           Ror { rd: u8, rn: u8, rot: u32, sf: bool }, // ror rd,rn,#rot
                           // ---- supervisor call (svc #imm) -> host syscall routing ----
                           Svc { imm: u16 },
                           // ---- NEON lane add: add Vd.4s, Vn.4s, Vm.4s ---------
    Simd4s {
        rd: u8,
        rn: u8,
        rm: u8,
        op: u8, // 0=add (currently), future sub/etc
    },
    // ---- NEON vector unary unsigned int->double: ucvtf Vd.2D, Vn.2D ----
    // Converts the two 64-bit lanes of Vn (viewed as unsigned) to two doubles
    // in Vd. Gate (insn & 0xffe0_fc00)==0x6e60d800 (verified vs real decir0x6e61d842
    // and compiler 0x6e61dbff; excludes scvtf/scalar/compare forms).
    Ucvtf2d { rd: u8, rn: u8 },
    // ---- SIMD dup: dup Vd.2D, Vn.D[index] (broadcast one 64-bit lane) ----
    // Both 64-bit lanes of Vd get Vn's selected lane. Gate
    // (insn & 0xffff_fc00)==0x4e180400 (the Q=1 vector dup-d; distinct from the
    // 0x6e18:0x4e18 ins-variant). index in bit 16 (`[.../inst]` D[0] vs D[1]).
    SimdDupD { rd: u8, rn: u8, index: u8 },
    // ---- SIMD 2xdouble FP arithmetic: op Vd.2D, Vn.2D, Vm.2D (lanewise) ----
    // op 0=fdiv,1=fmul,2=fadd,3=fsub. Gate mask 0xffe0_fc00 gives the
    // per-op constants {0x6e60fc00,0x6e60dc00,0x4e60d400,0x4ee0d400}.
    Simd2dFp { rd: u8, rn: u8, rm: u8, op: u8 },
    // ---- SIMD dup from a GPR: dup Vd.4S, Wn (broadcast Wn into 4x32-bit lanes) ----
    // Gate `(insn & 0xffff_fc00) == 0x4e040c00` (the 4S GPR-source dup; distinct
    // from the vector-lane dup 0x4e180400 / 0x4e040400). rn (W source) bits 5-9.
    SimdDupSReg { rd: u8, rn: u8 },
    // ---- SIMD 16-byte logical OR: orr Vd.16B, Vn.16B, Vm.16B ----
    // Gate (insn & 0xffe0_fc00)==0x0ea01c00 (also the `mov Vd.16B,Vn.16B` copy
    // alias rm==rn, e.g. real 0x4ea01c02). ORs the full 16-byte vector slot.
    SimdOrr16 { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD 32-bit lane multiply: mul Vd.4S/Vd.2S, Vn., Vm. ----
    // 4S gate (Q=1) 0x4ea09c00 ; 2S gate (Q=0) 0x0ea09c00. Per-lane low-32 product.
    SimdMul { rd: u8, rn: u8, rm: u8, lanes: u8 },
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

/// Decode an AArch64 logical-immediate bitmask from `N`/`immr`/`imms`,
/// following the ARM ARM `DecodeBitMasks(...)` procedure (DDI0487, "OG­7,
/// Logical instructions" / immediately-encoded uses). Returns the `datasize`-bit
/// operand mask, or `None` if the encoding is architecturally invalid.
///
/// Decode the 8-bit AArch64 FP-immediate (fmov Dd, #imm) into IEEE-754 bits.
///
/// The 8-bit `imm8` (from the instruction at bits[13:20]) packs
///   bit7 sign, bits[6:4] exponent (3-bit), bits[3:0] 4-bit mantissa.
/// The value is `(+/-1) * (1 + m/16) * 2^ex` where `ex = (e + 1) mod 8`
/// interpreted as a signed 3-bit exponent. Verified against 12 compiler-emitted
/// `fmov d,#imm` encodings (0.5, 1, 2, 3, 4, -2, 0.75, 1.5, 2.5, 5, 6, 10).
fn decode_fmov_imm(imm8: u32, f64: bool) -> u64 {
    let sign = (imm8 >> 7) & 1 == 1;
    let e = (imm8 >> 4) & 7;
    let m = imm8 & 0xf;
    let mut ex = ((e + 1) & 7) as i32; // exponent in one 3-bit 2's-complement
    if ex >= 4 {
        ex -= 8;
    }
    // value = (1 + m/16) * 2^ex, composed as IEEE-754 bits directly.
    if f64 {
        // value = (1 + m/16) * 2^ex as double: field = <sign> <ex+1023> <m-then-zeros>.
        let e_bits = (ex as u64).wrapping_add(1023); // biased exponent field
        let mant = (m as u64) << 48; // 4-bit mantissa in the top of the fraction
        (if sign { 1u64 } else { 0u64 } << 63) | (e_bits << 52) | mant
    } else {
        // single: value = 1.m/16 * 2^ex, bias 127, mantissa low 23 bits
        let e_bits = ((ex as u64).wrapping_add(127)) & 0xff;
        let mant = (m as u64) << 19; // 4 mantissa bits at [22..19]
        ((if sign { 1u64 } else { 0u64 } << 31) | (e_bits << 23) | mant) & 0xffff_ffff
    }
}
/// This handles every element size (64/32/16/8) and the N=0 case where `imms`
/// carries a leading run of 1s that simpler N==1/N==0 splits reject — that edge
/// is what made a real `mov x8,#0xcccccccccccccccc` (imm = alternating bits)
/// wrongly fall through to `Unsupported`.
fn decode_logical_mask(n: u32, immr: u32, imms: u32, datasize: u64) -> Option<u64> {
    // DecodeBitMasks (ARM ARM DDI0487): len = HighestSetBit( immN : NOT(imms) ).
    // `imms` is a 6-bit field; the "NOT" includes the immN carry-in at bit 6.
    let combined = (((n as u64) & 1) << 6) | (((!imms) & 0x3f) as u64);
    if combined == 0 {
        return None; // len = -1 => Undefined
    }
    let len = 63 - combined.leading_zeros(); // highest set bit index over 7-bit field (0..=6)
    if len < 1 || len >= 7 {
        return None; // element must be at least 2 bits
    }
    let esize: u64 = 1u64 << len; // element size in bits (2,4,8,16,32,64)
    let levels: u64 = esize - 1; // masks the search bits (S/R)
    if (imms as u64 & levels) == levels {
        // ARM DecodeBitMasks: S == all-ones (imms AND levels == levels) is
        // Undefined. S == 0 IS valid (yields the mask 0x1), so do NOT reject S==0.
        return None;
    }
    let s = (imms as u64 & levels) as u32; // S = imms AND levels
    let r = (immr as u64 & levels) as u32; // R = immr AND levels
    // welem = (1 << (S+1)) - 1, truncated to `esize` bits (a run of (S+1) ones),
    // then ROR by R **within the esize-bit element** (ARM: ROR(welem, R) on an
    // esize-bit value; rotating a full-u64 here is what produced the wrong mask).
    let esize_mask: u64 = if esize >= 64 { u64::MAX } else { (1u64 << esize) - 1 };
    let sl1 = s as u64 + 1; // S+1 in [1, esize]  (esize<=64)
    let ones: u64 = if sl1 == 64 {
        u64::MAX // 1<<64 not representable; all ones
    } else {
        (1u64 << sl1).wrapping_sub(1)
    } & esize_mask;
    let welem = if r == 0 {
        ones
    } else {
        // Rotate right by R **within the esize-bit element** (ARM ROR on an
        // esize-bit value). A full-u64 `rotate_right` pushes the high 4-bit
        // element into bits 63.. which `& esize_mask` would then discard — so
        // use a shift-based esize-local rotate.
        let r = r as usize;
        (ones << r | ones >> (esize as usize - r)) & esize_mask
    };
    // Replicate the `esize`-bit element across the full `datasize` register.
    let mut mask: u64 = 0;
    let mut i: u64 = 0;
    while i < datasize {
        mask |= welem << i;
        i += esize;
    }
    if datasize < 64 {
        mask &= (1u64 << datasize) - 1;
    }
    Some(mask)
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
        if let Some(mask) = decode_logical_mask(n, immr, imms, if sf { 64 } else { 32 }) {
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
        // 1-source scalar FP in the same 0x1e00_0000 class: fsqrt=0x1e61c000,
        // frintm(toward -inf)=0x1e654000, frintp(+inf)=0x1e648000, frintz=0x1e65c000.
        // fmov/fneg/fabs are separate (FmovFp / not yet modelled).
        // 1-source scalar FP in the same 0x1e00_0000 class. Gate `0xffff_fc00` masks
        // rn(5-9)+rd(0-4) only and keeps bits 16-31, which DISTINGUISHES the
        // frint/fsqrt bytes (bits16-23): fsqrt=0x1e61_c000, frintm=0x1e65_4000.
        // `fmov d,d` (=0x1e60_4000, bits16-19=0) is NOT matched, staying FmovFp.
        let unary = match insn & 0xffff_fc00 {
            0x1e61_c000 => Some(0), // fsqrt d{rd}, d{rn}
            0x1e65_4000 => Some(1), // frintm (round toward -inf) = floor
            _ => None,
        };
        if let Some(op) = unary {
            let rd = (insn & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            return Inst::FpUnary { rd, rn, op, sz };
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
    // Real encodings (verified): fcvtzs Wd,Dn = 0x1e78_0000 / Xd = 0x9e78_0000
    // (truncate); fcvtas Wd,Dn = 0x1e7a_0000 / Xd = 0x9e7a_0000 (nearest-away).
    // Rounding drives `mode`: 0=truncate (fcvtzs), 2=nearest-away (fcvtas).
    {
        let b = insn & 0xffff_f800;
        let (mode, ok) = if b == 0x1e78_0000 || b == 0x9e78_0000 {
            (0, true) // fcvtzs: truncate
        } else if b == 0x1e7a_0000 || b == 0x9e7a_0000 {
            (2, true) // fcvtas: round nearest-away
        } else {
            (0, false)
        };
        if ok {
            let sf = (insn >> 31) & 1 == 1;
            let sz = (insn >> 22) & 1 == 1; // 1 => source is double (d)
            if sz {
                let rn = ((insn >> 5) & 0x1f) as u8;
                let rd = (insn & 0x1f) as u8;
                return Inst::FcvtToInt {
                    rd,
                    rn,
                    mode,
                    sf,
                    unsigned: false,
                };
            }
            return Inst::Unsupported(insn); // single (s) source not modelled yet
        }
    }

    // ---- FP convert to UNSIGNED integer (fcvtzu): Dn -> Rd (unsigned int) ----
    // bases 0x1e79_0000 (W dest) / 0x9e79_0000 (X dest). Distinct from signed
    // fcvtzs at 0x1e78_0000/0x9e78_0000 (bit16 of the nibble: 0x79 vs 0x78).
    if (insn & 0xffff_f800) == 0x1e79_0000 || (insn & 0xffff_f800) == 0x9e79_0000 {
        let sf = (insn >> 31) & 1 == 1; // 1 => 64-bit (X) destination
        let sz = (insn >> 22) & 1 == 1; // 1 => source is double (d)
        if sz {
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            return Inst::FcvtToInt {
                rd,
                rn,
                mode: 0, // truncate-toward-zero (fcvtzu always truncates)
                sf,
                unsigned: true,
            };
        }
    }

        // ---- FP convert from signed integer (scvtf): Wn|Xn -> Dd (double) ----
        // `scvtf d0, w0 = 0x1e620000`. The int->FP family is at base 0x1e60_0000
        // (double dest) / 0x1e22_0000 (single); FP->int `fcvt*` sits at the SAME
        // 0x1e60_0000 base but has bit17=0 (fcvtns=0x1e600000, fcvtzs=0x1e780000,
        // fcvtas=0x1e7a0000), whereas `scvtf` sets bit17 — so require bit17.
        if (insn & 0x7ff0_fc00) == 0x1e60_0000 && (insn & 0x20000) != 0 {
            let sf = (insn >> 31) & 1 == 1; // 1 => 64-bit integer src (Xn)
            let to_double = true;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            return Inst::Scvtf {
                rd,
                rn,
                to_double,
                sf,
            };
        }

                // ---- FMOV scalar immediate (fmov Dd, #imm) / (fmov Sd, #imm) ----
                // Double imm family `0x1e_XX_1...` (imm8 in bits 13:20, `0x1000`
                // lane anchor). Gate `(insn&0xffe0_0000)==0x1e60_0000` (masks out
                // the imm byte at bits 16:23) selects the double-scalar base; a
                // distinct class from scvtf/ffcvt (handled above, need bit17/other)
                // and from fcvtzs/fcvtzu (their 0x1e78/0x1e79 bases differ). The
                // `0x1000` bit anchors the double-imm form vs `0x0_0000` shares.
                if (insn & 0xffe0_0000) == 0x1e60_0000 && (insn & 0x1000) != 0 {
                    let rd = (insn & 0x1f) as u8;
                    let imm8 = ((insn >> 13) & 0xff) as u32;
                    let value_bits = decode_fmov_imm(imm8, /* f64 */ true);
                    return Inst::FmovImm {
                        rd,
                        f64: true,
                        value_bits,
                    };
                }
                // ---- scalar FP register-to-register move: fmov Dd,Dn / fmov Sd,Sn ----
                // Double form = 0x1e60_4000, single form = 0x1e20_4000 (sz bit selects).
                if (insn & 0xffff_f000) == 0x1e60_4000 {
                    let sz = (insn >> 22) & 1 == 1; // 1 => double (d), 0 => single (s)
                    let rn = ((insn >> 5) & 0x1f) as u8;
                    let rd = (insn & 0x1f) as u8;
                    return Inst::FmovFp { rd, rn, sz };
                }
                // ---- scalar FP compare to NZCV: fcmp Dn, Dm (dbl) / fcmp Dn, #0.0 ----
                    // Gate `(insn & 0xffe0_fc00) == 0x1e602000` masks out rn(5-9)/rm(16-20)/rd(0-4)
                    // and keeps the fixed `0x...20...` + top bytes, so high rm registers (bit16-20
                    // feeding into the base nibble, e.g. fcmp d6,d16 = 0x1e7020c0) still resolve.
                    // rm==0 covers the `fcmp Dn, #0.0` form (ignored operand => compare with 0.0).
                    if (insn & 0xffe0_fc00) == 0x1e602000 {
                        let rn = ((insn >> 5) & 0x1f) as u8;
                        let rm = ((insn >> 16) & 0x1f) as u8;
                        return Inst::Fcmp { rn, rm };
                            }
                            // ---- scalar FP absolute difference: fabd Dd, Dn, Dm = |dn - dm| ----
                            // Gate (insn & 0xffe0_fc00) == 0x7ee0_d400 (scalar double; disjoint from
                            // fadd/fmul/fdiv/fcmp/scvtf). rn=bits5-9, rm=bits16-20, rd=bits0-4.
                            if (insn & 0xffe0_fc00) == 0x7ee0_d400 {
                                let rn = ((insn >> 5) & 0x1f) as u8;
                                let rm = ((insn >> 16) & 0x1f) as u8;
                                let rd = (insn & 0x1f) as u8;
                                return Inst::Fabd { rd, rn, rm };
                            }
                            // ---- scalar FP conditional select: fcsel Dd, Dn, Dm, <cond> ----
                                // Structural mask `(insn & 0x1f20_0c00) == 0x1e20_0c00` separates
                                // fp-select (the 0x800/0x400 in 0x..c00) from fcmp/fcmpe/fmov/fcvt
                                // (which mask to 0x1e200000/0x1e200400). cond in bits 12-15 is
                                // cleared by the 0x0c00 mask; rn/rm/rd are the low fields.
                                if (insn & 0x1f20_0c00) == 0x1e20_0c00 {
                                    let sz = (insn >> 22) & 1 == 1; // double if bit22 set
                                    let cond = ((insn >> 12) & 0xf) as u8;
                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                    let rd = (insn & 0x1f) as u8;
                                    return Inst::FcsSel { rd, rn, rm, cond, sz };
                                }
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
                                        // ---- NEON vector u64->f64: ucvtf Vd.2D, Vn.2D (2 unsigned lanes) ----
                                        // Gate `(insn & 0xffe0_fc00) == 0x6e60d800`: masks rn/rd (bits 0-9, 16-20 via
                                        // 0xffe0/fc00) and keeps the top+convert bits. Verified against real
                                        // 0x6e61d842 (ucvtf v2.2d,v2.2d) and compiler 0x6e61dbff (ucvtf v31.2d,v31.2d);
                                        // excludes scvtf (0x4e60d800), scalar d,d (0x7e60d800), and compare/fmov forms.
                                        if (insn & 0xffe0_fc00) == 0x6e60_d800 {
                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                let rd = (insn & 0x1f) as u8;
                                                return Inst::Ucvtf2d { rd, rn };
                                            }
                                            // ---- SIMD dup: dup Vd.2D, Vn.D[index] (broadcast one 64-bit lane) ----
                                                // Gate `(insn & 0xffff_fc00)==0x4e180400`: the Q=1 vector `dup` (element from
                                                // the same vector), distinguished from the GPR-source `dup Vd.2D,Xn`
                                                // (0x4e08_0000) and from `ins` (0x6e18_0400) by the top byte / imm.
                                                // index for the D (64-bit) lane is bit12 (0 => D[0] low lane, 1 => D[1]
                                                // high lane); rd in 0-4, rn in 5-9.
                                                if (insn & 0xffff_fc00) == 0x4e180400 {
                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                    let rd = (insn & 0x1f) as u8;
                                                    let index = ((insn >> 20) & 1) as u8;
                                                                                                            return Inst::SimdDupD { rd, rn, index };
                                                                                                        }
                                                                                                        // ---- SIMD dup from GPR: dup Vd.4S, Wn ----
                                                                                                        if (insn & 0xffff_fc00) == 0x4e040c00 {
                                                                                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                let rd = (insn & 0x1f) as u8;
                                                                                                                return Inst::SimdDupSReg { rd, rn };
                                                                                                            }
                                                                                                            // ---- SIMD 16-byte logical OR: orr Vd.16B, Vn.16B, Vm.16B ----
                                                                                                            // (insn & 0xffe0_fc00)==0x4ea01c00 catches both real `mov v2.16b` (0x4ea01c02,
                                                                                                                // rm==rn copy) and `orr v3.16b` (0x4ea41c63). Q=1 => 0x4ea0 (bit30). OR all 16B.
                                                                                                                if (insn & 0xffe0_fc00) == 0x4ea0_1c00 {
                                                                                                                let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                let rd = (insn & 0x1f) as u8;
                                                                                                                return Inst::SimdOrr16 { rd, rn, rm };
                                                                                                                    }
                                                                                                                    // ---- SIMD 32-bit lane multiply: mul Vd.4S/Vd.2S, Vn., Vm. ----
                                                                                                                    // 4S (Q=1) gate 0x4ea09c00 ; 2S (Q=0) gate 0x0ea09c00.
                                                                                                                    let sm = insn & 0xffe0_fc00;
                                                                                                                    let mul_lanes = if sm == 0x4ea0_9c00 { Some(4) } else if sm == 0x0ea0_9c00 { Some(2) } else { None };
                                                                                                                    if let Some(lanes) = mul_lanes {
                                                                                                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                        return Inst::SimdMul { rd, rn, rm, lanes };
                                                                                                                    }
                                                                                                        // ---- SIMD 2xdouble FP: op Vd.2D,Vn.2D,Vm.2D ----
                                                                                                        let s2 = insn & 0xffe0_fc00;
                                                                                                        let op2d = match s2 {
                                                                                                            0x6e60_fc00 => Some(0), // fdiv
                                                                                                            0x6e60_dc00 => Some(1), // fmul
                                                                                                            0x4e60_d400 => Some(2), // fadd
                                                                                                            0x4ee0_d400 => Some(3), // fsub
                                                                                                            _ => None,
                                                                                                        };
                                                                                                        if let Some(op) = op2d {
                                                                                                            let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                            let rd = (insn & 0x1f) as u8;
                                                                                                            return Inst::Simd2dFp { rd, rn, rm, op };
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

    // ---- supervisor call: svc #imm (0xd4000001 | imm<<5) -> host syscall ----
    if (insn & 0xffe0_001f) == 0xd400_0001 {
        let imm = ((insn >> 5) & 0xffff) as u16;
        return Inst::Svc { imm };
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
        // mrs xN, cntfrq_el0 = 0xd53be000: the cnt* group is op1=11 (bits 19:16),
        // CRn=14, CRm=0. op1 needs the full 4-bit field (the 3-bit
        // `op1` above only suffices for tpidr_el0's op1==3). Counters share the
        // same op1/crn/crm and differ by op2: cntfrq_el0=0 (rate Hz), cntpct_el0
        // =1 (physical time), cntvct_el0=2 (virtual time, live counter).
        if b(insn, 16, 19) == 11 && crn == 14 && crm == 0 && read {
            let rt = (insn & 0x1f) as u8;
            let sysreg = match op2 {
                0 => 1, // cntfrq_el0
                2 => 3, // cntvct_el0
                _ => return Inst::Unsupported(insn),
            };
            return Inst::SysReg { sysreg, rt, read };
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

                            #[test]
                            fn svc_decode() {
                                // svc #0 = 0xd4000001 ; svc #0x7a = 0xd4000f41
                                match decode(0xd4000001) {
                                    Inst::Svc { imm } => assert_eq!(imm, 0),
                                    other => panic!("expected Svc#0, got {other:?}"),
                                }
                                match decode(0xd4000f41) {
                                    Inst::Svc { imm } => assert_eq!(imm, 0x7a),
                                    other => panic!("expected Svc#7a, got {other:?}"),
                                }
                            }
}

#[cfg(test)]
mod logical_imm_regressions {
    use super::*;

    #[test]
    fn mov_ccc_imm_and_orr_one_and_scvtf() {
        // mov x8,#0xcccc... : DecodeBitMasks element edge we fixed.
        let w1 = decode(0xb202e7e8);
        assert!(
            matches!(w1, Inst::LogicImm { mask: 0xcccc_cccc_cccc_cccc, op: 1, ..}),
            "mov x8,#0xccc -> {w1:?}"
        );
        // orr x8,x22,#0x1  : S==0 is a valid immediate (we reject S==all-ones).
        let w2 = decode(0xb24002c8);
        assert!(matches!(w2, Inst::LogicImm { mask: 0x1, ..}), "orr #1 -> {w2:?}");
        // scvtf d0, w0 = 0x1e620000.
        let w3 = decode(0x1e620000);
        assert!(matches!(w3, Inst::Scvtf { rd: 0, rn: 0, to_double: true, sf: false , ..}), "scvtf -> {w3:?}");
        // fcvtns w0, d0 = 0x1e600000 must NOT decode as Scvtf (bit17=0).
        assert!(!matches!(decode(0x1e600000), Inst::Scvtf { .. }));
        // fcvtzu x19, d0 = 0x9e790013 (real libroblox) => unsigned FP->u64.
        match decode(0x9e790013) {
            Inst::FcvtToInt { rd, rn, mode, sf, unsigned } => {
                assert_eq!(rd, 19);
                assert_eq!(rn, 0);
                assert!(sf); // X dest
                assert!(unsigned); // fcvtzu (not the signed fcvtzs)
                assert_eq!(mode, 0); // truncate
            }
            other => panic!("fcvtzu x19,d0 -> {other:?}"),
        }
        // fcvtzu w0, d0 = 0x1e7903e0 => W dest.
        match decode(0x1e7903e0) {
            Inst::FcvtToInt { sf, unsigned, .. } => {
                assert!(!sf);
                assert!(unsigned);
            }
            other => panic!("fcvtzu w0,d0 -> {other:?}"),
        }
        // signed fcvtzs must NOT be flagged unsigned: 0x1e7803e0 = fcvtzs w0,d0.
        match decode(0x1e7803e0) {
            Inst::FcvtToInt { unsigned, .. } => assert!(!unsigned),
            other => panic!("fcvtzs w0,d0 -> {other:?}"),
        }
        // mrs x19, cntfrq_el0 (real libroblox) => SysReg cntfrq (sysreg==1).
        match decode(0xd53be013) {
            Inst::SysReg { sysreg, rt, read } => {
                assert_eq!(sysreg, 1);
                assert_eq!(rt, 19);
                assert!(read);
            }
            other => panic!("mrs cntfrq_el0 -> {other:?}"),
        }
        // mrs x25, cntvct_el0 (0xd53be059) => SysReg cntvct (sysreg==3).
        match decode(0xd53be059) {
            Inst::SysReg { sysreg, read, .. } => {
                assert_eq!(sysreg, 3);
                assert!(read);
            }
            other => panic!("mrs cntvct_el0 -> {other:?}"),
        }
        // fmov d6, d0 = 0x1e604006 (real libroblox) => register FP copy.
        match decode(0x1e604006) {
            Inst::FmovFp { rd, rn, sz } => {
                assert_eq!(rd, 6);
                assert_eq!(rn, 0);
                assert!(sz); // double
            }
            other => panic!("fmov d6,d0 -> {other:?}"),
        }
        // fcmp d7, d6 = 0x1e6620e0 (real libroblox audio loop) => Fcmp sets NZCV.
        match decode(0x1e6620e0) {
            Inst::Fcmp { rn, rm } => {
                assert_eq!(rn, 7);
                assert_eq!(rm, 6);
            }
            other => panic!("fcmp d7,d6 -> {other:?}"),
        }
        // fcmp d6, d16 = 0x1e7020c0 (real libroblox; high rm reg folded into the
        // base nibble) → must still decode as Fcmp with rm=16.
        match decode(0x1e7020c0) {
            Inst::Fcmp { rn, rm } => {
                assert_eq!(rn, 6);
                assert_eq!(rm, 16);
            }
            other => panic!("fcmp d6,d16 -> {other:?}"),
        }
        // fcsel d6, d16, d6, mi = 0x1e664e06 (real libroblox) => conditional FP select.
        match decode(0x1e664e06) {
            Inst::FcsSel { rd, rn, rm, cond, sz } => {
                assert_eq!(rd, 6);
                assert_eq!(rn, 16);
                assert_eq!(rm, 6);
                assert_eq!(cond, 0x4); // mi
                assert!(sz);
            }
            other => panic!("fcsel d6,d16,d6,mi -> {other:?}"),
                    }
                    // fsqrt d1, d1 = 0x1e61c021 (real libroblox audio mix) => FpUnary op0.
                    match decode(0x1e61c021) {
                        Inst::FpUnary { rd, rn, op, sz } => {
                            assert_eq!(rd, 1);
                            assert_eq!(rn, 1);
                            assert_eq!(op, 0); // fsqrt
                            assert!(sz);
                        }
                        other => panic!("fsqrt d1,d1 -> {other:?}"),
                    }
                    // frintm d3, d3 = 0x1e654063 (round toward -inf) => FpUnary op1.
                    match decode(0x1e654063) {
                        Inst::FpUnary { rd, rn, op, sz } => {
                            assert_eq!(rd, 3);
                            assert_eq!(rn, 3);
                            assert_eq!(op, 1); // frintm
                            assert!(sz);
                        }
                        other => panic!("frintm d3,d3 -> {other:?}"),
                    }
                    // fmov d6,d0 = 0x1e604006 must still be FmovFp (NOT FpUnary/frintm).
                            assert!(matches!(decode(0x1e604006), Inst::FmovFp { rd: 6, rn: 0, .. }));
                            // ucvtf v2.2d, v2.2d = 0x6e61d842 (real libroblox audio mix) => Ucvtf2d.
                            match decode(0x6e61d842) {
                                Inst::Ucvtf2d { rd, rn } => {
                                    assert_eq!(rd, 2);
                                    assert_eq!(rn, 2);
                                }
                                other => panic!("ucvtf v2.2d -> {other:?}"),
                            }
                            // ucvtf v31.2d, v31.2d = 0x6e61dbff (compiler-emitted) => Ucvtf2d.
                            match decode(0x6e61dbff) {
                                Inst::Ucvtf2d { rd, rn } => {
                                    assert_eq!(rd, 31);
                                    assert_eq!(rn, 31);
                                }
                                other => panic!("ucvtf v31.2d -> {other:?}"),
                            }
                            // scvtf d31,d31 = 0x5e61db9c (signed FP->int) must NOT decode as Ucvtf2d.
                                    assert!(!matches!(decode(0x5e61db9c), Inst::Ucvtf2d { .. }));
                                    // dup v4.2d, v2.d[1] = 0x4e180444 (real libroblox audio mix) => SimdDupD.
                                    match decode(0x4e180444) {
                                        Inst::SimdDupD { rd, rn, index } => {
                                            assert_eq!(rd, 4);
                                            assert_eq!(rn, 2);
                                            assert_eq!(index, 1);
                                        }
                                        other => panic!("dup v4.2d,v2.d[1] -> {other:?}"),
                                    }
                                    // // ins v2.d[1], v0.d[0] = 0x6e180402 (real) must stay InsD1D0 (not SimdDupD).
        assert!(matches!(decode(0x6e180402), Inst::InsD1D0 { .. }));
        // fabd d3, d8, d1 = 0x7ee1d503 (real libroblox audio mix) => Fabd |d8-d1|.
        match decode(0x7ee1d503) {
            Inst::Fabd { rd, rn, rm } => {
                assert_eq!(rd, 3);
                assert_eq!(rn, 8);
                assert_eq!(rm, 1);
            }
            other => panic!("fabd d3,d8,d1 -> {other:?}"),
        }
        // fabd d0,d0,d1 = 0x7ee1d400 (compiler) => Fabd.
        assert!(matches!(decode(0x7ee1d400), Inst::Fabd { rd: 0, rn: 0, rm: 1 }));
        // dup v1.4s, w10 = 0x4e040d41 (real libroblox audio mix channel loop) => SimdDupSReg.
        match decode(0x4e040d41) {
            Inst::SimdDupSReg { rd, rn } => {
                assert_eq!(rd, 1);
                assert_eq!(rn, 10);
            }
            other => panic!("dup v1.4s,w10 -> {other:?}"),
        }
        match decode(0x1e6c1001) {
            Inst::FmovImm {
                rd,
                f64,
                value_bits,
            } => {
                assert_eq!(rd, 1);
                assert!(f64);
                assert_eq!(value_bits, 0x3fe0_0000_0000_0000); // 0.5 double
            }
            other => panic!("fmov d1,#0.5 -> {other:?}"),
        }
        // fmov d0, #2.0 (0x1e601000) => f64 2.0
        match decode(0x1e601000) {
            Inst::FmovImm { value_bits, .. } => {
                assert_eq!(value_bits, 0x4000_0000_0000_0000); // 2.0 double
            }
            other => panic!("fmov d0,#2.0 -> {other:?}"),
        }
    }

    #[test]
    fn mov_single_f64_bits() {
        // decode_fmov_imm: 0.5 -> 0x3fe0...  , 1.0 -> 0x3ff0...,  -2.0 -> 0xc000...
        assert_eq!(decode_fmov_imm(0x60, true), 0x3fe0_0000_0000_0000); // 0.5
        assert_eq!(decode_fmov_imm(0x70, true), 0x3ff0_0000_0000_0000); // 1.0
        assert_eq!(decode_fmov_imm(0x80, true), 0xc000_0000_0000_0000); // -2.0
    }
}
