# Long-Run Goal — Status Ledger

goal: run Roblox through the Open-Sober runtime (specialized runtime, graphics,
sound) — the full "Roblox boots on the JIT/no-QEMU path" gate.
started: 2026-09-11T01:20:42Z

## Cycle (Sep 11, 2026) — randomized differential fuzz: 8 JIT correctness fixes (workspace 289/0, ~500 cases)
status: session-end (committed, tests green)
last_agent_claim: Persisted `fuzz_jit.py` (elfjit-vs-native-oracle differential
fuzzer over scalar/SIMD shift/xor/byte/fp/2D/3D/bitfield/saturating/mul, ~500
cases green seeds 1-11,99) and fixed EIGHT silent JIT miscompiles, each
regression-guarded + sensitivity-verified: ld2/st2 esize deinterleave
(36813d1); W-form bitfield high-32 mask (b739a6b); shrn/shrn2 shift-right-narrow
decode (04a1483); rbit 64-bit-mask width + clz REX.W-before-F3 byte order +
shl#imm decode (b83a1cf); UBFM extract mask sign-extension + ubfx-vs-ror
discriminator (750457a). cargo build clean; cargo test --workspace 289/0 (was
280). Real-boot HARD GATE unchanged — no GPU/APK/libroblox.so on this box.
updated: 2026-09-11T09:55:00Z
---

## Cycle (Sep 11, 2026) — guest_svc syscall surface: fstat/newfstatat (guest-layout), sockets, epoll, and 15+ more (workspace 270/0)
status: session-end (committed, tests green)
last_agent_claim: fstat(80)/newfstatat(79) with a hand-transcribed AArch64
`stat` layout (asm-generic, 128B: st_dev@0 st_ino@8 st_mode@16 st_uid@24
st_gid@28 st_rdev@32 st_size@48 st_blksize@56 st_blocks@64 times@72..112) —
the host libc::stat layout DIFFERS across arches, so forwarding the host struct
would silently mis-place every field; write_guest_stat converts. Also added
readv(65)/writev(66), uname(160), gettimeofday(169), clock_getres(114),
dup(23)/dup3(24), ioctl(29), eventfd2(19), epoll_create1(20)/epoll_ctl(21)/
epoll_pwait(22), ppoll(73), socket(198)/bind(200)/listen(201)/accept(202)/
connect(203)/setsockopt(208)/getsockopt(209), getrlimit(163)/setrlimit(164),
kill(129)/tgkill(131), timer_create(107)/timer_settime(110). All numbers
verified against /usr/aarch64-linux-gnu/include/asm-generic/unistd.h +
asm-generic/stat.h (not guessed). These are the syscalls a real Android
boot/ALooper/login-path needs that the table lacked. New integration test
guest_svc_stats_and_descriptors_roundtrip exercises fstat/newfstatat st_size+st_mode
into the guest-layout buffer, eventfd write/read, epoll_create1+epoll_ctl(ADD on
an eventfd/pipe — regular files EPERM), gettimeofday, uname=="Linux".
cargo build clean; cargo test --workspace 270/0 (was 269). HARD GATE unchanged
(no GPU/APK/libroblox.so on this VPS).
---

## Cycle (Sep 11, 2026) — EXTR operand-order inversion + ADC/SBC carry polarity fixed (2 REAL silent miscompiles; workspace 272/0, commit 912eff8)
status: session-end (committed, tests green)
last_agent_claim: Added differential canary diff_math128_and_carry (__int128
sq/mul/madd) + diff_switch_fnptr_hash (switch jump table / fnptr dispatch / FNV)
-> the math128 probe FAILED (JIT 0xc24b01e838c56079 vs native 0xb50f76ac635ab31b).
Bisected to TWO distinct real bugs in arm64jit, both now fixed & native-verified:

