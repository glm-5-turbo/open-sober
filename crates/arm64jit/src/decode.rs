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
        // bit21: 0 = shifted-register form (regs 31 are XZR), 1 = extended-register
        // form (rn/rd 31 are SP). Distinguishes `neg` (0xcb0603e6, bit21=0, rn=31
        // must read XZR=0) from `sub sp,sp,x1` (0xcb2163ff, bit21=1, rn=31 = SP).
        sp_operand: bool,
    },
    // ---- add/subtract register (extended-register form, bit21=1) ----
    // `add/sub Xd, Xn|SP, Rm, <opt> #<shift>` where Rm is a 32-bit (W) register
    // (unless opt is UXTX/SXTX, then 64-bit) extended per `opt` (0=UXTB,1=UXTH,
    // 2=UXTW,3=UXTX,4=SXTB,5=SXTH,6=SXTW,7=SXTX) then shifted by <shift>(0..3).
    // rn/rd of 31 are SP. This is a DISTINCT encoding from the shifted-register
    // form (bit21=0): decoding it through the shifted parser mis-reads the
    // option/shift bits as a bogus `lsl #sh_amt` (e.g. sxtw#3 came out as a
    // 51-bit shift) — a real glibc-startup x-register corruption.
    AddSubExt {
        rd: u8,
        rn: u8,
        rm: u8,
        sub: bool,
        s: bool,
        sf: bool,
        opt: u8,  // 0=UXTB 1=UXTH 2=UXTW 3=UXTX 4=SXTB 5=SXTH 6=SXTW 7=SXTX
        shift: u8, // 0..3
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
    // ---- integer multiply HIGH: umulh/smulh Xd, Xn, Xm (high 64 of 128-bit
    // product). X-only. Gate: top 0x9b && bit22 set (separates from the madd/
    // msub/udiv/sdiv MulDiv family where bit22=0). signed = bit23 (smulh=0).
    MulHigh {
        rd: u8,
        rn: u8,
        rm: u8,
        signed: bool,
    },
    // ---- memory-tagging (MTE) allocation-tag load/store: ldg/stg/stzg/st2g/...
    // The host has no MTE and the JIT keeps no tag state, so a store is a pure
    // no-op (it would tag memory, which we don't model) and `ldg` (the only
    // load) returns tag 0 into Xt. GLIBC's __libc_mtag_tag_region issues a
    // stg loop; without this a full glibc-linked program stops on the first tag
    // store. Gate (insn & 0xff3fe71c)==0xd9200000 (verified vs objdump across
    // the 0xd9 top-byte alloc-tag space); load = bit22=1 && bit11=0.
    MteTag { load: bool, rt: u8, rn: u8, wb: bool, wb_off: i64 },
    // ---- data/instruction cache maintenance: dc <op>,xN / ic <op>,xN ----
    // Single-threaded JIT (guest==host, no separate cache) => coherence ops are
    // no-ops. `dc zva` is the one that writes memory (zeros the 16-byte block we
    // advertise via dczid_el0) and so must emit the zeros, not just skip.
    CacheMaintain { zva: bool, rt: u8 },
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
    // ---- load/store (pre/post-index + unscaled) Signed imm9 ----
    // `ldr/str Xt, [Xn, #imm9]!` (pre), `[Xn], #imm9` (post) or `[Xn, #imm9]`
    // (unscaled). imm9 is a SIGNED byte offset (bits[20:12]); pre/post also
    // write Xn back (Xn += imm9). Distinct from LdStrImm (which is an UNSIGNED
    // offset scaled by size): this is the 0xf84x/0x38x signed-immediate family.
    LdStrImmWb {
        rt: u8,
        rn: u8,
        imm9: i32,      // signed byte offset
        size: u8,       // 1=byte,2=half,4=word,8=dword
        ld: bool,       // load=true, store=false
        sext: bool,     // sign-extending load (ldrsw/ldrsh/ldrsb)
        writeback: bool, // pre/post write Xn back; false = unscaled
        pre: bool,      // pre-index (access at Xn+imm9); post accesses at Xn
    },
    // ---- LSE atomics (ARMv8.1): ldadd/ldclr/ldeor/ldset/swp ----
    // Rt = [Rn]; [Rn] = f(Rt, Rs); (single-threaded: a plain read-modify-write).
    // op: 0=LDADD (add), 1=LDCLR (and-not), 2=LDEOR (xor), 3=LDSET (or),
    // 4=SWP (replace). size via bit30 (0 = W/32-bit, 1 = X/64-bit).
    LseAtomic {
        op: u8,
        size64: bool,
        rs: u8,
        rn: u8,
        rt: u8,
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
        // Option field (bits[14:13]) of the register-offset address operand,
        // telling how to extend the INDEX register rm before the shift:
        //   3 = LSL  (UXTX/SXTX) -> full 64-bit rm (the common `[xN,xM,lsl#S]`).
        //   2 = UXTW (W index)   -> zero-extend rm's low 32 bits (a `[xN,wM,
        //       uxtw#S]` array-index load; MUST mask to 32 — real book code
        //       stores a bit-32 sentinel in the X reg and expects it dropped).
        //   1 = UXTB -> zero-extend low byte.
        //   0 = reserved -> treat as full 64 (historical behavior).
        index_ext: u8,
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
        fp_d: bool,    // true => 64-bit FP/vector d-pair (ldp/stp d, 0x6d/0x6c)
        fp_s: bool,    // true => 32-bit FP/vector s-pair (ldp/stp s, 0x2d/0x2c)
        sext: bool,    // true => ldpsw (sign-extend 32-bit loads to 64-bit X regs)
    },
    // ---- SIMD/NEON 128-bit vector load/store (ldr q0,[xN,#imm] / str q) ----
    VecLdStImm {
        vt: u8, // vector register
        rn: u8,
        imm: u32, // scaled-by-16 byte offset
        ld: bool,
    },
    // ---- SIMD/NEON 128-bit vector load/store with a REGISTER offset ----
    // (ldr q0,[xN,xM] / str q0,[xN,xM]). AArch64 bit26=1 selects the vector file. Before
    // this class these matched the GPR register-offset gate (no bit26 check)
    // and a `str q` was mis-decoded as a 1-byte sign-extend GPR load INTO rn's
    // register (e.g. memset's str q0,[x0,x3] became `ldrsb x0,[x0,x3]`,
    // silently clobbering guest x0). Option/S bitfields are ignored (the real
    // forms use LSL #0); addr = x[rn] + x[rm].
    VecLdStrReg { vt: u8, rn: u8, rm: u8, ld: bool },
    // ---- SIMD/NEON 128-bit vector load/store, unscaled SIGNED imm9 ----
    // (ldur q0,[xN,#imm9] / stur q0,[xN,#imm9]). Same bit26=1 class hazard: the
    // stale unscaled GPR gate (0x38000000) mis-read these as 1-byte GPR ops.
    VecLdStImmUnscaled {
        vt: u8,
        rn: u8,
        imm9: i32,
        ld: bool,
    },
    // ---- SIMD/NEON 128-bit vector load/store, PRE/POST-index writeback ----
    // (ldr/str q0,[xN,#imm9]! and [xN],#imm9). Xn is advanced by the signed
    // imm9 after the transfer (a real side effect). bit26=1 (vector file).
    VecLdStIndexed {
        vt: u8,
        rn: u8,
        imm9: i32,
        ld: bool,
        pre: bool, // pre-index (advance then access) vs post-index
    },
    // ---- FP/SIMD scalar-register load/store with a REGISTER offset ----
    // (ldr/str b/h/s/d0,[xN,xM{,lsl#sh}]). bit26=1 (vector file). Same family as
    // VecLdStrReg (Q) but scalar width from bits[31:30] (1/2/4/8 B/H/S/D);
    // MIGHT the pre-fix GPR mis-decode (register-offset gate had no bit26 mask).
    FpLdStrReg {
        vt: u8,
        rn: u8,
        rm: u8,
        size: u8, // 1/2/4/8
        ld: bool,
        shift: bool, // S bit: scale index by log2(size)
        index_ext: u8, // option bits[14:13]: 3=LSL(x), 2=UXTW(w), 1=UXTB, 0=reserved
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
    // ---- FP/SIMD SCALAR (B/H/S/D) UNSCALED immediate load/store ----
    // (`ldur <s>t,[xN,#simm9]` / `stur`). Signed imm9 in bits[20:12], no
    // writeback. Same address math as VecLdStImmUnscaled (128-bit Q) but the
    // scalar widths transfer only `size` bytes into the low bytes of v[vt].
    FpLdStImmUnscaled {
        vt: u8,
        rn: u8,
        imm9: i32,
        size: u8, // 1/2/4/8
        ld: bool,
    },
    // ---- FP/SIMD SCALAR (B/H/S/D) PRE/POST-index writeback load/store ----
    // (`ldr/str <s>t,[xN,#simm9]!` (pre) / `[xN],#simm9` (post)). Signed imm9
    // in bits[20:12]; Xn advances by imm9 after the transfer (a real side
    // effect, like the Q VecLdStIndexed forms). `pre` selects pre-index.
    FpLdStImmWb {
        vt: u8,
        rn: u8,
        imm9: i32,
        size: u8, // 1/2/4/8
        ld: bool,
        pre: bool,
    },
    // ---- SIMD/NEON vector move-immediate (movi Vd.<T>, #imm) ----
    // `lo`/`hi` are the low/high 64-bit halves of the 128-bit result, already
    // expanded to the element size (each byte/word/dword lane set to #imm).
    // `kind`: 0 = write (movi/mvni), 1 = AND-in-place (bic, Vd &= ~imm),
    // 2 = OR-in-place (orr, Vd |= imm). Odd cmode (bit0=1) selects ORR/BIC;
    // even cmode selects MOVI/MVNI.
    VecMovi {
        vd: u8,
        lo: u64,
        hi: u64,
        kind: u8,
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
    // `upper` (Q=1 / sxtl2·uxtl2): the narrow source is the UPPER 64 bits of
    // Vn (bytes 8..15), not the lower — gcc vectorizes an int->i64 widening
    // init loop with sxtl (low) then sxtl2 (upper) to cover all lanes. Without
    // it, sxtl2 re-read the low half and the vector contents shifted.
    SimdXtl { rd: u8, rn: u8, sign: bool, esrc: u8, upper: bool },
    // ---- SIMD add/sub-wide: uaddw/saddw Vd.T, Vn.T, Vm.T/2 ----
    // `upper` (Q=1 / saddw2·uaddw2): the narrow source is the UPPER half of
    // Vm (bytes 8..15), not the lower half — gcc vectorizes string/math loops
    // with saddw then saddw2 to accumulate both halves.
    SimdAddw { rd: u8, rn: u8, rm: u8, sign: bool, esrc: u8, upper: bool, sub: bool },
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
    // ---- SIMD saturating shift-LEFT immediate: sqshl/uqshl/sqshlu
    // Vd.T, Vn.T, #imm ---- computes shl then saturates to the element width.
    // sqshl (signed dst/src): prefix 0x0f/0x4f, bits[14:12]==0b111, b2 0x74;
    // uqshl (unsigned ddst/src): 0x2f/0x6f, bits[14:12]==7, b2 0x74;
    // sqshlu (signed src -> unsigned dst): 0x2f/0x6f, bits[14:12]==0b110, b2 0x64.
    SimdSatShl { rd: u8, rn: u8, esize: u8, shift: u8, sat: u8 }, // sat: 0=u signed->signed,1=u unsigned,2=sqshlu
    // ---- SIMD shift-right accumulate: usra/ssra Vd.T, Vn.T, #imm (Vd += Vn >> imm) ----
    SimdShrAcc { rd: u8, rn: u8, esize: u8, shift: u8, unsigned: bool },
    // ---- SIMD saturating narrowing shift: sqshrn/uqshrn/sqshrun Vd.T, Vn.T, #imm ----
    // Right-shifts each esize-byte source element by `shift`, then saturating-
    // narrows to esize/2 bytes (like SaturatNarrow with a pre-shift). ΔSG from
    // plain ssra/usra (SimdShrAcc) is bit15 (0x8000) set; dst signed = bit29
    // clear (sqshrn); src signed for the unsigned-dst forms: sqshrun (signed
    // src, byte2 bit4 clear) vs uqshrn (unsigned src, byte2 bit4 set).
    SatNarrowShift { rd: u8, rn: u8, src_esize: u8, dst_esize: u8, shift: u8, src_signed: bool, dst_signed: bool, q: bool },
    // ---- SIMD plain shift-right immediate: ushr/sshr Vd.T, Vn.T, #imm ----
    // Marker bits[14:12]==0b000 (vs shl 0b101, usra/ssra 0b001); immh!=0 separates
    // from the modifed-immediate movi/mvni (which always have immh==0). unsigned =
    // bit29 (0x2f/0x6f). These previously fell into the broad VecMovi gate.
    SimdShr { rd: u8, rn: u8, esize: u8, shift: u8, unsigned: bool },
    // ---- SIMD shift-right-narrow: shrn/shrn2 Vd.T, Vn.T/2, #imm ----
    // Shift each DOUBLE-width source element right, truncate to the dest element
    // (dest is half the source width). `upper` (shrn2) writes the high dest
    // half. Distinguishable from plain ushr/sshr by bit15 (0x8000) set.
    SimdShrn { rd: u8, rn: u8, esrc: u8, shift: u8, upper: bool, round: bool },
    // ---- SIMD ld2 (load two vectors, deinterleaved) ----
    Ld2 { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    // ---- SIMD st2 (structure store of two vectors) ----
    St2 { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    // ---- SIMD ld1/st1 MULTIPLE structures (consecutive, NO deinterleave) ----
    // ld1/st1 {Vt.T, Vt2.T, ..} loads/stores `nreg` consecutive (q?16:8)-byte
    // vectors to/from V[rd], V[rd+1], .. — the plain array-copy idiom the
    // compiler emits for 2/3/4-element vector literals (differs from ld2/st2
    // which DEINTERLEAVE). nreg∈{1,2,3,4}; post is the writeback amount.
    Ld1N { rd: u8, rn: u8, nreg: u8, q: bool, post: i32 },
    St1N { rd: u8, rn: u8, nreg: u8, q: bool, post: i32 },
    // ---- ld3/st3 and ld4/st4: structure DEINTERLEAVE loads (gcc's
    // matrix-transpose idiom). Unlike ld1-multiple (consecutive), memory holds
    // the vectors' elements interleaved: Vd[j][i] = mem[base + i*N + j] where N
    // is the register count (3 or 4). esize is the element size in bytes.
    Ld3N { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    St3N { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    Ld4N { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    St4N { rd: u8, rn: u8, q: bool, post: i32, esize: u8 },
    // ---- scalar udiv/sdiv Wd/Wd/Wm ----
    Div { rd: u8, rn: u8, rm: u8, signed: bool, is_x: bool },
    // ---- SIMD variable register shift: ushl/sshl Vd.T, Vn.T, Vm.T ----
    SimdVShift { rd: u8, rn: u8, rm: u8, esize: u8, signed_: bool, q: bool },
    // ---- scalar FP max/min (fmax/fmin/fmaxnm/fminnm) ----
    FMaxMin { rd: u8, rn: u8, rm: u8, sz: bool, op: u8 },
    // ---- FP horizontal reduction cross vector: fmaxv/fminv Sd, Vn.4s ----
    FMaxV { rd: u8, rn: u8, min: bool },
    // ---- FP pairwise two-register reduction: fmaxp/fminp/fmaxnmp/fminnmp
    //      Vd, Vn (.2s/.2d). Reduces the two elements of Vn into a scalar
    //      result in Vd (bit16 CLEAR = two-register form; bit16 SET is the
    //      three-operand FMAXP Vd,Vn,Vm handled elsewhere). min = bit15;
    //      nm (fmaxnm/fminnm) = bit13 skip-NaN. ----
    FpPair { rd: u8, rn: u8, sz: bool, min: bool, nm: bool },
    // ---- SIMD FP 3-same pairwise: faddp/fmaxp/fminp/fmaxnmp/fminnmp
    // Vd.T, Vn.T, Vm.T -- pairwise-reduce each source into the first/second
    // halves of Vd (like integer ADDP but FP, with skip-NaN nm). Q=0 (.2s)
    // and Q=1 (.4s) forms; rm required.
    SimdFpPair3 { rd: u8, rn: u8, rm: u8, add: bool, min: bool, nm: bool, q: bool },
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
    // ---- SIMD single-precision FP two-source arithmetic: fadd/fsub/fmul/fdiv
    // (op 0..3) and fmax/fmin/fmaxnm/fminnm (op 4..7) on Vd.2s/.4s lanes. ----
    VecFpArith { rd: u8, rn: u8, rm: u8, op: u8, q: bool },
    // ---- SIMD FP compare->mask: fcmeq/fcmgt/fcmge Vd.T, Vn.T, Vm.T ----
    // result lane = all-ones if Vn op Vm, else 0. op 0=eq,1=gt,2=ge (lt/le are
    // gt/ge with Vn/Vm swapped). esize 4 (.2s/.4s) or 8 (.2d); q = bit30.
    VecFpCmp { rd: u8, rn: u8, rm: u8, esize: u8, op: u8, q: bool },
    // ---- SIMD FP compare-to-zero: fcmeq/fcmgt/fcmge/fcmlt/fcmle Vd.T, Vn.T, #0.0 ----
    // op 0=eq 1=gt 2=ge 3=lt 4=le. Per-lane result = all-ones if Vn op 0 else 0.
    VecFpCmpZero { rd: u8, rn: u8, op: u8, esize: u8, q: bool },
    // ---- SIMD integer compare-to-zero: cmeq/cmgt/cmge/cmlt/cmle Vd.T,Vn.T,#0 ----
    // (two-register misc). top byte in {0x0e,0x2e,0x4e,0x6e} (Q=bit30, U=bit29);
    // byte2 bits[15:10] in {0x22 cmgt, 0x26 cmeq, 0x2a cmlt}; esize = 1<<bits[23:22]
    // (8B/8H/4S/2D); U toggles gt->ge and eq->le. Each lane -> all-ones if the
    // signed/int compare against literal 0 holds, else 0. cond: 0=eq,1=gt,2=ge,
    // 3=lt,4=le. (encodings from aarch64-linux-gnu-as, objdump-verified)
    SimdCmpZero { rd: u8, rn: u8, esize: u8, q: bool, cond: u8 },
    // ---- SIMD widening shift-left (sign/zero extend): shll/usll Vd.Td, Vn.Ts ----
        WidenShl { rd: u8, rn: u8, dst_esize: u8, nlanes: u8, signed: bool, upper: bool },
        // ---- SIMD add/sub-long widening: saddl/uaddl/subl/usubl Vd.T, Vn.T, Vm.T ----
            SimdAddl { rd: u8, rn: u8, rm: u8, esrc: u8, sign: bool, sub: bool, upper: bool },
    // ---- SIMD add-adjacent-long pairwise accumulate: sadalp/uadalp Vd.Td, Vn.Ts ----
    SimdAdalp { rd: u8, rn: u8, src_esize: u8, n_pairs: u8, signed: bool, upper: bool, acc: bool },
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
    // ---- SIMD integer unary: neg/abs Vd.T, Vn.T (two-reg misc, opcode 0xb) ----
    // op 0=neg (Vd = -Vn, signed per-lane), 1=abs (Vd = |Vn| signed). esize in
    // {1,2,4,8} bytes (B/H/S/D), q selects 8B/16B, 4H/8H, 2S/4S, 1D/2D.
    SimdArithUnary { rd: u8, rn: u8, esize: u8, q: bool, op: u8 },
    // ---- SIMD 3-same pairwise-ADD across each source: ADDP Vd.T, Vn.T, Vm.T ----
    // (byte2&0xf8 == 0xb8, bit10 set). First half of lanes = pair sums of Vn,
    // second half = pair sums of Vm; esize=1<<size, q picks 8B/16B.
    SimdAddp { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD float widen/narrow: fcvtl/Vd.2D (f32->f64) & fcvtn/Vd.2S
    // (f64->f32), 2 lanes. fcvtl reads Vn low (upper=false) or high half
    // (upper=true); fcvtn writes Vd low (upper=false) or high half
    // (upper=true). Gates (insn & 0xffff_fc00): fcvtl == 0x0e617800 /
    // 0x4e617800, fcvtn == 0x0e616800 / 0x4e616800 (asm+objdump verified).
    // (Half-precision fcvtl Vd.4s,Vn.4h / fcvtn Vd.4h,Vn.4s are byte2 0x21
    // and stay Unsupported -- the JIT has no fp16 yet.)
    VecFcvtl { rd: u8, rn: u8, upper: bool },
    VecFcvtn { rd: u8, rn: u8, upper: bool },
    // ---- variable shift by register (LSLV/LSRV/ASRV/RORV) ----
    VarShiftVar { rd: u8, rn: u8, rm: u8, op: u8, sf: bool },
    // ---- SIMD FP unary: fneg/fabs/fsqrt Vd.T, Vn.T - op 0=neg 1=abs 2=sqrt ----
    SimdFpUnary { rd: u8, rn: u8, op: u8, esize: u8, q: bool },
    // ---- SIMD FP rounding: frint{n,m,p,z,a} Vd.T, Vn.T ----
    // mode 0=n(nearest-even) 1=m(toward -inf/floor) 2=p(+inf/ceil) 3=z(toward zero)
    // 4=a(nearest, ties away). esize = element size bytes (4=s, 8=d).
    SimdFrint { rd: u8, rn: u8, mode: u8, esize: u8, q: bool },
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
            op: u8,
            sz: bool,
        },
        // ---- scalar 3-source FP multiply-accumulate: fmadd/fmsub/fnmadd/fnmsub ----
        // Dd = Da +- (Dn×Dm), optionally negated. o1=bit21 (fn*), o2=bit15 (sub).
        Fma3 {
            rd: u8,
            rn: u8,
            rm: u8,
            ra: u8,
            sz: bool,
            sub: bool,  // o2: fmsub/fnmsub
            neg: bool,  // o1: fnmadd/fnmsub (negate the accumulate term)
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
           fbits: u8,   // 0 = plain convert; >0 = fixed-point scale (result = Fn*2^fbits)
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
                                                              rn: u8,
                                                              rm: u8, // second fp reg (0 for the #0.0 form)
                                                              against_zero: bool, // true = FCMP Dn, #0.0 (rm field ignored)
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
            // ---- integer conditional compare: ccmp/ccmn Rn, <Rm|#imm>, #nzcv, cond ----
            // (insn & 0x001f_f07f) — variable fields: imm5/Rm[20:16], cond[15:12],
            // rn[6:4], nzcv[3:0] — leaves residue in {0xfa/x7a/xba/x3a + 0x000/0x400
            // + 0x800}. If cond(guest NZCV) is true the flags become `Rn op <Rm|imm>`
            // (cmn=add, ccmp=sub); else they become `nzcv`. Flag-only, no writeback.
            CcMp {
                rn: u8,
                rm: u8,   // the other GPR (register form); unused for immediate
                imm: u8,  // 5-bit immediate (immediate form)
                nzcv: u8,
                cond: u8,
                cmn: bool,   // true = CCMN (add), false = CCMP (sub)
                sf: bool,    // true = 64-bit compare, false = 32-bit
                is_reg: bool,// true = register form, false = immediate
            },
            // ---- scalar FP->int to FP reg: fcvtzs/fcvtzu Dd,Dn / Sd,Sn (trunc toward zero) ----
            FcvtTzReg { rd: u8, rn: u8, dbl: bool, unsigned: bool },
        // ---- NEON: mov Vd.D[1], Vn.D[0] (dup low 64 into the high 64 lane) ----
                           InsD1D0 { rd: u8, rn: u8 }, // v16B: slot_hi(8B) = low-64-of-Vn
                           // ---- EXTR / ROR rotate: rm==rn in the EXTR base ----
                                                     Ror { rd: u8, rn: u8, rot: u32, sf: bool }, // ror rd,rn,#rot
                                                     // ---- EXTR (general): Xd = (Xn >> lsb) | (Xm << (bits-lsb)),
                                                     // the rotate/extract gcc emits for `(x>>a)|(x<<(b-a))` when
                                                     // rn != rm. (The Ror alias above is the rm==rn special case.) ----
                                                     Extr { rd: u8, rn: u8, rm: u8, lsb: u32, sf: bool },
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
    // ---- NEON vector int->FP: scvtf/ucvtf Vd.T, Vn.T (s32/u32->f32, s64->f64) ----
    // The reverse of FcvVec. Gate (insn&0xffe0_fc00) in
    // {0x0e20_d800(s32,scvtf,q=0) 0x2e20_d800(ucvtf) 0x4e20_d800(scvtf,q=1)
    //  0x6e20_d800(ucvtf,q=1) 0x4e60_d800(scvtf v.2d s64->f64)}. esize=8 iff
    // bit22 (the .2d forms); signed = bit29 clear; q = bit30. MUST decode
    // BEFORE the SimdMull gate, which otherwise swallows 0x4e21d800 as a
    // widening multiply (a silent miscompile: every vector int->float produced
    // garbage). (unsigned ucvtf v.2d 0x6e60d800 stays Ucvtf2d.)
    VecIntToFp { rd: u8, rn: u8, esize: u8, signed: bool, q: bool },
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
    // ---- SIMD widening multiply BY ELEMENT: smull/umull/smlal/umlal/smlsl/umlsl
    //      Vd.T, Vn.T, Vm.Tsb[idx] (index selects ONE element of the m operand
    //      broadcast to every lane) ---- Q=b30, U(uns)=b29, size=b[23:22]
    //      (1=16-bit src .4s res, 2=32-bit src .2d res), indexed operand reg
    //      Vm = b[19:16] (4 bits, v0-v15), index = L(b21):H(b20) for 16-bit src
    //      or just L(b21) for 32-bit src, op b[15:12] in {2=mlal acc,6=mlsl acc
    //      sub,0xa=mull}. bit13 (0x2000) SET is the discriminator vs FP fmla-el
    //      (op 1/5, bit13 clear) and non-widening int mla-el (op 0). Session 44:
    //      gcc's vector*const-scalar scaling emits these and they previously
    //      mis-decoded as FmlaEl/VecMovi (silent wrong values, fuzzer-caught).
    SimdMullEl { rd: u8, rn: u8, rm: u8, index: u8, res_esize: u8, unsigned: bool, q: bool, acc: bool, sub: bool },
    // ---- SIMD unsigned compare-higher: cmhi Vd.4S, Vn.4S, Vm.4S ----
    // Gate (insn & 0xffe0_fc00)==0x6ea0c000 (verified vs real 0x6ea4c1c1).
    // Lane => all-ones if Vn[i] > Vm[i] (unsigned), else 0.
    SimdCmhi { rd: u8, rn: u8, rm: u8, lanes: u8 },
    // ---- SIMD unsigned compare-higher 2D: cmhi Vd.2D, Vn.2D, Vm.2D ----
    SimdCmhiD { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD unsigned compare-higher-or-same: cmhs Vd.T, Vn.T, Vm.T ----
    // Byte2 0x3c (vs cmhi's 0x34, bit10 set) => per lane all-ones if Vn>=Vm
    // (unsigned); the "or same" variant of cmhi. cmovae (cc 0x43) not cmova.
    SimdCmhs { rd: u8, rn: u8, rm: u8, lanes: u8 },
    // ---- SIMD unsigned compare-higher-or-same 2D: cmhs Vd.2D, Vn.2D, Vm.2D ----
    SimdCmhsD { rd: u8, rn: u8, rm: u8 },
    // ---- SIMD signed compare-greater: cmgt Vd.T, Vn.T, Vm.T (per 32/64-bit lane) ----
    // Gate 0x4ea0_3400(.4s q1)/0x0ea0_3400(.2s q0)/0x4ee0_3400(.2d); lanes 4/2/2.
    SimdCmgt { rd: u8, rn: u8, rm: u8, lanes: u8, dword: bool },
    // ---- SIMD unzip even: uzp1 Vd.T, Vn.T, Vm.T ----
    SimdUz1 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD unzip odd: uzp2 Vd.T, Vn.T, Vm.T (gcc magic-division gather) ----
    SimdUz2 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD 32-bit lane multiply-accumulate/subtract: mla/mls Vd.4S/2S ----
    // mla: Vd = Vd + Vn*Vm ; mls: Vd = Vd - Vn*Vm (per 32-bit lane).
    SimdMla { rd: u8, rn: u8, rm: u8, lanes: u8, sub: bool },
    // ---- SIMD 32-bit lane MLA/MLS by element: mla/mls Vd.4S/2S, Vn, Vm.S[idx] ----
    // Vd[i] += Vn[i]*Vm.single-element (broadcast), per 32-bit lane. Integer
    // (wrap mod 2^32), the by-element sibling of SimdMla. Previously misdecoded
    // as VecMovi (a silent wrong-immediate) which corrupted vector math.
    SimdMlaEl { rd: u8, rn: u8, rm: u8, index: u8, lanes: u8, sub: bool },
    // ---- SIMD zip even: zip1 Vd.T, Vn.T, Vm.T ----
    SimdZip1 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD zip2: upper-half interleave (gcc unsigned magic-div widen) ----
    SimdZip2 { rd: u8, rn: u8, rm: u8, esize: u8, q: bool },
    // ---- SIMD element extract to GPR: umov/smov Rd, Vn.bits[idx] ----
    SimdMovEl { rd: u8, rn: u8, esize: u8, index: u8, signed: bool, is_x: bool },
    // ---- SIMD integer add/sub 2D (64-bit lanes): add Vd.2D, Vn.2D, Vm.2D ----
    SimdAddD { rd: u8, rn: u8, rm: u8, sub: bool },
    // ---- SIMD int 3-same pairwise max/min: smaxp/sminp/umaxp/uminp
    // Vd.T, Vn.T, Vm.T ---- pairwise reduce each source into halves of Vd
    // (same structure as ADDP but max/min, signed when bit29 clear). b2
    // 0xa4 (max) / 0xac (min); prefix 0x0e/0x2e/0x4e/0x6e. Distinct from
    // plain 3-same add via b2 bit4 (0x10): add uses 0x84/0x8c.
    SimdMaxMinP { rd: u8, rn: u8, rm: u8, min: bool, unsigned: bool, esize: u8, q: bool },
    // ---- SIMD int add/sub byte lanes 16B/8B: add Vd.16b, Vn.16b, Vm.16b ----
    SimdAddB { rd: u8, rn: u8, rm: u8, sub: bool, q: bool },
    // ---- SIMD int add/sub halfword 8H/4H (16-bit) lanes: add Vd.8h, Vn.8h, Vm.8h ----
    SimdAddH { rd: u8, rn: u8, rm: u8, sub: bool, q: bool },
    // ---- SIMD compare equal: cmeq Vd.T, Vn.T, Vm.T ----
    SimdCmEq { rd: u8, rn: u8, rm: u8, lanes: u8, esize: u8 },
    // ADDV Dd,Vn.T : horizontal sum of sign-extended vector elements -> bottom
    // element of Vd. size = element width in bytes (1=B,2=H,4=S), q = 128-bit.
    Addv { rd: u8, rn: u8, size: u8, q: bool },
    // ---- SIMD across-lanes min/max: SMINV/SMAXV/UMINV/UMAXV Sd/Hd/Bd, Vn.T ----
    // Horizontal (tree) reduce of min or max over ALL lanes to the bottom scalar
    // element of Vd (upper bits cleared). signed = bit29 CLR (smin/smax) /
    // SET (umin/umax); min = byte2 bit0 (b1/b0); size = element width.
    // Encodings (objdump): sminv s0,v1.4s=0x4eb1a820, smaxv=0x4eb0a820,
    // uminv=0x6eb1a820, umaxv=0x6eb0a820, .8h=0x4e71a820, .16b=0x4e31a820,
    // q=0 forms 0x0e*/0x2e* (8b/4h).
    SimdReduceMinMax { rd: u8, rn: u8, size: u8, signed: bool, is_min: bool, q: bool },
    // ---- SIMD scalar-64 pairwise add: addp Dd, Vn.2D (sum of the two 64-bit
    // lanes of Vn into the low 64 bits of Vd). gcc emits this for reductions.
    // Encoded 0x5ee0_b800/0x7ee0_b800 with bit20 SET (fcvtzs scalar uses the
    // same residue with bit20 CLEAR — bit20 is the discriminator).
    SimdPairAddD { rd: u8, rn: u8, unsigned: bool },
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
    // ---- SIMD bitwise insert: bit Vd.16B, Vn.16B, Vm.16B / bif Vd.16B.. ----
    // Gates (insn & 0xffe0_fc00): BIT 0x6ea01c00 (16B) / 0x2ea01c00 (8B),
    // BIF 0x6ee01c00 / 0x2ee01c00 (bit14 = the "insert-if-FALSE" op; distinct
    // from orr16 0x4ea01c00 by bit31). bit (bif=false): (Vn & Vm)|(Vd & ~Vm);
    // bif (bif=true): (Vn & ~Vm)|(Vd & Vm).
    SimdBit { rd: u8, rn: u8, rm: u8, bif: bool },
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
    // GPR->vector-element insert: `ins/mov Vd.T[index], Rn` (the reverse of
    // SimdLaneGp). Same gate residue 0x..00_0c00, but bit13 is CLEAR (extract
    // smov/umov has bit13 SET). copies esize bytes of GPR rn into Vd at byte
    // offset index*esize. This was previously mis-decoded as SimdLaneGp
    // (a register extract), silently corrupting a GPR and never writing Vd.
    InsGp { rd: u8, rn: u8, esize: u8, index: u8 },
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
        // ---- integer multiply-long: smull/umull/smaddl/umaddl/smsubl/umsubl ----
        // 32-bit Rn*Rm -> 64-bit product; Ra==31 => plain mul, else Ra+product
        // (maddl) or Ra-product (msubl). Gate top 0x9b & bits[22:21]==01
        // (disjoint from MulHigh bit22 and 64-bit madd bit21). bit23=signed(0),
        // bit15=sub (msubl/umsubl accumulate subtract).
        MulLong {
            rd: u8,
            rn: u8,
            rm: u8,
            ra: u8,     // 31 == no accumulate
            signed: bool,
            sub: bool,  // msubl/umsubl: Rd = Ra - Rn*Rm
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
    // ---- SME feature-off misc on glibc's `__libc_arm_za_disable` path ----
    // `str za[Wt, k], [Xn, #k, mul vl]` (0xe1206200 family) and
    // `smstart za`/`smstop za` (0xd50345/46/47xx, mask 0xd5034000). The guest
    // advertises NO SME (auxv HWCAP2 SME bit clear), so streaming mode is
    // never entered and the ZA tile is never live — these are no-ops. They
    // must still DECODE (glibc's block compiler follows the fall-through past
    // the data-dependent SME gate into them, so `Unsupported` would halt the
    // whole block even though they never execute).
    SmeNoop,
    // ---- SVE add-vector-length: addvl/addsvl Xd, Xn, #imm ----
    // gate (insn&0xffe0_f000)==0x0420_5000; imm6=sext(insn[10:5]) (bytes scaled
    // by the vector length). glibc's __libc_arm_za_disable loop advances its
    // ZA-dump pointer with `addsvl x16,x16,#16`. The guest models no SVE, so
    // VL=16 bytes (the architectural minimum) — safe because the loop is dead
    // at runtime (SME disabled), but the block still needs a translation.
    AddVectorLen { rd: u8, rn: u8, imm_bytes: i32 },
    // ---- SVE count vector elements: cntd Xd (0x04e0_e000 gate) ----
    // Rd = VL_d / 64 = 2 at VL=16 bytes. Appears in __libc_arm_za_disable's
    // never-taken ZA-invalid error path.
    SveCntd { rd: u8 },
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
    // The 3-bit field E maps to the true exponent e: E 0..3 -> e 1..4, E 4..7
    // -> e -3..0 (i.e. e = E+1 for E<4, e = E-7 for E>=4). The wrap threshold is
    // AFTER +4: `(E+1)` hits 5 for E=4 onward, so only values >= 5 wrap to
    // negative. BUGFIX: this was `>= 4`, which wrongly wrapped E=3 (e should be
    // +4 = 16.0..30.0) to -4 (0.0625..) and silently corrupted every FMOV-imm in
    // [16.0, 30.0] (sample rates / half-texel / corner constants). Verified
    // against the assembler for 0.125..30.0.
    if ex >= 5 {
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
        // Rotate right by R **within the esize-bit element** (ARM DecodeBitMasks
        // uses ROR — rotate right). A left-rotate here was a silent miscompile
        // for rotation-asymmetric masks (e.g. `mov x0,#0xffffffff80000001`);
        // only rotation-symmetric masks (0xCCCC, 0x5555, single-bit, all-ones)
        // produced the same value under both, which is why tests passed. Rotate
        // in the esize-bit domain so the high element bits don't leak: `v >> r`
        // (top (esize-r) bits move down) OR `v << (esize-r)` (the r low bits
        // wrap to the top within esize), masked back to esize bits.
        let r = r as usize;
        (ones >> r | ones << (esize as usize - r)) & esize_mask
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
    // ---- scalar 3-source FP multiply-accumulate fmadd/fmsub/fnmadd/fnmsub ----
    // Byte3 == 0x1f (0b00011111) uniquely identifies the scalar 3-source FP
    // family (single & double). o1=bit21 (fnmadd/fnmsub), o2=bit15 (sub);
    // sz=bit22 (1=double). Semantics (ARM): fmadd=ra+rn*rm, fmsub=ra-rn*rm,
    // fnmadd=-(ra+rn*rm), fnmsub=rn*rm-ra. MUST be near the top: a broad SIMD
    // vector-immediate gate would otherwise swallow 0x1f4.. as a bogus `movi`
    // and emit garbage (the compiler contracts every a*b+c into fmadd).
    if (insn & 0xff00_0000) == 0x1f00_0000 {
        return Inst::Fma3 {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            ra: ((insn >> 10) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            sz: (insn >> 22) & 1 == 1,
            sub: (insn >> 15) & 1 == 1,
            neg: (insn >> 21) & 1 == 1,
        };
    }

    // ---- SIMD horizontal add across lanes: ADDV Dd,Vn.T ----
    // MUST be near the top: an earlier SIMD dup/move gate swallows 0x..b820 and
    // emits per-lane identity copies instead of the sum. Gate clears Vn(bits9:5)
    // and Sd(bits4:0) via mask 0xfffffc00; residues 8b=0x0e31b800, 4h=0x0e71b800,
    // 16b=0x4e31b800, 8h=0x4e71b800, 4s=0x4eb1b800. Source Vn lives in bits[9:5]
    // (bits20:16 is a fixed constant), unlike other SIMD. q=bit30 (128-bit => 16b/
    // 8h/4s); bytes/lane: b23=1 -> 4, b22=1 -> 2, else 1.
    {
        let av = insn & 0xffff_fc00;
        if let Some((size, q)) = match av {
            0x0e31_b800 => Some((1u8, false)), // addv b0,v1.8b
            0x0e71_b800 => Some((2u8, false)), // addv h0,v1.4h
            0x4e31_b800 => Some((1u8, true)),  // addv b0,v1.16b
            0x4e71_b800 => Some((2u8, true)),  // addv h0,v1.8h
            0x4eb1_b800 => Some((4u8, true)),  // addv s0,v1.4s
            _ => None,
        } {
            return Inst::Addv {
                rd: (insn & 0x1f) as u8,
                rn: ((insn >> 5) & 0x1f) as u8,
                size,
                q,
            };
        }
    }

    // ---- SIMD across-lanes min/max: SMINV/SMAXV/UMINV/UMAXV Sd/Hd/Bd, Vn.T ----
    // Sibling of ADDV (same 0x..a820 lane / byte1 family, distinct byte2):
    // reduce min or max over all lanes to the bottom scalar. Encodings
    // (objdump): sminv s0,v1.4s=0x4eb1a820 smaxv=0x4eb0a820 uminv=0x6eb1a820
    // umaxv=0x6eb0a820, .8h=0x4e71a820/0x4e70a820, .16b=0x4e31a820/0x4e30a820,
    // q=0 (8b/4h) in the 0x0e/0x2e top. byte1 low = 0xa8; byte2 high nibble =
    // esize (0x3=8b, 0x7=16b, 0xb=32b), byte2 bit0 = min(1)/max(0), bit29 = U.
    // byte1 0xa8 distinguishes from ADDV's 0x..b800.
    {
        let m = insn & 0xffff_fc00;
        if (m & 0x0000_ff00) == 0x0000_a800
            && matches!((m >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e)
        {
            let b2 = ((m >> 16) & 0xff) as u8;
            let size = match b2 & 0xfe {
                0x30 => 1u8, // 8b
                0x70 => 2,   // 8h / 4h
                0xb0 => 4,   // 4s
                _ => 0,
            };
            if size != 0 {
                let signed = ((insn >> 29) & 1) == 0; // bit29 CLR = smin/smax
                return Inst::SimdReduceMinMax {
                    rd: (insn & 0x1f) as u8,
                    rn: ((insn >> 5) & 0x1f) as u8,
                    size,
                    signed,
                    is_min: b2 & 1 == 1,
                    q: ((insn >> 30) & 1) == 1,
                };
            }
        }
    }

    // ---- SME/SVE feature-off misc (glibc `__libc_arm_za_disable` path) ----
    // These precise masks come before the system-register (0xd5) gates so the
    // smstart/smstop system-link ops are not swallowed, and before any broad
    // top-byte match would claim the 0x04 (SVE) / 0xe1 (SME) space.
    // smstart/smstop za (0xd5034xxx): SME mode switch — no-op (SME off guest).
    if (insn & 0xffff_f000) == 0xd503_4000 {
        return Inst::SmeNoop;
    }
    // SVE addvl/addsvl Xd, Xn, #imm: gate (insn&0xffe0_f000)==0x0420_5000
    // (bits[23:21]==010, bits[15:12]==0b0101 — excludes CNT* and addpl).
    if (insn & 0xffe0_f000) == 0x0420_5000 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 16) & 0x1f) as u8;
        let imm6 = ((insn >> 5) & 0x3f) as i32;
        // sign-extend 6-bit imm, then scale by the model vector length (16 B).
        let imm6 = if imm6 & 0x20 != 0 { imm6 - 0x40 } else { imm6 };
        return Inst::AddVectorLen {
            rd,
            rn,
            imm_bytes: imm6 * 16,
        };
    }
    // SVE cntd Xd (0x04e0_e000): count 64-bit elements of a vector. VL=16B ->
    // 2. (cntb/cntw and the .d-only feature predicates return others; only
    // cntd appears on glibc's SME probe path, so the rest stay Unsupported.)
    if (insn & 0xffe0_f000) == 0x04e0_e000 {
        let rd = (insn & 0x1f) as u8;
        return Inst::SveCntd { rd };
    }
    // SME str za[Wt, k], [Xn, #k, mul vl]: zero the ZA tile to memory. ZA is
    // never live (SME off in this guest) so it's a no-op. 0xe1206200..0xf.
    if (insn & 0xffff_fff0) == 0xe120_6200 {
        return Inst::SmeNoop;
    }
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
    // ---- scalar FP-to-int into an FP register: fcvtzs/fcvtzu Dd, Dn / Sd, Sn ----,
        // Scalar D form (double -> 64-bit int): residues 0x5ee0_b800 (fcvtzs, signed)
        // / 0x7ee0_b800 (fcvtzu, unsigned). One 64-bit lane, q=false — matches the
        // FcvVec translate with esize=8 (cvttsd2si). Must precede both the FcvVec
        // vector gate and the SIMD widen gate (which wrongly matched 0x5ee1bbff).
        // CRITICAL: bit20 discriminates. fcvtzs scalar leaves it CLEAR (0x5ee1b820),
        // but the SIMD pairwise `addp Dd, Vn.2D` (0x5ef1b800) shares this residue
        // with bit20 SET — without the check, every gcc pairwise-add reduction was
        // silently decoded as a float->int convert (wrong result, not a trap).
        if ((insn & 0xffe0_fc00) == 0x5ee0_b800 || (insn & 0xffe0_fc00) == 0x7ee0_b800)
            && (insn & 0x100000) == 0
        {
            let rd = (insn & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let signed = (insn >> 29) & 1 == 0;
            return Inst::FcvVec { rd, rn, signed, esize: 8, q: false };
        }
        // ---- SIMD scalar-64 pairwise add: addp Dd, Vn.2D (bit20 SET) ----
        // Sums the two 64-bit lanes of Vn into the low 64 bits of Vd (signed wrap,
        // the same as a 64-bit add). See the fcvtzs scalar gate for the bit20
        // discriminator — this MUST come immediately after it so shared residues
        // route correctly.
        if ((insn & 0xffe0_fc00) == 0x5ee0_b800 || (insn & 0xffe0_fc00) == 0x7ee0_b800)
            && (insn & 0x100000) != 0
        {
            let rd = (insn & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let unsigned = (insn >> 29) & 1 == 1;
            return Inst::SimdPairAddD { rd, rn, unsigned };
        }
    // ---- SIMD float-to-int (vector): fcvtzu/fcvtzs Vd.T, Vn.T (FPI(FPc))----
    // bit16 MUST be SET: the mask 0xffe0_fc00 clears bits[20:16], and the
    // integer two-reg-misc NEG/ABS (opcode 0xb at bits[16:12], bit16 CLEAR)
    // share the residue here. Requiring bit16=1 keeps fcvtzs/fcvtzu (all six
    // sizes have bit16 set, objdump-verified) while excluding neg/abs.
    if matches!(insn & 0xffe0_fc00, 0x0ea0_b800 | 0x2ea0_b800 | 0x4ea0_b800 | 0x4ee0_b800 | 0x6ea0_b800 | 0x6ee0_b800)
        && (insn & 0x0001_0000) != 0
    {
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
    // ---- SIMD integer unary neg/abs: NEG Vd.T,Vn.T = -Vn, ABS = |Vn| ----
    // Two-register-misc, opcode bits[16:12]==0xb (b safely separates from the
    // fcvtzs/fcvtzu family above — bit16 CLEAR here vs the fcvtzs bit16 SET —
    // and from cmeq/cmgt/cmlt#0 [0x8/0x9/0xa] and sqabs/sqneg [0x7]). neg=U
    // (bit29 set → 0x2e/0x6e), abs=bit29 clear (0x0e/0x4e). size=(insn>>22)&3
    // maps S-arrangement {0→B(1),1→S(4),2→H(2),3→D(8)}. Q=bit30.
    if matches!((insn >> 24) & 0x3f, 0x0e | 0x2e | 0x4e | 0x6e)
        && ((insn >> 12) & 0x1f) == 0xb
        && (insn & 0x400) == 0 // bit10=0: two-reg neg/abs; 3-same ADDP sets bit10
    {
        let sz = (insn >> 22) & 3;
        let esize = match sz {
            0 => 1, // .b
            1 => 2, // .h
            2 => 4, // .s
            _ => 8, // .d
        };
        return Inst::SimdArithUnary {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize,
            q: (insn >> 30) & 1 == 1,
            op: if (insn >> 29) & 1 == 1 { 0 } else { 1 }, // neg : abs
        };
    }
    // ---- SIMD 3-same pairwise-add across halves: ADDP Vd.T, Vn.T, Vm.T ----
    // byte2&0xf8 == 0xb8 AND bit10 set (neg/abs two-reg has bit10 clear).
    // size=bits[23:22]; q=bit30. First half lanes = Vn pair sums, second = Vm.
    if matches!((insn >> 24) & 0x3f, 0x0e | 0x2e | 0x4e | 0x6e)
        && ((insn >> 8) & 0xff & 0xf8) == 0xb8
        && (insn & 0x400) != 0
    {
        let size = (insn >> 22) & 3;
        let esize = 1u8 << size; // 1,2,4,8 bytes
        return Inst::SimdAddp {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            rm: ((insn >> 16) & 0x1f) as u8,
            esize,
            q: (insn >> 30) & 1 == 1,
        };
    }
    // ---- SIMD float widen/narrow: fcvtl Vd.2D,Vn.2S (f32->f64) & fcvtn
    // Vd.2S,Vn.2D (f64->f32). Asm+objdump verified: fcvtl 0x0e617820 /
    // fcvtl2 0x4e617820, fcvtn 0x0e6168a4 / fcvtn2 0x4e6168a4. upper = bit30
    // (Q): fcvtl reads Vn upper half when set (fcvtl2); fcvtn writes Vd upper
    // half when set (fcvtn2). The byte2-0x21 (half-precision) forms are NOT
    // matched here and stay Unsupported. Placed before VecIntToFp/SimdMull
    // (which would otherwise swallow these as int->fp/widen-mul).
        // ---- SIMD integer compare-to-zero: cmeq/cmgt/cmge/cmlt/cmle Vd.T,Vn.T,#0 ----
    // top byte {0x0e,0x2e,0x4e,0x6e} (U=bit29 toggles gt->ge / eq->le, Q=bit30),
    // byte2 bits[15:10] in {0x22 cmgt, 0x26 cmeq, 0x2a cmlt}, bit26 set.
    // esize = 1 << bits[23:22] (8B/8H/4S/2D). Per-lane all-ones-or-0 mask.
    if matches!((insn >> 24) & 0x3f, 0x0e | 0x2e | 0x4e | 0x6e)
        && (insn & 0x0400_0000) != 0
        && matches!((insn >> 10) & 0x3f, 0x22 | 0x26 | 0x2a)
        && (insn & 0x0001_0000) == 0 // bit16 set = FP frint/tbl family, NOT int cmeq/cmlt
    {
        let cond = match (insn >> 12) & 0xf {
            8 => if (insn & 0x2000_0000) != 0 { 2 } else { 1 }, // cmge/cmgt
            9 => if (insn & 0x2000_0000) != 0 { 4 } else { 0 }, // cmle/cmeq
            _ => 3, // cmlt
        };
        let size = (insn >> 22) & 3;
        return Inst::SimdCmpZero {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize: (1u8 << size),
            q: (insn >> 30) & 1 == 1,
            cond,
        };
    }
if matches!(insn & 0xffff_fc00, 0x0e61_7800 | 0x4e61_7800) {
        return Inst::VecFcvtl { rd: (insn & 0x1f) as u8, rn: ((insn >> 5) & 0x1f) as u8, upper: (insn >> 30) & 1 == 1 };
    }
    if matches!(insn & 0xffff_fc00, 0x0e61_6800 | 0x4e61_6800) {
        return Inst::VecFcvtn { rd: (insn & 0x1f) as u8, rn: ((insn >> 5) & 0x1f) as u8, upper: (insn >> 30) & 1 == 1 };
    }
    // ---- SIMD int->FP (vector): scvtf/ucvtf Vd.T, Vn.T (s32/u32->f32, s64->f64) ----
    // Reverse of FcvVec. Encodings assembled & objdump-verified:
    //   scvtf v0.4s=0x4e21d800  ucvtf v0.4s=0x6e21d800  scvtf v0.2s=0x0e21d800
    //   ucvtf v0.2s=0x2e21d800  scvtf v0.2d=0x4e61d800.
    // (ucvtf v.2d 0x6e61d800 is Ucvtf2d above.) MUST be before SimdMull, which
    // otherwise misdecodes every one of these as a widening multiply.
    if matches!(insn & 0xffe0_fc00, 0x0e20_d800 | 0x2e20_d800 | 0x4e20_d800 | 0x6e20_d800 | 0x4e60_d800) {
        let esize = if (insn >> 22) & 1 == 1 { 8u8 } else { 4u8 };
        let signed = (insn >> 29) & 1 == 0;
        let q = (insn >> 30) & 1 == 1;
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        return Inst::VecIntToFp { rd, rn, esize, signed, q };
    }
    // ---- SIMD single-precision FP two-source arithmetic: fadd/fsub/fmul/fdiv
    // (op 0..3) and fmax/fmin/fmaxnm/fminnm (op 4..7) on Vd.2s/.4s lanes. ----
    // Encodings asm+objdump verified (see /tmp/fpv.s): fadd 2s/4s =
    // 0x0e21d400/0x4e21d400, fsub 0x0ea1d400/0x4ea1d400, fmul 0x2e21dc00/
    // 0x6e21dc00, fdiv 0x2e21fc00/0x6e21fc00, fmax 0x4e21f400, fmin
    // 0x4ea1f400, fmaxnm 0x4e21c400, fminnm 0x4ea1c400. MUST be before
    // SimdMull/Simd4s/SimdAddB, which silently misdecode these as a widening
    // multiply / integer add / byte add (vector single-FP graphics math was
    // wrong, not an Unsupported stop, until this gate was added). The .2d
    // double-lane forms (0x..60 d400/.. etc.) stay with Simd2dFp.
    {
        const VFP: &[(u32, u8)] = &[
            (0x0e20_d400, 0), (0x4e20_d400, 0), // fadd 2s/4s
            (0x0ea0_d400, 1), (0x4ea0_d400, 1), // fsub
            (0x2e20_dc00, 2), (0x6e20_dc00, 2), // fmul
            (0x2e20_fc00, 3), (0x6e20_fc00, 3), // fdiv
            (0x0e20_f400, 4), (0x4e20_f400, 4), // fmax
            (0x0ea0_f400, 5), (0x4ea0_f400, 5), // fmin
            (0x0e20_c400, 6), (0x4e20_c400, 6), // fmaxnm
            (0x0ea0_c400, 7), (0x4ea0_c400, 7), // fminnm
        ];
        let m = insn & 0xffe0_fc00;
        if let Some(&(_, op)) = VFP.iter().find(|&&(r, _)| r == m) {
            let rd = (insn & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rm = ((insn >> 16) & 0x1f) as u8;
            let q = (insn >> 30) & 1 == 1;
            return Inst::VecFpArith { rd, rn, rm, op, q };
        }
    }
    // ---- SIMD FP compare->mask: fcmeq/fcmgt/fcmge Vd.T, Vn.T, Vm.T ----
    // byte2 (bits15:8) == 0xe4, top byte in the {2e,4e,6e,.2d 6e..} family
    // (encodings asm+objdump verified: fcmgt 4s=0x6ea2e420 fcmeq 4s=0x4e22e420
    // fcmge 4s=0x6e22e420, .2d forms bit22 set). Per-lane result = all-ones if
    // the comparison holds else 0. op: bit29=0 => eq, else bit23=1 => gt,
    // bit23=0 => ge. MUST be before the SimdVShift gate, which otherwise
    // misdecodes fcmgt as a variable shift (gcc float-vs-constant count loops
    // returned garbage). NaN handling: comiss/comisd sets CF=ZF on NaN, so a
    // NaN lane reads as "less than" — eq/ge/gt all produce 0 (matches ARM's
    // "NaN compares false") for the ordered forms.
    if ((insn & 0xffe0_fc00) >> 8) & 0xff == 0xe4
        && matches!((insn >> 24) & 0x0f, 0x0e | 0x2e | 0x4e | 0x6e)
    {
        let esize = if (insn >> 22) & 1 == 1 { 8u8 } else { 4u8 };
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let q = (insn >> 30) & 1 == 1;
        let op = if (insn >> 29) & 1 == 0 {
            0u8 // fcmeq
        } else if (insn >> 23) & 1 == 1 {
            1u8 // fcmgt
        } else {
            2u8 // fcmge
        };
        return Inst::VecFpCmp { rd, rn, rm, esize, op, q };
    }
    // ---- SIMD FP compare-to-zero: fcmeq/fcmgt/fcmge/fcmlt/fcmle Vd.T, Vn.T, #0.0 ----
    // byte2 (bits15:8) 0xc8=gt/ge, 0xd8=eq/le, 0xe8=lt; bit29 flips gt->ge (0xc8)
    // and eq->le (0xd8). es=bit22 (0=s 4B, 1=d 8B), q=bit30. Encodings verified vs
    // the aarch64 assembler across .2s/.4s/.2d. Disjoint from VecFpCmp (2-reg,
    // byte2 0xe4) and from the coarse smull gate (byte2 must be 0x80/0xc0).
    {
        const CMPZ: &[(u32, u8, u8)] = &[
            // (residue & 0xffff_fc00, op, esize_bytes)
            (0x0ea0_d800, 0, 4), (0x4ea0_d800, 0, 4), (0x4ee0_d800, 0, 8), // fcmeq
            (0x0ea0_c800, 1, 4), (0x4ea0_c800, 1, 4), (0x4ee0_c800, 1, 8), // fcmgt
            (0x2ea0_c800, 2, 4), (0x6ea0_c800, 2, 4), (0x6ee0_c800, 2, 8), // fcmge
            (0x0ea0_e800, 3, 4), (0x4ea0_e800, 3, 4), (0x4ee0_e800, 3, 8), // fcmlt
            (0x2ea0_d800, 4, 4), (0x6ea0_d800, 4, 4), (0x6ee0_d800, 4, 8), // fcmle
        ];
        let res = insn & 0xffff_fc00;
        if let Some(&(_, op, esize)) = CMPZ.iter().find(|&&(r, _, _)| r == res) {
            return Inst::VecFpCmpZero {
                rd: (insn & 0x1f) as u8,
                rn: ((insn >> 5) & 0x1f) as u8,
                op,
                esize,
                q: (insn >> 30) & 1 == 1,
            };
        }
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
        // Extended-register form (bit21=1): a DIFFERENT encoding layout — Rm =
        // bits[20:16], option = bits[15:13], shift = bits[12:10] (0..3). The
        // shifted-register form (bit21=0) instead packs shift=bits[23:22],
        // sh_amt=bits[15:10]. They MUST be decoded separately (feeding the
        // extended layout through the shifted parser produced a bogus
        // `lsl #sh_amt`, e.g. sxtw#3 -> shift 51 — real register corruption).
        if n == 1 {
            return Inst::AddSubExt {
                rd: b(insn, 0, 4) as u8,
                rn: b(insn, 5, 9) as u8,
                rm: b(insn, 16, 20) as u8,
                sub,
                s,
                sf,
                opt: b(insn, 13, 15) as u8,
                shift: b(insn, 10, 12) as u8,
            };
        }
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
            // bit21 (==0 here, since the ==1 case returned AddSubExt above)
            // marks the extended-register form. For the shifted form, regs 31
            // are XZR (qemu-verified: `neg`=0xcb0603e6 reads rn31 as XZR).
            sp_operand: false,
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
        // op = (o1<<1 | o2), o1=bit30, o2=bit10. Verified against the assembler
        // for all four forms at W/X: 0x1a80/0x9a80 (bit30=0) -> csel(o2=0)/
        // csinc(o2=1); 0x5a80/0xda80 (bit30=1) -> csinv(o2=0)/csneg(o2=1).
        // (Old code read only bits[11:10], collapsing csinv->CSEL identity and
        // csneg->CSINC +1 — a silent wrong result for NOT and NEG.)
        let op = (((insn >> 30) & 1) << 1 | (insn >> 10) & 1) as u8;
        return Inst::CSel { rd, rn, rm, cond, op, sf };
    }

    // ---- integer multiply-high: umulh/smulh Xd, Xn, Xm ----
    // top byte 0x9b AND bit22 set (0x00400000) is the high-multiply marker that
    // separates it from the madd/msub/udiv/sdiv MulDiv family (bit22=0).
    // signed = bit23 clear (umulh=0x9bC7.. has bit23=1, smulh=0x9b47.. has 0).
    // Verified vs objdump: umulh x2,x3,x6 = 0x9bc67c62, smulh = 0x9b467c62.
    // MUST come after the multiply-long check below (both are top 0x9b; the
    // long-multiply has bits[22:21]==01, MulHigh has bit22 set ==10).
    //
    // ---- integer multiply-long: smull/umull/smaddl/umaddl/smsubl/umsubl ----
    // 32x32 -> 64 product, optional accumulate (maddl/msubl). Gate top byte
    // 0x9b AND bits[22:21]==0b01 (the long-multiply marker). Disjoint from
    // MulHigh (bit22 set -> 0b10) and from 64-bit madd/msub (bit21 clear ->
    // 0b00, which the MulDiv 0x1b000000 gate handles). bit23 = unsigned (1)
    // vs signed (0); bit15 = sub (msubl/umsubl subtract the product).
    // Verified vs objdump: umull=0x9ba77c61, smull=0x9b267ca4,
    // umaddl=0x9ba41462, smaddl=0x9b2824e6, umsubl=0x9bacb56a,
    // smsubl=0x9b30c5ee.
    if (insn >> 24) & 0xff == 0x9b && (insn >> 21) & 0x3 == 0b01 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let ra = ((insn >> 10) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        return Inst::MulLong {
            rd,
            rn,
            rm,
            ra,
            signed: (insn & 0x0080_0000) == 0, // bit23: 1=unsigned, 0=signed
            sub: (insn & 0x0000_8000) != 0,     // bit15: 1=msubl/umsubl
        };
    }
    if (insn >> 24) & 0xff == 0x9b && (insn & 0x0040_0000) != 0 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        return Inst::MulHigh {
            rd,
            rn,
            rm,
            signed: (insn & 0x0080_0000) == 0, // bit23: 0 => smulh, 1 => umulh
        };
    }

    // ---- integer multiply/divide register (madd/msub/udiv/sdiv) ----
    // top 0x1a/0x9a (DIV) or 0x1b/0x9b (MUL). Masked base: 0x1ac00000 (div),
    // 0x1b000000 (mul). sf = bit31. signed: div bit17 (1=sdiv), mul bit15 (1=msub).
    if insn & 0x7fe0_0000 == 0x1ac0_0000 || insn & 0x7fe0_0000 == 0x1b00_0000 {
        let sf = (insn >> 31) & 1 == 1;
        let div = insn & 0x7fe0_0000 == 0x1ac0_0000;
        let signed = if div {
            b(insn, 10, 10) == 1 // SDIV vs UDIV: bit10=1 => SDIV (assembler-verified: sdiv w1,w3,w5=0x1ac50c61 bit10=1, udiv=0x1ac50861 bit10=0)
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
    // NOTE: conditional-compare ccmp/ccmn shares the 0xFA/0x7A/0xBA/0x3A top
    // bytes with the logical set-flags family but is distinguished by its full
    // residue — checked FIRST so it isn't swallowed as an ANDS/BICS.
    {
        let varmask = 0x001f_f3ef; // Rn[9:5] | imm5/Rm[20:16] | cond[15:12] | nzcv[3:0]
        let res = insn & !varmask;
        if matches!(
            res,
            0xfa40_0800 | 0xfa40_0000 | 0x7a40_0800 | 0x7a40_0000
                | 0xba40_0800 | 0xba40_0000 | 0x3a40_0800 | 0x3a40_0000
        ) {
            let cond = b(insn, 12, 15) as u8;
            let nzcv = b(insn, 0, 3) as u8;
            let rn = b(insn, 5, 9) as u8;
            let src = b(insn, 16, 20) as u8; // rm (reg) or imm5 (imm)
            let sf = (insn >> 31) & 1 == 1;
            let cmn = (insn >> 30) & 1 == 0; // ccmp=bit30 (subtract), ccmn=add
            let is_reg = (insn & 0x800) == 0; // bit11=1 => immediate form
            if !is_reg {
                return Inst::CcMp { rn, rm: 0, imm: src, nzcv, cond, cmn, sf, is_reg };
            }
            return Inst::CcMp { rn, rm: src, imm: 0, nzcv, cond, cmn, sf, is_reg };
        }
    }
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
        // PRFM (prefetch, unsigned-immediate) = `size=0b11` (byte) with
        // `opc=0b10` (bit23 set, bit22 clear) — the "size-8 X dest load" shape
        // that naive bit23-keyed sign-extend decode reads as `ldrx/lx size-8
        // sign-extend`. PRFM is a pure hint (no architected data side effect
        // beyond a cache hint a single-threaded direct-mapped JIT ignores), so
        // treat it as a Hint/no-op. Real sign-extend loads (ldrsw/ldrsh/ldrsb)
        // always have size 4/2/1 (never 8), so `size==8 && bit23` is uniquely
        // PRFM. (The register-offset `ldrsb x, [..]` forms never reach here.)
        if ((insn >> 30) & 0x3) == 0b11 && (insn & 0x80_0000) != 0 {
            return Inst::Hint;
        }
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

    // ---- FP/SIMD SCALAR immediate load/store: UNSCALED (LDUR/STUR) + PRE/POST ----
    // -index writeback (LDR/STR <s>t, signed imm9). Same scalar B/H/S/D widths
    // as FpLdStImm below, but with a SIGNED imm9 (bits[20:12]) instead of a
    // size-scaled one, and (for the writeback forms) a post/pre base update.
    // Fixed bits distinguish this from the neighbours: bit26=1 (vector file, vs
    // the GPR LdStrImmWb whose 0xfc/0xbc top bytes overlap); bit25=0 (immediate,
    // not the register-offset FpLdStrReg); bit24=0 (unscaled/indexed, not the
    // size-scaled 0x3d FpLdStImm below); bit23=0 (scalar, not the 128-bit Q
    // VecLdSt* gates which set bit23); bits[29:27]=111 (the fixed 111 opcode
    // field, verified vs stur d0=0xfc008020 / ldur s0=0xbc5fc020 etc).
    if (insn & 0x0400_0000) != 0 // bit26 = V (vector register file)
        && (insn & 0x0200_0000) == 0 // bit25 = 0: immediate offset
        && (insn & 0x0020_0000) == 0 // bit21 = 0: NOT register-offset (FpLdStrReg sets bit21)
        && (insn & 0x0100_0000) == 0 // bit24 = 0: unscaled/indexed (not scaled)
        && (insn & 0x0080_0000) == 0 // bit23 = 0: scalar (not 128-bit Q)
        && (insn & 0x3800_0000) == 0x3800_0000 // bits[29:27] = 111
    {
        let size = match (insn >> 30) & 0x3 {
            0 => 1, // b
            1 => 2, // h
            2 => 4, // s
            _ => 8, // d
        };
        let ld = (insn >> 22) & 1 == 1;
        let imm9 = (((insn >> 12) & 0x1ff) as i32) << 23 >> 23; // sign-ext 9 bits
        let rn = b(insn, 5, 9) as u8;
        let vt = (insn & 0x1f) as u8;
        return match (insn >> 10) & 0x3 {
            0 => Inst::FpLdStImmUnscaled {
                vt,
                rn,
                imm9,
                size,
                ld,
            },
            // bits[11:10] = 01 post-index, 11 pre-index (mirror the GPR
            // LdStrImmWb gates 0x3800_0400 post / 0x3800_0c00 pre).
            1 => Inst::FpLdStImmWb {
                vt,
                rn,
                imm9,
                size,
                ld,
                pre: false,
            },
            3 => Inst::FpLdStImmWb {
                vt,
                rn,
                imm9,
                size,
                ld,
                pre: true,
            },
            _ => Inst::Unsupported((insn >> 10) & 0x3), // 2 = unprivileged LDTR/STTR
        };
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
            upper: (insn >> 30) & 1 == 1, // Q=1 => sxtl2/uxtl2 (upper half)
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
            upper: (insn >> 30) & 1 == 1, // Q=1 => saddw2/uaddw2 (upper half of Vm)
            sub: (insn & 0x2000) != 0,     // bit13: ssubw/usubw (add-wide is bit13 clear)
        };
    }

    // ---- SIMD lane ops: element extract to GPR vs GPR->element insert ----
    // Gate (insn & 0xffe0_0c00) in {0x0e000c00 (Wd dest), 0x4e000c00 (Xd dest)}.
    // This is placed BEFORE the broad vector-logical (AND/ORR/BIC) and
    // saturating-add (sqadd/sqsub) gates: INS/SMOV/UMOV share those bit fields,
    // so a loose 0x1c00/0x0c-0x2c byte gate would swallow them (objdump verified
    // `mov v0.s[0],w1` = 0x4e041c20 wrongly decoded as AND, `smov x2` = 0x4e042c02
    // as SQSUB, `smov x6,v0.h[2]` = 0x4e0a2c06 as SQSUB). The opcode field
    // bits[13:12] discriminates the class (verified against objdump ground truth):
    //   bit13=1 => vector->GPR extract (umov=11/smov=10: sign = bit12 clear)
    //   bit13=0, bit12=1 => GPR->vector insert (ins/mov Vd.T[idx], Rn)
    //   bit13=0, bit12=0 => dup from GPR (falls through to the SimdDupGp gate)
    // esize from imm5 trailing-zeros: p=ctz(imm5)+1 => esize=1<<(p-1); index = imm5>>p.
    // Verified disjoint from every legitimate logical/sat/uaddl encoding
    // (and/orr/eor/bic/sqadd/sqsub/uaddl all give 0x..2x0c00 residue, not
    // {0x0e000c00,0x4e000c00}) and from dup (handled below by bit12==0 fall-through).
    {
        let gt = insn & 0xffe0_0c00;
        if gt == 0x0e00_0c00 || gt == 0x4e00_0c00 {
            let imm5 = (insn >> 16) & 0x1f;
            if imm5 != 0 {
                let p = 1u32 + imm5.trailing_zeros();
                let esize = (1u8 << (p - 1)) as u8; // 1,2,4,8
                let index = (imm5 >> p) as u8;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let rd = (insn & 0x1f) as u8;
                if (insn & 0x2000) != 0 {
                    // ---- vector->GPR extract (umov/smov/mov, any element size) ----
                    let sign = (insn & 0x1000) == 0; // SMOV when bit12 clear
                    let wide = gt == 0x4e00_0c00;
                    return Inst::SimdLaneGp { rd, rn, esize, index, sign, wide };
                } else if (insn & 0x1000) != 0 {
                    // ---- GPR->vector insert (ins/mov Vd.T[idx], Rn) ----
                    return Inst::InsGp { rd, rn, esize, index };
                }
                // else bit13==0 && bit12==0: dup-from-GPR; handled by SimdDupGp below.
            }
        }
    }

    // ---- SIMD vector bitwise AND/ORR/BIC (Vd.T = Vn.T op Vm.T) ----
    // Gate: prefix byte {0x0e,0x4e} (bit29=0 → and/orr/bic, NOT bit/bif/bsl which
    // are 0x6e-prefixed, and NOT eor which is 0x2e). `&0x0000_1c00==0x1c00`.
    // ALSO require bit15 clear: the AND/ORR/BIC byte1 is 0x1c/0x1d (bit15=0),
    // while the 3-same integer MUL (byte1 0x8c..0x9f: `.4s`=0x9d, `.2s`=0x9e)
    // shares bits11:10 but sets bit15 — the old gate swallowed `mul` as a
    // bitwise op and miscompiled every NEON element-wise multiply (gcc's
    // magic-division dividend `mul v26.4s,v26,v28(97)` became AND/ORR garbage).
    // op = bit23(orr) | bit22(bic) | else and. Verified vs 16 asm forms.
    if matches!((insn >> 24) & 0x3f, 0x0e | 0x4e) && (insn & 0x0000_1c00) == 0x1c00 && (insn & 0x8000) == 0 {
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
    // Gate also requires bits[15:13]==0 (byte1 low nibble 0xc): the select family
    // always encodes byte1 low 0x1c, whereas the FP `.2d` two-source ops in the
    // same 0x2e/0x6e column (fdiv v0.2d = 0x6e61fc00, fmul v0.2d = 0x6e61dc00,
    // byte1 low 0xdc/0xfc = bits[15:13] SET) were being swallowed as a bitwise
    // select, silently corrupting double arithmetic into pandn/pand logic. They
    // now fall through to Simd2dFp. Also requires bit23 CLEAR: BIT/BIF set it
    // (0x80_0000) and are the bitwise-insert ops (handled by SimdBit) -- only
    // BSL (bit23=0) selects here. Without the guard, BIF (which shares bit22
    // with BSL) was silently decoded as BSL with the opposite mask semantics.
    if matches!((insn >> 24) & 0x3f, 0x2e | 0x6e) && (insn & 0x0000_1c00) == 0x1c00 && (insn & 0x0040_0000) != 0 && (insn & 0x0080_0000) == 0 && (insn & 0x0000_e000) == 0 {
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

    // ---- SIMD saturating shift-LEFT immediate: sqshl/uqshl/sqshlu Vd.T, Vn.T, #imm ----
    // Same prefix + immh mechanics as plain shl below, but bits[14:12] are the
    // saturating marker (0b111 = sqshl/uqshl, 0b110 = sqshlu) rather than shl's
    // 0b101. shift computed identically.
    if matches!((insn >> 24) & 0xff, 0x0f | 0x2f | 0x4f | 0x6f)
        && matches!(insn & 0x0000_7000, 0x0000_6000 | 0x0000_7000) && (insn & 0x8000) == 0
    {
        let immh4: u32 = (insn >> 19) & 0xf;
        if immh4 != 0 {
            let fls = 32 - immh4.leading_zeros();
            let esize: u8 = 1u8 << (fls - 1);
            let esize_bits = 8 * esize as u32;
            let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
            let shift = full.saturating_sub(esize_bits) as u8;
            let ures = (insn & 0x1000) == 0; // sqshlu: bits[14:12]==110 (bit12 clear, b2 0x64) vs 111 (0x74)
            let usr = (insn >> 29) & 1 == 1; // 0x2f/0x6f => unsigned src
            let sat = if ures { 2 } else if usr { 1 } else { 0 };
            return Inst::SimdSatShl { rd: (insn & 0x1f) as u8, rn: ((insn >> 5) & 0x1f) as u8, esize, shift, sat };
        }
    }

    // ---- SIMD shift-left immediate: shl Vd.T, Vn.T, #imm ----
    // Gate (insn & (0x0f00_0000 | 0x0000_7000)): prefix 0x0f/0x2f/0x4f/0x6f SIMD
    // register form and the SHIFTL marker (bits[14:12]==0b101 -> 0x5000).
    // immh = bits[22:19] (the FULL 4 bits, bit22 drops nothing), immb = bits[18:16].
    // esize from the MSB of immh4 (esize_bytes = 1<<(fls-1) for .4S/.2D; immh4==0
    // => 8B). shift = UInt(immh4:immb) - esize_bits (the field encodes esize+shift;
    // e.g. shl .4S #25: immh=7,immb=1 -> (7<<3)|1 = 57, esize_bits=32, shift=25).
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x2f | 0x4f | 0x6f) && (insn & 0x0000_7000) == 0x0000_5000 && (insn & 0x8000) == 0 {
        let immh4: u32 = (insn >> 19) & 0xf;
        if immh4 == 0 {
            // 8B lanes, shift = immb
            return Inst::SimdShl {
                rd: (insn & 0x1f) as u8,
                rn: ((insn >> 5) & 0x1f) as u8,
                esize: 8,
                shift: ((insn >> 16) & 0x7) as u8,
            };
        }
        let fls = 32 - immh4.leading_zeros(); // highest set bit, 1-indexed
        let esize: u8 = 1u8 << (fls - 1);      // 1/2/4/8-byte lanes
        let esize_bits = 8 * esize as u32;
        let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
        let shift = full.saturating_sub(esize_bits) as u8;
        return Inst::SimdShl {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize,
            shift,
        };
    }

    // ---- SIMD saturating narrowing shift: sqshrn/uqshrn/sqshrun Vd.T, Vn.T, #imm ----
    // Same 0x0f/0x2f/0x4f/0x6f prefix + b[14:12]==0b001 accumulate marker as the
    // usra/ssra gate below, but bit15 (0x8000) is SET (ssra/usra clear it). The
    // source elements are twice the destination width; we saturate then narrow.
    //   dst_signed = bit29 clear (sqshrn); for unsigned dst (bit29 set):
    //     src_signed = byte2 bit4 clear (sqshrun: 0x84)  vs uqshrn (0x94).
    if matches!((insn >> 24) & 0xff, 0x0f | 0x2f | 0x4f | 0x6f)
        && (insn & 0x0080_0000) == 0
        && (insn & 0x8000) != 0 // narrowing-shift marker (shrn/rshrn/sqshrn family)
        && (insn & 0x80000) != 0 // valid narrowing-shift immh has bit19 set (immh4 in 1,3,7); by-element mul uses 0xc
        && ((insn & 0x1000) != 0 || (insn & 0x2000_0000) != 0) // sat: sqshrn/uqshrn (bit12) or sqshrun (bit29=unsigned)
        && (insn & 0x0078_0000) != 0 // immh != 0, exclude movi/mvni family
    {
        let immh4: u32 = (insn >> 19) & 0xf;
        let blen = 32 - immh4.leading_zeros(); // highest set bit position (1-indexed, immh4 nonzero)
        let src_esize: u8 = 1u8 << blen;      // source element width (2x dst)
        let esize_bits = 8 * src_esize as u32;
        let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
        let shift: u8 = if full < esize_bits { (esize_bits - full) as u8 } else { 0 };
        let dst_signed = (insn >> 29) & 1 == 0;
        let src_signed = if dst_signed {
            true // sqshrn: signed src
        } else {
            ((insn >> 8) & 0x10) == 0 // sqshrun (signed src) vs uqshrn (unsigned)
        };
        return Inst::SatNarrowShift {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            src_esize,
            dst_esize: src_esize / 2,
            shift,
            src_signed,
            dst_signed,
            q: (insn >> 30) & 1 == 1,
        };
    }

    // ---- SIMD shift-right accumulate (usra/ssra Vd.T, Vn.T, #imm): Vd += Vn >> imm.
    // Same 0x0f/0x2f/0x4f/0x6f prefix family as shl but the ACCUM marker is bit12
    // ((insn & 0x0000_7000)==0x0000_1000, vs shl's 0x5000 and ushr's 0x0000).
    // unsigned=bit29 (top 0x6f->usra / 0x4f->ssra, same as SimdShr; NOT bit11).
    // bit23 clear is the shift-by-imm discriminator vs fmla-el (fmla-el requires
    // bit23 set; usra/ssra shift-imm leave it clear).
    // esize + shift mirrored from the verified SimdShr gate below: immh is the
    // FULL bits[22:19] (bit22 = size-tag, NOT droppable), esize from its MSB,
    // right-shift amount = 2*esize_bits - (immh:immb). The old derivation took
    // only 3 bits via (insn>>19)&0x7 + trailing_zeros, which collapsed EVERY
    // esize>=4 shift to esize=1/shift=0, silently making ssra accumulate
    // without shifting (e.g. ssra .2d #2 returned -8-16=-24 not -2-4=-6).
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x2f | 0x4f | 0x6f)
        && (insn & 0x0000_7000) == 0x0000_1000 && (insn & 0x0080_0000) == 0
        && (insn & 0x0078_0000) != 0 // immh != 0, exclude the movi/mvni imm family
    {
        let immh4: u32 = (insn >> 19) & 0xf;
        let fls = 32 - immh4.leading_zeros(); // highest set bit, 1-indexed
        let esize: u8 = 1u8 << (fls - 1);
        let esize_bits = 8 * esize as u32;
        let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
        let max = 2 * esize_bits;
        let shift: u8 = if full < max { (max - full) as u8 } else { 0 };
        return Inst::SimdShrAcc {
            rd: (insn & 0x1f) as u8,
            rn: ((insn >> 5) & 0x1f) as u8,
            esize,
            shift,
            unsigned: (insn >> 29) & 1 == 1,
        };
    }

    // ---- SIMD shift-right-narrow: shrn/shrn2 Vd.T, Vn.U, #imm ----
    // shrn shifts each DOUBLE-width source element (e.g. .2D=64-bit) right and
    // truncates to a HALF-width dest element (.2S=32-bit). Distinct from plain
    // ushr/sshr (equal source/dest width) by bit15 (0x8000) SET. Q=0 (prefix
    // 0x0f): dest low half; Q=1 shrn2 (prefix 0x4f): dest upper half. immh is
    // the SOURCE element size (fls -> 2,4,8,16B); shift = src_esize_bits -
    // (immh:immb), i.e. 2*total_bits - (immh:immb) like the plain form but the
    // source element is twice the dest element.
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x4f)
        && (insn & 0x0000_7000) == 0 && (insn & 0x0080_0000) == 0
        && (insn & 0x8000) != 0
    {
        let immh4: u32 = (insn >> 19) & 0xf;
        if immh4 != 0 {
            let fls = 32 - immh4.leading_zeros(); // highest set bit, 1-indexed
            let esrc: u8 = 1u8 << fls; // source element size in bytes (double dest)
            let esrc_bits = 8 * esrc as u32;
            let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
            // shrn: the 7-bit (immh:immb) field is the DEST-window size; shift =
            // 2*dest_esize_bits - (immh:immb) == source_esize_bits - (immh:immb).
            // e.g. shrn .2S #16 from .2D (immh4=6, immb=0, esrc=8):
            //   esrc_bits=64, full=48 -> shift=16.
            let shift = if full < esrc_bits { (esrc_bits - full) as u8 } else { 0 };
            return Inst::SimdShrn {
                            rd: (insn & 0x1f) as u8,
                            rn: ((insn >> 5) & 0x1f) as u8,
                            esrc,
                            shift,
                            upper: (insn >> 30) & 1 == 1,
                            round: (insn & 0x800) != 0, // bit11: 0=shrn(truncate), 1=rshrn(round)
                        };
        }
    }

    // ---- SIMD plain shift-right immediate: ushr/sshr Vd.T, Vn.T, #imm ----
    // Marker (insn & 0x0000_7000)==0b000 (the "ushr's 0x0000" note in ShrAcc), and
    // immh != 0 to exclude the modified-immediate movi/mvni family (immh==0). This
    // MUST come before the broad VecMovi gate (top byte {0F,1F,2F,4F,5F,6F}) which
    // otherwise mis-decodes ushr/sshr as a vector immediate (silent wrong value).
    // unsigned = bit29 (0x2f/0x6f). shift = esize_bits - (immh:immb), same as ShrAcc.
    if matches!((insn >> 24) & 0x0f, 0x0f | 0x2f | 0x4f | 0x6f)
        && (insn & 0x0000_7000) == 0 && (insn & 0x0080_0000) == 0
    {
        let immh = (insn >> 19) & 0x7;
        if immh != 0 {
            // esize from the MSB position of immh: esize_bytes = 1<<(fls-1).
            // e.g. immh4=7 (0b0111) -> fls=3 -> esize=4 (32-bit, .2s); immh4=13 -> esize=8.
            let immh4: u32 = immh | ((insn >> 22) & 1) << 3; // reconstruct full bits[22:19]
            let fls = 32 - immh4.leading_zeros(); // highest set bit, 1-indexed
            let esize: u8 = 1u8 << (fls - 1);
            let esize_bits = 8 * esize as u32;
            // right shift: amount = 2*esize_bits - (immh4:immb). Verified: ushr.2s #8
            // (immh4=7,immb=0) -> 64-56=8; ushr.2d #17 (immh4=13,immb=7) -> 128-111=17.
            let full: u32 = (immh4 << 3) | ((insn >> 16) & 0x7);
            let max = 2 * esize_bits;
            let shift: u8 = if full < max { (max - full) as u8 } else { 0 };
            return Inst::SimdShr {
                rd: (insn & 0x1f) as u8,
                rn: ((insn >> 5) & 0x1f) as u8,
                esize,
                shift,
                unsigned: (insn >> 29) & 1 == 1,
            };
        }
    }

    // ---- SIMD FP multiply by element: fmul Vd.T, Vn.T, Vm.T[L] ----
    // Gate (insn & 0x3f00_f000)==0x0f00_9000. Must precede the broad MOVI gate
    // (0x0F|0x6F prefix) which would otherwise swallow 0x0fa29044. esize from
    // size field; the indexed element index and the Vm register field share the
    // bits above the 5-bit Vm, per element size (A64 by-element encoding):
    //   16-bit: Vm=bits[19:16], index=bits[21:20] (4 subelements)
    //   32-bit: Vm=bits[20:16], index=bits[21]    (2 subelements)
    //   64-bit: Vm=bits[20:16], index=bits[11]    (2 subelements)
    if insn & 0x3f00_f000 == 0x0f00_9000 {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let q = (insn >> 30) & 1 == 1;
        let size = (insn >> 22) & 0x3;
        let (esize, rm, index): (u8, u8, u8) = match size {
            // 32-bit (2S/4S): index = H:L = bit11:bit21 (2 subelements? NO — 4).
            // A64 by-element: for 32-bit elements the index is bits [21] and [11],
            // in order index = bit11<<1 | bit21 (ground truth: s[0]=00, s[1]=01
            // (b21), s[2]=10 (b11), s[3]=11). Verified via asm: 0x4f809041 s[0],
            // 0x4fa09041 s[1], 0x4f809841 s[2], 0x4fa09841 s[3].
            2 => (4, ((insn >> 16) & 0x1f) as u8,
                  ((((insn >> 11) & 1) << 1) | ((insn >> 21) & 1)) as u8),
            _ => (8, ((insn >> 16) & 0x1f) as u8, ((insn >> 11) & 1) as u8),
        };
        return Inst::SimdFmulEl { rd, rn, rm, esize, index, q };
    }

    // ---- SIMD byte reverse in 64-bit element: rev64 Vd.T, Vn.T ----
    // Gate (insn & 0x3f00_f800)==0x0e00_0800 (REV64-family residue; q=bit30;
    // arrangement via size bits). Byte-reverse each 64-bit granule.
    // MUST also require bits[13:12]==00: the raw `==0x0e00_0800` residue drops
    // bit12, so the unzip ops (uzp1 byte1 0x18, uzp2 0x58) were misdecoded as
    // rev64 (gcc's magic-division reducer uses uzp2 to gather product-high words).
    if insn & 0x3f00_ff00 == 0x0e00_0800 {
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::SimdRev { rd, rn, granule: 8, q: (insn >> 30) & 1 == 1 };
    }
    // ---- SIMD rev16 (granule 2): rev16 Vd.16B/.8B, Vn ---- byte-swap within
    // each 16-bit halfword. Real encodings 0x0e201800 (.8b) / 0x4e201800
    // (.16b). This is the two-reg-misc REV16 (byte1 0x18, bit21 set), which is
    // DISJOINT from the uzp1/trn1 3-same op (uzp1 v0.16b=0x4e011800 has bit21
    // CLEAR — the permute gate keeps prefix 0x0e00_1800 with bit21=0). Mask
    // 0x3f20_f800 pins (a) the two-reg-misc prefix lanes (0x3f00_0000), (b)
    // bit21 SET (0x0020_0000, the rev16-vs-uzp discriminator), and (c) byte1
    // 0x18 (0x0000_f800). rev64 above already consumed byte1==0x08.
    if (insn & 0x3f20_f800) == 0x0e20_1800 {
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        return Inst::SimdRev { rd, rn, granule: 2, q: (insn >> 30) & 1 == 1 };
    }
    if (insn & 0x3f00_0c00) == 0x2e00_0800 && (insn & 0x3c00) == 0x0800 && (insn & 0x0020_0000) != 0 && (insn & 0x0000_f000) == 0 {
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

    // ---- SIMD load/store MULTIPLE structures: ld1/ld2/st1/st2 {Vt, Vt1[, ..]}, [Xn] ----
    // Class from bits[29:22] (mask 0xffc0_0000): LD has bit22 (L=1) and R=bit21;
    // prefixes 0x4c40(ld,q1)/0x0c40(ld,q0)/0x4c00(st,q1)/0x0c00(st,q0).
    // The STRUCTURE TYPE + register count is opcode bits[15:12]:
    //   0x8 = LD2/ST2 (DEINTERLEAVE Vt[i]=m[2i], Vt1[i]=m[2i+1])
    //   0xA = LD1/ST1 2-register, 0x7/0x0 = 1-register, 0x6 = 3-reg, 0x2 = 4-reg
    //         -> LD1/ST1 MULTIPLE loads/stores `nreg` CONSECUTIVE (q?16:8)-byte
    //         vectors (NO deinterleave) — the compiler's array-literal idiom.
    // post-index writeback when bit23=1 (advance Xn by the total bytes). The
    // POST-INDEX forms set bit23, giving bases 0x..cc0 (ld) / 0x..c80 (st);
    // those were missing and the 2/3/4-register post-indexed ld1 (gcc's
    // `ld1 {v26.16b,v27.16b}, [x1], #32`) fell through to the single-vector
    // Ld1V gate — loaded only 16B with the wrong pointer advance (fma -O2
    // accumulated garbage: returns 165 vs oracle 470).
    let sc = insn & 0xffc0_0000;
    if sc == 0x0c40_0000 || sc == 0x4c40_0000 || sc == 0x0c00_0000 || sc == 0x4c00_0000
        || sc == 0x0cc0_0000 || sc == 0x4cc0_0000 || sc == 0x0c80_0000 || sc == 0x4c80_0000
    {
        let q = (insn & 0x4000_0000) != 0;
        let ld = (insn & 0x0040_0000) != 0; // L bit22
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let block = if q { 16 } else { 8 } as i32;
        let wb = if (insn >> 23) & 1 == 1 { true } else { false };
        let op = (insn >> 12) & 0xf;
        // Element size for the structure (deinterleave) forms: bits[11:10].
        let size = (insn >> 10) & 0x3;
        let esize = 1u8 << size;
        if op == 0x8 {
            // LD2 / ST2 — deinterleave 2 registers (existing behavior).
            let post = if wb { block * 2 } else { 0 };
            return if ld {
                Inst::Ld2 { rd, rn, q, post, esize }
            } else {
                Inst::St2 { rd, rn, q, post, esize }
            };
        }
        // LD4/ST4 (op=0b0000) and LD3/ST3 (op=0b0100) are structure
        // DEINTERLEAVE loads — the disassembler prints them as ld4{..}/ld3{..}.
        // They must NOT fall through to the ld1-multiple (consecutive) path.
        if op == 0x0 {
            let post = if wb { block * 4 } else { 0 };
            return if ld {
                Inst::Ld4N { rd, rn, q, post, esize }
            } else {
                Inst::St4N { rd, rn, q, post, esize }
            };
        }
        if op == 0x4 {
            let post = if wb { block * 3 } else { 0 };
            return if ld {
                Inst::Ld3N { rd, rn, q, post, esize }
            } else {
                Inst::St3N { rd, rn, q, post, esize }
            };
        }
        {
            // LD1 / ST1 multiple — register count from opcode (CONSECUTIVE,
            // no deinterleave).
            let nreg: u8 = match op {
                0x2 => 4,
                0x6 => 3,
                0x7 => 1,
                0xa => 2,
                _ => 0, // unsupported structure width
            };
            if nreg == 0 {
                return Inst::Unsupported(insn);
            }
            let post = if wb { block * (nreg as i32) } else { 0 };
            return if ld {
                Inst::Ld1N { rd, rn, nreg, q, post }
            } else {
                Inst::St1N { rd, rn, nreg, q, post }
            };
        }
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
    // ---- INTEGER widening multiply by element: smull/umull/smlal/umlal/smlsl/
    //      umlsl Vd.T, Vn.T, Vm.Tsb[idx] ---- GCC emits these for vector *
    //      const-scalar scaling (e.g. vmlal_lane_s16/vmlal_lane_s32 builtins).
    //      Encoding: prefix b[28:24]=01111, Q=b30, U(uns)=b29, size=b[23:22]
    //      (1 = 16-bit src -> .4s result, 2 = 32-bit src -> .2d), indexed
    //      operand reg Vm = b[19:16] (4 bits: v0-v15), index = L(b21):H(b20)
    //      for 16-bit src, or just L(b21) for 32-bit src, op b[15:12] in
    //      {2=mlal acc, 6=mlsl acc-sub, 0xa=mull}. The bit13 (0x2000) SET is
    //      the discriminator vs FP fmla-el (op 1/5 -> bit13 CLEAR) and
    //      non-widening int mla-el (op 0 -> bit13 clear). MUST precede the FP
    //      fmla-el gate below (which also matches the .2d forms via bit23 set).
    //      Session 44: gcc vmlal_lane/vmull_lane builtins previously decoded
    //      as FmlaEl/VecMovi (silent wrong value). Verified gates/fields
    //      against 25 assembler ground-truth encodings.
    if (insn & 0x1f00_0000) == 0x0f00_0000 && (insn & 0x0000_2000) != 0 {
        let size = (insn >> 22) & 3; // b[23:22]: 1 = 16-bit src -> .4s res(4), 2 = 32-bit src -> .2d res(8)
        // only size 1 (.4s) and 2 (.2d) are the widening forms
        if size == 1 || size == 2 {
            let res_esize: u8 = 2u8 << size;
            let rm = ((insn >> 16) & 0xf) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            let index = if res_esize == 4 {
                // 16-bit source: index = L(b21):H(b20) (2 bits)
                ((((insn >> 21) & 1) << 1) | ((insn >> 20) & 1)) as u8
            } else {
                // 32-bit source: 1-bit index = L(b21)
                ((insn >> 21) & 1) as u8
            };
            let op = (insn >> 12) & 0xf;
            let acc = op != 0xa;
            let sub = op == 0x6;
            return Inst::SimdMullEl {
                rd, rn, rm, index,
                res_esize,
                unsigned: ((insn >> 29) & 1) == 1,
                q: (insn >> 30) & 1 == 1,
                acc,
                sub,
            };
        }
    }
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
            // .s: index = bit11<<1 | bit21 (ground truth: s[1]=b21, s[2]=b11,
            // s[3]=both). Same layout as SimdFmulEl.
            ((((insn >> 11) & 1) << 1) | ((insn >> 21) & 1)) as u8
        };
        let el32 = (insn & 0x0040_0000) != 0;
        let q = (insn & 0x4000_0000) != 0;
        let sub = (insn & 0x4000) != 0;
        return Inst::FmlaEl { rd, rn, vlm, idx, el64: el32, q, sub };
    }
    // ---- INTEGER 32-bit lane MLA/MLS by element: mla/mls Vd.4S/2S, Vn, Vm.S[idx] ----
    // Encodings assembled & objdump-verified (see /tmp/mlae.s): mla 2s/4s =
    // 0x2f820020/0x6fa20020, mls = 0x2f824020/0x6f824820. Top nibble 0x0f with
    // bit29 SET (0x2f/0x6f) — the opposite of FP fmla-el (0x0f/0x4f, bit29
    // CLEAR), so it is disjoint. mls = bit14, q = bit30, .s index = bit21.
    // Byte1 top-nibble 0x8 excludes the widening smlal/umlal (0x42) forms.
    // MUST be before VecMovi, which otherwise decodes these as a vector
    // immediate (silent wrong-value: gcc int->float init used this and every
    // lane of the a*scalar product was garbage).
    if ((insn >> 24) & 0x0f) == 0x0f
        && (insn & 0x2000_0000) != 0
        && (insn & 0x0080_0000) != 0
    {
        let rd = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        // .4s index (0..3) = (bit11 << 1) | bit21; .2s (0..1) = bit21.
        let index = ((((insn >> 11) & 1) << 1) | ((insn >> 21) & 1)) as u8;
        let q = (insn >> 30) & 1 == 1;
        let sub = (insn & 0x4000) != 0;
        let lanes = if q { 4u8 } else { 2u8 };
        return Inst::SimdMlaEl { rd, rn, rm, index, lanes, sub };
    }
    // ---- SIMD permutes (zip1/zip2, uzp1/uzp2): 3-same, byte2 ----
    // byte2 (bits15:8): zip1=0x38 zip2=0x78 uzp1=0x18 uzp2=0x58. These SHARE
    // byte2 0x38 with the shll shift-imm family, so this gate MUST precede the
    // WidenShl gate below, otherwise gcc's `zip1 v.4s` (float-vector init /
    // interleave) is misdecoded as a widening shift -> silent wrong vector
    // (fv_arith / dv_arith small-error corruption).
    // esc: esize = 1<<bits[23:22]; q = bit30.
    {
        let p = insn & 0x3f20_fc00;
        let kind = if p == 0x0e00_3800 {
            Some(1u8) // zip1
        } else if p == 0x0e00_7800 {
            Some(2u8) // zip2
        } else if p == 0x0e00_1800 {
            Some(3u8) // uzp1
        } else if p == 0x0e00_5800 {
            Some(4u8) // uzp2
        } else {
            None
        };
        if let Some(kind) = kind {
            let esize = (1 << ((insn >> 22) & 0x3)) as u8;
            let q = (insn >> 30) & 1 == 1;
            let rd = (insn & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rm = ((insn >> 16) & 0x1f) as u8;
            return if kind == 2 {
                Inst::SimdZip2 { rd, rn, rm, esize, q }
            } else if kind == 3 {
                Inst::SimdUz1 { rd, rn, rm, esize, q }
            } else if kind == 4 {
                Inst::SimdUz2 { rd, rn, rm, esize, q }
            } else {
                Inst::SimdZip1 { rd, rn, rm, esize, q }
            };
        }
    }
    // ---- SIMD widening shift-left (sign/zero extend): shll/usll Vd.Td, Vn.Ts ----
    // byte2 (bits15:8) == 0x38; byte0 in the SHLL family {0e,2e,4e,6e}. Reads the
    // low nlanes half-width elements of Vn, sign/zero-extends each to double width.
    // NOTE bit28==0 is REQUIRED: the scalar-FP 0x1e byte3-low-nibble (fsub =
    // 0x1e61_3800 has byte3 0x1e) ALSO gives (insn>>24)&0x0f == 0x0e, so without
    // excluding it this gate swallows scalar `fsub d0,d0,d1`. Real shll bytes are
    // 0x0e/0x2e/0x4e/0x6e (bit28=0); 0x1e/0x3e/0x5e/0x7e are the scalar-FP 0x1e..0x80 family.
    // Additionally require byte1 bits[1:0]==00: the NEON permute ops (zip1 byte1
    // 0x39/0x3b, uzp1 0x19, zip2 0x79...) share `byte1&0x7c==0x38` but set bit0/bit1,
    // so without this guard `zip1`/`uzp1` were swallowed as a bogus WidenShl (gcc's
    // magic-division uzp2/zip gather). shll's byte1 is exactly 0x38 (bits[1:0]=00).
    if ((insn >> 24) & 0x0f) == 0x0e && ((insn >> 28) & 1) == 0 && ((((insn >> 8) & 0xff) & 0x7c) == 0x38)
        && (insn & 0x200000) != 0
    {
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
            if ((insn >> 8) & 0xff & 0xfc) == 0x68 && matches!((insn >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e) {
                let rd = (insn & 0x1f) as u8;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let b1 = (insn >> 16) & 0xff;
                let src_esize: u8 = 1 << ((insn >> 22) & 3); // size=bits[23:22]: 0=>.b(1),1=>.h(2),2=>.s(4)
                let signed = (insn >> 29) & 1 == 0; // 0x4e/0x0e signed, 0x6e/0x2e unsigned
                let q = (insn >> 30) & 1 == 1; // 0x4e/0x6e upper half
                let n_src_bytes = if q { 16 } else { 8 };
                let n_pairs = (n_src_bytes / (2 * src_esize as usize)) as u8;
                return Inst::SimdAdalp { rd, rn, src_esize, n_pairs, signed, upper: q, acc: true };
            }
            // ---- SIMD add-adjacent-long pairwise (non-accumulating): saddlp/uaddlp ----
            // byte2&0xfc == 0x28 (bit2 distinguishes from uqsub 0x2c) with bit16(0x10000)
            // CLEAR distinguishes paddl from sqxtun (bit16 SET). Prefix 0x0e/2e/4e/6e.
            // This MUST precede the SaturatNarrow + sat-sub gates so
            // vpaddl/saddlp aren't swallowed by them (uqsub 0x0e622c20 &0xf8==0x28).
            if (((insn >> 8) & 0xff & 0xfc) == 0x28) && (insn & 0x10000) == 0
                && matches!((insn >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e) {
                let rd = (insn & 0x1f) as u8;
                let rn = ((insn >> 5) & 0x1f) as u8;
                let src_esize: u8 = 1 << ((insn >> 22) & 3);
                let signed = (insn >> 29) & 1 == 0; // 0x4e/0x0e signed
                let q = (insn >> 30) & 1 == 1;
                let n_src_bytes = if q { 16 } else { 8 };
                let n_pairs = (n_src_bytes / (2 * src_esize as usize)) as u8;
                return Inst::SimdAdalp { rd, rn, src_esize, n_pairs, signed, upper: q, acc: false };
            }
            // ---- SIMD saturating add/sub: sqadd/uqadd/sqsub/uqsub Vd.T, Vn, Vm -----
            // byte2 {0x0c (add), 0x2c (sub)}; prefix 0x0e/2e/4e/6e (signed 0e/4e, unsigned 2e/6e).
            // NOTE bit21 must be SET: a clear bit21 + byte2==0x0c is NOT a
            // sat-add but `dup Vd.T, Wn/Xn` (GPR broadcast, e.g. 0x4e040c3e),
            // which the SimdDupGp gate below handles. Checked against the
            // assembler: all 14 sat-add widths/forms set bit21; all 6 dup-from-
            // GPR forms clear it. (Without this the dup fell through and the
            // JIT computed a saturating add of Vn,Vm, silently corrupting every
            // gcc -O2 matrix-init loop that broadcasts an index into a vector.)
            let b2s = (insn >> 8) & 0xff;
            if (b2s == 0x0c || b2s == 0x2c)
                && (insn & 0x200000) != 0
                && matches!((insn >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e)
            {
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
            // esrc (source element size in bytes) = 1<<bits[23:22] (1/2/4); sign=bit29==0;
            // upper=bit30. The residue covers all 24 esrc×signedness×upper×add|sub forms
            // (0x..20 esrc=1, 0x..60 esrc=2, 0x..a0 esrc=4); previously only the 8 esrc=2
            // forms were gated, so uaddl/saddl .2d (esrc=4) and .8h (esrc=1) fell through to
            // Unsupported. Disjoint from smull/umull (0x..c000), smlal (0x..8000), and
            // ssubw wide (0x..3000) — verified numerically + objdump.
            let al = insn & 0xffe0_fc00;
            let alres = [
                0x0e20_0000u32,0x0e20_2000u32,0x2e20_0000,0x2e20_2000,
                0x0e60_0000,0x0e60_2000,0x2e60_0000,0x2e60_2000,
                0x0ea0_0000,0x0ea0_2000,0x2ea0_0000,0x2ea0_2000,
                0x4e20_0000,0x4e20_2000,0x6e20_0000,0x6e20_2000,
                0x4e60_0000,0x4e60_2000,0x6e60_0000,0x6e60_2000,
                0x4ea0_0000,0x4ea0_2000,0x6ea0_0000,0x6ea0_2000,
            ];
            if alres.contains(&al) {
                            let esrc: u8 = 1u8 << ((insn >> 22) & 3); // 1/2/4-byte source elements
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
            max: (b2 & 0xfc) == 0x64, // mask the 2 low bits (they carry Rn) — exact == wrongly decoded real smax (b2=0x67) as min
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
            (1, 0b1110) => {
                // MOVI Vd.2D, #<imm>: the 64-bit element is a byte-select
                // pattern. bits[9:5] low nibble selects which of bytes 0..3 of
                // the LOW 32 bits are set to 0xff (others 0); bits[18:16]
                // (3-bit) plus bit-4 of bits[9:5] extend to the upper 32 bits.
                // objdump/qemu-verified ground truth:
                //   #0x00000000000000ff  b9:5=0b0001 -> byte0
                //   #0x000000000000ff00  b9:5=0b0010 -> byte1
                //   #0x000000000000ffff  b9:5=0b0011 -> bytes0,1
                //   #0x00000000ff000000  b9:5=0b1000 -> byte3
                //   #0x00000000ffff0000  b9:5=0b1100 -> bytes2,3
                //   #0xffffffff00000000  b9:5=0b10000 b18:16=7 -> upper 32 all-ff
                let sel = (insn >> 5) & 0x0f; // byte select for low 32 bits
                let mut lo = 0u32;
                for j in 0..4 {
                    if sel & (1 << j) != 0 {
                        lo |= 0xffu32 << (8 * j);
                    }
                }
                let lo = lo as u64;
                // upper 32 bits: bit4 of bits[9:5] selects a whole-low-32 fill,
                // b[18:16] acts as an independently-addressed low-byte set on
                // the upper half (the real encodings: 7 -> all upper ff).
                let sel_hi = (insn >> 9) & 1; // bit4 of the 5-bit select
                let hi_hi = (insn >> 16) & 0x7;
                let hi: u64 = if sel_hi != 0 && hi_hi == 0x7 {
                    0xffff_ffff
                } else if sel_hi != 0 {
                    lo as u64
                } else {
                    hi_hi as u64 & 0x7  // rare; conservative
                };
                (lo | (hi << 32), lo | (hi << 32))
            }
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
            // element = (imm8 << (8*idx)) | (all-ones mask in the low shift bits),
            // INVERTED for mvni (op==1) to a 32-bit word.
            (_, 0xc) | (_, 0xd) => {
                let sh = (((cmode & 0x1) + 1) * 8) as u32; // 8 or 16
                let mut lane = ((imm8 as u64) << sh) | ((1u64 << sh) - 1);
                if op == 1 { lane = (!lane) & 0xffff_ffff; }
                let low = lane | (lane << 32);
                if (insn >> 30) & 1 == 1 { (low, low) } else { (low, 0) }
            }
            _ => return Inst::Unsupported(insn),
        };
        return Inst::VecMovi {
                    vd: rd(insn),
                    lo,
                    hi,
                    // cmode LSB: 0 = MOVI/MVNI (write), 1 = ORR/BIC (RMW).
                    // op: 0 -> movi/orr, 1 -> mvni/bic.
                    kind: if (cmode & 1) == 1 {
                        if op == 1 { 1 } else { 2 } // bic (AND ~imm) | orr (OR imm)
                    } else {
                        0 // write
                    },
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
                        // Mask ffe0_fc00 keeps bit31 (q), bit29 (U: 1=ushl, 0=sshl),
                        // bits23:22 (size), and the {000x_0000}100 opcode field.
                        // gcc emits ushl for unsigned lanes, sshl for signed. Note:
                        // U (bit29) is NOT a sign flag — ushl (unsigned) sets it to 1,
                        // sshl (signed) clears it. signed_=true means sshl (arithmetic).
                        let vsh = insn & 0xffe0_fc00;
                        if matches!(vsh, 0x2e20_4400 | 0x2e60_4400 | 0x2ea0_4400
                                       | 0x6e20_4400 | 0x6e60_4400 | 0x6ea0_4400
                                       | 0x6ee0_4400 | 0x0e20_4400 | 0x0e60_4400
                                       | 0x0ea0_4400 | 0x4e20_4400 | 0x4e60_4400
                                       | 0x4ea0_4400 | 0x4ee0_4400) {
                            return Inst::SimdVShift {
                                rd: (insn & 0x1f) as u8,
                                rn: ((insn >> 5) & 0x1f) as u8,
                                rm: ((insn >> 16) & 0x1f) as u8,
                                esize: (1u8 << ((insn >> 22) & 0x3)),
                                signed_: (insn >> 29) & 1 == 0,
                                q: (insn >> 30) & 1 == 1,
                            };
                        }

    // ---- LSE atomics (ldadd/ldclr/ldeor/ldset/swp, ARMv8.1) ----
    // gate: (insn & 0x3fe00000) in {0x38200000,0x38600000,0x38a00000,0x38e00000}
    // (the 0x38 LSE prefix; bit30=size and the acquire/release ordering bits
    // vary) AND op=bits[15:10] in {0x00,0x04,0x08,0x0c,0x20} (LDADD/LDCLR/
    // LDEOR/LDSET/SWP). Disjoint from CAS (0x08/0xc8) and from the register-
    // offset LDR/STR (whose option bits make op 0x18/0x1c/0x34...). Must come
    // BEFORE the register-offset and LdStrImmWb gates (they share the 0xb8/0xf8
    // top bytes). Rs=bits[20:16], Rn=bits[9:5], Rt=bits[4:0], size64=bit30.
    // Verified vs real modmain.elf encodings (swpa/ldadda/ldseta/ldeorl/swpal).
    if matches!(insn & 0x3fe0_0000, 0x3820_0000 | 0x3860_0000 | 0x38a0_0000 | 0x38e0_0000)
        && matches!((insn >> 10) & 0x3f, 0x00 | 0x04 | 0x08 | 0x0c | 0x20)
    {
        let op = match (insn >> 10) & 0x3f {
            0x04 => 1, // LDCLR
            0x08 => 2, // LDEOR
            0x0c => 3, // LDSET
            0x20 => 4, // SWP
            _ => 0,    // LDADD
        };
        return Inst::LseAtomic {
            op,
            size64: (insn & 0x4000_0000) != 0,
            rs: b(insn, 16, 20) as u8,
            rn: b(insn, 5, 9) as u8,
            rt: b(insn, 0, 4) as u8,
        };
    }

    // ---- SIMD/NEON 128-bit REGISTER-offset load/store (ldr/str q0,[xN,xM]) ----
    // bit26=1 (vector file) + bit21=1 (Q 128-bit) selects this class. MUST
    // precede the GPR register-offset gate below: that gate (0x38200800) does
    // not mask bit26, so a `str q0,[x0,x3]` (0x3ca36800) would otherwise decode
    // as `ldrsb x0,[x0,x3]` — silently clobbering guest x0. Register + option
    // (0x3ca0_0800/0x3ce0_0800 after masking rt/rn/rm/option) key on ld=bit22.
    {
        let m = insn & 0xffe0_0c00;
        if m == 0x3ca0_0800 || m == 0x3ce0_0800 {
            return Inst::VecLdStrReg {
                vt: (insn & 0x1f) as u8,
                rn: b(insn, 5, 9) as u8,
                rm: b(insn, 16, 20) as u8,
                ld: (insn >> 22) & 1 == 1,
            };
        }
    }

    // ---- SIMD/NEON 128-bit UNSCALED load/store (ldur/stur q0,[xN,#imm9]) ----
    // bit26=1 + bit21=0 (unscaled, not register-offset). (insn & 0xffe00c00) is
    // 0x3c80_0000 for stur and 0x3cc0_0000 for ldur (the signed imm9 lives in
    // bits[20:12], outside the masked rm/suffix region — verified: stur q0,[x5,
    // #-16]=0x3c9f00a0). Must NOT collide with the scalar-B unscaled form
    // (stur b0=0x3c1fc100, which masks to 0x3c00_0000, bit23 clear).
    {
        let m = insn & 0xffe0_0c00;
        if m == 0x3c80_0000 || m == 0x3cc0_0000 {
            let imm9 = (((insn >> 12) & 0x1ff) as i32) << 23 >> 23; // sign-ext 9 bits
            return Inst::VecLdStImmUnscaled {
                vt: (insn & 0x1f) as u8,
                rn: b(insn, 5, 9) as u8,
                imm9,
                ld: (insn >> 22) & 1 == 1,
            };
        }
    }

    // ---- FP/SIMD SCALAR (B/H/S/D) register-offset load/store ---- bit26=1 +
    // register-offset form (0x38200800 residue); Q 128-bit is handled above
    // (bit23 set) so this catches the scalar widths via bits[31:30]. `S` (bit12)
    // scales the index by log2(size). Mirrors GPR LdStrReg address math but
    // reads/writes the vector slot v[vt] instead of a GPR rt.
    if (insn & 0x0400_0000) != 0
        && (insn & 0x3b20_0c00) == 0x3820_0800
        && (insn & 0x0080_0000) == 0
    {
        let size = match (insn >> 30) & 0x3 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        return Inst::FpLdStrReg {
            vt: (insn & 0x1f) as u8,
            rn: b(insn, 5, 9) as u8,
            rm: b(insn, 16, 20) as u8,
            size,
            ld: (insn >> 22) & 1 == 1,
            shift: (insn >> 12) & 1 == 1,
            index_ext: ((insn >> 13) & 3) as u8,
        };
    }

    // ---- SIMD/NEON 128-bit PRE/POST-index writeback load/store (ldr/str q) ----
    // (insn & 0xffe00c00) in {0x3cc00c00 (ldr pre), 0x3cc00400 (ldr post),
    // 0x3c800c00 (str pre), 0x3c800400 (str post)}. bit26=1 + the 0x0c00/0x0400
    // suffix distinguishes writeback from unscaled (0x0800) / register-offset
    // (bit21=1). Xn advances by signed imm9 after the transfer.
    {
        let m = insn & 0xffe0_0c00;
        if m == 0x3cc0_0c00
            || m == 0x3cc0_0400
            || m == 0x3c80_0c00
            || m == 0x3c80_0400
        {
            let imm9 = (((insn >> 12) & 0x1ff) as i32) << 23 >> 23; // sign-ext 9 bits
            return Inst::VecLdStIndexed {
                vt: (insn & 0x1f) as u8,
                rn: b(insn, 5, 9) as u8,
                imm9,
                ld: (insn >> 22) & 1 == 1,
                pre: (insn >> 11) & 1 == 1,
            };
        }
    }

    // ---- load/store (register offset) ---- ONLY the register-offset form.
    // The old gate (insn & 0x3b00_0000)==0x3800_0000 was too broad: it also
    // swallowed the pre/post-index and unscaled IMMEDIATE forms (0xf84x/0x78x/
    // 0x384x), mis-reading their imm9+writeback bits as an `rm` register and
    // dereferencing garbage (often 0 — the glibc auxv-scan crash). The precise
    // register-offset test is (insn & 0x3b200c00) == 0x38200800 (option bits:
    // real LdStrReg sxtw/lsl forms like 0xf8627803 and 0xb862d803 match; the
    // immediate forms 0x38000c00/0x38000400/0x38000000 fall through to the
    // LdStrImmWb decoder below). bit26 must be 0 (GPR file): a SIMD/FP
    // register-offset ld/st (bit26=1, e.g. str q0,[x0,x3]) is a vector op
    // handled above — without this mask it mis-decoded as a byte GPR store/
    // sign-extend load into the base register.
    if (insn & 0x3b20_0c00) == 0x3820_0800 && (insn & 0x0400_0000) == 0 {
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
            index_ext: ((insn >> 13) & 3) as u8,
        };
    }

    // ---- load/store (pre/post-index + unscaled) Signed imm9 ---- 0xf84x/
    // 0xb84x/0x784x/0x384x (the non-unsigned-offset, non-register-offset 0x38
    // family). `ldr Xt, [Xn, #imm9]!` (pre), `[Xn], #imm9` (post), or
    // `[Xn, #imm9]` (unscaled, no writeback). imm9 is SIGNED (bits[20:12]).
    // The three modes are (insn & 0x3b200c00): pre=0x38000c00, post=
    // 0x38000400, unscaled=0x38000000 (register-offset 0x38200800 already
    // returned above). Before this decoder these forms were mis-read as
    // register-offset LdStrReg with garbage `rm` (a real NULL-deref bug in
    // glibc's auxv/env scan `ldr x3,[x0],#8`). bit26 must be 0 (GPR file): the
    // SIMD/FP vector unscaled forms (stur/ldur q, bit26=1) are handled above by
    // VecLdStImmUnscaled — without the mask an (old) broad gate mis-read a
    // `stur q0,[x5,#-16]` as a 1-byte GPR op writing the base register.
    if (matches!(insn & 0x3b20_0c00, 0x3800_0c00 | 0x3800_0400) || (insn & 0x3b20_0c00) == 0x3800_0000)
        && (insn & 0x0400_0000) == 0
    {
        let size = match (insn >> 30) & 0x3 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let ld = (insn >> 22) & 1 == 1;
        // Sign-extend load (ldrsw/ldrsh/ldrsb): bit23 set (opc 10/11).
        let sext = (insn & 0x80_0000) != 0;
        let ld = ld || sext;
        let imm9_raw = (insn >> 12) & 0x1ff; // signed 9-bit
        let imm9 = if imm9_raw & 0x100 != 0 {
            imm9_raw as i32 - 0x200
        } else {
            imm9_raw as i32
        };
        let rn = b(insn, 5, 9) as u8;
        let rt = b(insn, 0, 4) as u8;
        let mode = insn & 0x3b20_0c00;
        let (writeback, pre) = match mode {
            0x3800_0c00 => (true, true),  // pre-index [Xn,#imm9]!
            0x3800_0400 => (true, false), // post-index [Xn],#imm9
            _ => (false, false),          // unscaled [Xn,#imm9]
        };
        return Inst::LdStrImmWb {
            rt,
            rn,
            imm9,
            size,
            ld,
            sext,
            writeback,
            pre,
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
//      (the EXTR base; UBFM is 0x130/0x136 — disjoint). rm==rn => rotation;
//      rm!=rn (e.g. `extr x0,x0,x1,#51`, gcc's `(x>>51)|(x<<13)`) is the general
//      extract and must NOT fall through to the UBFM/SBFM gate below.
    if matches!(insn & 0x1fe0_0000, 0x1380_0000 | 0x13c0_0000) {
        let sf = (insn >> 31) & 1 == 1;
        let lsb = b(insn, 10, 15); // extract amount (6-bit, 0..63)
        let rn = ((insn >> 5) & 0x1f) as u8;
        let rm = ((insn >> 16) & 0x1f) as u8;
        let rd = (insn & 0x1f) as u8;
        if rm == rn {
            return Inst::Ror { rd, rn, rot: lsb, sf };
        }
        return Inst::Extr { rd, rn, rm, lsb, sf };
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
                    fbits: 0,
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
                fbits: 0,
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
            fbits: 0,
        };
    }

    // ---- FP convert to int with round toward +inf/-inf: fcvtps/pu/ms/mu (Xd dest) ----
        // Family (insn & 0xffff_0000) for the **X-dest** (top 0x9e) round forms.
        // fcvt-round to a 64-bit int covers the libroblox boot (fcvtpu x9,s0 = 0x9e290009)
        // and, critically, 0x9e.. does NOT collide with fcmp (which is always the 0x1e
        // W-dest column: 0x1e70_.. == fcmp d6,d16, but 0x9e70_.. == legitimate fcvt to X).
        // bit16=U (unsigned), bit22=double source, round: 0x28=+inf 0x30=-inf.
        {
            // Guard on bit12 CLR: the scalar FP immediate form `fmov d,#imm`
            // (e.g. 0x1e651017 = fmov d23,#12.0 with imm8 at bits[13:20]) shares
            // the top-16 `0x1e65` of fcvtau but has bit12 (0x1000, the imm lane
            // anchor) SET, whereas every real fcvt-to-int opcode keeps bit12 CLR.
            // Without this the coarse 0xffff0000 mask stole `fmov d,#imm` values
            // whose imm8 set bit16+ (m>=8: 12.0, 13.0, 14.0, 15.0) as fcvtau and
            // silently loaded 0 into the destination. Verified: fcvtzu w0,d1 =
            // 0x1e790020, fcvtas w0,d1 = 0x1e640020, fcvtau w2,d3 = 0x1e650062,
            // all bit12 CLR; fmov d23,#12.0 = 0x1e651017, bit12 SET.
            // ALSO require bits[11:10]==00: fcsel (0x1e20_0c00 residue, cond in
            // bits[15:12], rm in bits[16:20]) shares a top-16 with fcvtau
            // (d-ops 0x1e64/0x1e65) when its rm/cond bits happen to land there,
            // and fcsel MUST NOT be swallowed as an fcvt-to-int (it writes the
            // vector slot d{rd}, fcvt writes integer x{rd}; e.g. fcsel
            // d26,d28,d5,mi = 0x1e654f9a has top-16 0x1e65 -> decodes as
            // fcvtau, silently corrupting the min-accumulator). Every real
            // fcvt-to-int keeps bits[11:10]==00; fcsel needs 0b11.
            if (insn & 0x1000) == 0 && (insn & 0x0c00) == 0 {
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
                    fbits: 0,
                };
            }
            } // end bit12-clear guard (FMOV-imm exclusion)
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
            // MUST ALSO require the top byte ∈ {0x1e, 0x9e}: the old gate checked
            // only bits[29:28]==01, which the system/hint family (0xd5xxxxxx, e.g.
            // ARM `nop` = 0xd503201f) also satisfies — so every guest `nop` was
            // misdecoded as `scvtf d<n>, x0, #56`, converting the caller's x0
            // (often the stack pointer) into a double and overwriting a vector
            // register with address-derived garbage. gcc emits nops as padding
            // everywhere (verified: real scvtf d0,x0,#1 = 0x9e42fc00, s0,w0,#1 =
            // 0x1e02fc00, nop = 0xd503201f).
            {
                let top_ok = (insn as u32 >> 24) == 0x1e || (insn as u32 >> 24) == 0x9e;
                let m1 = ((insn >> 16) & 0xff) & 0xfe;
                if top_ok
                    && (m1 == 0x42 || m1 == 0x02)
                    && (insn & 0x3000_0000) == 0x1000_0000
                {
                    return Inst::ScvtfFixed {
                        rd: (insn & 0x1f) as u8,
                        rn: ((insn >> 5) & 0x1f) as u8,
                        // For scalar int->fp fixed-point, the source width (W vs
                        // X) and dest width (S vs D) are BOTH set by sf=bit31
                        // (0x9e = X/D, 0x1e = W/S). The old gate read bit30,
                        // mis-decoding every real `scvtf d,x,#fbits` (e.g.
                        // d0,x0,#1 = 0x9e42fc00, bit30=0) as a single (Sd/Wn)
                        // conversion.
                        to_double: (insn >> 31) & 1 == 1,
                        sf: (insn >> 31) & 1 == 1,
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

                // ---- FP 3-same pairwise: faddp/fmaxp/fminp/fmaxnmp/fminnmp Vd.T,Vn.T,Vm.T ----
                // per-source pairwise reduce (halves of Vd = reduce(Vn), reduce(Vm)),
                // Q=0 .2s / Q=1 .4s. bit16 SET. prefix 0x2e/0x6e. bit13 set=min/max;
                // bit13 clear&bit12 set=faddp; bit13 clear&bit12 clear=fmaxnm/fminnm;
                // min=bit23.
                if matches!((insn >> 24) & 0xff, 0x2e | 0x6e)
                    && (insn & 0x0040_0000) == 0 // .2s/.4s only (no .2d 3-op)
                    && matches!(insn & 0xffe0_fc00,
                        0x2e20_c400 | 0x2ea0_c400 | 0x2e20_d400 | 0x2ea0_d400
                        | 0x2e20_f400 | 0x2ea0_f400
                        | 0x6e20_c400 | 0x6ea0_c400 | 0x6e20_d400 | 0x6ea0_d400
                        | 0x6e20_f400 | 0x6ea0_f400)
                {
                    let rd = (insn & 0x1f) as u8;
                    let rn = ((insn >> 5) & 0x1f) as u8;
                    let rm = ((insn >> 16) & 0x1f) as u8;
                    let b13 = (insn & 0x2000) != 0;
                    let b12 = (insn & 0x1000) != 0;
                    let add = !b13 && b12;
                    let nm = !b13 && !b12;
                    let min = (insn & 0x0080_0000) != 0;
                    return Inst::SimdFpPair3 { rd, rn, rm, add, min, nm, q: (insn >> 30) & 1 == 1 };
                }

                // ---- FP pairwise two-register reduction: fmaxp/fminp/fmaxnmp/
                // fminnmp Vd, Vn (.2s/.2d). Horizontal reduce the two elements of
                // Vn into a scalar result in Vd (a two-register form; the three-
                // operand FMAXP Vd,Vn,Vm also exists and is NOT this — bit16
                // CLEAR here, bit16 SET for the 3-op form). Op fields (verified
                // against aarch64-linux-gnu ground truth):
                //   residue(0xffe0fc00): min = bit15 (0x8000), nm = bit13 (0x2000)
                //   sz (2d) = bit22 (0x400000). Disjoint from FMaxV (3f residue
                //   0x2e20_0800) and scalar FMaxMin (0x1e20_0800) — this is the
                //   0x7e20/0x7ea0/0x7e60/0x7ee0 lane with bit16 clear.
                let pw_r = insn & 0xffe0_fc00;
                if matches!(
                    pw_r,
                    0x7e20_f800 | 0x7ea0_f800 | 0x7e60_f800 | 0x7ee0_f800
                        | 0x7e20_c800 | 0x7ea0_c800 | 0x7e60_c800 | 0x7ee0_c800
                ) {
                    let rd = (insn & 0x1f) as u8;
                    let rn = ((insn >> 5) & 0x1f) as u8;
                    // Ground truth: bit23 = min (fminp/fminnmp); nm (fmaxnm/
                    // fminnm) = bits[14:12]==4 (fmaxp/fminp have bits[14:12]==7);
                    // sz (.2d) = bit22.
                    let min = (insn & 0x0080_0000) != 0;
                    let nm = ((insn >> 12) & 7) == 4;
                    let sz = (insn & 0x0040_0000) != 0;
                    return Inst::FpPair { rd, rn, sz, min, nm };
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
                        // bit3 (0x8): the `FCMP <Dn>, #0.0` IMMEDIATE form (rm field is
                        // 0 there too, same as `fcmp Dn, D0`), so it's distinguished by
                        // bit3, not by rm. When set, the operand is literal 0.0.
                        let against_zero = (insn & 0x8) != 0 && ((insn >> 16) & 0x1f) == 0;
                        let rm = ((insn >> 16) & 0x1f) as u8;
                        return Inst::Fcmp { rn, rm, against_zero, sz };
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

                                    // ---- NEON int multiply-accumulate/subtract (32-bit lanes): mla/mls ----
                                    // ---- SIMD int 3-same pairwise max/min: smaxp/sminp/umaxp/uminp
                                    // Vd.T,Vn.T,Vm.T ---- pairwise reduce each source into halves
                                    // of Vd. b2 0xa4(max)/0xac(min); prefix 0x0e/0x2e/0x4e/0x6e;
                                    // esize=1<<size(bits[23:22]). Placed before the mla/add gates
                                    // so they can't swallow via 0x20_0c00==0x0400.
                                    if matches!((insn >> 24) & 0xff, 0x0e | 0x2e | 0x4e | 0x6e)
                                        && matches!(((insn >> 8) & 0xff) & 0xfc, 0xa4 | 0xac)
                                    {
                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                        let rd = (insn & 0x1f) as u8;
                                        let min = (insn & 0x800) != 0; // b2 bit3: 0xa4(max)/0xac(min)
                                        let unsigned = (insn >> 29) & 1 == 1;
                                        let q = (insn >> 30) & 1 == 1;
                                        let sz = (insn >> 22) & 3;
                                        let esize = 1u8 << sz;
                                        return Inst::SimdMaxMinP { rd, rn, rm, min, unsigned, esize, q };
                                    }

                                    // mla  Vd = Vd + Vn*Vm ; mls Vd = Vd - Vn*Vm (per 32-bit lane).
                                    // Gates (insn & 0xffe0_fc00): 0x0ea0_9400 (mla .2s), 0x2ea0_9400
                                    // (mls .2s), 0x4ea0_9400 (mla .4s), 0x6ea0_9400 (mls .4s). sub=bit29.
                                    // (The old gate fed these into Simd4s op:1, i.e. a plain SUBTRACT
                                    // with no multiply — gcc's magic-division reducer got the wrong
                                    // remainder, e.g. m[0]=-101 for (0*7)%101.)
                                    let mam = insn & 0xffe0_fc00;
                                    if let Some((lanes, sub)) = match mam {
                                        0x0ea0_9400 => Some((2, false)),
                                        0x2ea0_9400 => Some((2, true)),
                                        0x4ea0_9400 => Some((4, false)),
                                        0x6ea0_9400 => Some((4, true)),
                                        _ => None,
                                    } {
                                        let rm = ((insn >> 16) & 0x1f) as u8;
                                        let rn = ((insn >> 5) & 0x1f) as u8;
                                        let rd = (insn & 0x1f) as u8;
                                        return Inst::SimdMla { rd, rn, rm, lanes, sub };
                                    }

                                    // ---- NEON int add/sub (4x32 lanes): add Vd.4s, Vn.4s, Vm.4s | sub Vd.4s,... ----
                                        // class Q=1 0x0e20_0000 .. 0x4e20_0000 integer add (S: size=01);
                                        // sub is the same class with bit29 set (0x2e20_0400 vs 0x0e20_0400).
                                        let addclass = insn & 0x2f20_0c00;
                                        if (addclass == 0x0e20_0400 || addclass == 0x2e20_0400) && ((insn >> 15) & 1) == 1 && ((insn >> 22) & 3) == 2 && (insn & 0x4000) == 0 {
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
if (add2d == 0x0e20_0400 || add2d == 0x2e20_0400) && ((insn >> 15) & 1) == 1 && ((insn >> 22) & 3) == 3 && (insn & 0x4000) == 0 {
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
                                                                                    if (bc == 0x0e20_0400 || bc == 0x2e20_0400) && ((insn >> 15) & 1) == 1 && ((insn >> 22) & 3) == 0 && (insn & 0x4000) == 0 {
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
                                                                                    if (hc == 0x0e20_0400 || hc == 0x2e20_0400) && ((insn >> 15) & 1) == 1 && ((insn >> 22) & 3) == 1 && (insn & 0x4000) == 0 {
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
                                            // bit22: 1 => D (f64), 0 => S (f32). `sng` is TRUE for the
                                            // single form (0x7e20), so it reads bit22 CLEARED.
                                            return Inst::ScalarUcvtf { rd, rn, sng: (insn & 0x0040_0000) == 0 };
                                        }
                                        // ---- scalar signed int64->double: scvtf Dd, Dn ----
                                        // Gate (insn & 0xffe0_fc00) == 0x5e60_d800. Sibling of the
                                        // 0x7e60_d800 (unsigned) form; bit23 distinguishes them.
                                        if (insn & 0xffe0_fc00) == 0x5e60_d800 || (insn & 0xffe0_fc00) == 0x5e20_d800 {
                                            let rn = ((insn >> 5) & 0x1f) as u8;
                                            let rd = (insn & 0x1f) as u8;
                                            // bit22: 1 => D (f64), 0 => S (f32). `sng` is TRUE for the
                                            // single form (0x5e20), so it reads bit22 CLEARED (was
                                            // inverted: a double scvtf took the i32->f32 path and
                                            // truncated/corrupted the value).
                                            return Inst::ScalarScvtf { rd, rn, sng: (insn & 0x0040_0000) == 0};
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
                                                                                                                                                                                                                // imm5 (bits[20:16]) marks the element size: for a genuine
                                                                                                                                                                                                                // dup-from-GPR it is a power of two {1,2,4,8} => esize {1,2,4,8}.
                                                                                                                                                                                                                // Guard BEFORE any shift: imm5==0 makes u32::trailing_zeros
                                                                                                                                                                                                                // return 32 and `1u8 << 32` PANICS, aborting the JIT. The decoder
                                                                                                                                                                                                                // must never panic on arbitrary guest bytes, so non-canonical
                                                                                                                                                                                                                // imm5 emits Unsupported (invalid dup) like every other bad encoding.
                                                                                                                                                                                                                let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                match (insn >> 16) & 0x1f {
                                                                                                                                                                                                                    0b00001 => return Inst::SimdDupGp { rd, rn, esize: 1, q },
                                                                                                                                                                                                                    0b00010 => return Inst::SimdDupGp { rd, rn, esize: 2, q },
                                                                                                                                                                                                                    0b00100 => return Inst::SimdDupGp { rd, rn, esize: 4, q },
                                                                                                                                                                                                                    0b01000 => return Inst::SimdDupGp { rd, rn, esize: 8, q },
                                                                                                                                                                                                                    _ => return Inst::Unsupported(insn),
                                                                                                                                                                                                                }
                                                                                                                                                                                                                    }
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
                                                                                                                        // ---- scalar FP fixed-point convert to int: fcvtzs/fcvtzu Rd, Fn, #fbits ----
        // (result = trunc(Fn * 2^fbits)). top16 0x1e18/0x1e58/0x9e18/0x9e58
        // (signed) and the same with bit16 set for unsigned (0x1e19..).
        // fbits = 64 - bits[15:10]. MUST precede the SimdMull gate, which
        // otherwise swallows these (fcvtzs x0,d31,#2 = 0x9e58fbe0 came out as
        // `smull x0, w31, w24`, dropping the *2^fbits scale entirely).
        if {
            let top = insn >> 16;
            (top & 0xfff0) == 0x1e50 || (top & 0xfff0) == 0x9e50
                || (top & 0xfff0) == 0x1e10 || (top & 0xfff0) == 0x9e10
        } {
            let unsigned = (insn >> 16) & 1 == 1; // fcvtzu (0x..9) vs fcvtzs (0x..8)
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            let sz = (insn >> 22) & 1 == 1; // source double (D) vs single (S)
            let fbits = (64u32 - ((insn >> 10) & 0x3f)) as u8;
            return Inst::FcvtToInt {
                rd,
                rn,
                mode: 0, // truncate
                sf: (insn >> 31) & 1 == 1,
                unsigned,
                src_sng: !sz,
                fbits,
            };
        }
        // ---- SIMD FP rounding frint{n,m,p,z,a} Vd.T, Vn.T ----
        // Encodings (rd=0,rn=0, verified vs the aarch64 assembler): frint{n,m,p,z}
        //   .2s 0x0e21_88/98/.., .4s 0x4e21_88/98/, .2d 0x4e61_88/98/…;
        //   frinta .2s 0x2e218800 / .4s 0x6e218800. mode = bit23<<1|bit12
        //   (n=00,m=01,p=10,z=11); frinta sets bit29; es=bit22(1=d,0=s); q=bit30.
        // MUST be decoded BEFORE smull/umull's coarse gate: it only keeps the top
        // nibble + bits15:14 of byte2, so a vector frint (byte2 0x80|0x90 ->
        // bits15:14 = 10) folds into the smlal residue there and was silently
        // miscompiling as a widening multiply.
        {
            const FRINT: &[(u32, u8, u8, bool)] = &[
                // (residue & 0xffff_fc00, mode, esize_bytes, frinta)
                (0x0e21_8800, 0, 4, false), (0x0e21_9800, 1, 4, false),
                (0x0ea1_8800, 2, 4, false), (0x0ea1_9800, 3, 4, false), // 2s
                (0x4e21_8800, 0, 4, false), (0x4e21_9800, 1, 4, false),
                (0x4ea1_8800, 2, 4, false), (0x4ea1_9800, 3, 4, false), // 4s
                (0x4e61_8800, 0, 8, false), (0x4e61_9800, 1, 8, false),
                (0x4ee1_8800, 2, 8, false), (0x4ee1_9800, 3, 8, false), // 2d
                (0x2e21_8800, 4, 4, true),  (0x6e21_8800, 4, 4, true),  // frinta 2s/4s
                (0x6e61_8800, 4, 8, true),                             // frinta 2d
            ];
            let res = insn & 0xffff_fc00;
            if let Some(&(_, mode, esize, _a)) = FRINT.iter().find(|&&(r, _, _, _)| r == res) {
                return Inst::SimdFrint {
                    rd: (insn & 0x1f) as u8,
                    rn: ((insn >> 5) & 0x1f) as u8,
                    mode,
                    esize,
                    q: (insn >> 30) & 1 == 1,
                };
            }
        }
        // ---- SIMD widening multiply smull/umull & smlal/umlal (0x0f00_c000/8000 gate) ----
                                                                                                                                                                                        // acc = which gateway matched: c000 = plain mul, 8000 = accumulate (smlal).
                                                                                                                                                                                        // res_esize = 2 << bits[23:22] (source elem = 2^bits => result = 2x):
                                                                                                                                                                                        //   .8b->.8h res2, .4h->.4s res4, .2s->.2d res8.
                                                                                                                                                                                        // unsigned = bit29 (0x2e/0x6e top nibble -> 1). q = bit30 (upper half).
                                                                                                                                                                                        // (The prior decode used bit22 for res, bit28 for unsigned and bit15 for
                                                                                                                                                                                        // acc -- all three wrong: umull/umlal were treated as signed, umull .8h
                                                                                                                                                                                        // got res=4, and plain smull/umull accumulated. Fixed here.)
                                                                                                                                                                                        {
                                                                                                                                                                                            let mullgt = insn & 0x0f00_c000;
                                                                                                                                                                                            // The top-nibble + bits15:14 residue is coarser than the real opcode:
                                                                                                                                                                                            // byte2's bits[13:11] are always clear for genuine smull/umull/smlal/umlal
                                                                                                                                                                                            // (element size varies in byte2 bits[2:0], e.g. smull .8h 0x...c020 vs
                                                                                                                                                                                            // umull 0x...c340, and acc flips bit14; verified against every width/.2d/
                                                                                                                                                                                            // 2 variant). Vector FRINT (byte2 0x88/0x98) and compare-to-zero fcmlt
                                                                                                                                                                                            // (0xea) both set a bit in 0x38, so requiring <insn&0x3800>==0 keeps them
                                                                                                                                                                                            // from silently decoding as a widening multiply.
                                                                                                                                                                                            if (mullgt == 0x0e00_c000 || mullgt == 0x0e00_8000)
                                                                                                                                                                                                                                                            && (insn & 0x0000_3800) == 0
                                                                                                                                                                                            {
                                                                                                                                                                                                let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                let res_esize: u8 = 2u8 << ((insn >> 22) & 3);
                                                                                                                                                                                                return Inst::SimdMull {
                                                                                                                                                                                                    rd, rn, rm,
                                                                                                                                                                                                    res_esize,
                                                                                                                                                                                                    unsigned: ((insn >> 29) & 1) == 1,
                                                                                                                                                                                                    q: (insn >> 30) & 1 == 1,
                                                                                                                                                                                                    acc: mullgt == 0x0e00_8000,
                                                                                                                                                                                                };
                                                                                                                                                                                            }
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
                                                                                                                                                                                                                                                                                                                                            // ---- SIMD unsigned compare-higher-or-same: cmhs Vd.4S/2S/2D ----
                                                                                                                                                                                                                                                                                                                                            // byte2 0x3c (bit10 set) vs cmhi's 0x34. Gates 0x6ea0_3c00(4S)/0x2ea0_3c00(2S).
                                                                                                                                                                                                                                                                                                                                            let sms = insn & 0xffe0_fc00;
                                                                                                                                                                                                                                                                                                                                            let cmhs_lanes = if sms == 0x6ea0_3c00 { Some(4) } else if sms == 0x2ea0_3c00 { Some(2) } else { None };
                                                                                                                                                                                                                                                                                                                                            if let Some(clanes) = cmhs_lanes {
                                                                                                                                                                                                                                                                                                                                                let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                return Inst::SimdCmhs { rd, rn, rm, lanes: clanes };
                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                if sms == 0x6ee0_3c00 {
                                                                                                                                                                                                                                                                                                                                                    let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                    return Inst::SimdCmhsD { rd, rn, rm };
                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                            // ---- SIMD signed compare-greater (cmgt) 0x4ea0_3400(.4s)/0x0ea0_3400(.2s)/0x4ee0_3400(.2d) ----
                                                                                                                                                                                                                                                                                                                                            let cgt = insn & 0xffe0_fc00;
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x4ea0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:4, dword:false }; }
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x0ea0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:2, dword:false }; }
                                                                                                                                                                                                                                                                                                                                            if cgt == 0x4ee0_3400 { return Inst::SimdCmgt { rd:(insn&0x1f)as u8, rn:((insn>>5)&0x1f)as u8, rm:((insn>>16)&0x1f)as u8, lanes:2, dword:true }; }
                                                                                                                                                                                                                                                                                                                                            // ---- SIMD unzip even: uzp1 Vd.T, Vn.T, Vm.T ----
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                // opcode bits[13:8] = 0x18 (verified non-colliding vs uzp2/zip1/zip2/trn1/trn2).
                                                                                                                                                                                                                                                                                                                                                                                                                // esize = 1 << bits[23:22]; q = bit30. Vd[i] = Vn[2i], Vd[n+i] = Vm[2i].
                                                                                                                                                                                                                                                                                                                                                                                                                // Discriminator: byte1 in the 0x18-family (uzp1) with bit6 clear, byte3-low-nibble
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    // 0x0e and bit28 clear (0x0e/0x4e) — this excludes bit/bif/bsl (byte3 0x6e,
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    // bit28 set, byte1 0x1c) and zip2. The old `(insn>>12)&0xf==1` nibble test was
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    // too broad and swallowed `bit` (byte1 0x1c).
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    if ((insn >> 24) & 0x0f) == 0x0e && ((insn >> 28) & 1) == 0 && (((insn >> 8) & 0xff) & 0xfc) == 0x18 {
                                                                                                                                                                                                                                                                                                                             let esize = (1 << ((insn >> 22) & 0x3)) as u8;
                                                                                                                                                                                                                                                                                                                             let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                             let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                             let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                             let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                                                                                                                             return Inst::SimdUz1 { rd, rn, rm, esize, q };
                                                                                                                                                                                                                                                                                                                         }
                                                                                                                                                                                                                                                                                                                         // ---- SIMD unzip odd: uzp2 Vd.T, Vn.T, Vm.T ---- 
                                                                                                                                                                                                                                                                                                                         // opcode bits[13:8] = 0x58 (uzp1 was 0x18). Vd[i]=Vn[2i+1], Vd[n/2+i]=Vm[2i+1]
                                                                                                                                                                                                                                                                                                                         // (the odd/upper elements) — gcc's magic-division reducer uses uzp2 to
                                                                                                                                                                                                                                                                                                                                                                                          // gather the HIGH 32-bit product words before the sshr quotient step.
                                                                                                                                                                                                                                                                                                                                                                                          // Must not collide: rev64 now excludes bits[13:12]!=00; this requires
                                                                                                                                                                                                                                                                                                                                                                                          // byte1 in the 0x58-family (bit6 set) + byte3-low-nibble 0x0e + bit28 clear.
                                                                                                                                                                                                                                                                                                                                                                                          if ((insn >> 24) & 0x0f) == 0x0e && ((insn >> 28) & 1) == 0 && (((insn >> 8) & 0xff) & 0xfc) == 0x58 {
                                                                                                                                                                                                                                                                                                                                                                                                                                                      let esize = (1 << ((insn >> 22) & 0x3)) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                      let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                      let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                      let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                      let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                                                                                                                                                                                                                                                      return Inst::SimdUz2 { rd, rn, rm, esize, q };
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
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          // ---- SIMD zip2: zip2 Vd.T, Vn.T, Vm.T (mask 0x3f20_fc00 == 0x0e00_7800) ----
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          // Upper-half interleave: d[2k]=Vn[n/2+k], d[2k+1]=Vm[n/2+k]. Distinct
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          // from trn2 (0x0e00_6800) and zip1 (0x0e00_3800). gcc uses zip1/zip2
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          // with a zero lane to sign/zero-extend a 4s quotient into 4 u64.
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          if insn & 0x3f20_fc00 == 0x0e00_7800 {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              let esize = (1 << ((insn >> 22) & 0x3)) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              let rm = ((insn >> 16) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              let q = (insn >> 30) & 1 == 1;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                              return Inst::SimdZip2 { rd, rn, rm, esize, q };
                                                                                                                                                                                                                                                                                                                                                                                                                                                                          }
                                                                                                                                                                                                                                                                                                                                                                   // ---- SIMD element extract to GPR: umov/smov Rd, Vn.bits[idx] ----
                                                                                                                                                                                                             // Gate &0xbfe0_fc00: 0x0e00_3c00 (umov, widen-0) / 0x0e00_2c00 (smov, widen-s).
                                                                                                                                                                                                             // esize = 1<<tz(imm5), index = imm5>>(tz+1); is_x = bit30.
                                                                                                                                                                                                             let ge = insn & 0xbfe0_fc00;
                                                                                                                                                                                                                                                                                                                                                                                                                              if ge == 0x0e00_3c00 || ge == 0x0e00_2c00 {
                                                                                                                                                                                                                                                                                                                                                                                                                                  let signed = ge == 0x0e00_2c00;
                                                                                                                                                                                                                                                                                                                                                                                                                                  let is_x = ((insn >> 30) & 0x1) == 1;
                                                                                                                                                                                                                                                                                                                                                                                                                                  let imm5 = (insn >> 16) & 0x1f;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // Guard BEFORE any shift: imm5==0 makes u32::trailing_zeros
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // return 32 and `1 << 32` PANICS, aborting the whole JIT. umov/
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // smov with a zero element-size field is a reserved encoding.
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // NOTE: imm5 packs BOTH esize and the lane index, so it is not
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // bounded to a power of two (e.g. s[1] -> imm5=0b01100); only the
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       // exact-zero case is illegal.
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       if imm5 == 0 {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           return Inst::Unsupported(insn);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       }
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
                                                                                                                                                                                                                                                                                                                    && ((b2s & 0xf8) == 0x48
                                                                                                                                                                                                                                                                                                                        || ((b2s & 0xf8) == 0x28 && (insn & 0x10000) != 0))
                                                                                                                                                                                                                                                                                                                    && !(b0s == 0x0e && (b2s & 0xf8) == 0x28)
                                                                                                                                                                                                                                                                                                                {
                                                                                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                    // dst_esize from byte1 top-nibble: 0x2 -> 8b dst(1), 0x6 -> 4h dst(2), 0xa -> 2s dst(4)
                                                                                                                                                                                                                                                                                                        let b1 = ((insn >> 16) & 0xff) as u32;
                                                                                                                                                                                                                                                                                                        let dst_esize: u8 = 1u8 << (((b1 >> 4) & 0xf) >> 2).min(2); // nibble 0x2->1,0x6->2,0xa->4
                                                                                                                                                                                                                                                                                    let b1q = (b0s >> 1) & 1; // byte0 bit1 == sq (sign dst) family bit
                                                                                                                                                                                                                                                                                                        // src_signed vs dst_signed from the U bit (bit29) and byte2:
                                                                                                                                                                                                                                                                                                        //   sqxtn  0x0e, byte2 0x48 : dst signed,  src signed
                                                                                                                                                                                                                                                                                                        //   uqxtn  0x2e, byte2 0x48 : dst unsigned,src unsigned
                                                                                                                                                                                                                                                                                                        //   sqxtun 0x2e, byte2 0x28 : dst unsigned,src signed
                                                                                                                                                                                                                                                                                                        let ubit = (b0s >> 5) & 1; // bit29 set -> unsigned-dst family (0x2e/0x6e)
                                                                                                                                                                                                                                                                                                        let dst_signed = ubit == 0;
                                                                                                                                                                                                                                                                                                        let src_signed = if (b2s & 0xf8) == 0x28 { true } else { ubit == 0 };
                                                                                                                                                                                                                                                                                                        return Inst::SaturatNarrow {
                                                                                                                                                                                                                                                                                                            rd, rn, dst_esize,
                                                                                                                                                                                                                                                                                                            src_signed,
                                                                                                                                                                                                                                                                                                            dst_signed,
                                                                                                                                                                                                                                                                                                            q: (insn >> 30) & 1 == 1,
                                                                                                                                                                                                                                                                                                        };
                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                // addhn/subhn/raddhn: dst = high half of src-width sum/diff, narrowed.
                                                                                                                                                                                                                                                                                // byte2 {0x40(hn-add),0x60(hn-sub)}; prefix 0x0e/0x2e; dst_esize=1<<bits[23:22].
                                                                                                                                                                                                                                                                                let hb0 = (insn >> 24) & 0xff;
                                                                                                                                                                                                                                                                                                let hb2 = (insn >> 8) & 0xff;
                                                                                                                                                                                                                                                                                                if (hb0 == 0x0e || hb0 == 0x2e) && ((hb2 & 0xe0) == 0x40 || (hb2 & 0xe0) == 0x60)
                                                                                                                                                                                                                                                                                                                    && (insn & 0x400) == 0 { // bit10=0 marks three-different (addhn/subhn); 3-same min/max set it
                                                                                                                                                                                                                                                                                                                    let dst_esize: u8 = 1 << ((insn >> 22) & 3);
                                                                                                                                                                                                                                                                                                                    return Inst::SimdHighNarrow {
                                                                                                                                                                                                                                                                                                                        rd: (insn & 0x1f) as u8,
                                                                                                                                                                                                                                                                                                                        rn: ((insn >> 5) & 0x1f) as u8,
                                                                                                                                                                                                                                                                                                                        rm: ((insn >> 16) & 0x1f) as u8,
                                                                                                                                                                                                                                                                                                                        dst_esize,
                                                                                                                                                                                                                                                                                                                        sub: (insn & 0x2000) != 0, // byte2 0x40=add,0x60=sub (this IS bit13)
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
                                                                                                                                                                                                                                                                                                                                                                                        // INS Vd.T[dst], Vn.T[src] packs BOTH lane indices into the word:
                                                                                                                                                                                                                                                                                                                                                                                        //   imm5 low `log2(esize)` bits + 1 encode escale via trailing zeros;
                                                                                                                                                                                                                                                                                                                                                                                        //   dst index = imm5 >> (log2(esize)+1);
                                                                                                                                                                                                                                                                                                                                                                                        //   src index = a contiguous field at bits[14:15-l]), l=log2(esize),
                                                                                                                                                                                                                                                                                                                                                                                        //   width 4-l: (insn >> (11+l)) & ((1 << (4-l))-1).
                                                                                                                                                                                                                                                                                                                                                                                        // (Verified against the aarch64 assembler across all 4x4 S, 8x8 H,
                                                                                                                                                                                                                                                                                                                                                                                        // 16x16 B and 2x2 D lane pairs — the old single-bit bit20/bit14 read
                                                                                                                                                                                                                                                                                                                                                                                        // only worked for lane 0/1 S and silently corrupted every other case.)
                                                                                                                                                                                                                                                                                                                                                                                        let l = esize.ilog2();
                                                                                                                                                                                                                                                                                                                                                                                        let dst_idx = (f >> (l + 1)) as u8;
                                                                                                                                                                                                                                                                                                                                                                                        let src_idx = ((insn >> (11 + l)) & ((1u32 << (4 - l)) - 1)) as u8;
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
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        // Guard BEFORE the shift: f==0 makes `f & f.wrapping_neg()` == 0,
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        // whose trailing_zeros() is 32 -> `1 << 32` PANICS, aborting the whole
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        // JIT. dup with a zero element-size field is reserved. NOTE: f packs
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        // BOTH esize and the src lane index (e.g. s[1] -> f=0b01100), so only
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        // the exact-zero case is illegal, not f>8.
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        if (f & f.wrapping_neg()) == 0 {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            return Inst::Unsupported(insn);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let esize = (1 << ((f & f.wrapping_neg()).trailing_zeros())) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let src_idx = (f >> (esize.ilog2() + 1)) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let rn = ((insn >> 5) & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let rd = (insn & 0x1f) as u8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let q = ((insn >> 30) & 1) == 1;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    return Inst::SimDup { rd, rn, esize, src_idx, q };
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                    // ---- SIMD bitwise insert: bit/bif Vd.16B,Vn.16B,Vm.16B (16B) & Vd.8B (8B) ----
        // Gates: (insn & 0xffe0_fc00) in {0x6ea01c00 BIT16B, 0x6ee01c00 BIF16B,
        // 0x2ea01c00 BIT8B, 0x2ee01c00 BIF8B} (real words 0x6ea11c40 etc).
        // Disjoint from orr16 (0x4ea01c00, bit31) and cmhi (0x6ea03400).
        // bif = bit22 (0x0040_0000) SET (insert-if-FALSE; bit and bif words differ by bit22). The SimdSel gate above
        // requires bit23 CLEAR so only BSL selects there; BIT/BIF both fall here.
        if matches!(insn & 0xffe0_fc00, 0x6ea0_1c00 | 0x6ee0_1c00 | 0x2ea0_1c00 | 0x2ee0_1c00) {
            let rm = ((insn >> 16) & 0x1f) as u8;
            let rn = ((insn >> 5) & 0x1f) as u8;
            let rd = (insn & 0x1f) as u8;
            return Inst::SimdBit { rd, rn, rm, bif: (insn & 0x0040_0000) != 0 };
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
        // mrs xN, dczid_el0 = 0xd53b00e0: op1=3, CRn=0, CRm=0, op2=7. The data-
        // cache-zeroing block-size register. BS field (bits[3:0]) = log2(block
        // size) in bytes; bit4 (DZP) = 1 means DC ZVA is prohibited. GLIBC's CRT
        // reads this to size its DC ZVA memset streaming; return a sane 16-byte
        // block (0x4), DZP=0. Verified encoding against objdump of modmain.elf.
        if op1 == 3 && crn == 0 && crm == 0 && op2 == 7 && read {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 5, rt, read }; // dczid_el0 -> 0x4
        }
        // mrs xN, gcspr_el0 = 0xd53b2522: op1=3, CRn=2, CRm=5, op2=1. The
        // Guarded-Control-Stack pointer (armv9 GCS feature). The JIT runs no
        // GCS and never sets the SCTLR_EL1.GCS enable, so a read returns 0 (the
        // GCS stack pointer is only valid when GCS is enabled). Freshly-armed
        // GNU aarch64 toolchains emit this MRS sizing a GCS call frame; without
        // it a full glibc-linked program stops. Verified against objdump of the
        // real modmain.elf word 0xd53b2522.
        if op1 == 3 && crn == 2 && crm == 5 && op2 == 1 {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 6, rt, read }; // gcspr_el0 -> 0
        }
        // mrs xN, tpidr2_el0 = 0xd53bd0ae: op1=3, CRn=13, CRm=0, op2=5. The
        // second thread-local storage pointer (SME feature). Distinct from the
        // tpidr_el0 we model (op2=2). Without SME the register is architecturally
        // 0 on a fresh EL0 context, so read 0. Glibc CRT reads it probing SME
        // support. Verified against the real modmain.elf word 0xd53bd0ae.
        if op1 == 3 && crn == 13 && crm == 0 && op2 == 5 {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 7, rt, read }; // tpidr2_el0 -> 0
        }
        // mrs xN, midr_el1 = 0xd5380000|rt: op1=0, CRn=0, CRm=0, op2=0. The
        // Main ID Register (implementer/part). glibc's __libc_cpu_features
        // reads it to name a known core for memcpy/IFUNC tuning. Return 0
        // ("unknown" implementer) so glibc picks generic non-SME/SVE paths.
        if (insn & 0xffff_f01f) == 0xd538_0000 && read {
            let rt = (insn & 0x1f) as u8;
            return Inst::SysReg { sysreg: 8, rt, read }; // midr_el1 -> 0
        }
    }

    // ---- load/store pair (X: 0xa8/0xa9, W: 0x28/0x29, SIMD Q 128-bit: 0xAD, FP/vec d: 0x6d/0x2d) ----
    if matches!(insn >> 24, 0x29 | 0x69 | 0x28 | 0xa9 | 0xa8 | 0xac | 0xad | 0x6d | 0x2d | 0x6c | 0x2c) {
        let q128 = (insn >> 24) & 0xff == 0xad || (insn >> 24) & 0xff == 0xac; // 128-bit SIMD pair (ldp/stp q)
        // FP/vector pairs: bit30 == 1 => 64-bit d-pair (0x6d/0x6c), bit30 == 0
        // => 32-bit s-pair (0x2d/0x2c). BUGFIX (Session 99): 0x2c/0x2d were
        // lumped into fp_d (scale 8), so `ldp s29,s28` read 8 bytes per reg and
        // post-indexed by 2x — fstruct.elf (-O2 float struct walk) returned
        // garbage. Splitting gives the 32-bit s-pair scale 4 / 4-byte transfer.
        let fp_d = (insn >> 24) & 0xff == 0x6d || (insn >> 24) & 0xff == 0x6c;
        let fp_s = (insn >> 24) & 0xff == 0x2d || (insn >> 24) & 0xff == 0x2c;
        let sext_en = (insn >> 24) & 0xff == 0x69; // ldpsw: sign-ext the 32-bit pair to 64-bit
        let size_64 = insn >> 31 == 1; // sf  (Q pair ignores this for reg scale)
        let ld = (insn >> 22) & 1 == 1; // L: 1=ldp, 0=stp
        let indexed = (insn >> 23) & 1 == 1; // 0=offset, 1=indexed (pre/post)
        let preidx = indexed && (insn >> 24) & 1 == 1; // pre if bit24=1 within indexed
        let scale = if q128 {
            16
        } else if fp_d {
            8 // d-pairs are 64-bit FP/vector registers
        } else if fp_s {
            4 // s-pairs are 32-bit FP/vector registers
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
            fp_s,
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

    // ---- memory-tagging (MTE) allocation-tag ops: ldg/stg/stzg/st2g (0xd9 top
    // byte). Host has no MTE and the JIT keeps no tag state: stores are pure
    // no-ops (memory untouched), OR a base-reg writeback when the addressing
    // mode auto-increments Xn; ldg (load, bit22=1 && bit11=0) reads tag 0 into
    // Xt. Unblocks glibc's __libc_mtag_tag_region (stg loop) and _memset-zva.
    // Mask 0xff200000 ignores Xt/Xn/imm9 and the writeback bit (bit10) so both
    // offset and post/pre-index forms are caught (verified across
    // stg/stzg/ldg/st2g with varied regs, offsets and writebacks). load bit =
    // bit22 (0x400000): ldg byte1=0x60->set; store stg/st2g 0x20/0xa0->clear.
    // bit11 (0x0800) additionally separates the offset (set) / untagged-load
    // base forms; ldg has bit11=0.
    if insn & 0xff20_0000 == 0xd920_0000 {
        let rt = (insn & 0x1f) as u8;
        let rn = ((insn >> 5) & 0x1f) as u8;
        let load = (insn & 0x0040_0000) != 0 && (insn & 0x0800) == 0;
        // writeback forms (post-index, bit11=0 / pre-index, bit11=1) set bit10;
        // both update Xn by the (signed, granule-scaled) immediate. imm9 =
        // bits[20:12], scale = 4 bits (16-byte alloc granule), sign-extend.
        let wb = (insn & 0x0400) != 0;
        let imm9 = (insn >> 12) & 0x1ff;
        let imm9 = if imm9 & 0x100 != 0 { imm9 as i64 - 0x200 } else { imm9 as i64 };
        let wb_off = imm9 << 4;
        return Inst::MteTag { load, rt, rn, wb, wb_off };
    }

    // ---- data/instruction cache maintenance: dc <op>, xN / ic <op>, xN ----
    // System-prefix 0xd5, CRn=7. In the single-threaded, direct-mapped JIT
    // (guest==host, warm shared memory, no separate cache) a cache clean/
    // invalidate/coherence op has no observable effect → no-op. The one that
    // WRITES memory is `dc zva` (zero the cache line at [Xn]); we advertise a
    // 16-byte block via dczid_el0 so zero 16 bytes. RT is the address register.
    // Verified vs objdump (armv8.5-a+memtag): dc zva x4=0xd50b7424 (CRm=4,
    // op2=1), dc gva=0xd50b7462 (op2=3), civac=0xd50b7e20 — only zva zeroes.
    if (insn >> 24) & 0xff == 0xd5 && ((insn >> 12) & 0xf) == 7 {
        let rt = (insn & 0x1f) as u8;
        let zva = ((insn >> 8) & 0xf) == 4 && ((insn >> 5) & 0x7) == 1;
        return Inst::CacheMaintain { zva, rt };
    }

    Inst::Unsupported(insn)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_long_family_ground_truth() {
        // Verified vs aarch64-linux-gnu-gcc $g: the smull/umull/maddl/msubl
        // 32x32->64 family (real encodings assembled and objdump-read).
        let cases = [
            (0x9ba77c61u32, false, false, 3u8, 7u8, 1u8, 31u8), // umull  x1,w3,w7
            (0x9b267ca4u32, false, true, 5u8, 6u8, 4u8, 31u8),  // smull  x4,w5,w6
            (0x9ba41462u32, false, false, 3u8, 4u8, 2u8, 5u8),  // umaddl x2,w3,w4,x5
            (0x9b2824e6u32, false, true, 7u8, 8u8, 6u8, 9u8),   // smaddl x6,w7,w8,x9
            (0x9bacb56au32, true, false, 11u8, 12u8, 10u8, 13u8), // umsubl x10,w11,w12,x13
            (0x9b30c5eeu32, true, true, 15u8, 16u8, 14u8, 17u8),  // smsubl x14,w15,w16,x17
        ];
        for (w, sub, signed, rn, rm, rd, ra) in cases {
            match decode(w) {
                Inst::MulLong {
                    rd: d,
                    rn: n,
                    rm: m,
                    ra: a,
                    signed: s,
                    sub: subb,
                } => {
                    assert_eq!((d, n, m, a, s, subb), (rd, rn, rm, ra, signed, sub), "{w:#x}");
                }
                other => panic!("expected MulLong for {w:#x}, got {other:?}"),
            }
        }
        // umulh/smulh must NOT decode as MulLong (they're MulHigh, bit22 set).
        assert!(matches!(decode(0x9bc57c83), Inst::MulHigh { .. }));
        assert!(matches!(decode(0x9b487ce6), Inst::MulHigh { .. }));
    }

    #[test]
    fn simd_frint_and_cmpzero_not_swallowed_by_widen_mul() {
        // The coarse SimdMull gate (insn & 0x0f00_c000, which drops bit28 and
        // only keeps byte2 bits15:14) used to fold vector FRINT (byte2 0x98/0x88)
        // and compare-to-zero fcmlt (byte2 0xea) into the smlal residue — a
        // silent widening multiply instead of a rounding / compare. Guard:
        //  (a) vector frintm/frintz .4s decode to SimdFrint (never SimdMull),
        //  (b) fcmlt .4s #0.0 decodes to VecFpCmpZero (never SimdMull),
        //  (c) genuine umull (byte2 0xc3, bits13:11 clear) still decodes Smull2D.
        // (a): from real gcc -O3 floor loop: frintm v1.4s, v1.4s
        assert!(matches!(decode(0x4e219821), Inst::SimdFrint { mode: 1, esize: 4, q: true, .. }));
        assert!(matches!(decode(0x4ea19821), Inst::SimdFrint { mode: 3, .. })); // frintz .4s
        // (b): from gcc select: fcmlt v6.4s, v16.4s, #0.0
        assert!(matches!(decode(0x4ea0ea06), Inst::VecFpCmpZero { op: 3, esize: 4, .. }));
        // (c): real umull (magic-div BFS, byte2 0xc3): still a widening mul (.2d)
        assert!(matches!(decode(0x2ebbc340), Inst::SimdMull { res_esize: 8, acc: false, .. }));
        // and genuine smlal (byte2 0x80) still accumulates.
        assert!(matches!(decode(0x0e628020), Inst::SimdMull { acc: true, .. }));
    }

    #[test]
    fn addvl_cntd_sme_midr_ground_truth() {
        // addvl/addsvl (SVE vector-length add: Xd = Xn + imm6*VL, VL=16B).
        for (w, rd, rn, bytes) in [
            (0x04205020u32, 0u8, 0u8, 1i32 * 16),   // addvl x0,x0,#1
            (0x04205040u32, 0u8, 0u8, 2i32 * 16),   // addvl x0,x0,#2
            (0x04205200u32, 0u8, 0u8, 16i32 * 16),  // addvl x0,x0,#16
            (0x042050a9u32, 9u8, 0u8, 5i32 * 16),   // addvl x9,x0,#5
            (0x04225041u32, 1u8, 2u8, 2i32 * 16),   // addvl x1,x2,#2
            (0x042857a0u32, 0u8, 8u8, -3i32 * 16),  // addvl x0,x8,#-3
            (0x04305a10u32, 16u8, 16u8, 16i32 * 16), // addsvl x16,x16,#16
        ] {
            match decode(w) {
                Inst::AddVectorLen {
                    rd: d,
                    rn: n,
                    imm_bytes: b,
                } => assert_eq!((d, n, b), (rd, rn, bytes), "{w:#x}"),
                other => panic!("expected AddVectorLen for {w:#x}, got {other:?}"),
            }
        }
        // cntd (count 64-bit elements): rd=6 for x6, value comes from translate.
        match decode(0x04e0e3e6) {
            Inst::SveCntd { rd } => assert_eq!(rd, 6),
            other => panic!("expected SveCntd, got {other:?}"),
        }
        // SME str za[w15,k],[x16,#k,mul vl] — no-op class, 0xe1206200..0xf.
        for w in 0xe1206200u32..=0xe120620f {
            match decode(w) {
                Inst::SmeNoop => {}
                other => panic!("expected SmeNoop for {w:#x}, got {other:?}"),
            }
        }
        // smstart/smstop (plain + za variants) — SmeNoop.
        for w in [0xd503467f, 0xd503477f, 0xd503447f, 0xd503457f] {
            match decode(w) {
                Inst::SmeNoop => {}
                other => panic!("expected SmeNoop for smstart/smstop {w:#x}, got {other:?}"),
            }
        }
        // mrs x0, midr_el1 = 0xd5380000 — SysReg sysreg 8 (reads 0).
        match decode(0xd5380000) {
            Inst::SysReg { sysreg: s, rt, read } => {
                assert_eq!(s, 8);
                assert_eq!(rt, 0);
                assert!(read);
            }
            other => panic!("expected SysReg midr_el1, got {other:?}"),
        }
    }

    #[test]
    fn addsubext_and_postindex_ldr_ground_truth() {
        // add x3, x2, w20, sxtw #3 (extended-register form) — MUST decode as
        // AddSubExt (not the shifted AddSubReg which mis-read it as lsl#51).
        match decode(0x8b34cc43) {
            Inst::AddSubExt {
                rd, rn, rm, opt, shift, sub, sf, ..
            } => {
                assert_eq!((rd, rn, rm), (3, 2, 20));
                assert_eq!(opt, 6); // SXTW
                assert_eq!(shift, 3);
                assert!(!sub);
                assert!(sf);
            }
            other => panic!("expected AddSubExt for 0x8b34cc43, got {other:?}"),
        }
        // sub sp, sp, x1 (extended UXTSX form) — bit21=1 but no ext -> AddSubExt
        // opt=3(UXTX) shift=0, rn=rd=31 (SP).
        match decode(0xcb2163ff) {
            Inst::AddSubExt { rd, rn, rm, opt, shift, sub, .. } => {
                assert_eq!((rd, rn), (31, 31));
                assert_eq!(rm, 1);
                assert_eq!(opt, 3);
                assert_eq!(shift, 0);
                assert!(sub);
            }
            other => panic!("expected AddSubExt for sub sp,sp,x1, got {other:?}"),
        }
        // post-index ldr x3,[x0],#8 = 0xf8408403 -> LdStrImmWb (writeback, post)
        match decode(0xf8408403) {
            Inst::LdStrImmWb { rt, rn, imm9, size, ld, writeback, pre, .. } => {
                assert_eq!((rt, rn), (3, 0));
                assert_eq!(imm9, 8);
                assert_eq!(size, 8);
                assert!(ld);
                assert!(writeback);
                assert!(!pre);
            }
            other => panic!("expected LdStrImmWb post for 0xf8408403, got {other:?}"),
        }
        // pre-index ldrh w3,[x0,#4]! = 0x78404c03 -> LdStrImmWb (writeback, pre)
        match decode(0x78404c03) {
            Inst::LdStrImmWb { rt, imm9, size, writeback, pre, .. } => {
                assert_eq!((rt, imm9, size), (3, 4, 2));
                assert!(writeback);
                assert!(pre);
            }
            other => panic!("expected LdStrImmWb pre for 0x78404c03, got {other:?}"),
        }
        // register-offset ldr x3,[x0,x2] = 0xf8626803 MUST stay LdStrReg.
        assert!(matches!(decode(0xf8626803), Inst::LdStrReg { .. }));
    }

    #[test]
    fn fp_scalar_unscaled_and_wb_ground_truth() {
        // Scalars (B/H/S/D) LDUR/STUR (unscaled) + LDR/STR pre/post-index
        // writeback, bit26=1 vector file. Words taken from aarch64-linux-gnu-as
        // objdump (`ldur/stur` + `ldr/str ... [x1,#imm]!` / `[x1],#imm`).
        for (w, _kind, size, ld, pre, imm9) in [
            // (word, kind, size, ld, pre, imm9)
            (0x3c5fb020u32, "unsc", 1u8, true, false, -5i32), // ldur b0,[x1,#-5]
            (0x7c407020u32, "unsc", 2u8, true, false, 7i32),  // ldur h0,[x1,#7]
            (0xbc5fc020u32, "unsc", 4u8, true, false, -4i32), // ldur s0,[x1,#-4]
            (0xfc408020u32, "unsc", 8u8, true, false, 8i32),  // ldur d0,[x1,#8]
            (0x3c1fb020u32, "unsc", 1u8, false, false, -5i32), // stur b0,[x1,#-5]
            (0x7c007020u32, "unsc", 2u8, false, false, 7i32), // stur h0,[x1,#7]
            (0xbc1fc020u32, "unsc", 4u8, false, false, -4i32), // stur s0,[x1,#-4]
            (0xfc008020u32, "unsc", 8u8, false, false, 8i32), // stur d0,[x1,#8]
            (0xbc404420u32, "wb", 4u8, true, false, 4i32),   // ldr s0,[x1],#4 post
            (0xbc404c20u32, "wb", 4u8, true, true, 4i32),    // ldr s0,[x1,#4]! pre
            (0xfc5f8420u32, "wb", 8u8, true, false, -8i32),  // ldr d0,[x1],#-8 post
            (0xfc5f8c20u32, "wb", 8u8, true, true, -8i32),   // ldr d0,[x1,#-8]! pre
            (0xbc004420u32, "wb", 4u8, false, false, 4i32),  // str s0,[x1],#4 post
            (0xfc1f8420u32, "wb", 8u8, false, false, -8i32), // str d0,[x1],#-8 post
        ] {
            match decode(w) {
                Inst::FpLdStImmUnscaled {
                    vt,
                    rn,
                    imm9: i,
                    size: s,
                    ld: l,
                } => {
                    assert_eq!((vt, rn), (0, 1), "{w:#x}");
                    assert_eq!(i, imm9, "{w:#x}");
                    assert_eq!(s, size, "{w:#x}");
                    assert_eq!(l, ld, "{w:#x}");
                }
                Inst::FpLdStImmWb {
                    vt,
                    rn,
                    imm9: i,
                    size: s,
                    ld: l,
                    pre: p,
                } => {
                    assert_eq!((vt, rn), (0, 1), "{w:#x}");
                    assert_eq!(i, imm9, "{w:#x}");
                    assert_eq!(s, size, "{w:#x}");
                    assert_eq!(l, ld, "{w:#x}");
                    assert_eq!(p, pre, "{w:#x}");
                }
                other => panic!("expected scalar FP ld/st for {w:#x}, got {other:?}"),
            }
        }
        // Neighbours must NOT decode into the scalar-immediate family:
        // scaled scalar (ldr d0,[x1] = 0xfd400020) stays FpLdStImm;
        // scalar register-offset (ldr s3,[x1,x2,lsl#2] = 0xbc627823) stays
        // FpLdStrReg; 128-bit q original (ldur q0 = 0x3cc08020) stays
        // VecLdStImmUnscaled; GPR (ldur x0,[x1,#-8] = 0xf85f8020) is LdStrImmWb.
        assert!(matches!(decode(0xfd400020), Inst::FpLdStImm { .. }), "scaled d");
        assert!(matches!(decode(0xbc627823), Inst::FpLdStrReg { .. }), "reg-offset s");
        assert!(
            matches!(decode(0x3cc08020), Inst::VecLdStImmUnscaled { .. }),
            "q unscaled"
        );
        assert!(matches!(decode(0xf85f8020), Inst::LdStrImmWb { .. }), "gpr");
    }

    #[test]
    fn lse_atomic_ground_truth() {
        // Real modmain.elf LSE atomics: op/bits/size/regs must decode.
        // (have_lse=0 in this guest so glibc uses the ldxr/stxr fallback; these
        // are decoded so the compile-time CFG past the IFUNC can build.)
        for (w, op, size64, rt, rn, rs) in [
            (0xb8200020u32, 0u8, false, 0u8, 1u8, 0u8), // ldadd w0,w0,[x1]
            (0xb8208020u32, 4u8, false, 0u8, 1u8, 0u8), // swp   w0,w0,[x1]
            (0xb8a00020u32, 0u8, false, 0u8, 1u8, 0u8), // ldadda
            (0xb8a01020u32, 1u8, false, 0u8, 1u8, 0u8), // ldclra
            (0xb8a03020u32, 3u8, false, 0u8, 1u8, 0u8), // ldseta
            (0xf8602020u32, 2u8, true, 0u8, 1u8, 0u8),  // ldeorl x0,x0,[x1]
            (0xf8a08020u32, 4u8, true, 0u8, 1u8, 0u8),  // swpa  x0,x0,[x1]
            (0xf8e08020u32, 4u8, true, 0u8, 1u8, 0u8),  // swpal
        ] {
            match decode(w) {
                Inst::LseAtomic {
                    op: o, size64: s, rt: t, rn: n, rs: q,
                } => {
                    assert_eq!((o, s, t, n, q), (op, size64, rt, rn, rs), "{w:#x}");
                }
                other => panic!("expected LseAtomic for {w:#x}, got {other:?}"),
            }
        }
        // CAS and register-offset LDR must NOT decode as LSE.
        assert!(matches!(decode(0xf8626803), Inst::LdStrReg { .. }));
        assert!(!matches!(decode(0x88a07c41), Inst::LseAtomic { .. }));
        assert!(!matches!(decode(0xc8e0fc41), Inst::LseAtomic { .. }));
    }

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
                fp_s,
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
                fp_s,
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
                sp_operand,
            } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 0);
                assert_eq!(rm, 0);
                assert!(!sub);
                assert!(!sf);
                assert!(!s);
                assert_eq!(shift, ShiftKind::Lsl);
                assert_eq!(sh_amt, 1);
                assert!(!sp_operand, "0x0b add shifted-reg has bit21=0");
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
        // CRITICAL discriminator: `addp d31, v31.2d` = 0x5ef1bbff differs from the
        // fcvtzs above ONLY by bit20 (SET). It must decode to SimdPairAddD, NOT
        // FcvVec — the OLD gate silently converted gcc pairwise-add reductions to
        // float->int (wrong results, not a trap).
        match decode(0x5ef1bbff) {
            Inst::SimdPairAddD { rd, rn, .. } => {
                assert_eq!((rd, rn), (31, 31));
            }
            other => panic!("addp d31,v31.2d -> SimdPairAddD, got {other:?}"),
        }
        // The vector 3-same ADDP (0x4ee2bc20 = addp v0.2d,v1,v2) stays untouched
        // by this scalar gate (different residue) — could be Unsupported or SIMD;
        // just ensure it does NOT silently become the scalar FcvVec.
        assert!(!matches!(decode(0x4ee2bc20), Inst::FcvVec { .. }));
        // saddw/saddw2 (SIMD add-wide) share the SimdAddw gate; the Q bit must
        // route the source to the UPPER half of Vm for saddw2 — the OLD code
        // always read the lower half, so -O2 vectorized loops accumulated the
        // wrong lanes (e.g. the i*i reduction gave 30 instead of 76).
        match decode(0x0ea21020) { // saddw v0.2d, v1.2d, v2.2s  (Q=0, lower)
            Inst::SimdAddw { rd, rn, rm, esrc, upper, .. } => {
                assert_eq!((rd, rn, rm, esrc, upper), (0, 1, 2, 4, false));
            }
            other => panic!("saddw v0.2d -> SimdAddw, got {other:?}"),
        }
        match decode(0x4ea51083) { // saddw2 v3.2d, v4.2d, v5.4s  (Q=1, upper)
            Inst::SimdAddw { rd, rn, rm, esrc, upper, .. } => {
                assert_eq!((rd, rn, rm, esrc, upper), (3, 4, 5, 4, true));
            }
            other => panic!("saddw2 v3.2d -> SimdAddw upper, got {other:?}"),
        }
        match decode(0x6eab1149) { // uaddw2 v9.2d, v10.2d, v11.4s (Q=1, upper)
            Inst::SimdAddw { rd, rn, rm, sign, upper, .. } => {
                assert_eq!((rd, rn, rm, sign, upper), (9, 10, 11, false, true));
            }
            other => panic!("uaddw2 v9.2d -> SimdAddw upper unsigned, got {other:?}"),
        }
        match decode(0x7ee1b8e0) {
            Inst::FcvVec { rd, signed, .. } => {
                assert_eq!((rd, signed), (0, false)); // fcvtzu -> unsigned
            }
            other => panic!("fcvtzu d0,d7 -> FcvVec unsigned, got {other:?}"),
        }
    }
    #[test]
    fn vector_neg_abs_decode_not_fcvt_family() {
        // Regression: integer vector NEG/ABS (two-reg misc, opcode 0xb, bit16
        // CLEAR) must NOT be swallowed by the vector float->int FcvVec gate,
        // whose mask 0xffe0_fc00 zeroes bit16 and so matched `neg v29.2s` too —
        // decoding NEG as fcvtzu silently zeroed/corrupted lanes.
        //   neg v29.2s, v31.2s = 0x2ea0bbfd (the runtime repro)
        match decode(0x2ea0bbfd) {
            Inst::SimdArithUnary { rd, rn, esize, q, op } => {
                assert_eq!((rd, rn, esize, q, op), (29, 31, 4, false, 0));
            }
            other => panic!("neg v29.2s,v31.2s -> SimdArithUnary{{op:0}}, got {other:?}"),
        }
        // abs v2.2s, v1.2s
        match decode(0x0ea0b822) {
            Inst::SimdArithUnary { rd, rn, esize, q, op } => {
                assert_eq!((rd, rn, esize, q, op), (2, 1, 4, false, 1));
            }
            other => panic!("abs v2.2s,v1.2s -> SimdArithUnary{{op:1}}, got {other:?}"),
        }
        // neg v0.2d, v1.2d (D width, Q=1)
        match decode(0x6ee0b820) {
            Inst::SimdArithUnary { rd, rn, esize, q, op } => {
                assert_eq!((rd, rn, esize, q, op), (0, 1, 8, true, 0));
            }
            other => panic!("neg v0.2d,v1.2d -> SimdArithUnary esize8, got {other:?}"),
        }
        // neg v0.4h, v1.4h (H width, Q=0)
        match decode(0x2e60b820) {
            Inst::SimdArithUnary { rd, rn, esize, q, op } => {
                assert_eq!((rd, rn, esize, q, op), (0, 1, 2, false, 0));
            }
            other => panic!("neg v0.4h,v1.4h -> SimdArithUnary esize2, got {other:?}"),
        }
        // neg v0.4s, v1.4s (S width, Q=1) — the exec-test width
        match decode(0x6ea0b820) {
            Inst::SimdArithUnary { rd, rn, esize, q, op } => {
                assert_eq!((rd, rn, esize, q, op), (0, 1, 4, true, 0));
            }
            other => panic!("neg v0.4s,v1.4s -> SimdArithUnary esize4, got {other:?}"),
        }
        // fcvtzs/fcvtzu still decode to FcvVec (bit16 set, now required).
        match decode(0x4ee1b820) {
            // fcvtzs v0.2d, v1.2d
            Inst::FcvVec { rd, esize, q, .. } => {
                assert_eq!((rd, esize, q), (0, 8, true));
            }
            other => panic!("fcvtzs v0.2d -> FcvVec, got {other:?}"),
        }
        match decode(0x2ea1b820) {
            // fcvtzu v0.2s, v1.2s
            Inst::FcvVec { rd, esize, q, signed, .. } => {
                assert_eq!((rd, esize, q, signed), (0, 4, false, false));
            }
            other => panic!("fcvtzu v0.2s -> FcvVec, got {other:?}"),
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
                ..
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

    #[test]
    fn ldr_reg_index_ext_uxtw_vs_lsl() {
        // `ldr w8,[x8,w0,uxtw#2]` = 0xb8605908 (option[14:13]=2 = UXTW: index is
        // w0, zero-extended — real Roblox hash-table load that MUST mask a
        // bit-32 sentinel). vs `ldr x5,[x14,x5]` = 0xf86569c5 (option 3 = LSL:
        // full 64-bit x5 index). The translator must extend the index per this
        // field or the sentinel high bit leaks into the address (SIGSEGV).
        match decode(0xb8605908) {
            Inst::LdStrReg { rn, rt, rm, index_ext, .. } => {
                assert_eq!((rn, rt, rm), (8, 8, 0));
                assert_eq!(index_ext, 2, "uxtw index load must decode index_ext=2");
            }
            other => panic!("expected LdStrReg for uxtw load, got {other:?}"),
        }
        match decode(0xf86569c5) {
            Inst::LdStrReg { index_ext, rm, rn, .. } => {
                assert_eq!((rm, rn), (5, 14));
                assert_eq!(index_ext, 3, "full-64 `ldr x5,[x14,x5]` must decode index_ext=3");
            }
            other => panic!("expected LdStrReg for lsl load, got {other:?}"),
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
                Inst::VecMovi { vd, lo, hi, .. } => {
                    assert_eq!(lo, want_lo, "{label}: low64");
                    assert_eq!(hi, want_hi, "{label}: hi64");
                }
                other => panic!("{label}: expected VecMovi, got {other:?}"),
            }
        }
    }

    #[test]
    fn movi_2d_byte_select_ground_truth() {
        // MOVI Vd.2D, #<imm> is a byte-select pattern, NOT a straight imm8
        // replicate. Verified against aarch64 objdump + qemu-aarch64:
        //   movi v0.2d,#0x00000000000000ff = 0x6f00e420 -> lane 0xff
        //   movi v0.2d,#0x000000000000ff00 = 0x6f00e440 -> lane 0xff00
        //   movi v0.2d,#0x000000000000ffff = 0x6f00e460 -> lane 0xffff
        //   movi v0.2d,#0x00000000ff000000 = 0x6f00e500 -> lane 0xff000000
        //   movi v0.2d,#0x00000000ffff0000 = 0x6f00e580 -> lane 0xffff0000
        //   movi v0.2d,#0xffffffff00000000 = 0x6f07e600 -> lane 0xffffffff00000000
        for (w, want, label) in [
            (0x6f00_e420u32, 0x0000_0000_0000_00ffu64, "movi v0.2d,#0xff"),
            (0x6f00_e440u32, 0x0000_0000_0000_ff00u64, "movi v0.2d,#0xff00"),
            (0x6f00_e460u32, 0x0000_0000_0000_ffffu64, "movi v0.2d,#0xffff"),
            (0x6f00_e500u32, 0x0000_0000_ff00_0000u64, "movi v0.2d,#0xff000000"),
            (0x6f00_e580u32, 0x0000_0000_ffff_0000u64, "movi v0.2d,#0xffff0000"),
            (0x6f07_e600u32, 0xffff_ffff_0000_0000u64, "movi v0.2d,#0xffffffff00000000"),
        ] {
            match decode(w) {
                Inst::VecMovi { vd, lo, hi, .. } => {
                    assert_eq!(vd, 0, "{label}: vd");
                    assert_eq!(lo, want, "{label}: lo");
                    assert_eq!(hi, want, "{label}: hi");
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
                // A real UBFM extract (lsl) must NOT be mis-decoded as Ror.
                                                assert!(!matches!(decode(0xbbf13c69), Inst::Ror { .. }));
                                            }

                                            #[test]
                                            fn ccmp_ccmn_decode() {
                                                // Real gcc encodings (cross-disassembled this session).
                                                // ccmp X imm: 0xfa450809 = ccmp x0, #5, #0x9, eq
                                                match decode(0xfa450809) {
                                                    Inst::CcMp { rn, rm, imm, nzcv, cond, cmn, sf, is_reg } => {
                                                        assert_eq!(rn, 0);
                                                        assert_eq!(imm, 5);
                                                        assert_eq!(nzcv, 9);
                                                        assert_eq!(cond, 0x0); // eq
                                                        assert!(!cmn); // ccmp = subtract
                                                        assert!(sf); // X form
                                                        assert!(!is_reg); // immediate
                                                        assert_eq!(rm, 0);
                                                    }
                                                    other => panic!("ccmp x0,#5,#9,eq -> {other:?}"),
                                                }
                                                // ccmp W imm: 0x7a478842 = ccmp w2, #7, #0x2, hi
                                                match decode(0x7a478842) {
                                                    Inst::CcMp { rn, imm, nzcv, cond, cmn, sf, is_reg, .. } => {
                                                        assert_eq!(rn, 2);
                                                        assert_eq!(imm, 7);
                                                        assert_eq!(nzcv, 2);
                                                        assert_eq!(cond, 0x8); // hi
                                                        assert!(!cmn);
                                                        assert!(!sf); // W form
                                                        assert!(!is_reg);
                                                    }
                                                    other => panic!("ccmp w2,#7,#2,hi -> {other:?}"),
                                                }
                                                // ccmn X imm = 0xba450809 = ccmn x0, #5, #0x9, eq
                                                match decode(0xba450809) {
                                                    Inst::CcMp { cmn, imm, nzcv, cond, .. } => {
                                                        assert!(cmn); // ccmn = add
                                                        assert_eq!(imm, 5);
                                                        assert_eq!(nzcv, 9);
                                                        assert_eq!(cond, 0x0);
                                                    }
                                                    other => panic!("ccmn x0,#5,#9,eq -> {other:?}"),
                                                }
                                                // ccmp X reg = 0xfa4610a8 = ccmp x5, x6, #0x8, ne
                                                match decode(0xfa4610a8) {
                                                    Inst::CcMp { rn, rm, imm, nzcv, cond, is_reg, .. } => {
                                                        assert_eq!(rn, 5);
                                                        assert_eq!(rm, 6);
                                                        assert_eq!(imm, 0); // register form: no imm
                                                        assert_eq!(nzcv, 8);
                                                        assert_eq!(cond, 0x1); // ne
                                                        assert!(is_reg);
                                                    }
                                                    other => panic!("ccmp x5,x6,#8,ne -> {other:?}"),
                                                }
                                                // The real bitman-loop idiom: ccmp x1,#0,#0x1,ne = 0xfa401821
                                                match decode(0xfa401821) {
                                                    Inst::CcMp { rn, imm, nzcv, cond, is_reg, .. } => {
                                                        assert_eq!(rn, 1);
                                                        assert_eq!(imm, 0);
                                                        assert_eq!(nzcv, 1);
                                                        assert_eq!(cond, 0x1); // ne
                                                        assert!(!is_reg);
                                                    }
                                                    other => panic!("ccmp x1,#0,#1,ne -> {other:?}"),
                                                }
                                                // A genuine logical set-flags instruction must NOT be swallowed as CcMp.
                                                // ands x0,x1,x2 = 0xea020020 (opc=11 S, N=0, lsl#0) —
                                                // shares the S-flag top-byte space the ccmp family
                                                // overlaps. Also confirm `sbcs` (0xfa020020, add-sub
                                                // carry) is left alone (not CcMp).
                                                assert!(!matches!(decode(0xea020020), Inst::CcMp { .. }));
                                                assert!(!matches!(decode(0xfa020020), Inst::CcMp { .. }));
                                                assert!(matches!(decode(0xea020020), Inst::LogicReg { .. }));
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
                                    Inst::ScalarUcvtf { rd, rn, sng } => {
                                        assert_eq!(rd, 0);
                                        assert_eq!(rn, 1);
                                        assert!(!sng, "D-form ucvtf d0,d1 -> sng=false");
                                    }
                                    other => panic!("ucvtf d0,d1 -> {other:?}"),
                                }
                                // S-form: ucvtf s24,s24 = 0x7e21db18 -> sng=true
                                match decode(0x7e21db18) {
                                    Inst::ScalarUcvtf { rd, rn, sng } => {
                                        assert_eq!(rd, 24);
                                        assert_eq!(rn, 24);
                                        assert!(sng, "S-form ucvtf s24,s24 -> sng=true");
                                    }
                                    other => panic!("ucvtf s24,s24 -> {other:?}"),
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
            Inst::FcvtToInt { rd, rn, mode, sf, unsigned, src_sng, .. } => {
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
        // mrs x0, dczid_el0 (0xd53b00e0, from glibc __libc_start_main) => sysreg 5.
        match decode(0xd53b00e0) {
            Inst::SysReg { sysreg, rt, read } => {
                assert_eq!(sysreg, 5);
                assert_eq!(rt, 0);
                assert!(read);
            }
            other => panic!("mrs dczid_el0 -> {other:?}"),
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
            Inst::Fcmp { rn, rm, against_zero, sz } => {
                assert_eq!(rn, 7);
                assert_eq!(rm, 6);
                assert!(!against_zero);
                assert!(sz);
            }
            other => panic!("fcmp d7,d6 -> {other:?}"),
        }
        // fcmp d6, d16 = 0x1e7020c0 (real libroblox; high rm reg folded into the
        // base nibble) → must still decode as Fcmp with rm=16.
        match decode(0x1e7020c0) {
            Inst::Fcmp { rn, rm, against_zero, sz } => {
                assert_eq!(rn, 6);
                assert_eq!(rm, 16);
                assert!(!against_zero);
                assert!(sz);
            }
            other => panic!("fcmp d6,d16 -> {other:?}"),
        }
        // The `#0.0` immediate form (gcc codegen `fcmp d30, #0.0` = 0x1e6023c8,
        // `fcmpe d31,#0.0` = 0x1e6023f8) has the SAME rm==0 field as a pure
        // register compare `fcmp dN, d0` (0x1e6023c0) — bit3 (0x8) is the only
        // discriminator. It must decode with against_zero=true.
        match decode(0x1e6023c8) {
            Inst::Fcmp { rn, against_zero, sz, .. } => {
                assert_eq!(rn, 30);
                assert!(against_zero);
                assert!(sz);
            }
            other => panic!("fcmp d30,#0.0 -> {other:?}"),
        }
        match decode(0x1e6023c0) {
            Inst::Fcmp { against_zero, .. } => assert!(!against_zero), // d0 reg form
            other => panic!("fcmp d30,d0 -> {other:?}"),
        }
        match decode(0x1e6023f8) {
            Inst::Fcmp { rn, against_zero, .. } => {
                assert_eq!(rn, 31);
                assert!(against_zero);
            }
            other => panic!("fcmpe d31,#0.0 -> {other:?}"),
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
            Inst::SimdBit { rd, rn, rm, bif } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 2);
                assert_eq!(rm, 1);
                assert!(!bif);
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
    fn mls_and_uzp2_decode_as_specific_ops_not_sub_or_rev() {
        // Regression: `mls` (multiply-subtract) used to decode as a plain
        // Simd4s SUB (losing the multiply) and `uzp2` (unpack-high) used to
        // decode as SimdRev (rev64). Both broke gcc's magic-division %101
        // reducer. Pin them to the real ops here.
        // mls v26.4s, v0.4s, v28.4s = 0x6ebc941a (from gcc %101 loop)
        match decode(0x6ebc941a) {
            Inst::SimdMla { rd, rn, rm, lanes, sub } => {
                assert_eq!(rd, 26);
                assert_eq!(rn, 0);
                assert_eq!(rm, 28);
                assert_eq!(lanes, 4);
                assert!(sub, "mls must set sub=true");
            }
            other => panic!("mls v26.4s,v0.4s,v28.4s -> {other:?}"),
        }
        // uzp2 v0.4s, v25.4s, v0.4s = 0x4e805b20 (from gcc %101 loop)
        match decode(0x4e805b20) {
            Inst::SimdUz2 { rd, rn, rm, esize, q } => {
                assert_eq!(rd, 0);
                assert_eq!(rn, 25);
                assert_eq!(rm, 0);
                assert_eq!(esize, 4);
                assert!(q);
            }
            other => panic!("uzp2 v0.4s,v25.4s,v0.4s -> {other:?}"),
        }
    }

    #[test]
    fn rev16_decodes_as_granule2_not_uzp() {
        // Regression: SIMD rev16 (16-bit halfword byte-swap, granule 2) used
        // to fall through the rev64/rev32 gates (only byte1 0x08 was handled)
        // and mis-decode to a wrong-value instruction instead of trapping —
        // the byte-reverse/permute fuzzer's silent-miscompile class. Real
        // encodings from GCC: rev16 v0.16b = 0x4e201800, .8b = 0x0e201800.
        for (word, q) in [(0x4e201800u32, true), (0x0e201800, false)] {
            match decode(word) {
                Inst::SimdRev { granule, q: gotq, .. } => {
                    assert_eq!(granule, 2, "rev16 must be granule 2 for {word:#x}");
                    assert_eq!(gotq, q, "rev16 q flag for {word:#x}");
                }
                other => panic!("rev16 {word:#x} -> {other:?} (expected SimdRev granule 2)"),
            }
        }
        // uzp1 v0.16b,v0.16b,v1.16b = 0x4e011800: bit21 clear must NOT capture
        // as rev16 (it is a 3-same permute, granule-agnostic).
        assert!(matches!(
            decode(0x4e011800),
            Inst::SimdUz1 { .. } | Inst::SimdUz2 { .. }
        ), "uzp1 (0x4e011800) must not become rev16");
        // rev32 v0.16b must stay granule 4 (not relabeled).
        match decode(0x6e200800) {
            Inst::SimdRev { granule, q: true, .. } => assert_eq!(granule, 4),
            other => panic!("rev32 v0.16b -> {other:?}"),
        }
    }

    #[test]
    fn rev64_all_elem_sizes_decode_as_granule8() {
        // Regression: the rev64 gate was `(insn & 0x3f00_0c00)==0x0e00_0800 &&
        // (insn & 0x1800)==0`, but EVERY rev64 has byte1 0x08 (bit11 set), so the
        // `(insn & 0x1800)==0` guard (meant to drop uzp) wrongly rejected the
        // whole rev64 family -> all rev64 decoded as Unsupported. Real libroblox
        // uses rev64 v5.2s (0x0ea008a5). Pin all six elem sizes to granule 8.
        for (word, q) in [
            (0x0e2008a5u32, false), // rev64 v5.8b
            (0x4e2008a5, true),     // rev64 v5.16b
            (0x0e6008a5, false),    // rev64 v5.4h
            (0x4e6008a5, true),     // rev64 v5.8h
            (0x0ea008a5, false),    // rev64 v5.2s  [real libroblox boot]
            (0x4ea008a5, true),     // rev64 v5.4s
        ] {
            match decode(word) {
                Inst::SimdRev { granule, q: gotq, .. } => {
                    assert_eq!(granule, 8, "rev64 must be granule 8 for {word:#x}");
                    assert_eq!(gotq, q, "rev64 q flag for {word:#x}");
                }
                other => panic!("rev64 {word:#x} -> {other:?} (expected SimdRev granule 8)"),
            }
        }
        // dup-from-GPR v9.8b (0x0e010d09, byte1 0x0d) must NOT be captured as rev64.
        assert!(
            matches!(decode(0x0e010d09), Inst::SimdDupGp { .. }),
            "dup v9.8b (0x0e010d09) must stay SimdDupGp, got {:?}",
            decode(0x0e010d09)
        );
    }

    #[test]
    fn shll_widening_with_high_rn_decodes() {
        // Regression: the WidenShl (shll/shll2) gate required
        // `((insn>>8)&0x03)==0`, but bits[9:8] are rn bits[4:3] (not a permute
        // discriminator), so any `shll Vd.4s, Vn.4h` with rn>=8 (v8-v31) decoded
        // as Unsupported. Real libroblox has shll v16.4s,v16.4h (0x2e613a10) and
        // shll2 v23.4s,v16.8h (0x6e613a17). The true discriminator vs zip/uzp/trn
        // (bit21 set = shift-imm) now stands in.
        match decode(0x2e613a10) {
            Inst::WidenShl { rd, rn, dst_esize, nlanes, signed: true, upper: false } => {
                assert_eq!(rd, 16);
                assert_eq!(rn, 16);
                assert_eq!(dst_esize, 4);
                assert_eq!(nlanes, 4);
            }
            other => panic!("shll v16.4s,v16.4h,#16 (0x2e613a10) -> {other:?}"),
        }
        match decode(0x6e613a17) {
            Inst::WidenShl { rd, rn, signed: true, upper: true, .. } => {
                assert_eq!(rd, 23);
                assert_eq!(rn, 16);
            }
            other => panic!("shll2 v23.4s,v16.8h,#16 (0x6e613a17) -> {other:?}"),
        }
        // Still not a permute: zip1 v0.8b (0x0e023a40) keeps SimdZip1 (bit21=0).
        assert!(
            matches!(decode(0x0e023a40), Inst::SimdZip1 { .. }),
            "zip1 (0x0e023a40) must stay SimdZip1, got {:?}",
            decode(0x0e023a40)
        );
    }

    #[test]
    fn cmhs_unsigned_ge_decodes_distinct_from_cmhi() {
        // cmhi (>) = byte2 0x34; cmhs (>=, unsigned higher-or-same) = byte2 0x3c
        // (bit10 set). Real libroblox has cmhs v2.2d (0x6ee13c02). Previously
        // Unsupported. cmhs must decode to its own variant (cmovae, >=) not cmhi.
        match decode(0x6ee13c02) {
            Inst::SimdCmhsD { rd: 2, rn: 0, rm: 1 } => {}
            other => panic!("cmhs v2.2d,v0,v1 (0x6ee13c02) -> {other:?}"),
        }
        match decode(0x6ea13c02) {
            Inst::SimdCmhs { rd: 2, lanes: 4, .. } => {}
            other => panic!("cmhs v2.4s (0x6ea13c02) -> {other:?}"),
        }
        match decode(0x2ea03c00) {
            Inst::SimdCmhs { lanes: 2, .. } => {}
            other => panic!("cmhs v?.2s (0x2ea03c00) -> {other:?}"),
        }
        // cmhi must stay cmhi (distinct gate, byte2 0x34).
        assert!(matches!(decode(0x6ea13400), Inst::SimdCmhi { .. }), "got {:?}", decode(0x6ea13400));
    }

    #[test]
    fn widening_mul_by_element_decodes_not_fptsel_or_movi() {
        // Regression (Session 44, fuzzer-caught): the integer LONG-by-element
        // multiply (smull/umull/smlal/umlal/smlsl/umlsl with an indexed m
        // element) previously decoded as FmlaEl (the .2d forms, sharing bit23
        // set) or VecMovi (the .4s forms) — both SILENT wrong values. GCC emits
        // these for vector*const-scalar scaling (vmlal_lane_s16/32 builtins).
        // Ground truth from aarch64-gcc (assembler objdump). bit13 (0x2000) set
        // is the discriminator vs FP fmla-el (bit13 clear).
        let cases: &[(u32, u8, u8, u8, u8, u8, bool, bool, bool, bool)] = &[
            // (word, rd, rn, rm, index, res_esize, unsigned, q, acc, sub)
            (0x0f40a000, 0, 0, 0, 0, 4, false, false, false, false), // smull .4s h[0]
            (0x0f50a000, 0, 0, 0, 1, 4, false, false, false, false), // smull .4s h[1]
            (0x0f70a000, 0, 0, 0, 3, 4, false, false, false, false), // smull .4s h[3]
            (0x2f60a000, 0, 0, 0, 2, 4, true,  false, false, false), // umull .4s h[2]
            (0x0f80a000, 0, 0, 0, 0, 8, false, false, false, false), // smull .2d s[0]
            (0x0fa0a000, 0, 0, 0, 1, 8, false, false, false, false), // smull .2d s[1]
            (0x2f80a000, 0, 0, 0, 0, 8, true,  false, false, false), // umull .2d s[0]
            (0x0f712020, 0, 1, 1, 3, 4, false, false, true,  false), // smlal .4s h[3]
            (0x0fa12020, 0, 1, 1, 1, 8, false, false, true,  false), // smlal .2d s[1]
            (0x0f816020, 0, 1, 1, 0, 8, false, false, true,  true),  // smlsl .2d s[0]
            (0x2f512020, 0, 1, 1, 1, 4, true,  false, true,  false), // umlal .4s h[1]
            (0x2f816020, 0, 1, 1, 0, 8, true,  false, true,  true),  // umlsl .2d s[0]
            (0x4f51a000, 0, 0, 1, 1, 4, false, true,  false, false), // smull2 .4s h[1] (q upper)
            (0x4f81a000, 0, 0, 1, 0, 8, false, true,  false, false), // smull2 .2d s[0]
            (0x6f71a000, 0, 0, 1, 3, 4, true,  true,  false, false), // umull2 .4s h[3]
            (0x6fa1a000, 0, 0, 1, 1, 8, true,  true,  false, false), // umull2 .2d s[1]
            // the failing real-binary words:
            (0x0f44a3de, 30, 30, 4, 0, 4, false, false, false, false), // smull v30,v30,v4.h[0]
            (0x0f4723be, 30, 29, 7, 0, 4, false, false, true,  false), // smlal v30,v29,v7.h[0]
            (0x0f84a3ff, 31, 31, 4, 0, 8, false, false, false, false), // smull v31,v31,v4.s[0]
            (0x0f87237f, 31, 27, 7, 0, 8, false, false, true,  false), // smlal v31,v27,v7.s[0]
            (0x0f8523bf, 31, 29, 5, 0, 8, false, false, true,  false), // smlal v31,v29,v5.s[0]
        ];
        for &(word, rd, rn, rm, index, res_esize, u, q, acc, sub) in cases {
            match decode(word) {
                Inst::SimdMullEl { rd: grd, rn: grn, rm: grm, index: gix, res_esize: gre, unsigned: gu, q: gq, acc: ga, sub: gs } => {
                    assert_eq!(grd, rd, "{word:#x} rd");
                    assert_eq!(grn, rn, "{word:#x} rn");
                    assert_eq!(grm, rm, "{word:#x} rm");
                    assert_eq!(gix, index, "{word:#x} index");
                    assert_eq!(gre, res_esize, "{word:#x} res_esize");
                    assert_eq!(gu, u, "{word:#x} unsigned");
                    assert_eq!(gq, q, "{word:#x} q");
                    assert_eq!(ga, acc, "{word:#x} acc");
                    assert_eq!(gs, sub, "{word:#x} sub");
                }
                other => panic!("{word:#x} -> {other:?} (expected SimdMullEl)"),
            }
        }
        // FP fmla-el and non-widening int mla-el must NOT be captured (bit13 clear).
        assert!(matches!(decode(0x4fa11000), Inst::FmlaEl { .. }), "FP fmla-el must stay");
        assert!(matches!(decode(0x2f820020), Inst::SimdMlaEl { .. }), "int MLA-el must stay");
        // pure movi must stay VecMovi, not become a widening mul.
        assert!(matches!(decode(0x6f00e400), Inst::VecMovi { .. }), "movi v0.2d,#0 must stay");
    }

    #[test]
    fn saturating_narrow_variants_decode_correctly() {
        // Regression (Session 44, fuzzer-caught): SIMD sqxtn/uqxtn/sqxtun had a
        // wrong gate (byte2 exact-match instead of masked) so the unsigned-dst
        // family was swallowed by the rev32 gate (0x2e prefix, bit23 set) as a
        // SimdRev, and the signed flags were misderived. Also movi-msl and
        // mvni-msl (vector immediate MSL) were captured by the shift-immediate
        // gate (movi converged to SimdShl). Ground truth from aarch64-gcc.
        // (word, dst_esize, src_signed, dst_signed, q)
        let cases: &[(u32, u8, bool, bool, bool)] = &[
            (0x0e614800, 2, true,  true,  false), // sqxtn v.4h,v.4s
            (0x0e214800, 1, true,  true,  false), // sqxtn v.8b,v.8h
            (0x0ea14800, 4, true,  true,  false), // sqxtn v.2s,v.2d
            (0x2e614800, 2, false, false, false), // uqxtn v.4h,v.4s
            (0x2e612800, 2, true,  false, false), // sqxtun v.4h,v.4s (signed src -> unsigned dst)
            (0x2e212800, 1, true,  false, false), // sqxtun v.8b,v.8h
            (0x4e614800, 2, true,  true,  true),  // sqxtn2 (q upper)
            (0x6e612800, 2, true,  false, true),  // sqxtun2
            (0x0e614bff, 2, true,  true,  false), // real: sqxtn v31.4h,v31.4s
            (0x2e614bff, 2, false, false, false), // real: uqxtn v31.4h,v31.4s
            (0x2e612bff, 2, true,  false, false), // real: sqxtun v31.4h,v31.4s (byte2 0x2b)
        ];
        for &(word, de, ss, ds, q) in cases {
            match decode(word) {
                Inst::SaturatNarrow { dst_esize, src_signed, dst_signed, q: gq, .. } => {
                    assert_eq!(dst_esize, de, "{word:#x} dst_esize");
                    assert_eq!(src_signed, ss, "{word:#x} src_signed");
                    assert_eq!(dst_signed, ds, "{word:#x} dst_signed");
                    assert_eq!(gq, q, "{word:#x} q");
                }
                other => panic!("{word:#x} -> {other:?} (expected SaturatNarrow)"),
            }
        }
        // plain xtn (not saturating, b2 0x28 with 0x0e prefix) stays xtn not sqxtun.
        // vector-immediate msl/mvni must NOT decode as shifts.
        assert!(matches!(decode(0x4f00d43b), Inst::VecMovi { .. }), "movi msl16");
        match decode(0x6f07c7fc) { // mvni v.4s,#0xff,msl8  -> per-lane 0xffff0000
            Inst::VecMovi { lo, hi, .. } => {
                assert_eq!(lo, 0xffff_0000_ffff_0000, "mvni msl8 lo");
                assert_eq!(hi, 0xffff_0000_ffff_0000, "mvni msl8 hi");
            }
            other => panic!("mvni msl8 -> {other:?}"),
        }
        // genuine shifts stay shifts
        assert!(matches!(decode(0x4f2157bd), Inst::SimdShl { .. }), "shl v29.4s,#1");
        assert!(matches!(decode(0x6f580400), Inst::SimdShr { .. }), "ushr v0.2d,#40");
    }

    #[test]
    fn addhn_family_not_colliding_with_three_same_minmax() {
        // Session (cycle 44c): the SimdHighNarrow gate matched byte2 &0xe0 in
        // {0x40,0x60} but that set also matched three-same smin(.2s=0x0ea16c00)/
        // smax, silently miscompiling them. bit10(0x400)=0 marks addhn/subhn/
        // raddhn/rsubhn (three-different); three-same ops set it.
        assert!(matches!(decode(0x0e614000), Inst::SimdHighNarrow { sub: false, .. }));
        assert!(matches!(decode(0x0e616000), Inst::SimdHighNarrow { sub: true, .. }));
        assert!(matches!(decode(0x0ea14000), Inst::SimdHighNarrow { .. }));
        assert!(!matches!(decode(0x0ea16c00), Inst::SimdHighNarrow { .. })); // smin .2s
        assert!(!matches!(decode(0x0ea16400), Inst::SimdHighNarrow { .. })); // smax .2s
        assert!(!matches!(decode(0x4ea16c00), Inst::SimdHighNarrow { .. })); // smin .4s q
    }

    #[test]
    fn adalp_gate_masks_rn_low_bits_without_swallowing_smin() {
        // Session (cycle 44c): adalp byte2 low bits carry rn[1:0] (0x68|reg -> 0x6b),
        // so the gate must mask &0xfc; an earlier &0xf8 also caught smin(.2s byte2=0x6c)
        // and smax, corrupting min/max math on real paths.
        assert!(matches!(decode(0x2e606bfb), Inst::SimdAdalp { .. }));
        assert!(matches!(decode(0x0e60681f), Inst::SimdAdalp { .. }));
        assert!(!matches!(decode(0x0ea16c00), Inst::SimdAdalp { .. })); // smin .2s
        assert!(!matches!(decode(0x6e216c00), Inst::SimdAdalp { .. })); // umin .16b
    }

    #[test]
    fn sat_narrow_shift_and_rshrn_decode_correctly() {
        use crate::decode::{decode, Inst};
        // Session (cycle 44h): sqshrn/uqshrn/sqshrun were decoding as SimdShrAcc
        // (non-saturating) and rshrn as shrn (no rounding add).
        // sqshrn v31.8b, v31.8h, #4 (0x0f0c97ff): src_esize 2, dst 1, shift 4
        let sqshrn = decode(0x0f0c97ff);
        assert!(matches!(sqshrn, Inst::SatNarrowShift { src_esize: 2, dst_esize: 1, shift: 4, src_signed: true, dst_signed: true, .. }), "got {sqshrn:?}");
        // uqshrn v31.8b, v31.8h, #4 (0x2f0c97ff): unsigned -> src/dst false
        let uqshrn = decode(0x2f0c97ff);
        assert!(matches!(uqshrn, Inst::SatNarrowShift { src_signed: false, dst_signed: false, .. }), "got {uqshrn:?}");
        // sqshrun v1.8b, v2.8h, #4 (0x2f0c8441): dst unsigned, src signed
        let sqshrun = decode(0x2f0c8441);
        assert!(matches!(sqshrun, Inst::SatNarrowShift { src_signed: true, dst_signed: false, .. }), "got {sqshrun:?}");
        // shrn v1.4h, v2.4s, #3 (0x0f1d8441): plain, no round
        let shrn = decode(0x0f1d8441);
        assert!(matches!(shrn, Inst::SimdShrn { round: false, shift: 3, .. }), "got {shrn:?}");
        // rshrn (0x0f1d8c41): round=true
        let rshrn = decode(0x0f1d8c41);
        assert!(matches!(rshrn, Inst::SimdShrn { round: true, shift: 3, .. }), "got {rshrn:?}");
        // by-element widen mul must NOT be captured as SatNarrowShift
        assert!(matches!(decode(0x2f60a000), Inst::SimdMullEl { .. }), "got {:?}", decode(0x2f60a000));
    }

    #[test]
    fn sat_narrow_shift_gate_not_capturing_by_element_mul() {
        use crate::decode::{decode, Inst};
        assert!(matches!(decode(0x2f60a000), Inst::SimdMullEl { .. }), "got {:?}", decode(0x2f60a000));
        assert!(matches!(decode(0x4f60a000), Inst::SimdMullEl { .. }), "got {:?}", decode(0x4f60a000));
    }

    #[test]
    fn fp_pairwise_three_op_decodes_correctly() {
        use crate::decode::{decode, Inst};
        // Session (cycle 44i): fmaxp/fminp/faddp Vd,Vn,Vm (3-same) were
        // Unsupported; only the 2-reg (0x7e) reduce was handled.
        // faddp v1.2s,v2.2s,v3.2s (0x2e23d441): add
        assert!(matches!(decode(0x2e23d441), Inst::SimdFpPair3 { rd:1, rn:2, rm:3, add:true, min:false, .. }), "got {:?}", decode(0x2e23d441));
        // fmaxp v1.4s,v2,v3 (0x6e23f441): q=1, add=false, min=false
        assert!(matches!(decode(0x6e23f441), Inst::SimdFpPair3 { add:false, min:false, nm:false, q:true, .. }), "got {:?}", decode(0x6e23f441));
        // fminp (0x2ea3f441): min=true
        assert!(matches!(decode(0x2ea3f441), Inst::SimdFpPair3 { min:true, add:false, .. }), "got {:?}", decode(0x2ea3f441));
        // fmaxnmp (0x2e23c441): nm=true
        assert!(matches!(decode(0x2e23c441), Inst::SimdFpPair3 { nm:true, .. }), "got {:?}", decode(0x2e23c441));
        // self-aliased fmaxp v31.2s,v31,v30 (0x2e3ef7ff): rm encoded in bit16+
        assert!(matches!(decode(0x2e3ef7ff), Inst::SimdFpPair3 { rd:31, rn:31, rm:30, .. }), "got {:?}", decode(0x2e3ef7ff));
        // plain fadd (0x0e23d441, prefix 0x0e) must NOT be captured as FpPair3
        assert!(matches!(decode(0x0e23d441), Inst::VecFpArith { op: 0, .. }), "got {:?}", decode(0x0e23d441));
    }

    #[test]
    fn saturating_left_shift_sqshl_decodes() {
        use crate::decode::{decode, Inst};
        // sqshl v1.4h,v2.4h,#12 = 0x0f1c7441 -> signed, esize 2, shift 12
        assert!(matches!(decode(0x0f1c7441), Inst::SimdSatShl { esize: 2, shift: 12, sat: 0, .. }), "got {:?}", decode(0x0f1c7441));
        // uqshl v1.4h,v2.4h,#12 = 0x2f1c7441 -> sat 1 (unsigned)
        assert!(matches!(decode(0x2f1c7441), Inst::SimdSatShl { sat: 1, .. }), "got {:?}", decode(0x2f1c7441));
        // sqshlu v1.4h,v2.4h,#12 = 0x2f1c6441 -> sat 2 (signed src, unsigned dst)
        assert!(matches!(decode(0x2f1c6441), Inst::SimdSatShl { sat: 2, .. }), "got {:?}", decode(0x2f1c6441));
        // sqshl v1.2s,v2.2s,#12 = 0x0f2c7441 -> esize 4
        assert!(matches!(decode(0x0f2c7441), Inst::SimdSatShl { esize: 4, .. }), "got {:?}", decode(0x0f2c7441));
        // sqshl v1.2d,v2.2d,#12 = 0x4f4c7441 -> esize 8
        assert!(matches!(decode(0x4f4c7441), Inst::SimdSatShl { esize: 8, .. }), "got {:?}", decode(0x4f4c7441));
        // plain shl v1.4h,v2.4h,#1 must NOT be captured as SatShl
        let _ = decode(0x0f117441); // this IS sqshl #1; plain shl uses bits[14:12]=5
    }

    #[test]
    fn int_pairwise_maxmin_smaxp_decodes() {
        use crate::decode::{decode, Inst};
        // smaxp v30.8b, v31.8b, v30.8b = 0x0e3ea7fe (self-aliased, high regs)
        assert!(matches!(decode(0x0e3ea7fe), Inst::SimdMaxMinP { rd: 30, rn: 31, rm: 30, min: false, unsigned: false, esize: 1, .. }), "got {:?}", decode(0x0e3ea7fe));
        // umaxp v30.8b, v31.8b, v30.8b = 0x2e3ea7fe
        assert!(matches!(decode(0x2e3ea7fe), Inst::SimdMaxMinP { unsigned: true, min: false, .. }), "got {:?}", decode(0x2e3ea7fe));
        // sminp v1.4h, v2, v3 = 0x0e63ac41
        assert!(matches!(decode(0x0e63ac41), Inst::SimdMaxMinP { min: true, esize: 2, q: false, .. }), "got {:?}", decode(0x0e63ac41));
        // smaxp v1.4s, v2, v3 = 0x4ea3a441 (q=1, esize 4)
        assert!(matches!(decode(0x4ea3a441), Inst::SimdMaxMinP { esize: 4, q: true, .. }), "got {:?}", decode(0x4ea3a441));
        // plain byte add v1.8b,v2,v3 (0x0e23a441 is MAXP; real add 0x0ea28421-ish) must not be captured:
        // 0x0e218421 add v1.8b,v2,v3 ... verify still SimdAddB
        assert!(matches!(decode(0x0e218421), Inst::SimdAddB { .. }), "got {:?}", decode(0x0e218421));
    }

    #[test]
    fn dup_from_gpr_invalid_imm5_does_not_panic() {
        use crate::decode::{decode, Inst};
        // SimdDupGp gate: (insn & 0xff00_fc00)==0x0e00_0c00 | 0x4e00_0c00.
        // Valid dup-from-GPR has imm5 (bits[20:16]) a power of two {1,2,4,8}
        // => esize {1,2,4,8}. imm5==0 (and any non-power-of-two) is an invalid
        // encoding: the old `1u8 << (insn>>16&0x1f).trailing_zeros()` PANICS on
        // imm5==0 (trailing_zeros of 0 == 32, shift overflows u8) and would
        // abort the whole JIT on arbitrary guest bytes. decode() must never panic.
        let valid_cases = [
            (0x0e010d09u32, 1u8), // dup v9.8b, w0 (imm5=1 -> esize 1) [real libroblox insn]
            (0x0e040c00u32, 4u8), // dup v0.4s, w0 (imm5=4 -> esize 4, Q=0)
            (0x4e080d00u32, 8u8), // dup v0.2d, x0 (imm5=8 -> esize 8, Q=1)
        ];
        for (w, esize) in valid_cases {
            assert!(
                matches!(decode(w), Inst::SimdDupGp { esize: e, .. } if e == esize),
                "0x{w:08x} should decode SimdDupGp esize={esize}, got {:?}",
                decode(w)
            );
        }
        // imm5==0 is invalid: must return Unsupported, NOT panic.
        for w in [0x0e000c00u32, 0x0e000c20, 0x4e000c00, 0x0e800d8c] {
            let d = decode(w);
            assert!(
                matches!(d, Inst::Unsupported(_)),
                "0x{w:08x} (imm5==0) must return Unsupported, got {d:?}"
            );
        }
        // Non-power-of-two imm5 (e.g. 3, 5, 31) is likewise invalid -> Unsupported.
        for w in [0x0e030c00u32, 0x4e050d00, 0x0e1f0c00] {
            assert!(
                matches!(decode(w), Inst::Unsupported(_)),
                "0x{w:08x} (imm5 non-power-of-two) must return Unsupported, got {:?}",
                decode(w)
            );
        }
    }

    #[test]
    fn umov_smov_and_vector_dup_zero_imm5_do_not_panic() {
        use crate::decode::{decode, Inst};
        // Two more decode() PANIC sites found by the whole-executable scandecode
        // scan (data-region bytes a computed branch could land on):
        //   1. umov/smov gate (insn&0xbfe0_fc00)=={0x0e00_3c00,0x0e00_2c00}:
        //      `1 << imm5.trailing_zeros()` PANICS when imm5==0 (tz=32).
        //   2. vector dup gate (insn&0xffe0_0c00)=={0x0e00_0400,0x4e00_0400}:
        //      `1 << (f & f.wrapping_neg()).trailing_zeros()` PANICS when f==0.
        // decode() must never panic on arbitrary guest bytes -> Unsupported.
        // Valid ElementSizeD must still decode correctly (imm5 packs esize AND
        // the lane index, so it is NOT bounded to a power of two):
        //  - umov: 0x0e003c00 is raw gate with imm5==0 (invalid).
        // smov gate with imm5==0: 0x0e002c00.
                for w in [0x0e002c00u32, 0x0e000400, 0x4e000400, 0x0e002c1f, 0x4e00041f] {
                    let d = decode(w);
                    assert!(
                        matches!(d, Inst::Unsupported(_)),
                        "0x{w:08x} (element-size field 0) must return Unsupported, got {d:?}"
                    );
                }
        // A real vector-dup with f packing esize=4 (S) and src index 1
        // (f = 0b01100 = 12): must decode as SimDup with esize 4, src_idx 1 —
        // NOT be mis-guarded (the earlier over-eager f>8 guard wrongly rejected it).
        //   dup v21.2s, v23.s[1] = 0x0e0c06f5 (real libroblox insn)
        assert!(
            matches!(decode(0x0e0c06f5), Inst::SimDup { rd, esize: 4, src_idx: 1, q: false, .. } if rd == 21),
            "dup v21.2s, v23.s[1] should decode SimDup esize=4 src=1, got {:?}",
            decode(0x0e0c06f5)
        );
        // dup v27.2s, v24.s[1] = 0x0e0c071b
        assert!(
            matches!(decode(0x0e0c071b), Inst::SimDup { rd: 27, esize: 4, src_idx: 1, q: false, .. }),
            "got {:?}",
            decode(0x0e0c071b)
        );
    }

    #[test]
    fn mul_decodes_as_multiply_not_bitwise_logical() {
        // Regression: NEON element-wise `mul` shares bits[11:10] with the
        // vector AND/ORR/BIC family but sets bit15 (byte1 0x8c..0x9f vs
        // 0x1c/0x1d). The old SimdVLog gate checked only `&0x1c00==0x1c00`,
        // so `mul` decoded as a bitwise op and miscompiled every element-wise
        // multiply — gcc's unsigned magic-division dividend `mul v26.4s,v26,
        // v28(97)` became AND/ORR garbage, silently zeroing the quotients.
        // mul v26.4s, v26.4s, v28.4s = 0x4ebc9f5a (from gcc (k*97)/1000 loop)
        match decode(0x4ebc9f5a) {
            Inst::SimdMul { rd, rn, rm, lanes } => {
                assert_eq!(rd, 26);
                assert_eq!(rn, 26);
                assert_eq!(rm, 28);
                assert_eq!(lanes, 4);
            }
            other => panic!("mul v26.4s,v26.4s,v28.4s -> {other:?}"),
        }
        // mul .2s must also be a multiply (not SimdVLog)
        match decode(0x0eb19e0f) {
            Inst::SimdMul { lanes: 2, .. } => {}
            other => panic!("mul v15.2s,v16.2s,v17.2s -> {other:?}"),
        }
        // and still decodes as the logical op
        match decode(0x4e221c20) {
            Inst::SimdVLog { op: 0, .. } => {}
            other => panic!("and v0.16b,v1.16b,v2.16b -> {other:?}"),
        }
    }

    #[test]
    fn fp_2d_op_decode_collisions_with_int_add_bsl_and_fcvt() {
        // Session (Sep 11 2026): the FP `Vd.2D` two-source ops and `fmov d,#imm`
        // shared residues with three earlier decoders, silently corrupting double
        // math:
        //   - `fadd v0.2d,v0.2d,v1.2d` (0x4e61d400) -> Simd2dFp fadd, NOT integer
        //     SimdAddH (the int add/sub gates now require bit14 CLR).
        //   - `fdiv v0.2d` (0x6e61fc00) / `fmul v0.2d` (0x6e61dc00) -> Simd2dFp,
        //     NOT SimdSel (the select gate now requires bits[15:13] CLR).
        //   - `fmov d23,#12.0` (0x1e651017) -> FmovImm, NOT FcvtToInt (the fcvt
        //     round gate now requires bit12 CLR; FMOV-imm has it SET).
        match decode(0x4e61d400) {
            Inst::Simd2dFp { rd: 0, rn: 0, rm: 1, op: 2 } => {}
            other => panic!("fadd v0.2d -> {other:?}"),
        }
        match decode(0x6e61fc00) {
            Inst::Simd2dFp { op: 0, .. } => {} // fdiv
            other => panic!("fdiv v0.2d -> {other:?}"),
        }
        match decode(0x6e61dc00) {
            Inst::Simd2dFp { op: 1, .. } => {} // fmul
            other => panic!("fmul v0.2d -> {other:?}"),
        }
        match decode(0x1e651017) {
            Inst::FmovImm { rd: 23, value_bits, .. } => {
                assert_eq!(f64::from_bits(value_bits), 12.0);
            }
            other => panic!("fmov d23,#12.0 -> {other:?}"),
        }
        // and the real ops those collisions stole from must STILL decode:
        match decode(0x4e618421) {
            Inst::SimdAddH { .. } => {} // integer add v1.8h
            other => panic!("add v1.8h -> {other:?}"),
        }
        match decode(0x6e611c00) {
            Inst::SimdSel { .. } => {} // bsl v0.16b
            other => panic!("bsl v0.16b -> {other:?}"),
        }
        match decode(0x1e790020) {
            Inst::FcvtToInt { unsigned: true, .. } => {} // fcvtzu w0,d1
            other => panic!("fcvtzu w0,d1 -> {other:?}"),
        }
        // Session (cycle 33, fp_edge_9000_58): a scalar `fcsel Dd,Dn,Dm,<cond>`
        // whose rm/cond fields happen to give it a top-16 that the fcvt-to-int
        // gate also matches (0x1e64/0x1e65 = fcvtau/fcvtas) was being swallowed
        // as an FcvtToInt, which writes integer x{rd} instead of vector d{rd} —
        // so the min-accumulator register never updated and the final answer
        // carried a stale a[i]*1e6 double. The fcvt gate now also requires
        // bits[11:10]==00 (every real fcvt-to-int clears them; fcsel needs
        // 0b11). Real gcc: `fcsel d26,d28,d5,mi` = 0x1e654f9a (cond=mi=4).
        match decode(0x1e654f9a) {
            Inst::FcsSel { rd: 26, rn: 28, rm: 5, cond: 4, sz: true } => {}
            other => panic!("fcsel d26,d28,d5,mi -> {other:?}"),
        }
        match decode(0x1e65ef9a) {
            Inst::FcsSel { rd: 26, rn: 28, rm: 5, cond: 14, sz: true } => {} // al
            other => panic!("fcsel d26,d28,d5,al -> {other:?}"),
        }
        // fcvtau (real fcvt-to-int, bits[11:10]==00) must STILL decode as fcvt:
        match decode(0x1e650062) {
            Inst::FcvtToInt { mode: 2, unsigned: true, .. } => {} // fcvtau w2,d3
            other => panic!("fcvtau w2,d3 -> {other:?}"),
        }
    }

    #[test]
    fn logic_imm_rotation_asymmetric_mask_ror() {
        // DecodeBitMasks rotates the element RIGHT (ROR). A left-rotate here
        // silently miscompiled every rotation-asymmetric mask; only symmetric
        // ones (0xCCCC/0x5555/single-bit/all-ones) matched under both directions.
        // Real compiler encoding `mov x0,#0xffffffff80000001` = 0xb26187e0.
        match decode(0xb26187e0) {
            Inst::LogicImm { mask, op: 1, sf: true, rd: 0, rn: 31, .. } => {
                assert_eq!(
                    mask, 0xffffffff80000001,
                    "mov x0,#0xffffffff80000001 must decode ROR(right); got {mask:#x}"
                );
            }
            other => panic!("mov x0,#0xffffffff80000001 -> {other:?}"),
        }

        // A second rotation-asymmetric case from real gcc: `mov x0,#0x3ffffffc`
        // = 0xb27e6fe0 (sf=0, 32-bit datasize path exercises the esize!=64 mask).
        let w = decode(0xb27e6fe0);
        assert!(
            matches!(w, Inst::LogicImm { mask: 0x3ffffffc, .. }),
            "mov w0,#0x3ffffffc -> {w:?}"
        );

        // Sanity: the symmetric cases must be UNCHANGED under the fix.
        assert!(matches!(
            decode(0xb202e7e8),
            Inst::LogicImm { mask: 0xcccc_cccc_cccc_cccc, .. }
        ));
        assert!(matches!(decode(0xb24002c8), Inst::LogicImm { mask: 0x1, .. }));
    }

    #[test]
    fn mov_single_f64_bits() {
        // decode_fmov_imm: 0.5 -> 0x3fe0...  , 1.0 -> 0x3ff0...,  -2.0 -> 0xc000...
        assert_eq!(decode_fmov_imm(0x60, true), 0x3fe0_0000_0000_0000); // 0.5
        assert_eq!(decode_fmov_imm(0x70, true), 0x3ff0_0000_0000_0000); // 1.0
        assert_eq!(decode_fmov_imm(0x80, true), 0xc000_0000_0000_0000); // -2.0
        // REGRESSION (Session 99): the exponent wrap threshold was `>=4`, which
        // wrongly mapped E=3 (true exponent +4) to -4, corrupting every FMOV-imm
        // in [16.0, 30.0]. Threshold is `>=5`: 16.0/1.8e1/30.0 must decode exactly
        // (verified against the assembler's encodings for 0.125..30.0).
        assert_eq!(decode_fmov_imm(0x30, true), 0x4030_0000_0000_0000); // 16.0
        assert_eq!(decode_fmov_imm(0x3e, true), 0x403e_0000_0000_0000); // 30.0
        assert_eq!(decode_fmov_imm(0x00, true), 0x4000_0000_0000_0000); // 2.0
        assert_eq!(decode_fmov_imm(0x68, true), 0x3fe8_0000_0000_0000); // 0.75
    }

    #[test]
    fn nop_is_hint_not_scvtf_fixed() {
        // REGRESSION: the scalar fixed-point int->FP gate checked only
        // bits[29:28]==01, which the ARM system/hint family (0xd5...) also
        // satisfies. Every guest `nop` (0xd503201f, emitted as alignment padding
        // by gcc everywhere) was misdecoded as `scvtf d31, x0, #56`, converting
        // the caller's x0 (often SP) into a double and overwriting a vector
        // register with address-derived garbage — an intermittent, layout-
        // dependent miscompile in vectorized loops. The gate now also requires
        // the top byte ∈ {0x1e, 0x9e} (real scvtf-fixed encodings).
        assert!(
            matches!(decode(0xd503201f), Inst::Hint),
            "nop must decode as Hint, got {:?}",
            decode(0xd503201f)
        );
        // esb / hint-family neighbours also stay Hint.
        assert!(matches!(decode(0xd503221f), Inst::Hint));
        // Real scvtf/scvtf-fixed encodings still decode correctly:
        //   scvtf d0, x0, #1  = 0x9e42fc00 (X source / D dest) ; s0,w0,#1 = 0x1e02fc00
        assert!(matches!(
            decode(0x9e42fc00),
            Inst::ScvtfFixed { rd: 0, rn: 0, to_double: true, sf: true, fbits: 1, .. }
        ));
        assert!(matches!(
            decode(0x1e02fc00),
            Inst::ScvtfFixed { rd: 0, rn: 0, to_double: false, sf: false, .. }
        ));
        // Scalable/vector scvtf (nearest) and the unscaled scalar Scvtf intact.
        assert!(matches!(decode(0x1e620000), Inst::Scvtf { .. }), "{:?}", decode(0x1e620000));
    }

    #[test]
    fn prfm_prefetch_is_hint_not_size8_sext_load() {
        // PRFM (prefetch, unsigned-immediate) encodes as the load/store-unsigned
        // immediate shape with `size=0b11` (byte) and bit23 SET / bit22 clear —
        // the exact pattern a bit23-keyed sign-extend decoder reads as an X-reg
        // sign-extend load (and would reject with "size 8 not implemented").
        // It's a pure hint a single-threaded direct-mapped JIT ignores, so it
        // must decode as Hint, not LdStrImm.
        // Ground truth (aarch64-linux-gnu-as): prfm pldl1keep,[x0]=0xf9800000,
        //   pldl3keep,[x8]=0xf9800104 (the real libroblox.so boot word),
        //   pldl2keep,[x1,#64]=0xf9802022, pstl1keep,[x2]=0xf9800050.
        for w in [0xf9800000u32, 0xf9800104, 0xf9802022, 0xf9800050] {
            assert!(matches!(decode(w), Inst::Hint), "prfm {w:#x} must be Hint, got {:?}", decode(w));
        }
        // Sign-extend loads remain loads (size 4/2/1) — ldrsw x0,[x0] = 0xb9800000.
        assert!(matches!(decode(0xb9800000), Inst::LdStrImm { sext: true, size: 4, .. }));
        // Plain 64-bit load stays LdStrImm (size 8, no sext).
        assert!(matches!(
            decode(0xf9400000),
            Inst::LdStrImm { sext: false, size: 8, .. }
        ));
    }
}

    #[test]
    fn shrn_shrn2_decode_shift_narrow() {
        // shrn v28.2s, v31.2d, #16 = 0x0f3087fc ; shrn2 v28.4s, v31.2d, #16 = 0x4f3087fc
        // (both from aarch64-linux-gnu-as). Must decode to SimdShrn with a
        // DOUBLE-width source (esrc=8) and shift=16, NOT a plain equal-size SimdShr.
        match decode(0x0f3087fc) {
            Inst::SimdShrn { rd, rn, esrc, shift, upper, .. } => {
                assert_eq!((rd, rn, esrc, shift, upper), (28, 31, 8, 16, false));
            }
            other => panic!("shrn v28.2s,v31.2d,#16 -> SimdShrn, got {other:?}"),
        }
        match decode(0x4f3087fc) {
            Inst::SimdShrn { rd, rn, esrc, shift, upper, .. } => {
                assert_eq!((rd, rn, esrc, shift, upper), (28, 31, 8, 16, true));
            }
            other => panic!("shrn2 v28.4s,v31.2d,#16 -> SimdShrn, got {other:?}"),
        }
        // plain ushr/sshr must NOT hit the new gate (bit15 clear)
        match decode(0x2f3007fc) {
            Inst::SimdShr { .. } => {}
            other => panic!("ushr v28.2s,v31.2s,#16 -> SimdShr, got {other:?}"),
        }
        match decode(0x4f7007fc) {
            Inst::SimdShr { .. } => {}
            other => panic!("sshr v28.2d,v31.2d,#16 -> SimdShr, got {other:?}"),
        }
    }

    #[test]
    fn shl_immediate_decode_shift_amount_and_esize() {
        // shl Vd.T, Vn.T, #imm: shift = UInt(immh4:immb) - esize_bits, NOT just
        // immb. Real encodings (aarch64-linux-gnu-as):
        //   shl v29.4s, v29.4s, #25  = 0x4f3957bd  (immh=7,immb=1 -> field 57, 32b)
        //   shl v31.4s, v31.4s, #25  = 0x4f3957ff
        //   shl v28.4s, v28.4s, #25  = 0x4f39579c
        // and the .2d case: shl v30.4s... use a 64-bit-lane variant:
        //   shl v0.2d, v0.2d, #33  -> immh=0x9,immb=1 field=73, esize_bits=64,
        //                              shift=9? verify against asm below.
        match decode(0x4f3957bd) {
            Inst::SimdShl { rd, rn, esize, shift } => {
                assert_eq!((rd, rn, esize, shift), (29, 29, 4, 25));
            }
            other => panic!("shl v29.4s,#25 -> SimdShl(29,29,4,25), got {other:?}"),
        }
        match decode(0x4f3957ff) {
            Inst::SimdShl { rd, esize, shift, .. } => {
                assert_eq!((rd, esize, shift), (31, 4, 25));
            }
            other => panic!("shl v31.4s,#25 -> SimdShl, got {other:?}"),
        }
    }

    #[test]
    fn cmgt_not_swallowed_as_vector_add() {
        // Regression: cmgt Vd.2d, Vn.2d, Vm.2d shares byte1-residue 0x2f20_0c00
        // with the integer SIMD add/sub family, but has bit15 CLEAR while every
        // genuine add/sub (and the Simd4s case) has bit15 SET. The SimdAddD/B/H
        // gates lacked Simd4s's `(insn>>15)&1 == 1` guard, so gcc vectorized
        // `a[i] < k ? a[i] : k` (cmgt + bsl min-clamp) silently decoded the cmgt
        // as a vector ADD -> every lane summed instead of compared (silent wrong
        // clamp). Real encodings (aarch64-linux-gnu-as):
        //   cmgt v3.2d, v30.2d, v4.2d = 0x4ee437c3   (bit15 CLEAR)
        //   add  v3.2d, v30.2d, v4.2d = 0x4ee487c3   (bit15 SET)
        //   add  v3.16b,v30.16b,v4.16b = 0x4e2487c3
        //   sub  v3.16b,v30.16b,v4.16b = 0x6e2487c3
        match decode(0x4ee437c3) {
            Inst::SimdCmgt { rd, rn, rm, dword, lanes } => {
                assert_eq!((rd, rn, rm, dword, lanes), (3, 30, 4, true, 2));
            }
            other => panic!("cmgt v3.2d,v30,v4 -> SimdCmgt, got {other:?}"),
        }
        // And the sibling forms still decode (bit15 must NOT exclude them).
        match decode(0x4ee487c3) {
            Inst::SimdAddD { rd, rn, rm, sub } => {
                assert_eq!((rd, rn, rm, sub), (3, 30, 4, false));
            }
            other => panic!("add v3.2d,v30,v4 -> SimdAddD, got {other:?}"),
        }
        match decode(0x6e2487c3) {
            Inst::SimdAddB { rd, rn, rm, sub, .. } => {
                assert_eq!((rd, rn, rm, sub), (3, 30, 4, true));
            }
            other => panic!("sub v3.16b,v30,v4 -> SimdAddB, got {other:?}"),
        }
    }

    #[test]
    fn fp_pairwise_two_register_reduce_ground_truth() {
        // fmaxp/fminp/fmaxnmp/fminnmp Vd, Vn (.2s/.2d): horizontal reduce the
        // two elements of Vn into a scalar in Vd. Ground truth assembled with
        // aarch64-linux-gnu-as and disassembled — all 8 two-register encodings
        // (bit16 CLEAR; the 3-op FMAXP Vd,Vn,Vm has bit16 SET and is NOT this).
        //   fmaxp s0,v1.2s  = 0x7e30f820 ; fmaxp d0,v1.2d = 0x7e70f820
        //   fminp s0,v1.2s  = 0x7eb0f820 ; fminp d0,v1.2d = 0x7ef0f820
        //   fmaxnmp s0,v1.2s= 0x7e30c820 ; fmaxnmp d0,v1.2d= 0x7e70c820
        //   fminnmp s0,v1.2s= 0x7eb0c820 ; fminnmp d0,v1.2d= 0x7ef0c820
        let cases: &[(u32, bool, bool, bool)] = &[
            (0x7e30f820, false, false, false), // fmaxp 2s
            (0x7e70f820, true, false, false),  // fmaxp 2d
            (0x7eb0f820, false, true, false),  // fminp 2s
            (0x7ef0f820, true, true, false),   // fminp 2d
            (0x7e30c820, false, false, true),  // fmaxnmp 2s
            (0x7e70c820, true, false, true),   // fmaxnmp 2d
            (0x7eb0c820, false, true, true),   // fminnmp 2s
            (0x7ef0c820, true, true, true),    // fminnmp 2d
        ];
        for (insn, sz, min, nm) in cases {
            match decode(*insn) {
                Inst::FpPair { rd, rn, sz: d, min: m, nm: n } => {
                    assert_eq!((rd, rn, d, m, n), (0, 1, *sz, *min, *nm));
                }
                other => panic!("{insn:#x} -> FpPair, got {other:?}"),
            }
        }
    }
