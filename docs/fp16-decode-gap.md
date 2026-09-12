# Decoder gap: real .text has 11,437 Unsupported (FP16/NEON) instructions

**Reproducible scan (scandecode, .text span [0x102d95980, 0x1072d5a84]):**
- total instructions:   13,961,732
- Unsupported hits:     9,552    (was 11,437; **-1,885** this cycle)
- decode() PANIC hits:  0        (decode never aborts — safe)
- distinct opcodes:     5,735     (was 6,281)

**Closed this cycle — scalar FP16 + gate families (commit 4dd5e3d, -1,831), SIMD
FP16 3-same (commit d284c2d, -1,650), and FP16 by-element fmla/fmls/fmul
(commit f3251b7, -1,885).**
- **Scalar FP16 convert** (`fcvt s,h / h,s / d,h / h,d`) — new `FcvtHalf`, translated via
  F16C (`vcvtph2ps` / `vcvtps2ph $0` RN, emitted as raw VEX bytes; host confirmed f16c).
- **Scalar FP16 arithmetic** (`fadd/fmul/fsub/fdiv h`) — extended `FpScalar` with a
  `half` flag (the 0x1eE0/bit23-set global); promote->op->demote via F16C inline.
- **`fccmp s/d` decode gate FIXED** — the old `(insn & 0xfff0_fc03) == 0x1e20_c400`
  never matched ANY real fccmp (mask/constant mismatch). Correct structural gate
  `(insn & 0xffe0_0c10) == {0x1e200400 s, 0x1e600400 d}`. MUST decode before the
  FMOV-imm gates: a cond≥8 sets bits[15:12] (e.g. `lt`=0xb) that the FMOV-imm
  `(insn & 0x1000) != 0` lane anchor read as an immediate → wrong FmovImm. Exposed
  by a regression test on the real `fccmp s0,s1,#0,eq` / compiler `lt`/`ne` forms.
- **`fabd s`** — added the single-precision form `0x7ea0_d400` (real `fabd s2,s2,s3`
  = 0x7ea3d442); `Fabd` gained a `sz` field; single path `subss`+clear bit31.
- **`urhadd v.16b/.8b`** — new `Urhadd` (unsigned rounding-halving add `(a+b+1)>>1`,
  byte-lane), pure integer per-byte.
- **SIMD FP16 3-same `fadd/fsub/fmul Vd.8h/.4h`** (commit d284c2d) — new `SimdFp16As`,
  per-half-lane promote->op->demote via F16C. Gate `(insn & 0x9f60_f400) == 0x0e40_1400`;
  op from bit29 (fmul) / bit23 (fsub). MUST decode BEFORE the SIMD-select (bsl) gate:
  the FP16 `fmul Vd.8h` opcode (byte1 0x1c, e.g. 0x6e451c82) aliases bsl's byte1 0x1c
  mask and was silently decoded as a bitwise select until a regression caught it.
- **SIMD FP16 by-element `fmla/fmls/fmul Vd.8h/.4h, Vn, Vm.h[idx]`** (commit
  f3251b7, -1,885) — new `SimdFp16BEl`. Gate byte0-nibble 0xf + bit29 CLEAR +
  bit23 CLEAR + **bit10 CLEAR**. The bit10 discriminator is the critical new
  finding: FP16 indexed (bit10=0) vs the shift-by-immediate family
  (shl/ushr/sshr/usra/ssra/srshr/srsra... bit10=1 FIXED) which shares byte0
  prefix AND bit23=0 AND aliases the FMUL/FMLA/FMLS opcode bits[15:12] onto the
  shift's [14:12] marker (fmul=bit15, fmls=bit14, fmla=bit12==usra's 0b001).
  bit13 CLEAR excludes widening by-element mul (SimdMullEl, requires bit13 SET);
  bit23 CLEAR excludes f32 FmlaEl/SimdFmulEl (bit23 SET). MUST precede the shift
  gates. op = bit15(fmul)|bit14(fmls)|else fmla; idx (.8h) = (bit11<<2)|(bit21<<1)|bit20,
  .4h = (bit21<<1)|bit20; vlm = bits[19:16] (v0-v15 only). Translate: hoist-splat
  Vm.h[idx]->xmm2 (promote once via F16C BEFORE the lane loop so rd==rm can't
  clobber), per-lane promote Vn.h[i]->f32, fma in f32, demote->store16. Vd MUST
  be promoted to f32 before the accumulate (Vd is fp16, unlike the f32 FmlaEl
  path); fmul copies xmm1->xmm0 so all ops demote the same xmm0.
