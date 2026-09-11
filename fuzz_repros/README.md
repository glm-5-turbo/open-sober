# Fuzz repro artifacts

## fp_edge_9000_58.c / fp_edge_9000_21.c — OPEN residual bug
Found by `gen_fp_edge` (FP-edge differential generator, added this session).
Both oracles (native x86 gcc AND qemu-aarch64) agree: result 2039651.
JIT returns 2474795 (diff = +435144 = exactly a[5]*1e6).

Diagnosis chain (all isolated and CLEAN individually):
- scalar fcvtzs (+/-inf, NaN): JIT==native x86 (INT64_MIN for inf/NaN); qemu
  differs (ARM: +inf->INT64_MAX, NaN->0) but that is NOT this bug (native oracle
  agrees with JIT, so the fuzzer can't see it; a genuine ARM-vs-x86 FP->int
  saturation difference in FcvtTzReg/FcvVec which ARM semantics require fixing).
- mn/mx (fcsel select, NaN operand): INT64_MIN in JIT, matches native.
- back = (long long)((double)ivals): correct 2039600 when isolated.
- The full program is the ONLY trigger: gcc jointly schedules the SIMD
  `fcvtzs v.2d` (in-place, 8 int lanes -> addp sum) INTERLEAVED with the scalar
  fcsel d-reg min/max and the cmgt+bsl clamps, sharing dN/vN vector slots.
  Under that exact allocation the JIT's host register file clobbers one value,
  producing the +435144 drift. Only reproducible from this exact .c (removing
  ANY of lo/hi, mn/mx, back, or ivals&0xff makes it pass), i.e. a block-compiler
  register-scheduling bug. Need a single-instruction host-emission diff (JIT_STEP
  is block-granular) to nail which translate arm's scratch collides.
Run: cargo build -p arm64jit --example elfjit; elfjit <(compile this at -O3
-aarch64 -static -nostdlib -Wl,-e,entry) <entry> ; diff vs native+gcc / qemu.
