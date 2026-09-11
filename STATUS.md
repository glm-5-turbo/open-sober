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

## Cycle (Sep 11, 2026) — SIMD/sysreg correctness sweep (4 real bugs + 3 walls)
status: session-end (committed, 127/0 tests green)
last_agent_claim: fixed four real silent miscompiles and crossed three ISA walls
in arm64jit, each driven through the cross-gcc battery + qemu and locked with
regressions.

### What landed
1. SimdShrAcc (usra/ssra) esize/shift decode bug: derived esize from the 3-bit
   tagless immh via trailing_zeros, so every esize>=4 shift decoded as
   esize=1/shift=0 — ssra accumulated WITHOUT shifting (ssra .2d #2 gave
   -8-16=-24 not -2-4=-6; .4s #2 gave 72 not 18). Now mirrors SimdShr's
   full-immh (bit22) esize + 2*esize_bits shift; unsigned discriminator also
   fixed to bit29 (was bit11).
2. neg (rn=31) XZR-vs-SP in AddSubReg: `neg xd,xm` (shifted-register sub) must
   read rn=31 as XZR=0, but the translate read SP for every non-S rn=31. bit21
   distinguishes shifted (31=XZR) vs extended (31=SP) forms — new `sp_operand`
   field on Inst::AddSubReg.
3. SysReg MRS reads were silent no-ops (latent): translate wrote
   buf.mov_ri64(rt,..) with rt a GUEST reg index, committing to a stray host
   reg, never the guest file. All mrs xN,<cntfrq|cntvct|nzcv|dczid|tpidr>
   returned garbage/0. Fixed all reads to load RAX then stg into the guest slot.
4. New: dczid_el0 (glibc CRT sizes DC ZVA; returns 0x4). New: umulh/smulh
   (high 64 of 128-bit product), gate top 0x9b && bit22 set, signed=bit23.

### Verified
- cargo build --workspace: ok; cargo test --workspace: 127/0 (arm64jit 93).
- Battery (all qemu-verified): ssra_2d=-6, ssra_4s=18, ushr=1, shl=24,
  shlimm=27, neg_d=2, cf(cntfrq)=100000000, dz(dczid)=4, mulh_e=2.
- modmain.elf (full glibc CRT) now advances past dczid + umulh/smulh to the
  next wall: MTE `stg x0,[x0]` (0xd9200800, __libc_mtag_tag_region).

### Next (ordered, HARD GATE still unmet on this host)
- MTE stg/ldg memory-tagging no-op (unblocks full glibc-linked programs).
- Continue the SIMD surface as the battery reveals it; then svc routing on use.
- libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
- Real-binary/GPU boot proof on a capable host + libroblox.so/APK.

### Addendum (same cycle) — data/instruction cache maintenance (dc/ic) no-ops
- `Inst::CacheMaintain{d zva: bool, rt}`: cache clean/invalidate/coherence ops
  (`dc gva/civac/ivac`, `ic ivau`, etc., top byte 0xd5, CRn=7) are no-ops in the
  direct-mapped single-threaded JIT — EXCEPT `dc zva` which zeros the 16-byte
  cache line at [Xt] (the block size dczid_el0 advertises). This unblocks
  full glibc-linked programs whose CRT `__libc_mtag_tag_region` ends with a
  `dc` op (modmain.elf stopped at 0x40c174 on `dc gva`).
- Two bugs fixed while landing it: (1) the session's draft used wrong encodings
  in tests (0xd50b7400 is a `sys`, not `dc zva`; real `dc zva x0`=0xd50b7420);
  (2) Rt was decoded from bits[9:5] instead of bits[4:0], so `dc zva x0` wrote
  the zero via guest x1 -> segfault. zva discriminator = CRm==4 && op2==1,
  objdump-verified across gva/civac/ivac/ivau. Regression in jit.rs.
- arm64jit 95, workspace 129/0.

# Session — guest auxv bootstrap + SME/SVE/MulLong decodes + zero-extend fix (Sep 11 2026)

Goal (no APK/GSI/GPU on this box): prove the JIT can boot a full statically
linked glibc aarch64 binary (`modmain.elf`: `main(){return 300%16;}`), which
exercises the whole CRT + loader + syscall gate. Real, verified progress:

## boot.rs — guest auxv on the initial stack (loader correctness)
The kernel's aarch64 process-start ABI puts `[argc][argv][envp][auxv AT_NULL]`
on the stack with sp at argc; glibc's static `_start` walks it for envp/auxv.
The JIT previously left garbage there, so glibc read bogus AT_HWCAP2 and
`_dl_hwcap2` landed with HWCAP2_SME (bit 23) set, driving `__libc_arm_za_disable`
into its ZA-store loop (an unsupported `str za` wall). New `arm64jit::boot`:
`standard_auxv(&LoadedElf, hwcap, hwcap2)` (AT_PHDR/PHENT/PHNUM/PAGESZ/ENTRY/
BASE/HWCAP/HWCAP2/CLKTCK/RANDOM) + `layout_initial_stack()` (builds the image,
fills AT_RANDOM from a bounded xorshift). Wired into both `elfjit` and
`sober-core::jit`. `_dl_hwcap=0 _dl_hwcap2=0 __aarch64_have_sme=0` verified.

## SME/SVE feature-off decodes (compile the CFG past __libc_arm_za_disable)
Even with SME off, glibc's ZA block is *linearly* reachable in the compiled CFG
(the block compiler follows fall-through past the data-dependent SME gate), so
its instructions had to DECODE:
- `Inst::SmeNoop` — `str za[Wt,k],[Xn,#k,mul vl]` (0xe1206200..f) + smstart/smstop
  za (0xd5034000). No-ops: SME is never enabled in this guest.
- `Inst::AddVectorLen` — SVE addvl/addsvl (gate 0xffe0_f000==0x0420_5000; Rn=
  bits[20:16], imm6=sext[10:5]); translate `Xd = Xn + imm6*16` (model VL=16B).
- `Inst::SveCntd` — SVE cntd (0x04e0_e000) -> rd=2 (VL_d/64 at VL=16B).
- `mrs xN, midr_el1` (sysreg 8) -> 0 (unknown core, generic glibc paths).

## Inst::MulLong — smull/umull/smaddl/umaddl/smsubl/umsubl
32x32->64 multiply-long family, found in glibc's `_dl_fixup` (dynamic IFUNC
resolution). Gate top 0x9b & bits[22:21]==01 (disjoint from MulHigh bit22 and
64-bit madd bit21). bit23=signed, bit15=sub. Verified vs real assembler.

