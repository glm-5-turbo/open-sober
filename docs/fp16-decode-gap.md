# Decoder gap: real .text has 11,437 Unsupported (FP16/NEON) instructions

**Reproducible scan (scandecode, .text span [0x102d95980, 0x1072d5a84]):**
- total instructions:   13,961,732
- Unsupported hits:     11,437   (was 14,918; **-3,481** this cycle)
- decode() PANIC hits:  0        (decode never aborts — safe)
- distinct opcodes:     6,281     (was 7,743)

**Closed this cycle — scalar FP16 + gate families (commit 4dd5e3d, -1,831) and
SIMD FP16 3-same (commit d284c2d, -1,650).**
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
- Regression tests: decode pins + runtime exec tests (`exec_bytes`) for fcvt h<->s,
  fabd s, urhadd bytes, fadd h, fadd v2.4h (vector) — all green, workspace 421/0.

The idle boot is stable because the engine's *own* main loop never executes these —
the FP16 scalar + gate ops live in the FMOD/audio and render/HDR math paths that only
run once the game drives audio/render. They don't block boot stabilization, but they
WILL JIT-abort (block-tracker breaks at `Inst::Unsupported`) the moment the real boot
reaches them. `0 PANIC` guarantee holds (decode() total).

## Dominant families (capstone ground-truth, instance counts)
```
~2300  fmla  Vd.8h/Vd.4h, Vn, Vm.h[0..7]   (FP16 multiply-add by element)
~500   fadd  Vd.8h/.4h                      (FP16 vector add)
~350   fabd  sD, sN, sM                     (FP32 absolute-diff, scalar)
~250   fcvt  sD, hN   (also sD,hN / hD,sN)  (FP16<->FP32 scalar convert)
~200   fccmp sD, sN, #imm, cond             (FP32 conditional compare)
~150+  urhadd Vd.16b   / uabd Vd.16b       (unsigned rounding/halving int)
~100   srsra Vd.8h                           (rounding shift-right accum, FP16)
~ 90   bic / orr   vector immediate          (0x2f047400 / 0x4f0177eX families)
plus   fmax/fmin/fdiv/fabs/fabd/fcvtas .8h/.4h, fmaxnm/fminnm, fneg, fsqrt,
       fmls .8h, fmul-by-elem, fmlal (FP16 widening), etc.
```
Total distinct by-mnemonic: 1625 (many are just register-id rotations of the
same shape: `fmla v28.8h`/`v29.8h`/`v30.8h` collapse to ONE translator rule).

## Next lever (fmla-by-element .8h — the single biggest remaining family, ~2000)
The scalar-FP16 + gate + 3-same work is done. The dominant remaining family is
**`fmla/fmls/fmul Vd.8h/.4h, Vn, Vm.h[idx]`** (the 0x4f0x_1x2x/1x9x opcodes — ~2000+).
Ground-truth encoding matrix is captured in-hand:
- index = (bit11<<2) | (bit21<<1) | bit20 for .8h (3-bit); .4h uses (bit21<<1)|bit20.
- vm held in bits[19:16] (v0-v15 only, bit20 is index[0]); fp16 fmla has bit23 CLEAR
  (unlike the f32 FmlaEl which requires bit23 SET — that's the discriminator); fmul
  is bit29 SET, fmls is bit14+bit12 SET (same-operand ground truth 0x4f321020 /
  0x4f325020 / 0x4f329020).
- Translate: splat Vm.h[idx] to xmm2 (promote once via F16C), then per-lane
  promote Vn.h[i] -> f32, fma (Vd ± Vn·M in f32), demote -> store. Reuses the
  established F16C pattern; the by-element splat MUST be hoisted before the lane
  loop (rd==rm case clobbers the element).
Then: SIMD-immediate orr/bic v.2s/.4s (~200), vector fcvtas v.4s (~150), fabs v.2s.

## Artifact
- Scan raw: `/home/hermes-worker/runs/open-sober/docs/fp16-scan/.text.unsupported.txt`
  (opcode x-count x-first@addr table, sorted by count).
- This doc: `docs/fp16-decode-gap.md`.
- Reproduce: `cargo run -p arm64jit --example scandecode -- <libroblox.so>
  0x102d95980 0x1072d5a84`