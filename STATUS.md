# Long-Run Goal — Status Ledger

goal: run Roblox through the Open-Sober runtime (specialized runtime, graphics,
sound) — the full "Roblox boots on the JIT/no-QEMU path" gate.
started: 2026-09-11T01:20:42Z

## Cycle (Sep 11, 2026) — JIT ABI correctness sprint
status: session-end (committed, tests green)
last_agent_claim: fixed four real silent miscompiles in arm64jit found via a real
  cross-gcc battery run through `elfjit`. 116 workspace tests green (arm64jit 82),
  battery 12/12. Commits b8b5e62, e5d78d3 (code) + docs.

### What landed
1. LdStPair offset-form ignored its immediate -> struct-by-value reads garbage.
2. ADD/SUB rn==31 read SP when S set (`negs w1,w0` = `sp-w0` not `-w0`).
3. 32-bit W writes did not zero-extend (`mov w0,w1` carried 0xffffffffffffffff).
4. scalar fcvtzu saturated at 2^63-1 instead of the correct u64 over [0,2^64).
Plus JIT_BUDGET debug knob and a bounded-truncation fall-through pc bug.

### Verified
- cargo build --workspace: ok
- cargo test --workspace: 116 passed / 0 failed (was 113 before; +3 regressions)
- cross-gcc battery via elfjit: loop1 45, structs/dispatch/fpfun/vtable 42,
  byvalue 44, bv2 300, signmod 12, iso_wrd 4321, iso_arith 300 — all correct
- HEAD: e5d78d3 (local dev branch)

### Blocked / not satisfiable on this host (HARD GATE still unmet)
Roblox still does NOT boot on an actual capability host. Requires:
- the real libroblox.so / Roblox APK (not present on this VPS), and
- a GPU-capable host to run graphics/audio.
Until then the work is: fix JIT/stub/loader correctness, commit, advance code-level.

### Next (ordered)
- FcvVec 4S-lane FP->int conversion (uses movq/cvttsd2si on 4-byte lanes)
- SIMD SMOV/UMOV lane->GPR + remaining FP-vs-int lane ops
- libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth
- Real-binary/GPU boot proof (capture `elfjit <libroblox.so> 0x1f0db20 --jni`)