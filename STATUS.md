# Long-Run Goal — Status Ledger

goal: run Roblox through the Open-Sober runtime (specialized runtime, graphics,
sound) — the full "Roblox boots on the JIT/no-QEMU path" gate.
started: 2026-09-11T01:20:42Z

## Cycle (Sep 11, 2026) — SIMD lane-insert/extract fix (INS/SMOV/UMOV)
status: session-end (committed, tests green)
last_agent_claim: fixed a real silent miscompile in arm64jit's SIMD lane ops,
verified end-to-end via the cross-gcc/asm elfjit harness. Commit 0f2d806.

### What landed
`mov v0.s[i],w1` (INS: GPR->vector-element insert) and `smov`/`umov` (element
extract to GPR) share decode fields with the vector-logical / saturating-add
classes. The precise lane-op gate ran AFTER those loose gates, so a real INS
was silently decoded as AND (never wrote the vector, clobbered x0) and SMOV as
SQSUB — silent GPR/vector corruption on NEON-heavy graphics/audio paths.
Fix: move the lane-op gate before the broad vector gates and key on the
bits[13:12] opcode field (bit13=1 extract, bit13=0+bit12=1 insert w/ new
Inst::InsGp, bit13=0+bit12=0 dup->fall through to SimdDupGp). SimdLaneGp
translate now covers all element sizes 1/2/4/8.

### Verified
- cargo build --workspace: ok
- cargo test --workspace: 119 passed / 0 failed (arm64jit 85: +2 regressions this cycle)
- elfjit harness: lane_test.elf / smov.elf return -570 (=0xfffffffffffffdc6, was 0);
  and/orr/eor/bic/sqadd/sqsub still decode+execute as their real ops
  (logical.elf runs them all, stops honestly at not-yet-implemented uaddl);
  dup v1.4s,w10 still SimdDupGp; prior battery (loop1 45, byvalue 44, iso_wrd
  4321, vtable 42, structs/fpfun/dispatch 42, bv2 300) all still correct.
- HEAD: 0f2d806 (local dev branch)

### Blocked / not satisfiable on this host (HARD GATE still unmet)
Roblox still does NOT boot on an actual capability host. Requires:
- the real libroblox.so / Roblox APK (not present on this VPS), and
- a GPU-capable host to run graphics/audio.
Until then the work is: fix JIT/stub/loader correctness, commit, advance code-level.

### Next (ordered)
- SIMD FP-vs-int lane ops beyond the element insert/extract family (uaddl/saddl
  widening-multiply, etc. as surfaced by the logical.elf battery stop).
- libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
- Real-binary/GPU boot proof (capture `elfjit <libroblox.so> 0x1f0db20 --jni`
  on a capable host).