- Regression tests: decode pins + runtime exec tests (`exec_bytes`) for fcvt h<->s,
  fabd s, urhadd bytes, fadd h, fadd v2.4h (vector) + fmla/fmls .4h + fmul .8h (8-lane)
  — all green, workspace 423/0 (was 421).

The idle boot is stable because the engine's *own* main loop never executes these —
the FP16 scalar + gate ops live in the FMOD/audio and render/HDR math paths that only
run once the game drives audio/render. They don't block boot stabilization, but they
WILL JIT-abort (block-tracker breaks at `Inst::Unsupported`) the moment the real boot
reaches them. `0 PANIC` guarantee holds (decode() total).

## Dominant families (decode-verified scan, instance counts)
```
~2300  fmla  Vd.8h/Vd.4h, Vn, Vm.h[idx]   (FP16 multiply-add by element)  [CLOSED f3251b7]
~500   fadd  Vd.8h/.4h                      (FP16 vector add)            [CLOSED d284c2d]
~350   fabd  sD, sN, sM                     (FP32 absolute-diff, scalar) [CLOSED 4dd5e3d]
~250   fcvt  sD, hN   (also hD,sN)          (FP16<->FP32 scalar convert)[CLOSED 4dd5e3d]
~200   fccmp sD, sN, #imm, cond             (FP32 conditional compare)   [CLOSED 4dd5e3d]
~150+  urhadd Vd.16b   / uabd Vd.16b       (unsigned rounding/halving int)[CLOSED 4dd5e3d]
~100   SIMD-immediate orr/bic v.2s/.4s      (0x4f0177eX etc — NEXT lever)
plus   fmax/fmin/fmaxnm/fminnm, fcvtas/fcvtzu, fneg, fsqrt, srsra,
       vector-immediate bic/orr (0x2f047400), fmlal (FP16 widening), etc.
```
Total distinct by-mnemonic: 1625 (many are just register-id rotations of the
same shape: `fmla v28.8h`/`v29.8h`/`v30.8h` collapse to ONE translator rule).

## Next lever (SIMD-immediate orr/bic v.2s/.4s — ~100 real hits, 0x4f0177eX)
The fmla-by-element family is CLOSED (f3251b7). The next-largest remaining
families are the **vector modified-immediate** ops:
- `orc/bic/orr/mvni/movi Vd.2S/.4S` encoded 0x4f0177eX / 0x2f047400 (cmode-based
  128/64-bit immediate; ~sum 100+ real instances) — currently falling through
  to `Unsupported` (or occasionally a wrong decode). Same AArch64 "vector ORR/
  BIC/mov immediate" shape the existing VecMovi already handles for byte/word
  cmodes; extend it for the .2S/.4S cmode-0001 shifts.
- `fmaxnm/fminnm Vd.4S/.2S/.4H` (0x4e21c8xx / 0x4ea0xxxx, ~30) — FP min/max
  (maxnm propagates NaN, minnm ignores; x86 maxss/minss differ only on NaN, a
  documented approximation).
- vector `fcvtas/fcvtzu Vd.4S, Vn.4S` (~150) — FP16/FP32 vector float->int
  with round-to-nearest.
- scalar `fabs/fneg/fsqrt Vd.2S/.4S` and `fmax/fmin` (0x0ea00xxx / 0x0ea0f8xx).

## Artifact
- Scan raw: `/home/hermes-worker/runs/open-sober/docs/fp16-scan/.text.unsupported.txt`
  (opcode x-count x-first@addr table, sorted by count).
- This doc: `docs/fp16-decode-gap.md`.
- Reproduce: `cargo run -p arm64jit --example scandecode -- <libroblox.so>
  0x102d95980 0x1072d5a84`