1. **EXTR operand order INVERTED.** ARM `extr Xd,Xn,Xm,#lsb` = (Xn << (64-lsb))
   | (Xm >> lsb): Xn is the HIGH word, Xm the LOW word. The translator did the
   opposite. The rotate alias rn==rm is symmetric so it hid the bug (all prior
   ror/rotate tests passed); gcc's real 128-bit cross-word shifts (e.g.
   `extr x1, x2, x1, #32`) got operands swapped -> garbage. dp.c d>>63 went
   0x2ea61d950eca8651 -> 0x59e26ad155555533 == native x86. A standalone
   force_extr probe (extr#32) alone also reproduces: was 0x1111deadbeef (wrong),
   now 0xcafe000012341234 (right).

2. **ADC/SBC carry polarity.** store_nzcv stores the C flag in the b.cond
   "borrow" convention (!carry-out for adds, borrow for subs) so the TRUE ARM
   carry consumed by adc/sbc is !stored in BOTH cases. adc now cmc's the loaded
   C before the native adc_carry (and cmc's back before store_nzcv in the s-case
   so a later b.cond reads borrow-sense); sbc drops its leading cmc (sbb consumes
   1-TrueC = stored C directly). Runtime adds+adc (adc3: 0xffffffffffffffff+5)
   returned 0xb, now 0xc. add_carry_reference unit test updated to the
   borrow-convention nzcv encoding.

math128 now EXACTLY matches the native oracle. diff_switch_fnptr_hash (jump
tables / blr dispatch / FNV) was already clean. cargo build clean; cargo test
--workspace 272/0 (was 270). HARD GATE unchanged (no GPU/APK/libroblox.so).
---

## Cycle (Sep 11, 2026) — SIMD across-lanes min/max + smax/smin & movsx fixes (workspace 268/0)
status: session-end (committed, tests green)
last_agent_claim: new int SIMD min/max differential canary (im_running_minmax)
exposed one missing ISA and two silent bugs: (1) SMINV/SMAXV/UMINV/UMAXV
across-lanes reduce was unimplemented -> added Inst::SimdReduceMinMax
(CMOVcc-reduce); (2) element-wise smax/smin decoded as the wrong op for high
source regs because the max assignment was exact `b2 == 0x64` while the gate
masks (b2's low 2 bits carry Rn; real gcc smax b2=0x67 -> decoded MIN, returned
Vn verbatim: canary -2600 vs 3640) — now masks; (3) movsx_word_mem/byte_mem
lacked REX.W so negative 8/16-bit lanes compared as huge positive u64 in signed
reductions — both emitters fixed to REX.W. +2 canaries, +2 exec regressions.
cargo build clean; cargo test --workspace 268/0, 0 ignored. HARD GATE unchanged
(no GPU/APK/libroblox.so on this VPS).

## Cycle (Sep 11, 2026) — three silent FP `.2d`/FMOV-imm decode collisions fixed (workspace 264/0)
status: session-end (committed, tests green)
last_agent_claim: extended the differential battery (4 new game-critical probes:
SIMD fmin/fmax reduction, reciprocal/division funnel, integer widening mul-acc,
double FP loop-condition); `l_double_div_accum_guard` failed (2001 vs 8820) and
bisecting it exposed THREE silent FP decode collisions: (1) fadd/fmul/fsub Vd.2D
was compiled as integer `paddw` by the Simd4s/AddD/AddB/AddH gates (bit14 not
masked); (2) fdiv/fmul Vd.2D was compiled as `pandn/pand` bsl by the SimdSel
gate (bits[15:13] not masked); (3) `fmov d,#imm` 12.0-15.0 (bit12 SET) was
swallowed by the coarse fcvt-to-int round gate (0xffff_0000 top match on
`fcvtau 0x1e65`). Fixed with bit14/bit15:13/bit12 guards on those gates; all
verified vs the real aarch64 assembler; +1 decode +2 exec regressions + the 4
canaries now permanent. cargo build clean; cargo test --workspace 264/0, 0
ignored. HARD GATE unchanged (no GPU/APK/libroblox.so on this VPS).

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

### Continued (same session) — post commit 5d40fdd: three more real JIT bugs (142/0)
- Inst::AddSubExt (extended-reg add/sub): bit21=1 was decoded as `lsl#<garbage>`;
  proper opt/shift semantics + SP.
- Inst::LdStrImmWb (pre/post-index + unscaled LDR/STR): the register-offset gate
  too broad -> NULL deref on `ldr x,[x],#8`; narrowed + signed-imm9 writeback.
- Inst::LseAtomic (ldadd/ldclr/ldeor/ldset/swp): glibc IFUNC atomics decoded +
  single-threaded emulation.
- modmain.elf now runs __libc_start_main into __tunable_get_val, then dies on a
  run-varying-high-garbage address (next bug: a residual 32-bit/zero-extend leak).
- cargo test --workspace 142/0; elfjit keeps a permanent SIGSEGV diagnostic.
---
### Final (same session) — LSE atomics; modmain -> __tunable_get_val block-register bug (142/0)
- Committed 36a47f7: LSE atomics decoded + emulated; modmain booted through
  __libc_start_main and glibc's IFUNC-atomic ldxr/stxr fallback.
- modmain now crashes in __tunable_get_val (0x4128ec): x4 should be
  adrp/add(0x48dc88)+ubfiz(0xC00)=0x48e888 but carries 0x7f8000000000 high
  garbage at the `add x4,x7,x4` (rm=x4 read stale). ubfiz + the add are BOTH
  proven-clean in isolation (incl. on a dirty x4); it's a BLOCK-COMPILER
  register-interaction bug (intervening mov w5,w0 / adrp reusing the host reg
  for rm=x4?). Persists across JIT_BUDGET. Next: JIT_DUMP/trace x4 through the
  block to find where the 0x7f800000 prefix enters.
- elfjit: permanent SIGSEGV diagnostic (guest pc + x0..x7 + sp from CpuState).
- cargo test --workspace 142/0 (arm64jit 108). HARD GATE still unmet (no GPU/APK).
---
## Cycle (Sep 11, 2026) — libbadcpu sigill-emulator register map + arm64jit BitField disarm (151/0)
status: session-end (committed, tests green)
last_agent_claim: two crates hardened against silent miscompiles; the HANDOFF's
documented open arm64jit bug (`__tunable_get_val` x4 corruption) is FIXED.
Commits 8325edc (libbadcpu), 9081bfc (arm64jit BitField).

### libbadcpu (8325edc) — three emulator correctness bugs + 6 new tests
1. Register-index table matched the WRONG glibc gregs layout. ucontext_t gregs
   is greg_t[23] laid out R8..R15,RDI,RSI,RBP,RBX,RDX,RAX,RCX,RSP (0..15) then
   RIP=16/EFL=17 — ONLY RIP/EFL match the x86 reg number. Old table indexed
   gregs[0] as RAX etc., so every emulated POPCNT/MOVBE/LZCNT/TZCNT/BMI1 read/
   wrote the WRONG register. Now GREGS_IDX maps x86 reg number -> true slot;
   popcnt/lzcnt/tzcnt exec-tests prove the dest lands in the right slot.
2. VEX vvvv was never decoded (vex_vvvv stayed 0) — both 3-byte (C4) and 2-byte
   (C5) forms now extract the inverted bits[6:3]. ANDN uses vvvv as its first
   source (was reading the dest register); BLSI/BLSMSK/BLSR correctly ignore it.
3. VEX opcode byte was read from the C4/C5 prefix position, never the byte after
   the VEX fields — no 0F38/0F3A-map VEX instruction ever decoded correctly.

### arm64jit (9081bfc) — BitField dispatches by class; UBFIZ/SBFIZ vs BFI/BFXIL
modmain.elf (full static-glibc boot) now runs PAST the crash the HANDOFF
flagged, into `_dl_determine_tlsoffset` (a newer, distinct frontier).
1. UBFIZ/SBFIZ landed in the BFI merge path and PRESERVED old Rd's upper bits.
   `ubfiz x4,x0,#7,#32` = (0x18<<7)&mask kept a stale 0x7f8000000000 prefix;
   glibc's `__tunable_get_val` then ldr'd [x4,#48] at 0x7f800048e888 (should be
   0x48e888) and SIGSEGV'd. UBFIZ now zero-fills; SBFIZ sign-fills from top.
2. Genuine BFI (insert=true) was also swallowed by the ROR shortcut
   (imms+immr+1==bits) — bfi x4,x0,#16,#16 (0xb3703c04) compiled as a rotate.
   The LSR/LSL/ROR shortcuts are UBFM/SBFM aliases; BFM inserts now handled
   first via an early return (BFXIL in-place mask, BFI shifted-merge).
Regressions: ubfiz_zero_extends_field_and_discards_old_rd,
bfi_still_merges_into_old_rd (objdump-verified encoding).
arm64jit 110/110; workspace 151/0. Battery (loop1 45, structs/dispatch/fpfun/
vtable 42, byvalue 44, bv2/iso_arith 300, signmod 12, iso_wrd 4321, arr/shacc/
fact/ldrsw/fp_only/A/C 42) ALL unchanged.

### Honest remaining
- modmain now boots through `_dl_determine_tlsoffset` (cleared by the udiv fix)
  into glibc `__tls_init_tp` and segfaults inside `__memset_generic` on a small
  address (x0=0 passed to memset by the caller) — the next glibc-CRT frontier.
  HANDOFF judges this synthetic-glibc tail NOT a Roblox boot blocker (real
  libroblox boot path already fully decoded/exit 0).
- Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
  HARD GATE; blocked on a capable host + the real binary/APK (none on this box).

## Cycle (Sep 11, 2026) — UNSIGNED DIV WRONG-RESULT BUG FIXED in arm64jit (152/0)
status: session-end (committed, tests green)
last_agent_claim: a silent, wide x86-emitter bug fixed — every UNSIGNED divide
in the JIT returned 0. Commits 28dba32 (div), c1e41cb (svc additions).

### The bug (would corrupt ANY guest integer mathematics, incl. Roblox)
x86 group-3 `F7` uses /6 = DIV and /7 = IDIV, but `div_r64`/`div_r32` emitted
modrm(3,0,..) = group-3 /0 = TEST — so `48 f7 c1` decoded as `test rcx,eax`
and no quotient was ever produced. idiv_r64 already used /7 and was correct;
ONLY the unsigned forms were broken. Found by driving glibc's
`_dl_determine_tlsoffset` through modmain.elf (earlier battery only exercised
signed div). Regression `udiv_computes_quotient` (100/10=10, 7/20=0), verified
`48 f7 f1`=div rcx / `48 f7 f9`=idiv rcx via as+objdump.

### svc table additions (c1e41cb)
set_robust_list(99)->0 and membarrier(283)->no-op, surfaced by glibc
`__tls_init_tp`. rseq(293) deliberately left -ENOSYS (no valid rseq area).

modmain now reaches real AArch64 syscalls (unhandled-99 -> booted past) before
a memset frontier. arm64jit 111/111; workspace 152/0; battery unchanged.

### Honest remaining
- modmain SIGSEGVs in glibc `__memset_generic` (caller passed x0=0) — glibc-CRT
  tail, NOT a Roblox boot blocker (HANDOFF; libroblox boot already decoded).
- HARD GATE unchanged.
cycle: 8
status: <updating>
last_agent_claim: <no completion claim yet>
updated: 2026-09-11T22:45:00Z
---

## Cycle 8 (Sep 11, 2026) — 128-bit SIMD ld/st mis-decode fix (workspace 156/0)
Root-caused the modmain `__memset_generic` SIGSEGV: `str q0,[x0,x3]`
(0x3ca36800) decoded as GPR `ldrsb x0,[x0,x3]` and silently clobbered guest x0
(GPR register-offset/pre-post/unscaled gates lacked a bit26 vector-file mask).
Added VecLdStrReg + VecLdStImmUnscaled + VecLdStIndexed (128-bit q classes,
bit26=1) and bit26==0 on the two GPR gates. modmain now advances cleanly
(honest Unsupported stop) instead of SIGSEGVing. +4 regressions, arm64jit
111->115. Commits: 853cc44 on dev.
next: scalar S/D register-offset -> also added (FpLdStrReg, commit 921af2a,
arm64jit 116/116, workspace 157/0); remaining scalar S unscaled/pre-post is a
glibc-CRT tail, not a Roblox boot blocker; then libloader -> libbadcpu ->
services/auth.
cycle: 8
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T03:52:13Z
---

# Session (Sep 11, 2026) — scalar FP/SIMD unscaled + pre/post-index ld/st; libbadcpu VEX.0F38 completion (workspace 164/0)

Agenda: verified the handoff's flagged failing test (`libloader android
test_setup_android_layout_idempotent`) is ALREADY FIXED (8b72828: per-call
unique temp root + `create_dir_all` before `set_permissions`) — workspace was
157/0 at start, not failing. Then implemented two higher-leverage pieces, both
committed and regression-locked.

## 1. arm64jit — scalar FP/SIMD (B/H/S/D) UNSCALED + PRE/POST-index ldr/st (commit 36f01ff)
Closed the last of the `bit26=1` immediate family the status drumbeat had
deferred ("the last of the same bit26=1 family; glibc-CRT memset tail"):
- `FpLdStImmUnscaled` (LDUR/STUR, signed imm9) and `FpLdStImmWb` (pre/post
  index writeback that advances Xn). Shared `fp_scalar_xfer` transfer helper.
- Decode gate (all verified vs aarch64-linux-gnu-as ground truth): bit26=1
  (vector), bit25=0 (immediate), bit21=0 (NOT register-offset — FpLdStrReg
  READS bit21, found by the regression; register-offset words ALSO have
  bit25=0), bit24=0 (not the scaled 0x3d form), bit23=0 (not 128-bit Q),
  bits[29:27]=111; ld=bit22; addressing = bits[11:10] (00 unscaled, 01 post,
  11 pre — pre is 0b11/0x0c00, NOT 0b10, found by the decode test).
- +4 tests (decode ground truth incl. neighbor non-collision; stur/ldur
  round-trip; post-index advances Xn; pre-index applies offset then writes
  back). arm64jit 116->120. **modmain.elf (full static glibc) now boots PAST
  its documented `stur s0` memset wall** (0x40a95c) into
  `__libc_setup_tls`/`_dl_get_dl_main_map`, stopping at a new null-deref
  (fault 0x0, guestpc 0x400b30 right after `bl _dl_get_dl_main_map`) — again
  the non-Roblox glibc-CRT tangent, NOT a Roblox boot blocker.

## 2. libbadcpu — complete VEX.0F38 integer family (commit 0fe7d8a)
Added BEXTR (F7 pp=0), BZHI (F5), and the BMI2 shifts SHRX/SARX/SHLX (F7 with
pp=F3/F2/66). Encodings verified against host gcc+objdump. Subtlety the
pre-existing code never had to face: fix-size for the 0F38 integer ops must be
driven by VEX.W (`vex_w`), NOT the legacy `has_66=>16-bit` rule — SHLX rax is
pp=1 yet 64-bit. +3 tests. libbadcpu 12->15.

## Gate
- `cargo build --workspace` clean; `cargo test --workspace` **164/0** (arm64jit
  120, libbadcpu 15, libloader 16, +1+1+11 others).
- Commits 36f01ff (arm64jit scalars), 0fe7d8a (libbadcpu) on local `dev`,
  tree clean.
- HARD GATE unchanged (real Roblox boot + run log on GPU/APK host).
---
cycle: 9
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T03:59:38Z
---

## Session (Sep 11, 2026) — loader→JIT e2e regression test + libbadcpu LZCNT fix (169/0)

Commits `7f54fbd` (arm64jit `crates/arm64jit/tests/loader_run.rs`, the bdd8b03
"confirm no regression" loader deliverable): cross-compiles -nostdlib aarch64
programs and runs load_elf_image→bind_image_plt→jit_run end-to-end
(add=42/loop=45/fp=10/fib=13); serializes the 4 parallel test threads because
load_elf_image MAP_FIXEDs the shared 0x400000 non-PIE base (parallel threads
clobber each other's image → stub_of_target panic). Skips when cross-gcc absent.
`6d6ef08` (libbadcpu): fixed 16-bit LZCNT off-by-16 wrong-result (u16 count is
already 16-bit; TZCNT had no offset) + lzcnt_16bit regtest.

Gate: build clean; tests **169/0** (arm64jit 120+4, libbadcpu 16, libloader 16,
+1+1+11). HARD GATE unchanged: `elfjit <libroblox.so> 0x1f0db20 --jni` on a
GPU + real-binary host.
cycle: 10
status: cycle_end
last_agent_claim: Focused ad-hoc verification complete (explicitly **not** a suite-green claim): (rc=0)
updated: 2026-09-11T04:03:22Z
---
---
## Cycle (Sep 11, 2026) — three silent FP/SIMD miscompiles fixed (workspace 172/0)
Built a new cross-gcc **double-precision** battery (real `double` C arrays,
Horner, matmul, exact division, 3^10 loop, |x|>thresh count, weighted avg),
each result diffed against a native x86-64 compile. Three real bugs found+fixed
(commit a0465a0), all silent wrong results:
1. scalar `fsub` (0x1e613800) swallowed by the SIMD WidenShl/shll gate (its
   `(insn>>24)&0x0f==0x0e` nibble test also matched the scalar-FP 0x1e family):
   shll gate now requires bit28==0 (real shll 0x0e/2e/4e/6e; scalar-FP 0x1e).
   dscale.elf 0→1.
2. store_nzcv_fp hardcoded N=0 → FP compare b.mi/b.lt/b.le never fired, b.gt
   always true for ordered non-equal. N = CF∧¬ZF. dclamp 6→4.
3. `ld1 {v30,v31}` (2-reg) swallowed by the ld2 (deinterleave) gate → array
   literals loaded interleaved garbage. Added Ld1N/St1N consecutive handlers,
   discriminated by opcode bits[15:12] (ld1-2reg=0xA vs ld2=0x8).
+3 regression tests. arm64jit 120→123, workspace 169→**172/0**. Full cross-gcc
+SIMD hand-battery still green (loop1=45, lane_test/smov=-570, iso_*=doc).
Gate: cargo build clean; cargo test --workspace 172/0. HARD GATE unchanged:
real Roblox boot + run log on a GPU/APK host (elfjit <libroblox.so> 0x1f0db20
--jni). HEAD a0465a0 on local dev.
---
## Cycle addendum — FMOV-immediate [16,30] decode bug (workspace 172/0, commit bc5ab89)
Second cross-gcc FP battery (single-precision array div, double-struct-byvalue,
float matmul, int/float casts, FP loop condition) → one more silent bug:
`decode_fmov_imm` exponent wrap at `>=4` wrongly mapped E=3 (true exp +4 =
constants 16.0..30.0) to -4 (0.0625), so every FMOV-imm in [16,30] was ~256x too
small. fdivf float array {6,12,18,24}/3 gave 6 instead of 20 (isolated
divss/addss/fcvtzs verified OK; the constants were wrong). Fixed threshold
`>=5` (verified vs assembler 0.125..30.0). +4 asserts (16,30,2,0.75). fdivf
6→20; rest of battery matches native. arm64jit 123/123, workspace 172/0.
Gate: build clean, tests 172/0, HEAD bc5ab89 local dev. HARD GATE unchanged.
---
## Cycle addendum — scalar FMA3 + prior FP fixes (workspace 173/0, commits a0465a0..578faaf)
Two more cross-gcc FP layers this cycle:
- **FMOV-immediate [16,30] decode bug** (bc5ab89): decode_fmov_imm exponent wrap
  `>=4` wrongly mapped E=3 (true exp +4) to -4 — every FMOV-imm in [16,30] was
  ~256x too small. Threshold `>=5`. fdivf 6→20.
- **scalar FMA3** (578faaf): fmadd/fmsub/fnmadd/fnmsub (0x1f.. scalar 3-source
  FP) was swallowed by a broad SIMD-immediate gate and mis-decoded as garbage;
  -O2-contracted polynomials returned 0x8000000000000000. Now decoded at the top
  of decode + translated (mulsd/addsd/subsd, pxor-negate), single & double.
  fma1 160=native. Verified: fmadd/fmsub/fnmadd/fnmsub d0,d1,d2,d3 = 17/-7/-17/7.
Cycle total: 5 FP/SIMD correctness fixes + FMADD feature; new cross-gcc FP
battery as regression ground truth (dpoly/dsum/dmat/ddiv/dscale/dclamp/
ddmat2 / fdivf/dloop/mixed/dstruct/dneg/fmat/drec / fma1). arm64jit 124/124,
workspace 173/0. HEAD 578faaf local dev. HARD GATE unchanged (real libroblox.so
boot on GPU/APK host).
---
## Cycle addendum — shifted-register add/sub clobber + fclamp fix (workspace 174/0, commit 9cc0f16)
Third cross-gcc FP battery (all -O2: fma-heavy Horner, quadratic-formula div,
clamp, mixed div) surfaced one more silent miscompile: **apply_shift_const** wrote
the shift count into RCX then `shl rcx,cl`, but both AddSubReg/AddSubExt callers
pass x==RCX (the Rm value) — so `add x1,x2,x0,lsl#3` became x2+24 CONSTANT and
-O2 double-array loops read the same element each iteration (fclamp 10 vs 9).
Fixed to immediate-shift C1/4..7. fclamp 9. +regression. arm64jit 125/125,
workspace 174/0; full 3-batch FP battery + core C battery all match ground
truth. (dnorm.elf degenerates to +inf overflow = C UB; not counted.)
Cycle total across sunday's grind: 6 FP/SIMD miscompile fixes + scalar FMA3,
all regression-locked. HEAD 9cc0f16 local dev. HARD GATE unchanged (real
libroblox.so boot + GPU host).
---
## Cycle addendum — LdStPair D-stride + 4th -O2 battery (workspace 175/0, commit 1eb7c0a)
New -O2 battery (3x3 int matmul, struct double-array, byte scan, 64-bit loop,
short array) surfaced one more silent miscompile: **LdStPair fp_d stride was
rt*8** but Dn = low 8B of the 16-byte Vn slot (rt*16), so `ldp d29,d28` loaded
the wrong offsets and a follow-on fmadd read stale vector slots
(structfield.elf 128 vs 52). Fixed both directions. structfield 52.
Battery: m3 structfield bytes iloop shorts = 45/52/5/150/24, all = native.
arm64jit 126/126, workspace **175/0**. Cycle total: 7 FP/SIMD miscompile fixes
+ scalar FMA3, all regression-locked. HEAD 1eb7c0a local dev. HARD GATE
unchanged (real libroblox.so boot + GPU host).
---
## Cycle addendum — single-precision LdStPair + 5th -O2 battery (workspace 176/0, commit 8d2d57b)
5th cross-gcc -O2 battery (float struct array, 2D float matmul det, float exp,
string copy, unsigned arith) surfaced one more: byte3 0x2c/0x2d (single-precision
FP pair) was treated as fp_d (scale 8) so `ldp s0,s1` read 8 bytes/reg and
post-indexed 2x -> fstruct 0x391c0000 garbage (should 34), fmat2 det -108
(should 0). Split fp_s (32-bit, 0x2d/0x2c, scale 4, low 4B of vector slot).
fstruct 34, fmat2 0, fexp 649, str 0, uint 999 = native. +regression. arm64jit
127/127, workspace **176/0**. Cycle total: 8 FP/SIMD miscompile fixes + scalar
FMA3, regression-locked. HEAD 8d2d57b local dev. HARD GATE unchanged.
---
## Cycle addendum — sdiv/udiv signedness inversion (workspace 177/0, commit dae2e05)
6th -O2 battery surfaced a SEVERE silent one: MulDiv gate decoded sdiv/udiv via
bit17==1, but bit17=0 for BOTH (discriminator is bit10, sdiv=1). Every sdiv was
labeled unsigned -> emitted unsigned `div`, negatives went huge (idivA returned
0xaaaaaa2d, wanted -35). gcc magic-constant div hid it previously. Fixed to
bit10 + regression sdiv_is_signed_udiv_is_unsigned_same_negative_input.
arm64jit 128/128, workspace **177/0**. Cycle net: 9 FP/int miscompile fixes +
FMADD, all regression-locked. HEAD dae2e05. HARD GATE unchanged (real boot +
GPU artifact).
---
## Cycle addendum — MSUB operand direction (workspace 178/0, commit ccbf55c)
7th -O2 battery (byte/modulo, switch, SIMD reduce, shorts, bits, strcmp):
bytelen n%50 -> -48 (wanted 48). MulDiv MSUB arm did rn*rm-ra; ARM MSUB is
ra-rn*rm. Fixed to sub RDI,RAX + mov (ra - rn*rm). Constant-folded addrs
masked it. +regression. arm64jit 133/133, workspace **178/0**, all 7
cross-gcc -O2 batches + core + FMA green. Cycle net: 10 miscompile fixes +
FMADD. HEAD ccbf55c. HARD GATE unchanged (reproducible boot+GPU artifact).
---
## Cycle addendum — ADDV horizontal add implemented (workspace 179/0, commit 6290937)
8th -O2 battery surfaced a missing SIMD op: gcc SIMD-vectorizes short sums to
`addv s0,v1.4s`, which an earlier dup/move gate swallowed (identity copies ->
0 instead of 360). Implemented Inst::Addv (sum sign-extended lanes -> bottom
element). KEY trap: ADDV source Vn is at bits[9:5], bits20:16=fixed 17 (unusual
SIMD layout); mask 0xfffffc00, gate at decode top. vadd 360=native.
+regression addv_horizontal_sum_across_4s_lanes. arm64jit 134/134, workspace
**179/0**, all 8 batches + core + FMA green. Cycle net: 11 miscompile fixes +
FMA + ADDV. HEAD 6290937. HARD GATE unchanged.
---
## SESSION WRAP (Sep 11) — arm64jit hardened; 45/45 battery, workspace 179/0
9th (-O3/64-bit magic div, SIMD vmacc, FP recursion, bit-rot) and 10th (-O3
float->int conversion boundaries, fcvtzs, affine FMA) batteries were BOTH clean.
Definitive sweep: **45/45 cross-gcc programs across 10 batches match native
exactly**; `cargo test --workspace` **179/0**; `arm64jit` 134/134.
Full back-half fixes (11 commints): sdiv/udiv signedness inversion (MulDiv
bit17->bit10, dae2e05), MSUB direction ra-rn*rm (ccbf55c), LdStPair s-pair scale
4 (8d2d57b), ADDV horizontal add w/ bits9:5 Vn + top-of-decode gate (6290937),
plus pre-compaction WidenShl, FP-compare N, ld1-2reg, FMOVimm, FMA3,
apply_shift_const, d-pair stride. All regression-locked.
JIT now robust for FP/int/SIMD/conversion under -O2/-O3 after 11 bug classes.
HARD GATE unchanged (reproducible boot + GPU artifact on real host).
cycle: 11
status: cycle_end
last_agent_claim: - `cargo build --workspace` → clean (Finished in 0.17s, only a pre-existing warning in `sober-core`) (rc=0)
updated: 2026-09-11T04:34:32Z
---

## cycle 12 (Sep 11, 2026) — libloader: Android packed relocations (APS2) + RELATIVE application (workspace 184/0)
status: session-end (committed, tests green)
last_agent_claim: The Rust loader now materializes R_AARCH64_RELATIVE data
relocations for ET_DYN/PIE libraries in-process (no external unpack_rela.py),
so the JIT path can load real Roblox APK libs and their Android/GSI
dependencies whose .data/.data.rel.ro needs relocation before code dereferences
pointer globals/vtables.

### What landed (commit on `dev`)
- `crates/libloader/src/android_relocs.rs` (new): read_sleb128, decode_aps2
  (AOSP for_all_packed_relocs port, validated vs real 2.726.1142 in Session 14),
  read_elf_relocations (DT_ANDROID_RELA 0x60000011/12 preferred over
  DT_RELA/RELASZ; APS2 decode or stock Elf64_Rela), apply_relatives.
- `load_elf_image` (elf.rs): reordered segment loop to copy->apply-relocs
  (while image still RW, gated on is_pie)->mprotect. Non-PIE ET_EXEC untouched.
- Tests +4 libloader unit (hand-built APS2 golden bitstream — first attempt's
  `[0x80,0x40]` was really SLEB -8192, not +0x2000, proving the decoder is
  faithful; offset-delta stride; SLEB negative; bad magic) and +1 integration
  (crates/libloader/tests/reloc_apply_test.rs: cross-gcc ET_DYN asserts every
  RELATIVE slot == load_bias+addend), then +3 DT_RELR unit tests — DT_RELR
  (0x23) fallback when no RELA table is present, per shipped glibc/Android
  DO_RELR decode (offset word + bitmap bit i -> base+(i-1)*8).

### Gate (verified this cycle)
- cargo build --workspace: clean
- cargo test --workspace: 190 passed / 0 failed
  (libloader 16 -> 23 unit + 1 integration; libbadcpu 16 -> 18;
  loader_run 4 -> 5 incl. PIE+RELATIVE e2e)
- HEAD: 437e4fd (local `dev`)

### Honest remaining
- The pre-existing `crates/libloader/tests` skips cleanly when cross-gcc absent.
- Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni` run log)
  is the HARD GATE — blocked until a capable host + the real binary/APK (none
  on this box). load_elf_image now handles the RELATIVE data-reloc the real
  dependency chain needs from DT_RELA / DT_ANDROID_RELA(APS2) / DT_RELR; next
  ordered items: GLOB_DAT/JUMP_SLOT main-dynamic resolver (bind_image_plt
  already covers JUMP_SLOT), libbadcpu gaps, services/auth.
cycle: 12
status: cycle_end
last_agent_claim: Ad-hoc verification complete and passing. (rc=0)
updated: 2026-09-11T04:49:07Z
---

## Cycle (Sep 11, 2026) — GLOB_DAT/ABS64 binding + libbadcpu BMI2 completion + auth identity
status: session-end (committed, tests green) HEAD 2a9c6eb
last_agent_claim: workspace 199/0 (was 190). Four focused commits, each gated
on `cargo build --workspace` + `cargo test --workspace`. The handoff-flagged
android::test_setup_android_layout_idempotent is already fixed (8b72828, per-call
temp root + create_dir_all before set_permissions) and passes.

### Landed
1. 7f03937 arm64jit `bind_glob_dat`: GLOB_DAT (1025) + ABS64 (257) main-GOT
   symbol-address relocations were never bound (bind_image_plt only did
   DT_JMPREL; load_elf_image only RELATIVE) -> guest `adrp;ldr x,[GOT]` read 0
   and deref'd/called NULL. Now writes guest_of(st_value)+addend / dlsym /
   host-thunk, in the normal path and the pltrelsz==0 early-return.
   Cross-gcc -shared fixture: SIGSEGV -> entry()=37. +1 integration test.
2. b177f7c libbadcpu MULX (VEX.0F38.F6, RDX*rm high->reg low->vvvv) + RORX
   (VEX.0F3A.F0 rotate-right, new 0F3A dispatch, imm8 after ModR/M).
   +2 hardware-verified tests.
3. 2a9c6eb libbadcpu ADCX/ADOX (66/F3 0F38 F6): Dest+=Src+flag writing only
   the working flag; width from REX.W not operand_size (mandatory 66), and
   32-bit carry in u32 domain. Verified ADOX sets OF = unsigned carry-out
   (not signed overflow). +2 hardware-verified tests.
4. a8249f4 sober-services auth: AuthResult user_id/username were dead fields
   (callback only captured token -> IPC always sent None). Now captures
   token + user_id + username and forwards the full result. +4 tests.

### HARD GATE (unchanged)
Roblox actually running (load -> JNI init -> main loop -> frame on a GPU host)
is NOT met and cannot be on this GPU-less VPS without the real libroblox.so/APK.
The gate stays `elfjit <libroblox.so> 0x1f0db20 --jni` on a capable host.
---

## Cycle 14 (Sep 11, 2026) — LogicImm DecodeBitMasks rotate-RIGHT fix (workspace 200/0)
status: cycle_end (committed, tests green) HEAD 8c1706b
last_agent_claim: fixed a severe silent miscompile in the commonest ARM64
instruction path (logical-immediate MOV/AND/ORR masks).

### The bug (would corrupt ANY guest that loads an asymmetric bitmask)
`decode_logical_mask` applied a **LEFT**-rotate to the element
(`ones << r | ones >> (esize-r)`) but ARM `DecodeBitMasks` (DDI0487) uses
**ROR — rotate right**. Every rotation-ASYMMETRIC logical-immediate mask was
miscompiled. Only symmetric masks happen to give the same value under both
directions (<15% of encodings), so the whole prior test set stayed green while
real compiler output was silently wrong:
`mov x0,#0xffffffff80000001` (0xb26187e0) returned 0xfffffffe00000007 instead
of 0xffffffff80000001.

### Verification (quantified, not hand-waved)
- Ground-truth cross-check vs the real `aarch64-linux-gnu-as` + `objdump` over
  ~700 (N,immr,imms) encodings: the FIXED right-rotate agrees with the
  assembler on **592/592 valid** masks (112 architecturally invalid). The OLD
  left-rotate would have been wrong on **509** of those 592.
- End-to-end elfjit (no QEMU): `mov x0,#0xffffffff80000001` -> 0xffffffff80000001
  and `mov x0,#0x3ffffffc` (esize!=64) -> 0x3ffffffc, matching native x86-64.
- New regression `logic_imm_rotation_asymmetric_mask_ror` locks both asymmetric
  encodings + sanity that symmetric 0xCCCC/#1 stay correct.
- Ad-hoc cross-gcc batteries (shifts, sign-ext, 64-bit math corners, 128-bit,
  switch, SIMD reduce, bitfield, struct-by-value, FP conv, -O3 loops/arrays/
  auto-vectorized SIMD) all PASS vs native.
- `cargo build --workspace` clean (0 errors); `cargo test --workspace`
  **200 passed / 0 failed** (arm64jit 131+6).

### Commit
- 8c1706b  arm64jit: fix LogicImm DecodeBitMasks to rotate RIGHT (ROR), not left

### Next (ordered)
1. Continue ISA-surface probing via cross-gcc battery to flush out more silent
   miscompiles (this ROR bug was #12-class: silent, symmetric-mask-masked).
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. HARD GATE (real Roblox boot + run log on a GPU/APK host) stays unmet here.
---

## Cycle 14b (Sep 11, 2026) — three silent SIMD/logic-imm miscompiles fixed (workspace 202/0)
status: session covered two focused commit rounds (8c1706b + 01aa402) and several
ledger-only updates (7628024); workspace 202/0 at end.
last_agent_claim: three silent arm64jit miscompiles found via cross-gcc batteries
driven end-to-end through elfjit (no QEMU) vs native, all fixed + regression-locked.

1. LogicImm DecodeBitMasks rotate-RIGHT (8c1706b). Left-rotate vs ARM ROR —
   every rotation-asymmetric logical-immediate mask miscompiled; symmetric masks
   masked it from the test set. Quantified: fixed right-rotate agrees 592/592
   valid encodings vs real as; old would be wrong on 509.
2. Scalar-D ADDP vs fcvtzs collision (01aa402). 0x5ef1 (addp) hits the fcvtzs
   scalar gate 0x5ee0b800; bit20 discriminates. New SimdPairAddD.
3. saddw2/uaddw2 upper-half (01aa402). SimdAddw read Vm@0 always; Q=1 form reads
   upper 8 bytes. -O2 loops accumulate both halves now.

Verified: i*i reductions return 76 (native) at -O3 (addp) and -O2 (saddw+saddw2);
50+ cross-gcc battery programs green. Commits: 8c1706b, 076b024, 01aa402.
cargo build clean; cargo test --workspace 202/0.

HARD GATE unchanged: real Roblox boot + run log on a GPU/APK host (none here).
---
## Session (Sep 11, 2026) — differential battery; sxtl2 + nop/scvtf-family fixes (workspace 212/0, 3 known-broken)
New permanent differential battery `crates/arm64jit/tests/diff_battery.rs`: runs
the SAME C through the loader→JIT path (cross-gcc aarch64) AND a native gcc
oracle, requiring exact equality (JIT == native == cross-gcc, all three agreed).
Three verified fixes landed this session; the "intermittent SIMD-loop bug" from
the prior cycle is ROOT-CAUSED and FIXED:

1. [FIXED] `sxtl2`/`uxtl2` upper-half source: SimdXtl had no `upper` field, so
   `sxtl2 v.2d, v.4s` re-read Vn's LOW 64 bits. decode +`upper=insn>>30&1`;
   translate +`n_half=upper?8:0`. 3 deterministic linear regressions in jit.rs.
2. [FIXED, root cause of the intermittent class] ARM `nop` (and the whole
   system/hint 0xd5... family) misdecoded as **ScvtfFixed**: the scalar
   int->fp fixed gate checked only bits[29:28]==01, which 0xd5xxxxxx also
   satisfies — so EVERY guest `nop` ran as `scvtf d<n>, x0, #56`, converting
   the caller's x0 (often SP) into a double and overwriting a vector register
   with address-derived garbage. That is exactly the old intermittent,
   layout-dependent corruption. Gate now requires top byte ∈ {0x1e,0x9e}.
3. [FIXED] ScvtfFixed sf/to_double read bit30; real `scvtf d,x,#f` (0x9e...,
   bit30=0) decodes as Sd/Wn single. Both are bit31 (0x9e=X/D, 0x1e=W/S).
   Decode regression `nop_is_hint_not_scvtf_fixed` pins 1-3.

Two DETERMINISTIC bugs remain (washed out as "intermittent" before):
- magic-division `%N` reducer (smull/smull2/uzp2/sshr/mls): quotient off by
  the divisor (m[0]=-101 for k*7%101 which should be 0).
- -O3 addp/smulh reduction (diff_mixed): deterministic wrong.
Battery gates the fixed/passing families (maskf, mod_pow2, times7, count6,
div/mod, FP, bitfield, unsigned-compare); magicdiv/struct_arr/mixed are
#[ignore]d as documented-known-broken.
cargo build clean; cargo test --workspace 212/0 (3 ignored).
cycle: 15
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
cycle: 16
status: cycle_end
last_agent_claim: float-vector SIMD correctness: 6 silent NEON/FP miscompiles fixed; workspace 227/0, 0 ignored (was 220/0)
updated: 2026-09-11T05:52:40Z
---
cycle: 16
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T06:06:26Z
---

---
cycle: 17
status: cycle_end (committed, tests green) HEAD 5cbe6ce
last_agent_claim: 9 silent arm64jit miscompiles fixed, incl. root-cause of the
  long-open nondeterministic SIMD-loop corruption (ADDV-to-scalar stale bytes).
  Commits 6ffd490, e1ec841, 7e2689a (ADDV), 5cbe6ce (CMN C-flag polarity),
  Workspace `cargo test --workspace` 234/0, 0 ignored.
updated: 2026-09-11T06:50:00Z
cycle: 17
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T06:30:56Z
---
cycle: 18
status: cycle_end
last_agent_claim: Ad-hoc verification complete (script `/tmp/hermes-verify-arm64jit.sh`, created and cleaned up): (rc=0)
updated: 2026-09-11T07:29:39Z
---
cycle: 19
status: cycle_end (committed, tests green)
last_agent_claim: Fixed 2 silent scalar single-precision FP miscompiles in
  arm64jit (FpUnary single path): (a) loaded float bits in RAX were never moved
  into xmm0 before `cvtss2sd`, so EVERY single frint*/fsqrt s read stale xmm0
  (a floor loop returned 450 vs 360); (b) `frintz s` used roundsd mode 0b00
  (round-to-nearest) instead of 0b11 (toward-zero), so trunc of negatives came
  out 20 vs native 40. Verified vs native oracle (floor 360/360, ceil 440/440,
  trunc 40/40). Added differential canary diff_scalar_fp_round_single (3
  probes: fr_floor_single, fr_ceil_single, fr_trunc_single_negatives); confirmed
  the trunc canary reverts to a 20-vs-40 MISCOMPILE when the mode fix is
  reverted. Workspace `cargo test --workspace` 254/0, 0 ignored (battery 30).
updated: 2026-09-11T08:20:00Z
---
---
cycle: 19b
status: cycle_end (committed, tests green)
last_agent_claim: Follow-on to the scalar FP-rounding fix. Vector-FP probe
  sweep exposed a SYSTEMIC misdecode: the SIMD widen-multiply gate
  (insn AND 0x0f00_c000) drops bit28 and only keeps byte2 bits15:14, so any op
  sharing those folded into smlal. Two real SILENT (garbage, not stop)
  miscompiles fixed: (1) VECTOR frint{n,m,p,z,a} (a floor loop's
  frintm v1.4s decoded as smlal, returned 1.9e16 vs 590): new SimdFrint
  decode+translate BEFORE smull, per-lane cvtss2sd->roundsd->cvtsd2ss;
  (2) VECTOR compare-to-zero fcmeq/fcmgt/fcmge/fcmlt/fcmle Vd,Vn,#0.0
  (gcc x<0?a:b select, byte2 0xea -> smlal, wrong branch): new VecFpCmpZero
  decode+translate (comiss vs zeroed xmm1 + setcc). Plus SimdMull gate
  tightened to (insn AND 0x3800)==0 (byte2 bits13:11 clear) — genuine
  smull/umull/smlal always clear them, frint(0x88/98)/fcmlt(0xea) set one;
  an earlier byte2==0xc0/0x80 was too strict (size lives in bits[2:0]) and
  broke diff_magic_div; corrected. Proved prior 'vfa fneg sign flip' was a
  UB false alarm (unsigned cast of a negative; aarch64 fcvtzu clamps to 0,
  x86 signed trunc -363 — JIT returning 0 was CORRECT). Verified fneg/bsl/
  fcmlt+bsl isolate clean. New canaries diff_vector_frint_rounding +
  diff_vector_fp_compare_zero; decode regression
  simd_frint_and_cmpzero_not_swallowed_by_widen_mul. cargo build clean;
  cargo test --workspace 257/0 (battery 32). HARD GATE unchanged (no GPU/APK).
updated: 2026-09-11T08:45:00Zcycle: 19
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T07:44:48Z
---
cycle: 20
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T07:45:20Z
---
cycle: 21
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T08:08:57Z
---
cycle: 22
status: cycle_end
last_agent_claim: Ad-hoc verification of this turn's changes is complete and clean (the leftover `/tmp/hermes-verify-cycle12.sh` is a pre- (rc=0)
updated: 2026-09-11T08:19:37Z
---
cycle: 23
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T08:41:54Z
---
cycle: 24
status: committed
last_agent_claim: Fixed a silent SIMD miscompile: the ld2/st2 structure
(DE)INTERLEAVE translate arms scaled at BYTE granularity regardless of element
size, so 'ld2 {v.8h,v.8h}' (2-byte elements — gcc strided u16 accumulate
'for(i+=2) s2+=b[i]') read mem[2i],mem[2i+1] instead of mem[4i],mem[4i+2];
the even-index sum registered the wrong memory elements (isolated repro
311814 -> 281606 == native). Decode computed esize for ld3/ld4 but dropped it
for the 2-register ld2/st2 case. Threaded esize through decode + deinterleave
at element stride. Verified no-QEMU; +diff_battery canary
diff_ld2_halfword_strided_accumulate (proven sensitive: reverting only the
esize change re-fails) + decode regression with assembled 8h/16b/4s encodings.
cargo build clean; cargo test --workspace 282/0 (was 280). commit 36813d1.
Pre-existing SEPARATE co-resident bug documented (two widen loops in one block
contaminate bits 32-63 of acc lanes; byte-only esize=1 path, independent of
this fix) — next high-value target. HARD GATE unchanged (no GPU/APK).
updated: 2026-09-11T09:10:00Z
---
cycle: 24
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T10:41:01Z
---

---
## Session close (2026-09-11, cycle 27) — gen_signed_div fuzz campaign: 3 more silent JIT miscompiles fixed (workspace 298/0)
Extended `fuzz_jit.py` with two new generators (signed-div / widen-byte-LUT) and ran
a fresh sweep. It surfaced a class of REAL silent miscompiles the older generators
(the long, looped morphologies) had never exercised — gcc -O3 fully-unrolled /
software-pipelined straight-line blocks. All three fixed + regression-tested, on top
of this session's boot-path syscall expansion:

1. **32-bit shifted-register ASR sign bug** (a3e6056). `apply_shift_const` did a 64-bit
   `sar` on a zero-extended W operand, so `sub w1,w1,w2,asr#31` (gcc's magic-division
   sign-correction) turned a negative dividend's `-1` into `+1`; signed quotients &
   remainders were off by 2/element (a/3, a/5, /17 all corrupt). Now sign-extends
   (movsxd) before the sar. Fixed 3 of the 5 original fuzz failures.
2. **In-place widening sxtl/uxtl (rd==rn)** (a3e6056). `sxtl v30.2d,v30.2s` writes the
   widened 8-byte lane0 at byte0, clobbering the narrow source bytes lane1 reads at
   byte4; gcc's vector-reduction idiom dropped lane1 (plain and shrn-gen sums wrong).
   Now snapshots Vn to permscratch when rd aliases rn.
3. **In-place saddw/saddw2 narrow-source alias (rd==rm)** (e313a77). Same class for the
   add-wide op: `saddw v31.2d,v29.2d,v31.2s` clobbers Vm.s[1] before it's read
   (reduced sums off by a lane, addp 1001 vs 1003). Snapshot the aliasing source.
   Verified the full sxtl2+saddw+saddw2+addp reduction chain in isolation (red6,
   10026 == oracle).

Also (commit 54a70b1) closed the SAME in-place widening-alias class in the three
remaining SIMD widening ops: SimdMull (smull/umull/smlal/umlal), VecFcvtl, VecFcvtn2
(covers the `fmov s,w`/ins/zip staging + saddw reduction gcc emits) — all snapshot the
aliasing source to permscratch. New regression widen_in_place_smull_fcvtl_snapshot_source.
Also committed: guest_svc boot-path syscall expansion (22 AArch64 numbers) + 2 new fuzz
generators. Confirmation sweep 300/304 green across 19 fresh seeds (the 4 fails are all
the two-loop bug below, no regressions). Workspace 299/0, build clean, tree clean.

## RESOLVED (2026-09-11, commit 47b1007) — the open fused two-loop signed-div bug
Root cause was a DECODE COLLISION, not a register-clobber: integer vector
NEG/ABS (two-reg misc, opcode bits[16:12]==0xb, bit16 CLEAR) collided with the
vector float->int FcvVec gate (mask 0xffe0_fc00 zeroes bits[20:16], so
`neg v29.2s,v31.2s`=0x2ea0bbfd matched the fcvtzu v0.2s residue and executed as
a float->int convert, silently producing 0). Fixed by requiring bit16 SET on
the FcvVec gate, adding a real SimdArithUnary NEG/ABS decode+translate, and a
JIT_STEP per-instruction register trace. Repro jit=406144671==oracle; ~420 fuzz
cases / 12 seeds green; decode+exec regressions + diff_vector_neg_abs_unary
canary; workspace 302/0. No known-open arm64jit correctness items remain.

OPEN, documented (CYCLE 27, superseded): a real bug still reproducibly failing —
loops (e.g. gen_signed_div's `s += a[i]/D; s += a[i]%D` pos loop THEN the negative-
divisor loop) in ONE function** miscompile at ANY size (n=4 reproduces:
pos + neg /7, oracle 406144671 vs jit 78184144; pos alone PASSES, neg alone PASSES,
each per-element q/r PASSES, and the full vector accumulation chain PASSES in
isolation). The failing code is fully-unrolled straight-line scalar magic-division
feeding vectors via ins/zip1/saddw — the scalar registers are reused across the two
fused computations and one clobbers the other's live value. Suspect: a scalar
translate arm (smull/sdiv/lsl/sub) corrupting a guest reg in a large straight-line
block under software-pipelining. Needs a JIT per-instruction register tracer
(persist: store-based [pc, v-slot] ring in CpuState, emits PLAIN stores — host calls
mid-block are illegal because guest x31==host RSP). Repro kept at fuzz_jit.py
gen_signed_div (FUZZFAIL_50_8, 9001_1, 9003_16). HARD GATE unchanged (no
GPU/APK/libroblox.so on this box).
cycle: 25
status: cycle_end
last_agent_claim: <no completion claim> (rc=0)
updated: 2026-09-11T11:09:44Z
---
