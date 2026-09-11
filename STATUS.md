# Long-Run Goal — Status Ledger

goal: run Roblox through the Open-Sober runtime (specialized runtime, graphics,
sound) — the full "Roblox boots on the JIT/no-QEMU path" gate.
started: 2026-09-11T01:20:42Z

## Cycle (Sep 11, 2026) — SIMD NEON widening/lane-op correctness sweep
status: session-end (committed, tests green)
last_agent_claim: four real silent miscompiles fixed in arm64jit's SIMD path,
each driven through the cross-gcc asm battery via `elfjit` and regression-locked.
Commits 0f2d806 (lane ops), 80d27e5 (add-sub-long), d052723 (multiply-long),
c6eb8df (shift-right + sshr/ssra sign-ext).

### What landed
1. INS/SMOV/UMOV lane ops (0f2d806): `mov v0.s[i],w1` (GPR->vector insert) and
   smov/umov (extract) shared decode fields with vector AND/ORR/EOR/BIC and
   SQADD/UQSUB, and the precise lane gate ran AFTER those loose gates, so a real
   INS decoded as AND (never wrote the vector, clobbered x0) and SMOV as SQSUB.
   Moved the lane gate first; keyed on bits[13:12] (bit13=1 extract, b13=0+b12=1
   insert = new Inst::InsGp, b13=0+b12=0 dup -> SimdDupGp). LensGp translate now
   covers esize 1/2/4/8.
2. ADDL saddl/uaddl/subl/usubl (80d27e5): gate missed esrc=4 (.2d) and esrc=1
   (.8h) residues; translate read/wrote the wrong lane widths. Full 24-residue
   gate + exact-width per-lane load/store.
3. MULL smull/umull/smlal/umlal (d052723): FIVE silent miscompiles — res_esize
   (bit22, missed .8h), unsigned (bit28, never true -> umull sign-extended),
   acc (bit15, plain smull accumulated), translate lanes (missed 8 lanes) and
   non-exact store width.
4. ushr/sshr (c6eb8df): plain shift-right had no gate and was swallowed by the
   VecMovi gate (wrote a wrong immediate instead of shifting); immutable the same
   sign-extend bug in SimdShr/SimdShrAcc (sshr/ssra shifted negative elements
   positive). New SimdShr (immh!=0 gate) + exact esize/shift from fls(immh).

### Verified
- cargo build --workspace: ok
- cargo test --workspace: 122 passed / 0 failed (arm64jit 88; +5 regressions this cycle)
- elfjit harness: lane_test.elf / smov.elf return -570 (was 0);
  logical/addl/mull/shifts/shiftimm all run to completion and return correct
  values (shiftimm 3, was 0xfffffffc; sshl .2d 256); smull .2d {7,-3}*{5,-2}={35,6};
  umull .8h 0xFE*2=508 (proves unsigned fix); prior battery (loop1 45, byvalue
  44, iso_wrd 4321, vtable 42, structs/fpfun/dispatch 42, bv2 300) unchanged.
- HEAD: c6eb8df (local dev branch)

### Blocked / not satisfiable on this host (HARD GATE still unmet)
Roblox still does NOT boot on an actual capability host. Requires:
- the real libroblox.so / Roblox APK (not present on this VPS), and
- a GPU-capable host to run graphics/audio.
Until then the work is: fix JIT/stub/loader correctness, commit, advance code-level.

### Next (ordered)
- Continue the SIMD surface as the cross-gcc battery reveals it (shift-by-imm,
  tbl, dup .b, post-index SIMD ld; then svc syscall routing on real use).
- libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
- Real-binary/GPU boot proof (capture `elfjit <libroblox.so> 0x1f0db20 --jni`
  on a capable host).