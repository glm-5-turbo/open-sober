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
    // ---- add/subtract with carry: adc/sbc/adcs/sbcs Xd, Xn, Xm ----
    AddCarry {
        rd: u8,
        rn: u8,
        rm: u8,
        sub: bool,
        sf: bool,
        s: bool,
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
        // Sign-extending load (ldrsw/ldrsh/ldrsb): bit23=1 + bit22=0, which the
        // naive `ld=bit22` read as a store. When set, this is a LOAD that
        // sign-extends the loaded size into the 64-bit dest X-register.
        sext: bool,
    },
    // ---- load/store (register offset) ----
    LdStrReg {
        rt: u8,
        rn: u8,
        rm: u8,
        size: u8,
        ld: bool,
        shift: bool, // S bit: scaled by element size
        // Sign-extending register-offset load (ldrsw/ldrsh/ldrsb): bit23=1 +
        // bit22=0, misread by `ld=bit22` as a store.
        sext: bool,
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
        sext: bool,    // true => ldpsw (sign-extend 32-bit loads to 64-bit X regs)
    },
    // ---- SIMD/NEON 128-bit vector load/store (ldr q0,[xN,#imm] / str q) ----
    VecLdStImm {
        vt: u8, // vector register
        rn: u8,
        imm: u32, // scaled-by-16 byte offset
        ld: bool,
    },
    // ---- FP/SIMD scalar-register load/store (ldr/str d0,s0,h0,b0,[xN,#imm]) ----
    // bit26=1 selects the vector/FP register file; width = size (1/2/4/8 bytes:
    // B/H/S/D). Unlike a GPR store there is no XZR quirk — all 32 vector regs
    // are writable — so rt maps to CpuState.v[vt] (16-byte slot).
    FpLdStImm {
        vt: u8,
        rn: u8,
        imm: u32, // scaled-by-size byte offset
        size: u8, // 1/2/4/8
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
    // ---- SIMD load-and-replicate: ld1r {Vt.T}, [Xn] ----
    Ld1 { rd: u8, rn: u8, esize: u8, q: bool }, // lanes = (q?16:8)/esize
    // ---- SIMD lane load: ld1 {Vt.T}[idx], [Xn] ----
    Ld1L { rd: u8, rn: u8, esize: u8, index: u8 },
    // ---- SIMD single-vector load: ld1 {Vt.T}, [Xn], #imm ----
    Ld1V { rd: u8, rn: u8, bytes: i32 },
    // ---- SIMD single-vector store: st1 {Vt.T}, [Xn], #imm ----
    St1V { rd: u8, rn: u8, bytes: i32 },
    // ---- SIMD widen/long load: uxtl/sxtl Vd.TL, Vn.T (sign/zero extend) ----
    SimdXtl { rd: u8, rn: u8, sign: bool, esrc: u8 },
    // ---- SIMD add/sub-wide: uaddw/saddw Vd.T, Vn.T, Vm.T/2 ----
    SimdAddw { rd: u8, rn: u8, rm: u8, sign: bool, esrc: u8 },
    // ---- SIMD vector bitwise AND/ORR/EOR/BIC (128b lanes) ----
    SimdVLog { rd: u8, rn: u8, rm: u8, op: u8 },
    // ---- SIMD bitwise select: bsl/bit/bif Vd.128 (op 0/1/2) ----
    SimdSel { rd: u8, rn: u8, rm: u8, op: u8 },
    // ---- SIMD bitwise NOT (two-input mvn alias, single-source): mvn Vd.16B/8B, Vn ----
        SimdNot { rd: u8, rn: u8 },
        // ---- SIMD add/sub-high narrow: addhn/subhn Vd.T, Vn.W, Vm.W (dst = low half of
        //     the high half of the src-width sum/diff; raddhn rounds). ---
        SimdHighNarrow { rd: u8, rn: u8, rm: u8, dst_esize: u8, sub: bool, round: bool },
    // ---- SIMD shift-left immediate: shl Vd.T, Vn.T, #imm ----
    SimdShl { rd: u8, rn: u8, esize: u8, shift: u8 },
    // ---- SIMD shift-right accumulate: usra/ssra Vd.T, Vn.T, #imm (Vd += Vn >> imm) ----
    SimdShrAcc { rd: u8, rn: u8, esize: u8, shift: u8, unsigned: bool },
    // ---- SIMD ld2 (load two vectors, deinterleaved) ----
    Ld2 { rd: u8, rn: u8, q: bool, post: i32 },
    // ---- SIMD st2 (structure store of two vectors) ----
    St2 { rd: u8, rn: u8, q: bool, post: i32 },
    // ---- scalar udiv/sdiv Wd/Wd/Wm ----
    Div { rd: u8, rn: u8, rm: u8, signed: bool, is_x: bool },
    // ---- SIMD variable register shift: ushl/sshl Vd.T, Vn.T, Vm.T ----
    SimdVShift { rd: u8, rn: u8, rm: u8, esize: u8, signed_: bool },
    // ---- scalar FP max/min (fmax/fmin/fmaxnm/fminnm) ----
    FMaxMin { rd: u8, rn: u8, rm: u8, sz: bool, op: u8 },
    // ---- FP horizontal reduction cross vector: fmaxv/fminv Sd, Vn.4s ----
    FMaxV { rd: u8, rn: u8, min: bool },
    // ---- switchable FP multiply-accumulate: fmla/fmls Vd.4s/.2s/.2d, Vn, Vm ----
    Fmla {
        rd: u8,
        rn: u8,
        rm: u8,
        // el64: .2d double; q: .4s/.2d (high) width vs .2s
        el64: bool,
        q: bool,
        sub: bool,
    },
    // ---- FP by-element multiply-accumulate: fmla/fmls Vd.T, Vn, Vm.Ts[idx] ----
    FmlaEl {
        rd: u8,
        rn: u8,
        vlm: u8,
        idx: u8,
        el64: bool,
        q: bool,
        sub: bool,
    },
    // ---- SIMD widening shift-left (sign/zero extend): shll/usll Vd.Td, Vn.Ts ----
        WidenShl { rd: u8, rn: u8, dst_esize: u8, nlanes: u8, signed: bool, upper: bool },
        // ---- SIMD add/sub-long widening: saddl/uaddl/subl/usubl Vd.T, Vn.T, Vm.T ----
            SimdAddl { rd: u8, rn: u8, rm: u8, esrc: u8, sign: bool, sub: bool, upper: bool },
    // ---- SIMD add-adjacent-long pairwise accumulate: sadalp/uadalp Vd.Td, Vn.Ts ----
    SimdAdalp { rd: u8, rn: u8, src_esize: u8, n_pairs: u8, signed: bool },
    // ---- SIMD saturating add/sub: sqadd/uqadd/sqsub/uqsub Vd.T, Vn, Vm ----
    SimdSatAdd { rd: u8, rn: u8, rm: u8, esize: u8, sub: bool, unsigned: bool, q: bool },
    // ---- scalar FP multiply / negate-multiply: fmul/fnmul Sd/Dd, Sn, Sm ----
    FmulScalar { rd: u8, rn: u8, rm: u8, double: bool, neg: bool },
    // ---- SIMD signed/unsigned integer min/max: smin/smax/umin/umax Vd.T, Vn, Vm ----
    SminMax { rd: u8, rn: u8, rm: u8, max: bool, unsigned: bool, esize: u8, q: bool },
    // ---- scalar fixpoint int->FP (ucvtf/scvtf Dd/Xn,#fbits or Sd/Wn,#fbits) ----
    ScvtfFixed { rd: u8, rn: u8, to_double: bool, sf: bool, unsigned: bool, fbits: u8 },
    // ---- SIMD element copy (vector, 64-bit lane): mov Vd.d[i], Vn.d[j] ----
        SimdInsD { rd: u8, rn: u8, dst_idx: u8, src_idx: u8, esize: u8 },
        // ---- SIMD fp mul by element: fmul Vd.T, Vn.T, Vm.T[L] ----
        SimdFmulEl { rd: u8, rn: u8, rm: u8, esize: u8, index: u8, q: bool },
    // ---- SIMD byte reverse (rev64/rev32): reverse bytes within each granule ----
    SimdRev { rd: u8, rn: u8, granule: u8, q: bool },
    // ---- SIMD lane extract to FP reg: mov Sd/Dd, Vn.T[idx] ----
    SimdLaneS { rd: u8, rn: u8, esize: u8, index: u8 },
    // ---- SHA-1/SHA-256 crypto ops (mode: 1=sha1h,2=sha1c,3=sha1p,4=sha1m,
//        5=sha256h,6=sha1su0,7=sha1su1,8=sha256su0,9=sha256su1) ----
    Sha { mode: u8, rd: u8, rn: u8, rm: u8 },
    // ---- SIMD dup (vector, element): dup Vd.T, Vn.T[i] ----
    SimDup { rd: u8, rn: u8, esize: u8, src_idx: u8, q: bool },
    // ---- SIMD vector immediate: fmov Vd.T, #imm ----
    SimdFmovImm { rd: u8, esize: u8, value_bits: u64, q: bool },
    // ---- SIMD float-to-int (vector): fcvtzu/fcvtzs Vd.T, Vn.T ----
    FcvVec { rd: u8, rn: u8, signed: bool, esize: u8, q: bool },
    // ---- variable shift by register (LSLV/LSRV/ASRV/RORV) ----
    VarShiftVar { rd: u8, rn: u8, rm: u8, op: u8, sf: bool },
    // ---- SIMD FP unary: fneg/fabs/fsqrt Vd.T, Vn.T - op 0=neg 1=abs 2=sqrt ----
    SimdFpUnary { rd: u8, rn: u8, op: u8, esize: u8, q: bool },
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
           rn: u8,     // source fp reg
           mode: u8,   // 0=fcvtzs(trunc), 1=fcvtzu(uns-trunc), 2=fcvtas(nearest),
                       // 3=fcvtpu/ps (+inf round), 4=fcvtmu/ms (-inf round)
           sf: bool,   // 64-bit dest (X)
           unsigned: bool, // unsigned result (uclamp negatives to 0 / u64 result)
           src_sng: bool, // source is single (S) not double (D)
       },
        // ---- FP convert from signed integer (scvtf: Wn|Xn -> Sd|Dd) ----
        Scvtf {
            rd: u8,          // destination FP reg
            rn: u8,          // source integer reg
            to_double: bool, // true => Dd (double), false => Sd (single)
            sf: bool,        // true => 64-bit source reg (Rn), false => 32-bit (Wn)
            unsigned: bool,  // true => ucvtf (unsigned int), false => scvtf (signed)
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
                                   sz: bool, // true = double (fcmp Dn,Dm), false = single (fcmp Sn,Sm)
                               },
    // ---- scalar FP conditional select: fcsel Dd, Dn, Dm, <cond> ----
            FcsSel {
                rd: u8,   // destination FP reg
                rn: u8,   // "if-true" FP reg
                rm: u8,   // "else" FP reg
                cond: u8, // AArch64 condition code (0-14)
                sz: bool, // true = double (8B), false = single (4B)
            },
            // ---- scalar FP conditional compare: fccmp Dn, Dm, #nzcv, <cond> ----
            Fccmp {
                rn: u8,
                rm: u8,
                nzcv: u8, // NZCV set when cond is false
                cond: u8, // AArch64 condition code
                sz: bool, // true = double (fcmp Dn,Dm), false = single
            },
            // ---- scalar FP->int to FP reg: fcvtzs/fcvtzu Dd,Dn / Sd,Sn (trunc toward zero) ----
            FcvtTzReg { rd: u8, rn: u8, dbl: bool, unsigned: bool },
        // ---- NEON: mov Vd.D[1], Vn.D[0] (dup low 64 into the high 64 lane) ----
                           InsD1D0 { rd: u8, rn: u8 }, // v16B: slot_hi(8B) = low-64-of-Vn
                           // ---- EXTR / ROR rotate: rm==rn in the EXTR base ----
                           Ror { rd: u8, rn: u8, rot: u32, sf: bool }, // ror rd,rn,#rot
                           // ---- supervisor call (svc #imm) -> host syscall routing ----
                           Svc { imm: u16 },
                           // ---- breakpoint (brk #imm) -> guest trap; JIT halts gracefully
                               //       (a real AArch64 CPU would take a SIGTRAP/exception here) ----
                               Brk { imm: u16 },
                               // ---- undefined instruction (udf #imm = 0x00000000 | imm) -> guest abort
                               //       trap; a real AArch64 CPU raises UndefinedInstruction. JIT halts like Brk ----
                               Udf { imm: u16 },
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
    // ---- scalar unsigned int64->double: ucvtf Dd, Dn (int in Dn -> double) ----
        // Gate (insn & 0xffe0_fc00) == 0x7e60_d800, disjoint from the vector Ucvtf2d
        // (0x6e60_d800, bit23 differs), Fabd (0x7ee0_d400) and fmov (0x1e604000).
        // Reads Dn's low 64 bits as an unsigned integer, writes the double to Dd.
        ScalarUcvtf { rd: u8, rn: u8, sng: bool },
        // ---- scalar signed int64->double from a vector sub-reg: scvtf Dd, Dn ----
        ScalarScvtf { rd: u8, rn: u8, sng: bool },
    // Both 64-bit lanes of Vd get Vn's selected lane. Gate
    // (insn & 0xffff_fc00)==0x4e180400 (the Q=1 vector dup-d; distinct from the
    // 0x6e18:0x4e18 ins-variant). index in bit 16 (`[.../inst]` D[0] vs D[1]).
    SimdDupD { rd: u8, rn: u8, index: u8 },
    // ---- SIMD 2xdouble FP arithmetic: op Vd.2D, Vn.2D, Vm.2D (lanewise) ----
    // op 0=fdiv,1=fmul,2=fadd,3=fsub. Gate mask 0xffe0_fc00 gives the
    // per-op constants {0x6e60fc00,0x6e60dc00,0x4e60d400,0x4ee0d400}.
    Simd2dFp { rd: u8, rn: u8, rm: u8, op: u8 },
    // ---- SIMD dup from a GPR: dup Vd.T, Wn/Xn (broadcast the element read from the
    // GPR into Vd's lanes). Gate (insn & 0xff00_fc00)==0x0e00_0c00(q=0)/0x4e00_0c00(q=1);
    // element size from imm5 trailing-zeros in (insn>>16)&0x1f. Covers 8b/16b/4h/8h/2s/4s/2d.
    SimdDupGp { rd: u8, rn: u8, esize: u8, q: bool },
    // ---- SIMD 16-byte logical OR: orr Vd.16B, Vn.16B, Vm.16B ----
    // Gate (insn & 0xffe0_fc00)==0x0ea01c00 (also the `mov Vd.16B,Vn.16B` copy
    // alias rm==rn, e.g. real 0x4ea01c02). ORs the full 16-byte vector slot.
    SimdOrr16 { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD 32-bit lane multiply: mul Vd.4S/Vd.2S, Vn., Vm. ----
    // 4S gate (Q=1) 0x4ea09c00 ; 2S gate (Q=0) 0x0ea09c00. Per-lane low-32 product.
    SimdMul { rd: u8, rn: u8, rm: u8, lanes: u8 },
    // ---- SIMD widening multiply: smull/umull Vd.T, Vn.T, Vm.T (src*U res) ----
        // Gate (insn & 0x0f00_c000): 0x0e00_c000 = mull, 0x0e00_8000 = mlal (accumulate).
        // unsigned=bit28; res_esize 4(.4s from .4h) or 8(.2d from .2s) by bit22; q=bit30.
        SimdMull { rd: u8, rn: u8, rm: u8, res_esize: u8, unsigned: bool, q: bool, acc: bool },
    // ---- SIMD unsigned compare-higher: cmhi Vd.4S, Vn.4S, Vm.4S ----
    // Gate (insn & 0xffe0_fc00)==0x6ea0c000 (verified vs real 0x6ea4c1c1).
    // Lane => all-ones if Vn[i] > Vm[i] (unsigned), else 0.
    SimdCmhi { rd: u8, rn: u8, rm: u8, lanes: u8 },
    // ---- SIMD unsigned compare-higher 2D: cmhi Vd.2D, Vn.2D, Vm.2D ----
    SimdCmhiD { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD signed compare-greater: cmgt Vd.T, Vn.T, Vm.T (per 32/64-bit lane) ----
    // Gate 0x4ea0_3400(.4s q1)/0x0ea0_3400(.2s q0)/0x4ee0_3400(.2d); lanes 4/2/2.
    SimdCmgt { rd: u8, rn: u8, rm: u8, lanes: u8, dword: bool },
    // ---- SIMD unzip even: uzp1 Vd.T, Vn.T, Vm.T ----
    SimdUz1 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD zip even: zip1 Vd.T, Vn.T, Vm.T ----
    SimdZip1 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD element extract to GPR: umov/smov Rd, Vn.bits[idx] ----
    SimdMovEl { rd: u8, rn: u8, esize: u8, index: u8, signed: bool, is_x: bool },
    // ---- SIMD integer add/sub 2D (64-bit lanes): add Vd.2D, Vn.2D, Vm.2D ----
    SimdAddD { rd: u8, rn: u8, rm: u8, sub: bool },
    // ---- SIMD int add/sub byte lanes 16B/8B: add Vd.16b, Vn.16b, Vm.16b ----
    SimdAddB { rd: u8, rn: u8, rm: u8, sub: bool, q: bool },
    // ---- SIMD int add/sub halfword 8H/4H (16-bit) lanes: add Vd.8h, Vn.8h, Vm.8h ----
    SimdAddH { rd: u8, rn: u8, rm: u8, sub: bool, q: bool },
    // ---- SIMD compare equal: cmeq Vd.T, Vn.T, Vm.T ----
    SimdCmEq { rd: u8, rn: u8, rm: u8, lanes: u8, esize: u8 },
    // ---- SIMD compare (nonzero) test: cmtst Vd.T, Vn.T, Vm.T ----
        SimdCmTest { rd: u8, rn: u8, rm: u8, lanes: u8, esize: u8 },
        // ---- SIMD table lookup: tbl Vd.16B, {Vn..Vn+N}, Vm (N+1 regs, N<=3) ----
        Tbl { rd: u8, rn: u8, rm: u8, tbx: bool, n_tables: u8 },
    // ---- SIMD narrowing extract: xtn Vd.T, Vn.U (low halves) ----
    SimdXtn { rd: u8, rn: u8, dst_esize: u8 }, // lanes = 8/dst_esize (Q=0)
    // ---- SIMD saturating narrowing: sqxtn/uqxtn/sqxtun/uqxtun Vd.T, Vn.U ----
    // Gate: prefix {0x0e,0x2e,0x4e,0x6e} + byte2 {0x28 (x.un/u-un), 0x48 (xtn)}.
    // dst_esize 1(b from h) or 2(h from s) by b1 bit6; src_signed=bit29 clear/unsigned etc.
    SaturatNarrow { rd: u8, rn: u8, dst_esize: u8, src_signed: bool, dst_signed: bool, q: bool },
    // ---- SIMD bitwise insert: bit Vd.16B, Vn.16B, Vm.16B ----
    // Gate (insn & 0xffe0_fc00)==0x6ea01c00 (16B bit-select, real 0x6ea11c40;
    // distinct from orr16 0x4ea01c00 by bit31). Out = (Vn & Vm) | (Vd & ~Vm).
    SimdBit { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD extract immediate: ext Vd.16B/Vd.8B, Vn., Vm., #imm ----
    // Byte-shift extract. Gate (insn & 0xffe0_0400) == 0x6e000000 (16B, Q=1) /
    // 0x2e000000 (8B, Q=0); imm = bits[15:11] (byte count, 0..15 for 16B,
    // 0..7 for 8B). Semantics: bytes of Vd = the 128(64)-bit window of the
    // concatenation {Vn(high), Vm(low)} starting at byte `imm`. For Vn==Vm and
    // imm=8 this is the classic 64-bit half-swap. `q` = has Q (16B) set.
    SimdExt { rd: u8, rn: u8, rm: u8, imm: u8, q: bool },
        // ---- SIMD lane extract to GPR: mov/umov/smov Wd,Xd, Vn.T[idx] ----
    // Copies an element (esize bytes) of vector lane into a GPR, zero- (umov/mov)
    // or sign- (smov) extended. Gate (insn & 0xffe0_0c00) in {0x0e000c00 (Wd dest),
    // 0x4e000c00 (Xd dest)}. esize via imm5 trailing-zeros (p=ctz(imm5)+1 gives
    // esize=1<<(p-1)); index = imm5>>p. sign flag = (bit12 cleared). Distinct
    // real ops (orr16 0x4ea41c40, mul 0x0ea09c00, InsDv1D0 0x4e18400) verified
    // not to fall under this mask.
    SimdLaneGp { rd: u8, rn: u8, esize: u8, index: u8, sign: bool, wide: bool },
                           // ---- bitfield (UBFM/SBFM): decoded to the lsr/lsl/asr and extraction aliases ----
           BitField {
        rd: u8,
        rn: u8,
        immr: u32,
        imms: u32,
        sf: bool,   // 64-bit
        arith: bool, // true = arithmetic shift (SBFM/asr) sign-extends
        insert: bool, // true = BFM insert (opc=00, immr<=imms): merge field into Rd
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
    // ---- count leading zeros / sign bits: clz Wd,Xd,Rn ; cls Wd,Xd,Rn ----
    ClzCls {
        rd: u8,
        rn: u8,
        sf: bool,     // 64-bit operand
        cls: bool,    // true = CLS (count leading sign bits), false = CLZ
    },
    // ---- byte/bit reverse: rbit/rev16/rev/rev32 (0x5ac0/0xdac0 row) ----
    Rev {
        rd: u8,
        rn: u8,
        op: u8,  // 0=rbit, 1=rev16, 2=rev, 3=rev32(X only)
        sf: bool, // 64-bit operand
    },
    // ---- HINT / PAC NOP (nop, yield, esb, csdb, paciasp, autiasp, bti, ...) ----
    // Dealt with as a no-op for execution (PAC is ignored in the guest).
    Hint,
    // ---- memory/DMB/DSB/ISB barrier (no-op in the single-threaded JIT) ----
    WaitBarrier,
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
    // ---- SIMD vector immediate: fmov Vd.T, #imm ----
    // Disjoint gate (15 asm-verified): Q/esz prefix in {0x0f,2f,4f,6f}00_0000
    // and fixed low bits 15-10 == 0xF400. imm8 = [5..9]|[16..18]<<5. MUST precede
    // the broad MOVI/vector-imm gate (which also claims 0x6f), so it is first here.
    if ((insn & 0xffe0_0000) == 0x0f00_0000 || (insn & 0xffe0_0000) == 0x2f00_0000
    || (insn & 0xffe0_0000) == 0x4f00_0000 || (insn & 0xffe0_0000) == 0x6f00_0000)
    && (insn & 0x0000_f400) == 0x0000_f400 {
        let esize = if (insn >> 29) & 1 == 0 { 4u8 } else { 8u8 };
        let q = (insn >> 30) & 1 == 1;
        let imm8 = ((insn >> 5) & 0x1f) | (((insn >> 16) & 0x7) << 5);
        let value_bits = decode_fmov_imm(imm8, esize == 8);
        return Inst::SimdFmovImm { rd: (insn & 0x1f) as u8, esize, value_bits, q };
    }
    // ---- scalar FP-to-int into an FP register: fcvtzs/fcvtzu Dd, Dn / Sd, Sn ----
    // Converts the FP value in Vn to an integer stored back into a vector reg.
    // Scalar D form (double -> 64-bit int): residues 0x5ee0_b800 (fcvtzs, signed)
    // / 0x7ee0_b800 (fcvtzu, unsigned). One 64-bit lane, q=false — matches the
    // FcvVec translate with esize=8 (cvttsd2si). Must precede both the FcvVec
    // vector gate and the SIMD widen gate (which wrongly matched 0x5ee1bbff).
    if (insn & 0xffe0_fc00) == 0x5ee0_b800 || (insn & 0xffe0_fc00) == 0x7ee0_b800 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let signed = (insn >> 29) & 1 == 0;
        return Inst::FcvVec { rd, rn, signed, esize: 8, q: false };
    }
    // ---- SIMD float-to-int (vector): fcvtzu/fcvtzs Vd.T, Vn.T (FPI(FPc))----
    if matches!(insn & 0xffe0_fc00, 0x0ea0_b800 | 0x2ea0_b800 | 0x4ea0_b800 | 0x4ee0_b800 | 0x6ea0_b800 | 0x6ee0_b800) {
        // esize discriminator is bit22: .2d (imm-64) has it set, .4s/.2s clear —
        // e.g. fcvtzs v0.2d=0x4ee1b820 vs fcvtzs v0.4s=0x4ea1b820 differ by
        // 0x400000 (bit22). (bit20 does NOT distinguish: both 0x4ee1b820 and
        // 0x4ea1b820 have bit20=0, so the old `(insn>>20)&1` mis-decoded .2d as
        // .4s and corrupted every double lane.)
        let e = if (insn >> 22) & 1 == 1 { 8u8 } else { 4u8 };
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let signed = (insn >> 29) & 1 == 0;
        let q = (insn >> 30) & 1 == 1;
        return Inst::FcvVec { rd, rn, signed, esize: e, q };
    }
    // ---- variable shift by register: lslv/lsrv/asrv/rorv Wd|Xd, Wn|Xn, Wm|Xm ----
    // Gate (insn & 0xffe0_2000) in {0x1ac0_2000, 0x9ac0_2000}; distinct from MulDiv
    // (0x1ac0_0000). op = bits[12:10]: 0=lsl, 1=lsr, 2=asr, 3=ror.
    if (insn & 0xffe0_2000) == 0x1ac0_2000 || (insn & 0xffe0_2000) == 0x9ac0_2000 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let op = ((insn >> 10) & 3) as u8;
        let sf = (insn >> 31) & 1 == 1;
        return Inst::VarShiftVar { rd, rn, rm, op, sf };
    }
    // ---- SIMD FP unary: fneg/fabs/fsqrt Vd.T, Vn.T (2D/4S/2S) ----
    // Gate (insn & 0xffe0_f800) in {0x2ea0,0x4ea0,0x4ee0,0x6ea0,0x6ee0}_f800.
    // op: abs (bit29==0), else sqrt if bit16 else neg. esize = 8 iff bit22.
    if (insn & 0xffe0_f800) == 0x2ea0_f800 || (insn & 0xffe0_f800) == 0x4ea0_f800
        || (insn & 0xffe0_f800) == 0x4ee0_f800 || (insn & 0xffe0_f800) == 0x6ea0_f800
        || (insn & 0xffe0_f800) == 0x6ee0_f800
    {
        let op = if (insn >> 29) & 1 == 0 {
            1 // fabs
        } else if (insn >> 16) & 1 == 1 {
            2 // fsqrt
        } else {
            0 // fneg
        };
        let esize = if (insn >> 22) & 1 == 1 { 8u8 } else { 4u8 };
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let q = (insn >> 30) & 1 == 1;
        return Inst::SimdFpUnary { rd, rn, op, esize, q };
    }
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

    // ---- MoveWide (MOVZ/MOVK/MOVN): identify by bits[28:23]==0x25 (100101),
    // i.e. (insn & 0x1f80_0000) == 0x1280_0000. NOTE: the LogicalImmediate family
    // (AND/ORR/EOR/ANDS with immediate) has bits[28:23]==0x24, so a loose `top`
    // byte match {0x12,0x92,0x52,0xD2,...} swallowed `and w1,w0,#0xffff` (0x12003c01)
    // as a `movn` — every AND/EOR/ANDS-immediate broke. Moving it before the
    // MoveWide gate so AND-immediate decodes as LogicImm.
    let movewide = insn & 0x1f80_0000 == 0x1280_0000;
    if movewide && matches!((insn >> 24), 0x12 | 0x52 | 0x72 | 0x92 | 0xD2 | 0xF2) {
        // opc: bit30 (sf=0 uses bit29=m... ). Derive opcode from identifier bits:
        //   -(sf, opc2) : movz <-> opc=0, movk: opc=1, movn: opc=2
        // Use displacement: the top-nibble distinguishes mov | k | n via bit3.
        let opc = match (insn >> 24) & 0xF0 {
            0xD0 | 0x50 => 0, // movz
            0xF0 | 0x70 => 1, // movk
            _ => 2,           // movn
        };
        let hw = b(insn, 21, 22) as u8;
        let imm16 = (insn >> 5) as u16;
        let rd = rd(insn);
        return Inst::MoveWide {
            rd,
            imm16,
            hw,
            opc,
            sf: insn >> 31 == 1,
        };
    }

    // `top` (byte 31:24) and `sf` (bit 31) are reused by several later decoders
    // (add/sub immediate+reg, CSEL, ...).
    let top = insn >> 24;
    let sf = (insn >> 31) & 1 == 1;

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

    // ---- add/subtract (shifted register) ----  REAL tops only.
    // 0x0b/0x2b(add W, ±S) 0x4b/0x6b(sub W) 0x8b/0xab(add X) 0xcb/0xeb(sub X).
    // NOTE: 0x1b/0x9b are MADD/MSUB/MUL (0x9b007c20 = `mul x0,x1,x0`) and 0x3b/
    // 0xbb are multiply-family too — they must NOT be caught here or every
    // multiply decodes as a spurious `add ...,lsl #N` (verified: madd came out
    // AddSubReg lsl#31). They fall through to the MulDiv gate below.
    if matches!(
        top,
        0x0b | 0x2b | 0x4b | 0x6b | 0x8b | 0xab | 0xcb | 0xeb
    ) {
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

    // ---- add/subtract with carry: adc/sbc/adcs/sbcs Xd, Xn, Xm ----
    // Mask (insn & 0x1fe0_0000)==0x1a00_0000. Distinct from AddSubReg-shifted
    // (top 0x0b/0x1b/0x2b...), madd (0x1b00_0000) and CSEL (0x1a80_0000).
    if (insn & 0x1fe0_0000) == 0x1a00_0000 {
        let sub = b(insn, 30, 30) == 1;
        let s = b(insn, 29, 29) == 1;
        let rd = b(insn, 0, 4) as u8;
        let rn = b(insn, 5, 9) as u8;
        let rm = b(insn, 16, 20) as u8;
        let sf = b(insn, 31, 31) == 1;
        return Inst::AddCarry { rd, rn, rm, sf, s, sub };
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

    // ---- count leading zeros / leading sign bits: clz Wd,Xd, Rn ; cls Wd,Xd, Rn ----
    // clz W: mask 0x5ac0_1000, clz X: 0xdac0_1000 ; cls W: 0x5ac0_1400,
    // cls X: 0xdac0_1400 (mask 0xffff_fc00). sf=bit31. rn=bits5-9, rd=bits0-4.
    {
        let zz = insn & 0xffff_fc00;
        let clz_cls = match zz {
            0x5ac0_1000 => Some((false, false)), // clz W
            0xdac0_1000 => Some((true, false)),  // clz X
            0x5ac0_1400 => Some((false, true)),  // cls W
            0xdac0_1400 => Some((true, true)),   // cls X
            _ => None,
        };
        if let Some((sf, is_cls)) = clz_cls {
            let rd = b(insn, 0, 4) as u8;
            let rn = b(insn, 5, 9) as u8;
            return Inst::ClzCls { rd, rn, sf, cls: is_cls };
        }
    }

    // ---- byte/bit reverse: rbit/rev16/rev/rev32 (the 0x5ac0/0xdac0 row) ----
    // opcode = bits[11:10]: 00=rbit, 01=rev16, 10=rev, 11=rev32 (X only).
    // sf=bit31 (0x5ac0 => W, 0xdac0 => X). rn=bits5-9, rd=bits0-4.
    {
        let bb = insn & 0xffff_fc00;
        let rev_op = match bb {
            0x5ac0_0000 | 0xdac0_0000 => Some(0), // rbit
            0x5ac0_0400 | 0xdac0_0400 => Some(1), // rev16
            0x5ac0_0800 | 0xdac0_0800 => Some(2), // rev
            0xdac0_0c00 => Some(3),               // rev32 (X only)
            _ => None,
        };
        if let Some(op) = rev_op {
            let sf = (insn >> 31) & 1 == 1;
            let rd = b(insn, 0, 4) as u8;
            let rn = b(insn, 5, 9) as u8;
            return Inst::Rev { rd, rn, op, sf };
        }
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

    // ---- load/store (unsigned immediate offset) ---- GPR (W/X registers
    // only). The FP/SIMD register-file loads/stores (`str d/s/h/b`, `str q`)
    // share the size/addressing encodings but differ in bit26 (0x04000000):
    // here bit26 must be 0 (GPR). Enforce it in the mask so an FP-register
    // load/store is not mis-decoded as `str x` (which would read/write the GPR
    // x[rt] slot instead of the vector v[rt], corrupting FP/vector state). The
    // FP forms are routed below (VecLdStImm for 128-bit q, FpLdStImm for the
    // D/S/B/H scalars).
    if (insn & 0x3f00_0000) == 0x3900_0000 {
        let size = match (insn >> 30) & 0x3 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let ld = (insn >> 22) & 1 == 1;
        // Sign-extend load (ldrsw/ldrsh/ldrsb): opc[1]=bit23 is 1 for the 10/11
        // sign-extend forms and 0 for plain LDR(opc=01)/STR(opc=00). bit22 alone
        // is ambiguous (ldrsh w0,[x0] at 0x79c0 has bit22=1 yet IS sign-extend),
        // so key on bit23. Size-8 (X) never sets bit23 (only STR/LDR X exist).
        let sext = (insn & 0x80_0000) != 0;
        let ld = ld || sext;
        let imm = (insn >> 10) & 0xfff;
        let rn = b(insn, 5, 9) as u8;
        let rt = b(insn, 0, 4) as u8;
        return Inst::LdStrImm {
            rt,
            rn,
            imm,
            size,
            ld,
            sext,
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

    // ---- FP/SIMD scalar-register load/store (ldr/str d0,s0,h0,b0,[xN,#imm]) ----
    // bit26=1 (FP/vector file), bit25=0 (immediate-offset form). The 128-bit q
    // form was consumed above; the scalar B/H/S/D widths here map by size.
    if (insn & 0x3f00_0000) == 0x3d00_0000 {
        let size = match (insn >> 30) & 0x3 {
            0 => 1, // b
            1 => 2, // h
            2 => 4, // s
            _ => 8, // d
        };
        let ld = (insn >> 22) & 1 == 1;
        let imm = (insn >> 10) & 0xfff; // scaled by `size`
        let rn = b(insn, 5, 9) as u8;
        let vt = b(insn, 0, 4) as u8;
        return Inst::FpLdStImm {
            vt,
            rn,
            imm,
            size,
            ld,
        };
    }

    // ---- SIMD widen/long (sxtl/uxtl): Vd.TL, Vn.T ----
    // Gate &0x3f80_0c00 in {0x0f00_0400 (sxtl), 0x2f00_0400 (uxtl)}. Wider
    // result element = 2x source; esrc = 1<<(bits[13:11]) bytes. MUST precede the
    // broad MOVI gate (0x2f/0x0f prefix) or it's swallowed as Unsupported.
    let xt = insn & 0x3f80_0c00;
    if (xt == 0x0f00_0400 || xt == 0x2f00_0400) && ((insn >> 12) & 0xf) == 0xa {
        let esrc = 1u8 << ((insn >> 20) & 0x7); // 1/2/4-byte source lanes (B/H/..)
        return Inst::SimdXtl {
            rd: (insn & 0x1f) as u8,
            rn: b(insn, 5, 9) as u8,
            sign: xt == 0x0f00_0400,
            esrc,
        };
    }

    // ---- SIMD add/sub-wide: uaddw/saddw Vd.T, Vn.T, Vm.(T/2) ----
    // Gate: >>24 in {0x0e,0x2e,0x4e,0x6e} (u=bit29=1, s=0) with bit12=1 (wide not
    // long -- addl has bit12=0). Narrow source elem = 1<<((insn>>22)&3) (B/H/S);
    // dest element = 2x source; lanes = (8 / esrc).
    if matches!((insn >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e) && (insn & 0x0000_1800) == 0x0000_1000 && (insn & 0x0000_0c00) == 0 {
        let esrc = 1u8 << ((insn >> 22) & 0x3);
        return Inst::SimdAddw {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            sign: ((insn >> 29) & 1) == 0,
            esrc,
        };
    }

    // ---- SIMD vector bitwise AND/ORR/BIC (Vd.T = Vn.T op Vm.T) ----
    // Gate: prefix byte {0x0e,0x4e} (bit29=0 → and/orr/bic, NOT bit/bif/bsl which
    // are 0x6e-prefixed, and NOT eor which is 0x2e). `&0x0000_1c00==0x1c00`.
    // op = bit23(orr) | bit22(bic) | else and. Verified vs 16 asm forms.
    if matches!((insn >> 24) & 0x3f, 0x0e | 0x4e) && (insn & 0x0000_1c00) == 0x1c00 {
        let op = if (insn >> 23) & 1 == 1 { 2 } // ORR
        else if (insn >> 22) & 1 == 1 { 3 } // BIC
        else { 0 };                          // AND
        return Inst::SimdVLog {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            op,
        };
    }

    // ---- SIMD vector EOR (Vd.128 = Vn.128 ^ Vm.128) ----
    // Encoded with prefix {0x2e,0x6e} (bit29=1, unlike and/orr/bic). Disjoint
    // from bit/bif/bsl sel ops which also use 0x6e but set a select bit in
    // 0x00c0_0000 (bif=bit23,bsl=bit22,bit=both); EOR/have NEITHER set.
    if matches!((insn >> 24) & 0x3f, 0x2e | 0x6e)
        && (insn & 0x0000_1c00) == 0x1c00
        && (insn & 0x00c0_0000) == 0
    {
        return Inst::SimdVLog {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            op: 1, // EOR
        };
    }

    // ---- SIMD bitwise select BSL only (Vd = (Vd&Vn)|(~Vd&Vm)); bit/bif handled by SimdBit ----
    if matches!((insn >> 24) & 0x3f, 0x2e | 0x6e) && (insn & 0x0000_1c00) == 0x1c00 && (insn & 0x0040_0000) != 0 {
        return Inst::SimdSel {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            op: 0,
        };
    }

    // ---- SIMD bitwise NOT: mvn Vd.16B/8B, Vn (0x6e205800 / 0x2e205800) ----
    if (insn & 0xffe0_fc00) == 0x6e20_5800 || (insn & 0xffe0_fc00) == 0x2e20_5800 {
        return Inst::SimdNot {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
        };
    }

    // ---- SIMD shift-left immediate: shl Vd.T, Vn.T, #imm ----
    // Gate (insn & (0x0f00_0000 | 0x0000_7000)): prefix 0x0f/0x2f/0x4f/0x6f SIMD
    // register form and the SHIFTL marker (bits[14:12]==0b101 -> 0x5000). Shift
    // = immb (bits[18:16], 0..7); esize from immh ((bits[22:19]), immh==0 => 8B).
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x2f | 0x4f | 0x6f) && (insn & 0x0000_7000) == 0x0000_5000 {
        let immh = (insn >> 19) & 0x7;
        let es2 = if immh == 0 { 3 } else { immh.trailing_zeros() };
        return Inst::SimdShl {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize: (1u8 << es2),          // 1/2/4/8-byte lanes
            shift: ((insn >> 16) & 0x7) as u8,
        };
    }

    // ---- SIMD shift-right accumulate (usra/ssra Vd.T, Vn.T, #imm): Vd += Vn >> imm.
    // Same 0x0f/0x0f/0x4f/0x6f prefix family as shl but the ACCUM marker is bit12
    // ((insn & 0x0000_7000)==0x0000_1000, vs shl's 0x5000 and ushr's 0x0000).
    // unsigned=bit11 (0x6f/0x6e -> usra). shift = esize_bits - (tagless immh:immb).
    // acc=bit12; bit23 clear is the shift-by-imm discriminator vs fmla-el
    // (fmla-el requires bit23 set; usra/ssra shift-imm leave it clear).
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x2f | 0x4f | 0x6f)
        && (insn & 0x0000_7000) == 0x0000_1000 && (insn & 0x0080_0000) == 0 {
        let immh = (insn >> 19) & 0x7;
        let es2 = if immh == 0 { 3 } else { immh.trailing_zeros() };
        let esize: u8 = 1 << es2;
        let immh4: u32 = (insn >> 19) & 0xf;         // immh incl. the size-tag top bit
        let val = if immh4 == 0 { 0 } else { immh4 & ((1u32 << (32 - immh4.leading_zeros() - 1)) - 1) };
                let full: u32 = (val << 3) | ((insn >> 16) & 0x7);
                let cap: u32 = (esize as u32) * 8;
                let shift: u32 = if full < cap { cap - full } else { 0 };
        return Inst::SimdShrAcc {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize,
            shift: shift as u8,
            unsigned: (insn >> 11) & 1 == 1,
        };
    }

    // ---- SIMD FP multiply by element: fmul Vd.T, Vn.T, Vm.T[L] ----
    // Gate (insn & 0x3f00_f000)==0x0f00_9000. Must precede the broad MOVI gate
    // (0x0F|0x6F prefix) which would otherwise swallow 0x0fa29044. esize from
    // size field; index = bit11 low | bit13 high.
    if insn & 0x3f00_f000 == 0x0f00_9000 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let q = (insn >> 30) & 1 == 1;
        let esize: u8 = match (insn >> 22) & 0x3 { 1 => 2, 2 => 4, _ => 8 };
        let index = (((insn >> 11) & 1) | (((insn >> 13) & 1) << 1)) as u8;
        return Inst::SimdFmulEl { rd, rn, rm, esize, index, q };
    }

    // ---- SIMD byte reverse in 64-bit element: rev64 Vd.T, Vn.T ----
    // Gate (insn & 0x3f00_f800)==0x0e00_0800 (REV64-family residue; q=bit30;
    // arrangement via size bits). Byte-reverse each 64-bit granule.
    if insn & 0x3f00_0c00 == 0x0e00_0800 {
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::SimdRev { rd, rn, granule: 8, q: (insn >> 30) & 1 == 1 };
    }
    if (insn & 0x3f00_0c00) == 0x2e00_0800 && (insn & 0x3c00) == 0x0800 && (insn & 0x0020_0000) != 0 {
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::SimdRev { rd, rn, granule: 4, q: (insn >> 30) & 1 == 1 };
    }

    // ---- SIMD lane extract to FP reg: mov Sd/Dd, Vn.T[idx] ----
        // Gate prefix 0x5e (bits31:24) AND byte2 (bits15:8)==0x04. Covers all idx variants
        // (s[0..3], d[0..1], h[0..7], incl. the byte1-nibble set resids 0x5e10/0x5e14/0x5e1c
        // that the narrower 0xfff0_0c00==0x5e00_0400 form missed). esize=1<<tz(imm5),
        // index=imm5>>(tz+1), imm5 = bits20:16.
        // Mask 0xffe0_fc00 (drops rn/rd, keeps imm5) -- fixes rn>=8 forms like
        // mov s4,v19.s[2]=0x5e140664 vs rn0 0x5e140402 (both -> 0x5e000400).
        if insn & 0xffe0_fc00 == 0x5e00_0400 {
            let imm5 = (insn >> 16) & 0x1f;
            if imm5 != 0 {
                let esize = (1u8 << imm5.trailing_zeros()) as u8;
                let index = (imm5 >> (imm5.trailing_zeros() + 1)) as u8;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let rd = (insn & 0x1f) as u8;
                return Inst::SimdLaneS { rd, rn, esize, index };
            }
        }

    // ---- SHA-1 / SHA-256 crypto ops ----
    // Recognise by the specific (masked) v8 SHA opcodes. mode -> helper op.
    let sh = (insn & 0xffe0_fc00, (insn >> 16) & 0x10);
    let sha_op = match sh {
        (0x5e20_0800, _) => 1, // sha1h
        (0x5e00_0000, _) => 2, // sha1c
        (0x5e00_1000, _) => 3, // sha1p
        (0x5e00_2000, _) => 4, // sha1m
        (0x5e00_4000, _) => 5, // sha256h
        (0x5e00_3000, _) => {
            // sha1su0 (bit20 clear) / sha1su1 (bit20 set)
            if (insn & 0x100000) == 0 { 6 } else { 7 }
        }
        (0x5e20_2800, _) => 8, // sha256su0
        (0x5e00_6000, _) => 9, // sha256su1
        _ => 0,
    };
    if sha_op != 0 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        return Inst::Sha { mode: sha_op, rd, rn, rm };
    }

    // ---- SIMD ld2: load two vectors, deinterleaved (ld2 {Vt, Vt1}, [Xn]) ----
    // Prefix 0x0c40 (Q=0) / 0x4c40 (Q=1); st2 is 0x0c00/0x4c00. post-index when
    // bit23=1 (the load is `[Xn], #imm`). Deinterleave: Vt[i]=m[2i], Vt1[i]=m[2i+1].
    if (insn & 0xffc0_0000) == 0x0c40_0000 || (insn & 0xffc0_0000) == 0x4c40_0000 {
        let q = (insn & 0x4000_0000) != 0;
        return Inst::Ld2 {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            q,
            post: if (insn >> 23) & 1 == 1 { 32 } else { 0 },
        };
    }

    // ---- SIMD st2: store two vectors (Vt, Vt2) consecutively to [Xn] ----
    // Prefix 0x0c00 (Q=0) / 0x4c00 (Q=1) -- disjoint from ld2 (0x0c40/0x4c40, bit20).
    if (insn & 0xffc0_0000) == 0x0c00_0000 || (insn & 0xffc0_0000) == 0x4c00_0000 {
        let q = (insn & 0x4000_0000) != 0;
        let bytes = if q { 16 } else { 8 } as i32;
        return Inst::St2 {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            q,
            post: if (insn >> 23) & 1 == 1 { bytes * 2 } else { 0 },
        };
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
    // ---- FP multiply-accumulate by-element: fmla/fmls Vd.T, Vn, Vm.T[idx] ----
    // MUST precede the broad MOVI/mvni gate below (which matches all 0x0f/0x4f
    // prefixes and would otherwise swallow these). Indexed form: byte0 nibble
    // == 0xf (0x4f/0x0f) with bit29 clear (excludes by-element fmul (bit29 set)).
    if ((insn >> 24) & 0x0f) == 0x0f && (insn & 0x2000_0000) == 0 && (insn & 0x0080_0000) != 0 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let vlm = ((insn >> 16) & 0x1f) as u8;
        let idx = if (insn & 0x0040_0000) != 0 {
            ((insn >> 11) & 1) as u8            // .2d: index is L(bit11) only
        } else {
            ((((insn >> 21) & 1) << 1) | ((insn >> 11) & 1)) as u8 // .s: H21<<1|L11
        };
        let el32 = (insn & 0x0040_0000) != 0;
        let q = (insn & 0x4000_0000) != 0;
        let sub = (insn & 0x4000) != 0;
        return Inst::FmlaEl { rd, rn, vlm, idx, el64: el32, q, sub };
    }
    // ---- SIMD widening shift-left (sign/zero extend): shll/usll Vd.Td, Vn.Ts ----
    // byte2 (bits15:8) == 0x38; byte0 in the SHLL family {0e,1e,2e,3e,6e,7e}. Reads the
    // low nlanes half-width elements of Vn, sign/zero-extends each to double width.
    if ((insn >> 24) & 0x0f) == 0x0e && ((((insn >> 8) & 0xff) & 0x7c) == 0x38) {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let b1 = (insn >> 16) & 0xff;
        let (dst_esize, nlanes) = if b1 & 0xa0 == 0xa0 {
            (8u8, 2u8) // .2d
        } else if b1 & 0x60 == 0x60 {
            (4u8, 4u8) // .4s
        } else {
            (2u8, 8u8) // .8h
        };
        return Inst::WidenShl { rd, rn, dst_esize, nlanes, signed: true, upper: (insn >> 30) & 1 == 1 };
            }
            // ---- SIMD add-adjacent-long pairwise accumulate: sadalp/uadalp Vd.Td, Vn.Ts ----
            // byte2 == 0x68; prefix 0x2e(u,q0)/0x4e(s,q0)/0x6e(u,q1)/0x0e(s,q1). For q=0 the
            // dst lanes n_pairs = 8/(2*... ) derived in translate from src_esize below.
            if ((insn >> 8) & 0xff) == 0x68 && matches!((insn >> 24) & 0x0f, 0x0e | 0x2e | 0x4e | 0x6e) {
                let rd = (insn & 0x1f) as u8;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let b1 = (insn >> 16) & 0xff;
                let src_esize: u8 = if b1 & 0x40 != 0 { 1 } else { 2 }; // byte1 bit6: 0x20=>.b, 0x60=>.h
                let signed = (insn >> 29) & 1 == 0; // 0x4e/0x0e signed, 0x6e/0x2e unsigned
                return Inst::SimdAdalp { rd, rn, src_esize, n_pairs: 0, signed };
            }
            // ---- SIMD saturating add/sub: sqadd/uqadd/sqsub/uqsub Vd.T, Vn, Vm -----
            // byte2 {0x0c (add), 0x2c (sub)}; prefix 0x0e/2e/4e/6e (signed 0e/4e, unsigned 2e/6e).
            let b2s = (insn >> 8) & 0xff;
            if ((b2s == 0x0c || b2s == 0x2c) && matches!((insn >> 24) & 0x0f, 0x0e | 0x2e | 0x4e | 0x6e)) {
                let b1 = (insn >> 16) & 0xff;
                let esize: u8 = 1u8 << ((insn >> 22) & 0x3);
                return Inst::SimdSatAdd {
                    rd: (insn & 0x1f) as u8,
                    rn: ((insn >> 5) & 0x1f) as u8,
                    rm: ((insn >> 16) & 0x1f) as u8,
                    esize,
                    sub: b2s == 0x2c,
                    unsigned: ((insn >> 29) & 1) == 1, // 0x2e/0x6e
                    q: (insn >> 30) & 1 == 1,
                };
            }
            // ---- SIMD add/sub-long: saddl/uaddl/subl/usubl Vd.T, Vn.T, Vm.T -----
            // bit12==0 (long, vs addw wide which is bit12=1); byte2 low 0x00(add)/0x20(sub).
            // esrc = 1<<bits[23:22]; sign=bit29==0; upper=bit30.
            let al = insn & 0xffe0_fc00;
            let alres = [
                0x0e60_0000u32,0x0e60_2000u32,0x2e60_0000,0x2e60_2000,
                0x4e60_0000,0x4e60_2000,0x6e60_0000,0x6e60_2000,
            ];
            if alres.contains(&al) {
                            let b1 = (insn >> 16) & 0xff;
                            let esrc: u8 = if b1 & 0x80 != 0 { 4 } else if b1 & 0x20 != 0 { 2 } else { 1 };
                            return Inst::SimdAddl {
                                rd: (insn & 0x1f) as u8,
                                rn: ((insn >> 5) & 0x1f) as u8,
                                rm: ((insn >> 16) & 0x1f) as u8,
                                esrc,
                                sign: ((insn >> 29) & 1) == 0,
                                sub: (insn & 0x2000) != 0,
                                upper: (insn >> 30) & 1 == 1,
                            };
                        }
                        // ---- scalar FP multiply / negate-multiply: fmul/fnmul Sd/Dd, Sn, Sm ----
        // Gate (insn&0x1fe0_0c00) in {0x1e20_0800 (single), 0x1e60_0800 (double)}
        // AND byte2 (bits15:8) in {0x08, 0x88} (fmul opcode; excludes fdiv 0x18, fadd 0x28,
        // and the `ut`-family ucvtf d0,d1=0x7e61d820 byte2 0xd8). neg = fnmul (bit15).
        if ((insn & 0x1fe0_0c00) == 0x1e20_0800 || (insn & 0x1fe0_0c00) == 0x1e60_0800)
            && ((insn >> 8) & 0x78 == 0x08)
        {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let neg = (insn & 0x8000) != 0;         // bit15: fnmul vs fmul
        let double = (insn & 0x0040_0000) != 0; // bit22: .d vs .s
        return Inst::FmulScalar { rd, rn, rm, double, neg };
    }
    // ---- SIMD signed/unsigned int min/max: smin/smax/umin/umax Vd.T, Vn, Vm ----
    // byte0 (bits31:24) in {0x0e,0x2e,0x4e,0x6e} AND byte2 (bits15:8) in {0x64(max),0x6c(min)}.
    // unsigned = bit29 (set: umin/umax; clear: smin/smax), q=bit30, esize: .s (bit23 set) else .b.
    let b0 = (insn >> 24) & 0xff;
    let b2 = (insn >> 8) & 0xff;
    if (b0 == 0x0e || b0 == 0x2e || b0 == 0x4e || b0 == 0x6e)
            && ((b2 & 0xfc) == 0x64 || (b2 & 0xfc) == 0x6c) {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        return Inst::SminMax {
            rd,
            rn,
            rm,
            max: b2 == 0x64,
            unsigned: (insn & 0x2000_0000) != 0,
            esize: if (insn & 0x0080_0000) != 0 { 4 } else { 1 }, // .s vs .b
            q: (insn & 0x4000_0000) != 0,
        };
    }
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
            // .2S/.4S: mvni (inverted word lanes) — replicate ~imm8 in 32-bit words
            (1, 0b0000) => {
                let lane = (!(imm8 as u64)) & 0xffff_ffff;
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
            // .2S/.4S with LSL shift: value = imm8 << (cmode<<2) (word lanes; cmodes
            // 2,4,6 are pure word-lsl; cmodes 8,9,a,b are HALFWORD and handled below).
            (0, 2) | (0, 4) | (0, 6) => {
                let sh = (cmode << 2) as u32;
                let lane = ((imm8 as u64) << sh) & 0xffff_ffff;
                let low = lane | (lane << 32);
                if (insn >> 30) & 1 == 1 { (low, low) } else { (low, 0) }
            }
            // mvni .2S/.4S with LSL shift: value = ~(imm8 << (cmode<<2))
            (1, 2) | (1, 4) | (1, 6) => {
                let sh = (cmode << 2) as u32;
                let lane = (!((imm8 as u64) << sh)) & 0xffff_ffff;
                let low = lane | (lane << 32);
                if (insn >> 30) & 1 == 1 { (low, low) } else { (low, 0) }
            }
            // .4H/.8H halfword immediates (cmode 0x8..0xb): 16-bit element = imm8
            // shifted by (cmode&0x2)?8 when the lsl#8 marker is set, then inverted
            // if op==1 (mvni/bic). objdump-verified v14.4h,#0xfc,lsl8 -> 0x03ff.
            (_, 8) | (_, 9) | (_, 0xa) | (_, 0xb) => {
                let sh = (((cmode >> 1) & 1) * 8) as u32;
                let mut lane = (imm8 as u64) << sh;
                if op == 1 { lane = (!lane) & 0xffff; }
                let low = lane | (lane << 16) | (lane << 32) | (lane << 48);
                if (insn >> 30) & 1 == 1 { (low, low) } else { (low, 0) }
            }
            // MSL (mask shift left) word immediates (cmode 0xc=msl#8, 0xd=msl#16):
            // element = (imm8 << (8*idx)) | (all-ones mask in the low shift bits).
            (_, 0xc) | (_, 0xd) => {
                let sh = (((cmode & 0x1) + 1) * 8) as u32; // 8 or 16
                let lane = ((imm8 as u64) << sh) | ((1u64 << sh) - 1);
                let low = lane | (lane << 32);
                if (insn >> 30) & 1 == 1 { (low, low) } else { (low, 0) }
            }
            _ => return Inst::Unsupported(insn),
        };
        return Inst::VecMovi {
                    vd: rd(insn),
                    lo,
                    hi,
                };
            }

            // ---- scalar udiv/sdiv Wd,Wn,Wm (data-processing 2-source) ----
            if (insn & 0xffe0_0000) == 0x1ac0_0000 || (insn & 0xffe0_0000) == 0x9ac0_0000 {
                return Inst::Div {
                    rd: (insn & 0x1f) as u8,
                    rn: ((insn >> 5) & 0x1f) as u8,
                    rm: ((insn >> 16) & 0x1f) as u8,
                                signed: (insn >> 10) & 1 == 1,
                                is_x: (insn >> 30) & 1 == 1,
                            };
                        }

                        // ---- SIMD variable register shift: ushl/sshl Vd.T, Vn.T, Vm.T ----
                        // Gate &0x3f00_0c00 in {0x2e00_0400 (ushl Q=0), 0x4e00_0400 (sshl), 0x6e00_0400 (ushl Q1)}.
                        // sign = bit29. Per-lane shifts by the count in the matching Vm lane.
                        let vsh = insn & 0x3f00_0c00;
                            if (vsh == 0x2e00_0400 || vsh == 0x4e00_0400 || vsh == 0x6e00_0400) && (insn & 0x0000_4000) != 0 {
                            return Inst::SimdVShift {
                                rd: (insn & 0x1f) as u8,
                                rn: ((insn >> 5) & 0x1f) as u8,
                                rm: ((insn >> 16) & 0x1f) as u8,
                                esize: (1u8 << ((insn >> 22) & 0x3)),
                                signed_: (insn >> 29) & 1 == 1,
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
        // Sign-extend load (ldrsw/ldrsh/ldrsb): bit23=1 (opc 10/11) is always a
        // sign-extend load; bit22 alone is ambiguous (0x79c0 ldrsh w0,[x0] has
        // bit22=1 yet IS sign-extend). Size-8 never sets bit23.
        let sext = (insn & 0x80_0000) != 0;
        let ld = ld || sext;
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
            sext,
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
            0x1e60_0800 | 0x1e20_0800 => Some(4), // fmul
            0x1e60_2800 | 0x1e20_2800 => Some(5), // fadd
            0x1e60_3800 | 0x1e20_3800 => Some(6), // fsub
            0x1e60_1800 | 0x1e20_1800 => Some(7), // fdiv
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
            0x1e21_c000 => Some(0), // fsqrt s{rd}, s{rn} (single)
            0x1e65_4000 => Some(1), // frintm (round toward -inf) = floor
            0x1e25_4000 => Some(1), // frintm s (floor)
            0x1e64_8000 => Some(2), // frintp (round toward +inf) = ceil
            0x1e24_8000 => Some(2), // frintp s (ceil)
            0x1e64_c000 => Some(2), // frintp d (alt imm, e.g. frintp s0,s0 = 0x1e24c000)
            0x1e24_c000 => Some(2), // frintp s (alt imm)
            0x1e65_c000 => Some(3), // frintz (round toward zero)
            0x1e25_c000 => Some(3), // frintz s (trunc)
            0x1e60_c000 => Some(5), // fabs d{rd}, d{rn} (clear sign)
            0x1e20_c000 => Some(5), // fabs s{rd}, s{rn} (clear sign)
            0x1e61_4000 => Some(6), // fneg d{rd}, d{rn} (flip sign)
            0x1e21_4000 => Some(6), // fneg s{rd}, s{rn} (flip sign)
            0x1e66_4000 => Some(7), // frinta d (round half-away; residue keeps bit14)
            0x1e26_4000 => Some(7), // frinta s
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
        // (ubfx/sbfx/uxb/sxtb/ughl. any immr<=imms) AND the UBFIZ insert form
        // (immr>imms => zero-extend+shift, all translate BitField). BFM-insert
        // (bfi/bfc) is 0xb3/0x33, handled in the block below.
        let is_valid = (imms == bits - 1) // LSR/ASR
            || (immr == (imms + 1) % bits) // LSL
            || (immr <= imms) // UBFX/SBFX + zero/sign-extend
            || (immr > imms); // UBFIZ/ASR-or-UBFIZ insert (0xd3/0x53, immr>imms)
        if is_valid && (rd != 31) {
            return Inst::BitField { rd, rn, immr, imms, sf, arith, insert: false };
        }
    }

    // ---- bitfield insert (BFM/BFI/BFC/BFXIL): inserts bits of Rn into Rd ----
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
        // Both BFM insert forms merge bits of Rn into Rd:
        //  - wrap (immr>imms) => BFI/BFC: lsb=(bits-immr)&(bits-1), w=imms+1
        //  - non-wrap (immr<=imms) => BFXIL: copy Rn bits [imms:immr] into the
        //    same positions of Rd. Both are `insert` (Rd's other bits preserved).
        if immr > imms || immr <= imms {
            return Inst::BitField { rd, rn, immr, imms, sf, arith: false, insert: true };
        }
    }

    // ---- FP convert to signed integer (fcvtzs/fcvtas): Dn|Sn -> Rd ----
    // Real encodings (verified): fcvtzs Wd,Dn = 0x1e78_0000 / Xd = 0x9e78_0000
    // (truncate); fcvtas Wd,Dn = 0x1e7a_0000 / Xd = 0x9e7a_0000 (nearest-away).
    // Rounding drives `mode`: 0=truncate (fcvtzs), 2=nearest-away (fcvtas).
    {
        let b = insn & 0xffff_f800;
        let (mode, ok) = if b == 0x1e78_0000 || b == 0x9e78_0000 || b == 0x1e38_0000 || b == 0x9e38_0000 {
            (0, true) // fcvtzs: truncate (0x1e38/0x9e38 = single-source S form)
        } else if b == 0x1e7a_0000 || b == 0x9e7a_0000 || b == 0x1e3a_0000 || b == 0x9e3a_0000 {
            (2, true) // fcvtas: round nearest-away (0x1e3a/0x9e3a = single-source)
        } else if b == 0x1e24_0000 || b == 0x9e24_0000 {
            (2, true) // fcvtas Wd/Xd, Sn single-source (objdump-confirmed 0x1e24000b)
        } else if b == 0x1e64_0000 || b == 0x9e64_0000 {
            (2, true) // fcvtas Wd,Dn / Xd,Dn double-source (objdump-confirmed 0x1e640013)
        } else if b == 0x1e30_0000 || b == 0x9e30_0000 || b == 0x1e70_0000 || b == 0x9e70_0000 {
            (4, true) // fcvtms: round toward -inf / floor
        } else if b == 0x1e28_0000 || b == 0x9e28_0000 || b == 0x1e68_0000 || b == 0x9e68_0000 {
            (3, true) // fcvtps: round toward +inf / ceil
        } else if b == 0x1e20_0000 || b == 0x9e20_0000 || b == 0x1e60_0000 || b == 0x9e60_0000 {
            (2, true) // fcvtns: round to nearest (even) — closest via cvtsd2si
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
                    src_sng: false,
                };
            }
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            return Inst::FcvtToInt {
                rd,
                rn,
                mode,
                sf,
                unsigned: false,
                src_sng: true, // single (S) source
            };
        }
    }

    // ---- FP convert to UNSIGNED integer (fcvtzu): Dn -> Rd (unsigned int) ----
    // bases 0x1e79_0000 (W dest) / 0x9e79_0000 (X dest). Distinct from signed
    // fcvtzs at 0x1e78_0000/0x9e78_0000 (bit16 of the nibble: 0x79 vs 0x78).
    // ---- FP convert to UNSIGNED integer (fcvtzu): Dn -> Rd (unsigned int) ----
    // bases 0x1e79_0000 (W dest) / 0x9e79_0000 (X dest), double source; and
    // 0x1e39_0000/0x9e39_0000 single source (src_sng). Distinct from signed
    // fcvtzs at 0x1e78_0000/0x9e78_0000 (bit16 of the nibble).
    if (insn & 0xffff_f800) == 0x1e79_0000 || (insn & 0xffff_f800) == 0x9e79_0000
        || (insn & 0xffff_f800) == 0x1e39_0000 || (insn & 0xffff_f800) == 0x9e39_0000
    {
        let sf = (insn >> 31) & 1 == 1; // 1 => 64-bit (X) destination
        let sz = (insn >> 22) & 1 == 1; // 1 => source is double (d)
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::FcvtToInt {
            rd,
            rn,
            mode: 0, // truncate-toward-zero (fcvtzu always truncates)
            sf,
            unsigned: true,
            src_sng: !sz,
        };
    }

    // ---- FP convert to int with round toward +inf/-inf: fcvtps/pu/ms/mu (Xd dest) ----
        // Family (insn & 0xffff_0000) for the **X-dest** (top 0x9e) round forms.
        // fcvt-round to a 64-bit int covers the libroblox boot (fcvtpu x9,s0 = 0x9e290009)
        // and, critically, 0x9e.. does NOT collide with fcmp (which is always the 0x1e
        // W-dest column: 0x1e70_.. == fcmp d6,d16, but 0x9e70_.. == legitimate fcvt to X).
        // bit16=U (unsigned), bit22=double source, round: 0x28=+inf 0x30=-inf.
        {
            let fam = insn & 0xffff_0000;
            let mode = match fam {
                0x9e28_0000 | 0x9e29_0000 | 0x9e68_0000 | 0x9e69_0000
                    | 0x1e28_0000 | 0x1e29_0000 | 0x1e68_0000 | 0x1e69_0000 => 3, // +inf (X & W dest)
                0x9e30_0000 | 0x9e31_0000 | 0x9e70_0000 | 0x9e71_0000 => 4, // -inf (X dest only; 0x1e70 collides with fcmp)
                0x9e24_0000 | 0x9e25_0000 | 0x9e64_0000 | 0x9e65_0000 | 0x9e60_0000 | 0x9e61_0000 | 0x1e65_0000 => 2, // nearest (X dest; + W fcvtau 0x1e65)
                _ => 255,
            };
            if mode != 255 {
                let szd = (insn >> 22) & 1 == 1; // 1 => source is double (d)
                let unsigned = (insn >> 16) & 1 == 1;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let rd = (insn & 0x1f) as u8;
                return Inst::FcvtToInt {
                    rd,
                    rn,
                    mode,
                    sf: (insn >> 31) & 1 == 1, // Xd (64-bit) when 0x9e, Wd (32-bit) when 0x1e
                    unsigned,
                    src_sng: !szd,
                };
            }
        }

        // ---- `scvtf d0, w0 = 0x1e620000`. The scalar int->FP family gate
            // `(insn & 0xf7be_fc00)` resolves to {0x16220000 (W source), 0x96220000 (X)}
            // for both signed and unsigned, and is disjoint from fmov/fcvt/fcmp/fmul
            // (verified vs all 8 encodings + neighbours). Fields: X-src = bit31,
            // unsigned (ucvtf) = bit16, double-dest = bit22. FP->int fcvt* (bit17=0)
            // and fmov are handled above, so this family is unambiguous.
            {
                let gi = insn & 0xf7be_fc00;
                if gi == 0x1622_0000 || gi == 0x9622_0000 {
                    let sf = (insn >> 31) & 1 == 1; // 1 => 64-bit integer src (Xn)
                    let to_double = (insn >> 22) & 1 == 1; // 1 => Dd (double), 0 => Sd
                    let unsigned = (insn >> 16) & 1 == 1; // ucvtf (unsigned)
                    let rn = ((insn >> 5) & 0x1f) as u8;
                    let rd = (insn & 0x1f) as u8;
                    return Inst::Scvtf {
                        rd,
                        rn,
                        to_double,
                        sf,
                        unsigned,
                    };
                }
            }

            // ---- scalar fixed-point int->FP: ucvtf/scvtf Dd,Xn,#fbits / Sd,Wn,#fbits ----
            // Gate: byte1 (bits 16:23)&0xfe ∈ {0x42, 0x02}; byte0 ∈ {0x9e(D), 0x1e(S)}.
            // distinct from unscaled Scvtf (byte1 0x63/0x23) and fmov/fcvt.
            {
                let m1 = ((insn >> 16) & 0xff) & 0xfe;
                if (m1 == 0x42 || m1 == 0x02) && (insn & 0x3000_0000) == 0x1000_0000 {
                    return Inst::ScvtfFixed {
                        rd: (insn & 0x1f) as u8,
                        rn: ((insn >> 5) & 0x1f) as u8,
                        to_double: (insn >> 30) & 1 == 1,
                        sf: (insn >> 30) & 1 == 1,
                        unsigned: (insn >> 16) & 1 == 1,
                        fbits: (64 - ((insn >> 10) & 0x3f)) as u8,
                    };
                }
            }

            // ---- scalar FP max/min: fmax/fmin/fmaxnm/fminnm Sd,Xn,Xm ----
            // Gate (insn & 0x1f20_0c00)==0x1e20_0800; op=bits12-15 (4=fmax,5=fmin,6=fmaxnm,7=fminnm); sz=bit22.
            if (insn & 0x1f20_0c00) == 0x1e20_0800 {
                let mop = (insn >> 12) & 0xf;
                if matches!(mop, 4 | 5 | 6 | 7) {
                    return Inst::FMaxMin {
                        rd: (insn & 0x1f) as u8,
                        rn: ((insn >> 5) & 0x1f) as u8,
                        rm: ((insn >> 16) & 0x1f) as u8,
                        sz: (insn >> 22) & 1 == 1,
                        op: mop as u8,
                    };
                }
            }

                // ---- FP horizontal reduction: fmaxv/fminv Sd, Vn.4s ----
                // Cross-lane max/min of all 4 single-precision lanes of Vn into
                // scalar Sd. Residue (insn&0x3f20_0c00)==0x2e20_0800, disjoint
                // from the 3-operand scalar FMaxMin (residue 0x1e20_0800). Bit23
                // selects min (fminv) vs max (fmaxv).
                if (insn & 0x3f20_0c00) == 0x2e20_0800 && (insn & 0x0010_0000) != 0 {
                    let rd = (insn & 0x1f) as u8;
                    let rn = ((insn >> 5) & 0x1f) as u8;
                    let min = (insn & 0x0080_0000) != 0;
                    return Inst::FMaxV { rd, rn, min };
                }

                // ---- FP multiply-accumulate: fmla/fmls Vd.T, Vn, Vm ----
                // Vd = Vd +/- Vn*Vm per-lane. Tight gate (insn&0x1fe0_0c00)==0x0e20_0c00
                // with bit29 (=0x2000_0000) CLEAR, which excludes fmul (bit29 set) and
                // SIMD-3-same logical `bit/bif/bsl` (0x6ea11c40 -> residue 0x0ea00c00).
                // Disjoint from fmaxv (0x0e20_0800) / ucvtf2d (0x0e60_0800). Fields:
                                // bit23 = subtract(fmls), bit22(el64) = .2d double, bit30(q) = high width.
                                // Mask 0xffe0_fc00 drops rd/rn/rm; base .2s/.4s/.2d widths (bit22 el
                                // -> 0x0e60/0x4e60) and stray fmls (bit23 -> 0x..a0). Excludes fmul
                                // (0x6e prefix, bit29) and by-element (0x0f, handled earlier).
                                let fmla_b = insn & 0xffe0_fc00;
                                if (fmla_b == 0x0e20_cc00 || fmla_b == 0x4e20_cc00 || fmla_b == 0x0e60_cc00
                                    || fmla_b == 0x4e60_cc00 || fmla_b == 0x0ea0_cc00 || fmla_b == 0x4ea0_cc00
                                    || fmla_b == 0x0ee0_cc00 || fmla_b == 0x4ee0_cc00)
                                    && (insn & 0x2000_0000) == 0
                                {
                                    let rd = (insn & 0x1f) as u8;
                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                    let el64 = (insn & 0x0040_0000) != 0;
                                    let q = (insn & 0x4000_0000) != 0;
                                    let sub = (insn & 0x0080_0000) != 0;
                                    return Inst::Fmla { rd, rn, rm, el64, q, sub };
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
                // ---- FMOV scalar immediate single (fmov Sd, #imm) ----
                if (insn & 0xffe0_0000) == 0x1e20_0000 && (insn & 0x1000) != 0 {
                    let rd = (insn & 0x1f) as u8;
                    let imm8 = ((insn >> 13) & 0xff) as u32;
                    let f64bits = decode_fmov_imm(imm8, true);
                    let v = f64::from_bits(f64bits) as f32;
                    return Inst::FmovImm {
                        rd,
                        f64: false,
                        value_bits: v.to_bits() as u64,
                    };
                }
                // ---- scalar FP register-to-register move: fmov Dd,Dn / fmov Sd,Sn ----
                    // Double form = 0x1e60_4000, single form = 0x1e20_4000 (sz bit22 selects).
                    if (insn & 0xffff_f000) == 0x1e60_4000 || (insn & 0xffff_f000) == 0x1e20_4000 {
                        let sz = (insn >> 22) & 1 == 1; // 1 => double (d), 0 => single (s)
                        let rn = ((insn >> 5) & 0x1f) as u8;
                        let rd = (insn & 0x1f) as u8;
                        return Inst::FmovFp { rd, rn, sz };
                }
                // ---- scalar FP compare to NZCV: fcmp Dn, Dm / fcmp Sn, Sm ----
                    // Double gate (insn & 0xffe0_fc00)==0x1e602000, single ==0x1e202000 (bit22
                    // selects). Masks rn(5-9)/rm(16-20)/rd(0-4); rm==0 covers `fcmp Dn,#0.0`.
                    let fcmp_sz = (insn & 0xffe0_fc00) == 0x1e60_2000
                        || (insn & 0xffe0_fc00) == 0x1e20_2000;
                    if fcmp_sz {
                        let sz = (insn & 0x400000) != 0; // 1 => double (0x1e6...), 0 => single
                        let rn = ((insn >> 5) & 0x1f) as u8;
                        let rm = ((insn >> 16) & 0x1f) as u8;
                        return Inst::Fcmp { rn, rm, sz };
                    }
                    // ---- scalar FP conditional compare: fccmp Dn, Dm, #nzcv, <cond> ----
                    // Mask 0xfff0_fc03 (drops rn/rm/rd/cond/nzcv) yields 0x1e60_c400 (d)
                    // / 0x1e20_c400 (s); disjoint from fcmp (0x1e602000).
                    if (insn & 0xfff0_fc03) == 0x1e60_c400 || (insn & 0xfff0_fc03) == 0x1e20_c400 {
                        let sz = (insn & 0x400000) != 0; // 1 => double (0x1e6...), 0 => single
                        let rn = ((insn >> 5) & 0x1f) as u8;
                        let rm = ((insn >> 16) & 0x1f) as u8;
                        let nzcv = (insn & 0xf) as u8;
                        let cond = ((insn >> 12) & 0xf) as u8;
                        return Inst::Fccmp { rn, rm, nzcv, cond, sz };
                    }
                    // ---- scalar FP->int stored to a FP reg: fcvtzs Dd,Dn / Sd,Sn ----
                    // Prefix 0x5e (bit29 set = scalar FP target, vs vector 0xfe/0x4e);
                    // residue (insn&0xffe0_fc00) in {0x5ea0_b800 (s), 0x5ee0_b800 (d)}.
                    if (insn & 0xffe0_fc00) == 0x5ea0_b800 || (insn & 0xffe0_fc00) == 0x5ee0_b800
                        || (insn & 0xffe0_fc00) == 0x7ee0_b800 || (insn & 0xffe0_fc00) == 0x7ea0_b800 {
                        let rd = (insn & 0x1f) as u8;
                        let rn = ((insn >> 5) & 0x1f) as u8;
                        let dbl = (insn & 0x0040_0000) != 0; // 0x5ee/7ee vs 0x5ea (bit22)
                        let unsigned = (insn & 0x2000_0000) != 0; // 0x7e vs 0x5e
                        return Inst::FcvtTzReg { rd, rn, dbl, unsigned };
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

                                    // ---- NEON int add/sub (4x32 lanes): add Vd.4s, Vn.4s, Vm.4s | sub Vd.4s,... ----
                                        // class Q=1 0x0e20_0000 .. 0x4e20_0000 integer add (S: size=01);
                                        // sub is the same class with bit29 set (0x2e20_0400 vs 0x0e20_0400).
                                        let addclass = insn & 0x2f20_0c00;
                                        if (addclass == 0x0e20_0400 || addclass == 0x2e20_0400) && ((insn >> 15) & 1) == 1 && ((insn >> 22) & 3) == 2 {
                                            let rm = ((insn >> 16) & 0x1f) as u8;
                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                            let rd = (insn & 0x1f) as u8;
                                            let op = if addclass == 0x2e20_0400 { 1 } else { 0 };
                                            return Inst::Simd4s { rd, rn, rm, op };
                                        }
                                        // ---- SIMD int add/sub 2D (64-bit lanes): add Vd.2D, Vn.2D, Vm.2D ----
                                        // Same walk as Simd4s but size-field == 3 (D lanes), Q=0/1.
                                        // sub = 0x6e.. vs add 0x4e.. (bit29). Disjoint: Simd4s above only when size!=3.
                                        let add2d = insn & 0x2f20_0c00;
if (add2d == 0x0e20_0400 || add2d == 0x2e20_0400) && ((insn >> 22) & 3) == 3 {
                                            let rm = ((insn >> 16) & 0x1f) as u8;
                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                            let rd = (insn & 0x1f) as u8;
                                            let sub = (insn >> 29) & 1 == 1;
                                            return Inst::SimdAddD { rd, rn, rm, sub };
                                        }
                                        // ---- SIMD int add/sub byte lanes 16b/8b: add Vd.16b, Vn.16b, Vm.16b ----
                                                                                // size field bits[23:22] == 0 (byte). Same walk residue as Simd4s but byte lane.
                                                                                {
                                                                                    let bc = insn & 0x2f20_0c00;
                                                                                    if (bc == 0x0e20_0400 || bc == 0x2e20_0400) && ((insn >> 22) & 3) == 0 {
                                                                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                        let sub = (insn >> 29) & 1 == 1;
                                                                                        let q = (insn >> 30) & 1 == 1;
                                                                                        return Inst::SimdAddB { rd, rn, rm, sub, q };
                                                                                    }
                                                                                }
                                                                                // ---- SIMD int add/sub halfword 8H/4H (16-bit) lanes ----
                                                                                // Same walk/wrap residue as Simd4s/AddB but size-field == 1 (H lanes).
                                                                                {
                                                                                    let hc = insn & 0x2f20_0c00;
                                                                                    if (hc == 0x0e20_0400 || hc == 0x2e20_0400) && ((insn >> 22) & 3) == 1 {
                                                                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                        let sub = (insn >> 29) & 1 == 1;
                                                                                        let q = (insn >> 30) & 1 == 1;
                                                                                        return Inst::SimdAddH { rd, rn, rm, sub, q };
                                                                                    }
                                                                                }
                                                                                // Gate `(insn & 0xffe0_fc00) == 0x6e60d800`: masks rn/rd (bits 0-9, 16-20 via
                                                                                // 0xffe0/fc00) and keeps the top+convert bits. Verified against real
                                                                                // 0x6e61d842 (ucvtf v2.2d,v2.2d) and compiler 0x6e61dbff (ucvtf v31.2d,v31.2d);
                                                                                // excludes scvtf (0x4e60d800), scalar d,d (0x7e60d800), and compare/fmov forms.
                                        if (insn & 0xffe0_fc00) == 0x6e60_d800 {
                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                let rd = (insn & 0x1f) as u8;
                                                return Inst::Ucvtf2d { rd, rn };
                                            }
                                            // ---- scalar unsigned int64->double: ucvtf Dd, Dn ----
                                            // Gate (insn & 0xffe0_fc00) == 0x7e60_d800 (scalar, disjoint from
                                            // vector Ucvtf2d 0x6e60_d800 by bit23). Reads Dn low 64 as u64 -> double.
                                            if (insn & 0xffe0_fc00) == 0x7e60_d800 || (insn & 0xffe0_fc00) == 0x7e20_d800 {
                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                            let rd = (insn & 0x1f) as u8;
                                            return Inst::ScalarUcvtf { rd, rn, sng: (insn & 0x0040_0000) != 0 };
                                        }
                                        // ---- scalar Ssigned int64->double: scvtf Dd, Dn ----
                                        // Gate (insn & 0xffe0_fc00) == 0x5e60_d800. Sibling of the
                                        // 0x7e60_d800 (unsigned) form; bit23 distinguishes them.
                                        if (insn & 0xffe0_fc00) == 0x5e60_d800 || (insn & 0xffe0_fc00) == 0x5e20_d800 {
                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                            let rd = (insn & 0x1f) as u8;
                                            return Inst::ScalarScvtf { rd, rn, sng: (insn & 0x0040_0000) != 0};
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
                                                                                                        if (insn & 0xff00_fc00) == 0x0e00_0c00 || (insn & 0xff00_fc00) == 0x4e00_0c00 {
                                                                                                                                                                                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                let esize = 1u8 << ((insn >> 16) & 0x1f).trailing_zeros();
                                                                                                                                                                                                                let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                return Inst::SimdDupGp { rd, rn, esize, q };
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
                                                                                                                        // ---- SIMD widening multiply smull/umull & smlal/umlal (0x0f00_c000/8000 gate) ----
                                                                                                                                if (insn & 0x0f00_c000) == 0x0e00_c000 || (insn & 0x0f00_c000) == 0x0e00_8000 {
                                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                    let res_esize: u8 = if (insn >> 22) & 1 == 1 { 8 } else { 4 };
                                                                                                                                    return Inst::SimdMull {
                                                                                                                                        rd, rn, rm,
                                                                                                                                        res_esize,
                                                                                                                                        unsigned: (insn >> 28) & 1 == 1,
                                                                                                                                        q: (insn >> 30) & 1 == 1,
                                                                                                                                        acc: (insn & 0x0000_8000) == 0x0000_8000,
                                                                                                                                    };
                                                                                                                                }
                                                                                                                            // ---- SIMD unsigned compare-higher: cmhi Vd.4S/Vd.2S, Vn., Vm. ----
                                                                                                                                // Gate &0xffe0_fc00: 0x6ea03400 (4S, Q=1, real 0x6ea13461) / 0x2ea03400 (2S).
                                                                                                                                // Each 32-bit lane = all-ones if Vn[i] > Vm[i] (unsigned), else 0.
                                                                                                                                let scm = insn & 0xffe0_fc00;
                                                                                                                                let cm_lanes = if scm == 0x6ea0_3400 { Some(4) } else if scm == 0x2ea0_3400 { Some(2) } else { None };
                                                                                                                                if let Some(clanes) = cm_lanes {
                                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                    return Inst::SimdCmhi { rd, rn, rm, lanes: clanes };
                                                                                                                                                                                                    }
                                                                                                                                                                                                    // 2D (64-bit lanes): 0x6ee0_3400 (Q=1). Per 8-byte lane all-ones if Vn>Vm.
                                                                                                                                                                                                    if scm == 0x6ee0_3400 {
                                                                                                                                                                                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                        return Inst::SimdCmhiD { rd, rn, rm };
                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                            // ---- SIMD signed compare-greater (cmgt) 0x4ea0_3400(.4s)/0x0ea0_3400(.2s)/0x4ee0_3400(.2d) ----
                                                                                                                                                                                                                                                                                                                                            let cgt = insn & 0xffe0_fc00;
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x4ea0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:4, dword:false }; }
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x0ea0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:2, dword:false }; }
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x4ee0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:2, dword:true }; }
                                                                                                                                                                                                                                                                                                                                            // ---- SIMD unzip even: uzp1 Vd.T, Vn.T, Vm.T ----
                                                                                                                                                                                                    // opcode bits[13:8] = 0x18 (verified non-colliding vs uzp2/zip1/zip2/trn1/trn2).
                                                                                                                                                                                                    // esize = 1 << bits[23:22]; q = bit30. Vd[i] = Vn[2i], Vd[n+i] = Vm[2i].
                                                                                                                                                                                                    if matches!((insn & 0x3f00), 0x1800 | 0x1a00) {
                                                                                                                                                                                                         let esize = (1 << ((insn >> 22) & 0x3)) as u8;
                                                                                                                                                                                                         let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                         let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                         let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                         let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                         return Inst::SimdUz1 { rd, rn, rm, esize, q };
                                                                                                                                                                                                                                                                                      }
                                                                                                                                                                                                                                                                                      // ---- SIMD zip1: zip1 Vd.T, Vn.T, Vm.T (mask 0x3f20_fc00 == 0x0e00_3800) ----
                                                                                                                                                                                                                                                                                                                                                                   // Exact base excludes `ext` (SimdExt shares 0x..3800 but differs in
                                                                                                                                                                                                                                                                                                                                                                   // higher permute bits). d[2k]=Vn[k], d[2k+1]=Vm[k] (lower halves).
                                                                                                                                                                                                                                                                                                                                                                   if insn & 0x3f20_fc00 == 0x0e00_3800 {
                                                                                                                                                                                                                                                                                                                                                                       let esize = (1 << ((insn >> 22) & 0x3)) as u8;
                                                                                                                                                                                                                                                                                                                                                                       let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                       let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                       let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                       let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                                                                                                                                                                       return Inst::SimdZip1 { rd, rn, rm, esize, q };
                                                                                                                                                                                                                                                                                                                                                                   }
                                                                                                                                                                                                                                                                                                                                                                   // ---- SIMD element extract to GPR: umov/smov Rd, Vn.bits[idx] ----
                                                                                                                                                                                                             // Gate &0xbfe0_fc00: 0x0e00_3c00 (umov, widen-0) / 0x0e00_2c00 (smov, widen-s).
                                                                                                                                                                                                             // esize = 1<<tz(imm5), index = imm5>>(tz+1); is_x = bit30.
                                                                                                                                                                                                             let ge = insn & 0xbfe0_fc00;
                                                                                                                                                                                                             if ge == 0x0e00_3c00 || ge == 0x0e00_2c00 {
                                                                                                                                                                                                                 let signed = ge == 0x0e00_2c00;
                                                                                                                                                                                                                 let is_x = ((insn >> 30) & 0x1) == 1;
                                                                                                                                                                                                                 let imm5 = (insn >> 16) & 0x1f;
                                                                                                                                                                                                                 let tz = imm5.trailing_zeros();
                                                                                                                                                                                                                 let esize = (1 << tz) as u8;
                                                                                                                                                                                                                 let index = (imm5 >> (tz + 1)) as u8;
                                                                                                                                                                                                                 let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                 let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                 return Inst::SimdMovEl { rd, rn, esize, index, signed, is_x };
                                                                                                                                                                                                             }
                                                                                                                                                                                                             // ---- SIMD compare equal: cmeq Vd.T, Vn.T, Vm.T ----
                                                                                                                                                                                                        // Each element is all-ones if Vn[i]==Vm[i], else 0.
                                                                                                                                                                                                        // Gate &0xffe0_fc00 residues: 2d=0x6ee08c00, 4s=0x6ea08c00,
                                                                                                                                                                                                        // 2s=0x2ea08c00, 16b=0x6e208c00, 8b=0x2e208c00, 8h=0x6e608c00,
                                                                                                                                                                                                        // 4h=0x2e608c00. Disjoint from cmhi (0x3/0x34), bit (0x1c), etc.
                                                                                                                                                                                                        let ce = insn & 0xffe0_fc00;
                                                                                                                                                                                                                if let Some((lanes, esize, teq)) = match ce {
                                                                                                                                                                                                                    0x6ee0_8c00 => Some((2u8, 8u8, false)),  // cmeq 2d
                                                                                                                                                                                                                    0x6ea0_8c00 => Some((4u8, 4u8, false)),  // cmeq 4s
                                                                                                                                                                                                                    0x2ea0_8c00 => Some((2u8, 4u8, false)),  // cmeq 2s
                                                                                                                                                                                                                    0x6e20_8c00 => Some((16u8, 1u8, false)), // cmeq 16b
                                                                                                                                                                                                                    0x2e20_8c00 => Some((8u8, 1u8, false)),  // cmeq 8b
                                                                                                                                                                                                                    0x6e60_8c00 => Some((8u8, 2u8, false)),  // cmeq 8h
                                                                                                                                                                                                                    0x2e60_8c00 => Some((4u8, 2u8, false)),  // cmeq 4h
                                                                                                                                                                                                                    // cmtst: same layout but byte0 0x4e/0x0e (bit29 clear)
                                                                                                                                                                                                                    0x4ee0_8c00 => Some((2u8, 8u8, true)),  // cmtst 2d
                                                                                                                                                                                                                    0x4ea0_8c00 => Some((4u8, 4u8, true)),  // cmtst 4s
                                                                                                                                                                                                                    0x0ea0_8c00 => Some((2u8, 4u8, true)),  // cmtst 2s
                                                                                                                                                                                                                    0x4e20_8c00 => Some((16u8, 1u8, true)), // cmtst 16b
                                                                                                                                                                                                                    0x0e20_8c00 => Some((8u8, 1u8, true)),  // cmtst 8b
                                                                                                                                                                                                                    0x4e60_8c00 => Some((8u8, 2u8, true)),  // cmtst 8h
                                                                                                                                                                                                                    0x0e60_8c00 => Some((4u8, 2u8, true)),  // cmtst 4h
                                                                                                                                                                                                                    _ => None,
                                                                                                                                                                                                                } {
                                                                                                                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                    if teq {
                                                                                                                                                                                                                                    return Inst::SimdCmTest { rd, rn, rm, lanes, esize };
                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                return Inst::SimdCmEq { rd, rn, rm, lanes, esize };
                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                    // ---- SIMD table lookup: tbl Vd.16B, {Vn..Vn+N}, Vm (multi-register form) ----
                                                                                                                                                                                                                                                                                                                                                                            // Gate (insn & 0xffe0_9c00)==0x4e00_0000 (drops Vd/Vn/len/Vm; rejects smull
                                                                                                                                                                                                                                                                                                                                                                            // 0x4e62c020). tbx sets 0x1000; len = tables-1 (bits13:14).
                                                                                                                                                                                                                                                                                                                                                                            if (insn & 0xffe0_9c00) == 0x4e00_0000 {
                                                                                                                                                                                                                                                                                                                                                                                let vn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                let vd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                let vm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                let tbx = (insn & 0x1000) != 0;
                                                                                                                                                                                                                                                                                                                                                                                let n_tables = (((insn >> 13) & 0x1f) + 1) as u8; // len+1 (bits14:13 more...: bits 14:13)
                                                                                                                                                                                                                                                                                                                                                                                return Inst::Tbl { rd: vd, rn: vn, rm: vm, tbx, n_tables: n_tables.min(4) };
                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                // Saturating narrow sqxtn/uqxtn/sqxtun/uqxtun Vd.T, Vn.U. Precedes the plain
                                                                                                                                                                                                                                                                                // xtn gate. byte2 0x48 = saturating, 0x28 = truncating-but-saturating (sqxtun)
                                                                                                                                                                                                                                                                                // for the 0x2e/4e/6e prefixes; plain xtn is (0x0e,0x28) and falls through.
                                                                                                                                                                                                                                                                                let b0s = (insn >> 24) & 0xff;
                                                                                                                                                                                                                                                                                let b2s = (insn >> 8) & 0xff;
                                                                                                                                                                                                                                                                                if (b0s == 0x0e || b0s == 0x2e || b0s == 0x4e || b0s == 0x6e)
                                                                                                                                                                                                                                                                                    && (b2s == 0x28 || b2s == 0x48)
                                                                                                                                                                                                                                                                                    && !(b0s == 0x0e && b2s == 0x28)
                                                                                                                                                                                                                                                                                {
                                                                                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    // dst_esize from byte1 bit6 (0x21->1, 0x61->2); b1 high bit=1u -> 4
                                                                                                                                                                                                                                                                                    let b1 = ((insn >> 16) & 0xff) as u32;
                                                                                                                                                                                                                                                                                    let dst_esize: u8 = match (b1 >> 4) & 0xf {
                                                                                                                                                                                                                                                                                        0x2 => 1,   // 0x21: 8b <- 8h
                                                                                                                                                                                                                                                                                        0x6 => 2,   // 0x61: 4h <- 4s
                                                                                                                                                                                                                                                                                        _ => 1,
                                                                                                                                                                                                                                                                                    };
                                                                                                                                                                                                                                                                                    let b1q = (b0s >> 1) & 1; // byte0 bit1 == sq (sign dst) family bit
                                                                                                                                                                                                                                                                                    return Inst::SaturatNarrow {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           rd, rn, dst_esize,
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           src_signed: (b0s & 0x40) == 0,  // 0x0e/0x4e = signed src; 0x2e/0x6e unsigned
                                                                                                                                                                                                                                                                                        dst_signed: b1q == 0 && (b0s & 0x20) == 0,
                                                                                                                                                                                                                                                                                        q: (insn >> 30) & 1 == 1,
                                                                                                                                                                                                                                                                                    };
                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                // addhn/subhn/raddhn: dst = high half of src-width sum/diff, narrowed.
                                                                                                                                                                                                                                                                                // byte2 {0x40(hn-add),0x60(hn-sub)}; prefix 0x0e/0x2e; dst_esize=1<<bits[23:22].
                                                                                                                                                                                                                                                                                let hb0 = (insn >> 24) & 0xff;
                                                                                                                                                                                                                                                                                let hb2 = (insn >> 8) & 0xff;
                                                                                                                                                                                                                                                                                if (hb0 == 0x0e || hb0 == 0x2e) && (hb2 == 0x40 || hb2 == 0x60)
                                                                                                                                                                                                                                                                                    && (insn & 0x2000) == 0 {
                                                                                                                                                                                                                                                                                    let dst_esize: u8 = 1 << ((insn >> 22) & 3);
                                                                                                                                                                                                                                                                                    return Inst::SimdHighNarrow {
                                                                                                                                                                                                                                                                                        rd: (insn & 0x1f) as u8,
                                                                                                                                                                                                                                                                                        rn: ((insn >> 5) & 0x1f) as u8,
                                                                                                                                                                                                                                                                                        rm: ((insn >> 16) & 0x1f) as u8,
                                                                                                                                                                                                                                                                                        dst_esize,
                                                                                                                                                                                                                                                                                        sub: hb2 == 0x60,
                                                                                                                                                                                                                                                                                        round: (insn >> 29 & 1) == 1, // raddhn/rsubhn (bit29 set)
                                                                                                                                                                                                                                                                                    };
                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                // Low-half truncation. Gate &0xffe0_fc00 residues (all Q=0, no
                                                                                                                                                                                                                                                                                // collision): 0x0e202800 (8b<8h), 0x0e602800 (4h<4s), 0x0ea02800 (2s<2d).
                                                                                                                                                                                                                                                                                let xe = insn & 0xffe0_fc00;
                                                                                                                                                                                                                                                                                let xdst = match xe {
                                                                                                                                                                                                                                                                                    0x0e20_2800 => Some(1u8), // 8b <- 8h
                                                                                                                                                                                                                                                                                    0x0e60_2800 => Some(2u8), // 4h <- 4s
                                                                                                                                                                                                                                                                                    0x0ea0_2800 => Some(4u8), // 2s <- 2d
                                                                                                                                                                                                                                                                                    _ => None,
                                                                                                                                                                                                                                                                                };
                                                                                                                                                                                                                                                                                if let Some(de) = xdst {
                                                                                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    return Inst::SimdXtn { rd, rn, dst_esize: de };
                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                    // ---- SIMD load-and-replicate: ld1r {Vt.T}, [Xn] ----
                                                                                                                                                                                                                                                                                                                                                                    // Gate (insn & 0xfff0_fc00) == 0x0d40_c000. esize=1<<(bits[11:10]),
                                                                                                                                                                                                                                                                                                                                                                    // q=bit30. Loads one element from [x[rn]] and replicates it across
                                                                                                                                                                                                                                                                                                                                                                    // 16 (q) or 8 (q=0) bytes of V[rd].
                                                                                                                                                                                                                                                                                                                                                                    if matches!(insn & 0xfff0_fc00, 0x4d40_c000 | 0x4d40_c400 | 0x4d40_c800 | 0x4d40_cc00) {
                                                                                                                                                                                                                                                                                                                                                                        let size = (insn >> 10) & 3;
                                                                                                                                                                                                                                                                                                                                                                        let esize = 1u8 << (size as u8);
                                                                                                                                                                                                                                                                                                                                                                        let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                                                                                                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                        return Inst::Ld1 { rd, rn, esize, q };
                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                    // ---- SIMD single-vector load/store: ld1/st1 {Vt.T}, [Xn], #imm ----
                                                                                                                                                                                                                                                                                                                                                                                    // Gate (insn & 0x3fc0_0000)==0x0cc0_0000 (single-reg ld1/st1, q=bit30).
                                                                                                                                                                                                                                                                                                                                                                                    // ld1 (bit13 set) vs st1 (bit13 clear).
                                                                                                                                                                                                                                                                                                                                                                                    if (insn & 0x3fc0_0000) == 0x0cc0_0000 || (insn & 0x3fc0_0000) == 0x0c80_0000 {
                                                                                                                                                                                                                                                                                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let bytes = if (insn >> 30) & 1 == 1 { 16 } else { 8 };
                                                                                                                                                                                                                                                                                                                                                                                        if (insn & 0x3fc0_0000) == 0x0cc0_0000 { return Inst::Ld1V { rd, rn, bytes }; }
                                                                                                                                                                                                                                                                                                                                                                                        else { return Inst::St1V { rd, rn, bytes }; }
                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                     // ---- SIMD copy 64-bit lane (INS vector): mov Vd.d[dst], Vn.d[src] ----
                                                                                                                                                                                                                                                                                                                                                                        // Gate (insn & 0xffe0_0c00)==0x6e00_0400 (Q=1 INS element-from-element).
                                                                                                                                                                                                                                                                                                                                                                        // d-lane: imm5 field (insn>>16)&0x1f lowest set bit == 0x8 (esize 8).
                                                                                                                                                                                                                                                                                                                                                                        // dst index = bit20, src index = bit14 (verified vs aarch64 assembler).
                                                                                                                                                                                                                                                                                                                                                                        if (insn & 0xffe0_0c00) == 0x6e00_0400 {
                                                                                                                                                                                                                                                                                                                                                                                    let f = (insn >> 16) & 0x1f;
                                                                                                                                                                                                                                                                                                                                                                                    if f.trailing_zeros() <= 3 {
                                                                                                                                                                                                                                                                                                                                                                                                                                            let esize = (1 << f.trailing_zeros()) as u8; // 8(d)/4(s)/2(h)/1(b)
                                                                                                                                                                                                                                                                                                                                                                                                                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let dst_idx = ((insn >> 20) & 1) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let src_idx = ((insn >> 14) & 1) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        return Inst::SimdInsD { rd, rn, dst_idx, src_idx, esize };
                                                                                                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                // ---- SIMD dup (vector, element): dup Vd.T, Vn.T[i] ----
                                                                                                                                                                                                                                                                                                                                                                                                                                                                // Gate (insn & 0xffe0_0c00) in {0x0e00_0400, 0x4e00_040019} (Q=bit30).
                                                                                                                                                                                                                                                                                                                                                                                                                                                                // Disjoint from InsD (0x6e00_0400) via top byte. Broadcast Vn.T[src]
                                                                                                                                                                                                                                                                                                                                                                        // the imm5 field; src_idx = imm5 >> es.bit_length(). Broadcast Vn.T[src]
                                                                                                                                                                                                                                                                                                                                                                        // across all lanes of Vd.
                                                                                                                                                                                                                                                                                                                                                                        if ((insn & 0xffe0_0c00) == 0x0e00_0400 || (insn & 0xffe0_0c00) == 0x4e00_0400) {
                                                                                                                                                                                                                                                                                                                                                                            let f = (insn >> 16) & 0x1f;
                                                                                                                                                                                                                                                                                                                                                                            let esize = (1 << ((f & f.wrapping_neg()).trailing_zeros())) as u8;
                                                                                                                                                                                                                                                                                                                                                                            let src_idx = (f >> (esize.ilog2() + 1)) as u8;
                                                                                                                                                                                                                                                                                                                                                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                            let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                            let q = ((insn >> 30) & 1) == 1;
                                                                                                                                                                                                                                                                                                                                                                            return Inst::SimDup { rd, rn, esize, src_idx, q };
                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                    // ---- SIMD bitwise insert: bit Vd.16B, Vn.16B, Vm.16B ----
                                                                                                                                        // Gate (insn & 0xffe0_fc00)==0x6ea01c00 (16B; real 0x6ea11c40). Disjoint
                                                                                                                                        // from orr16 (0x4ea01c00, bit31) and cmhi (0x6ea03400). Out=(Vn&Vm)|(Vd&~Vm).
                                                                                                                                        if (insn & 0xffe0_fc00) == 0x6ea0_1c00 {
                                                                                                                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                    return Inst::SimdBit { rd, rn, rm };
                                                                                                                                                                                                                }
                                                                                                                                                                                                                if (insn & 0xffe0_fc00) == 0x2ea0_1c00 {
                                                                                                                                                                                                                                                                                            let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                            let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                            return Inst::SimdBit { rd, rn, rm };
                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                        // ---- SIMD extract immediate: ext Vd.16B/Vd.8B, Vn., Vm., #imm ----
                                                                                                        // Gate (insn & 0xffe0_0400) == 0x6e000000 (16B, Q=1) / 0x2e000000
                                                                                                        // (8B, Q=0). Verified disjoint from Ucvtf2d (0x6e60d800), SimdCmhi
                                                                                                        // (0x6ea03400), SimdBit (0x6ea01c00), Simd2dFp (0x6e60fc00),
                                                                                                        // InsD1D0 (0x6e180400), and the orr16/mul gates. imm = bits[15:11]
                                                                                                        // (byte count). Real: ext v1.16b,v0.16b,v0.16b,#8 = 0x6ee004001.
                                                                                                        {
                                                                                                            let sxe = insn & 0xffe0_0400;
                                                                                                            if sxe == 0x6e00_0000 || sxe == 0x2e00_0000 {
                                                                                                                let q = sxe == 0x6e00_0000;
                                                                                                                let imm = ((insn >> 11) & 0x1f) as u8;
                                                                                                                // imm must fit the vector: 0..15 (16B) / 0..7 (8B).
                                                                                                                let hi = if q { 15u8 } else { 7u8 };
                                                                                                                if imm <= hi {
                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                    return Inst::SimdExt { rd, rn, rm, imm, q };
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                        // ---- SIMD lane extract to GPR: mov/umov/smov Wd,Xd, Vn.T[idx] ----
                                                                                                        // Gate (insn & 0xffe0_0c00) in {0x0e000c00 (Wd dest), 0x4e000c00 (Xd dest)}.
                                                                                                            // esize from imm5 trailing-zeros: p=ctz(imm5)+1 => esize=1<<(p-1);
                                                                                                            // index = imm5 >> p. sign flag = bit12 clear (SMOV). Verified disjoint from
                                                                                                            // orr16 (0x4ea41c40), mul (0x0ea09c00), cmhi, bit, InsDv1D0 (0x4e18400),
                                                                                                            // dup, ucvtf, and the add lane form (they don't mask to the 0x0c00 residue).
                                                                                                            {
                                                                                                                let gt = insn & 0xffe0_0c00;
                                                                                                                if (gt == 0x0e00_0c00 || gt == 0x4e00_0c00) {
                                                                                                                    let imm5 = (insn >> 16) & 0x1f;
                                                                                                                    let p = 1u32 + imm5.trailing_zeros();
                                                                                                                    let esize = (1u8 << (p - 1)) as u8; // 1,2,4,8
                                                                                                                    if esize == 4 || esize == 8 {
                                                                                                                        let index = (imm5 >> p) as u8;
                                                                                                                        let wide = gt == 0x4e00_0c00;
                                                                                                                        // sign (SMOV) when bit12 clear (umov has it set).
                                                                                                                        let sign = (insn & 0x1000) == 0;
                                                                                                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                        let rd = (insn & 0x1f) as u8;
                                                                                                                        return Inst::SimdLaneGp { rd, rn, esize, index, sign, wide };
                                                                                                                    }
                                                                                                                }
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
    // ---- memory/DMB/DSB/ISB barriers: no-op for a single-threaded JIT ----
    // Gate (insn & 0xfffff01f) == 0xd503_301f catches dmb/dsb/isb (which share
    // 0xd503_301f, differing only in the barrier opt field). Distinct from the
    // Hint gate above (0x20) and from real mrs/msr/tlb (nonzero rt / other fields).
    if (insn & 0xffff_f01f) == 0xd503_301f {
        return Inst::WaitBarrier;
    }

    // ---- supervisor call: svc #imm (0xd4000001 | imm<<5) -> host syscall ----
    if (insn & 0xffe0_001f) == 0xd400_0001 {
        let imm = ((insn >> 5) & 0xffff) as u16;
        return Inst::Svc { imm };
    }

    // ---- breakpoint: brk #imm (0xd4200000 | imm<<5) -> guest trap ----
    if (insn & 0xffe0_001f) == 0xd420_0000 {
        let imm = ((insn >> 5) & 0xffff) as u16;
        return Inst::Brk { imm };
    }

    // ---- undefined instruction: udf #imm = 0x0000_0000 (high 16 bits zero) ----
    // AArch64 defines UDF as an always-undefined trap. 0x00000000, at our wall,
    // is `udf #0`. Mirror Brk: decode it so translate can halt gracefully.
    if (insn >> 16) == 0 {
        return Inst::Udf { imm: (insn & 0xffff) as u16 };
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
        // mrs xN, nzcv = 0xd53b4201: op1=3, CRn=4, CRm=2, op2=0. Read the packed
        // condition flags (N=31,Z=30,C=29,V=28 from CpuState.nzcv) into rt.
        if op1 == 3 && crn == 4 && crm == 2 && op2 == 0 && read {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 4, rt, read };
        }
        // msr nzcv, xN = 0xd51b420d: W is a 0xD51b... (bit21 L=0). Write xN's low
        // 4 flag bits (N=31,Z=30,C=29,V=28 of xN) into CpuState.nzcv.
        if op1 == 3 && crn == 4 && crm == 2 && op2 == 0 && !read {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 4, rt, read };
        }
    }

    // ---- load/store pair (X: 0xa8/0xa9, W: 0x28/0x29, SIMD Q 128-bit: 0xAD, FP/vec d: 0x6d/0x2d) ----
    if matches!(insn >> 24, 0x29 | 0x69 | 0x28 | 0xa9 | 0xa8 | 0xac | 0xad | 0x6d | 0x2d | 0x6c | 0x2c) {
        let q128 = (insn >> 24) & 0xff == 0xad || (insn >> 24) & 0xff == 0xac; // 128-bit SIMD pair (ldp/stp q)
        let fp_d = (insn >> 24) & 0xff == 0x6d || (insn >> 24) & 0xff == 0x2d
            || (insn >> 24) & 0xff == 0x6c || (insn >> 24) & 0xff == 0x2c; // FP/vec d pair (offset+indexed)
        let sext_en = (insn >> 24) & 0xff == 0x69; // ldpsw: sign-ext the 32-bit pair to 64-bit
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
            sext: sext_en,
        };
    }

    // ---- SIMD lane load: ld1 {Vt.T}[idx], [Xn] (single-element, byte) ----
    // Verified vs 25 asm ground-truth encodings (fixed-base index sweep + varied base):
    // prefix 0x0d (idx 0-7) / 0x4d (idx 8-15), size bits 11:10 nonzero, index = bits[14:10]
    // plus +8 when the prefix is 0x4d. Branch-table-indexed lane loads in rblx use this.
    if matches!((insn >> 24) & 0xff, 0x0d | 0x4d) {
        let hi8 = ((insn >> 24) & 0xff) == 0x4d;
        return Inst::Ld1L {
            rd: (insn & 0x1f) as u8,
            rn: b(insn, 5, 9) as u8,
            esize: 1,
            index: (((insn >> 10) & 0x1f) + if hi8 { 8 } else { 0 }) as u8,
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
                sext,
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
                sext,
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
                sext,
            } => {
                assert_eq!(rt, 0);
                assert_eq!(rn, 0);
                assert_eq!(imm, 2);
                assert_eq!(size, 8);
                assert!(ld);
                assert!(!sext);
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
                sext,
            } => {
                assert_eq!(rt, 1);
                assert_eq!(rn, 0);
                assert_eq!(imm, 3);
                assert_eq!(size, 8);
                assert!(!ld);
                assert!(!sext);
            }
            other => panic!("expected LdStrImm, got {:?}", other),
        }
    }
    #[test]
    fn ldrsw_sign_extend_load_ground_truth() {
        // Regression: `ldrsw x0,[sp,#20]` is a SIGN-EXTEND LOAD, not a store. It
        // encodes bit22=0 (like STR) but bit23=1; mis-decoding it as a store
        // corrupted memory in int/factorial loops. Encodings from
        // aarch64-linux-gnu-as (rn=x1, rt=x0):
        //   ldrsw x0,[x1,#20]=0xb9801420 ; ldrsh x0,[x1,#20]=0x79802820
        //   ldrsb x0,[x1,#20]=0x39805020 ; str w0,[x1,#20]=0xb9001420
        match decode(0xb9801420) {
            Inst::LdStrImm {
                rt, rn, imm, size, ld, sext,
            } => {
                assert_eq!((rt, rn, imm, size, ld, sext), (0, 1, 5, 4, true, true));
            }
            other => panic!("ldrsw -> sext LdStrImm, got {other:?}"),
        }
        match decode(0x79802820) {
            Inst::LdStrImm { size, ld, sext, .. } => {
                assert_eq!((size, ld, sext), (2, true, true));
            }
            other => panic!("ldrsh -> sext LdStrImm, got {other:?}"),
        }
        match decode(0x39805020) {
            Inst::LdStrImm { size, ld, sext, .. } => {
                assert_eq!((size, ld, sext), (1, true, true));
            }
            other => panic!("ldrsb -> sext LdStrImm, got {other:?}"),
        }
        // A genuine STR is NOT a sign-extend load.
        match decode(0xb9001420) {
            Inst::LdStrImm { ld, sext, .. } => {
                assert!(!ld && !sext);
            }
            other => panic!("str w -> plain store, got {other:?}"),
        }
    }
    #[test]
    fn fp_reg_ldst_decode_ground_truth() {
        // Regression: FP/vector register loads/stores must touch the VECTOR
        // file, not be mis-decoded as GPR x/w ops. (Previously the GPR gate left
        // bit26 unmasked, so str d0/ldr s0/ldr q all became LdStrImm on the GPR
        // file — str q even as a 1-byte GPR load.) Encodings from
        // aarch64-linux-gnu-as/objdump ground truth.
        //   str d0,[x8,#8] = 0xfd000500 ; ldr d31,[x1,#16] = 0xfd40083f
        //   ldr s1,[x2]    = 0xbd400041
        //   str q6,[x3,#16]= 0x3d800466 ; ldr q7,[x4,#32]= 0x3dc00887
        match decode(0xfd000500) {
            Inst::FpLdStImm { vt, rn, imm, size, ld } => {
                assert_eq!((vt, rn, imm, size, ld), (0, 8, 1, 8, false));
            }
            other => panic!("str d0 -> FpLdStImm, got {other:?}"),
        }
        match decode(0xfd40083f) {
            Inst::FpLdStImm { vt, rn, imm, size, ld } => {
                assert_eq!((vt, rn, imm, size, ld), (31, 1, 2, 8, true));
            }
            other => panic!("ldr d31 -> FpLdStImm, got {other:?}"),
        }
        match decode(0xbd400041) {
            Inst::FpLdStImm { vt, rn, imm, size, ld } => {
                assert_eq!((vt, rn, imm, size, ld), (1, 2, 0, 4, true));
            }
            other => panic!("ldr s1 -> FpLdStImm, got {other:?}"),
        }
        // 128-bit q must go to VecLdStImm (used to be swallowed as a 1-byte GPR load).
        match decode(0x3dc00887) {
            Inst::VecLdStImm { vt, rn, .. } => {
                assert_eq!((vt, rn), (7, 4));
            }
            other => panic!("ldr q7 -> VecLdStImm, got {other:?}"),
        }
        match decode(0x3d800466) {
            Inst::VecLdStImm { vt, rn, ld, .. } => {
                assert_eq!((vt, rn, ld), (6, 3, false));
            }
            other => panic!("str q6 -> VecLdStImm, got {other:?}"),
        }
        // Plain GPR unchanged.
        assert!(matches!(decode(0xf9000c01), Inst::LdStrImm { rt: 1, rn: 0, .. }));
        // Scalar FP->int into an FP register must be FcvVec (scalar D, single 64-bit
        // lane), NOT the SIMD widen gate which used to mis-catch 0x5ee1bbff.
        match decode(0x5ee1bbff) {
            Inst::FcvVec { rd, rn, signed, esize, q } => {
                assert_eq!((rd, rn, signed, esize, q), (31, 31, true, 8, false));
            }
            other => panic!("fcvtzs d31,d31 -> FcvVec scalar, got {other:?}"),
        }
        match decode(0x7ee1b8e0) {
            Inst::FcvVec { rd, signed, .. } => {
                assert_eq!((rd, signed), (0, false)); // fcvtzu -> unsigned
            }
            other => panic!("fcvtzu d0,d7 -> FcvVec unsigned, got {other:?}"),
        }
    }
    #[test]
    fn and_immediate_is_logic_imm_not_movn() {
        // Regression: `and w1,w0,#0xffff` (0x12003c20, LogicalImmediate) was
        // decoded as `movn` — the MoveWide gate matched the whole `top` byte
        // {0x12,0x92,0x52,...}, which also spans the AND/EOR/ANDS-immediate
        // class. MoveWide now keys on bits[28:23]==0x25 (0x1280_0000), so AND
        // immediates route to LogicImm. Real compiler encodings:
        //   and w0,w1,#0xffff = 0x12003c20 ; and x0,x1,#0xff = 0x92401c20
        match decode(0x12003c20) {
            Inst::LogicImm { rd, rn, mask, op, sf } => {
                assert_eq!((rd, rn, mask, op, sf), (0, 1, 0xffff, 0, false));
            }
            other => panic!("and w1,w0,#0xffff -> LogicImm, got {other:?}"),
        }
        match decode(0x92401c20) {
            Inst::LogicImm { mask, op, sf, .. } => {
                assert_eq!((mask, op, sf), (0xff, 0, true));
            }
            other => panic!("and x1,x0,#0xff -> LogicImm, got {other:?}"),
        }
        // MoveWide still decodes correctly.
        assert!(matches!(decode(0xd2917620), Inst::MoveWide { opc: 0, .. }));
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
                sext,
            } => {
                assert_eq!(rt, 0);
                assert!(!sext);
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
                            fn clz_scalar_ucvtf_decode() {
                                // clz w8, w24 = 0x5ac01308 (real libroblox LocalStorage): Clz W, sf=false.
                                match decode(0x5ac01308) {
                                    Inst::ClzCls { rd, rn, sf, cls } => {
                                        assert_eq!(rd, 8);
                                        assert_eq!(rn, 24);
                                        assert!(!sf);
                                        assert!(!cls);
                                    }
                                    other => panic!("clz w8,w24 -> {other:?}"),
                                }
                                // clz x9, x2 = 0xdac01400-ish: X (sf=true) form. (0xdac01009)
                                match decode(0xdac01009) {
                                    Inst::ClzCls { rd, rn, sf, cls } => {
                                        assert_eq!(rd, 9);
                                        assert_eq!(rn, 0);
                                        assert!(sf);
                                        assert!(!cls);
                                    }
                                    other => panic!("clz x9,x0 -> {other:?}"),
                                }
                                // cls w8, w24 = 0x5ac01708 (signs): cls flag set.
                                match decode(0x5ac01708) {
                                    Inst::ClzCls { cls, .. } => {
                                        assert!(cls);
                                    }
                                    other => panic!("cls -> {other:?}"),
                                }
                                // scalar ucvtf d0, d1 = 0x7e61d820 (real libroblox audio mix): ScalarUcvtf.
                                match decode(0x7e61d820) {
                                    Inst::ScalarUcvtf { rd, rn, sng: _ } => {
                                        assert_eq!(rd, 0);
                                        assert_eq!(rn, 1);
                                    }
                                    other => panic!("ucvtf d0,d1 -> {other:?}"),
                                }
                                // vector ucvtf v2.2d,v2.2d must NOT decode as scalar.
                                assert!(!matches!(decode(0x6e61d842), Inst::ScalarUcvtf { .. }));
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
            Inst::FcvtToInt { rd, rn, mode, sf, unsigned, .. } => {
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
        // fcvtpu x9, s0 = 0x9e290009 (real libroblox) => round +inf, unsigned, single-src.
        match decode(0x9e290009) {
            Inst::FcvtToInt { rd, rn, mode, sf, unsigned, src_sng } => {
                assert_eq!(rd, 9);
                assert_eq!(rn, 0);
                assert_eq!(mode, 3); // round toward +inf
                assert!(sf); // X dest
                assert!(unsigned); // fcvtpu (unsigned)
                assert!(src_sng); // single (s) source
            }
            other => panic!("fcvtpu x9,s0 -> {other:?}"),
        }
        // fcvtpu x9, d0 (double-src form = 0x9e690009) => src_sng false, still +inf.
        match decode(0x9e690009) {
            Inst::FcvtToInt { mode, unsigned, src_sng, .. } => {
                assert_eq!(mode, 3);
                assert!(unsigned);
                assert!(!src_sng); // double source
            }
            other => panic!("fcvtpu x9,d0 -> {other:?}"),
        }

        // BFXIL: bfxil w8, w9, #0, #1 = 0x33000128 (real libroblox boot wall) is
        // the non-wrap BFM insert -- must decode as BitField with insert flag set.
        match decode(0x33000128) {
            Inst::BitField { rd, rn, immr, imms, sf, arith, insert } => {
                assert_eq!(rd, 8);
                assert_eq!(rn, 9);
                assert_eq!(immr, 0);
                assert_eq!(imms, 0);
                assert!(!sf); // 32-bit (w)
                assert!(!arith);
                assert!(insert); // BFM insert (not a plain extract)
            }
            other => panic!("bfxil w8,w9,#0,#1 -> {other:?}"),
        }
        // BFM (opc=00, non-wrap) with immr<=imms must NOT be miscast as a plain
        // UBFM extract that would overwrite Rd's other bits on translate.
        assert!(matches!(
            decode(0xb3400000), // bfxil x0,x0,#0,#1 (X-form, immr=imms=0)
            Inst::BitField { insert: true, sf: true, .. }
        ));
        // fcmp d6,d16 = 0x1e7020c0 must NOT decode as FcvtToInt round (regression guard).
        assert!(!matches!(decode(0x1e7020c0), Inst::FcvtToInt { .. }));
        // fmov d6,d0 = 0x1e604006 must still be FmovFp (not round-fcvt).
        assert!(matches!(decode(0x1e604006), Inst::FmovFp { .. }));
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
            Inst::Fcmp { rn, rm, sz } => {
                assert_eq!(rn, 7);
                assert_eq!(rm, 6);
                assert!(sz);
            }
            other => panic!("fcmp d7,d6 -> {other:?}"),
        }
        // fcmp d6, d16 = 0x1e7020c0 (real libroblox; high rm reg folded into the
        // base nibble) → must still decode as Fcmp with rm=16.
        match decode(0x1e7020c0) {
            Inst::Fcmp { rn, rm, sz } => {
                assert_eq!(rn, 6);
                assert_eq!(rm, 16);
                assert!(sz);
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
            Inst::SimdDupGp { rd, rn, esize, q } => {
                assert_eq!(rd, 1);
                assert_eq!(rn, 10);
                assert_eq!(esize, 4);
                assert!(q);
            }
            other => panic!("dup v1.4s,w10 -> {other:?}"),
        }
        // cmhi v1.4s, v3.4s, v1.4s = 0x6ea4c1c1 (real libroblox audio mix) => SimdCmhi.
        match decode(0x6ea13461) {
            Inst::SimdCmhi { rd, rn, rm, lanes } => {
                assert_eq!(rd, 1);
                assert_eq!(rn, 3);
                assert_eq!(rm, 1);
                assert_eq!(lanes, 4);
            }
            other => panic!("cmhi v1.4s,v3.4s,v1.4s -> {other:?}"),
        }
        // bit v0.16b, v2.16b, v1.16b = 0x6ea11c40 (real libroblox audio mix) => SimdBit.
        match decode(0x6ea11c40) {
            Inst::SimdBit { rd, rn, rm } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 2);
                assert_eq!(rm, 1);
            }
            other => panic!("bit v0.16b,v2.16b,v1.16b -> {other:?}"),
        }
        // ext v1.16b, v0.16b, v0.16b, #8 (real libroblox boot stop) => SimdExt 16B imm=8.
        match decode(0x6e004001) {
            Inst::SimdExt { rd, rn, rm, imm, q } => {
                assert_eq!(rd, 1);
                assert_eq!(rn, 0);
                assert_eq!(rm, 0);
                assert_eq!(imm, 8);
                assert!(q);
            }
            other => panic!("ext v1.16b,v0.16b,v0.16b,#8 -> {other:?}"),
        }
        // ext v2.16b, v3.16b, v4.16b, #15 (max 16B imm) => SimdExt.
        match decode(0x6e047862) {
            Inst::SimdExt { rd, rn, rm, imm, q } => {
                assert_eq!(rd, 2);
                assert_eq!(rn, 3);
                assert_eq!(rm, 4);
                assert_eq!(imm, 15);
                assert!(q);
            }
            other => panic!("ext v2.16b,v3.16b,v4.16b,#15 -> {other:?}"),
        }
        // ext v5.8b, v6.8b, v6.8b, #1 (8B form) => SimdExt Q=0.
        match decode(0x2e0708c5) {
            Inst::SimdExt { rd, rn, rm, imm, q } => {
                assert_eq!(rd, 5);
                assert_eq!(rn, 6);
                assert_eq!(rm, 7);
                assert_eq!(imm, 1);
                assert!(!q);
            }
            other => panic!("ext v5.8b,v6.8b,v6.8b,#1 -> {other:?}"),
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
        // fmla v29.4s, v21.4s, v2.s[0] = 0x4f8212bd (real boot wall) => FmlaEl.
        match decode(0x4f8212bd) {
            Inst::FmlaEl { rd, rn, vlm, idx, q, sub, .. } => {
                assert_eq!(rd, 29);
                assert_eq!(rn, 21);
                assert_eq!(vlm, 2);
                assert_eq!(idx, 0);
                assert!(q);
                assert!(!sub);
            }
            other => panic!("fmla v29.4s,v21,v2.s[0] -> {other:?}"),
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