## 🔧 Latent x86-emitter bug: `and_ri64(_, 0xffffffff)` was a NO-OP
`and r64, imm32` sign-extends the imm, so `and rax, 0xffffffff` = `and rax,
0xFFFFFFFFFFFFFFFF` = no-op — 10 sites meant to zero-extend a W (32-bit) value
instead silently leaked uninitialized high bits (the glibc startup x3/x0
corruption). Added `CodeBuf::zero_ext_r32` (`mov r32,r32`, clears upper 32)
and replaced all 10 uses. Proven by new umull/smull/umsubl/addvl exec tests.

## Verification
- `cargo build --workspace` clean; `cargo test --workspace` **137/0**
  (arm64jit 103, +5: multiply_long_family_ground_truth, addvl_cntd_sme_midr_
  ground_truth, mullong_umull_exec, mullong_smull_and_msubl_exec,
  addvl_scales_by_16_bytes).
- `modmain.elf` boot now advances past __libc_arm_za_disable + _dl_fixup + midr
  (previously stopped at `str za`), then hits a further glibc-startup crash
  (x-register high-bit corruption on the auxv scan / __libc_start_main prologue;
  the zero-extend fix is in but not sufficient). qemu still returns 12; JIT does
  not yet boot it to completion.

### Next (ordered)
1. Finish the glibc-startup corruption (localize the leftover W-reg high-bit
   leak / bad-computation feeding the __libc_start_main auxv scan).
2. Then modmain.elf -> exit 12 proves full-statical-glibc boot through the JIT.
3. libloader ELF/loader gaps -> libbadcpu gaps -> services/auth.
4. HARD GATE remains: real libroblox.so/APK + GPU host run (`elfjit <lib> 0x1f0db20 --jni`).

Commit summary: new boot.rs (auxv), SmeNoop/AddVectorLen/SveCntd/MulLong decodes
+ translate, sysreg 8 (midr), zero_ext_r32 fix, auxv wired into elfjit + sober-core.
---
