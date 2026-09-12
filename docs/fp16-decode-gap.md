# Decoder gap: real .text has 13,087 Unsupported (SOC/FP16/NEON) instructions

**Reproducible scan (scandecode, .text span [0x102d95980, 0x1072d5a84]):**
- total instructions:   13,961,732
- Unsupported hits:     13,087   (was 14,918; -1,831 this cycle)
- decode() PANIC hits:  0        (decode never aborts — safe)
- distinct opcodes:     7,225     (was 7,743)

**Closed this cycle (commit …): the scalar FP16 + real scalar-FP gate families.**
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
- Regression tests: decode pins + runtime exec tests (`exec_bytes`) for fcvt h<->s,
  fabd s, urhadd bytes, fadd h (F16C path) — all green, workspace 419/0.

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

## Why hand-translating all of it is the wrong immediate move
7743 distinct encodings over ~30 FP16/FP32 shapes. The existing translator builds
per-lane host x86 inline (promote h->f32, op, demote) — correct but ~30 new
translation arms + decode gates, high regression risk against a clean 411-test
workspace in one cycle. The idle boot doesn't execute these, so there is no
*boot* pressure to close them this cycle.

## Bounded next-lever (when decoder coverage is the priority)
1. **FP16 scalar convert first** (fcvt s,h / h,s / d,h / h,d — ~250, few distinct
   encodings): reuse the existing `FpScalar`/`SimdFrint` scalar-promote machinery
   (VECTOR_BASE low-8B slot). Smallest-correct, immediately testable.
2. **fmla-by-element .8h/.4h** (~2300, the single biggest family): fold every
   `fmla v*.8h, v*, v*.h[n]` into one decoder gate + one upconvolve→fmadd→demote
   translation arm (reuse `SimdFmulEl` pattern but esize 2 → promote lanes).
3. urhadd/uabd (byte) — reuse the SIMD int reduce/halve arms; esize 1, no FP16 math.
4. fabd s / fccmp / fadd — FP32 already well-supported; these are scalar/vector
   gaps in the esize-2 float path.
After each: re-run `scandecode` on .text and assert the Unsupported count drops
monotonically; keep the `0 PANIC` invariant.

## Artifact
- Scan raw: `/home/hermes-worker/runs/open-sober/docs/fp16-scan/.text.unsupported.txt`
  (opcode x-count x-first@addr table, sorted by count).
- This doc: `docs/fp16-decode-gap.md`.
- Reproduce: `cargo run -p arm64jit --example scandecode -- <libroblox.so>
  0x102d95980 0x1072d5a84`