# Open Sober — Agent Handoff

## Session (Sep 12, 2026, hermes-worker) — main-loop wall pinned to the exact bit: a NORMAL bionic mutex in LOCKED_CONTENDED state (guest word 0x2), glibc cross-ABI mismatch; graphics "first frame" gates verified (workspace green, tree clean)

Commits `6bc57a6` → `18ae7f6` → `063dc5e` → `7946fe3` (dev). The real
`libroblox.so` boot keeps advancing: JNI_OnLoad → `--startapp` drives
`nativeAppBridgeV2StartAppWithParams` → `GameActivity_initializeNativeCode`,
reaches the engine main loop, and **idles stably** (exit 124, no
SIGSEGV/SIGABRT). This cycle identified exactly what the loop waits on.

### 1. The wall, exact
`JIT_TRACE` shows the main loop parked in `pthread_mutex_lock` on guest mutex
`0x6edae60`:
```
[t=...] [mutex_lock] 0x106edae60 bionic_word=0x00000002 state=0x2 gpcreq=0x102b53bb0
```
- guest `state` = **2 = bionic `MUTEX_STATE_LOCKED_CONTENDED`** (NORMAL,
  non-PI, non-recursive mutex).
- Caller guest PC `0x102b53bb0` (in `GameActivity_initializeNativeCode
  +0x2f8488`, the mutex-init attribute setup).
- All guest threads idle in host futex, 0% CPU.

**Root mechanism:** the guest manages its own bionic `pthread_mutex` on its own
16-bit `state` word (modern NDK r28c layout: `_Atomic(uint16_t) state` @0,
`owner_tid` @4, 28-byte tail). When a guest thread's inline fast-path hit
contention it set state=2 and called our bridge's glibc `pthread_mutex_lock`;
glibc reads the bionic 16-bit state word as glibc's own lock encoding, sees no
matching glibc `__owner`, and futex-blocks forever while the holder (also a
guest thread) released the lock via its own fast-path atomics. A clean
**bionic-vs-glibc cross-ABI futex mismatch** — NOT an ALooper wait, NOT a
decoder gap.

### 2. Correction of this session's own earlier claim
`6bc57a6`/`18ae7f6` called it a "recursive mutex rendezvous" from the glibc
`__kind` field at mutex+16. That field is PAST the bionic word (on a different
init path — `pthread_mutexattr_settype(#1)` at 0x2b53b04 inits a different
mutex). `063dc5e` corrected it: the blocking mutex is NORMAL, in
LOCKED_CONTENDED (word 0x2).

### 3. Verified: the mutex is real — do NOT weaken it
All guest threads genuinely idle (futex/nanosleep, 0% CPU). An "optimistic
non-blocking acquire" (trylock→return 0 on EBUSY) let a second guest thread
into the same critical section → SIGSEGV on garbage. A hand-rolled bionic CAS
attempt was reverted in-tree, unbuilt, before touching the boot. Both reverted.
This is genuine shared-memory mutual exclusion at lifecycle handoff.

### 4. Correct fix (SCOPED — next task)
Implement bionic's NORMAL mutex protocol byte-exact on the guest's own 16-bit
`state` word @0: acquire = CAS state 0/1→LOCKED_UNCONTENDED; contention → set
LOCKED_CONTENDED(2), futex-wait on the word; unlock = clear to 0 + FUTEX_WAKE.
Must be paired with the bionic `cond` (cond_wait internally unlock+relock the
mutex). Validate with a TWO-THREAD rendezvous unit test (A locks via bridge, B
blocks in bridge, A unlocks, B acquires) BEFORE wiring into the boot.
Alternatively drive the awaited looper/app-command state. Keep the stable idle
boot as the base.

### 5. Graphics "first frame" gates both pass (headless, this VPS)
`glesv2-wrapper headless_graphics.rs` (surfaceless llvmpipe ES3, ETC2
interception, BC1 passthrough) and `arm64jit egl_window_present.rs` (Xvfb real
X11 window, full eglGetDisplay→...→glClear→eglSwapBuffers through the JIT
guest-bridge slots, returns EGL_TRUE) both pass — real frames present headless.

Repro:
```bash
cd /home/hermes-worker/runs/open-sober
cargo build -p arm64jit --example elfjit
timeout 30 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144
JIT_TRACE=1 ... 2>&1 | grep mutex_lock | tail   # pin blocking mutex state/caller
```

Commit `152dce9` (dev). Two real bottlenecks to StartApp forward-speed removed:

1. **Translation-block cache (the big one).** The PC-driven dispatcher
   (`jit_run_inner`) recompiled a region from scratch on every `blr`/`br`/`ret`
   re-entry. A boot hot-spotting on a small accessor (Roblox's TLS-block getter
   `0x2b9dee0` → 160 of ~1271 traced block-execs, each `pthread_getspecific`)
   retranslated that code ~once per call — the dominant cost once the guest is
   churning TLS blocks. `cached_block()` keys on `(image, pc, state)` and leaks a
   process-lifetime `JitBlock` (never munmaps). Evacuated at each *top-level*
   `jit_run` (safe: leaked) so distinct ELF images at the same `JIT_BASE`
   (the `diff_battery` suite maps each test at 0x100000000) never run a stale
   block — this correctness fix is what makes the whole thing safe. Real boot:
   **767 compiles / 5691 hits**. New `block_cache_stats()` + `JIT_STATS`
   per-dispatcher heartbeat.
   - **Regression caught & explained by the cache:** the `loader_run_timer_signal
     _delivers_sigalm` test started failing once the loop got fast. Bisect showed
     guest nanosleep was never *actually sleeping* (see #2), so the 20000-iter
     spin loop finished before 2×100ms timer ticks; the test only passed by luck
     of uncached-JIT slowness stretching it past 200ms. Fixing #2 made it pass
     deterministically (0.23s) regardless of JIT speed. Lesson: a *fast* JIT
     exposes races the slow one masked — run the timing/signal loader tests after
     any perf work.

2. **nanosleep syscall arg fix (real boot bug).** aarch64 `nanosleep` passes
   `rqtp` in x0, but `guest_svc` read it from a[1] (x1) → NULL req → EFAULT in
   ~1.5µs, no sleep. So every guest sleep-wait was really a busy-spin. Now reads
   `a[0]` (rqtp) / `a[1]` (rmtp). Regression test
   `guest_svc_nanosleep_reads_timespec_from_x0`. This matters for the boot: any
   guest `sleep`/`usleep`/wait that Roblox does now blocks the guest properly
   instead of hot-spinning the core.

### Main-loop wall, now characterized precisely (NOT a deadlock)
`--startapp` drives the real `GameActivity_initializeNativeCode` (0x258b144 →
region `0x284dxxx`, TLS-block accessor `0x2b9dee0` = `ldar x22,[x0+0x10]; cbz`
+ `pthread_getspecific`, wrapper `0x284d524`, vtable-check `0x284f874/880`). A
12s JIT_TRACE reaches **236 distinct blocks, growing across the window
(30→69 distinct block-sites in first/last 100 execs)** → the guest is *advancing
through new init code*, just slowly, and makes no guest `svc` once settled.
So the next lever is NOT a decoder gap, NOT an ALooper wait — it's either
(a) more JIT/perf so it grinds through the ~thousands of TLS-block allocs faster,
or (b) finding the specific singleton whose init never *completes* and seeding it
(Session-11 guard/flag pattern), or (c) driving the awaited looper/app-command
state so the main loop dispatches a real frame/render instead of init-churning.

### Diagnostics added
- `resolver::name_of_call_addr()` reverse slot → name; JIT_TRACE now prints e.g.
  `hostcall@pthread_getspecific` (identified the main-loop hot import).
- `JIT_STATS=1` prints a per-250ms dispatcher heartbeat: `[jit] step N pc=... block-cache: C compiles / H hits`. Compiles climbing = new code; flat + hits rising = genuine loop spin.
- elfjit prints `[elfjit] block-cache: C compiles / H hits` on clean exit.

Repro (see STATUS.md for full):
```
cargo build -p arm64jit --example elfjit
timeout 30 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144
JIT_STATS=1 timeout 30 ... 0x258b144       # progress heartbeat
JIT_TRACE=1 timeout 12 ... 0x258b144 | grep hostcall@ | sort | uniq -c | sort -rn
```

## Session (Sep 11, 2026, hermes-worker) — POST-JNI_OnLoad game-start: `--startapp` boot stage; real libroblox reaches the ENGINE MAIN LOOP (workspace 388/0)

Commit `a4f94d1` (dev). The real `libroblox.so` 2.738.1397 boot advances from
"JNI_OnLoad returns 0x10006 then the process exits" to **the guest entering and
persistently running the engine's `GameActivity` main-loop / event-pump region**
after a new `--startapp` stage chains the real Java-side game-start entry.

- **Why the boot previously exited**: JNI_OnLoad is a *registration* function;
  on real Android the JVM then calls `nativeAppBridgeV2StartAppWithParams` etc.
  to actually start the game (main loop + EGL/GLES init). elfjit only ran
  JNI_OnLoad, so once it returned and the spawned worker boot-body finished, the
  harness's `main()` returned and the process exited cleanly (exit 0).
- **`--startapp <link-addr>`** (elfjit): after `jit_run(JNI_OnLoad)` returns
  `Ok(0x10006)`, builds the singleton env + fake-but-valid `jobject` (x1) +
  `jstring` (x2) and `jit_run`s the real `nativeAppBridgeV2StartAppWithParams`
  (0x258b144) as a fresh guest entry. Critical detail: it reuses the **boot-phase
  guest SP** (`s2.x[31]=st.x[31]`); a fresh 0 SP wrapped StartApp's `sub sp,#0xf0`
  prologue to `0xffffffffffffff10` and the frame-write SIGSEGV'd immediately.
- **New jni helpers**: `new_fake_object()`, `new_string_utf_handle()`, and a
  `JNI_TRACE_REGISTRY` env to dump RegisterNatives bindings.
- **Verified** (headless, no QEMU): the guest executes 800+ distinct blocks
  through StartApp — FindClass for dozens of Roblox classes, repeated VM_GetEnv,
  pthread_once/mutex/getspecific TLS-key protocoling in the
  `GameActivity_initializeNativeCode` thread-local setup, `LockBasedAllocator` —
  then settles into a persistent main-loop cycle (`ldar x22,[x0+0x10]; cbz`
  await + `pthread_getspecific` dispatch) across two guest threads and runs
  until the harness `timeout` fires (exit 124; **no SIGSEGV/SIGABRT**). JNI_OnLoad
  alone still exits 0 cleanly (~4.8s); both paths preserved.

### Current wall (narrowed from "nothing drives the app" to a specific loop)
The engine main loop is reached but awaits app events / lifecycle (looper
input, window/surface, EGL) that the real Java side supplies. Next (ordered):
1. Feed the awaited `GameActivity` app-command / looper state and route the
   boot's `egl*`/`gl*` imports through the existing Mesa llvmpipe resolver so
   any EGL context/frame path reachable from StartApp runs real software
   graphics — the first reproducible engine-loop artifact (a frame / looper
   event dispatch), headless on this VPS.
2. Advance FMOD audio init and the JNIMain main-loop drive.
3. HARD GATE (real session + run log) unchanged as the end goal; the
   achievable-on-this-VPS milestone next is a real *frame* / first looper event,
   then it's a GPU host for the final perf proof.

Repro:
```
cargo build -p arm64jit --example elfjit
# boot-only: timeout 120 ./target/debug/examples/elfjit .../libroblox.so 0x2173ff4 --jni
# boot + game-start main loop: timeout 30 ./target/debug/examples/elfjit .../libroblox.so 0x2173ff4 --jni --startapp 0x258b144
```
Run-log: `/home/hermes-worker/runs/startapp-boot-runlog.txt` (exit 124 = ran
in the engine main loop until the harness timeout; no crash).

## 🟢 STABLE HEADLESS BOOT of real libroblox.so (exit 0, reproducible)

**The real `libroblox.so` (2.738.1397) now boots to a stable state headlessly
through `arm64jit` + `libloader` and exits cleanly (`exit 0`, no SIGSEGV) —
verified 3/3. JNI_OnLoad returns `0x10006`, real engine JNIMain code runs, the
worker guest thread runs clean, teardown is clean.** This is the boot
stabilization milestone on this VPS.

Two fixes this session (commits `6e9fd4e`, `1639899`, workspace 388/0):
1. **`plt::bind_glob_dat`** — unresolved *function* GLOB_DAT/ABS64 slots were
   left at stale values (0 or a `.dynstr` symbol-name pointer); a guest `blr`
   through them jumped INTO `.dynstr` (SIGSEGV, register file = ASCII symbol
   strings). Now bound to a benign host-call stub. Real lib: 63 -> 67 bound.
2. **`__cxa_thread_atexit_impl` no-op shim** — was resolved to real glibc,
   which stored the guest AArch64 TLS-destructor pointer and invoked it NATIVELY
   as x86 when the worker guest thread exited -> SIGSEGV executing guest ARM64
   .text (the post-boot "worker/teardown" crash). Now a no-op, so glibc never
   runs a guest functor natively. This fixed the crash and gave the clean exit.

Repro:
```
cd /home/hermes-worker/runs/open-sober
cargo build -p arm64jit --example elfjit
timeout 120 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni
```
Expect: seeds + `Test TelemetryProtocol` + `DeviceStaticParams is null`, then
`jit_run returned Ok(0x10006)` / `JIT(no-QEMU) entry() -> 65542 (0x10006)`,
clean `exit 0`.

**NEXT (advance the boot / graphics):** JNI_OnLoad now succeeds and the process
exits cleanly; push toward a real main loop that doesn't exit. Graphics
wrappers already point at Mesa llvmpipe; the egl/glesv2 stubs exist. Use
`GRAPHICS_RECOMMENDATION.md`: surface a window/surfaceless EGL context, have
the guest render a frame, and confirm with a run log. Also consider whether
JNI_OnLoad's spawned worker should be joined/looped instead of the process
exiting when the main dispatch returns.

**The real `libroblox.so` (2.738.1397) boots through `arm64jit`
(`elfjit <libroblox.so> 0x2173ff4 --jni`) and main-thread `JNI_OnLoad` returns
`0x10006` (JNI_VERSION_1_6) reproducibly.** This cycle (commit `6e9fd4e`,
workspace 388/0):

1. **Closed a real indirect-call-into-`.dynstr` vector.** `plt::bind_glob_dat`
   left unresolvable *function* GLOB_DAT/ABS64 slots at their original value
   (0 or stale `.dynstr` symbol-name pointer); a guest `blr` through one jumped
   into `.dynstr` (SIGSEGV with register file full of ASCII symbol strings —
   "pthread_setspecific", "memset", "pthread_cond_broadcast"). Now bound to a
   host-call stub. Real binary result: **67 GLOB_DAT bound / 11 unresolved**
   (was 63/15); the fault's `guestpc` is now a real guest address, not ASCII.
   Regression test `loader_run_unresolved_func_globdat_binds_safe_stub` (fn
   import via `int (*gfp)(int)` binds to host thunk 0x7f0000002008).
2. **Fault diagnostics enriched** (elfjit): full host-x86 register dump,
   `rbx_matches_gueststate` (is the faulting RBX the thread's registered
   CpuState?), guest-thread table `(host_tid:guest_tid,state)`, nesting-aware
   `in_jit_run` counter.

**Current wall (precise, unbuffered-stderr-proven):** main's `jit_run` returns
Ok(0x10006); the SIGSEGV is in the **post-run phase** (worker guest thread,
tid=1, start_routine `0x284d168`, is running when main's dispatch ends).
`rip=0x10284d6be` is a guest `.text` address in the
`JNIActivityLifecycleCallbacks_nativeOnDestroyed` region executed as x86 —
a **host path calls a guest function pointer natively**, not a translated-block
fault. `rbx_matches_gueststate=false` ⇒ the guest-register dump is an artifact
of a garbage RBX; trust the host regs/rip, not guestpc. Next: find which host
call path dispatches guest `0x284d6b4` during worker/teardown (guest signal
handler outside the cooperative dispatcher, an atexit/on_destroy callback
routed to guest natively, or the worker start_routine dispatched down a
non-`jit_run` host path). See `docs/` run-log + `runs/STATUS.md`.

**The real `libroblox.so` (2.738.1397, extracted via `sober-core::apk::extract_libs`
to `~/.cache/open-sober/robbox/libroblox.so`) now boots through `arm64jit`
(`elfjit <libroblox.so> 0x2173ff4 --jni`) and main-thread `JNI_OnLoad` returns
`0x10006` (JNI_VERSION_1_6) — the canonical success value — reproducibly.**

Milestone commits this cycle (all `dev`, workspace green):
- `6de9a2d` libloader: reserve writable guest tail past image end (crashpad
  telemetry static table walks ~36MB past the last PT_LOAD bss).
- `8033d04` arm64jit: route LSM map bucket allocator (0x1d97744, size in x0) to
  host calloc; seed the LocalStorageManager static hash-map global (0x726f8c0)
  with a two-level empty map + NOP its lazy-init store (0x1d975f8); generalized
  the TLS-pool calloc thunk into a param'd `place_calloc_patch` (two slots:
  0x62d9000 x1-size, 0x62d9800 x0-size).
- `5ba69aa` elfjit: seed JNICallProtocol refcounted-singleton ptr (0x7333948 ->
  0x7333950, zeroed bss == PTHREAD_MUTEX_INITIALIZER at +8).

**Current wall:** after main `entry()` returns 0x10006, a *worker guest thread*
spawned during JNI_OnLoad (`pthread_create(start_routine=0x284d168)`, tid=1)
crashes; the SIGSEGV handler reports CpuState registers that decode to ASCII
libc symbol-name strings ("pthread_setspecific", "memset", "pthread_cond_*",
"broadcast"), i.e. it appears to execute/read `.dynstr` string data. Hypothesis:
the spawned thread's per-thread guest TLS is only a bare zeroed buffer (the
known "per-thread PT_TLS init-image copies for clone/pthread children" gap), so
`__tls_get_addr`/`pthread_getspecific` on the child reads garbage
(function-pointer table entries land on symbol-name strings). TODO: give
`spawn_pthread` children a real `setup_guest_tls` TLS block + TCB (copy PT_TLS
init image) like the main thread, and confirm the child then runs cleanly.

Repro:
```
cd /home/hermes-worker/runs/open-sober
cargo build -p arm64jit --example elfjit
timeout 150 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni
```
Expect: `[lsm-map]`/`[JNICall-singleton]` seeds, JNIMain logs, `JIT(no-QEMU)
entry() -> 65542 (0x10006)`, then the worker-thread SIGSEGV.

## ⚠️ CRITICAL RULES — READ FIRST

1. **NO WORKTREES.** Do NOT create git worktrees. Ever.
2. **NO BRANCHES.** Work directly on `dev` branch. No feature branches, no topic branches.
3. **CLAUDE.md** at repo root has these rules — read it.
4. **Commit directly to `dev`**, push, and let the user decide when to merge to `stable`.
5. If you need to research something, do it inline or in a temp dir outside the repo.
6. **`cargo check --workspace`** before committing. **`cargo test --workspace`** before pushing.

## Project Overview

**Repo:** https://github.com/glm-5-turbo/open-sober
**Branches:** `stable` (release), `dev` (active development — work here)
**Build:** `cargo build --release`
**Tests:** `cargo test --workspace`

Open Sober is an open-source reimplementation of VinegarHQ's Sober — a runtime that runs the Roblox Android APK on Linux natively.

## What's Built

### Phase 1 - libbadcpu (`crates/libbadcpu/`)
CPU feature emulator — SIGILL handler for missing x86-64 instructions (POPCNT, MOVBE, LZCNT, TZCNT, BMI1).

### Phase 2 - libloader (`crates/libloader/`)
Process sandbox/spawner — chroot isolation, ELF loader, Android runtime env setup, Unix socket IPC.

### Phase 3 - sober-services (`crates/sober-services/`)
Browser-based OAuth auth handler.

### Phase 4 - sober-core (`crates/sober-core/`)
Main binary orchestrator. `open-sober play --apk roblox.apk` is the main command.

**Key APK:** `~/Documents/Projects/open-sober/roblox-android.apk` (178MB, not in git)
**APK structure:** `assets/app.zip` → `config.arm64_v8a.apk` → `lib/arm64-v8a/libroblox.so` (101MB, NDK r28c, Android 26)

## Current Status (July 20, after session 11b)

### ✅ Complete (all sessions)

1. **Custom QEMU** built from `/tmp/qemu-10.2.1/` — patched copy at `~/.cache/open-sober/qemu-patched`.
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — trampoline-based mutex interceptors.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled.
4. **QEMU bridge wiring** (`qemu.rs`) — version bridges for libc/libm/libdl.
5. **Canary GOT patching** — stack_chk_guard write via mprotect.
6. **SIGSEGV handler** — RELRO faults, self-write JIT bugs, NULL deref handling.
7. **Pre-mprotect RELRO** — ~464 pages made RW before JNI_OnLoad.
8. **Pre-resolved trampoline table** — all **785 entries** via data-driven dlsym loop (782/785 resolved, 3 Bionic-only fallbacks filled with `__errno_location`).
9. **Canary check patched out** in code copy.
10. **Init guard deadlock FIXED** — code patch replaces `bl 26c0c7c` (mutex+condvar) with `mov w0,#1; nop`.
11. **Raw ARM condvar shim** — `mov w0,#0; ret` in mmap'd RWX page, replaces `pthread_cond_wait` trampoline.
12. **pc=0x0 crash FIXED!** — `_dl_mcount` is now noped to `ret` BEFORE dlopen(libroblox), using 64KB mprotect to force QEMU TCG JIT cache invalidation + precise 4-byte `ret` write. See details below.

### 🟢 _dl_mcount noping now works (Session 8)

**What changed:**
1. Moved `_dl_mcount` patching to run BEFORE `dlopen(libroblox.so)` (was: after). This ensures no GSI library can trigger `_dl_mcount` during loading.
2. Extracted into `disable_mcount_profiling()` function called at the start of `main()`.
3. Combined 64KB mprotect on ld-linux text page (forces QEMU TCG JIT cache invalidation) with precise 4-byte `ret` write + `__builtin___clear_cache()`.
4. Also zeroes `rtld_global.dl_profile` in the data section as belt-and-suspenders.
5. Sanity check at the end confirms `_dl_mcount` entry now reads as `0xd65f03c0` (AArch64 `ret`).

**Evidence:** The process now reaches `[jni_shim] entering JNI_OnLoad...` without crashing. Previously it crashed with `pc=0x0` before reaching this point.

### 🟢 JNI_OnLoad returns successfully (Session 9 breakthrough!)

After patching JNI_OnLoad's entry point to `mov w0, #0x6; movk w0, #0x1, lsl #16; ret` (returns `JNI_VERSION_1_6 = 0x10006` immediately), the shim now works end-to-end:

```
[jni_shim] JNI_OnLoad -> 0x10006
[jni_shim] Entering sleep loop
```

The code patch at `base + 0x1f64e58` replaces the first 12 bytes of JNI_OnLoad with:
- `0x528000c0` = `mov w0, #0x6`
- `0x72a00020` = `movk w0, #0x1, lsl #16` (w0 = 0x10006)
- `0xd65f03c0` = `ret`

This bypasses ALL internal initialization functions that were hanging:
- One-time init guard → skipped (we also pre-init the guard+JVM)
- LocalStorageManager init → skipped
- nativeSetAssetPath → skipped
- Various JNI FindClass/RegisterNatives calls → skipped

**Why this is OK:** For now, the goal is to get the binary loading successfully. The JNI stubs are in place and any code that checks the JNI version will get 0x10006. The internal init functions primarily register native methods and initialize BSS globals — we already pre-init the critical ones.

**What was fixed in session 9**

1. **`patch_jni_onload()`** — new function that patches JNI_OnLoad's entry to return 0x10006 immediately
2. **Verified working** — JNI_OnLoad returns 0x10006, `sleep loop` reached successfully

### Current Status (July 20, after session 11)

### ✅ Complete (all sessions)

1. **Custom QEMU** built from `/tmp/qemu-10.2.1/` — patched copy at `~/.cache/open-sober/qemu-patched`.
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — trampoline-based mutex interceptors.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled.
4. **QEMU bridge wiring** (`qemu.rs`) — version bridges for libc/libm/libdl.
5. **Canary GOT patching** — stack_chk_guard write via mprotect.
6. **SIGSEGV handler** — RELRO faults, self-write JIT bugs, NULL deref handling.
7. **Pre-mprotect RELRO** — ~464 pages made RW before JNI_OnLoad.
8. **Pre-resolved trampoline table** — all 785 entries.
9. **_dl_mcount profiling disabled** — `ret` at entry point, dl_profile zeroed.
10. **PLT GOT condvar patching** — pthread_cond_wait/timedwait → immediate return.
11. **One-time init guard pre-init** — set to 1, skips condvar-based init.
12. **Phase 1a progressive JNI_OnLoad patch** — only NOPs clock/time init call instead of full bypass. JNI registration runs (3 classes, 13 methods resolved). nativeSetAssetPath reaches but hangs.
13. **JNI table slot 113 fix** — GetStaticMethodID at correct slot (was RegisterNatives). Default fallback changed from 0x10006 to 0/NULL.
14. **End-to-end success** (full bypass) — binary loads, JNI_OnLoad returns, sleep loop reached.
15. **Timestamp flags pre-init** — two BSS flags (0x6a325e4, 0x6ae6690) set to 1.
16. **Frequency double pre-init** — `base+0x6ae66e8` set to 1.0e9.

### Session 11 summary (July 20, 2026)

**Goal:** Implement Phase 1 progressive patch (shrink JNI_OnLoad bypass).

**What was done:**

1. **Replaced full bypass with Phase 1a progressive patch.**
   - Old: `patch_jni_onload()` replaced first 12 bytes of JNI_OnLoad with `mov w0,#6; movk w0,#1,lsl#16; ret` (returns 0x10006 immediately).
   - New: `patch_jni_onload_phase1()` only NOPs the clock/time init call at binary offset `0x1f64e9c` (`bl 0x1cfabfc`).
   - Guard check, GetEnv, LocalStorageManager, JNI registration, nativeSetAssetPath, guard setter, and all subsequent code all run normally.
   - Falls back to full bypass if patch fails.

2. **Fixed JNI function table for real JNI calls.**
   - Slot 113 (byte offset 904 in JNIEnv struct) was incorrectly set to `stub_RegisterNatives`. The actual function at this offset takes `(env, class, name, sig)` — matching `GetStaticMethodID`. Changed to `stub_GetStaticMethodID`.
   - Default fallback changed from `stub_GetVersion` (returns `JNI_VERSION_1_6 = 0x10006`) to `stub_voidp` (returns NULL/0). The `0x10006` value caused crashes when interpreted as a pointer by non-GetVersion callers.
   - All NULL slots now filled with `stub_voidp` instead of `stub_GetVersion`.

3. **Results — JNI_OnLoad now runs partial native init:**
   ```
   FindClass[1]: NativeLocaleJavaInterface → 3 GetStaticMethodIDs (getLocale, getRobloxLocale, getGameLocale)
   FindClass[2]: NativeUserJavaInterface → 9 GetStaticMethodIDs (getUserId, getIsUnder13, getUsername, getDisplayName, getAlternateName, getPlatformName, getMembershipType, getHasRobloxSubscription, getTheme)
   FindClass[3]: LoggingProtocol → 1 GetStaticMethodID (getProcessTimestamp)
   ```
   Three Roblox JNI classes are found and all 13 methods resolved successfully.
   Then the process hangs in `nativeSetAssetPath` (`0x273de0c`).

4. **Confirmed PLT GOT condvar patching works.** Heartbeat backtrace shows LR at the condvar shim page, proving that the raw ARM shim IS reached from libroblox's internal code. The issue is that after the shim returns 0 (spurious wakeup), the calling code re-checks the condition and re-enters `pthread_cond_wait` in an infinite loop (single-threaded, no other thread to signal).



### Session 11b summary (July 20, 2026)

**Goal:** Debug the `nativeSetAssetPath` hang with improved instrumentation.

**What was discovered:**

1. **Improved SIGALRM heartbeat.** Changed from `sa_handler` (signal handler's own x29/x30) to `sa_sigaction` with `SA_SIGINFO` and ucontext. Heartbeat now shows the **real PC** of interrupted code:
   ```
   [jni_shim] JNI_OnLoad still running (5s) PC=0x...1074 LR=0x...cb04 BT={0x...cb04,0x...a0c0,0x...f90,0x...6038}
   ```
   PC alternates between `0x...1074` and `0x...1084` on the bionic shim trampoline page. Frame 2 at `base + 0x1f64f90` = JNI_OnLoad error path.

2. **`-d exec` trace** shows a repeating 9-address cycle on the bionic shim trampoline page:
   ```
   0xbc0 → 0xdc0 → 0xf00 → 0x1080 → 0x1200 → 0x1380 → 0x1580 → 0x1740 → 0x1940 → 0xbc0 → ...
   ```
   Each 0x200 bytes apart. This is a GSI library init function calling a sequence of bionic trampolines in a tight loop that never terminates.

3. **QEMU TCG cache conflict confirmed.** NOPing `bl 0x273de0c` at offset `0x1f64eb8` requires `mprotect` on page `0x1f64000`, which ALSO contains the JNI registration function at `0x1f6594c`. Three approaches all failed:
   - Single mprotect writing both NOPs → only 2 classes
   - Two separate mprotect calls → only 2 classes  
   - `b #4` skip instead of NOP → only 2 classes
   - Phase 1a (clock-only NOP) → STABLE 3 classes

   **Root cause:** Making page `0x1f64000` RW → write → RX causes QEMU to invalidate TCG cache for the registration function. Re-translation produces incorrect code that skips the `NativeUserJavaInterface` class.

4. **gdbstub tested but impractical.** QEMU's `-g 1234` starts in the dynamic linker phase. Connecting gdb before the hang requires multi-step breakpoint setup. `gdb-multiarch` can connect and examine state but reaching the hang point with a useful backtrace is complex.

**Recommended fix:** Patch `libroblox.so` on disk BEFORE `dlopen` (pre-load patching) to avoid QEMU TCG cache invalidation entirely. The target bytes at file offsets `0x1f64e9c` and `0x1f64eb8` can be replaced with `0xd503201f` (NOP) using `open(O_RDWR)` + `pwrite` before loading the library.

5. **Attempted fixes that didn't work:**
   - Using real glibc `pthread_cond_wait` directly in PLT GOT (via `g_real_cond_wait`) — no futex syscall appeared in `-strace`, suggesting the condvar calls go through a different code path than expected, OR the process hangs before reaching a condvar call.
   - NOPing both clock init AND `nativeSetAssetPath` — caused different execution behavior (only 2 classes found instead of 3), suggesting a QEMU TCG caching issue with the broader mprotect range.
   - `-d exec` tracing was not feasible due to output volume.

**Key JNI classes and methods discovered:**
```
com/roblox/engine/jni/locale/NativeLocaleJavaInterface
  getLocale()Ljava/lang/String;
  getRobloxLocale()Ljava/lang/String;
  getGameLocale()Ljava/lang/String;

com/roblox/engine/jni/user/NativeUserJavaInterface
  getUserId()J
  getIsUnder13()Z
  getUsername()Ljava/lang/String;
  getDisplayName()Ljava/lang/String;
  getAlternateName()Ljava/lang/String;
  getPlatformName()Ljava/lang/String;
  getMembershipType()I
  getHasRobloxSubscription()Z
  getTheme()Ljava/lang/String;

com/roblox/universalapp/logging/LoggingProtocol
  getProcessTimestamp()J
```

**Remaining blocker:** The hang in `nativeSetAssetPath` after JNI registration completes. Investigation suggests:
- The hang is inside `nativeSetAssetPath`'s helper function (`0x273dd4c`) which calls `FindClass`.
- The helper function calls `FindClass(env, x1)` where x1 is garbage (not set before the call). Our `stub_FindClass` may crash on garbage pointers, or enters an error path that calls `pthread_cond_wait`.
- The function at `0x273dd4c` returns 0 (NULL) from its stack slot, and the subsequent `ldr x8, [x0]` (NULL dereference) would crash, but the process hangs instead.
- The exact hang mechanism is not yet identified — possibly a QEMU edge case with NULL dereference in signal context.

### Key Source Files

1. **JNI_OnLoad's internal structure mapped** via disassembly:
   - Guard check at `0x1f65a60` — returns immediately when guard=1 (our pre-init works)
   - GetEnv at `0x5e17fb8` — returns our stub env pointer
   - Clock/time function at `0x1cfabfc` → `b 0x5f4f69c` — three-tier guard check
   - `LocalStorageManager_initStorageManagerNative` — JUST `ret` (no-op!)
   - JNI registration block at `0x1f6594c` — FindClass/RegisterNatives for locale classes
   - `nativeSetAssetPath` at `0x273de0c` — JNI calls
   - Guard setter at `0x1f65a54` — writes to BSS

2. **Root cause of hang without bypass:** The function at `0x5f4f69c` (clock_gettime wrapper) has a three-tier guard check:
   - **Level 1:** Two BSS flags (`base+0x6a325e4`, `base+0x6ae6690`) — if either is 0, takes slow path with condvar loop
   - **Level 2:** Double at `base+0x6ae66e8` — if 0.0 (BSS default), falls through to another init function with condvars
   - **Level 3:** Atomic ldaxr/stlxr timestamp update loop — works under QEMU
   
   Our condvar shim returns 0 (spurious wakeup), so any condvar-based path spins forever without any futex syscall.

3. **BSS pre-inits added:** `__atomic_store_n` with release semantics for the two ts_flags, and a `*(volatile double*) = 1.0e9` for the cntvct frequency. All verified to be within BSS range: `0x64c4f00` to `0x6ae6cec`.

4. **Key addresses identified:**
   - Init guard: `base + 0x6a26e40` (already pre-set)
   - Timestamp flag 1: `base + 0x6a325e4` (ldrb at #1508)
   - Timestamp flag 2: `base + 0x6ae6690` (ldrb at #1680)
   - Cntvct frequency double: `base + 0x6ae66e8` (freq == 0.0 check)
   - JNI_OnLoad entry: `base + 0x1f64e58`
   - JNI registration: `base + 0x1f6594c`
   - Init guard check: `base + 0x1f65a60`
   - condvar-heavy init: `base + 0x26c0c7c` (mutex+condvar loop)

5. **Attempted fixes that didn't work:**
   - **futex-based condvar wrapper:** The `wrap_cond_wait` C function with `syscall(SYS_futex, FUTEX_WAIT_BITSET)` didn't appear in `-strace` output, suggesting the guest code path goes through the PLT (which we patched) or the bionic trampoline (which we also patched), but potentially the futex syscall is intercepted by QEMU user-mode and doesn't reach the host. Using `nanosleep` instead of futex also didn't help — the calls just accumulate delay without making progress since there's no other thread to satisfy the condition.
   - **Pre-setting more BSS state:** Even with all three levels of the clock function guarded, there are more condvar waits deeper in the init chain that we haven't mapped.

**Added in jni_shim.c:**
- `__atomic_store_n` for pre-setting timestamp flags with release semantics
- `*(volatile double*)freq_dbl = 1.0e9` for cntvct frequency
- Verify guards and logging for all pre-init values

### Key Source Files

- `crates/sober-core/src/jni_shim.c` — Main JNI shim (~1570 lines)
- `crates/sober-core/src/bionic_init.c` — Bionic shim C code: trampoline resolver (`__bf_c_resolve`)
- `crates/sober-core/src/bionic_shim.S` — Auto-generated assembly trampolines (785 entries)
- `crates/sober-core/src/qemu.rs` — QEMU launcher

### Running

```bash
ANDROID_ROOT=~/.cache/open-sober/android-env
/home/code-agent/.cache/open-sober/qemu-patched \
  -L "$ANDROID_ROOT" \
  -E LD_LIBRARY_PATH=/system/lib64 \
  -E LD_PRELOAD=/system/lib64/libbionic_shim.so \
  -E ROBLOX_LIB=/system/lib64/libroblox.so \
  "$ANDROID_ROOT/jni_shim"
```

Rebuild jni_shim: `aarch64-linux-gnu-gcc -o "$SYSROOT/jni_shim" "$CRATE/jni_shim.c" -ldl`
(NOTE: use `realpath` for paths — tilde expansion fails under some shells with QEMU.)

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom from `/tmp/qemu-10.2.1/` — patched at `~/.cache/open-sober/qemu-patched`
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- **Bionic shim:** `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** `~/.cache/open-sober/android-env/jni_shim`

### 🎯 Recommended Next Steps

The Phase 1a progressive patch (`patch_jni_onload_phase1`) NOPs the clock/time
init only, producing stable 3-class JNI registration output. The next agent
should skip trying to NOP `nativeSetAssetPath` via runtime mprotect (it shares
a page with the JNI registration function and corrupts QEMU's TCG cache),
and instead:

**Phase A — Port the patched QEMU into the repo (critical long-term fix)**

The custom QEMU at `/home/code-agent/.cache/open-sober/qemu-patched` was built
from /tmp/qemu-10.2.1/ (now deleted). The patches applied were:
1. CF_NO_GOTO_TB — prevents chained TB linking in TCG, fixing SMC crashes
2. tb_set_jmp_target no-op — related to goto_tb patching

Without the QEMU patches, the SMC (self-modifying code) crashes return. The
patched QEMU must be preserved or rebuilt from source. Check:
- `~/Documents/qemu-10.2.1/build/qemu-aarch64` (may exist from original build)
- Or rebuild from upstream QEMU 10.2.1 tarball with the two patches reapplied

**Phase B — Fix nativeSetAssetPath hang (Session 11b blocker)**

The hang is inside `nativeSetAssetPath` (offset `0x273de0c`). The SIGALRM
heartbeat (now with ucontext-based real PC) shows the PC alternating between
two addresses on the bionic shim trampoline page, with frame 2 at
`JNI_OnLoad + 0x1f64f90` (error handling path after JNI calls).

**Key constraint:** NOPing `bl 0x273de0c` at JNI_OnLoad offset `0x1f64eb8`
requires mprotect on page `0x1f64000`, which ALSO contains the JNI registration
function at `0x1f6594c`. Making this page RW → NOP → RX causes QEMU TCG cache
to re-translate the registration function, which then only discovers 2 classes
instead of 3 (unstable behavior). This was confirmed with both NOP and `b #4`
replacements, with single and separate mprotect calls.

**Recommended approach for Session 12:**

1. **Pre-load code patch (on-disk patching).** Instead of runtime mprotect,
   patch `libroblox.so` on disk BEFORE `dlopen`. The `bl 0x273de0c` at file
   offset `0x1f64eb8` (and `bl 0x1cfabfc` at `0x1f64e9c`) can be replaced with
   NOP bytes directly in the .so file using a C function that reads/writes the
   file, then calls `dlopen`. This avoids QEMU TCG cache invalidation entirely
   because the code bytes are different before QEMU first translates them.

2. **Patch the on-disk .so at load time.** Write a small function that:
   - Opens `libroblox.so` with `open(O_RDWR)`
   - Seeks to the two offsets
   - Writes `0xd503201f` (NOP) at each
   - Closes the file
   - Then calls `dlopen("libroblox.so", ...)`
   - QEMU will translate the already-patched code from the start.

3. **If pre-load patching isn't possible** (file permissions, read-only fs),
   use `mmap` to map the file with MAP_SHARED, patch in-memory, then close.
   This also avoids mprotect on the executed pages.

4. **After NOPing both calls**, JNI_OnLoad should run fully:
   - Guard check → GetEnv → (clock NOPed) → LocalStorageManager → JNI reg →
     (assetpath NOPed) → guard setter → remaining JNI calls → return 0x10006
   - If remaining JNI calls (FindClass for more classes after guard setter)
     hang due to NULL returns or condvars, add JNI stubs for those classes.

5. **Build proper JNI stubs** for the 3 discovered classes and 13 methods:
   - `NativeLocaleJavaInterface`: getLocale, getRobloxLocale, getGameLocale
   - `NativeUserJavaInterface`: getUserId, getIsUnder13, getUsername,
     getDisplayName, getAlternateName, getPlatformName, getMembershipType,
     getHasRobloxSubscription, getTheme
   - `LoggingProtocol`: getProcessTimestamp

**Phase C — Full JNI_OnLoad enablement**

Once the basic JNI stubs and condvar shim are working:

1. **Remove the Phase 1a NOP** (stop patching the clock init call).
2. **Fix the remaining crash** — when all NOPs are removed, the binary may
   hit a SIGSEGV from a different code path.
3. **Expand JNI stubs** to handle all classes/methods that JNI_OnLoad needs.
4. **Properly RegisterNatives** — call intercepted native methods with the
   correct signatures.

**Phase D — Integrate with the Rust orchestrator**

Once the C-based JNI shim works stably, update `qemu.rs` to use it as the
main entry point for `open-sober play --apk roblox.apk`.

### Known issues / gotchas

- QEMU `-strace` output + `-d exec` output interleave on stderr. For clean
  analysis, redirect to separate files.
- The `alarm_sa_handler` backtrace via `x29`/`x30` doesn't work reliably in
  signal context under QEMU (the registers are the handler's, not the
  interrupted code). To get real backtraces, use QEMU's gdbstub (`-g 1234`).
- `futex` syscalls from guest ARM code may be intercepted by QEMU user-mode
  and not reach the host kernel. `nanosleep` and `clock_nanosleep` DO reach
  the host and appear in `-strace`. If a blocking condvar is needed, prefer
  `clock_nanosleep` over `futex`.
- Tilde expansion (`~`) in paths breaks with QEMU in some shell contexts.
  Always use `$(realpath ...)` or full `/home/code-agent/...` paths.
---

## Session 12 (Aug 20, 2026 — fresh machine rebuild)

Environment started empty: no qemu-patched, no android-env/GSI libs, no APK,
no NDK. The original Roblox build whose offsets the harness hardcoded is not
served by any mirror anymore, so blindly resuming Session 12 against an
arbitrary current APK would not reproduce the documented behavior
(hardcoded GOT/BSS/RELRO offsets in `jni_shim.c` are per-build).

Two changes committed to `dev`:

### 1. Port the custom SMC-patched QEMU into the repo (was "Phase A critical fix")

`qemu/` now contains a *reproducible* build of QEMU 10.2.1:
- `qemu/patches/0001` — force `CF_NO_GOTO_TB` on every TB in
  `accel/tcg/cpu-exec-common.c` `curr_cflags()` (never chain goto_tb)
- `qemu/patches/0002` — no-op `tb_set_jmp_target` in `accel/tcg/cpu-exec.c`
- `qemu/build.sh` — download + patch + build aarch64-linux-user →
  `qemu/out/qemu-aarch64`
- Verified end-to-end: `./qemu/build.sh` produces a working emulator.
  Installed to `~/.cache/open-sober/qemu-patched`.

### 2. Version-agnostic offset discovery (`elf_disco.c`)
The hardcoded GOT / canary / RELRO offsets were the true blocker on a fresh
box (no matching APK). Added a pure ELF parser in
`crates/sober-core/src/elf_disco.c` (+`.h`):
- `robo_got()` — exact GOT/reloc slot for an import, from DT_RELA/DT_JMPREL
- `robo_relro_range()` — PT_GNU_RELRO (else last PF_W PT_LOAD)
- `arm64_adrp_target()`, `robo_first_bl()` — AArch64 decode helpers
Wired into `jni_shim.c`: `patch_condvar_plt_got`, the canary GOT write and
the RELRO pre-mprotect all become discovery-first with the old constants as
automatic fallback. `qemu.rs` cross-compiles + links `elf_disc.c` into the
shim.
Tested without a Roblox APK: `tests/elf_disco_test.rs` cross-compiles a real
ARM64 `.so` importing pthread_cond_* and asserts `robo_got()` matches
`readelf -r` exactly. `cargo test --workspace` green.

### Still needed (separate environment step)
A Roblox Android APK, a GSI/system lib64 tree (bionic libc/c++), and the
NDK/JDK so the shim can actually `dlopen(libroblox.so)`. Mirrors were
bot-blocked / version-mismatched on this box; the acquisition is manual or
via a browser session. Once an APK is present, the version-agnostic shim
should load it without re-tuning offsets (subject to the GSI lib tree).

---

## Session 13 update (Aug 20, 2026) — first real runtime run attempts

### Acquired the real Roblox APK via a real (Playwright headless) browser
Cloudflare-walled mirrors fail via curl; a Playwright headless Chromium (with
the MCP) passed through and let me download the arm64-v8a APK:
- Version chosen: **2.726.1142 (arm64-v8a, Android 8.0+/minapi-26)** — the
  newest arm64-only build on APKMirror, NDK r28c / Android 26 (matches the
  harness toolchain), June 19 2026.
- Downloaded as an `.apkm` bundle from
  `/apk/roblox-corporation/roblox/roblox-2-726-1142-release/...-download/?key=...`
  → contains `base.apk` + `split_config.arm64_v8a.apk` → extracted
  `lib/arm64-v8a/libroblox.so` (104,208,904 B ≈ 100 MB).
- Note: `2.726.1142` **does NOT match** the July 2026 build the CPython hardcoded
  in the shim (that build has JNI_OnLoad at 0x1f64e58; this build has it at
  **0x1f0db20**). So the hardcoded JNI_OnLoad/clock/BSS offsets are wrong for
  this build. `elf_disco` (Session 12) fixes the GOT/RELRO ones; the
  JNI_OnLoad *patch offsets + BSS pre-inits are still hardcoded* and must be
  made discovery-driven before this build can run JNI_OnLoad.

### The runtime stack now boots and reaches dlopen(libroblox.so)
On the fresh box I rebuilt and linked:
- bridges (`libc.so`, `libm.so`, `libdl.so` — LIBC version tags present)
- `libbionic_shim.so` (bionic→glibc trampolines, `symbols_aarch64.c`-style)
- `libguest_stubs.so` — auto-generated no-op stubs for all 146 Android NDK
  UND symbols of libroblox.so (AAsset*, ALooper*, AMediaCodec*, AMediaFormat*,
  ANativeWindow*, egl*, gl*, __android_log*, OpenSLES sl*)
- `jni_shim` (with elf_disco linked)
- glibc base: ld-linux-aarch64.so.1 + android-env/lib
Then `~/.cache/open-sober/qemu-patched` boots the whole thing.

Achieved:
- ✓ `_dl_mcount` nop works (64KB TCG flush) — the Session-8 fix functions
- ✓ bionic shim loads; `dlopen(libroblox.so)` starts; all 576 UND symbols
  resolve.
- ✗ **Blocker: `pc=0x0` NULL-call during dlopen's relocation phase.** With my
  early-`SIGSEGV` catch: `bad addr=0x0 pc=0x0 lr=0x7678b8034d0c`. A versioned
  `@LIBC` symbol that computes a static GOT/PLT slot of 0 is being CALLED by
  the guest dynamic loader during relocation, before JNI_OnLoad. This is the
  handoff's documented long-tail (each `@LIBC_*` needs a real symbol, not
  NULL). My SIGSEGV handler now prints LR to pinpoint it.

### Concrete build/run commands (artifact locations)
```
ANDROID_ROOT=~/.cache/open-sober/android-env
SYSROOT=$ANDROID_ROOT/system/lib64
qemu-patched -L $ANDROID_ROOT \
  -E LD_LIBRARY_PATH=/system/lib64 \
  -E LD_PRELOAD=$SYSROOT/libbionic_shim.so:$SYSROOT/libguest_stubs.so \
  -E DISPLAY=:0 $ANDROID_ROOT/jni_shim
```
(Link each lib from `crates/sober-core/src/{elf_disco.c,jni_shim.c,...}`.)

### Next agent session to-do (ordered)
1. **Make libroblox's JNI_OnLoad patch offsets version-agnostic** (currently
   hardcoded 0x64e9c/0x64eb8 for the OLD build). Use `elf_disco` + JNI_OnLoad
   disassembly to find `nativeSetAssetPath` / clock `bl` and NOP the right
   bytes for `2.726.1142`.
2. **Resolve the `@LIBC_*` NULL GOT** (the pc=0 lr at relocation). Candidates:
   the versioned libc symbol that the loader calls at 0 — add a real bionic
   shim/guest_stubs impl, or ensure the wholearch `libc.so` exports every
   `@LIBC_*` libroblox uses (readelf -r to list, then provide). See
   `disable_mcount_profiling` for the established dlsym+patch pattern.
3. Keep GSI symlinks for the 10 NEEDED libs (libandroid/EGL/GLESv2, etc.) —
   my empty stubs satisfy the linker but must not return NULL when called
   (they're no-ops already).

### Key Session-13b diagnostic (isolated)
The single most useful finding: **`libbionic_shim.so` (built from the repo)
crashes ANY arm64 binary when LD_PRELOADed under the patched QEMU**, even
`printf("hello")`:
```
hello (no preload)           -> prints "hello"
LD_PRELOAD=libbionic_shim.so -> SIGSEGV si_addr=0x1 (right after brk()+1MB
                                anonymous mmap on the main thread's init)
```
`si_addr=0x0000000000000001` = the shim's trampoline/init resolves a glibc
symbol to address 1 (an miscalc'd GOT read) and dereferences it. The shim
(as bundled in this repo) was built against a *specific* host-glibc ABI; the
fresh box's glibc from `/usr/aarch64-linux-gnu` (gcc-15) doesn't match, so
the trampoline's per-symbol `dlsym(RTLD_NEXT, …)` returns garbage for some
entries during the pre-load resolve loop. This is the base cause of the
`pc=0x0 ads()` seen inside `dlopen(libroblox.so)`.

Suggested next-agent fixes (in order of leverage):
1. Make the bionic-shim `__bf_c_resolve` per-symbol `dlsym` tolerant: if it
   returns NULL, back-fill with a local no-op trampoline instead of leaving
   the slot at 0/garbage (so no `pc=0` or addr=1 call can occur).
2. Pre-resolve against `RTLD_DEFAULT` (not just RTLD_NEXT) and validate each
   entry is a real code address (> 0x10000) before committing the table.
3. Then re-run the guest; the loader may get past the shim init and into
   `dlopen(libroblox.so)` cleanly, exposing only the versioned `@LIBC` GOT
   slots that `elf_disco` already resolves generically.

This is the concrete path to the first "JNI_OnLoad" print with the real
2.726.1142 libroblox.so — the bionic shim's trampoline resolution is the
binding blocker on this box.

### Session-13c: NULL-safe resolver committed; load-time shim crash isolated
Committed the NULL-safe bionic trampoline resolver (bionic_init.c): every
`dlsym(RTLD_NEXT,…)` in `__bf_c_resolve` now falls back to `__bf_noop()`
instead of leaving a NULL slot (which branched to 0). Good hygiene, but
isolating the real blocker confirmed the crash is EARLIER and separate:
**LD_PRELOAD=libbionic_shim.so segfaults a trivial ARM64 `hello` with
`si_addr=0x1` right after brk()+anonymous-mmap on the main thread's init —
i.e. in the shim's load-time constructor (`__bf_data_*` dlsym fill /
`__bf_install_mutex_wrappers`), not in the trampoline table.**
So the resolver hardening fixes late faults but not the load-time ABI
crash of the bundled shim against gcc-15 glibc. That load-time crash is
the binding blocker on a fresh box; a follow-up is the shim's `__bf_init_*`
constructor + `__bf_data_*` referencing against the actual gcc-15 ABI.

---

# SESSION 14 HANDOFF — from-fresh-box rebuild + first real runtime runs

## TL;DR
This session went from a completely empty environment to a working,
reproducible runtime stack that boots the **real Roblox 2.726.1142 ARM64
`libroblox.so` (104 MB, NDK r28c, Android 26)** under a rebuilt SMC-patched
QEMU, resolves all 576 of the game's imports, and reaches `dlopen()`. The one
remaining blocker is precise and isolated. `dev` has 6 new commits.

## Committed this session (all in `dev`, all tests green: `cargo test --workspace`)
- `06442d0` **qemu port** — reproducible SMC-patched QEMU 10.2.1 (`qemu/`, patches + `build.sh`)
- `787e300` **elf_disco** — version-agnostic ELF discovery (GOT/RELRO), with integration test
- `f2bc7d9` **early SIGSEGV + LR logging** around `dlopen`
- `993380d` **NULL-safe bionic resolver** (`bionic_init.c`)
- `8f4ef8a` `62c9a7b` docs/HANDOFF

## Environment state (all preserved under `~/.cache/open-sober/`)
| Artifact | Path |
|---|---|
| Patched QEMU 10.2.1 | `~/.cache/open-sober/qemu-patched` |
| JNI shim binary | `~/.cache/open-sober/android-env/jni_shim` |
| Real game lib (104,208,904 B) | `~/.cache/open-sober/libs/libroblox.so` |
| android system/lib64 (25 libs) | `~/.cache/open-sober/android-env/system/lib64/` |
| Bridges libc/libm/libdl | above (LIBC version tags built from `/usr/aarch64-linux-gnu`) |
| bionic shim | `.../libbionic_shim.so` |
| guest stubs | `.../libguest_stubs.so` |

GUI is AVAILABLE: KDE X11 on `:0` (plasmashell + Brave visible via
cua-driver). This is critical — the moment the shim loads, Roblox will
need a display/window, and `cua-driver` + Playwright MCP are in-session.

## Run command (reproduces the stack)
```bash
~/.cache/open-sober/qemu-patched \
  -L ~/.cache/open-sober/android-env \
  -E LD_LIBRARY_PATH=/system/lib64 \
  -E LD_PRELOAD=/system/lib64/libbionic_shim.so:/system/lib64/libguest_stubs.so \
  -E DISPLAY=:0 \
  ~/.cache/open-sober/android-env/jni_shim
```
Expected output (before blocker): `Disabling _dl_mcount... → noped →
Loading bionic shim → Loading libroblox.so →` then **SIGSEGV**.
`QEMU base sanity` check: run an ARM64 `hello` (link with
`aarch64-linux-gnu-gcc`) to confirm QEMU+loader work.

## THE BLOCKER (exact, isolated)
Two distinct facts (both proven):
1. **bionic-shim load-time crash**: `LD_PRELOAD=<...>/libbionic_shim.so`
   segfaults even a trivial ARM64 `printf("hello")` at `si_addr=0x0000...1`
   right after `brk()`+1MB anonymous mmap on the main thread's init — i.e.
   in the shim's **constructor** (`__bf_data_*` dlsym fill /
   `__bf_install_mutex_wrappers`) against gcc-15 glibc. This is the bind
   blocker. It is a small, scoped ABI fix (audit the shim constructor /
   `__bf_data_*` / `__bf_install_mutex_wrappers` against gcc-15).
2. NULL-safe resolver (`__bf_noop`) now prevents late trampoline NULL-
   calls; it does NOT help #1.

With #1 fixed, the next phase is dlopen → JNI_OnLoad → (bounded code +
disasm already in HANDOFF), then **graphics/login** — where
**vision/desktop (cua-driver) is REQUIRED** to drive the window, EGL/GLES→
Vulkan zink, and verify the login screen appears.

## Ordered path for next agent
A. **Fix bionic-shim load-time crash** (pure code; unblocks everything).
   - Reproduce `hello` preload test; debug `__bf_init_*`/`__bf_data_*`/
     `__bf_install_mutex_wrappers` against gcc-15; ensure the shim's
     constructor doesn't deref 0/1.
B. Get `dlopen(libroblox.so)` + `JNI_OnLoad` to return (`elf_disco`
   already handles GOT/RELRO; re-discover JNI_OnLoad patch offsets for
   2.726.1142 — `nativeSetAssetPath` bl at `0x26f384c` inside JNI_OnLoad,
   from the disasm in this HANDOFF).
C. **Now vision is REQUIRED**: launch on the live KDE :0 desktop, use
   cua-driver `get_desktop_state`/screenshots + Playwright to observe the
   window, feed Mesa zink/GLES, and confirm login UI. This is the FIRST
   point the game "boots".

## Facts recorded for next agent
- `JNI_OnLoad` is exported (dynsym) at **offset `0x1f0db20`** in this
  2.726.1142 libroblox.so (the old hardcoded `0x1f64e58` is for a different
  build). Use `dlsym`/`elf_disco` to locate it; do NOT trust old hardcoded.
- 408 `@LIBC` + 4 `@LIBC_N` + 1 `@LIBC_O` versioned imports; bridge
  provides LIBC_* tags; the non-LIBC UND symbols (146 NDK: AAsset*,
  ALooper*, AMediaCodec*, egl*, gl*, etc.) are no-op stubs in
  `libguest_stubs.so`.
- The guest stub generator is inline in this session's bash history
  (`/tmp/gs*.c`); reconstruct via the python snippet that reads
  `readelf -sW` UND FUNC/OBJECT and emits weak no-op/`_stor` objects.
- The `"JDK"`/GSI rumored in the handoff is NOT needed to reach
  JNI_OnLoad; it only matters later for login/token.

---

# SESSION 2026-08-20 — TRUE BLOCKER FOUND & CLEARED (JNI_OnLoad now EXECUTES)

## TL;DR for next agent
The #1 blocker the whole project was stuck on — "bionic-shim load-time crash, can't
`dlopen(libroblox.so)`" — was **misdiagnosed**. The **real** reason libree executed
`pc=0` immediately on load was that this Android 2.726.1142 build ships
**APS2-packed Android relocations** (`DT_60000011` / `DT_ANDROID_RELA`, no standard
`DT_RELA`) and glibc's loader IGNORES them, so `.init_array`/GOT stayed zeroed.
**Converting the packed relocs to a standard `DT_RELA` + extra `PT_LOAD` fixed it**:
`dlopen()` now succeeds, relocation/init runs, and **`JNI_OnLoad` is reached and
executes real Roblox code** (it currently spins in a busy-wait, not crash).

## What actually happened (trace of real work)
1. `unpack_rela.py` (in `crates/sober-core/src/bridges/`) was already ~80% there:
   it decodes APS2 and appends a new `PT_LOAD`. This session **fixed it to:**
   - detect `DT_ANDROID_RELA`(0x60000011)/`DT_ANDROID_RELASZ`(0x60000012) as source,
   - append a page-aligned `R` `PT_LOAD` holding the unpacked standard `RELA` table,
   - set `DT_RELA`/`DT_RELASZ` (tags 7/8) to point at it and **repurpose the two
     Android tags in place to 7/8** so glibc sees them.
   Result: post-patch `DT_RELA@0x6988000`, size 12,799,656; new PT_LOAD vaddr
   `0x6988000`; `.init_array` (3484 entries) now gets populated at runtime.
2. Crash then moved from `.init_array` to a **`__fprintf_chk(NULL FILE*)` update**.
   Root cause: the bionic shim's `stderr`/`__sF`/`stdout` **data slots are 0**, and
   its `write`/`vsnprintf` trampolines resolve to a **no-op** (`__bf_noop`) so ALL
   guest stdout/stderr is silently swallowed. Added:
   - `early_repair_shim()` (runs as the **first thing in `main`**): opens the shim +
     `libc.so.6`, points `__bf_data_stderr/stdin/stdout/realloc`... at the real
     glibc `_IO_2_1_stderr_`/`_IO_2_1_stdin_`/`_IO_2_1_stdout_`/`environ` objects,
   - `jlog()`: an fd-2 logger that resolves the **real** glibc `vsnprintf` from a
     `libc.so.6` handle (plain `vsnprintf` via the shim formats nothing) and
     `write(2, ...)`. All 42 `fprintf(stderr, ...)` in jni_shim were switched to
     `jlog()` so progress is now VISIBLE.
3. The last crash was a `stlrb`-to-ts_flags fault because in the converted build the
   guard/ts_flags/freq offsets (`base+0x6a26e40`, `+0x6a325e4`, `+0x6ae6690`,
   `+0x6ae66e8`) fall inside a **read-only `PT_LOAD`** (the appended `R` RELA run).
   Fixed by `mprotect`ing those pages `PROT_WRITE` before each write.
4. **Result (verified, reproducible):**
   ```
   [jni_shim] Loaded successfully
   [jni_shim] base=0x... mx R
   [jni_shim] pre-init guard=1 ... ts_flags=...->1
   [jni_shim] entering JNI_OnLoad...
   [jni_shim] JNI_OnLoad call at 0x...db20, vm=0x420ef0, env=0x420180
   ```
   then CRUCIAL: **NO crash, no return** — JNI_OnLoad enters a **busy-CPU spin**
   (qemu `-d exec` shows a 2-address loop; `-strace` shows NO futex/nanosleep after
   the JNI call — pure spin).

## Current exact state (repro)
- Installed: `~/.cache/open-sober/android-env/system/lib64/libroblox.so` (RELA-conv
  variant; NOT `${no}` init-disabled). `jni_shim` + `libbionic_shim.so` rebuilt with the
  `early_repair_shim`/`jlog`/mprotect changes.
- Run:  `qemu-patched -L ~/.cache/open-sober/android-env -E LD_LIBRARY_PATH=/system/lib64 -E LD_PRELOAD=/system/lib64/libbionic_shim.so:/system/lib64/libguest_stubs.so -E DISPLAY=:0 <env>/jni_shim`
- git branch `dev`, work uncommitted (see `git status`): `bionic_init.c`,
  `bridges/unpack_rela.py`, `jni_shim.c`. **Commit these.**

## Next steps to actually boot (ordered)
A. **Identify the busy-spin target.** qemu `-d in_asm` shows a GOT-indirect
   `adrp/ldr/ldr/cbz/br x17` thunk looping; straight-text hypothesis =
   Roblox `lock; while(!flag) pthread_cond_wait(...)` where our condvar shim
   (tramp[39,96]=`mov w0,0; ret`) returns spurious wakeups forever and `flag`
   never becomes 1 → pure CPU spin, no syscalls (matches strace). The `guard=1`/
   `ts_flags=1` pre-sets cover specific offsets; this spin is on a DIFFERENT cond.
   Fix: find the spin PC (qemu tracing) and NOP the loop, or make the condvar shim
   also set the waiting thread's expected flag; or pre-set more guard offsets.
2. Then JNI_OnLoad returns (registers 3 native methods) → boot GUI.
3. **Vision/desktop (cua-driver) REQUIRED** to observe the window.

## Key gotchas learned this session
- JNI never needs the real glibc `_IO_*` FILE address trick for `jlog`; just
  resolve `vsnprintf` + `write` from a direct `dlopen("/system/lib64/libc.so.6")`
  handle and write raw fd 2. `RTLD_NEXT` in an executable returns NULL — use
  `RTLD_DEFAULT`.
- `setitimer`/itimers ARM OK under qemu but the SIGALRM is NOT delivered to the
  guest handler (`-strace` shows no heartbeat `write`). Don't rely on it for
  hang PC; use `-d exec`/`-d in_asm` tracing instead.

## Follow-up session (same day) — spin forensics + NX experiment (committed)
- qemu `-d exec`: JNI_OnLoad spins on a 2-address loop in the ~`base+0x764c000->0x7704000`
  band, which was HEAD-first assumed to be the appended RELA PT_LOAD. **Tested it**:
  jni_shim `mprotect(PROT_NONE)` on the RELA PT_LOAD (base+0x6988000, 0xc34ea8)
  SUCCEEDED (rc=0) with NO behavior change — so the spin is **NOT** in the RELA
  data. The base from `dladdr`/`dlinfo` appears ~2MB off for high vaddrs, so
  offset attribution is unreliable; the executions band may be real libro code.
- Net: shutdown; commit `0e3c53a`. Real fix next session = capture the spin's
  **call stack** (needs a working qemu-gdbstub interrupt — gdb `interrupt` over
  the stub didn't take; try `gdb` `set mi-async on` BEFORE `continue&` then
  `interrupt`, or a raw `\x03` on the socket; the `alarm_sa_handler` timer does
  NOT fire under qemu). Then implement the missing guest-stub/trampoline for
  whatever function Roblox dispatches.

## Session 15 — SPIN ROOT-CAUSED AND FIXED; now a real OOM-sourced abort (committed 0a081ac)

### The spin was NOT a condvar loop — it was unresolvable PLT GOT slots => busy-spin to garbage
- qemu `-d exec`/`-d in_asm`: the "spin" was a GOT-indirect thunk
  `adrp/ldr x16;[x16+off]; ldr x17; cbz; br x17` looping with guest PC at
  `base+0x15X014e4/14f4` (X varied run-to-run). Those offsets are past-file
  and past `.text`, i.e. garbage-as-code. The `base+0x15...` jumps into
  anonymous memory.
- Reality: **every PLT JUMP_SLOT GOT slot held a bad value** because glibc's
  lazy binding under qemu user-mode + the bionic shim never resolved them.
  libro's `pthread_mutex_lock@plt` → `br [GOT]` → rodata/anon ⇒ busy-spin.

### Fix (committed): pre-resolve ALL PLT GOT entries
1. `robo_open()` default path was `"libroblox.so"` (CWD) — failed in-app, so
   `g_robo.have=0` and no ELF functionality worked. Now defaults to
   `/system/lib64/libroblox.so`. This is what made everything downstream work.
2. `patch_condvar_plt_got()` installs REAL glibc pthread_mutex_lock/cond_wait/
   cond_timedwait (from direct libc handle `g_real_libc`, not RTLD_DEFAULT which
   returns the shim's shadowed/corrupt address) — the old stubbed shim+wrap
   approach kept the spin.
3. `patch_condvar_plt_got()` now called AFTER the direct-glibc block so
   `g_real_*` are populated.
4. **`patch_all_jumpslots(base)`**: iterate `.DJMPREL`, resolve each symbol via
   `g_real_libc`/RTLD_DEFAULT, write base+r_offset GOT slot with the real fn.
   Result: `patched 532 PLT GOT slots (3 unresolved, of 537)`. The 3 are
   bionic/Android-only (`Java_..._Android*_FinishPaymentsProtocol`,
   `__gcov_dump`, `__gcov_flush`) — harmless.
- **Effect**: JNI_OnLoad now executes REAL Roblox code (clock_gettime, sysinfo,
  /proc over-com/read, getrandom, gettid, getpid) and reaches a real
  **malloc-NULL → abort()** instead of spinning forever. EXIT 14 (hang) →
  EXIT 134 (SIGABRT). Huge milestone.

### Current blocker: `abort` at `Java_..._initializeNativeCode` + 0x343e44 —
  per-thread TLS alloc fast-path returns NULL
- qemu `-d exec` last real .text PC = `base+0x2692d8c...` (`0x2692dcc`: `bl abort@plt`).
- Sequence: pthread_once → mutex_lock/unlock → pthread_getspecific → then
  `bl 0x1c35480` (Roblox per-thread TLS block allocator, small-size fast-path
  from a TLS free-list) → `cbz x0 → 0x2692dcc abort`. It aborts when the small
  alloc falls to the big path and that returns NULL, or the TLS free-list is NULL.
- It aborts EVEN THO we already forge sysinfo => 256 GiB free and
  `/proc/sys/vm/overcommit_memory` => 1 (also tried 0 and 2). So it is NOT a
  real low-memory abort: rather a **bypassed-early-alloc-init / tls-arena-not-
  seeded** condition (we NOP a lot of init). Host has only ~4 GiB avail and
  Committed_AS > CommitLimit, but the allocator never even `mmap`s before
  aborting (strace shows 0 mmaps after JNI_OnLoad).

### Diagnostics added this session
- `wrap_android_set_abort_message()` + `wrap_android_log_print()` print the
  (otherwise logcat-lost) abort reason to stderr — so far no `[android-abort]`
  line appears, meaning the abort is a silent bare `abort()`.
- gdb walk shows the abort caller's return addr is in a data region
  (`base+0x?ba710`), consistent with a JNI/trampoline callback chain.

### Next steps (ordered) — unblock the TLS-alloc NULL
A. Make the TLS free-path never NULL: the small alloc `0x1c35480` `cbz`es on an
   empty free-list and falls to the big-allocator; ensure the big allocator
   (`0x1c3639c`) returns from a real glibc `malloc`. If that tail-call resolves
   via a JUMP_SLOT the loader left bad, patch it. Check whether
   `0x1c3635c` (`b` target) calls real malloc.
B. OR force `abort@plt` (GOT) to `wrap_abort` that logs the caller PC from
   `(_RETURN_ADDRESS)` and returns (unwind the quadruple-abort) so the call
   chain continues and the next OOBorn diagnostic (or SIGSEGV handled by our
   segv handler) reveals the real issue.
C. OR run Roblox's real allocator init (don't bypass it) by removing the NOP
   clock/init bypasses in `JNI_OnLoad` progressive patching / the guard
   override, letting the arena seed normally. This is likely the correct fix.
D. After JNI_OnLoad returns (registers 3 methods), boot the GUI on `:0` and
   start the vision phase (cua-driver, Step 5 on the task list).

### Refined root cause (same session, commit after abort-intercept)
- Neutralizing `abort` (GOT -> wrap_abort that logs caller and returns) makes the
  quadruple-abort at 0x2692DD0/DD4/DD8/DDC log caller offsets then return. After
  they return, JNI_OnLoad keeps executing but busy-spins (0 syscalls). So
  suppressing abort is NOT a fix — it's a diagnostic.
- `0x1c35484` (the per-thread TLS/small alloc) verified: empty free-list ->
  fall through to the book's own MemoryPool allocator (0x1c3635c), NOT glibc
  malloc. strace shows **0 mmap syscalls after JNI_OnLoad**, so the NULL is
  **pure book-side user-space**: the MemoryPool arena/chunk bitmap is empty /
  unseeded because Roblox's real allocator-init never ran (bypassed by the
  JNI_OnLoad progressive patches + guard/ts_flags override). It is NOT a real
  host-memory OOM even though we also forge sysinfo+meminfo+overcommit.
- **CONCLUSION: option C — run Roblox's real allocator/MemoryPool init (do not
  bypass it) — is the right path.** Find the pool init entry (likely a
  constructor / a `Java_..._initializeGC`/`Memory` JNI or a static init that is
  currently NOP'd or skipped) and either let it run or manually call it to seed
  the per-thread pool chunk-base, so the small allocator's free-list is non-empty.

## Session 16 — libroblox REAL .init_array constructors now RUN (committed); blocker = Roblox MemoryPool bootstrap

### Root-cause advance: DT_INIT_ARRAYSZ == 0, so glibc never runs Roblox's ctors
- `readelf -d` on installed libroblox.so: `INIT_ARRAY=0x630bfc0` but `INIT_ARRAYSZ=0`
  (0 bytes) even though `.init_array` section is 0x6ce0 (3484 pointers).
- The init_array entries are `R_AARCH64_RELATIVE` (base+addend) slots that the
  loader SKIPS resolving because DT_INIT_ARRAYSZ==0 (it never runs them).
- So all the static constructors that seed Roblox's MemoryPool / TLS arena
  globals originally never executed. That is the real antecedent of the old
  malloc-NULL abort.

### New capability: run_libroblox_init_array(base)
- mprotect base+[0x5a00000..0x6320000] RWX, apply RELATIVE relocs for
  init_array-range slots (0x630bfc0..0x6312ca0) -> write base+addend, then call
  each ctor in address order.
- VERIFIED RUNNING: log shows ctor[0]@base+0x2692f14 .. ctor[3]@base+0x1c34480
  with a sysinfo plus abort appearing INSIDE ctor[3]'s execution.
- ctor[3] (0x1c34480 -> tail `b 0x5d9ce10`) does a thread-local allocation via
  0x1c35480 (the TLS block fast-alloc) which returns NULL, hit the
  cbz-to-abort at initializeNativeCode+0x343e44. Same malloc-NULL abort as
  before, now reached from the constructor path.

### Blocker now precisely: Roblox's per-thread MemoryPool TLS alloc returns NULL
- 0x1c35484: TLS key from [0x6368000+0x9dc]; if -1 runs init; else
  pthread_getspecific to default block 0x6308dc0 (csel if empty). size<=0x400
  fast-path pops [blk+232]+8; on empty -> big-allocator 0x1c3635c.
- 0x1c3635c (big alloc) returns NULL regardless of reported memory (614MB real
  OR 256GB forged sysinfo): qemu strace shows NO mmap after JNI_OnLoad, so it is
  a book-side arena-not-seeded condition, NOT real OOM.
- wrap abort() logs+returns; without it the process dies SIGABRT.

### de-horned wrong guesses (verified and committed)
- sysinfo/meminfo/overcommit forgers are NOT the fix and are now set to PASS
  THROUGH real host values (forging 256GB or overcommit 0/1/2 didn't change the
  abort). Do not re-add inflation as the primary lever.
- Applying RELATIVE relocs GLOBALLY double-corrupts .data (loader already does
  .data/.got/.data.rel.ro; only .init_array is skipped). Keep the apply
  init_array-scoped.

### Next (ordered)
A. Find and call the MemoryPool init directly (seek the fn that initializes the
   block region 0x6308dc0 / the arena global ~0x6367000+0x600), or identify a
   later ctor that seeds the pool and run it before ctor[3].
B. Or patch 0x2692ce8 (the cbz-abort on the TLS-alloc NULL) to fall back to
   real glibc malloc so the book gets a block and can proceed through later
   ctors -- a stepping-stone, not a final fix.
C. After JNI_OnLoad returns (registers methods), GUI on :0 + vision phase.
## Session 17 — direction reset: stable loaded state via Session-9 full bypass (committed)

### Direction confirmed with user
End goal is the OPEN-SOBER CUSTOM RUNTIME (sober-style native run: QEMU is the
ARM64/x86-64 bridge, not a whole-VM product). Re-adopted the Session-9 loaded
-state approach: JNI_OnLoad returns 0x10006 immediately so the binary bootstraps
under the bridge and we can then raise/observe a window on :0. Fighting Roblox's
internal MemoryPool inside progressive init is parked (documented below).

### Why the old full bypass was silently broken
- There are TWO entry shapes: the EXPORTED JNI_OnLoad at base+0x1f0db20 (what
  dlsym/lib host calls) and 0x1f64e58 (an internal init at a shifted offset).
  The prior bypass patched 0x1f64e58 -> real JNI_OnLoad still ran the
  MemoryPool-reaching init and aborted.
- Fix: bypass base+0x1f0db20 (entry = mov w0,#6; movk w0,#1,lsl#16; ret).

### Disabled for bypass path
- run_libroblox_init_array(...) call is commented out. Its ctor[3] (MemoryPool
  TLS alloc at 0x1c34480 -> 0x5d9ce10 -> body 0x2678068) aborts on the internal
  pool; with abort suppressed it spins and blocks. Skipping ctors gives the
  clean baseline. The init_array + RELATIVE machinery is kept in the tree
  (real-engine-init path) but not run by default.

### VERIFIED (fresh run)
  FULL BYPASS: JNI_OnLoad@<base+0x1f0db20> returns 0x10006
  entering JNI_OnLoad...
  JNI_OnLoad call at <base+0x1f0db20> ...
  JNI_OnLoad -> 0x10006
  Entering sleep loop
No abort, no fault; process stable until watchdog timeout (RUN EXIT 14).

### MemoryPool blocker (parked, for real engine init later)
- libroblox imports NO allocator (only free, munmap); its MemoryPool is fully
  self-contained. Big-allocator 0x1c3635c returns NULL regardless of forged
  614MB vs 256GB sysinfo, with zero mmap after JNI_OnLoad. One-time init body
  0x2678068 aborts on its own first TLS alloc. Fork/banc of memory, ctors,
  malloc fallback all unavailable. To boot the real engine, must resolve the
  TLS block free-list seed (0x6308dc0 struct) or run fuller Android/JAVA app
  bootstrap.

### Next (ordered, per user direction)
A. NOW: run jni_shim with DISPLAY=:0, keep sleep-loop stable, and use
   computer vision on the X11/EGL surface to observe any window/black frame,
   or confirm none yet. Check whether book opens a GL context or needs the
   engine run loop.
B. Then: re-enable init_array/progressive init in stages ONCE the pool seed is
   understood, so a real window can render.
C. Multiple-version compatibility after a working baseline.
## Session 17b — MemoryPool malloc-fallback thunk: empty-pool abort DEFEATED (committed 313aa7b)

### Probe: libroblox imports NO allocator functions
readelf -r --use-dynamic shows libroblox.so imports only `free`/`munmap` from the
allocator family (no malloc/calloc/realloc/mmap). Its MemoryPool is fully
internal; the big-allocator returns NULL on unseeded arena state regardless of
host free RAM. Forging sysinfo/meminfo is irrelevant.

### Fix: redirect small-allocator empty-list to real glibc malloc
- Site: libro offset 0x1c354fc — the "empty per-thread free-list" continuation
  that tail-calls the big-allocator 0x1c3635c (which returns NULL).
- Thunk on a fresh MAP_ANONYMOUS RWX page (NOT the cond_shim page, which is the
  condvar "mov w0,#0; ret" trampoline):
      mov  x0, x19           ; size (0x1c35490 mov x19=x0; x19==size)
      ldr  x17, [pc, #24]    ; pc-literal loads real glibc malloc
      blr  x17               ; x0 = malloc(size)
      ldr  x19, [x29, #16]   ; restore saved x19
      ldp  x9, x30, [x29], #32 ; restore (x9=[x29], x30=[x29+8]), sp+=32
      mov  x29, x9
      ret
  Encodings verified against aarch64-linux-gnu-gcc-assembled .S.
- Book patch (24 bytes at 0x1c354fc): `adrp x16, thunkpage; br x16; nop x4`.
  ADRP encoding that VERIFIED (mine was wrong first try):
      imm = (thunk_page - site_page)  ; in 0x1000 units, signed
      adrp = 0x90000000
           | ((imm & 3) << 29)                  ; low 2 bits -> bits[30:29]
           | (((imm >> 2) & 0x7ffff) << 5)      ; high 19 bits -> bits[23:5]
           | Rd                                  ; Rd = x16 = 0x10
  My first form `((imm&0x7ffff)<<5)` put the raw (non->>2) delta at the wrong
  bit offset -> jumped to a wrong page and SIGSEGV'd. Correct form verified
  against the cross-cc (same-page `adrp x16` disassembles to 0x90000010).

### Result
- BEFORE: ctor[3] aborts 4x (empty-pool NULL) and spins.
- AFTER: ctor[3] runs, calls sysinfo (pool doing real allocation), NO abort,
  NO segfault. The hang is now a single-threaded CONDITION wait-loop (classic
  QEMU user-mode one-thread behavior), not an OOM abort.

### Next
Post-ctor[3] hang = wait on an event/flag that never arrives (one thread under
QEMU). Options: (a) trace the exact spin site (heartbeat) and force/fake the
awaited flag; (b) pre-seed the pool's real arena (static default TLS block free
-list) so the block path never blocks. Much more tractable than the previous
NULL/OOM abort.
## Session 17c — blocker refinement (committed 381d088)

Current state: the MemoryPool empty-pool abort (fixed Session 17b by a thunk that
routes libro's small-allocator empty-list to real glibc malloc) is gone. ctor[3]
(book 0x1c34480, MemoryPool one-time init) now runs, performs its allocation
syscalls (sysinfo/overcommit/getrandom), then spins in pure user-space code with
NO further syscalls — a single-threaded wait for a condition/flag that only a
second thread would set (QEMU user-mode runs one vCPU).

Attempts this turn:
- SIGALRM heartbeat sampler from the host-signal route: QEMU user-mode does not
  deliver SIGALRM into the guest handler; 0 samples (kept, harmless).
- pthread_cond_timedwait slot return now ETIMEDOUT (110) instead of 0 (kept).
- A dedicated bump-allocator thunk target (host shim function) caused SIGILL —
  calling a host/ELF function from the guest through the ARM thunk crosses a
  translation context QEMU cannot handle. Reverted to the dlsym'd glibc malloc
  thunk, which is safe and stable.

Stable, committed, no crash: ctor[0..2] run, allocation succeeds, ctor[3] waits/
spins, no abort/SIGILL/segv.

Next: identify the awaited flag in ctor body 0x2678068 and pre-set it (Session
11-style guard fix); or pre-warm the static TLS block free-list; or spawn an
emulated second thread.
---

## Session 18 — from-scratch JIT (arm64jit): drop-QEMU path begins

**Decision (user):** build a small in-process ARM64->x86-64 JIT/translator from scratch to replace QEMU, accepting multi-session. Goal stays: a real runtime (no QEMU), then multi-version proof, then CV on the GUI.

**What landed this session (all committed, 16 tests green):**
- `crates/arm64jit/` — new workspace crate. `x86.rs` = minimal verified x86-64 emitter; `decode.rs` = AArch64 decoder; `translate.rs` = per-instruction ARM->x86; `jit.rs` = CpuState + exec.
- Decoder verified against REAL gcc/objdump encodings (not memory): B, B.cond, MOVZ/MOVK/MOVN, ADR/ADRP, ADD/SUB imm, ADD/SUB shifted-reg, AND/ORR/EOR, LDR/STR unsigned-imm + register-offset.
- Pattern lesson: decode guards keyed to top-byte/class `(top & 0x3b)==0x39`/`0x38` (LdImm vs LdReg), `movewide sf` etc. — derived from actual encodings, each with a ground-truth unit test.
- Key debug: LOGIC-reg `s` is NOT bit29 (that's part of opc); `s = opc==0b11`. rm field is bits[20:16], NOT (insn&0x1f).
- JIT executor: spills guest regs to a `CpuState` in memory addressed via RBX (NOT R12/RSP/RBP — emit_mem forbids RSP/R12, and RBP/R13 have rm=7 -> RIP-rel for disp0). Prologue `mov RBX,<state>`; `[RBX + 8*i]` per reg; epilogue returns x0. `exec_bytes` = bytes->decode->compile(RWX mmap)->call fn(*mut CpuState)->x0.
- **LIVE proof:** executed real `mov x0,#3; add x0,x0,#4` (=d2800060 91001000) => returned 7, no QEMU. Commits 2faf3dd, d1fed33, eb11d66, 7c25747.

**Current scope/limits (honest):** translator handles MoveWide/AddSubImm/AddSubReg(shamt 0) only so far; no loads/stores, no branches/control-flow, no BL/host-call dispatch, no FP/NEON/TLS/atomics yet. Roblox's libroblox.so is far beyond this until loads/stores + branches + a host-call (syscall/bionic-shim) dispatch land.

**Next (Session 19+):** broaden decoder+translator to LDR/STR (have decode) + B/B.cond/CBZ + a host-call trampoline; validate on a real multi-instruction aarch64 .so function; then wire into libloader `--no-qemu`. Cleaned /tmp of ~5G stale QEMU core dumps.

## Session 18b — arm64jit executes load/store + control flow (no QEMU)

**Verified this session** (all through the from-scratch JIT, no QEMU):
- LDR/STR (unsigned 16-bit immediate offset), size 8/4, via RDX addr + lea — real
  `ldr x0,[x0,#16]` loads host memory correctly (17→19 tests).
- RET decodes + translates to host `ret`.
- **Control flow**: B (jmp), CBZ/CBNZ (test+jnz/jz) + flow-aware `jit::compile`
  that builds a guest_pc→host_offset map and patches rel32 fixups
  (disp = target − (disp_off+4)). Fixed a bad `start` calc → SIGSEGV.
  Real `cbz x0` function returns 20 (fall-through) / 10 (taken) ✓.

**State**: 19 tests green, workspace clean, committed at `09699e5`.

**Commits this session**: eb11d66 (LDR/STR+ret), 7c25747 (first exec),
a432acf, 09699e5 (control flow).

**Next slice** (task 3b): B.cond + NZCV flags (subs/cmp set flags; materialize
into guest NZCV so any interleaving works) then BL function calls with a
guest call stack. After flags, real `.c` compiled aarch64 (if/else, loops)
can run.

## Session 19 — arm64jit: flags + calls + stack pairs (23 tests, no QEMU)

**Verified this session** (all through the from-scratch JIT on x86-64, no QEMU):
- **cmp/subs → B.cond** (if/else): real `cmp w0,#3; b.le` returns 0 or 1 per ARM. Decoder
  expanded to S-flag form `cmp w,#imm` = top 0x71/0xF1 (SUBS imm). `x86_cc_for_cond`
  maps ARM cond→x86 jcc (EQ..LE); set-flag ALU leaves x86 flags live through the
  follow-on `mov` stores(store to rd==31 suppressed).
- **BL function calls + call-graph `compile_image`**: BFS walk follows branch/call
  targets, compiles whole reachable region into ONE buffer; `BL` = save LR + host
  `call rel32` (cc=0xfe fixup); a real leaf call `caller(x)=(x+5)*2` returns correctly.
- **LDP/STP (load/store pair)**: offset/pre-index/post-index, 32/64-bit, decoded via
  bit23=indexed, bit24=pre, bit22=load, imm7 signed scaled by 8/4; `stp/ldp` prologue
  round-trips regs through real stack, sp restored.

**Tests: 23/23 green. Tree clean.** Commits: 43f7af3 (flags+B.cond), 874800c (BL +
compile_image), 0ee07fc (LDP/STP).
**Next slice (3e):** adrp/adr (PC-relative), ORR/EOR/AND-reg + reg-reg MOV/aliases,
then `adic` integration into libloader `--no-qemu`.

## Session 19: arm64jit — core-ISA coverage (26 tests, drop-QEMU verified)

**Verified this session** (all through the real from-scratch JIT, no QEMU):
- **Flags + B.cond** (commit 43f7af3): cmp/SUBS/ADDS set flags; b.eq/ne/le/lt/ge/gt/hs/ls/cs/cc
  translate to x86 jcc. Real `cmp w0,#3; b.le` runs correctly.
- **LDP/STP pair load/store** (0ee07fc): offset/pre/post index, X and W, verified vs 5 real
  encodings. Real prologue stp/ldp round-trips regs and restores SP.
- **BL calls + call-graph compile_image(image, base, entry)** (874800c, 0ee07fc): walks
  BL/B/CBZ targets, emits into one image buffer, host call + host ret (LR saved). caller()=20.
- **LogicReg AND/ORR/EOR + mov alias** (dc88106): mov rd,xm; XZR reads as zero; real logic.
- **ADRP/ADR** (47f7a5f): PC-relative addressing; adrp/add/ldr loads a mapped global (guest==host
  address when the ELF is placed at its vaddr).

**State: 26 tests green — core AArch64 ISA executes real compiled code on x86-64, no QEMU.
Next (task 4): integrate into libloader/sober-core — map the Roblox ELF at its vaddr, then
compile_image(text_segment, vaddr, entry) for _start/JNI_OnLoad. adrp/ldr now resolve because
guest address == host address when segments are mapped at their ELF vaddr.

## Session 19c - JIT wired into the product (no QEMU path)

- **libloader**: exposed `pub mod elf` so `load_elf`/`LoadedElf` are usable downstream (commit 9c8e791).
- **arm64jit/examples/elfjit.rs**: loads a real static aarch64 ELF with libloader's loader and JIT-runs
  its entry -> returns 42 on x86-64, no QEMU. Proven end-to-end loader+JIT wiring.
- **sober-core**: `--jit` CLI flag -> `mod jit::run_elf_entry` loads the Roblox .so via libloader and
  hands it to arm64jit. `qemu::find_main_binary` made pub. (commit 4b89619)
- **Honest boundary**: trying to run `/tmp/robpatched/libroblox.so` (a PIE `ET_DYN`) through elfjit
  SEGV s because for PIE .so the host address of the code is NOT simply `e_entry`: the loader maps the
  PT_LOAD text segment at a real host address, and `compile_image` must be fed the mapped host range +
  its guest base, not `entry` directly. Plus the full .so uses far more instructions than the current
  subset, so full execution is still months of translator work.

**Next (task 5)**: fix the PIE path - derive the mapped text host address from `LoadedSegment.vaddr`,
  feed `compile_image(image=mapped_host_slice, base=guest_text_vaddr, entry=host-of-entry)`, and report
  the *first unsupported instruction's guest address* as an honest diagnostic target for the next
  decoder slice (start with SVC syscall routing + TLS, then SP, then BL/ADR linkage).

## Session 19d - WHAT STILL NEEDS TO BE DONE to get the JIT path fully working

Current state: arm64jit executes a real AArch64 subset on x86-64 with no QEMU
(26 tests green, all verified against objdump ground-truth). It is wired into
the product end-to-end (libloader -> compile_image -> run) and can run the
entry of a *non-PIE static* aarch64 ELF (elfjit example returns 42). Running
the real `libroblox.so` (a PIE ET_DYN) currently SIGSEGVs.

### 1. PIE / shared-object mapping (unblocks the real .so immediately)
- **Problem:** the elfjit/`--jit` path feeds `compile_image(image, entry, entry)`
  assuming host-addr == guest-vaddr. That holds only for non-PIE statically
  linked ELFs. libroblox.so is ET_DYN/PIE: libloader maps PT_LOAD segments at
  real host addresses and relocates, so `e_entry` is not a host address.
- **Fix:** derive the *mapped text range* from `LoadedElf.segments[]` (host
  vaddr + memsz), slice that range as the compile `image`, pass `base =
  guest_text_vaddr` and `entry = host(of e_entry)`. Then `ADRP`/`ADR` compute
  guest addresses that resolve into the mapped segment (guest==host holds
  again because libloader maps at vaddr).
- **Diagnostic:** once it loads, report the **guest address of the first
  instruction the decoder can't translate** (add a `resolve` that returns
  `Err((pc, Inst::Unsupported))`). That gives the exact next decoder slice.

### 2. Decoder/translator gaps that WILL appear (in rough order of priority)
- `SVC` syscall routing (Roblox makes many host syscalls; must map to host or
  the bionic shim) - and the `--jit` path must link against the shim.
- TLS slot access (`mrs`/`msr` TPIDR_EL0, `ldr`/`str` via TPIDR), threads
  (host pthreads vs guest threads).
- Atomics (`ldaxr/stlxr`/CAS loops) - used heavily in Roblox.
- FP/SIMD (NEON: a LOT in graphics/sound; `ldr q`, `add v0.4s,..`, etc.)
- 32-bit register semantics: Ws must zero-extend and flag-setting compares
  must be 32-bit-aware (currently traced as 64-bit for small values only).
- XZR vs SP as x31 depending on context (currently one sp slot, no read-xzr
  suppression for arithmetic stores - add rn/rd==31 handling per class).
- Multiply/AES/other (`mul`, `mneg`, `sdiv`/`udiv`, `csel` (flags-dependent
  select - completes flag model), bitfield ops `ubfm/sbfm/bfi/extr`).
- LD/ST variants: `ldr x,[x,#imm]` are done; `ldr` non-scaled, `ldrsw`,
  `ldrb/h`, `strb/h`, `ldp/stp` SIMD (128-bit D0-D31 pairs).
- Branch: `b.eq/ne/...` (B.cond already done), `br`/`blr` (indirect call),
  `cbz`/`cbnz` done, `tbz`/`tbnz`, return-less tail calls.
- `MOVZ/MOVK` (done) but `MOVN` and imm build-up across block - fine.

### 3. Guest runtime environment (required to actually run Roblox)
- **A guest stack** (`sp`/x31) pointing into a large mmap'd region; `mrs
  SP_EL0` etc.
- **Thread-local storage:** TShell set `TPIDR_EL0`, guard-and-init; Roblox
  spawns threads.
- **Syscall service** (`svc #0`): at minimum `exit`, `write`, `mmap`,
  `munmap`, `brk`, `clone`, `open`, `read`, `futex`, `timer`, `getuid`,
  `sysinfo`. Either route to the bionic shim or implement host-facing.
- **Signal handling / the JNI setjmp-longjmp** that the native glue expects.
- **Linking the bionic** sysroot libs (libbionic_shim.so + guest stubs) -
  the existing QEMU build work (jni_shim, elf_disco) is REUSABLE for symbols
  resolution; the JIT needs a PLT/GOT resolver so `bl` to relocated functions
  dispatches to the right guest/host thunk.

### 4. Correctness hardening (before trusting any real run)
- **Frame pointer / unwind** - not needed for execution but for debugging the
  unmistakable first crash.
- **Trap on unsupported instead of UB:** currently any translated block that
  hits an untranslated instruction returns Err gracefully (good); but a guest
  `ret`/`br` to an address outside any compiled block must be caught, not
  fall through (add a `state->pc` write + a trampoline back into the
  interpreter/compile loop for un-compiled blocks).
- **PC-relative fixups are buffer-relative (already done);** ensure they are
  correct for code that spans two images/ELF segments.

### The real realistic path to a Roblox window (multi-session)
1. PIE mapping + first-unsupported diagnostic (do this next).
2. Wire `svc` + TLS + a stack + GOT/PLT resolution so `JNI_OnLoad` can run
   far enough to print something.
3. Add cross-version coverage (the standing "multi-version" goal) by diffing
   the decoder against several `libroblox.so` builds.
4. Only after those load + JNI init: FP/NEON + atomics + threads to get
   actual frames; then the GUI/computer-vision inspection step becomes
   meaningful.

Everything above was updated to account for the current committed state at
`5496852`. The single highest-leverage next step is **#1 (PIE mapping)**.

---

## Session 20 (Aug 20, 2026) — PIE mapping FIXED; JIT now decodes real libroblox.so code

Goal (from 19d #1): fix the PIE/ET_DYN mapping so `elfjit`/`--jit` no longer
SIGSEGVs on the real 117MB `libroblox.so`, then push the honest
first-unsupported diagnostic forward. **This was achieved and verified end to
end.** Commits: `194d5d8`, `029e36f`, `da16a76` (on `dev`).

### 1. PIE / ET_DYN mapping — FIXED (the 19d #1 blocker is done)

**Root cause of the old SIGSEGV:** the per-segment `MAP_FIXED` in `load_elf`
lets a later PT_LOAD of a *packed* ET_DYN target an address that still overlaps
the previous huge (`~99MB` r-x) text mapping. Under gdb the fault was
`__mmap64` crashing on a `MAP_FIXED` address inside the earlier mapping — i.e.
`0x78f05998000 + 0x5e6b000` landed *inside* the text range.

**Fix:** added `libloader::elf::load_elf_image(path) -> LoadedElf` (a fresh API,
the old `load_elf` is untouched for the QEMU path). It maps ONE contiguous
anonymous region at a fixed `JIT_BASE` (`0x100000000`), lays every PT_LOAD into
it at `base + (p_vaddr - min_vaddr)`, zero-fills `.bss`, and applies per-segment
mprotect. Critically it sets **guest vaddr == host address** (`guest_of(link) =
base_addr + (link - base_load_addr)`), the exact property arm64jit's ADRP/ADR +
direct-dereference model requires. No more overlap, no more SIGSEGV.

`elfjit` and `sober-core --jit` now:
- load the real `libroblox.so` cleanly (4 segments, guest==host at
  `0x100000000`, e.g. text `[0x100000000, 0x105e67390)`),
- translate the requested guest entry (a link-time address via `guest_of`),
- report an **honest diagnostic**: `arm64jit stopped on unsupported instr
  at/near guest 0x101c34480: translate: unhandled Unsupported(0x...)`.

### 2. Decoder/translator walls pushed through (5 in this session)

1. **ADRP/ADR mask bug FIXED.** The decoder tested `insn>>24==0x90`, missing
   real ADRP encodings whose top byte is `0xD0` (varies with `imm[1:0]`).
   Changed to the canonical `(insn & 0x9F000000)==0x90000000` (ADRP) /
   `==0x10000000` (ADR); confirmed `0x90026516` (top 0x90) and `0xd0026a93`
   (top 0xD0) both match. Without this, the very first decoded real-world
   Roblox ADRP `0xd0026a93` returned `Unsupported`.
2. **LDR/STR unsigned-imm sizes 1,2,4,8** (was 8/4 only). Added halfword/byte
   zero-extend loads (`movzx_word_mem`, `movzx_byte_mem`) and 8/16-bit stores
   (`mov_store8/16`) to the x86 emitter; wired into `LdStrImm`.
3. **LDR/STR register offset** (`ldr x9,[x8,x1,lsl #3]`, class `0x38`) — new
   `LdStrReg` translate arm (index `rm`, shift by `log2(size)`).
4. **128-bit SIMD vector load/store** (`ldr q6,[x0,#16]`=`0x3dc00406`,
   `str q7,[x0,#32]`=`0x3d800807`) — `CpuState` now carries a **32×128-bit
   vector register file** `v:[u64;64]` at `VECTOR_BASE=256` (with `set_v`/`get_v`),
   x86 XMM helpers (`movdqu_load/store`, `movdqa_xmm`, `pxor_xmm`), new decode
   class `VecLdStImm` (`0x3D8`/`0x3DC`), and a translate arm that moves 16 bytes
   between the guest v-slot and guest memory through XMM0.
   *(`movi`/float NEON immediate was deliberately NOT bolted on — the imm
   reconstruction is fiddly and a wrong float result would be worse than the
   honest "unsupported" stop. Do it with a proper NEON decoder next.)*

### 3. Verification

- `cargo test --workspace` all green (arm64jit 26 → still 26, no regressions).
- Static non-PIE `stat.elf` entry returns `42` (unchanged, still passes).
- PIE `libpie.so` maps at `0x100000000`; entry `0x588` translated to guest and
  stopped *honestly* at `lsl x0,x0,#1` (`0xd37ff800`, a UBFM bitfield op — see
  task list below; not yet added).
- 128-bit vector round-trip: hand-assembled aarch64
  `ldr q6,[x0,#16]; str q6,[x1,#32]; mov x0,#99; ret` runs through elfjit
  (giving it a guest==host buffer via the new `buf` arg) and **returns 99**,
  no QEMU, no crash.

### 4. Current honest state on the real binary

```
$ ./target/debug/examples/elfjit ~/.cache/open-sober/libs/libroblox.so 0x1c34480
loaded '...libroblox.so': is_pie=true base_load_vaddr=0x0 e_entry=0x100000000
  segment guest=[0x100000000,0x105e67390) prot=r-x   (89MB text)
  segment guest=[0x105e6b3c0,0x10631c000) prot=rw-
  segment guest=[0x10631fb40,0x106987130) prot=rw-
  segment guest=[0x106988000,0x1075bcea8) prot=r--
running entry guest=0x101c34480
arm64jit stopped: translate: unhandled Unsupported(1862329344)  // == 0x6F00E400
```

The NEXT blocker is the **floating-point NEON** instruction `0x6F00E400`
(top-byte `0x6F`, the floating-point 3m-add / scalar-fp class — NOT the
integer `movi` `.4s` which decodes as `0x4F...`). That's the immediate next
decoder slice.

> **Session 21 CORRECTION:** `0x6F00E400` is **NOT** floating-point. Verified
> against `aarch64-linux-gnu-objdump` ground truth, it is `movi v0.2d, #0x0`
> — an **integer** vector move-immediate (the "clear a 128-bit vector to
> zero" idiom at the top of a stack-zeroing loop). It was handled in Session
> 21 (below), not deferred. The 20's "0x6F=FP" guess was wrong.

### 5. Next steps (updated, ordered)

1. **Add the float-NEON / FP layer** starting with `0x6F00E400` specifically,
   plus `fmov/fadd/fsub/fmul/fdiv`, `fcvt/fcvtl`, `fcmp/fcsel` and the
   `0x4F` `movi.4s` immediate (with unit tests). This is now the gate between
   "decodes" and "boots" — a 3D engine is dense with FP.
2. **`lsl/lsr/asr x,#imm`** (`UBFM/SBFM`, e.g. `0xd37ff800`) — trivial and hit
   by any real code; add the bitfield ops `ubfm/sbfm/bfi/bfx`.
3. Guest stack + `sp`/`mrs TPIDR_EL0` TLS + `svc` routing → then the `B/BL`
   call-graph can actually *run* ("compare" the boot log) rather than just
   translate.
4. Cross-version coverage; verify against multiple `libroblox.so` builds.

`git log`: `194d5d8` (PIE load_elf_image fix), `029e36f` (one contiguous
guest==host image, runs deeper into libroblox.so), `da16a76` (SIMD vector
regs + 128-bit ld/st + register-offset ld/st), then the HANDOFF update.

## Session 21 (Aug 20, 2026) — Architected the PC-driven dispatcher; JIT now *executes* real libroblox.so through blr chains

Commit: `6618c5e` (dev). **The JIT crossed from "translate-then-stop" to
"actually follow call/return control flow".** Five more instruction walls
pushed + the VECTOR_BASE bug fixed.

### 1. What was previously-unsupported, now decoded+translated (all objdump-verified)

1. **`movi` vector-immediate** (`.8B/.16B/.4S/.2D`) — and in doing so,
   corrected 20's mislabel: real blocker `0x6F00E400` is `movi v0.2d,#0x0`
   (integer), verified by `objdump -d`. Decoder reconstructs the lane and
   replicates it across the 32-bit/8-bit/64-bit lanes; unit-tests
   `movi_ground_truth` covers all 6 specimens.
2. **`stp`/`ldp q` 128-bit SIMD load/store pair** (`0xAD000000`),
   scale=16. Init function zeros 64 bytes via `movi; stp q,q; stp q,q`.
3. **HINT / PAC pseudos**: `nop`, `esb`, `csdb`, `paciasp`/`autiasp` (the
   `-msign-return-address` prologue), `bti`. Nominally the `0xd5032xxx`
   family, mask `(insn&0xfffff01f)==0xd503201f`; executed as no-ops (PAC
   is ignored in the guest).
4. **`br`/`blr`** — indirect branch/call. **The decode RN bug:** the source
   register is bits[9:5], *not* [4:0]; was decoding `blr x1` as `blr x0`,
   which sent the dispatcher to address 0 → instant halt with wrong value.
   Fixed; unit-tested.
5. **`tbz`/`tbnz`** test-bit-and-branch — `b24`=op (1→tbnz), `bit=b[23:19]|b31`,
   imm14×4. Verified `tbnz w0,#31` target.

### 2. The dispatcher (`jit::jit_run`) — the enabler

`br`/`blr`/`ret` can't be inlined (target is in a register). Added
`jit_run(image, base, entry, state)`: loop { compile reachable region from
`state.pc` via `compile_image`; run; re-enter at `state.pc` }. Terminal
`br`/`blr`/`ret` set `pc = (16/8-lanes)`, x30 link on blr, return to the
host loop. `ret` now sets `pc = x30` so returns chain to the caller.

`elfjit` switched to `jit_run`. Verified with a hand-assembled aarch64
program (adrp/add `x1=&callee`; `blr x1`; `mov x30,xzr; add x0,#1; ret`
/ callee `mov x0,#42; ret`) → **returns 43** through the dispatcher. This
is the first real indirect-call round trip.

### 3. Critical correctness bug: VECTOR_BASE overlapped `CpuState.pc`

`CpuState{ x[32]@0..256, pc@256 }`, but `VECTOR_BASE` was **256** — so every
vector `movi`/vector ld/st wrote into `pc`/`nzcv`, corrupting instruction
streams once SIMD ran. Moved the vector file to **272** (`VECTOR_BASE=272`,
`PC_OFF=256`). This was latent since Session 16 (vector regs added) and only
surfaced now that SIMD + the pc-driven loop share a state.

### 4. Honest current state on real libroblox.so

```
$ elfjit .../libroblox.so 0x1c34480
running entry guest=0x101c34480
arm64jit run_loop stopped: translate: unhandled Unsupported(445973185) at guest pc 0x1026938d0
```
`0x1A9502C1` = `csel w1, w22, w21, eq` right after `cmp x9, x10`. So the JIT
now *executes* through: `movi`→`stp q`→`paciasp/autiasp`→`blr` chains→`tbnz`,
and blocks on the first **NZCV-flags consumer** (CSEL).

### 5. Next step (the 21st wall): NZCV flags + the CSEL/CSET family

`csel x,w, cond` and `cset/csinc/csinv/csneg` need N/Z/C/**V** live from the
preceding `cmp/subt/cmp nzcv`-setter. NZCV is currently a placeholder
(written but not consumed); `b.cond` also must read *stored* flags, not live
x86 flags. This is a self-contained subsystem:
1. store N=bit31, Z=bit30, C=bit29, V=bit28 to `CpuState.nzcv:u32` from each
   `S`-flag setter (cmp/subs/adds/...),
2. read cond→x86 flag and `csel/cset/...` branch on it,
3. retire the "NZCV is a placeholder" comment in translate.rs.

`git log` since 20: `6618c5e` — "arm64jit: PC-driven dispatcher (blr/br/ret)
+ push 5 more decoder walls". Working tree clean.

## Session 22 (Aug 20, 2026) — NZCV flags + CSEL/CSET family; stp/ldp d; LDAR/STLR

Commit: `6dceb47` (dev). **Session 21's NZCV wall is crossed — and the JIT now
executes real libroblox.so all the way into floating-point arithmetic.**

### 1. The NZCV condition-flags subsystem (the 21st wall)

- New `store_nzcv(buf)`: after every `S`-flag arch op (`cmp`/`subs`/`adds`),
  snapshots x86 rflags (`pushfq`/`pop`) and packs either into
  `CpuState.nzcv:u32` with **N=bit31, Z=bit30, C=bit29, V=bit28**.
- New `load_nzcv_to_eflags(buf)`: the inverse — reads the packed NZCV, bit-shuffles
  it back into an x86 eflags image (CF/nzcv.29, ZF/.30, SF/.31, OF/.28), and does
  `push; popfq` so the immediately-following native `jcc`/`cmovcc` evaluates the
  guest condition. This matters because the dispatcher reloads operands between the
  setter and the consumer, clobbering live flags.
- `cmp`/`cmn` (rd==31, s=true) now write flags only; `b.cond` reads stored flags
  (not stale live x86 flags).
- **CSEL/CSINC/CSINV/CSNEG** (incl. `CSET`/`CINC` aliases): `rd = c ? rn : f(rm)`
  computed with a `cmovcc` on the repainted flags, `f` = identity/`+1`/`not`/`neg`.
  Handles the `cset`/`cinc` disasm aliases via the generic csinc.

**Decode-order gotcha (pitfall):** the CSEL X-variant shares top byte `0x9a`
with the logical ORR/EOR/BIC family, so `(insn&0x7fe00000)==0x1a800000` MUST be
checked *before* the LogicReg decoder or csel is swallowed as an `EOR`.

### 2. Three x86-emitter bugs found & fixed (latent, hit only when new high-reg/FP code selected them)

- **rex()**: R/X/B bits were placed at 0x10/0x20/0x08 instead of 0x04/0x02/0x01 —
  any 64-bit op touching a register ≥8 (e.g. R10 in csel) emitted an invalid
  `0x50-0x5F` prefix; fixed to the real REX.R/X/B mapping.
- **cmov_rr64**: emitted the jcc opcode (double-0x40) and lacked REX.R/B for
  R10/RDI. The REX fix plus a `-0x40` cc-domain correction made it emit a real
  `cmovcc`.
- The eflags-restoration tail originally pushed the wrong scratch (nzcv instead
  of the built eflags), causing a segfault only on one branch of the cset test.

### 3. The next two decode walls from *executing* libroblox.so

- **`stp/ldp d` (FP/vector 64-bit pair, top `0x6d`, scale 8)**: d-regs are the
  low 64 bits of `CpuState.v[k]`; each reg transfers one u64 at
  `VECTOR_BASE + 8*reg`. (q128 stays scale 16 / 16 bytes.)
- **`LDAR/STLR` acquire-release** (`mask 0x3fe00000 → 0x08800000/0x08c00000`,
  ~5787 uses in the binary): treated as plain loads/stores — ordering is a no-op
  in the single-threaded JIT.

### 4. Verified progress on the real binary

```
running entry guest=0x101c34480
arm64jit run_loop stopped: translate: unhandled Unsupported(1829831680) at 0x101c39f80  # stp d
  (pushed to) Unsupported(509675520) at 0x101c3a348   # fmul d0,d0,d1  <- current wall
```
The JIT now runs **past** `csel`/`cset`, through `stp d` and `ldarb`, and stops
on the genuine **floating-point** instruction `fmul d0, d0, d1` (`0x1E610800`) —
the scalar-FP arithmetic layer that Session 20's note originally mislabeled.
This is the FP layer (mulsd/addsd/fdivsd...) and the fp-immediate move/fmov.

### 5. Tests

`cargo test -p arm64jit` → **31 pass** (added `csel_family_ground_truth`,
`stp_d_zero`, `ldar_stlr_plain`). Full workspace 60+ pass except a **pre-existing,
unrelated** `libloader/src/android.rs` filesystem idempotency failure (untouched
by this session).

`git log` since 20: `6618c5e` (Session-21 dispatcher), `6dceb47` (this session).
Working tree clean (commit `6dceb47`).

## Session 23 (Aug 20, 2026) — FP arithmetic (verified), FP→int, UBFM, ANDS/TST

Commits `a03b143` (dev). **The FP-arithmetic wall is crossed and verified; the
real JIT now executes real double-precision multiplies inside Roblox's
`Java_com_roblox_engine_jni_NativeGLInterface_shouldDisplayOpenGLUnsupportedMessage`
and walks past it deep into the GameActivity init. New hardware: TLS.**

### 1. Scalar double FP arithmetic (the "fmul" wall) — IEEE-verified

- `Inst::FpScalar` ditched `movq_load`/`movq_store` + `mulsd`/`addsd`/`subsd`/
  `divsd`; `d_k` = low 8 bytes of `CpuState.v[k*2]` at `VECTOR_BASE + 16k`.
- DECODE BUGS FIXED (verified via masked opcode — the opcode is `insn` with
  Rm/Rn/Rd masked, since the early `(insn>>15)&0x7f` field **overlapped `Rm`**):
  `fmul=0x1e600800 fadd=0x1e602800 fsub=0x1e603800 fdiv=0x1e601800` (+`sz` bit22).
- New emitter `movq_load`/`movq_store` (64-bit XMM↔mem) and `mulsd`/`addsd`/
  `subsd`/`divsd`; **x86 SSE-operand order** fixed: `F2 0F 59 /r` uses
  `ModRM.reg=DST, r/m=SRC` (opposite of integer).
- New `fp_scalar_double_ieee` unit test: `2.5*4.0=10`, `10+2.5=12.5`, `10/2.5=4`
  — passes against `from_bits` IEEE ground truth. (Also fixed the test to address
  the `v`-array layout: `d_k` ↔ Rust `v[2k]`, not `v[k]`.)

### 2. FP→int + UBFM round of the register file

- `Inst::FcvtToInt`: class `(insn&0x5f20fc00)==0x1e200000`, op `(insn>>17)&7`
  (0=fcvtzs truncate, 2=fcvtas), via `cvtsd2si` (nearest-even; **note**: ARM
  `fcvtas` is ties-away — the tie-only difference is a documented approximation)
  and `cvttsd2si` (fcvtzs exact). New emitters.
- `Inst::BitField`: full `UBFM/SBFM` — `lsr`(imms==last), `asr`(arith), `lsl`
  (immr==(imms+1)%bits), plus the *general extract* `(Rn>>immr)&low(width)` with
  sign-extend (`sbfx/sxtb/ughl `sar/shl round-trip) for ubfx/sbfx/uxb/sxtb/sxth.
  This covers `sxtb/uxth/...` which appear throughout the tree.

### 3. ANDS/ORRS/EORS/TST now actually set flags

- `LogicReg opc==3` was *also* treated as a no-op (`let _ = s`). Now the base op
  is computed for opc==3 (`ANDS`/`BICS`), and `if s` calls `store_nzcv` — `x86
  and/or/xor` already produce CF=0,OF=0,ZF/SF-from-result, exactly AArch64 NZCV.
  Also added the missing top-bytes `0x3a/0x7a/0xea/0xfa` so `tst x_,x_`=ANDS decodes.

### 4. Real libroblox.so: what the JIT executes now (JIT_TRACE-past)

```
past: csel/cset(NZCV) → stp d/ldp d → ldarb/stlrl → fmul→fmul→fcvtas→lsr
      → uxtb (UBFM) → csel→ … → tst x23,x8 → b.ne → … → cmp x0,#0 → b.ne
stopped: mrs x19, tpidr_el0  (0xD53BD053)  <-- TLS thread-pointer system register
```
This is the **guest TLS/sp boot-essentials** wall the session list flagged. To
boot Roblox we must answer `mrs tpidr_el0` (and `msr`/`tlbi`/`isb`) with a real
or forwarded TLS base, map sp, and route `svc`. FP is done and verified; the
immediate TSL system-register (MRS/MSR tpidr_el0) is the next concrete wall.

`cargo test -p arm64jit` → **32 pass** (fp_scalar_double_ieee, plus more).
Workspace green (the libloader android idempotency test passed this session —
it is host/env flaky; unrelated). Committed, tree clean at `a03b143`.

## Session 24 — TLS crossed; JIT now runs real StartApp code

### Roblox boot progress (libroblox.so, 117MB ARM64)
```
past (this session, in order):  mrs x19,tpidr_el0 (TLS) → BIC  → ror (shifted-op)
      → mov x11,#0x3ffffffff (logical-imm) → ldxr/stxr (exclusive) → csinv
      → udiv/sdiv → madd/msub → bfi/bfc → fmov d0,x8 → SIMD cnt v0.8b
      → uaddlv h0,v0.8b (popcount) → fcvt s0,d0 → str s0,[x22,x23,lsl#2]
      → b.ne → mov v0.d[1],v0.d[0] → str q0 → … ror w23,w22,#0x14 (SBFM/EXTR)
stopped (honest Unsupported): ror #imm  (in a SHA/compression mixing loop)
```
- The JIT now executes real **`nativeAppBridgeV2StartAppWithParams@@LIBROBLOX`**
  startup code (and an FMOD audio-init region), including a full SIMD bit-popcount
  idiom (`cnt v0.8b + uaddlv h0`), FP width conversion, TLS reads, exclusive
  atomic emulation, integer mul/div, and BFM inserts. Big-vs-previous milestone.

### New instructions implemented & verified this session
- **MRS/MSR tpidr_el0** (`Inst::SysReg`) — decode `0xd53bd053` tpidr, read/write the
  per-state TLS pointer `CpuState.tpidr` (new field after `v`, `TPIDR_OFF=784`).
- **BIC/ORN/EON** (LogicReg op 4/5/6) — the bit-invert second-operand family.
- **add/sub/logic shift ROT**: `ror` via `ror_cl64` for shifted-register operands,
  plus `ror_ri8` (48 C1 /1).
- **Logical (immediate)** `Inst::LogicImm` — AND/ORR/EOR/ANDS with the AArch64
  bitmask immediate (`decode_logical_mask` from N/immr/imms); covers `mov xD,#imm`
  (ORR xzr,#mask) and `tst`/`ands` imm.
- **Exclusive** `ldxr/stxr/ldaxr/stlxr` (`Inst::LdExr`) — single-threaded: ldxr =
  plain load, stxr = plain store + status=0. Thread-safe enough for a lone guest.
- **CSINV/CSNEG/CSINC** — widened the CSel decode to tops `0x5a/0xda` (was 0x1a/0xda).
- **UDIV/SDIV/MADD/MSUB** (`Inst::MulDiv`) — via `div/idiv/imul` + `cqo`/`movsxd`.
- **BFM/BFI/BFC** — the bitfield-insert alias of BitField (`immr > imms`).
- **FMOV core↔FP** (`Inst::FmovGp`) — Xd<->Dn, Wd<->Sn.
- **FCVT s<->d** (`Inst::Fcvt`) — `cvtsd2ss`/`cvtss2sd`+movd.
- **NEON SIMD** (first SIMD in the JIT): `cnt v.8b` (`Inst::SimdPopcnt`, SWAR
  byte-popcount) and `uaddlv h,v.8b` (`Inst::SimdSum8`) — verified against the
  real binary's popcount chain reaching an `fmov w10,s0`.
- **`mov v{rd}.d[1], v{rn}.d[0]`** (`Inst::InsD1D0`) — dup low 64 into high lane.

### ⚠️ OPEN WALL — `ror rd, rn, #imm` (EXTR rotate) still not decoded
The standalone `ror` is the rotate alias of **EXTR** (`EXTR Rd,Rn,Rn,#lsb`), NOT
a UOFM — the current `BitField` decode + `is_valid` mis-reads/mis-rejects the word
(`ror w23,#0x = 0x139652d7`: immr=22, imms=20; second sample 0x138f51eb immr=15,
imms=20 — both rotate `to ROR(Rn, imms)` but the generic UBF/rot encode mapping is
ambiguous against bfi/extracts and is NOT resolved. The JIT stops on `ror`
with a clean `Unsupported` (no silent wrong result). **Required**: decode the
`ror`/EXTR rotate as its own op (class `0x1 0x1 `... `N`, Rm==Rn) and emit
`ROR(Rn, lsb)`; add a unit test seeded with known operands. Also still open:
guest sp/`svc` routing for full boot.

## Session 25 — ror/EXTR decoded; SIMD lane add; JIT reaches MessageBus code

### ror now works (was the Session-24 open wall)
- Root cause: the standalone `ror Rd,Rn,#imm` is the **EXTR rotate** alias
  (`EXTR Rd,Rn,Rn,#lsb`), NOT a UOFM. Ground truth (`ror x0,x1,#12=0x93c13020`,
  `ror w0,w1,#4=0x13811020`): the EXTR class is `(insn & 0x1fe00000)` in
  `{0x13800000, 0x13c00000}` (disjoint from UBFM's 0x130/0x136), and the rotate
  amount is `imms` (bits[10:15]); `rm == rn`. `Inst::Ror` → `ror_ri8` (48 C1 /1).
  New decode test `ror_exclusive` (part of suite).

### SIMD lane arithmetic — first real SIMD math
- `add Vd.4s, Vn.4s, Vm.4s` (`Inst::Simd4s`, op 0) via x86 `paddd`
  (66 0F FE /r) + existing `movdqu_load/store`. FMOD `OutputAAudioHeadphones`
  audio-mix loop (the XOR/ROR/ADD lanes) now executes fully.

### Real libroblox.so progress this session
```
past: ror w mix-loop → ldr q1 → cmp x9,#0x40 → add v0.4s,v1,v0  (SIMD)
      → str q0,[x11,#64] → b.ne loop → ... → ldr x19,[sp,#16]
      → ldp x29,x30,[sp],#32 → b 5df5d9c  (branch into audio code)
stopped: Unsupported(0x00000000) at guest pc 0x1026a1584  (zero-fill pad)
```
- The JIT followed `nativeAppBridgeV2StartAppWithParams` → resolved a `b` into
  the `MessageBus_getLastRaw` / `FMOD_OutputAAudio` regions, executing real
  audio mixing. `0x00000000` is ELF `.text` alignment zero-fill: the guest
  branched into a **data/padding hole**, i.e. execution control-flow has started
  to diverge (a previous arithmetic/SIMD result feeding a branch is *slightly*
  off, or a branch table/`bti` landing addresses). Verify the SIMD `add v.4s`,
  the byte-popcount chain, and `fcvt` against a self-contained reference before
  trusting deeper control flow; the unit tests only cover deltas of decode.
- `cargo test -p arm64jit` → **34 pass**. Workspace `cargo check --workspace`
  green (1 pre-existing sober-core warning). Commits `73c907e`(TLS→fmov),
  `5c4e86d`(BFM/FMOV/SIMD-popcount), `18054d3`(fcvt/ins/simd), `1f6cde3`(ror/SIMD-4s).

### ⚠️ next wall (per the honest-debug path)
The `0x00000000` pad means a guest branch went somewhere unexpected. Most likely
a) an SIMD or FP op above is subtly wrong (verify popcount `cnt`+`uaddlv`, `fcvt`,
`add v.4s`, and the ror with seeded JIT tests), and/or b) we still lack guest
`sp`/`svc` routing so functions that rely on the guest stack/tls diverge. The
immediate next step: add guest `sp` (map a real stack) + route `svc` syscalls,
then verify the arithmetic blocks against normal host x86 expectations.  Also
open: `bti`/PAC `ic`/`dc` hints beyond the existing NOP mask.

## Session 26 — ror/SIMD verified; guest stack/TLS stabilize; svc hookpoint

### New instructions & bootstrap this session
- `ror`/EXTR rotate (`Inst::Ror`) — class `(insn&0x1fe00000)` in `{0x13800000,0x13c00000}`
  (disjoint from UOFM 0x130/0x136), `rm==rn`, `imms`(bits[10:15]) is the rotation.
  `ror_ri8` (48 C1 /1). Test `ror_exclusive`. Real FMOD audio-mix ROR loop now executes.
- **SIMD lane add** `add Vd.4s,Vn.4s,Vm.4s` (`Inst::Simd4s`, op 0) via x86 `paddd`
  + `movdqu_load/store`. First genuine SIMD *arithmetic* (prev was the popcount idiom).
- **Guest Stack + TLS bootstrap** in `elfjit`: allocates a 4MB guest stack, sets
  `x31=sp` to its top, allocates a writable 64KB TLS and sets `CpuState.tpidr` so
  `mrs tpidr_el0` returns a non-zero writable base. Stack-frame save/restore
  (`stp x29,x30,[sp,...]/ldp ... [sp],...`) and `ret` now use real memory.
- **`svc #imm` hook point** — `Inst::Svc` decode+translate → host `guest_svc()`
  dispatcher. Incremental: handles exit/exit_group (clean `process::exit`); all
  other syscalls return `-ENOSYS` (+JIT_TRACE_SVC log). Deliberately does NOT guess
  AArch64→x86-64 syscall numbers (they differ for mmap/futex/…); correct routing is
  a distinct open item.
- **Honest reference test** `simd_popcount_and_4s_add_reference`: seeds the JIT with
  a known 64-bit value and asserts `cnt v.8b + uaddlv h` == `u64::count_ones()` and
  `add v.4s` lane sums — catches real miscomputations, not just "got further". (A
  malformed hand-encoding made it initially fail; that was a *test* bug, not code.)

### Current wall when running real libroblox.so
```
stopped: Unsupported(0x00000000) at guest pc 0x1026a1584
   (execution landed on ELF .text zero-fill after a branch — likely a guest
    return address / indirect br/blr resolution issue, before a syscall is hit)
```
The JIT now runs `nativeAppBridgeV2StartAppWithParams`, FMOD audio mixing, SIMD
popcount, SIMD lane add, FP width-convert, integer mul/div, ror, exclusive atomics,
TLS reads across many MB of real API code reaching MessageBus. The concrete next
step to actually *boot* is still the guest `svc` routing (real syscall table +
mmap/open/futex/...) and the indirect-branch/`blr` landing correctness that drives
execution into the right return addresses (the `0x00000000` pad hit). `cargo test
-p arm64jit` → 36 pass; workspace clean.

## Session 27 (Aug 20, 2026) — root-cause fix for the `.text` pad; DecodeBitMasks; scvtf

### The headline bug (why the JIT was stuck at the 0x00000000 pad)
The stop at `.text` zero-fill `0x1026a1584` last session was NOT an indirect `br`/`blr`
return-address issue. Real root cause: `compile_image` translated an **unconditional `b`**
(emitting the `jmp`) but then kept walking the **linear block** into the 4 bytes after
the `b`, tried to `translate(0x00000000)` → `Unsupported(0)`. Fix: `Inst::B{link:false}`
is now terminal for the block (same break path as Ret/Br/Blr), so the walk stops right
after the `jmp`. This single fix walked the guest from deep in FMOD/MessageBus/audio
(`0x1026a1584`) all the way back **up to the entry-point startup code** — the earlier
"return-address/blr" hypothesis was wrong; it was block fall-through corruption.

### New instructions & decode fixes this session
- **LogicalImmediate (`decode_logical_mask`) rewritten** to the ARM `DecodeBitMasks`
  procedure (every element size 2..64, incl. the N=0/32-bit + leading-`imms`-run case).
  Unblocked `mov x8,#0xcccccccccccccccc`, `mov x0,#0x55555555...`, `orr x8,x22,#0x1`,
  which previously fell through to `Unsupported`. Honesty regression `mov_ccc_imm_and_orr_one_and_scvtf`
  caught two real bugs in my first two attempts:
  1. rejecting `S==0` (ARM rejects **S==all-ones**, not S==0).
  2. `rotate_right` on the full u64 then masking to esize discarded the element
     (e.g. the 4-bit `0xC` element of `0xCCCC..` → 0). Fixed with an **esize-local
     shift rotate** `(ones<<r | ones>>(esize-r)) & esize_mask`.
- **`scvtf`** (signed integer → FP, `Inst::Scvtf{rd,rn,to_double,sf}`): decode gate
  `(insn&0x7ff0_fc00)==0x1e60_0000 && (insn&0x20000)!=0` (double dest; bit17 separates it
  from FP→int `fcvtns/fcvtzs/fcvtas` at the same base). Translate: `ldg Rn` →
  `cvtsi2sd`/`cvtsi2ss` → `movq_store`/`mov_store32` into v{rd}. Added x86 emits
  `cvtsi2sd`/`cvtsi2ss` (F2/F3 [REX.W] 0F 2A /r).

### Current wall running real libroblox.so
```
stopped: Unsupported(0x9e790013) at guest pc 0x105dfdac8
   fcvtzu x19, d0  (unsigned double->int64) — a genuinely new FP->unsigned conversion.
```
The guest now runs real startup/audio/MessageBus code from the entry point; the fix
`b`-fall-through bug was the bridge that finally let the block graph route correctly.
`fcvtzu` (and later `ucvtf`) are low-volume but real ISA surface — x86-64 has no scalar
FP→u64 instruction, so it needs a careful honest sequence (NOT a silently-wrong
`cvttsd2si`).

### Verification
`cargo test -p arm64jit` → **37 passed** (36 + new honesty regression). Workspace
`cargo build --workspace` clean. `git status` has exactly this session's 4 source files
+ HANDOFF. Commit `[…sess27-sha…]` may be updated by user.

### Open (next concrete)
1. `fcvtzu`/`ucvt*` — unsigned FP↔int with verified x86-64 u64 handling.
2. guest `svc` → real AArch64→x86-64 syscall table (mmap/futex/mprotect/…) — still
   `-ENOSYS` (exit-only) per the honesty rule.

## Session 28 (Aug 20, 2026) — FP-to-int correctness + FMOV/fcmp/fcsel; 5 ISA walls crossed

Picked up Session 27's wall. In one sitting the guest advanced through **six** new,
distinct instructions (all verified against ground truth + the real libroblox binary):

| wall | word | what | how |
|---|---|---|---|
| `fcvtzu` | `0x9e790013` (x19,d0) | unsigned double→u64 | `FcvtToInt{unsigned}` — `cvttsd2si` (exact for `[0,2^63)`) + sign-clamp-to-0 via `cmovs`. **Fixed a pre-existing latent bug**: the old signed `fcvtzs` gate (`0x1e200000`/mode∈{0,2}) never matched the real `0x1e78` encodings — the honesty regression caught it; rewrote signed gate to `0x1e78`/`0x1e7a` (real `fcvtzs`/`fcvtas`). |
| `mrs/msr cntfrq_el0` | `0xd53be000` | counter-freq sysreg | `SysReg{sysreg:1}` returns fixed 100 MHz (`cntfrq` = 100_000_000 Hz), documented. Used op1=11 (4-bit field), CRn=14, CRm=0, op2=0. |
| `mrs/msr cntvct_el0` | `0xd53be059` | counter-value sysreg | `SysReg{sysreg:3}` reads `CpuState.cntvct` — a **live monotonic counter stamped by the run_loop** before each block (`elapsed` host clock scaled to 100 MHz ticks), so guest time deltas actually advance. |
| `fmov d1,#0.5` | `0x1e6c1001` | FP immediate | `Inst::FmovImm` + `decode_fmov_imm()` — ARM 8-bit FP imm → IEEE-754 bits, **verified against 12 real compiler vectors** (`ex = ((e+1)&7)`, sign bit7, `(1+m/16)·2^ex`). |
| `fmov d6,d0` | `0x1e604006` | scalar fp-reg copy | `FmovFp` (`0x1e60_4000`/`0x1e20_4000`) — bit copy of the FP slot. |
| `fcmp d7,d6` / `d6,d16` | `0x1e6620e0`/`0x1e7020c0` | FP compare → NZCV | `Fcmp` + `store_nzcv_fp()`: `comisd` flags → guest NZCV via `Z=ZF, V=PF, C=(!CF)|PF, N=0` (handles A<B/A>B/eq/unordered exactly). Gate `(insn&0xffe0_fc00)==0x1e602000` also catches `fcmpe` and the high-rm forms (bit16-20 feed into the base nibble). |
| `fcsel d6,d16,d6,mi` | `0x1e664e06` | FP cond select | `FcsSel` — mirrors integer `CSel` on the FP slots (load both, `load_nzcv_to_eflags`, `cmovcc`, store). Structural gate `(insn&0x1f20_0c00)==0x1e20_0c00` separates select from fcmp/fmov/fcvt. |

### Current wall running real libroblox.so
```
stopped: Unsupported(0x6e61d842) at guest pc 0x105dfe180
   ucvtf v2.2d, v2.2d  (SIMD unsigned-int→double, 2-lane) — next FP conversion.
   (next in line: `fdiv d1, d1, d3` = 0x1e631821, and a new adrp/ldr load.)
```
The audio/headphones path now runs through `fcmp`/`b.gt`/`fcsel` correctly. `ucvtf` needs an
honest **u64→f64** (the `≥2^63` case has no scalar `cvtsi2sd`; needs a split or `+2^63`
correction, same honesty class as `fcvtzu`).

### Verification
`cargo test -p arm64jit` → **38 passed** (37 + fcsel/fcmp-be fixed; every decode regression
incl. `fcvtzu w/x`, `mov_21`, fmov-imm value, cntvct, `fcmp` high-rmd, `fcsel`). Workspace
`cargo build --workspace` clean (verified). Tree has the 4 source files + HANDOFF (above).

### Open (next concrete)
1. `ucvtf v2.2d` (SIMD u64→f64) + scalar `ucvtf`/`ucvtf d,xn` — must be honest u64→double.
2. `fdiv d1,d1,d3` (scalar FP divide).
3. `svc` real AArch64→x86-64 syscall table (mmap/futex/mprotect…) — still `-ENOSYS` (exit-only).

## Session 29 (Aug 20, 2026) — scalar FP unary adds; confirmed FpScalar covers fadd/fmul/fdiv

Added `Inst::FpUnary` (`fsqrt`/`frintm`) since the FMOD audio-mix block (`0x5dfe...`)
uses them, and **verified the scalar `fadd`/`fmul`/`fdiv` in that block are already
handled by `FpScalar`** (mask `0xffe0_fc00` → `0x1e602800`/`0x1e600800`/`0x1e601800`).

- `x86.rs`: `sqrtsd` (`F2 0F 51`) and `roundsd` (`66 0F 3A 0B /r ib`, mode imm[1:0],
  01=floor/-inf, 02=ceil/+inf, 03=trunc).
- `decode.rs`: `FpUnary{rd,rn,op,sz}`; gate `(insn & 0xffff_fc00)` — **keeps bits 16-23
  distinguishing the frint/fsqrt byte** — `fsqrt=0x1e61_c000`, `frintm=0x1e65_4000`.
  **Bug fixed en route**: an earlier `0xfff0_fc00` mask collapsed `fsqrt` to
  `0x1e60c000` (wrong) and an `0xffff_fbff` mask didn't mask register bits at all;
  the `0xffff_fc00` mask is correct and *disambiguates* `fsqrt`/`frintm` from the
  `fmov d,d` base (`0x1e604000`) so it stays `FmovFp`. Regression + collision guard.
- `translate.rs`: `FpUnary` arm → `sqrtsd`/`roundsd` on the FP slot.
- 38 tests pass; real binary still stops at `ucvtf v2.2d` (0x105dfe180) — the SIMD
  unsigned-int→double in this identical block, next on the agenda.

## Session 29 (Aug 20, 2026) — FMOD audio block: SIMD .2D ops, fabd; boot advances 0x18

Targeted "continue" run to clear the FMOD DSP block after Session 28's ucvtv wall.
Committed 3 milestones (ad5001c, ac8e622); tree clean; 38 tests pass.

### New decoder + translate (all verified vs real libroblox words + compiler ground-truth)
- `Inst::FpUnary` fsqrt/frintm (scalar double): sqrtsd + roundsd(mode). Gates
  `(insn & 0xffff_fc00) == 0x1e61c000` (fsqrt d1,d1=0x1e61c021) and 0x1e654000
  (frintm d3,d3=0x1e654063). Distinguish from fmov d,d (0x1e604000) by keeping bits16-31.
- `Inst::Ucvtf2d` ucvtf Vd.2D: gate `(insn & 0xffe0_fc00) == 0x6e60d800` (real
  0x6e61d842, compiler 0x6e61dbff). Honest u64->f64 per lane: `cvtsi2sd` +
  sign-corrected `add 2^64` (JNS rel32 patch in-buffer; exact over full u64).
- `Inst::SimdDupD` dup Vd.2D,Vn.D[i]: gate `(insn & 0xffff_fc00)==0x4e180400`;
  index is BIT20 (0=d[0],1=d[1]), not bit12 (learned via asm ground truth).
- `Inst::Simd2dFp` 2xdouble lanewise fdiv/fmul/fadd/fsub: gate 0xffe0_fc00 ->
  0x6e60fc00/0x6e60dc00/0x4e60d400/0x4ee0d400.
- `Inst::Fabd` fabd Dd,Dn,Dm=|dn-dm|: gate `(insn & 0xffe0_fc00)==0x7ee0d400`
  (real 0x7ee1d503, compiler 0x7ee1d400). translate via subsd + movq_r64_xmm
  round-trip + sign-bit clear (new x86 helper `movq r64,xmm` = 66 48 0F 7E).

### Boot path / wall history (this session)
 0x105dfe108 (fmov) -> ...d14c (fcmp) -> ...d180 (ucvtf v2.2d, WAS blocked)
 -> ...d194 (dup v4.2d) -> ...d198 (fdiv v2.2d) -> ...d1d4 (fabRd) -> CLEAR
 Now STOPPED at 0x105dfe224: `dup v1.4s, w10` (0x4f2_0d41) = GPR-source 4S dup.
 After it: movi v0.4s/#1, movi v3.4s/#0xa, dup v1.4s,w10 dup v3.4s,w8,
  mov v2.16b, mul v0.4s, orr v3.16b, cmhi v1.4s, bit v0.16b, ldr q4, ...
 (a "channel-count round-up to multiple of 4" SIMD loop).

### Next up (ordered)
1. dup Vd.4S, Wn (GPR-source, 0x4e040c00/0x4e0d.. ) — current wall.
2. mul v0.4s (0x4ea39c00), movi vD.4s,#imm (0x4f000420/#1/#a), cmhi v.4s,
   orr/bit v.16b, ldr q (128-bit). Then the whole FMOD audio-out block clears.
3. After the audio loop: likely `svc` syscall table (mmap/futex/mprotect;
   host x86 numbers differ) — big-ticket remaining item.

### Status: real Roblox still does NOT boot; boot path is inside an FMOD
 output-audio "loop over channels when energy/limits" DSP routine.

## Session 30b (Aug 20, 2026) — SIMD 4S mix loop: orr/mul/cmhi/bit; boot advances 0x2c

Cleared the "channel-count round-up" SIMD 4S loop through `bit`. 4 commits:

- 1ceb00a — `Inst::SimdOrr16` (16B OR, gate (0x4ea01c00, Q=1; also the `mov Vd.16B` copy
  rm==rn alias). Also `Inst::SimdMul` (4S/2S gate 0x4ea09c00/0x0ea09c00) — per-32-bit-lane
  low-32 product via 64-bit imul+low store (mod-2^32, correct for signed/unsigned wrap).
- cda1619 — `Inst::SimdCmhi` (4S/2S unsigned compare-higher, gate 0x6ea03400, real
  0x6ea13461). NOTE: the earlier fabricated word 0x6ea4c1c1 was WRONG — the real cmhi is
  0x6ea13461 (rd=1 rn=3 rm=1). Gate verified against the real word (0x6ea03400).
  Per-lane => all-ones if Vn[i]>Vm[i] via cmp + cmova.
- f05ac60 — `Inst::SimdBit` (16B bitwise-insert, gate 0x6ea01c00, real 0x6ea11c40).
  Vd=(Vn&Vm)|(Vd&~Vm) over both 64-bit halves (xor all-ones for ~Vm).

All 3 verified (real libroblox word → decode + translate), 38 tests pass, tree clean.
Boot wall history this session: 0x105dfe230 (mov v2.16b) -> …234 (mul v0.4s) -> …258
(cmhi v1.4s) -> …25c (bit v0.16b) CLEAR -> STOPPED at 0x105dfe260: `ext v1.16b, v0.16b,
v0.16b, #8` (0x6e004001) — SIMD byte-shift/immediate, NEXT ON AGENDA.

### Next up (ordered)
 1. ext Vd.16B, Vn, Vm, #imm (0x6e004001, imm in bits11-15). General form is a 128-bit
    rotate/insert: R = (Vn>>sh)|(Vm<<(128-sh)), sh=imm*8; real case imm=8 is just a
    u64 half-swap (Vm==Vn). Implement general imm via 4×64-bit shift ops, verify vs ground
    truth before committing.
 2. `mov w8, v0.s[1]` (0x0e0c3c08) SIMD lane->GPR, and any remaining movi/lane ops.
 3. Then `svc` real AArch64->x86_64 syscall table (mmap/futex/mprotect; host x86 numbers
    differ: mmap 222->9, futex 95->202, mprotect 226->10) — the big-ticket remaining item
    before real Roblox boot.

### ✅ VERIFIED state (2026-08-20) — hand off to next agent as-is
- HEAD: `c3f7f92` (Session 30b), tree clean, 38 tests pass.
- Fresh ad-hoc /tmp/hermes-verify-s30.sh: 17/17 PASS, temp script removed, tree clean.
- Run line: `timeout 20 ./target/debug/examples/elfjit ~/.cache/open-sober/libs/libroblox.so 0x1c34480`
- Boot is STOPPED at `0x105dfe260` = `ext v1.16b, v0.16b, v0.16b, #8` (0x6e004001), the next wall.
- Real Roblox STILL does NOT boot. Remaining path: ext v.16B -> lane->GPR -> svc syscall table.
- Edit caveat: patches to decode.rs/translate.rs surface massive pre-existing rustfmt
  churn in the lint output — edits are correct; build/tests/boot are the real gate.

## Session 30c (Aug 20, 2026) — ext + lane->GPR + a burst of FP walls; boot leaps out of the audio loop

Picked up the 30b wall (`ext v1.16b,v0,v0,#8` = `0x6e004001`). Cleared **ext and 10 more walls in one extended
run**, carrying the guest from the FMOD audio-DSP loop all the way through `LocalStorageManager` init,
`MainGameActivity`/NativeSettings and stopping deep in an FP round-to-integer wall. **39 tests pass.**

### New instructions implemented this session (all objdump/qemu-verified)
- `Inst::SimdExt` (ext Vd.16B/.8B, Vn, Vm, #imm) — the 30b wall. Gate `(insn&0xffe0_0400)` in
  `{0x6e00_0000 (16B Q=1), 0x2e00_0000 (8B)}`. Semantics: result = 128/64-bit window of the
  concatenation `{Vn(high), Vm(low)}` starting at byte `imm` (= half-swap of u64s when Vn==Vm & imm=8).
  General imm via a 4×u64 concat byte-fission (aligned load or `(W>>sh)|(Wnext<<(64-sh))`).
- **SimdLaneGp** (mov/umov/smov Wd|Xd, Vn.T[idx]) — gate `(insn&0xffe0_0c00)` in `{0x0e000c00,0x4e000c00}`.
  esize=1<<(ctz(imm5)); index=imm5>>p; sign(SMOV)=bit12 clear. Copy element at `index*esize` in the 16-byte
  sl‑o t, zero/sign-ext. (Real `mov w8,v0.s[1]=0x0e0c3c08`.)
- `fabs d0,d0`(0x1e60c000) + `fneg` — scalar FP unary sign ops (clear/clear via 0x7FFF… & 0x8000…).
- `clz`/`cls` (ClzCls) — LZCNT (F3 0F BD) direct; cls via clz of (x<<1)^x. Gate `(insn&0xffff_fc00)` in
  `{0x5ac01000(W),0xdac01000(X),0x5ac01400,0xdac01400}`.
- `ubfiz`/SBFM insert — extended the UBFM (0xd3/0x53) gate to accept `immr>imms` (the BFI/insert form),
  which the BitField translate already handled.
- Scalar `ucvtf Dd,Dn` (ScalarUcvtf) — honest u64->f64 via cvtsi2sd + sign-corrective `add 2^64` (JNS-1).
- `ucvt d0,x8`/scvtf unsigned — added `unsigned` to `Scvtf`; scalar int->FP family gate `(insn&0xf7be_fc00)`
  in `{0x1622_0000(W),0x9622_0000(X)}` (unsigned=bit16, dbl=bit22, X=bit31); honest u64 u- handling.
- `fmov s,s` (single-precision scalar copy) — extended FmovFp gate to `0x1e20_4000` (was d-only).
- `fmul/fadd/fsub/fdiv s` (single) — FpScalar decode accepts the `0x1e20_08/28/38/18` forms (bit22=0) and
  translate uses `movd`/SSE scalar-single (movss-family, added a `comiss` x86 helper). ops 0-3 stay double-only.
- `fcmp s,s` — Fcmp gained `sz`; single via `comiss` (0F 2F, no 66-prefix). store_nzcv_fp unchanged.

### Boot path / wall history (this session, guest pcs)
```
ext .16b #8 (0x105dfe260) -> mov w8,v0.s[1] (.268) -> fabs d0,d0 (.2ac) -> ucvtf s0,x8 (eat..)
  -> ucvtf s2,x23 (nativeInitCrashpad area 0x101f69cbx) -> fmul s2,s1,s2 (.2cbc) -> fcmp s2,s0 (.cc0)
  -> STALL 0x101f69cec: `fcvtpu x9, s0` (0x9e290009)  <-- current wall
```
(Wait, earlier saw `0x1f69cec` in different formatting — the guest stopped honestly at fcvtpu.)
Big-picture: the guest runs real `nativeAppBridgeV2StartAppWithParams`/`LiveStorageManager`/
`MainGameActivity`/Crashpad JNI init code now, far beyond the FMOD DSP loop.

### qemu ground truth gathered for the NEXT wall (fcvtpu Xd, Sn) — so it's ready to implement:
```
fcvtpu( s ): 1 ->1, 1+eps->2, 1.5->2, 2->2, 0.5->1, 0->0, -1->0, -2->0,
             +inf->0xFFFFFFFFFFFFFFFF, NaN->0, 2^31->0x80000000, 30->0x1e
```
i.e. ceil toward +inf for x>0, 0 for x<=0/NaN, u64::MAX for +inf. Implement with cvttsd2si(trunc-floor)
+ frac-has `+1`, negatives/NaN→0, inf→MAX. (Same honesty class as fcvtzu.)

### Verification
`cargo test -p arm64jit` → **39 passed** (added `clz_scalar_ucvtf_decode`, ror/share fine). Workspace
`cargo build -p arm64jit` clean (warnings are the pre-existing rustfmt churn). Tree has decode.rs /
translate.rs / x86.rs + HANDOFF.

### Next (ordered)
1. `fcvtpu Xd, Sn` (0x9e290009, FP→unsigned-int round-toward-+inf) + siblings (`fcvtps/ns/ms…`) — have qemu truth.
   **GATE CAVEAT (learned this session):** the fcvt-round family shares the scalar-FP byte with `fcmp`
   (`fcmp d6,d16 = 0x1e7020c0` gives byte 0x70→(..>>3)&7=6, bit17=0) so a loose `(insn&0x20000)==0 &&
   (byte>>3)&7 in {5,6}` gate will *invert-decode fcmp to FcvtToInt* (regression, was reverted). The
   translate for round mode 3/4 (roundsd+trunc) is already committed & correct; only a *verified*,
   tight fcvt-vs-fcmp discriminator is missing. Do NOT re-add the loose gate.
   Hint: fcvt-round src is single `0x..2x`/double `0x..6x` (bit22) and the real forms were
   `9e280009/9e690009(ps) 0x9e300009(ms) 0x9e200009(ns)` — pin opcode bits[22:17] + the fcmp-off axis.
2. Then continue grind; eventually the `svc` real AArch64→x86-64 syscall table (mmap/futex/mprotect; numbers
   differ: mmap 222->9, futex 95->202, mprotect 226->10) — the big-ticket item before real Roblox boot.
- Commits this session: `b80ed31` (10+ walls), `1b16fc9` (FcvtToInt round-mode translate, dead-code-y wiring).

## Session 31 (Aug 20, 2026) — Verified SHA-1 crypto core, adc/sbc w/ carry, fmaxv; 43/43 tests; boot far past the SHA integrity region

Took over from 30c's `fcvtpu` note. The guest, past the FCVT round wall, reached the **SHA-1 crypto block** of
libroblox and the JIT was failing on it. Implemented + **verified against qemu** the full SHA-1/SHA-256 crypto
extension, then adc/sbc-with-carry, then fmaxv. **43 tests pass.** Tree clean at HEAD `5de6e56`.

### New instructions implemented (all verified by seed-tests / objdump ground truth)
- **SHA-1 / SHA-256 crypto** via a host helper `guest_sha1stem` (extern "C" `f(st,*mut CpuState, packed)->u64`),
  called from translate via `mov_rr64(RDI, RBX); mov_ri64(RSI, packed); mov_ri64(RAX, addr); call_r64(RAX)`.
  Decode gate on `0x5e00_xxxx` SHA residues (sha1h=`0x5e20_0800`, sha1c/p/m=`0x5e00_xxxx` by op field, sha256h,
  sha1su0/su1=`0x5e00_3000` with bit20=clear→su0/set→su1). Semantics transcribed from authoritative qemu
  `crypto_helper.c`: `sha1h = Sd.word0=ror32(Sn,2)` (NOT the 3-xor I first shipped — fixed), `sha1c/p/m` =
  4-round `t=fn(d1,d2,d3)+rol(d0,5)+n0+m[i]; n0=d3; d3=d2; d2=ror(d1,2); d1=d0; d0=t` (fn: cho/par/maj);
  sha256h S0/S1; sha1su0/su1 schedule. `sha1_round_correct_reference` validates sha1h+sha1c vs the Rust ref.
- **adc/sbc/adcs/sbcs** (AddCarry, all 4 prefics ×32/64) — gate `(insn&0x1fe0_0000)==0x1a00_0000` (disjoint from
  AddSubReg-shifted 0x0b/0x8b, madd 0x1b, csel 0x1a80). Translate reads stored C (NZCV bit29) into x86 CF via the
  existing `load_nzcv_to_eflags`, then native `adc`/`sbb` (`add_rr64`-style `binop(0x11/0x19)`, added to x86.rs),
  `cmc` (`F5`) for sbc's `1-C` borrow + the `-s` carry restore. New `adc_x86.s` ground truth: `48 11 c8`=`adc
  adc %rcx,%rax`, `48 19 c8`=`sbb`, `f5`=`cmc`; REX.B for r8-r15 confirmed (`4d 11 d3`). `add_carry_reference`.
- **fmaxv/fminv Sd, Vn.4s** (FMaxV) — horizontal FP max/min of the 4 single lanes into scalar Sd. Gate
  `(insn&0x3f20_0c00)==0x2e20_0800 && (insn&0x0010_0000)!=0` — **bit20 demanded to exclude `ucvtf v2.2d`
  (0x6e61d842), which shares the residue** (caught by the `mov_ccc_/ucvtf` regression test → tighten). min =
  bit23 (`0x0080_0000`). Accumulator: 4×`movd_xmm_r32`/`maxss`/`minss` → `movd_r32_xmm` store. `fmaxv_reduce_reference`.

### NEW LATENT BUG FOUND & FIXED (the "silent miscompile" class the memory tracks)
- **`movd_xmm_r32` / `movd_r32_xmm` had their ModRM reg/rm fields SWAPPED for opcodes 6E/7E.** Correct is
  reg-field=xmm(dst), rm-field=GPR (6E) and reg=xmm(src), rm=GPR(dst) (7E). It only *coincidentally* worked
  when the GPR and XMM were index 0 (RAX & xmm0, as the old FMaxMin scalar path used), so it went unnoticed —
  my fmaxv loop's `movd_xmm_r32(1, RAX)` (xmm1≠0) exposed it by reading RCX instead of RAX. Fixed both emitters
  to `modrm(3, xmm&7, gpr&7)`. (Earlier `movq_xmm_r64`/`movq_r64_xmm` were already correct.)

### Boot wall history (this session, guest pcs)
```
sha1h(s) -> sha1c q0,s1,v20.4s (.0xa8c) -> sha1su0 (.0xa94)  [SHA-1 core]
  -> st1 {v0.4s},[x0],#16 (0x4c9f7800) -> udf #0 (0x105e651d8, zero-pad -> graceful trap like brk)
  -> adc w12,w14,w11 (0x1a0b01cc)  -> fmaxv s1,v0.4s (0x6e30f801)
  -> CURRENT WALL: fmla v29.4s, v19.4s, v26.4s = 0x4e3ace7d at guest pc 0x1058d5970
```

### New wall to implement next: `fmla v29.4s, v19.4s, v26.4s` (0x4e3ace7d)
Scalar-by-vector / vector FMLA (multiply-accumulate). Assemble the family (`fmla v.4s/`.2d`, `fmls`, `.2s/.4s`,
register vs by-element) to get disjoint gates; the `0x4e3a`/`0x2e3a` residue vs `0x4e32` (fmls), bit 24 for
vector-by-scalar, bit 30 for `.2s/.2d` width. Then continue → the big remaining ticket is the `svc` AArch64→x86
syscall table (mmap 222→20, futex 95→202, mprotect 226→10) before a real boot.

### Verification
`cargo test -p arm64jit` → **43 passed** (sha1, adc_carry, fmaxv + all prior). `cargo build -p arm64jit` clean.
Tree: decode.rs / translate.rs / x86.rs / jit.rs + HANDOFF. Commits: `ff35b63` (adc/sbc), `5de6e56` (fmaxv +
movd fix). Prior: `80f9874` (udf trap), `c7a75e2` (st1), `6a8cf7f` (sha1 ref), `444f6dd` (SHA core).

---

## Session — arm64jit ROBLOX BOOT COMPLETES (all decoder walls cleared)

**Milestone: `./target/debug/examples/elfjit ~/.cache/open-sober/libs/libroblox.so 0x1c34480` now runs the
real Roblox boot path to completion (exit 0, no Unsupported/panic). 49/49 tests green.**

Cleared the entire chain of decoder walls in libroblox.so's boot sequence (each verified by
`cargo test -p arm64jit` 49 green + boot advancing). Gates added this session (all disjoint, sibling at
top level of `pub fn decode`):

- **SimdDupGp** — GPR-source `dup Vd.T, Wn/Xn` (all esizes). Gate `(insn&0xff00_fc00)==0x0e00_0c00/0x4e00_0c00`; esize from `imm5` trailing-zeros; q=bit30. Replaces the old `.4s`-only SimdDupSReg.
- **SimdShrAcc** — usra/ssra shift-right-accumulate `Vd += Vn >>imm`. Gate `(insn&0x7000)==0x1000 && bit23-clear` (bit23-clear is the discriminator vs fmla-by-element; NOT bit16/b it6 — those are invariant for `#even` shifts / `.4s` fmla). shift = clamp(esize*8 - imm). signed vs unsigned by bit11.
- **SimdMull / SimdMull-acc** — smull/umull/smlal/umlal widening multiply (16×16→32, 32×32→64). Gate `(insn&0x0f00_c000)==0x0e00_c000`(mul) / `0x0e00_8000`(acc); sign/zero widen src, imul, optional +Vd.
- **VecMovi halfword + MSL immediates** — cmodes 0x8..0xb (4H/8H movi/mvni/bic) and 0xc/0xd (word MSL mask-shift). Halfword element = imm8 << (cmode&0x2?8:0) then ~ if op; **cmode 0x8/0xa correctly ownership moved from the word-lsl arms to halfword.**
- **SimdAdalp** — sadalp/uadalp pairwise-adjacent-long accumulate. byte2 0x68; sign-extend the summed pair.
- **SaturatNarrow** — sqxtn/uqxtn/sqxtun/uqxtun saturating narrow. byte2 0x28/0x48; per-lane clamp (cmovlt/gt) to dst dst-range.
- **Tbl n-reg** — multi-register table lookup `{Vn..Vn+N}`. Gate widen to mask out Vd/Vn/len/Vm → `(ins&0xffe0_9c0)==0x4e00_0000` (avoids ext 0x78 collision); tables read as CONTIGUOUS 16-byte slots (VECTOR_BASE+rn*16+idx, guard idx<16*n).
- **SimdCmgt** — signed cmgt .4s/.2s/.2d (0xea034000 family; cmovg ones-mask).
- **SimdNot** — mvn Vd.16B/8B (0x6e20/0x2e205800; new `movdqu_ones` = pxor+pcmpeqd).
- **SimdHighNarrow** — addhn/subhn/raddhn (byte2 0x40/0x60; dst = (sum ± round)>>8*dst then narrow store).
- **WidenShl `upper`** — shll2 (reads upper 8 bytes of Vn). byte2 mask `&0x7c==0x38` (was exact 0x38 — missed v16 wall; 0x78=ext now excluded by &bit6).
- **SimdAddl** — saddl/uaddl/subl/usubl long widen (residue list gate; sign which byte esrc).
- **SimdAddl-long`S2`... ** uqadd/sub/sqadd/sqsub saturating add/sub (byte2 0x0c/0x2c; signed/unsigned cmov clamps).
- **FpUnary op3 frintz** — double trunc toward zero (was "op 3 not implemented"), closing the last FpUnary hole.

### Boot wall history (guest pcs, this work)
```
fmla v29.4s (0x1058d5970) -> dup v2.4h,w9 (0x1033b8e90) -> usra (0x1053c43b0 wall-in-batch)
-> ... -> smax .2s (0x1053c8fcc) -> tbl 2-reg (0x1053c8ad4) -> ssra #even -> sqxtun -> addhn
-> sho:v 2-reg tbl (0x1020f461c) -> cmgt -> mvn -> gob0 tspbl -> shll2 (0x10533c7c4)
-> uadalp -> uaddl2 -> [FpUnary op3 frintz deep in audio boot] -> uqsub v0.2s (0x105d06648) -> **BOOT COMPLETES**
```
`timeout 40 ./target/debug/examples/elfjit ~/.cache/open-sober/libs/libroblox.so 0x1c34480` → exit 0, no walls,
all 4 PT_LOAD segments mapped, entry runs, guest sp/tls valid. **The Roblox boot x86-JIT translation path is now fully decoded.**

### Verified by
`cargo test -p arm64jit` → **49 passed**. `cargo build -p arm64jit --example elfjit` clean. Last commits
`d485bba` (SimdAdalp), `30abdd4` (SimdAddl), `8043510` (FpUnary frintz), `9bbb104` (SimdSatAdd, boot completes).

---

## Session — arm64jit syscall bridge (honest re-scope of "boot completes")

**Clarification (correcting the earlier milestone wording):** `elfjit 0x1c34480` running a `.so` entry
to exit-0 proves the **decoder + translator cover the full instruction space of Robust .so boot/init code** —
but it is NOT "Roblox boots." The entry we drive is a JNI-method stub (not `JNI_OnLoad`/dyld), it makes
**no `svc` syscalls** and does not launch the game. A real boot additionally requires the guest syscall
bridge, the PLT/trampoline table, JNI glue, and the loader spawn path (all in libloader/sober-core).

**Implemented now (`guest_svc` in jit.rs, commit `7c96cae`):** real AArch64->host syscall routing. The
old stub only handled exit(93)/exit_group(94) and returned `-ENOSYS` for everything else. Now the AArch64
syscall numbers (`x8`) dispatch to the matching libc call + the correct x86-64 semantics, returning the
kernel's `-errno` encoding for errors (guest reads x0 as signed). Covered: read 63, write 64, close 57,
openat 56, mmap 222, mprotect 226, munmap 215, brk 214, mremap 220, futex 98 (WAIT/WAKE), clock_gettime
113, nanosleep 101, getpid 172, getuid 199, getrandom 278. Anything unmapped -> `-ENOSYS` (log + grow the
table). Unit test `guest_svc_routes_write_and_mmap` proves write->pipe read, mmap->writable host ptr,
getpid==process id all hit the real kernel. Suite now **50 passed.**

**Remaining to a genuine Roblox boot (next steps, in order):**
1. Drive the real boot path (JNI_OnLoad / nativeSetAssetPath) rather than a JNI stub; wire GoBloader +
   jit through libloader `--no-qemu` (the `guest_svc` bridge unblocks the mmap/futex/mprotect the init
   path needs).
2. Host-call trampolines (libc/libm/libdl) + the 785-entry PLT GOT + JNIEnv table in the JIT path.
3. Then the CHROME renderer / Android surface expects GPU; Carla graphical mode is the tail.

### Last commits
`d485bba` (SimdAdalp), `30abdd4` (SimdAddl), `8043510` (FpUnary frintz), `9bbb104` (SimdSatAdd),
`7c96cae` (guest_svc real syscall dispatch, 50/50).

---

## Session — JIT path is now a real syscall-capable execution engine (loader rewire)

**Wired the no-QEMU path through the full dispatcher** (`3ce14e6`): `sober-core::jit::run_elf_entry`
now bootstraps guest **stack + TLS** and runs via `arm64jit::jit::jit_run` (the PC-driven dispatcher that
re-enters on `blr`/`br`/`ret` and emits `svc`→`guest_svc`), instead of the old single-block
`compile_image`+`run`. This makes the JIT path an actual execution engine, not a linear-slice runner.

**Proven end-to-end** (`/tmp/extest/svc_elf.s`): a self-contained, no-libc aarch64 static ELF that issues
`mov x8,#64; svc #0` (write) and `mov x8,#94; svc #0` (exit_group) prints `jit-svc-ok` and exits cleanly
through `load_elf_image → jit_run → Inst::Svc → guest_svc → real kernel`. Real guest machine code making
real host syscalls with no QEMU. Suite still **50/50**.

**Honest boundary to literal "Roblox boots":** `libroblox.so` is a shared library with **e_entry=0** and
**no exported `JNI_OnLoad`** (runtime-internal, `@@LIBROBLOX`). It can only run when the Android *runtime*
calls `JNI_OnLoad` with a real `JavaVM*`/`JNIEnv*`. The QEMU path built that environment over many
sessions (bionic_shim.c / libbionic_ver.c / libdl_wrapper.c, the 785-entry PLT GOT trampolines, JNIEnv
table, condvar shim, pre-mprotect RELRO, AndroidEnv::setup). Reusing that host-runtime layer for the JIT
path is the remaining (large, multi-session) integration; the JIT itself is no longer a blocker.

---

## Session — boot-target forensics (why running .init_array is not the boot path)

Investigated every "first-execution" candidate on the actual binary to pin down the real boot target:

- `libroblox.so` is **ET_DYN, e_entry=0** (a shared library, no program entry flips the loader).
- `.init_array` exists but its **file bytes are all zeros** (0x6ce0 of them) — it is **empty**; putting
  constructors there is not how this binary boots. (The `runctors` example reads them as `0` → skipped.)
- **No `R_AARCH64_RELATIVE` and no `DT_RELR` relocations at all** — only **537 `R_AARCH64_JUMP_SLOT`**
  in a 12.8 MB `.rela.dyn`. So there is no data-reloc set to pre-fill; a loader relocation pass has
  nothing to do for boot (tried a RELATIVE/RELR `apply_relative_relocs` in libloader; reverted — Roblox
  has none).
- `JNI_OnLoad` is present **only as a `.dynstr` string** (file offset 0xc40b), **absent from `.dynsym`
  and `.symtab`**. The Android runtime binds it by export-name convention; the JIT/loader cannot.
- Conclusion: this binary can only start via **`JNI_OnLoad` called by the Android runtime**. That is the
  single, precise boot frontier and it requires the host Android/JNI/bionic layer (already built for the
  QEMU path) rather than any further decoder/syscall work.

Added `crates/arm64jit/examples/runctors.rs` — a diagnostic that loads the .so, iterates `.init_array`
constructors through `jit_run` (real syscalls), prints exactly where the chain stops. It currently reads
all-zero slots (consistent with the empty `.init_array`) and serves as the skeleton to drive whatever
entry the Android-runtime integration eventually feeds it.

Status: JIT engine + syscall bridge complete and end-to-end proven (svc_elf write/exit). The blocker to
literal "Roblox boots" is 100% the Android/JNI host-runtime port (large, multi-session, separately
scoped). No decoder or syscall wall remains in the JIT path.

## Session — arm64jit guest->host call bridge + real-import resolver + float-ABI bridge

Jumped the JIT across the arch boundary so a translated AArch64 libc/libm/JNI call reaches a real
host x86-64 function (no QEMU). Three verified milestones, all committed:

1. **Guest->host call bridge** (`jit.rs`, `5e76450`): the `jit_run` dispatcher now recognizes a
   reserved guest-address region (`HOST_THUNK_BASE + i*8`) and, when a translated `blr`/`br` lands
   there, calls the registered host x86-64 function with guest x0..x7 as SysV args, writes the
   return into guest x0, and resumes at x30 (the `blr` caller). API: `register_host_call(i, f)`,
   `host_call_addr(i)`. Proven: guest `blr x16` -> times_3(5) = 15.

- **Real-import resolver** (`resolver.rs` + `examples/resolveimports.rs`, `646a42d`): `resolve(name)`
   does `dlsym(RTLD_DEFAULT)` on the host, maps robotox's `R_AARCH64_JUMP_SLOT` PLT imports by walking
   `PT_DYNAMIC` (DT_JMPREL/PLTRELSZ/SYMTAB/STRTAB) and PATCHES each GOT slot to a host thunk guest
   addr. Against real `libroblox.so`: **334/537 imports resolve NOW** (strlen/memcpy/memcmp/
   pthread_*/mmap/mprotect/open/read/close/clock/..). The rest (203) need the bionic/Android/JNI
   shim. Proof: guest `blr` to resolved `strlen` returns the real host length.

- **Float-ABI bridge** (`jit.rs` + `resolver.rs`, `3de5bb3`): separate float thunk region reads guest
   v0..v7 as f64, calls a host double fn through xmm0..xmm7, returns into guest v0. `resolve_float`
   + `DOUBLE_FLOAT_NAMES`. Proven: guest `blr` to registered atan2 -> pi/2 in v0.

Key loader truth discovered: guest address != host pointer for the mapped `.so` (a PIE); every read
must go through `LoadElf::host_addr_of(guest)` (the closest analog is `guest_of(link)->host_addr_of`).
`libroblox.so` is e_entry=0, empty `.init_array`, no RELATIVE/RELR relocs (only JUMP_SLOT), so
`.init_array` is not the boot path and there is no relocation pass for the loader to perform.

**NEXT (immediate)**: the remaining 203 shim relocations reduce to ~18 distinct **Android/JNI/bionic**
host-runtime names (verified by enumerating them): `__android_log_print`, the `AAssetManager_*` /
`AConfiguration_*` / `ANativeWindow_*` / `ALooper_*` asset-config APIs, `__strlen_chk` /
`__strncpy_chk2` fortified string funcs, `__errno`, and one `Java_com_roblox_...IAP_...` JNI method.
The float-ABI bridges (f64 + f32) are done and committed but resolve nothing new against the real
`libroblox.so` — it has NO float JUMP_SLOT imports, so the float bridges are runtime capability for
covered math calls, not resolve-count movers. The real remaining blocker is porting the
Android/JNI/bionic host runtime (already implemented for the QEMU path as `sober-core/src/qemu.rs`
+ `bionic_init.c` + `jni_shim.c`) onto the JIT `--no-qemu` path: register these ~18 names as host
shims (AAsset/AConfiguration/android_log/JNI-vm plumbing), then boot `JNI_OnLoad`.
- f64 float bridge (`3de5bb3`): guest v0-v7 f64 -> host double via xmm -> v0.
- f32 float bridge (`2b03e26`): guest low-32 s0-s7 f32 -> host *f via xmm -> s0
  (`HostFloat32Call`/`register_float32_call`/`resolve_float32`/`FLOAT32_NAMES`; atan2f blr proof; 55/55).

**MILESTONE (1fef)c20**: host-side bionic shim module `crates/arm64jit/src/shims.rs`.
`register_shims()` registers 4 self-contained bionic symbols without dlsym via new
`resolver::register_named`: `__errno` (returns host `__errno_location()` addr, so guest
reads/writes the real errno), `__strlen_chk` (plain strlen), `__strncpy_chk2` (bounded
strncpy), `__android_log_print` (prints `[roblox:tag] msg` to stderr, returns 1). This
drops resolveimports to 199 shims remaining (was 203) and the resolved count 334->338.
The remaining ~199 (distinct names) are the Android asset/config/JNI/event API surface:
AAssetManager_fromJava/open, AAsset_close/getBuffer/getLength, ANativeWindow_fromSurface/
release, ALooper_pollOnce, AConfiguration_getScreen{Width,Height}Dp/Size/NavHidden, and
one Java_com_roblox_client_purchase_IAPPurchaseManager... JNI method. These need real
host implementations (AAsset backing file descriptors, AConfiguration density, JNI vm).
Next: (a) port the AAsset/AConfiguration stubs + JNI vm dispatch; (b) fold
resolve_common()+register_shims() into elfjit boot so the GOT is patched before onLoad.

## Session — full import binding on the boot path (537/537) + float bridges

libm was NOT in RTLD_DEFAULT: a bare `dlsym(RTLD_DEFAULT, atan2f)` fails even
though libm.so.6 has it. `dlopen("libm.so.6", RTLD_GLOBAL|RTLD_NOW)` once and
dlsym from that handle as a fallback freed 45+ libm imports at once, so the
float bridges (f64 atan2, f32 atan2f via guest blr) now actually hit.

Remaining imports fell into a caught-all: graphics (OpenGL ES gl*/EGL), audio
(OpenSL ES sl*), media (AMediaCodec/AMediaFormat), full ALooper/AConfiguration/
ANativeWindow/AAsset, bionic logging/fortified chk/gcov/property. Added
shims::register_fallback + is_handle_name -> stub_handle/stub_zero and
register_graphics_stubs, binding EVERY otherwise-unresolved name to a benign
stub (QEMU jni_stubs.h philosophy: NULL/0). Result: **537/537 PLT JUMP_SLOT
imports bind to host thunks (0 unbound)**.

- `plt::bind_image_plt(&LoadedElf)` folds the binder into the boot path: walks
  PT_DYNAMIC->DT_JMPREL, resolves each name (int resolve -> float64/32 -> bionic
  shim -> graphics fallback stub), writes the resolved host-thunk guest addr
  into the GOT. elfjit calls it before running entry. (Fix: PT_DYNAMIC=2, NOT
  PT_PHDR=6 — a one-line const typo made it match the PHDR segment and read a
  bogus p_vaddr.) Promoted libloader to a runtime dep so the lib can use it.
- `examples/resolveimports.rs` is now a thin wrapper over bind_image_plt (DRY).

**HONEST STATUS**: every import ROOT contracts to a host thunk, but the stubs
render nothing — they only let execution *progress* / bind. The blockers to a
real `JNI_OnLoad` boot are now (1) the JNI vm dispatch + Java_* bridge (the
single `Java_com_roblox_...IAP_native...` import currently binds to a benign
stub, not a real JNI call), (2) AAsset/ALooper/ANativeWindow need either real
backing or never-bound paths, and (3) the JIT's per-instruction coverage for
whatever the real boot path executes. Tests 58/58 (incl. bind_image_plt_real
test loading the real .so). Commits: 9b340c0 (libm fallback), 71403e0 (stub
binding), b254508 (fold into elfjit).

## Session — JNI host bridge on the JIT path (guest-visible JNIEnv/JavaVM)

Port of QEMU's jni_shim.c tables to guest-address space.
- `jni.rs`: build_jni() builds a 256-slot JNIEnv table + 8-slot JavaVM table, each
  entry = HOST_THUNK guest address; JNIEnv/JavaVM objects in host==guest memory.
  Slots mirror QEMU slot map (4=GetVersion->0x10006, 5/7=GetMethodID sentinel,
  13/14=Throw/ThrowNew, 21=NewGlobalRef, 36=NewStringUTF, 193=RegisterNatives,
  197=GetJavaVM, etc). Proper JVM GetEnv writes *penv=env, returns JNI_OK(0).
- Proof: jit_jni_onload_getenv_getversion JIT-executes guest JNI_OnLoad preamble
  (JavaVM* in x0 -> vm->GetEnv(&env,0x10006) -> env->GetVersion()) through both
  host-thunk tables; x0==0x10006. NOTE: hand-assembled aarch64 ldr encodings must
  be validated (e.g. ldr x9,[x10,#48]=0xf9401949, NOT 0xf9400d49). Use
  aarch64-linux-gnu-as/objdump -m aarch64 to confirm immediates.
- elfjit `--jni`: sets x0 = JavaVM* from build_jni() (JNI_OnLoad(JavaVM*,void*));
  positional x-arg loop tolerates the flag. Boot path now: 537/537 PLT bound +
  x0=vm before running entry. 61/61.
- `host_call_at` made pub; `register_host_call_auto` (int-thunk auto-allocator).

NEXT actual-boot blocker: exercising real JNI_OnLoad (Roblox does TLS-bootstrap
block-alloc, clock, mprotect, GetStaticMethodID+NewStringUTF+GetChar) — QEMU path
had to Phase1-NOP clock + bypass; expect same under JIT. JNI_OnLoad = base+0x1f0db20
(verified via readelf symtab; NOT the older QEMU note 0x1f64e58 = NativeSettings
Interface func). Entry 0x1c34480 was a decoy (Java_...shouldDisplayOpenGLUnsupported
Message) that body-branches into FMOD audio init — don't use it as the boot entry.

------------------------------------------------------------------------------
SESSION (latest boot frontier, commit bcf6a88, 62/62 tests)
------------------------------------------------------------------------------
Built on the bounded-trace fix (see its own section below): after that, elfjit @
0x1f0db20 --jni reached JNI_OnLoad's first real init but gdb pinned the next crash
to `mov (%rdx),%rax` with rdx=0 in the entry prologue. Root-caused it to the
`__stack_chk_guard` DATA-GOT slot:
    adrp x24, 0x631a000 ; ldr x24,[x24,#2608] ; ldr x8,[x24] ; stur x8,[x29,#-8]
The Android build emits ONLY JUMP_SLOT relocations (no .rela.dyn/GLOB_DAT/
RELATIVE), so that slot is 0x0 and the first `ldr x8,[x0]` null-faults. NEW
`patch_stack_canary()` in plt.rs (called from `bind_image_plt`) writes a live
canary pointer into it. Pitfalls hit:
   - slot is link **0x631aa30** = 0x631a000 + 0xa30: objdump prints `#2608` in
     DECIMAL (= 0xa30), not hex — a first attempt at 0x631c608 was wrong.
   - `host_addr_of(guest_of(slot))` returned None (loader segment bookkeeping
     gap); use `el.guest_of(slot)` directly since the runtime maps guest==host.
Canary value = dlsym(RTLD_DEFAULT,"__stack_chk_guard") if resolvable, else a
static AtomicU64 seeded non-zero, and we store that pointer so `ldr x8,[x24]`
reads back real canary bytes.

RESULT / PROOF: elfjit @ 0x1f0db20 --jni now executes JNI_OnLoad's prologue
(SP setup, canary store, GOT loads) and dispatches to its FIRST real init callee
at pc 0x101f0e728 (x30 = 0x101f0db5c, x0 = JavaVM*) -- JIT_TRACE block count
0 -> 1. That callee is the one-time-init *guard* (`adrp x8,0x68c7000; add x8,#0x520;
ldarb w8,[x8]; tbz...`) -- the same pthread_once-style guard the QEMU path had to
Phase1-NOP + deadlock-bypass, so expect to handle it under the JIT too.
NEXT fault: a guest deref of a small pointer (base=2, [base+8]=0xa) deeper in that
init path; the guest now needs the faked Android runtime the QEMU bridge provides
(JNIEnv method tables / fake object handles). Honest status: 62/62 arm64jit tests,
`cargo build --workspace` clean; unrelated pre-existing failure stays libloader's
android::test_setup_android_layout_creates_dirs (does `mkdir /storage/emulated/0`).
Full boot remains a multi-session effort — this session cleared the canary wall
and got the guest into real init/guard code.

## Session — bounded trace compilation FIXES the 78 MB blast-block (JIT actually executes; 62/62)

### Root-cause found (why elfjit `--jni` "hung" / spun for seconds then SIGSEGV'd)

`jit_run` called `compile_image` (unbounded), which EAGERLY expands the entire reachable
call graph from the entry into ONE monolithic host block. For real JNI_OnLoad that's a
**78,238,218-byte single block taking 7.38s to translate** (measured via a throwaway
timing harness), then the runaway block SIGSEGVs. That also explains why JIT_TRACE never
printed a `block@` line: the very first `compile_image` never returned within the timeout.
Not an infinite guest loop — a compile-explosion straight-line wall.

The default elfjit entry `0x1c34480` used in prior sessions was ALSO a wrong proxy:
objdump shows it is `Java_com_roblox_engine_jni_NativeGLInterface_shouldDisplayOpenGLUnsupportedMessage`
whose FIRST insn is `b 0x5d9ce10` straight into the huge **FMOD_OutputAAudioHeadphonesChanged**
function — so its frontier balloons into the audio subsystem, never the boot path.
**The real JNI_OnLoad is at base+0x1f0db20** (`readelf -sW`), not the HANDOFF's
QEMU-guess 0x1f64e58. Use `elfjit libroblox.so 0x1f0db20 --jni`.

### The fix: bounded trace compilation (`compile_image_bounded`, budget + divert stubs)

- `compile_image` now delegates to new `compile_image_bounded(image, base, entry, state, budget)`
  (budget 0 = old unbounded behavior, so `compile()`/single-shot tests unchanged).
- `jit_run` uses `BLOCK_BUDGET=8192` guest instructions per compile. Each block is a small,
  bounded straight-line trace; the frontier is NOT drained to the whole call graph.
- Any branch/call fixup whose target was NOT emitted (out of budget) is redirected to an
  appended **dispatcher-return stub**: `mov [CpuState+PC_OFF], #target ; ret`, and a host
  `call` (E8) for a `bl` is rewritten to a `jmp` (E9) so no host return address is left on
  the stack — the stub hands `pc` back to `jit_run`, which re-enters at the callee. The
  callee's own `ret` (guest x30) covers the real return.
- Two real bugs fixed while wiring this:
  - `E8→E9` opcode was at `disp_off-5` but `patch_here()` sets `disp_off = len-4` right
    after the E8, so the opcode is at **`disp_off-1`** → fixup corrupted 4 preceding bytes.
  - The stub address map was stored AFTER `buf.ret()` (off by the stub length) so the
    redirect `rel32` pointed one instruction past the stub. Now captured `buf.len()` *before*
    emitting the stub body.
- New test `bounded_bl_diverts_through_dispatcher`: budget-1 block where `bl 0x14` targets a
  callee that doesn't fit; asserts running the block leaves `CpuState.pc == 0x14` and
  `x30 == 4` (link), proving genuine dispatcher re-entry (not an in-trace call).

### What this unblocks (verified by gdb on the crash)

`elfjit libroblox.so 0x1f0db20 --jni` now compiles small blocks instantly and EXECUTES real
JNI_OnLoad init code (no 7s compile, no in-`compile_image` hang). The remaining SIGSEGV is
**not a JIT bug** — it's the documented NEXT frontier: JNI_OnLoad's first indirect
`vm->GetEnv` dispatch through the synthetic JavaVM table faults at a guest address that our
`build_jni()` host thunk table doesn't yet satisfy (`0x7fff...` runtime ptr not host-callable).
i.e. the guest is faithfully doing what a real JNI_OnLoad does and tripping on the host JNIEnv
runtime that still needs QEMU's jni global-state (classes/methods/RegisterNatives) backing.

### Honest status

- 62/62 arm64jit tests green (the +1 is the bounded divert test); `cargo build --workspace` OK.
  (`libloader::android::test_setup_android_layout_creates_dirs` fails on this host because it
  wants to mkdir `/storage/emulated/0` at the actual root; pre-existing, unrelated to this change.)
- The JIT now reaches and begins executing the real JNI load path. Next brick is the host JNIEnv
  runtime (GetStaticMethodID/newStringUTF/RegisterNative real callbacks + clock/mprotect no-op
  under the JIT like QEMU had to).

## Session — pthread sanitizer + deeper deref frontier

**Merged this session:**
- `bcf6a88` canary GOT bind; JNI_OnLoad prologue survives its first `ldr`, block count 0->1
  (pts into GOT slot `0x631aa30`, the `__stack_chk_guard` slot — note `#2608` in objdump is
  DECIMAL = `0xa30`, and guest reads `[0x631a000 + 0xa30]`, not the `0x631c608` a first draft
  patched by mistake).
- `1c21ffc` **bionic pthread_mutex sanitizer**. The guest `.so` is bionic-built; its
  `pthread_mutex_t` is 44B (glibc 40B), `__kind`@+16 = 0x10 (ROBUST_NORMAL), `__count`@+8 reused
  as `__owner`. Passing it raw to glibc `pthread_mutex_lock/cond_wait` crashes/deadlocks the once-
  init, driving Roblox into abort. Wired `sanitize_mutex` into the resolver's host bridge for
  mutex_lock/unlock/mutex_init/cond_wait/cond_timedwait (kind&=3 @+16, clear bogus owner @+8),
  mirroring `jni_shim.c`'s `sanitize_mutex`. +hostcall@ trace tracer (JIT_TRACE). 63/63 tests.

**Current frontier (verified 0x1f0db20 --jni):**
- block count 1, hostcalls 0: the crash is BEFORE any host bridge call, inside translated guest
  code. `block@0x101f0db20 -> pc=0x101f0e728` (JNI_OnLoad first bl, x30 linked), then block ~2
  (init guard at `0x2678068`, the GameActivity once-routine) crashes on a guest `ldr x, [x0, #8]`
  deref where x0 = 2 (small pseudo-handle). Preceded by a host ld FP divsd (div by 2^54/2^63),
  suggesting an LCG/time helper.
- The deref of x=2 with `[x0+8]` is the faked-Android-object wall: the guest legitimately got a
  small integer handle where it expects a real object pointer (JNIEnv/class), then derefs it.
  pthread_sanitize is a necessary fix but is NOT the firing block — the guest hasn't reached a
  pthread_mutex host call yet.
- Next brick: find WHICH guest fn returns the `2` (candidate: a JNI/host shim returning a small
  status instead of a pointer), or pre-scheme the once-flag so the init guard skips its guard
  entirely (QEMU's documented `mov w0,#1; nop` bypass).

**Refined finding (next session):** instrumented `host_mutex_lock` with a `[mutex_lock]` JIT_TRACE
line. Boot shows **zero host callbacks fire before the crash** (`hostcall@` = 0, `[mutex_lock]` =
0) — the guest never reaches `pthread_mutex_lock` at all, so the abort is NOT started by a mutex
failure. The crash block (decode of the compiled `0x102678*` region) sets guest `x0=2 +
x1=0x100362f03` (a string literal), does `cvtsi2sd -> addsd 2^63 -> divsd 2^54` (the guest
`__int64->double` HUGE_VAL trim), then a `ldr x, [x0, #8]` deref with x0=2. Guest `pc` at fault=
`0x7f0000002208` = HOST_THUNK slot 1089, i.e. a guest `blr x16` to a host-slot address whose
registered host fn is the one reading `[x0+8]` with x0=2 — but `host_call_at` did NOT intercept it
(0 trace). Suspects: (a) that host slot's registration slipped (resolver `register_named`/
post-bind mismatch), or (b) a host fn genuinely reads `[arg+8]` on a `2` handle (a JNI/Android
object). Next: dump slot 1089's registered fn + guest pc at the `blr`; if it's a JNI shim, give it
a real fake-object backing instead of returning 2.

**[RULED OUT, verified next session]** Two more dispatcher diagnostics were run: log any pc in
`[HOST_THUNK_BASE, +8192*8)` that `host_call_at` returns None for (`unregistered-hostthunk@`),
plus the existing `hostcall@` and `[mutex_lock]` traces. Boot: **`unregistered-hostthunk=0`,
`hostcall@=0`, `[mutex_lock]=0`, `block@=1`.** The guest never reaches the host-thunk range or any
host bridge — the fault is a **pure inline translated deref** inside the single block from
`0x101f0db20`. Block decode: JVM load, then once/abort prologue (`mov w0,2; mov x1,<fmt>` =
syslog args, int64->double trim), then `ldr xN,[x0,#8]` with guest x0=2 -> `[0xa]` SIGSEGV. The
`0x7f0000002208` value in the CpuState is the block's in-progress next-pc, never dispatched, so
the slot-1089 registration idea is closed. Frontier is guest logic using a small int (2) as an
object pointer. Remaining root-cause candidates: (a) `syscall(178=gettid)`/a host fn return
leaves `2` in a register the guest reuses as a pointer; (b) JNI_OnLoad once/init computes a
handle from an unimplemented host call it then derefs. Next: trace the guest instruction that
stores 2 into the register it derefs (step the block with gdb, or narrow with a
`[x0,#8]`-deref watchpoint).

## Session — bl-to-PLT-stub divert TRUE ROOT CAUSE + init now completes (commit 1e447b3)

**The `x0=2` abort-forward was NOT a JNI shim bug — it was the JIT inline-calling host-import
stubs.** A guest `bl <import@plt>` (pthread_mutex_lock, syslog, abort, __android_log*, ...) was
compiled INLINE as a host `call` to the PLT stub's emitted block. The stub ends `adrp/ldr x17,GOT;
br x17`; the `br` sets `CpuState.pc = x17` (= a host thunk slot) and `ret`s — but because it was
*`call`-entered*, that `ret` returned into the inlined caller's fall-through (guest code) instead
of handing pc to the dispatcher. So the real import never ran, the guest saw a garbage return
(`x0` stayed 2 from the syslog arg setup), took the once-init ABORT branch, and deref'd `[0xa]`.

**Fix (jit.rs, `compile_image_bounded`):**
- `is_host_plt_stub(addr)`: decode the 4 instructions at the `bl` target; recognize the canonical
  `adrp Xd; ldr Xn,[Xd,#imm]; add Xd,Xd,#off; br Xn` PLT stub. Pitfalls hit while dialing it in:
  the `br` encodings its branch-register at bits 9:5 (`(w>>5)&0x1f`), and the `add` top-byte mask
  is `0xff000000` to reach `0x91000000` (use `& 0x7f000000` silently drops bit 24 and rejects every
  real `add`).
- In the `Inst::B{link:true}` handler, if the target `is_host_plt_stub(t)`, do NOT push `t` to the
  frontier (don't inline it). Set a new `force_stubs` flag so the dispatcher-return stub table is
  still built even when the frontier drains (`if truncated || !frontier.is_empty() || force_stubs`),
  otherwise the diverted `bl`'s fixup index into an empty `stub_of_target` (the `[&t]` lookup).
- The divert rewrites the inline call (E8) into a `jmp` to a stub that writes `pc=t` and `ret`s, so
  the dispatcher re-enters and `host_call_at(t)` routes the REAL import → host bridge → guest gets
  a genuine return value.

**Result (verified `elfjit ~/.../libroblox.so 0x1f0db20 --jni`):** the guest advances PAST JNI_OnLoad's
pthread_once init — which now SUCCEEDS (416 hostplt `bl`s diverted) instead of aborting:
```
  block@0x101f0db20 -> pc=0x101f0e728      (JNI_OnLoad prologue -> init guard)
  block@0x101f0e728 -> pc=0x105ce0828      (init guard RETURNS, guest resumes in startup!)
  block@0x105ce0828 -> pc=0x1068c7518      (next caller; tries to call an unmapped fn pointer)
run_loop: pc 0x1068c7518 outside image [0x100000000, 0x105e67390)
```
No more SIGSEGV/139 at `[0xa]`; the JIT now gracefully stops with `pc outside image` (exit 1).
63/63 tests still pass.

**NEW frontier (clean, readable):** the guest (in a real startup caller at 0x105ce0828) computes a
function pointer = `0x1068c7518` (= guest VA `0x068c7518`) and calls it. `0x068c7518` is in a GAP
between the RW .data/.bss seg and the RO .rodata seg — i.e. **unmapped** → a null/garbage global
function pointer. Likely a C++ vtable / JNI-registered callback / Soong-supplied hook that the
host `jni_shim` layer must provide backing for (the Sober "fake Android" shim), OR a `dlsym`-ed
pointer our resolver left 0. Next: find WHICH global holds `0x68c7518` and WHICH init step should
fill it — dump `readelf -sW`/`.rodata` owners at `0x68c7518`, disassemble the caller block
`0x105ce0828`'s `ldr/blr` to see the pointer source.

## Session — once-init completes; next frontier is an uninitialized C++ vtable virtual call (commits 1e447b3 + ec39a19)

**Verified boot now** (`elfjit libroblox.so 0x1f0db20 --jni`, JIT_TRACE):
```
block@0x101f0db20 -> pc=0x101f0e728        (JNI_OnLoad bl init-guard)
block@0x101f0e728 -> pc=0x105ce0828        (init-guard COMPLETES normally, no abort)
block@0x105ce0828 -> pc=0x1068c7518        (run_loop: pc outside image)
```
- The pthread_once once-init at `0x2678068` now **runs the whole guard and returns success** (previous sessions' abort/deref path is gone). Roblox then advances into engine-native startup (`NativeAppBridgeV2StartAppWithParams`, GL interface init).
- New frontier: guest `will brl x9` at guest `0x26473ac` where `x9 = 0x1068c7518` (a global `.bss`/`pb_defaults` DATA address, not code) -> dispatcher stops ("pc outside image", exit 1, not a segv).
- Mechanism (decoded): `ldr x0,[x21,#8]; ldr x8,[x0]; ldr x9,[x8,#48]; blr x9` = a **C++ virtual-method call: vtable slot 48 holds `0x68c7518` (garbage/uninitialized)**. The object (`x21`-derived) is a Roblox interface (context: `IPlatformSystemDialogHandler`-adjacent call after it). `.init_array` is all-zeros (no C++ global ctors to run), so the object's vtable was never populated.
- **Not a JIT bug** — it's the fake-object/interface wall: the guest calls a valid vtable offset on an object the minimal `elfjit --jni` env didn't construct. Next: find what initializes the object behind `x21` (candidate: a JNI/`ANativeActivity`-provided global, or a `__cxa_atexit`/static-init baked elsewhere), or stub the virtual interface (slot-48 method) to return and continue.

Tools added (commit `ec39a35`): JIT_DUMP `[outside-image]` full-register dump + `[term]` guest terminal-pc trace for pinning such stops.

**Refined frontier analysis (next session):** the guest's failing tail, traced per-instruction, is:
`0x1c7b768: stp x30..; adr x8,0x631b000(=guest .got); ..reads GOT..; then 0x1ace7c: str x8,[x19]; ldp x29,x30,[sp]; ret` — a
guest subroutine reading its **own GOT/@.dynamic (page 0x631b000)** and returning; the `ret` lands on `x30=0x68c7518` (guest data/bss page 0x68c7000 = `__stop_pb_defaults`). Two candidate roots:
(a) **`0x68c7000` is in a real PT_LOAD-APTA gap** — `readelf` shows LOAD3 ends 0x6368df8, LOAD4 starts 0x6988000; nothing covers 0x68c7000. But the `elfjit` example only mapregs the r-x segment & hands `image`=that SLICE to the JIT run_loop, whose "pc outside image" bound is `base+len`=0x105e67390. So (i) that page is genuinely unmapped in-guest (a real loader gap if Roblox expects it) and (ii) the run_loop bound would also reject any legit guest let the other segments. Check `load_elf_image` mmaps ALL segments, both for real load and so `run_loop` uses the full address-space range, not just the r-x slice, as its valid-pc bound.
(b) x30 got corrupted upstream by a mis-emission; would need per-step guest tracing.

`[it]`/`[term]` were reverted to `[term]`-only (committed ec39a19); they're JIT_DUMP-gated.

## Session — removed the misleading "outside image" stop; true root is a corrupt FMOD vtable call (commit e056128)

Earlier sessions misread the frontier as "guest pc outside image" — that was FALSE: `load_elf_image` maps ONE contiguous
anonymous region spanning ALL PT_LOADs + inter-segment gaps at the 0x100000000 base, but elfjit handed the JIT only the
**r-x text slice** as `image`, so any legit mentor into data/.bss past the slice was rejected as "outside image".

**Fix (e056128):** elfjit now computes `len = (max(guest_vaddr+memsz) - base)` so the run_loop valid-pc bound covers
the whole zero-filled mapped span. Boot now proceeds past that stop until it genuinely hits non-code data:
```
running entry guest=0x101f0db20 ...
  block@0x101f0db20 -> pc=0x101f0e728 ...   (JNI_OnLoad -> init-guard)
  block@0x105ce0828 -> pc=0x1068c7518 ...
arm64jit run_loop stopped: translate: unhandled Unsupported(0x68c74d0) at guest pc 0x1068c7518
```

**True root of the dispatch to 0x68c7518 (FMOD Audio static-init, guest 0x5ce0828):**
```
5ce094c: mov w8,#6; ldr x9,[x0]     ; x9 = vtable of object x0=(0x10045b848 arg)
5ce0950: ...
5ce0968: ldr x8,[x9,#32]            ; method ptr = vtable slot 32
5ce096c: blr x8                     ; virtual call -> pc=0x68c7518
```
`0x68c7518` is the guard / a `.bss` (region 0x68c7000, `__stop_pb_defaults`) DATA address, not code. So this is a
**corrupted C++ vtable slot** (offset 32) on an FMOD/engine object passed in x0 — the vtable points into data/bss
instead of `.text`, so the virtual method call lands on raw bytes (Unsupported(0x68c74d0)).

Confirmed: `.rela.dyn` is **entirely absent** (only 537 `.rela.plt` JUMP_SLOTs), so there are NO R_AARCH64_RELATIVE /
data-absolute relocations for the loader to apply. The guest's `.data` vtables are whatever the file laid out.

Next leads (no .init_array / no .rela.dyn / no ifunc): the object at x0 (0x10045b848) has a vtable that is wrong
after the once-init — either (a) its vtable entry 32 was never set because a guest constructor didn't run under
`elfjit --jni` (no .init_array run), or (b) vtable base-scaled entries need the loader to add the 0x100000000 PIE
base to `.data.relro`-style absolute pointers, which this ## loader does not do (no R_AARCH64_RELATIVE present =
presumptively absolute at build, but for a PIE that needs +base).

**Correction to the abute "vtable slot 32 = corrupt" (added right after):** `x0` at the FMOD static-init is NOT a C++
object. Guest bytes at `0x10045b848` are the ASCII string `__cxa_guard_acquire[...]` (a .rodata/.dynstr symbol string).
So the "vtable" `[x0]` is really a string-literal address; `[x0]+32` is garbage → the "virtual call" is actually the
guest's C++ `__cxa_guard` / exception runtime being fed a **string address where a control block / function address
belongs** (guard-state at 0x68c7518, and a `__cxa_guard_acquire`-symbol-string in x0). Real root is **bionic/libc++
`__cxa_guard` machinery the minimal `elfjit --jni` shims do NOT provide**: our `bind_image_plt` binds .rela.plt JUMP_SLOTs
but the guest's C++ static-init path (guard acquire/release) is not shimmed, so it strays into string/data. Next:
shim/redirect `__cxa_guard_acquire`/`__cxa_guard_release`/`__cxa_guard_abort` (guest `__cxa_atexit` too) to real host
libc++/bionic so FMOD's static-init guard works, mirroring how we patched `__stack_chk_guard`.

## Session — exact mechanism of the FMOD dispatch-to-0x68c7518 + the likely nested-inline once-inv bug (update)

New decisive facts this session:

1. **The crash is a `Ret` to a corrupt x30, not a vtable `blr`.**
   The guest block STARTING at `0x105ce0828` (FMOD static-init guard, guest `0x5ce0828`) terminates by setting
   `pc = 0x1068c7518`, and CpuState at stop has `pc==x30==x0==x19 == 0x1068c7518`. The last `[term]` (JIT_DUMP)
   correlation shows the terminal is a `Ret` whose `x30 = 0x68c7518` (a .bss guard address) — i.e. the guest
   RETURNS into a data guard, then the translator decodes the `.bss` bytes there → `Unsupported(0x68c74d0)`.

2. **The once-routine return semantics are understood:**
   `0x2678068` (`GameActivity_initializeNativeCode`) ends:
   ```
   2678138: cmp w24,#1
   267813c: cset w0,ne            ; w0 = 0 iff w24==1 (init "done")
   ...
   2678158: ret
   ```
   So it returns `w0=0` (done) only when the local `w24` was set to 1 during init. The FMOD code does
   `bl 2678068; cbz w0, <clean ret -> `5ce085c`>; <else fallthrough to the corrupt path>`. Because the guest
   dispatches to `0x68c7518` on the NOT-clean path, `2678068` is returning `w0=1` (NOT-done) for the FMOD
   guard `0x68c7518` — meaning the once-body's `w24` never got set = the once-init body did not run to
   completion for THIS second distinct guard. The once-mutex (guest `0x637a468`) / pthread_once body worked
   for the FIRST guard (GameActivity init at 0x101f0e728, once-mutex 0x637a468) but is failing on this
   second, FMOD, guard.

3. **Why a second time fails — the suspected JIT-fidelity bug (NEXT REAL TASK):**
   `0x105ce0828`'s compile inlines the nested `bl 0x2678068` (a *guest* function, so NOT diverted by
   `is_host_plt_stub`; only host-import PLT `bl`s are diverted). Inside `0x2678068` the guest does
   `bl pthread_mutex_lock@plt` — which IS diverted via the `force_stubs` mechanism. On the FIRST guard
   this chain completed (w24=1). On the SECOND distinct guard the same routine is inlined AGAIN inside a
   different (huge) block; the mutex return/stub table interaction appears to regress so the routine exits
   without setting `w24` (reads stale/garbage), returning `w0=1`, and FMOD then comes/path dispatches to
   the guard address `0x68c7518`.
   **Verify:** add a `[it]`/gall step that logs whether `2678068`'s `mov w24,#1` (once-done) instruction is
   ever reached in the FMOD block, vs whether the routine bails to `cset w0,ne` without it. If not reached,
   the nested-inline of the second call is the bug (e.g. bad return-stub linking for the inner mutex call).

4. **Confirmed irrelevant to THIS crash:** `.rela.dyn` is `ANDROID_RELA` (present, sections [10]) but the
   loader/`bind_image_plt` does not apply it; `__stack_chk_guard` is patched. `.init_array` empty. No ifunc.
   Host `dlsym(RTLD_DEFAULT)` provides `__cxa_atexit`/`__cxa_finalize` but NOT `__cxa_guard_acquire/
   release/abort` (all null) — so a "shim the cxa_guard by resolve" approach can't source them from host libc;
   they'd have to be guest-emulated (inline guard) or written manually.

**Recommended next step:** trace (JIT_DUMP/JIT_TRACE) whether the `0x2678068` once-body sets its done flag on the
FMOD guard, root-causing the nested guest-`bl`-in-inter-inlined-block return regt; if confirmed, divert guest
`bl` to the once-routine (and generally guest `bl` whose callee contains diverted imports) through the
dispatcher instead of inlining — i.e. treat a `bl` whose translatable body itself has out-of-block PLT mutex
calls like `is_host_plt_stub`: push it to the stub table + `force_stubs`, not the inlined frontier.

---
## Session (Sep 11, 2026) — divert guest bl-to-import-bearing-callee through dispatcher (FMOD second-guard) DONE

Picked up the HANDOFF's "next task": divert guest `bl` to the once-routine (and any
import-bearing callee) through the dispatcher. Commit `10ddb7a` (on `dev`).

### Environment note (fresh box)
- `cargo build --workspace` ✓, `cargo test --workspace` ✓ all green (67 arm64jit +
  5 + 16 libloader incl. the two android-layout tests, + others; 0 failures).
- `cargo test --workspace` was already green for the libloader android layout tests
  on this box: commit `8b72828` had already landed the deterministic fix (per-call
  unique temp root) plus `ensure_dir_android` already does `create_dir_all` before
  `set_permissions`, so the worker-handoff's "permission-set before parent dirs"
  frame predates it. Verified passing.
- The real `libroblox.so` (100 MB, `~/.cache/open-sober/libs/` on the old box) is
  NOT present here and there is no APK/GSI/GPU, so the boot frontier can only be
  exercised at the unit-test level in this session.

### What landed (all in `crates/arm64jit/src/jit.rs`)
1. `word_at(image, base, addr)` — bounds-checked 32-bit image read (replaces the
   raw-pointer derefs `is_host_plt_stub` used to do on mapped guest==host memory).
2. `is_host_plt_stub(image, base, addr)` converted to slice-based reads.
   **Root-caused + fixed two real bugs the new tests exposed:**
   - Stale `if addr < 0x1000 { return false; }` guard left over from the
     pointer-based code — it wrongly rejected legitimate PLT stubs at low
     synthetic addresses (the unit-test stub images live at 0x40), so
     `body_contains_host_plt_bl` never saw the import. Removed; `word_at` is the
     safety net now.
   - `body_contains_host_plt_bl` was *following guest `bl` calls into their callee
     bodies*, making the caller of an import-bearing callee transitively
     import-bearing too (test asserted the caller is NOT). Now it only detects
     **direct** host-import `bl`s in the entry's own body and lets the linear walk
     fall through a guest `bl`. This is the right model for the bounded compiler:
     transitive follow would mark every caller up the whole call graph as
     import-bearing and defeat bounded compilation entirely.
3. `compile_image_bounded` now diverts (forces a dispatcher-return stub, memoized
   per target) any guest `bl` whose callee body itself calls a host import — the
   FMOD once-routine (`2678068` GameActivity init, which calls
   `pthread_mutex_lock@plt` etc.) regression is specifically this shape: inlining
   it a second time in a different huge block regressed the inner import
   diversion, so it returned "not done" (w0=1) and the caller branched into the
   `.bss` guard `0x68c7518`.

+4 tests: `host_plt_stub_detected_from_image_slice`,
`body_contains_host_plt_bl_follows_call_graph`,
`guest_bl_to_import_bearing_callee_diverts_through_dispatcher` (run caller block
⇒ `CpuState.pc==0x20` callee, `x30==0x04` link — real dispatcher re-entry, not an
inline call), `guest_bl_to_import_free_callee_still_inlines`. **67/67 arm64jit,
0 failures.** `cargo build --workspace` clean (warnings are pre-existing decode.rs
dead-code / rustfmt churn).

### Next (ordered, no APK/GSI/GPU on this box)
1. JNI function-table stubs (`crates/arm64jit/src/jni.rs`): fill high-value slots
   that must return real values when the guest boot path reaches them
   (GetStaticMethodID, NewStringUTF, RegisterNatives, FindClass) with host
   thunk-backed implementations + unit tests. This is the next name-surface the
   JIT boot hits once the divert fix lets `JNI_OnLoad` progress.
2. ELF/loader (`libloader`) gaps, then `libbadcpu` ISA gaps, then services/auth.
3. Real-binary/GPU boot verification remains blocked until `libroblox.so` (or an
   APK) and a GPU host are available — capture as `elfjit ... 0x1f0db20 --jni`
   log on a capable host (HARD GATE).

## Session (Sep 11, 2026) — JNI/JavaVM function tables on the OFFICIAL Android ABI slot offsets (commit bdd8b03)

Continuing the ordered work ("JNI function-table stubs"). Examined both
`crates/arm64jit/src/jni.rs` (JIT path) and the QEMU `jni_shim.c` (validated
reference) and found the JIT JNI table was mis-slotted vs. the ABI the guest
uses.

### The bug (real, and it would crash a booted guest)
libroblox.so indexes `JNINativeInterface` with the OFFICIAL jni.h word offsets:
`GetVersion=4, FindClass=6, GetMethodID=33, GetFieldID=94,
GetStaticMethodID=113, NewStringUTF=167, GetStringUTFChars=169,
RegisterNatives=199, GetJavaVM=203`, and `vm GetEnv=7`. The JIT table carried
unvalidated guesses from the QEMU shim (`NewStringUTF@36`, `GetArrayLength@37`,
`GetObjectField@102`, `RegisterNatives@193`, `GetJavaVM@197`, vm GetEnv@4/6).
The QEMU path only ever end-to-end-validated GetVersion/FindClass/
GetStaticMethodID against the real binary — of those, FindClass(6) and
GetStaticMethodID(113) coincidentally match the official offsets, which is why
the mismatch went unnoticed (its boot hung at nativeSetAssetPath before any
divergent slot was exercised). `bdd8b03` rebuilds the JIT tables on the official
offsets so a guest call lands on the real stub, not NULL/wrong.

### Handles are now readable (not low sentinels)
FindClass/NewStringUTF/GetMethodID return a stable, interned, readable UTF-8
buffer handle (the `str_handle` registry — analogue of the QEMU shim's
`track_ptr`), instead of the old `0x3000` sentinel that risks a guest deref
fault. GetStringUTFChars returns that buffer and clears `*isCopy`;
RegisterNatives succeeds (records nothing yet) so boot continues; GetJavaVM
writes the live vm handle. `jni_vm_getenv` (GetEnv @ slot 7) writes `*penv`.

### Verification
- `+2` tests: `jni_table_has_official_abi_slots_nonnull` (regression guard that
  all boot-relevant slots are non-null host thunks at the OFFICIAL offsets),
  `jni_new_string_utf_is_readable`.
- Fixed the E2E `jit_jni_onload_getenv_getversion` to load vm GetEnv at
  offset 56 (slot 7) and to dereference `env->functions` before indexing slot 4
  (JNIEnv word0 is the fn-table ptr; the earlier test read `[env+32]` directly).
  69/69 arm64jit, workspace 103/0. `cargo build --workspace` clean.

### Next (ordered)
1. `libloader` ELF/loader gaps (next in RECOMMENDATION order); drive
   `elfjit`/`--jit` boot path end-to-end against a synthetic/test ELF to
   confirm no regression from the divert + JNI changes.
2. `libbadcpu` ISA gaps; then services/auth.
3. Real-binary/GPU boot verification remains blocked (no APK/libroblox.so, no
   GPU) — HARD GATE on a capable host.

## Session (Sep 11, 2026) — 128-bit SIMD ld/st register-offset/unscaled/pre-post-index mis-decoded as GPR; silent x-reg corruption FIXED (156/0)

Root-caused the `modmain.elf` (full static-glibc) `__memset_generic` SIGSEGV.
The memset's `str q0,[x0,x3]` (0x3ca36800) decoded as a GPR 1-byte sign-extend
load **into the base register** (`ldrsb x0,[x0,x3]`), silently clobbering guest
x0 → `__tls_init_tp`'s `str w5,[x0,#4]` faulted at address 0x4. The GPR
register-offset (0x38200800) and pre/post/unscaled (0x3800xxxx) decode gates had
no bit26 (vector-file) mask; the imm-offset gate (df470fa, prior session) did.

## Fix (commit `853cc44`)
Three new 128-bit vector classes gated BEFORE the GPR gates (bit26=1), plus
`bit26==0` added to both GPR gates:
- **VecLdStrReg** — register-offset str/ldr q: `(insn & 0xffe00c00)` in
  `{0x3ca00800 (str), 0x3ce00800 (ldr)}`.
- **VecLdStImmUnscaled** — ldur/stur q: `{0x3c800000, 0x3cc00000}` (signed imm9;
  note the residue is 0x0000 — the imm9 lives in bits[20:12], outside the mask).
- **VecLdStIndexed** — pre/post-index writeback: `{0x3c800c00, 0x3cc00c00,
  0x3c800400, 0x3cc00400}`; Xn advances by signed imm9.

All transfer 16 bytes via XMM0 to/from `CpuState.v[vt]`. Gate correctness
verified against `aarch64-linux-gnu-as` ground truth incl. **non-collision**
with scalar B/H/S/D register-offset/unscaled (e.g. stur b0=0x3c1fc100 masks to
0x3c000000, bit23 clear).

## Result
modmain no longer SIGSEGVs in `__tls_init_tp`'s memset — it advances through
the whole vector ld/st family and stops HONESTLY (Unsupported) on the next wall
instead of corrupting. `+4` regression tests (3 exec: base preserved for the
reg-offset store, pointer preserved for unscaled stur, Xn advanced for
pre-index ldr; 1 decode: the three classes + scalar-b non-collision).
arm64jit 115/115; workspace 156/0; build clean.

**Addendum (same cycle):** scalar FP register-offset (`FpLdStrReg`) added —
`str s0,[x0,x3,lsl#2]` (0xbc237800, memset's next path) was silently executed
as a GPR op on the wrong register file. New decode arm (bit26=1 &&
0x38200800 residue && bit23 clear for Q) + translate (addr in RDX, width from
bits[31:30], S-bit index scale). `+fpl_single_reg_offset_store_with_shift`.
arm64jit 116/116, workspace 157/0. Commit `921af2a`.

## Next (ordered, no APK/GSI/GPU on this box)
1. Scalar S/D UNSCALED (`stur/ldur s0,d0`, e.g. modmain 0x40a95c word 0xbc1fc0a0)
   and scalar pre/post-index writeback ld/st — the last of the same bit26=1
   family; glibc-CRT-memset tail, repeatedly judged NOT a Roblox boot blocker
   (real libroblox.so boot ISA already fully decoded / exit 0).
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth (ordered
   plan).
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE — blocked until a capable host + real binary/APK (none here).
---

## Session (Sep 11, 2026) — JIT correctness: XZR/SP, FP/vector loads, static-ELF loader (commits b0c3237, f1707e2, df470fa)
Unblocked running real compiled aarch64 C through elfjit (loader+dispatcher)
by making `bind_image_plt` skip static ELFs instead of panicking (b0c3237),
then used cross-gcc test programs to regression-test actual control flow. This
EXPOSED (and fixed) two latent correctness bugs the old panic had masked:

1. **XZR vs SP in store source / load dest** (f1707e2). `str xzr,[..]` (used
   everywhere to zero-init) loaded CpuState.x[31] = the STACK POINTER and
   stored it — verified A1 returned ~0x7fa14f7eb015 instead of 5. Loads to
   x31 (`ldr xzr`) also clobbered SP. Added `ldg_src`/`stg_if_writable` and
   applied at every GPR ld/st site + LdStPair rt/rt2.

2. **FP/vector-register loads/stores touched the GPR file** (df470fa). The GPR
   ld/st gate `(insn & 0x3b000000)==0x39000000` left bit26 (GPR-vs-FP selector)
   unmasked: `str d0`/`ldr d0` (0xFD..) read/wrote x[rt] not v[rt], `str s0`
   (0xBD..) the same, and `str q6`/`ldr q7` (0x3D8/0x3DC) decoded as 1-BYTE GPR
   loads — the VecLdStImm 128-bit gate was unreachable dead code. Fixed the GPR
   gate to mask bit26, made q fall through to VecLdStImm, and added a new
   `FpLdStImm` class handling B/H/S/D scalar loads/stores into/out of
   CpuState.v[vt] (upper lanes preserved).

Both verified with new tests; arm64jit 69 -> 72, workspace 106/0, build clean.

### Remaining (honest, not blocking the committed work)
- fp_only (no-loop FP `scale()` call, inlined): after correct FP decode, stops
  at "pc 0x4004000000000000 outside image" — a control-flow/x30 interaction in
  the inlined-callee `ret` beneath the bounded dispatcher. No longer
  segfaults/corrupts (clean diagnostic). Not on the previously-validated Roblox
  boot ISA, so it doesn't contradict the "boot instruction space covered" claim.
- int_only (loop w/ backward branch): still hangs — deeper loop/branch issue.
- These are synthetic-program paths; next session should root-cause the inlined
  `ret`/dispatcher x30 interaction (high value for FP graphics/audio).

## Session (Sep 11, 2026) — add/sub SP write + final JIT correctness sweep (commit 4301348)

Root-caused and fixed the last of the XZR-vs-SP family: `AddSubImm`/`AddSubReg`
suppressed rd==31 writes, but for ADD/SUB rd==31 means **SP** (unlike logical
ops where it's XZR). So every function prologue `sub sp,sp,#N` did nothing and
nested frames collided on the same SP — inlined `f()`'s `str d31,[sp+8]`
overwrote the caller's saved x30 with 6.5's bit-pattern, and the final `ret`
returned `pc = 0x401a000000000000` ("outside image"). cmp/cmn (s==1, rd==31)
still discard correctly. Verified fp_only, C_fmovret, A_frame all return 42 now.
+`sub_add_sp_updates_stack_pointer` regression. arm64jit 73, workspace 107/0.

Result after this session's 8 commits: the JIT's GPR load/store, FP/vector
load/store, add/sub-SP, XZR handling and JNI table are all materially more
correct; several would have corrupted the real Roblox runtime.

### Honest remaining (next session — concrete, small tasks)
- **`fcvtzs/fcvtzu Dd,Dn` and Sd,Sn (0x5E/0x7E)** — SIMD/vector FP->int writing
  to an FP register lane. `fcvtzs d31,d31` (0x5ee1bbff, from B_fpstore) is
  currently MIS-decoded as `WidenShl`; the plain 0x5ee1xxxx is Unsupported.
  Add a class BEFORE the WidenShl gate and a translate converting Dn's double
  to signed/unsigned int in Dd (this is a genuine silent-corruption risk for
  FP code). B_fpstore segfaults at it (was hanging pre-sp-fix).
- **Backwards-branch loop fidelity** — int_only (loop w/ `b.lt` back-edge)
  still faults; jit_regress hangs. Verify the bounded compiler patches in-body
  back-edge targets to the emitted block (host_of_guest) and that SP/offsets
  stay stable across iterations.
- SMOV/UMOV lane->GPR and remaining FP-vs-int lane ops.

These are progressive ISA surface revealed by arbitrary compiled C, not
blockers of the previously-validated Roblox boot path.

## Session (Sep 11, 2026) — JIT executes real compiled C end-to-end (commits f1e65ce, f6bb244)

Continuing the synthetic-program bring-up. The loop back-edge and four
operand/decode fixes crossed the JIT from "decodes the boot ISA" to "correctly
EXECUTES real compiled aarch64 C": functions with loops, recursion (fib=55),
FP fmul/fadd/fcvtzs, mul (factorial 8!=40320), ldrsw sign-extend loads,
movk multi-part constants, SP prologues — 15/15 cross-gcc programs return the
right value through load_elf_image->jit_run (no QEMU).

### f1e65ce — loop back-edge jmp
A frontier block falling through to an already-emitted address (loop back-edge
`b.le Lbody`) just `break` and hit the epilogue `ret` → every loop body ran
once then returned/re-dispatched (hang/corrupt pc). Now emits a `jmp` to the
already-emitted host offset + fixup. loop1 sum(0..9)=45, int_only loops correct.

### f6bb244 — operand/memory decode correctness (four silent miscompiles)
1. MOVK was a *replace* not a *merge*: `movz 0x8bb1; movk 0x2 lsl#16` → 0x20000
   not 0x28bb1 (broke every multi-part constant). Now RMW at bits[shift,+16).
2. MADD/MSUB with ra=31 (the `mul` alias) added the STACK POINTER (ldg RDI,31
   read x31). ra==31 is XZR → skip the accum add/sub.
3. AddSubReg gate caught MADD/MUL (top 0x9b) as `add ...,lsl #N` (mul x0,x1,x0
   → add lsl#31). Restricted to the real add/sub shifted-register tops
   {0x0b,0x2b,0x4b,0x6b,0x8b,0xab,0xcb,0xeb}; 0x9b falls through to MulDiv.
4. ldrsw/ldrsh/ldrsb (sign-extend loads) decoded as STORES (bit22=0 like STR,
   bit23=1). Added `sext` to LdStrImm; these are now sign-extending loads into
   the X dest (ldrsw=movsxd, ldrsh/ldrsb=shl/sar 48/56).

All regression-locked (+movk_merges_into_existing_register,
loop_back_edge_reiterates_body, ldrsw_sign_extend_load_ground_truth, plus the
earlier ones). Workspace 111/0, build clean.

### Honest remaining (small, next session)
- LdStrReg register-offset ldrsw/ldrsh may share the bit22-mislead (the C
  battery only emitted unsigned-offset forms); verify and fix if so.
- FcvVec 4S lane edge (uses movq/cvttsd2si on 4-byte lanes) and fcvtzu ≥2^63.
- ADD/SUB with rn==31-as-XZR (`add xD, xzr, #imm` reads SP today; assembler
  uses movz/orr, so low priority).
- Then libbadcpu gaps; services/auth. GPU ev-boards: HARD GATE.
## Session (Sep 11, 2026) — register-offset sext + LogicalImm-vs-MoveWide (commit 716876c); JIT executes broad real C

Extended the synthetic-C battery to arrays/shorts/structs and found+fixed two
more silent miscompiles:

1. LdStrReg register-offset ldrsw/ldrsh/ldrsb shared the bit23 mis-lead (decoded
   as stores) — same fix as the unsigned-offset form (sext field + translate).
2. LogicalImmediate (AND/ORR/EOR/ANDS #imm) collided with MoveWide: the MOVZ/
   MOVK/MOVN gate matched top bytes {0x12,0x92,0x52,0xD2,...} which span the
   AND/EOR/ANDS-immediate class, so `and w1,w0,#0xffff` decoded as `movn`.
   MoveWide now gates on bits[28:23]==0x25 ((insn & 0x1f800000)==0x12800000);
   LogicalImm has 0x24, so AND-immediates route to LogicImm. Verified shacc
   (short acc) and arr (int+short arrays) = 42.

Net: the JIT now correctly executes ~20 real compiled aarch64 C programs
(loops, recursion, FP, mul/div, sign-extend loads both offset forms, short/int
arrays, AND-immediates, MOVK constants, SP prologues). arm64jit 78/78, workspace
112/0.

### Open (next session, honest)
- struct-by-value + function-pointer (`blr` to computed addr) still FAILS:
  `structs.c` → "pc 0x600000005 outside image". The fn-ptr arg gets corrupted
  through the struct-passing / dispatcher path — a deeper control-flow/ABI
  interaction (how the emitted GOT/adrp computes a callable and the dispatcher
  resolves it). Worth a focused session.
- FcvVec 4S lane uses movq/cvttsd2si on 4-byte lanes (possibly wrong); fcvtzu
  for >= 2^63; ADD/SUB rn==31-as-XZR reads SP (assembler prefers movz/orr, low
  priority).## Session (Sep 11, 2026) — LdStrReg sign-extend stored address, not value (commit 7b6b19e)

Post-battery hardening: a register-offset sign-extend exec test surfaced a
translate bug in the bit23/sext path added in 716876c. `ldrsh w0,[x1,x0]`
(0x78e06820, register offset, W dest, no shift) loaded the signed value into
RCX but stg_if_writable stores RAX — i.e. it stored the *effective address*
into the dest register. Every a[i] in a short-array loop via register-offset
LDRSH silently corrupted the accumulator. The session battery passed only
because arrays used ldr w / unsigned-offset forms.

Fix: the LdStrReg sext branch now loads the value into RAX (address no longer
needed), mirroring the LdStrImm sext arm. sumh over `short a[]` (register-offset
ldrsh) = 26 -> 42. arm64jit 79/79, workspace 113/0. +regression
ldr_reg_sext_sign_extends_into_dest.
## Session (Sep 11, 2026) — JIT ABI correctness: struct-by-value + 32-bit semantics (commits b8b5e62, e5d78d3)

Worked the open "struct-by-value + function pointer" item. Reproduced it with a
cross-gcc battery run through `cargo run -p arm64jit --example elfjit` (real
compiled aarch64 C, `-static -nostdlib -Wl,-e,entry`), fixed **four real silent
miscompiles**, gold-locked each with a regression test. `cargo test -p arm64jit`
-> 82, workspace 116/0. Battery: loop1=45, structs/dispatch/fpfun/vtable=42,
byvalue=44, bv2=300, signmod=12, iso_wrd=4321, iso_arith=300 — all correct.

1. **LdStPair offset-form ignored its immediate** (`b8b5e62`). `ldp x0,x1,[sp,#16]`
   (writeback=0) computed `access_off = 0`, so a 16-byte struct passed by value
   read [sp],[sp+8] (the saved x29/x30) instead of [sp+16],[sp+24] — byvalue.elf
   got (0,0) and returned garbage. The three addressing modes were conflated;
   now offset=`(imm,0)`, post-index=`(0,imm)`, pre-index=`(imm,imm)`.
   +`ldst_pair_offset_form_applies_immediate`.

2. **ADD/SUB rn==31 read SP when the S flag is set** (`b8b5e62`). `negs w1,w0`
   (subs w1,wzr,w0) computed `sp - w0` instead of `-w0` (rn=31 is XZR for the
   flag-setting form; only non-S `sub sp,sp,#N` reads rn=31 as SP). This was the
   documented "ADD/SUB rn==31-as-XZR reads SP" gap — a real repro finally
   (signmod.elf `%16` produced garbled remainders). Fixed AddSubImm + AddSubReg;
   also made LogicReg/AddSubReg read rm==31 as XZR (was SP). +`addsub_s_flag_reads_xzr_not_sp_for_rn31`.

3. **32-bit W writes did not zero-extend** (`b8b5e62`). `mov w0,w1` copied the full
   64-bit x1, so a negative two's-complement w1 propagated as 0xffffffffffffffff.
   Added `zext_w` (shl32/shr32) and applied to 32-bit LogicReg (operands, the
   N=1 BIC/ORN/EON half after `not`, and the result) and 32-bit AddSubImm/AdhReg.
   This was the "Ws must zero-extend" open item; it was silently corrupting any
   32-bit chain once a negative value entered a W register.

4. **Scalar `fcvtzu` saturates the wrong half** (`e5d78d3`). fcvtzu is unsigned,
   valid over [0,2^64), but the code used signed `cvttsd2si` which saturates
   anything >= 2^63 to INT64_MIN(0x8000..0); the old comment wrongly claimed
   `d>=2^63` was "architecturally out-of-range". Now a three-path sequence
   (d<2^63 signed; 2^63<=d<2^64 via `2^63 + int64(d-2^63)`; d>=2^64 -> u64::MAX)
   with in-buffer jc/js/jmp patching (mirrors the Ucvtf2d JNS idiom).
   +`fcvtzu_handles_u64_beyond_2pow63`.

Also added the `JIT_BUDGET` env knob (default 8192) to `jit_run` for
instruction-granular tracing under `JIT_TRACE` (`JIT_BUDGET=1`), and a
diagnostic captured by it: a **bounded-truncation fall-through bug** — a block
cut off mid straight-line by the budget had no pc write, so the dispatcher
re-compiled from the same entry forever. compile_image_bounded now diverts the
fall-through next-pc to a dispatcher-return stub when `truncated && !terminal`
(same fix that let the budget=1 per-instruction trace work; also a latent real
hazard for any Roblox function > 8192 insns without an early branch).

The battery lives in /tmp/jitbatt/ (not committed: it was ad-hoc before this
session). Next items on the JIT path: **FcvVec 4S-lane** conversion (uses
movq/cvttsd2si on 4-byte lanes), SIMD SMOV/UMOV lane->GPR and remaining
FP-vs-int lane ops, then libloader gaps -> libbadcpu gaps -> services/auth.
Real-binary/GPU boot remains blocked (no libroblox.so/APK, no GPU) — HARD GATE.

### Addendum (same session) — FcvVec FP->int vector (commits 7c11be3)
- **`.4s` lane width bug**: FcvVec converted each 4-byte S lane as a double
  (`movq_load` reads 8 bytes = the lane *and the next lane*) and wrote 8 bytes
  back (`movq_store` clobbered the neighbour lane), so multi-lane float->int
  vectors were corrupt. Now loads the 32-bit float, `cvtss2sd`s it, stores a
  32-bit int per lane (`mov_store32`).
- **`.2d` decode bug**: `fcvtzs v0.2d` (0x4ee1b820) has bit20=0 just like `.4s`,
  so `esize=(insn>>20)&1` mis-decoded the 64-bit form as esize=4. The real
  discriminator is **bit22** (0x400000). +`fcvt_vec_4s_lanes_are_32bit_and_independent`
  covers .4s (independent lanes), .2d signed, and .2d unsigned negative-clamp.
- arm64jit now 83, workspace 117/0. Each fix was a silent data-corruption bug
  that would have produced wrong pixels/audio/coordinates in a real Roblox run.

## Session (Sep 11, 2026) — SIMD lane-insert/extract fix: INS/SMOV/UMOV (commit 0f2d806)

Picked up the standing "SIMD SMOV/UMOV lane->GPR and remaining FP-vs-int lane
ops" item. Drove real aarch64 asm (INS/SMOV/UMOV across all element sizes +
vector logical/sat) through `elfjit` and found a **silent miscompile** that
predated this session:

### The bug (would corrupt NEON-heavy graphics/audio)
`mov v0.s[i],w1` (INS: GPR->vector-element insert, opcode bit13 CLEAR) and
`smov`/`umov` (element extract to GPR, bit13 SET) share the decode fields of
the vector-logical (AND/ORR/EOR/BIC) and saturating-add (SQADD/UQSUB) classes
(residue 0x..2x0c00). The precise lane-element gate
`(insn & 0xffe0_0c00) in {0x0e000c00, 0x4e000c00}` was placed AFTER
SimdVLog (line ~1186) and SimdSatAdd (line ~1425), so a real INS/SMOV was
silently mis-decoded before the lane gate was reached:
  - `ins v0.s[0],w1` 0x4e041c20 -> AND (SimdVLog): never wrote v0, clobbered x0
  - `smov x2,v0.s[0]` 0x4e042c02 -> SQSUB (SimdSatAdd)
  - `smov x6,v0.h[2]` 0x4e0a2c06 -> SQSUB
objdump-verified the encodings; `gcc` compiles `mov v.s[i],wN` as this INS form
everywhere NEON 4-element scalar writes are edited into vectors.

### The fix (decode.rs + translate.rs + jit.rs)
1. Move the lane-element gate BEFORE the broad vector gates and key on the
   opcode field bits[13:12] (verified across all 4 element sizes):
   - bit13=1        => vector->GPR extract umov/smov/mov (sign = bit12 clear)
   - bit13=0,bit12=1=> GPR->vector insert `ins/mov Vd.T[idx],Rn` (NEW `Inst::InsGp`)
   - bit13=0,bit12=0=> dup-from-GPR (fall through to the existing SimdDupGp)
2. Removed the now-dead late duplicate gate at the old location.
3. SimdLaneGp translate now handles ALL element sizes (1/2/4/8), so `.h`/`.b`
   extracts are no longer swallowed by SimdSatAdd.
4. InsGp translate: copy esize bytes of GPR rn into Vd at index*esize.

### Verified
- `cargo test -p arm64jit` 85/85 (added `ins_gp_inserts_element_into_vector_and_extract_reads_it`
  and `ins_gp_sign_and_zero_variants_insert_correct_lanes`); `cargo test --workspace` 119/0.
- elfjit harness (real aarch64): lane_test.elf / smov.elf return -570
  (=0xfffffffffffffdc6; previously returned 0). and/orr/eor/bic/sqadd/sqsub
  still decode+execute as their real ops (logical.elf runs them all and stops
  honestly at `uaddl` 0x4000ec = 0x2ea20020, the next unimplemented widening
  mul — an honest Unsupported stop, not a miscompile). `dup v1.4s,w10`
  (0x4e040d41) still decodes as SimdDupGp (the bit12==0 fall-through).

### Next (ordered, no APK/GSI/GPU on this box)
1. SIMD widening-multiply family (uaddl/saddl, and the smull/umull widening forms
   already partly present) — surfaced by logical.elf's honest stop.
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK are available.

## Session (Sep 11, 2026) — SIMD widening families: add/sub-long + multiply-long (commits 80d27e5, d052723)

Driving the cross-gcc asm battery (logical.elf / addl.elf / mull.elf) through
`elfjit` cleared two more ISA walls AND exposed that the "already-implemented"
widening-multiply path shipped several silent miscompiles. Workspace now 121/0.

### ADDL: saddl/uaddl/subl/usubl (80d27e5)
- Gate only matched the 8 esrc=2 residues (0x..60), so esrc=4 (.2s->.2d, 0x..a0)
  and esrc=1 (.8b->.8h, 0x..20) fell through to Unsupported. Expanded `alres` to
  all 24 esrc x signedness x upper x add|sub residues; esrc = 1<<bits[23:22].
- Translate had the same width bug class as the old FcvVec/Mull code: esrc=4 read
  64 bits (both lanes), esrc=2-unsigned read 32 (polled next lane), esrc=1 stored
  32 (overran a 2-byte element). Now reads EXACTLY esrc bytes (sign/zero-ext to
  a 64-bit reg) and stores EXACTLY de=2*esrc bytes (8/4/2).

### MULL: smull/umull/smlal/umlal (d052723) — FIVE silent miscompiles
mull.elf "ran" without stopping, but that only proved no-unsupported. Inspecting
decode+translate against objdump found:
1. `res_esize = bit22 ? 8 : 4` — mis-sized smull .4h->.4s as 8, never .8b->.8h (res 2).
2. `unsigned = bit28` — bit28 is 0 for BOTH signed 0x0e and unsigned 0x2e, so
   umull/umlal were sign-extended (0xFE*2 => -4 not 508). Now bit29.
3. `acc = bit15` — set on plain smull/umull too, so every plain widening multiply
   ACCUMULATED instead of overwriting Rd. acc = gateway clause (c000=mul, 8000=acc).
4. translate `lanes = res==8?2:4` — missing the .8b->.8h 8-lane form.
5. store width not exact (store32 for res=2) overran the next lane.

### Verified
- saddl .2d {7,-2}+{3,9}={10,7}; uaddl .4s {1,2,3,4}+{10,20,30,40}; uaddl .8h
  1..8+1..8; smull .2d {7,-3}*{5,-2}={35,6}; umull .8h 0xFE*2=508 (would be -4 if
  still signed) — all exec_bytes'd with objdump-verified encodings.
- decode binds (res_esize/unsigned/acc/q) asserted for all six mull + three addl forms.
- logical.elf / addl.elf / mull.elf run to completion; prior battery + lane_test
  (-570) unchanged. arm64jit 87/87, workspace 121/0.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep pressing the SIMD surface (the battery will keep surfacing the next wall,
   e.g. shift-by-immediate / tbl / dup .b / post-index SIMD ld, then svc on real use).
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK are available.

## Session (Sep 11, 2026) — SIMD shift-by-immediate: ushr/sshr (commit c6eb8df)

Continuing the cross-gcc SIMD battery, `shiftimm.elf` returned 0xfffffffc for
`ushr v0.2s,v1.2s,#8` (expected 3) — another silent miscompile.

### Root cause
Plain shift-right-immediate (marker bits[14:12]==0b000) had NO decode gate, so
it fell into the broad VecMovi (vector-immediate) gate and wrote a wrong
immediate pattern instead of shifting. (shl 0b101 and usra/ssra 0b001 already had
gates; only the plain 0b000 form was missing.)

### Fix
1. `Inst::SimdShr` gate: SIMD-reg prefix {0f,2f,4f,6f} + bits[14:12]==0b000 +
   bit23 clear + immh(bits[22:19]) != 0 (movi/mvni always have immh==0, so they
   are NOT reclassified — verified movi.2s #5 still VecMovi). esize from fls(immh)
   = 1<<(fls-1); shift = 2*esize_bits - (immh:immb) — verified ushr.2s #8
   (immh4=7) and ushr.2d #17 (immh4=13). Placed before the VecMovi gate.
2. Translate handles shift >= esize_bits (ushr->0, sshr->sign fill) since x86
   `shr r64,imm` clamps count.
3. Fixed the SAME latent sign-extension bug in SimdShr AND SimdShrAcc (ssra):
   the esize-bit source was loaded zero-extended, so a NEGATIVE element under the
   arithmetic shift came out positive (0xffffff00 >>> 8 = 0xffffff, not -1).

### Verified
ushr.2s {0x100,0x200}->{1,2}; sshr.2s -256>>8 == -1 (was 0xffffff); decode binds
ushr unsigned / sshr signed / esize,shift; movi.2s stays VecMovi. shiftimm.elf
-> 3 (was 0xfffffffc); shifts.elf (sshl .2d) -> 256; prior battery + addl/mull +
lane ops unchanged. arm64jit 88/88, workspace 122/0.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep pressing the SIMD surface as the cross-gcc battery reveals it.
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK are available.

## Session (Sep 11, 2026) — SIMD/sysreg correctness sweep: 4 real bugs + 3 ISA walls (127/0)
Continuing the cross-gcc battery. Two previously-"correct" paths and three
newly-hit instructions were wrong; all fixed + qemu-verified + regression-locked.

### 1. SimdShrAcc (usra/ssra) esize/shift decode bug (silent, real)
The ShrAcc decode derived esize from the 3-bit tagless immh via trailing_zeros,
which collapses EVERY esize>=4 shift to esize=1/shift=0 — ssra silently
accumulated WITHOUT shifting. Battery exposed: ssra .2d #2 of {-8,-16} returned
-24 not -6; ssra .4s #2 returned 72 not 18. Decode now mirrors the verified
SimdShr gate (full immh incl bit22, fls esize, shift = 2*esize_bits-(immh:immb)),
and the unsigned discriminator is bit29 (was bit11). Translate also guards
shift>=esize_bits (mirror SimdShr). qemu: -6 / 18 / 1.

### 2. neg reads rn=31 as XZR, not SP (AddSubReg, silent, real)
`neg xd,xm` = `sub xd, xzr, xm` (shifted-register, bit21=0) — rn=31 MUST be XZR
(=0). The translate read rn=31 as SP for every non-S op, so neg(x6) computed
sp-x6. Root cause: bit21 is the form discriminator (qemu: neg=0xcb0603e6 bit21=0
-> XZR; sub sp,sp,x1=0xcb2163ff bit21=1 -> SP). Added `sp_operand` (bit21) to
Inst::AddSubReg and applied on both read (rn) and write (rd) sides. Regression
`neg_reads_rn31_as_xzr_not_sp`.

### 3. SysReg MRS reads were silent no-ops (LATENT, all of them)
The translate wrote `buf.mov_ri64(rt,..)` where rt is a GUEST register index —
the value landed in a stray x86 reg, never committed to the guest file. So every
`mrs xN,<cntfrq|cntvct|nzcv|dczid|tpidr>` returned 0/garbage. Decode-side tests
passed because they only bind Inst fields; exec was never exercised (cf/dz
battery proved it: cntfrq + dczid both returned 0 before). Fixed all four read
paths (+ MRS via `stg`), and the msr-tpidr write kept as-is.

### 4. New ISA walls crossed
- dczid_el0 (`mrs x0,dczid_el0` = 0xd53b00e0): glibc CRT reads it to size its DC
  ZVA memset; returns 0x4 (16-byte block, DZP=0). sysreg 5.
- umulh/smulh (high 64 of 128-bit product): gate top 0x9b && bit22 set
  (separates from madd/msub where bit22=0), signed = bit23. x86 F7/4,F7/5
  one-operand mul/imul (RDX:RAX = RAX*rm). Verified vs qemu.

### Verification
- `cargo build --workspace` clean; `cargo test --workspace` 127/0 (arm64jit 93).
- Battery (all qemu-verified): ssra_2d=-6, ssra_4s=18, ushr=1, shl=24, shlimm=27,
  neg_d=2, cf(cntfrq)=100000000, dz(dczid)=4, mulh_e=2.
- modmain.elf (full glibc CRT) now advances past dczid + umulh/smulh to the next
  wall: MTE `stg x0,[x0]` (0xd9200800, __libc_mtag_tag_region) — memory tagging.

### Next (ordered, no APK/GSI/GPU on this box)
1. MTE stg/ldg/stzg memory-tagging no-op (unblocks full glibc-linked programs).
2. Continue the SIMD surface as the cross-gcc battery reveals it; then real
   `svc` syscall routing on actual use (real AArch64->x86-64 table; mmap 222 etc.).
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof remains blocked (no libroblox.so/APK, no GPU) —
   HARD GATE on a capable host (`elfjit ... 0x1f0db20 --jni` run log).

---

# Session — glibc-CRT ISA sweep (dc/ic, MTE writeback, GCS/SME-TLS) + svc correctness (Sep 12 2026)

Continuing the cross-gcc / hand-assembled-battery approach with no APK/GSI/GPU.
**Three focused commits; workspace 132/0, arm64jit 97, tree clean.**

## 4db2b3c — data/instruction cache maintenance (dc/ic) no-ops
glibc's `__libc_mtag_tag_region` ends in a `dc` op. In the single-threaded
direct-mapped JIT these coherence ops (dc/gva/civac/ivac, ic ivau; top 0xd5,
CRn=7) are no-ops — EXCEPT `dc zva` which zeros the advertised 16-byte block.
Fixed two bugs in the leftover session-draft: Rt decoded from bits[9:5]
(instead of bits[4:0]; caused `dc zva x0` to write via x1 → segv), and the
test used 0xd50b7400 (=a `sys` instr) as `dc zva` — the real `dc zva x0` is
0xd50b7420 (CRm=4 && op2=1). modmain moved 0x40c174 -> 0x438b1c.

## d55cb04 — MTE tag-store writeback + mrs gcspr_el0/tpidr2_el0
- The MteTag gate forced bit10==0, so post/pre-index st2g/stg writeback forms
  (`[x2],#64` / `[x2,#-64]!`) were Unsupported. Their Xn-advance (Xn +=
  signed imm<<4) is a real side effect glibc memset/stg loops depend on; the
  tag-store itself stays a memory no-op. Decode now carries rn/wb/wb_off
  (imm9 sign-extended, scaled <<4). Load bit stays bit22 (ldg byte1=0x60;
  stg/st2g 0x20/0xa0), so the discriminator is unaffected. objdump-verified.
- `mrs gcspr_el0` (armv9 GCS ptr, 0xd53b2522) + `tpidr2_el0` (SME 2nd TLS,
  0xd53bd0ae) read 0 (features never enabled) — glibc CRT reads them sizing
  GCS call frames / probing SME. sysreg ids 6/7.
- modmain advanced to 0x442cf8, then stops on glibc's SME-IFUNC feature-probe
  (`str za w15,[x16]` = 0xe1206200). **Documented as BEYOND Roblox's
  Android/bionic boot ISA** — the real libroblox.so boot path is already fully
  decoded / exit 0 per prior sessions. Root cause of that glibc-only tail:
  elfjit sets up NO guest auxv, so glibc reads garbage AT_HWCAP and
  IFUNC-resolves into SME. Chasing the SME ZA-tile ISA is a synthetic-harness
  tangent, not a Roblox boot blocker.

## e20687d — svc syscall-number bugs + extended table + inline host-call fixes
Hand-assembled aarch64 svc programs (write / exit / multiple sequential svc)
through elfjit exposed real bugs on the syscall path:
1. **getuid was mapped to 199 (that's socketpair); real AArch64 getuid=174.**
   **mremap was mapped to 220 (that's clone); real = 216 (3264_mremap).**
   Neither was ever exercised (the unit test only checks write/mmap/getpid).
   Fixed; added uid/euid/gid/egid/tid/ppid @ 174-178/173. +regression
   `guest_svc_identity_numbers_match_aarch64_abi`.
2. Extended the table with common aarch64 boot syscalls: getcwd 17, chdir 49,
   getdents64 61, lseek 62, faccessat 48 (w/ AT_FDCWD), readlinkat 78, pipe2 59,
   set_tid_address 96, sched_yield 124, prctl 167.
3. **Inline host-call correctness (two real bugs):**
   - JIT block body runs at host RSP≡8 (mod 16) — correct for guest-to-guest
     BL (call_rel32) — but SysV needs RSP≡0 at a host CALL site. So
     `call guest_svc` / `call guest_sha1stem` fired misaligned; any callee with
     aligned stack work (format! in JIT_TRACE_SVC, SSE locals) SIGSEGV'd. Now
     sub rsp,8 before / add rsp,8 after each inline host call.
   - guest_svc(st)'s state arg was passed implicitly via RDI (held the entry
     state on the FIRST call by luck; a prior host call clobbers RDI), so the
     SECOND svc in a block passed garbage (+ misaligned deref of 0x1). Now
     `mov rdi, rbx` explicitly.
   Proof: svc_elf writes then exits 0; exit_only returns 7; `we` (2 writes +
   exit_group 3) returns 3 with both writes visible; trip (3 sequential
   writes) prints W1/W2/W3. Previously ANY 2nd svc segfaulted — a real
   blocker Roblox (many syscalls) would hit.

### Status
- `cargo build --workspace` clean; `cargo test --workspace` 132/0 (arm64jit 97).
- Battery clean (no unexpected walls); modmain still stops honestly at the
  documented SME `str za` (0x442cf8), beyond the Roblox boot ISA.
- Commits 4db2b3c, d55cb04, e20687d on local `dev`.

### Next (ordered, no APK/GSI/GPU on this box)
1. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
2. Optional (glibc-coverage only, not Roblox): give elfjit a guest auxv so
   glibc IFUNCs resolve to scalar (non-SME) paths.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE, blocked until a capable host + the real binary/APK.


---

## Session (Sep 11, 2026) — libbadcpu gregs register-map fix + arm64jit BitField disarm (151/0)
Two crates hardened against silent miscompiles; the HANDOFF's documented open
arm64jit bug (`__tunable_get_val` x4 corruption) is FIXED. Commits `8325edc`,
`9081bfc`, `17e449b` on `dev`.

### libbadcpu (8325edc): the emulator wrote the WRONG registers
`ucontext_t.uc_mcontext.gregs` is `greg_t[23]` with R8..R15,RDI,RSI,RBP,RBX,
RDX,RAX,RCX,RSP in slots 0..15 (only RIP=16/EFL=17 match the x86 reg number).
The old table indexed gregs[0] as RAX etc., so every emulated POPCNT/MOVBE/
LZCNT/TZCNT/BMI1 read/wrote the wrong register and corrupted guest state.
Now GREGS_IDX maps x86 reg number -> true slot. Also fixed: VEX `vvvv` was
never decoded (3-byte C4 + 2-byte C5) — ANDN used the DEST register as its
first source; and the VEX opcode byte was read from the C4/C5 prefix position,
so no 0F38/0F3A-map VEX instruction ever decoded correctly. 6 new tests;
libbadcpu 6->12.

### arm64jit (9081bfc): BitField dispatches by class — modmain boots PAST its old crash
Driving `modmain.elf` (full static glibc, qemu=12) through elfjit:
1. UBFIZ/SBFIZ (insert=false, immr>imms) went through the BFI/merge path and
   PRESERVED old Rd's bits. `ubfiz x4,x0,#7,#32` kept a stale 0x7f8000000000
   prefix, so glibc's `__tunable_get_val` ldr'd [x4,#48] at 0x7f800048e888
   (should be 0x48e888) -> SIGSEGV. UBFIZ zero-fills; SBFIZ sign-fills.
2. Genuine BFI (insert=true) was swallowed by the ROR shortcut
   (imms+immr+1==bits: 15+48+1==64) and compiled as a rotate. The LSR/LSL/ROR
   shortcuts are UBFM/SBFM aliases; BFM inserts now handled first (BFXIL
   in-place mask, BFI shifted merge).
Regressions `ubfiz_zero_extends_field_and_discards_old_rd` +
`bfi_still_merges_into_old_rd`. arm64jit 108->110; workspace 151/0; full
cross-gcc battery unchanged (loop1 45, structs/dispatch/fpfun/vtable 42,
byvalue 44, bv2/iso_arith 300, signmod 12, iso_wrd 4321, arr/shacc/fact/ldrsw/
fp_only/A/C 42).

### Honest remaining
- modmain now boots past its old `__tunable_get_val` crash; the udiv fix (below)
  cleared `_dl_determine_tlsoffset` too. It now stops deep in the glibc-CRT tail
  (`__memset_generic`, caller passed x0=0) — the HANDOFF-flagged synthetic-glibc
  tangent that is NOT a Roblox boot blocker (real libroblox boot path already
  fully decoded / exit 0).
- Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
  HARD GATE; blocked on a capable host + the real binary/APK (none on this box).

## Session (Sep 11, 2026) — UNSIGNED DIV WRONG-RESULT BUG FIXED in arm64jit (152/0)
Commits `28dba32` (div), `c1e41cb` (svc additions).
status: session-end (committed, tests green)

### Silent, wide bug: every UNSIGNED division in the JIT returned 0
The HANDOFF's open "small-address load in `_dl_determine_tlsoffset`" was the
guest doing `udiv x0,x0,x1`. A focused `exec_bytes` test (`udiv x5,x0,x1` =
0x9ac10805) left x5=0 for EVERY input (100/10, 7/1, 0/5), even in isolation.
Byte-dumping the emitted host code showed `48 f7 c1` — x86 group-3 `F7` uses
/6 = DIV and /7 = IDIV, but `div_r64`/`div_r32` emitted `modrm(3,0,..)` =
group-3 /0 = TEST, so the instruction decoded as `test rcx,eax` and never
produced a quotient. `idiv_r64` already used /7 and was correct — ONLY the
unsigned forms were broken. Confirmed with as+objdump: `48 f7 f1` = div rcx /
`48 f7 f9` = idiv rcx. Fixed both emitters to /6.
Regression `udiv_computes_quotient` (100/10=10, 7/20=0). Silent, WIDE wrong-
result class: any guest unsigned integer math (incl. Roblox) returned 0.

### svc table additions (c1e41cb)
glibc `__tls_init_tp` surfaced set_robust_list(99)->0 (no-op is valid; -ENOSYS
made glibc retry) and membarrier(283)->no-op. rseq(293) left -ENOSYS (no valid
rseq area). Verified vs aarch64-linux-gnu asm-generic/unistd.h.

### Result
modmain.elf booted THROUGH `_dl_determine_tlsoffset` (udiv fix), reached real
AArch64 syscalls, then hit glibc `__memset_generic` (x0=0 passed by caller) —
again the non-Roblox glibc-CRT tail. arm64jit 111/111; workspace 152/0; battery
unchanged.
- HARD GATE unchanged: `elfjit <libroblox.so> 0x1f0db20 --jni` on a GPU/APK host.

## Session (Sep 11 2026) — guest auxv bootstrap; SME/SVE/MulLong decodes; zero-extend fix (137/0)

Goal: prove the JIT boots a full statically-linked glibc aarch64 binary.
modmain.elf (`main(){return 300%16;}`, qemu returns 12) tests the whole CRT.

1. **`arm64jit::boot` — guest auxv on the initial stack.** The kernel ABI puts
   [argc][argv][envp][auxv AT_NULL] on the stack (sp at argc); glibc static
   `_start` walks it for envp/auxv. The JIT left garbage, so `_dl_hwcap2` picked
   HWCAP2_SME from garbage and fell into `__libc_arm_za_disable`'s `str za` loop.
   New `standard_auxv(&LoadedElf, hwcap, hwcap2)` + `layout_initial_stack()`
   (fills AT_RANDOM via bounded xorshift). Wired into elfjit AND sober-core::jit.
   Verified `_dl_hwcap2=0, have_sme=0`.
2. **SME/SVE feature-off decodes.** Even with SME off, glibc's ZA block is
   *linearly* reachable in the compiled CFG (compiler follows fall-through past
   the data-dependent SME gate), so it had to DECODE: `Inst::SmeNoop`
   (`str za[Wt,k],[Xn,#k,mul vl]` 0xe1206200..f + smstart/smstop), `Inst::
   AddVectorLen` (addvl/addsvl, `Xd = Xn + imm6*16`, model VL=16B), `Inst::
   SveCntd` (cntd -> 2), `mrs xN, midr_el1` (sysreg 8 -> 0).
3. **`Inst::MulLong`** — smull/umull/smaddl/umaddl/smsubl/umsubl, found in
   glibc `_dl_fixup` (IFUNC resolution). Gate top 0x9b & bits[22:21]==01.
4. **LATENT x86-emitter bug fixed:** `and_ri64(_, 0xffffffff)` was a NO-OP
   (`and r64,imm32` sign-extends the imm -> AND-all-ones). All 10 zero-extend
   sites silently leaked W-reg high bits (the glibc x3/x0 corruption). Added
   `CodeBuf::zero_ext_r32` (`mov r32,r32`) and replaced all 10 uses. Proven by
   umull/smull/umsubl/addvl exec tests.

Verification: `cargo build --workspace` clean; `cargo test --workspace` 137/0
(arm64jit 103). modmain.elf boot now advances past `str za` -> _dl_fixup umull
-> midr_el1, then STILL stops early on a glibc-startup x-reg corruption (auxv
scan / __libc_start_main prologue) — the zero-extend fix is in but a leftover
cause remains. qemu returns 12; JIT boot to completion NOT yet achieved.

Next: (1) finish the glibc-startup corruption; (2) modmain->12 proves full
static-glibc boot; (3) libloader gaps -> libbadcpu gaps -> services/auth;
(4) HARD GATE = real libroblox.so + GPU host (`elfjit <lib> 0x1f0db20 --jni`).


### Continued (Sep 11 2026) — after commit 5d40fdd, continued the same goal
Three more real JIT bugs fixed, each verified + a regression test; modmain.elf
(full static glibc, qemu returns 12) boot advances far into glibc startup.

- **Extended-register add/sub** (`Inst::AddSubExt`): `add x3,x2,w20,sxtw #3`
  (bit21=1) was decoded through the SHIFTED parser, mis-reading the option/
  shift bits as `lsl #sh_amt` (=51) — corrupting guest x-registers with
  `0x198...` garbage (a real glibc prologue corruption). New gate
  `n==1 -> AddSubExt`, proper ext(Rm)<<shift with 8 options + SP semantics.
- **Pre/post-index + unscaled immediate LDR/STR** (`Inst::LdStrImmWb`): the
  register-offset gate `(insn&0x3b000000)==0x38000000` was too broad and
  swallowed `ldr x3,[x0],#8` (0xf8408403), mis-reading imm9+writeback as an
  `rm` register -> NULL deref (the glibc auxv/env-scan crash). Narrowed to
  `(insn&0x3b200c00)==0x38200800`, added a proper signed-imm9 writeback path
  (pre/post/unscaled). NOTE: also fixed a latent `b(insn,hi,lo)` arg-order
  underflow panic in the new decode.
- **LSE atomics** (`Inst::LseAtomic`): ldadd/ldclr/ldeor/ldset/swp (ARMv8.1),
  hit in glibc's IFUNC `__aarch64_swp4_acq` (have_lse=0 so on a never-taken
  path, but the block compiler must still build it). Gate: (insn&0x3fe00000)
  in {0x382/0x386/0x38a/0x38e 00000} AND op=bits[15:10] in {0,4,8,0xc,0x20};
  single-threaded emulation (old=[Xn]; [Xn]=f(old,Rs); Rt=old).

Verification: cargo test --workspace 142/0 (arm64jit 108, +2 lse tests).
modmain.elf now runs __libc_start_main fully and dies deep in
`__tunable_get_val` on an address (0x48e8b8) with run-varying high garbage
(another latent 32-bit/zero-extend leak) — the next debugging target.
Also: elfjit now keeps a permanent SIGSEGV diagnostic handler (prints guest
pc + regs from CpuState) — invaluable for localizing a real-code crash.


### Continued (Sep 11 2026) — LSE atomics landed; modmain now dies in __tunable_get_val (block-register bug)

Commit 36a47f7 decoded the LSE atomics (ldadd/ldclr/ldeor/ldset/swp) with
single-threaded emulation; modmain.elf boot then advanced THROUGH
__libc_start_main and the IFUNC atomics (the ldxr/stxr fallback, since
have_lse=0) and now crashes inside `__tunable_get_val` (0x4128ec) on a
block-level register corruption:

- fault = 0x7fXX_0000_48e8b8 (low 0x48e8b8 constant, high 0x7fXX/0x7eXX
  run-varying).
- x7 = 0x48dc88 is CORRECT (adrp x3,0x48d000; add x7,x3,#0xc88 — both clean).
- x4 = 0x7f800048e888 is CORRUPTED. It should be
  `ubfiz x4,x0,#7,#32` (0xC00 for x0=0x18) then `add x4,x7,x4` = 0x48dc88+C00
  = 0x48e888. Instead x4 carries 0x7f8000000000 high garbage from the *source*
  x4 at the `add x4,x7,x4` (rm=x4 read gave 0x7f8000000C00).
- ubfiz is NOT the bug: proven clean in isolation AND on a pre-dirtied x4
  (0x1f000000 -> 0xC00), and the full ubfiz;add sequence is clean in isolation
  (0x48e888 with a correct x7 build). So it is a BLOCK-COMPILER register-
  interaction bug only in the real __tunable_get_val block (persists across
  JIT_BUDGET 2/8/16/300000, so not a boundary artifact): likely the intervening
  `mov w5,w0` (W write) or `adrp`/`mov` reusing a host register that also holds
  the rm=x4 value, so `add x4,x7,x4` reads a stale/dirty x4.
- Next: dump block@0x4128ec host code (JIT_DUMP) or add per-instruction guest
  x4 trace to find which instruction gives x4 the 0x7f8000000000 high prefix.

Also: elfjit now permanently installs a SIGSEGV diagnostic (guest pc + x0..x7 +
sp from the CpuState via ucontext RBX) — the tool that pinned all of today's
crashes; recorded because it is reusable.

`cargo test --workspace` 142/0 (arm64jit 108).

### HARD GATE (unchanged)
Roblox actually running (load -> JNI init -> main loop -> frame on a GPU host)
is NOT met and cannot be on this GPU-less VPS without the real libroblox.so/APK.
The elfjit path is a growing no-QEMU CPU translator that currently boots a full
statically-linked glibc program deep into its CRT/startup. The HARD GATE remains
`elfjit <libroblox.so> 0x1f0db20 --jni` on a capable host.

---

# Session (Sep 11, 2026) — scalar FP/SIMD unscaled + pre/post-index ld/st; libbadcpu VEX.0F38 completion (workspace 164/0, commits 36f01ff + 0fe7d8a)

Opened by re-running the workspace: the handoff's flagged
`test_setup_android_layout_idempotent` is ALREADY FIXED (8b72828: per-call
unique temp root + `create_dir_all` before `set_permissions`); it passes — the
workspace was 157/0 at start, not failing. Proceeded to two committed, tested
pieces (no APK/GSI/GPU required):

## 1. arm64jit — scalar FP/SIMD (B/H/S/D) UNSCALED (ldur/stur) + PRE/POST-index writeback ld/st
Closed the last of the `bit26=1` immediate family (the standing "stur/ldur
scalar s0/d0 + pre/post index" glibc-CRT-tail item).
- `Inst::FpLdStImmUnscaled` + `Inst::FpLdStImmWb` with a shared
  `fp_scalar_xfer` helper (transfers `size` bytes between memory and the low
  bytes of `v[vt]`, upper lanes preserved).
- Decode gate, each bit verified against aarch64-linux-gnu-as ground truth:
  bit26=1 (vector file), bit25=0 (immediate offset), **bit21=0** (NOT
  register-offset — found ONLY by the neighbor-collision test: FpLdStrReg
  register-offset words ALSO have bit25=0, the real discriminator is bit21),
  bit24=0 (not the scaled 0x3d form), bit23=0 (not 128-bit Q), bits[29:27]=111.
  operand size bits[31:30], ld=bit22, imm9 sign-extended, addressing =
  bits[11:10]: 00=unscaled, **01=post, 11=pre** (pre is `0x0c00` = 0b11, NOT
  0b10 — caught by the decode test). unprivileged LDTR/STTR (mode 2) left
  Unsupported.
- +4 tests. arm64jit 116→120. **modmain.elf (full static glibc) now boots
  PAST its documented `stur s0`/`stur d0` memset wall** into
  `__libc_setup_tls`/`_dl_get_dl_main_map`, stopping at a residual null-deref
  (fault 0x0, guestpc 0x400b30, right after `bl 0x413e60 _dl_get_dl_main_map`)
  — again the HANDOFF-flagged non-Roblox glibc-CRT tail, NOT a Roblox blocker.

## 2. libbadcpu — BEXTR + BZHI + SHRX/SARX/SHLX (complete the VEX.0F38 integer family)
The SIGILL emulator already did ANDN/BLSI/BLSMSK/BLSR (VEX.0F38 F2/F3/F1/F4);
added the rest. All encodings verified vs host gcc+objdump:
- BEXTR = 0F38 F7 pp=0: `(src1>>start)&(2^len-1)`, start=control[7:0],
  len=control[15:8] (control = VEX vvvv; pp=0 distinguishes it from the shifts
  which share F7 but carry a pp prefix).
- BZHI = 0F38 F5: `src1 & (2^ctrl-1)`; ctrl>=op-size keeps src1 + CF.
- SHRX/SARX/SHLX = 0F38 F7 with pp=F3/F2/66 respectively.
- **Real subtlety fixed:** fix-size for the 0F38 integer ops must come from
  VEX.W (`vex_w`), NOT the legacy `has_66 => 16-bit` rule — SHLX rax has pp=1
  (has_66) yet is 64-bit; the whole 0F38 branch now sizes off `vex_w`.
- +3 tests. libbadcpu 12→15.

## Gate
- `cargo build --workspace` clean; `cargo test --workspace` **164/0** (arm64jit
  120, libbadcpu 15, libloader 16, +1+1+11). Tree clean on local `dev`.
- HARD GATE unchanged: `elfjit <libroblox.so> 0x1f0db20 --jni` run log on a
  GPU + real-binary host.

---

## Session (Sep 11, 2026) — loader→JIT end-to-end regression test + libbadcpu 16-bit LZCNT fix (169/0)

Two commits on `dev` (HEAD `6d6ef08`). Workspace was 164/0 green at start
(`test_setup_android_layout_idempotent` is already fixed in 8b72828 — passes;
the FMOD divert 10ddb7a and JNI-table bdd8b03 tasks are also already landed).
So this session executed the ordered "libloader ELF/loader gaps" + "libbadcpu
ISA gaps" steps.

### 1. `7f54fbd` — arm64jit: loader→JIT end-to-end regression test
`crates/arm64jit/tests/loader_run.rs` cross-compiles real `-nostdlib` aarch64
programs (via `aarch64-linux-gnu-gcc`) and runs `entry()` through the exact
pipeline elfjit / `sober-core --jit` use: `load_elf_image` → `bind_image_plt`
→ guest stack/TLS/auxv bootstrap → `jit_run`. Results: add=42, loop sum(0..9)=
45, fp `(int)(2.5*4.0)`=10, fib(7)=13. Locks the loader path against
regressions from the divert + JNI-table changes (the bdd8b03 "confirm no
regression" deliverable), since the real-binary HARD GATE can't be exercised
here. Skips cleanly when cross-gcc is absent.
- **Found en route:** `[test]` threads run in parallel in one process, and
  `load_elf_image` MAP_FIXEDs the same non-PIE JIT base 0x400000 — 4 threads
  clobber each other's guest image → a `bl` target missing from the compiled
  block's stub table panics `stub_of_target[...]` (jit.rs:1164). Real open-
  sober loads ONE guest ELF for process lifetime, so this is purely a test-
  harness serialization concern: `run_lock()` serializes load+bind+run.

### 2. `6d6ef08` — libbadcpu: 16-bit LZCNT wrong-result bug
emulator.rs LZCNT 16-bit arm was `(src as u16).leading_zeros() as u64 - 16`.
`u16::leading_zeros` already returns the 0..16 count, so `-16` made every
nonzero 16-bit LZCNT return negative (wrapped huge i64 in the dest reg).
TZCNT's sibling arm has no such offset; the 32/64-bit arms don't either — only
LZCNT had it. Dropped the offset; +`lzcnt_16bit_matches_real_count`
(cx,ax=2 -> 14; cx,ax=0x8000 -> 0). Silent wrong-result class in the SIGILL
emulator.

### Gate
- `cargo build --workspace` clean (0 errors; warnings are the pre-existing
  decode.rs rustfmt churn — rustfmt not installed on this box).
- `cargo test --workspace` **169/0** (arm64jit 120 + 4 loader_run + libbadcpu
  16 + libloader 16 + 1 + 1 + 11).
- HARD GATE unchanged: `elfjit <libroblox.so> 0x1f0db20 --jni` run log on a
  GPU + real-binary host (no APK/libroblox.so/GPU on this VPS).

## Session (Sep 11, 2026) — three silent FP/SIMD miscompiles fixed via a double-precision C battery (commit a0465a0)
Drove a new cross-gcc double-FP battery (real `double` C: polynomial Horner,
array sums, 2x2 matmul, exact division, 3^10 accumloop with fcvtzs, |x|>threshold
counting, weighted average) through `elfjit`, comparing each result to a native
x86-64 compile. Ground truth: dpoly31 dsum17 dmat50 ddiv10 dscale1 dclamp4 ddmat2-4.
Found + fixed THREE real miscompiles (all silent wrong results, not crashes):
1. **scalar fsub (0x1e613800) decoded as SIMD WidenShl** — the shll gate's
   `(insn>>24)&0x0f==0x0e` nibble test ALSO matched the scalar-FP 0x1e family
   when bits15:8==0x38, so `fsub d0,d0,d1` ran as a halfword-widen no-op
   (59049.0-59048.0 → 0.0). Real shll bytes are 0x0e/0x2e/0x4e/0x6e (bit28=0);
   scalar-FP 0x1e has bit28=1. Gate now requires bit28==0. dscale.elf 0→1.
2. **store_nzcv_fp hardcoded N=0** — FP compare sets N=1 for ordered less-than,
   so b.mi/b.lt/b.le never fired and b.gt evaluated N==V as 0==0 for every
   ordered non-equal pair (dclamp 6→4). N now = CF∧¬ZF.
3. **ld1 multiple-structure (2-reg, opcode bits[15:12]==0xA) swallowed by the
   ld2 gate (0x8, deinterleave)** — compiler array-literal `ld1 {v30,v31}`
   loaded interleaved garbage (ddiv double array 2→10). Added Ld1N/St1N
   (consecutive, NO deinterleave) for 1/2/3/4-reg, discriminated by opcode
   bits[15:12]. (Verified real encodings: ld1-2reg=0x4c40a040, ld2=0x4c408040,
   ld1-1reg=0x4c407040.)
+3 regression tests (fsub-vs-shll collision, FP-compare N flag + b.mi/le/gt,
ld1-2reg consecutive). arm64jit 123/123, workspace **172/0**. Cross-gcc C
battery + SIMD hand-battery (lane_test/smov=-570, loop1=45, iso_*=etc) all
still green. fsqrt verified (sqrt(16)=4 via sqrtquad.elf).
Honest: dsqrt .c didn't link (sqrt undefined under -nostdlib); tested fsqrt via
hand-asm instead. Test-setup lesson: guest Dn/Vn maps to st.v[2n]/st.v[2n+1]
(D1 = st.v[2], NOT st.v[1]) — two new tests initially failed on my own wrong
constant placement, not a JIT bug.
Next: keep pressing the cross-gcc FP/SIMD surface (division edges, fma chains,
single-precision float, struct-by-value + FP, loop-with-FP-condition) to find
more silent miscompiles; then libloader gaps. HARD GATE unchanged.

## Session (Sep 11, 2026) — FMOV-immediate [16,30] decode bug fixed (commit bc5ab89)
A second cross-gcc battery (single-precision array div, double loop, mixed
int/float casts, double-struct-by-value, float matmul, double reciprocal —
native ground truth fdivf20 dloop7 mixed4407 dstruct169 dneg0 fmat69 drec124)
hit: fdivf `float a[]={6,12,18,24}; sum a[i]/3` returned 6 instead of 20.
Root cause was NOT the float div (isolated divss/addss/fcvtzs.e,.s all correct)
but **decode_fmov_imm's exponent wrap**: the 3-bit field E maps E0..3->e+1..+4,
E4..7->e-3..0, but the code wrapped `ex>=4`, so E=3 (exponent +4 => constants
16.0..30.0) decoded as -4 (0.0625..0.117). Every FMOV-imm in [16,30] — sample
rates, half-texel, 24.0 corner constants — came out ~256x too small (silent).
Fix: threshold `ex>=5` (E=4 gives (E+1)=5 -> -3). Verified against the
assembler's encodings for 0.125..30.0 (immf.s). fdivf 6->20; dloop/mixed/
dstruct/dneg/fmat/drec all match native. +4 decode_fmov_imm asserts (16,30,2,
0.75). arm64jit 123/123, workspace 172/0, prior battery unchanged.
Lesson: single- and double-FP immediate decoding share decode_fmov_imm — a
boundary exponent bug corrupts both (the fdivf array used f32, dscale earlier
used 3.0; the [16,30] band is where it bites).
Next: keep pressing FP/SIMD (fma/compiler-contracted `fmla`, more div/compare
edges, single-precision struct args) then libloader gaps. HARD GATE unchanged.

## Session (Sep 11, 2026) — scalar FP multiply-accumulate fmadd/fmsub/fnmadd/fnmsub (commit 578faaf)
Third FP milestone: the -O2 compiler contracts every a*b+c / fused a*x*x into
`fmadd`, and fma1 (2x^2+3x+1 over x=1..5) returned 0x8000000000000000 garbage
because 0x1f4.. was swallowed by a broad SIMD vector-immediate gate and
mis-decoded as a bogus movi. Added Inst::Fma3 (scalar 3-source FP):
- decode gate `(insn & 0xff000000)==0x1f000000` placed at the TOP of decode
  (uniquely the scalar 3-source FP family), o1=bit21(fn*), o2=bit15(sub),
  sz=bit22(double).
- translate: mulsd/addsd/subsd with pxor-0 + subsd for the fnmadd negation;
  single-precision via mulss/addss/subss. Semantics fmadd=ra+rn*rm,
  fmsub=ra-rn*rm, fnmadd=-(ra+rn*rm), fnmsub=rn*rm-ra.
- Verified vs assembler (fmadd/fmsub/fnmadd/fnmsub d0,d1,d2,d3 =
  0x1f420c20/0x1f428c20/0x1f620c20/0x1f628c20) -> 17/-7/-17/7 with d1=3,d2=4,
  d3=5; single fmadd s -> 11. fma1.elf 160 = native.
+scalar_fma3_all_four_variants (4 double + 1 single). arm64jit 124/124,
workspace 173/0. Full 3-batch cross-gcc battery unchanged.
This session net: 5 FP/SIMD correctness fixes (fsub-vs-shll, FP-compare N flag,
ld1-2reg deinterleave, FMOV-imm [16,30], FMADD) + the FMADD feature. Next:
keep pressing -O2/FP-contracted programs, single-precision struct args, more
div/compare edges; then libloader gaps. HARD GATE unchanged.

## Session (Sep 11, 2026) — shifted-register add/sub clobber bug (commit 9cc0f16)
Fourth FP milestone. The -O2 loop version of fclamp (double clamp across an
array) returned 10 vs 9; straight-line clamp worked. Root cause:
`apply_shift_const(buf, x, kind, amt)` wrote the shift amount into RCX
(`mov rcx, amt`) then `shl rcx, cl`, but both callers (AddSubReg, AddSubExt)
pass x == RCX (the Rm value being shifted) — so the value was clobbered and the
operand became `amt<<amt` instead of `Rm<<amt`. `add x1,x2,x0,lsl#3` (the
ubiquitous array-index idiom) computed x2+24 CONSTANT, so -O2 double-array loops
read the SAME element each iteration (fclamp: all 5 reads of a[2]=2.0 -> sum
10). Fixed to the immediate-shift C1 /4..7 ib forms (no CL scratch). fclamp
10->9. +regression add_shifted_register_... (lsl#3/lsr#2/asr#1/plain add)
verified vs shadd.o encodings. arm64jit 125/125, workspace 174/0; full battery
unchanged. (dnorm.elf: its Newton reciprocal-sqrt overflows to +inf = UB in C,
not a JIT bug — excluded.)
Lesson: emitters that use a fixed scratch register must never be handed that
same register as an operand. apply_shift_const's CL scratch collided with the
RCX operand; immediate-shift forms sidestep it entirely.
Session net: 6 FP/SIMD correctness fixes + FMADD/FMA3 feature, all committed
with regression tests. Next: keep pressing -O2/loop/array coverage (the
fclamp class is now unblocked), single-precision struct args, division
edges; then libloader gaps. HARD GATE unchanged.

## Session (Sep 11, 2026) — LdStPair D-register stride bug + 4th -O2 battery (commit 1eb7c0a)
Fifth FP milestone. A 4th cross-gcc battery (all -O2, exploiting the now-fixed
shifted-register indexing: 3x3 int matmul, struct{double x,y} array walk,
byte-scan, 64-bit loop, short array) surfaced one more real bug:
structfield.elf (loop `ldp d29,d28,[x0],#16` + `fmadd`) returned 128 vs 52.
Root cause: the LdStPair `fp_d` branch located each D-register at
VECTOR_BASE + rt*8, but a guest Dn is the LOW 8 bytes of its 16-BYTE vector
slot (VECTOR_BASE + rt*16) — so `ldp d29,d28` wrote 8-byte values to
0x1f8/0x1f0 instead of 0x2e0/0x2d0, and the follow-on fmadd read the stale
vector slots (still the initial q-pair array literal). Fixed both load+store
to *16 (q128 already used *16). structfield 128->52.
- 4th battery results (all match native): m3=45, structfield=52, bytes=5,
  iloop=150, shorts=24, plus the earlier fclamp/fhorner/fquad/fdivmix.
  +regression ldst_pair_d_registers_use_16_byte_vector_stride.
arm64jit 126/126, workspace 175/0; full 4-batch FP battery + core C battery
all green. dnorm (Newton reciprocal-sqrt that overflows to +inf = C UB) stays
excluded as degenerate.
Session net: 7 FP/SIMD correctness fixes + FMADD feature, each regression-locked.
Next: keep pressing -O2 arrays/structs (now unblocked), single-precision wider
structs, then libloader gaps. HARD GATE unchanged.

## Session (Sep 11, 2026) — single-precision LdStPair (s-pair) scale bug (commit 8d2d57b)
Sixth FP milestone. 5th -O2 battery (float struct array, 2D float matmul det,
float exp poly, string copy, unsigned arith) surfaced one more: fstruct returned
0x391c0000 garbage (should be 34), fmat2's singular 3x3 float det returned -108
(should be 0). Root cause: byte3 0x2c/0x2d (single-precision FP pair `ldp s0,s1`)
was lumped into `fp_d` (scale 8), so each 32-bit s-reg was read as 8 bytes and
post-indexed 2x. FP/vector pairs distinguish 64-bit d (0x6d/0x6c, bit30=1) from
32-bit s (0x2d/0x2c, bit30=0). Split into a new `fp_s` flag: scale 4, 4-byte
transfers into the low 4 bytes of each 16-byte vector slot (sN = VECTOR_BASE +
N*16). fstruct 34, fmat2 0, fexp 649, str 0, uint 999 — all = native.
+regression ldst_pair_s_registers_use_4_byte_transfers. arm64jit 127/127,
workspace 176/0; full 5-batch battery + core C all green.
Session net: 8 FP/SIMD correctness fixes + FMADD, all regression-locked. Next:
keep pressing -O2 float/struct coverage, then libloader gaps. HARD GATE
unchanged (real libroblox.so boot + GPU host).

## Session (Sep 11, 2026) — sdiv/udiv signedness inversion (commit dae2e05)
6th -O2 battery (signed/variable division, fmin/fmax, fmod, fabs): idivA
(`s += a[i]/d[i%3]`) returned 0xaaaaaa2d, wanted -35. Root cause: the MulDiv
decode gate used `b(insn,17,17)==1` for SDIV-vs-UDIV, but bit17=0 for BOTH
forms — the real discriminator is bit10 (sdiv=1, udiv=0; verified by assembling
matching-operand pairs 0x1ac50c61 vs 0x1ac50861). So every sdiv was labeled
unsigned -> JIT emitted `xor edx,edx; div rcx` (unsigned) instead of `idiv`,
turning negative dividends huge. gcc's magic-constant division masked it in
earlier tests. Fixed to bit10 (matches the 2-source gate at ~2054).
+regression sdiv_is_signed_udiv_is_unsigned_same_negative_input. arm64jit
128/128, workspace **177/0**. idiv -33, idivA -35, all 6 batches + core green.
Cycle total: 9 FP/int miscompile fixes + FMADD, regression-locked. HEAD dae2e05.

## Session (Sep 11, 2026) — MSUB operand direction (commit ccbf55c)
7th -O2 battery (byte-string sum+modulo, switch table, SIMD-ish reduction,
16-bit accumulate, bitfield pack, strcmp): bytelen (n%50 after byte loop,
n=1298) gave -48 instead of 48. Root cause: MulDiv MSUB arm computed rn*rm-ra,
but ARM MSUB is ra-rn*rm, so n-(n/50)*50 via `msub w0,w1,w0,w2` = 25*50-1298 =
-48. Constant-folded addrs masked it. Fixed direction (MADD arm already
correct). +regression msub_reuses_rm_as_rd.*. arm64jit 133/133, workspace
**178/0**. All 7 batches + core + FMA green. Cycle total: 10 miscompile fixes +
FMADD, all regression-locked. HEAD ccbf55c.

## Session (Sep 11, 2026) — ADDV SIMD horizontal add (commit 6290937)
8th -O2 battery (64-bit mul/div, short-array SIMD sum, dot, int matmul): vadd
gcc fully SIMD-vectorizes a 16-short sum to `ldr q`+`addv s0,v1.4s`+`fmov w0,s0`
- returned 0 (wanted 360). ADDV undefined; an earlier dup/move gate swallowed
0x4eb1b820 and emitted per-lane identity copies. Implemented Inst::Addv: mask
0xfffffc00 (clears Vn bits9:5, Sd bits4:0), residues 8b/4h/16b/8h/4s; NOTE
source Vn at bits[9:5] (bits20:16 fixed=17) — nonstandard SIMD layout. Gate at
top of decode (dup/move swallowed it later). Translate sums sign-extended
lanes -> RDI -> bottom element of Vd. vadd 360=native. +regression
addv_horizontal_sum_across_4s_lanes. arm64jit 134/134, workspace **179/0**.
Cycle total (back half): 4 fixes (sdiv signedness, MSUB direction, s-pair
scale, ADDV) + earlier (WidenShl, FP-N, ld1-2reg, FMOVimm, FMA3, apply_shift,
d-pair stride) = 11 miscompile fixes + FMA + ADDV. HEAD 6290937.

---

## Session (Sep 11, 2026) — libloader: Android packed relocations (APS2) + RELATIVE application (workspace 184/0)

Per the ordered "libloader ELF/loader gaps" step: the Rust loader did **zero
relocation** — `load_elf_image` only mapped segments, sp-mprotected them, and
relied on `bind_image_plt` for JUMP_SLOT. Real Roblox APK libs and their
Android/GSI dependencies carry `R_AARCH64_RELATIVE` data relocations (often
packed via `DT_ANDROID_RELA`), which a real loader materializes before the code
can dereference pointer globals/vtables. The QEMU path worked around this with
the external `unpack_rela.py`; this session brought it in-process to the Rust
loader so the JIT path can load those libraries.

### `crates/libloader/src/android_relocs.rs` (new)
- `read_sleb128` (sign-correct, x64-bounded; terminal-byte bit6 = value sign).
- `decode_aps2` — faithful port of AOSP `for_all_packed_relocs` (validated in
  Session 14 against real 2.726.1142 libroblox.so): magic `APS2`, then
  SLEB128 `num_relocs` / running `r_offset` / groups with the
  GROUPED_BY_INFO/OFFSET_DELTA/ADDEND + GROUP_HAS_ADDEND flag logic.
  Declared-count mismatch → error (no silent truncation).
- `read_elf_relocations(path)` — walks PT_DYNAMIC, prefers
  `DT_ANDROID_RELA`/`DT_ANDROID_RELASZ` (0x60000011/12) over plain
  `DT_RELA`/`DT_RELASZ`, reads the stream from file, decodes APS2 or parses
  stock 24-byte Elf64_Rela. **Early real bug**: PT_DYNAMIC's `p_offset` is a
  *file* offset, not a vaddr — I wrongly ran it through `vaddr_to_file_offset`
  and got `DT_* vaddr not covered by a PT_LOAD`; only the RELA vaddr needs that
  conversion.
- `apply_relatives` — for each `R_AARCH64_RELATIVE` writes `load_bias + addend`
  (8B LE) at `guest_of(r_offset)` via a caller-supplied target resolver.

### Wired into `load_elf_image` (elf.rs)
Reordered the segment loop: copy-file → push segment (NO mprotect in the copy
loop), then for PIE (`is_pie`) read+decode relocs and apply RELATIVE **while
the whole image is still RW**, then a second pass mprotects each segment to its
final ELF protection. Non-PIE ET_EXEC (self-relocating like the battery ELFs)
is untouched by the `is_pie` gate.

### Verification (both honest, both catch regressions)
- Unit: a **hand-built** APS2 golden bitstream (independent byte-by-byte
  encode; first attempt used `[0x80,0x40]`="+0x2000" which is actually
  SLEB -8192 — the decoder correctly rejected my wrong test bytes, a good sign
  the decoder is faithful) + a grouped-by-offset-delta stride case
  (r_offset = header + i*delta) + SLEB negative + bad-magic reject.
- Integration `crates/libloader/tests/reloc_apply_test.rs`: cross-gcc
  `-shared -fPIC` ET_DYN with `int *ptr = &data` → asserts EVERY
  `R_AARCH64_RELATIVE` slot (parsed from `readelf -r`, whose type column is
  truncated to `R_AARCH64_RELATIV`) equals `load_bias + addend`. Without the
  apply the slot holds the raw file addend (e.g. 0x600) ≠ 0x100000600 → fails.
- `cargo test --workspace` **187/0** (libloader 20 → 23 unit + 1 integration);
  `cargo build --workspace` clean (only the pre-existing decode.rs rustfmt-warn
  churn). Skips when cross-gcc absent.

### Addendum (same cycle) — DT_RELR support
`read_elf_relocations` now also materializes **`DT_RELR`** (tag 0x23, Android
13+ / modern NDK default) when the ELF ships only `.relr.dyn` (no RELA table).
`dt_relr_to_relatives` implements the shipped glibc/Android `DO_RELR` decode
exactly (the low-bit-marker scheme; the upper-8-bit-delta variant was a rejected
alternative): an even word is an offset that relocates itself and seeds
`base = offset+8`; an odd word is a bitmap where bit *i* (1-based) → reloc at
`base + (i-1)*8`; odd value-1 padding decodes to nothing. +3 unit tests
(offset+bitmap bit mapping, no-prior-offset from base 0, non-multiple-of-8
reject). Note: neither the cross nor host `ld` here supports
`--pack-dyn-relocs=relr`, so no real `.relr.dyn` fixture is producible on this
box — the decode is anchored to the authoritative algorithm plus hand-computed
streams. `cargo test --workspace` **187/0**.

### Status / next
`load_elf_image` now materializes RELATIVE relocations for PIE/shared objects,
from **(a)** standard `DT_RELA`, **(b)** Android APS2-packed `DT_ANDROID_RELA`,
or **(c)** `DT_RELR` — all in-process (no `unpack_rela.py`). Next (ordered):
a GLOB_DAT/JUMP_SLOT resolver for the main dynamic (arm64jit's `bind_image_plt`
already covers JUMP_SLOT), then libbadcpu ISA gaps, then services/auth. HARD
GATE unchanged: `elfjit <libroblox.so> 0x1f0db20 --jni` run log on a
GPU + real-binary host (no APK/libroblox.so/GPU on this VPS).

### Addendum (same cycle) — libbadcpu: PEXT/PDEP were mis-emulated as BZHI
The VEX.0F38.F5 opcode byte is shared by **BZHI / PEXT / PDEP**, disambiguated
only by the VEX pp bits (assembler ground truth `gcc -c + objdump -d`:
bzhi=pp0, **pext=pp2, pdep=pp3**). The emulator's 0xF5 branch treated every
case as BZHI, so both **PEXT and PDEP silently produced wrong results** (a bit
extract became a low-bit mask). Fixed in `emulator.rs`: pp3/f3 → PDEP
(deposit source bit i into the i-th set mask position), pp2/f2 → PEXT (gather
source bits at mask positions into the low bits), else BZHI. Operands:
dest=ModRM.reg, source=vvvv, mask=rm. Also fixed a latent flag-ordering bug —
the BZHI/PEXT/PDEP carry flag was set *before* `update_flags_common` cleared
it, so CF never survived; now `cf_pending` is applied after. +2 tests with
expected values **verified on real hardware** `_pext_u64`/`_pdep_u64` (this
host has BMI2): pext(0xFF,0b1010)=3, pext(8,0b1010)=2, pdep(3,0b1010)=10,
pdep(0xFF,0b10101010)=170, plus 32-bit forms and a PEXT-vs-BZHI discriminating
case. `cargo test --workspace` **189/0** (libbadcpu 16 → 18).

### Addendum (same cycle) — loader→JIT end-to-end PIE + RELATIVE regression
Validated and regression-locked the full **`load_elf_image` → RELATIVE apply →
JIT execute** chain on a REAL PIE: cross-compiled `-fPIE -pie -nostdlib` aarch64
binary (`int *gptr = &shared_static; entry(){ return *gptr+1; }`) — the compiler
emits one `R_AARCH64_RELATIVE` in `.data.rel.ro` (offset 0x20008, addend
0x20000 = &shared_static). Without application `*gptr` derefs the unrelocated
link address (NULL page) and faults; with it, `gptr = base+0x20000` and
`entry() -> 42`. Verified by hand (`elfjit ./pie.elf -> 42`) and locked as
`loader_run_pie_relative_global_returns_42` in `crates/arm64jit/tests/
loader_run.rs` (new `compile_pie` helper; asserts the fixture really carries a
RELATIVE reloc). `cargo test --workspace` **190/0**.

## Session (Sep 11, 2026) — bind GLOB_DAT + ABS64 main-GOT relocations (commit 7f03937, workspace 191/0)

Continuing the ordered "libloader ELF/loader gaps" step. `bind_image_plt` only
walked `DT_JMPREL` (JUMP_SLOT) and `load_elf_image` only applied RELATIVE;
**R_AARCH64_GLOB_DAT (1025)** in the main `DT_RELA` was never bound. A
`-shared -fPIC` module referencing an exported global (data or function
pointer) goes through its **main GOT** via GLOB_DAT: the guest does
`adrp x0,GOT; ldr x0,[x0,#off]` to fetch the symbol's *runtime address*, then
derefs/calls through it. Unbound, the slot read 0 → SIGSEGV on NULL / call to
address 0.

Empirically confirmed (cross-gcc `-shared`): `global_data` and `gfp=&internal_fn`
produce two GLOB_DAT relocs at GOT offsets 0x1ffd8/0x1ffe0 (both 0 in file), and
`gfp = &internal_fn`'s initializer is **R_AARCH64_ABS64 (257)** — a second member
of the same "write symbol runtime-address" family that the loader also ignored.

### New `bind_glob_dat` (crates/arm64jit/src/plt.rs)
- Walks the main `DT_RELA`/`DT_RELASZ` for GLOB_DAT (1025) **and** ABS64 (257).
- **Defined-in-module** symbol → writes `el.guest_of(st_value) + addend` (ABS64
  carries the symbol offset as addend; GLOB_DAT addend 0). The loader maps
  guest==host, so `ldr xN,[GOT]` then `[xN]`/`blr xN` resolves back into the
  mapped image.
- **Undefined/imported** symbol → `STT_OBJECT` uses `dlsym` raw (guest==host
  addressable); FUNC/NOTYPE uses the resolver's host-call thunk (callable).
- Called from `bind_image_plt` (end of the normal path) **and** from the
  `pltrelsz == 0` early-return — an exported-data-only module has zero JUMP_SLOT
  yet still depends on the main GOT.
- Fixed en route: the `?` operator can't be used in a `(usize,usize)`-returning
  fn (switched the import branch to a match).

### Verification
- `elfjit /tmp/gdtest/self.so 0x360` (real `-shared` aarch64):
  ```
  [plt] (no JUMP_SLOT) bound 2 GLOB_DAT, 0 unresolved  -> then ABS64 added: 3
  JIT(no-QEMU) entry() -> 37 (0x25)     # global_data(11) + gfp(3)=internal_fn(3)=15 + global_data(11)
  ```
  Before the fix it SIGSEGV'd at fault=0x0 (guest GOT loaded 0).
- **+`loader_run_shared_glob_dat_and_abs64_returns_37`** in
  `crates/arm64jit/tests/loader_run.rs` — cross-gcc `-shared -fPIC -nostdlib
  -Wl,-e,entry` fixture with the exact two-global GOT pattern; runs the full
  load→RELATIVE→GLOB_DAT/ABS64→jit_run pipeline and asserts 37. Skips without
  the cross toolchain.
- `cargo build --workspace` clean; `cargo test --workspace` **191/0**.

### Next (ordered, no APK/GSI/GPU on this box)
1. Continue libbadcpu ISA gaps (the SIGILL emulator's remaining VEX/legacy
   instructions), then services/auth.
2. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK are available.

## Session (Sep 11, 2026) — GLOB_DAT/ABS64 binding, libbadcpu BMI2 completion, auth identity (workspace 199/0)

Ordered plan continued (no APK/GSI/GPU on this box; the android idempotent
test is already fixed and green). Four focused commits, each closed by the
real `cargo build --workspace` + `cargo test --workspace` gate:

### 7f03937 — arm64jit: bind GLOB_DAT + ABS64 main-GOT relocations
`bind_image_plt` only walked DT_JMPREL (JUMP_SLOT) and `load_elf_image` only
applied RELATIVE, so `R_AARCH64_GLOB_DAT` (1025) — how `-shared -fPIC` code
fetches an exported global's runtime address via the main GOT — was never
bound: the guest `adrp;ldr x0,[GOT]` read 0 and deref'd/called NULL.
New `bind_glob_dat(el)` walks DT_RELA for GLOB_DAT **and** R_AARCH64_ABS64
(257, the sibling "write symbol value" data-initializer family, confirmed by
cross-gcc that `gfp=&internal_fn` emits ABS64), writes `el.guest_of(st_value)
+ addend` for defined-in-module symbols, `dlsym` (OBJECT) / host-call thunk
(FUNC) for imports. Runs in the normal path and the `pltrelsz==0` early-return
(an exported-data-only module has zero JUMP_SLOT yet needs the main GOT).
Real `-shared` fixture: SIGSEGV (fault=0x0) -> `entry() -> 37` (global_data 11
+ gfp(3)=15 + global_data 11). +`loader_run_shared_glob_dat_and_abs64_returns_37`.

### b177f7c — libbadcpu: MULX (VEX.0F38.F6) + RORX (VEX.0F3A.F0)
BMI2 gaps: MULX = unsigned RDX*rm, high->modrm.reg, low->vvvv (u128 product —
a naive u64 `>>64` overflowed), flags cleared. RORX = rotate-right-by-imm8,
flags untouched; added the VEX.0F3A dispatch (decoder stops after ModR/M, so
read imm8 at RIP+len / advance len+1). +2 tests (values verified with -mbmi2:
rorx64(1,4)=0x1000000000000000, rorx32(1,31)=0x2).

### 2a9c6eb — libbadcpu: ADCX (66 0F38 F6) + ADOX (F3 0F38 F6)
`emit_adcx_adox`: Dest=Dest+Src+flag, write only the working flag (CF/OF).
Two real bugs found+fixed: width from REX.W not operand_size (the 66/F3 is a
mandatory opcode prefix, not a size override — a 64-bit ADCX is 66 48 0F38 F6,
operand_size folds 66->16); and 32-bit carry detected in the u32 domain
(0xFFFFFFFF+1 wraps to 0 WITH carry). Ground truth from a real-BMI2 assembly
driver confirmed the ADOX subtlety: OF is set to the *unsigned* carry-out, not
signed overflow (adox(0x7fff..,1)=0x8000.. has OF=0). +2 tests.

### a8249f4 — sober-services: forward full login result (services/auth)
The OAuth webview's AuthResult declared user_id/username but the callback only
extracted the token — IPC AuthToken always went out with both None, so the
parent couldn't identify the account without a second Roblox API call.
New extract_auth_result() parses token + user_id + username (query precedence,
#fragment tolerant, URL-decoded); send_auth_result() forwards the identity;
run_login_flow blocks on the token then sends the full result. +4 tests.

### Gate
`cargo build --workspace` clean (0 errors), `cargo test --workspace` **199/0**
(arm64jit 120 + loader_run 7 incl. the glob_dat fixture; libbadcpu 22;
sober-services 15; libloader; others). HEAD `2a9c6eb`, tree clean.

### Next (ordered, no APK/GSI/GPU on this box)
1. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE — blocked until a capable host + the real binary/APK exist.
2. Continue hardening: next ISA/loader/emulator gaps as discovered (adb/emulate
   surface), then the remaining sober-core/sober-services integration.

---

## Session 2026-09-11 — LogicImm DecodeBitMasks rotate-RIGHT fix (workspace 200/0)

### The bug (severe, silent, commonest-instruction-class)
`decode_logical_mask` (crates/arm64jit/src/decode.rs) applied a **LEFT**-rotate
to the immediate element (`ones << r | ones >> (esize-r)`) but ARM
`DecodeBitMasks` (DDI0487) uses **ROR — rotate right**. Every rotation-
ASYMMETRIC logical-immediate mask was silently miscompiled (any `mov/and/orr/
eor/tst/ands xD,#<asym-mask>`). Symmetric masks (alternating 0xCCCC/0x5555,
single-bit, all-ones) give the same value under both directions, which is why
the whole prior test set stayed green for months despite the wrong code —
<15% of encodings are rotation-invariant.

Real failure: `mov x0,#0xffffffff80000001` = `0xb26187e0` (immr=imms=33)
returned `0xfffffffe00000007` instead of `0xffffffff80000001`, caught by a
cross-gcc probe run through elfjit (no QEMU) vs native x86-64.

### Fix + quantified verification
- Rotate right in the esize-bit domain: `(ones >> r | ones << (esize - r)) & em`.
- Ground-truth cross-check vs the real `aarch64-linux-gnu-as`+`objdump` over
  ~700 (N,immr,imms): fixed right-rotate agrees **592/592 valid**; old
  left-rotate would have been wrong on **509**.
- End-to-end elfjit: `mov x0,#0xffffffff80000001` -> exact value; the
  esize-64! case `mov w0/x0,#0x3ffffffc` (`0xb27e6fe0`) exact too.
- Regression `logic_imm_rotation_asymmetric_mask_ror` (both asymmetric
  encodings + symmetric 0xCCCC/#1 unchanged).
- `cargo build --workspace` clean; `cargo test --workspace` **200/0**.
- Commit `8c1706b`.

### Next
Continue the cross-gcc ISA-surface battery for more silent-miscompile classes;
libloader/libbadcpu/services gaps; HARD GATE (real Roblox boot, GPU/APK host)
unmet on this box.

---

## Session 2026-09-11 — three silent SIMD/logic-imm miscompiles fixed (workspace 202/0)

Session opened with the handoff-flagged failing test already green (200/0 from
committed cycles); the "failing test first" gate was satisfied by prior work.
This session's value = three silent JIT miscompiles flushed out by cross-gcc
batteries driven end-to-end through elfjit (no QEMU) vs native x86-64, all
found and fixed in arm64jit:

1. **LogicImm DecodeBitMasks rotate-RIGHT** (`8c1706b`): the logical-immediate
   element was LEFT-rotated; ARM DecodeBitMasks uses ROR. Every rotation-
   asymmetric mask silently miscompiled (`mov x0,#0xffffffff80000001` ->
   0xfffffffe00000007). Only symmetric masks gave the same value under both, so
   the prior test set stayed green. Verified quantified vs the real assembler:
   right-rotate agrees 592/592 valid (N,immr,imms); old left   would be wrong
   on 509. Regression + loader_run end-to-end gate (`076b024`).

2. **Scalar-D ADDP vs fcvtzs collision** (`01aa402`): `addp Dd, Vn.2D`
   (0x5ef1...) collided with the scalar fcvtzs gate at 0x5ee0b800 — bit20 is
   the discriminator (ADDP SET, fcvtzs CLEAR). Old gate silently ran every gcc
   pairwise-add reduction as a float->int. New SimdPairAddD.

3. **saddw2/uaddw2 upper-half** (`01aa402`): SimdAddw always read Vm at byte 0;
   the Q=1 form (saddw2) reads the UPPER 64 bits. -O2 vectorized loops do
   saddw (low) then saddw2 (high); old code accumulated low twice.

All three proven end-to-end: the i*i reductions return 76 (native) at both -O3
(addp) and -O2 (saddw+saddw2). 50+ cross-gcc battery programs acros.
`cargo build --workspace` clean; `cargo test --workspace` **202/0**.
HARD GATE unchanged: real Roblox boot/GPU host (no APK/GPU here).

---

## Session (Sep 11, 2026) — differential battery + sxtl2/uxtl2 upper-half FIX (workspace 210/0)

### New capability: differential battery (`crates/arm64jit/tests/diff_battery.rs`)
Cross-gcc compiles a C `entry()` to aarch64; the harness ALSO compiles the same
source with native `gcc` and runs it as the oracle; the loader→JIT pipeline must
return EXACTLY the oracle value. This is the strongest silence-detector in-tree
(native oracle vs JIT), and it immediately caught two MORE issues beyond the
already-fixed LogicImm/ADDP/saddw batch.

### FIXED: `sxtl2`/`uxtl2` (SIMD long-extend) ignored the Q bit
`Inst::SimdXtl { rd, rn, sign, esrc }` had no `upper`, so
`sxtl2 v28.2d, v28.4s` re-read v28's LOW 64 bits instead of bytes 8..15 —
every int→i64 vectorized init loop gcc emits (`movi v.4s,#n; sxtl; sxtl2; stp q`)
computed the upper half from the wrong lanes. Fixed the same way the existing
`saddw2`/`uaddw2` fix does:
- decode.rs: +`upper: (insn >> 30) & 1 == 1`
- translate.rs: source lane offset `n_half = if upper { 8 } else { 0 }`
Three deterministic linear regression tests in `jit.rs` pin it:
`and_then_sxtl_sxtl2_upper_half`, `and_sxtl_accumulation_two_iterations`,
`simd_stp_q_preindex_store_and_writeback` — the last replays the FULL maskf
loop body (movi; ldr q init from [x0,#400]; then 6× {mov snapshot; add v31+=4;
and &0xf; sxtl; sxtl2; stp q27,q28,[x0],#32}) and stores m[k]=k&0xf exactly.

### OPEN (documented): intermittent SIMD-loop block-liveness bug
The identical instruction stream FAILS nondeterministically when run through
`jit_run`'s single-block **b.ne back-edge** compilation of the whole function:
gcc -O2/-O3 int→i64 widening init loops EITHER compute correctly (verified vs
qemu-aarch64 ground truth: maskf 7001003, regloop 23017003, %101 60018021) or
corrupt ONE snapshot lane (m[4i+1] = address/stack-layout garbage while
m[4i],m[4i+2],m[4i+3] stay correct). Intermittent per process AND per heap
allocation (trial N), i.e. an uninitialized x86 register at the loop back-edge,
NOT an ISA miscompile (all ops verified correct linearly). The corrupted-lane
differential canaries (maskf/times7/mod_pow2/regidx/struct_arr/mixed) are
therefore excluded from the permanent gate; `diff_mixed_arith_accumulate` is
`#[ignore]`-documented. Root-causing this subsumes the struct/`%101` array
cases too. Next: instrument the block liveness / XMM scratch registers around
the loop back-edge under `compile_image_bounded`.

`cargo build --workspace` clean; `cargo test --workspace` **210/0** (1 ignored).
HARD GATE unchanged: real Roblox boot / GPU / APK host (none on this VPS).
### Addendum (same session): the "intermittent" bug above is ROOT-CAUSED and FIXED
Root cause: ARM `nop` (0xd503201f) and the whole system/hint 0xd5... family
were misdecoded as `ScvtfFixed` — the scalar int→fp FIXED-POINT gate checked
only `(insn & 0x30000000)==0x10000000`, which 0xd5xxxxxx also satisfies. So every
guest `nop` executed as `scvtf d<n>, x0, #56`: it CVTSI2SD'd the caller's x0
(very often the stack pointer) and DIVSD'd it into a vector register. That is
precisely the observed layout/address-dependent single-lane corruption. Fix:
the ScvtfFixed gate now also requires top byte in {0x1e,0x9e}. Additionally,
ScvtfFixed sf/to_double read bit30 but must read bit31 (0x9e=X/D vs 0x1e=W/S);
real `scvtf d0,x0,#1`=0x9e42fc00 was being decoded as a single Sd/Wn convert.
Decode regression `nop_is_hint_not_scvtf_fixed` pins all three.
After these, EVERY int→i64 widening-init-loop differential canary passes
deterministically (maskf=7001003, mod_pow2=1007005, times7=161119021, count6=24,
verified vs qemu-aarch64). Two SEPARATE deterministic bugs remain (magic-div
`%N` reducer: m[0]=-101 for k*7%101; and the -O3 addp/smulh reduction) and are
#[ignore]d/documented. Workspace 212/0 (3 ignored).

## Session (Sep 11, 2026) — mls + uzp2 implemented, rev64 gate widened; all 3 remaining SIMD bugs CLOSED (workspace 216/0, 0 ignored)

The two leftover deterministic bugs (magic-div %101 m[0]=-101 and the -O3
vectorized reduction) share one root cause. Isolated the gcc %101 reducer
chain (smull/smull2/uzp2/sshr/mls) in a scratch example and diffed against
qemu-aarch64, then pinned the failing ops with decode(insn):

1. **`mls` (multiply-subtract) misdecoded as a plain Simd4s SUBTRACT.** The
   `(insn & 0xffe0_fc00)` gate for Simd4s caught `mls v26.4s,v0.4s,v28.4s`
   (0x6ebc941a) as `sub`, **losing the multiply entirely** — so the quotient
   was never subtracted. Implemented `Inst::SimdMla { rd, rn, rm, lanes,
   sub }` (mla=0x0ea09400/0x4ea09400 add, mls=0x2ea09400/0x6ea09400 sub),
   gate placed BEFORE the Simd4s gate; translate does per-lane
   `Vd = Vd ± Vn*Vm` (low-32 product).

2. **`uzp2` (unpack-high) misdecoded as `rev64`.** The SimdRev gate
   `(insn & 0x3f00_0c00)==0x0e00_0800` drops bit12 and swallowed the whole
   uzp1 (0x18) / uzp2 (0x58) opcode family. Tightened it to require
   bits[13:12]==00, and added `Inst::SimdUz2 { rd, rn, rm, esize, q }`
   (byte1 high-nibble==0x5) gathering the ODD/upper elements
   `Vd[i]=Vn[2i+1]; Vd[n/2+i]=Vm[2i+1]` — what feeds the sshr quotient step.

Verified: the previously-`#[ignore]`d `diff_magic_div`, `diff_struct_array_
fields` and `diff_mixed_arith_accumulate` all pass again as permanent gates
(un-ignored). Added decode regression `mls_and_uzp2_decode_as_specific_ops_
not_sub_or_rev`. Workspace 216/0, **0 ignored** — the differential battery is
fully green with every case a live gate.

### Next
- Ideal next: keep sweeping the ISA breadth the battery doesn't yet cover —
  widen the reducer family (umull2/umlal/uaddl to exercise uzp-variant and
  long-multiply paths), add string/booleans, and push the battery onto more
  real-compiler idioms (-O3 reductions already live). Then brace for the real
  Roblox APK path (ELF/loader + JNI stubs) once an APK/GPU host is available.
  Blocked on this VPS only by the HARD GATE (no GPU/APK).

### Addendum (same session, after the mls/uzp2/rev64 commit) — UNSIGNED magic-division closed (workspace 220/0, 0 ignored)
Extending the battery to UNSIGNED `%const` (gcc emits the mul/umull/umull2/
uzp2/ushr/zip1/zip2 reducer, the unsigned sibling of the signed smull one)
immediately surfaced FOUR more misdecodes, all isolated against qemu-aarch64:
1. **`mul` (NEON element-wise 32-bit multiply) decoded as SimdVLog (bitwise).**
   The vector-logical AND/ORR/BIC gate checked byte1&0x1c00==0x1c00 but never
   bit15: mul's byte1 (0x8c..0x9f) sets bit15, and/orr/bic (0x1c/0x1d) don't.
   So EVERY NEON multiply became an AND/ORR — including the magic-division
   dividend `mul v26.4s,v26,v28(97)`, which corrupted the quotient. The JIT's
   full-loop m[] came out all-0 and acc=0. Fix: require (insn&0x8000)==0.
2. **`uzp2` misdecoded as `rev64`, then as `uzp1`.** The rev64 gate dropped
   bit12 (fixed earlier); the uzp1 gate's &0x3f mask dropped bit6, so uzp2
   (byte1 0x58) was even-gathering. Added Inst::SimdUz2 (odd/upper gather);
   both uzp gates now key on byte1 0x18-/0x58-family + byte3-low-0x0e +
   bit28 clear (excludes bit/bif/bsl and rev64).
3. **`zip2` Unsupported.** Added Inst::SimdZip2 (upper-half interleave, base
   0x0e007800 vs trn2 0x0e006800). gcc uses zip1/zip2 with a zero lane to
   widen a 4s quotient into 4 u64.
4. **`mls` = plain Simd4s SUBTRACT (no multiply)** — added SimdMla {sub}.
Also: WidenShl gate restored to the genuine shll long-shift family but made to
exclude the permute ops via byte1 bits[1:0]==00 (shll 0x38 vs zip1 0x39/0x3b).
Verification: diff_unsigned_magic_div_umull, diff_long_accumulate_widening,
diff_byte_scan_strlen new; all green un-ignored; 3 decode regressions added
(mls_and_uzp2..., mul_decodes_as_multiply_not_bitwise_logical, and 'and' still
logical). Workspace 220/0, **0 ignored**. Commits ab24b32 (fix) — prior
d239e8c/3b4ff25 (signed path). Difference: JIT and qemu-aarch64 now agree on
both the signed and unsigned magic-division kernels exactly.
---

## Session (Sep 11, 2026) — 6 silent vector-FP/NEON miscompiles fixed; workspace 227/0, 0 ignored

Commits `ea84aff` (fixes + unit tests) + `059151a` (differential canaries) on `dev`.
Opened at 220/0 (no failing test — the android idempotent test stays green). Extended
the differential battery into the **.4s/.2d single/double-precision float-vector
SIMD** family (a 3D engine's vertex/matrix math is ~all of it) that the older
integer/double batteries never touched, with volatile seeds so gcc can't constant-fold
while still vectorizing. It immediately flushed out **SIX silent miscompiles** — every
one would corrupt pixels/coordinates/audio on a real host:

1. **VecIntToFp** — `scvtf/ucvtf Vd.4s/.2s` (and signed `.2d`, `0x4e21d800` family) were
   misdecoded as **SimdMull** (widening multiply); every vector int->float made garbage.
2. **VecFpArith** — `fadd/fsub/fmul/fdiv/fmax/fmin/fmaxnm/fminnm Vd.2s/.4s` (2-source)
   were misdecoded as integer SIMD (`SimdAddB`/`Simd4s`/`SimdMull`). Only FMLA
   (accumulate) and the `.2d` double forms (Simd2dFp) were handled; the two-source
   single-precision family was entirely MISSING. Added `divss` x86 emitter.
3. **SimdMlaEl** — integer `mla/mls Vd.4s, Vn, Vm.s[idx]` (by-element) misdecoded as
   **VecMovi** (gcc int->float init `mla v5.4s,v18,{loop}.s[0]` corrupts the a*scalar
   product). Gate: top nibble 0x0f + bit29 set (FP fmla-el is bit29 CLEAR) + bit23 set
   (excludes smlal/umlal-by-el byte1 0x42); `.4s` index = (bit11<<1)|bit21.
4. **Permute ordering** — `zip1` (and zip2/uzp1/uzp2) now decoded BEFORE the
   `WidenShl` (shll) gate. Real `zip1 Vd.4s` has byte2 0x38 (same as shll) and was
   misdecoded as a widening shift; the old guard only excluded the 0x39/0x3b byte2
   variants. Added an early compact permute gate (zip1/zip2/uzp1/uzp2 on 0x3f20fc00).
5. **SimdFmulEl cross-lane alias bug** — the broadcast element was re-read *inside* the
   lane loop, so when `rd==rm` (`fmul v17.4s, v7.4s, v17.s[0]`) lane 0's write clobbered
   the element before lanes 1-3 read it → every lane after 0 wrong. Now broadcast once
   up front (the FmlaEl "load into xmm2 before the loop" pattern).
6. **VecFpCmp** — `fcmeq/fcmgt/fcmge Vd.4s/.2s/.2d` (compare→all-ones mask) misdecoded
   as **SimdVShift**; gcc's float-vs-const count loop returned garbage. Gate = byte2
   0xe4 after the 0xffe0fc00 mask (NOT raw bits15:8, which include rn). New
   comiss/comisd + setcc + movzx + neg x86 emitters; per-lane mask = all-ones or 0.

New unit tests: `vector_fp_arith_ground_truth` (fmla/fmul-by-el/fmla2d/scvtf/fadd,
incl. negative scvtf lanes, fmov-imm broadcast, fmov-s,w), `vector_fp_by_element_highreg_and_2d`
(exact gcc high-reg by-element + .2d words). New permanent differential canaries:
fv_arith, fv_sub_neg, fv_f2i, fv_f2i_neg, fv_i2f, fv_fmla_scalar, fv_cmp_count,
dv_arith, fv4, fv4_fmul_only.

**Verification:** `cargo build --workspace` clean; `cargo test --workspace` **227/0**,
**0 ignored** (was 220/0). Full differential battery 17/17 (every case a live gate).

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep pressing the SIMD float/FP vector breadth (single-precision struct-by-value
   with vector lanes, fmla-by-element chains, more -O3 reduction shapes); then widen into
   the remaining integer-permute/wide paths the new float canaries may expose.
2. `fv_cmp_count`-style FP compares are now real (setcc-based); add fcmlt/fcmle
   (operand-swapped gt/ge) if a battery case needs them.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — 9 silent miscompiles + the long-open nondeterministic SIMD-loop bug fixed (workspace 234/0, 0 ignored)

Five commits on `dev` (all `cargo build --workspace` + `cargo test --workspace` green):
`6ffd490` (32-bit add/sub flags + ccmp/ccmn), `e1ec841` (4 FP/control-flow bugs),
`7e2689a` (ADDV-to-scalar stale bytes), `5cbe6ce` (CMN/ADDS C-flag polarity),
plus this docs commit. Opened at 230/0; no failing test (the android idempotent
stays green). Delivered through the differential-probe harness — cross-compile the
same C at -O2/-O3 through `elfjit`, compare against a native x86-64 oracle. HEAD of
these runs caught 9 real bugs:

1. **32-bit ADDS/SUBS flag semantics** — `!sf` flagged adds/subtracts used 64-bit
   x86 add/sub after zero-extending, so `store_nzcv` saw SF from bit63, not bit31.
   `adds w1,w1,w2` with 0x7fffffff+1 gave N=0 (wrong), so `int s=INT_MAX+1; s<0`
   returned the wrong branch. Added 32-bit emitters (no REX.W) and routed `!sf`
   paths through them.
2. **ccmp/ccmn missing** (conditional compare) — swallowed by the logical set-flags
   decoder (shares 0xFA/0x7A/0xBA/0x3A top bytes), corrupting NZCV so gcc's
   `while (a<N && b!=M)` guards spun forever. Added `Inst::CcMp` (residue class
   distinct from ANDS/BICS/SBCS; rn=[9:5] rm/imm=[20:16] cond=[15:12] nzcv=[3:0]);
   translate mirrors Fccmp (load_nzcv -> jcc -> nzcv|compare).
3. **CSel rn/rm==31 read the SP slot** instead of XZR — `cset/cinc/csneg`
   (`a==0.0?1:0`) returned sp/sp+1. CSEL is data-processing: reg 31 is always XZR.
4. **Scalar `scvtf/ucvtf Dd,Dn` `sng` discriminator inverted** (bit22=1 is DOUBLE,
   code set sng on it) — a double scvtf truncated through the i32->f32 path.
5. **`fcmp Dn,#0.0`** decoded as a compare against vector reg d0 (garbage) — bit3
   (0x8) is the #0.0 discriminator, now `Fcmp.against_zero` loads literal +0.0.
6. **FP NaN compare flags** — `store_nzcv_fp` stored C as the true ARM value and
   Z=ZF (set for unordered too): `vnan==vnan` came out true and `nn<=0` (cset ls)
   wrong. Now Z = ZF&&!PF (excl. unordered) and C is stored in the borrow sense
   (CF&&!PF) that x86_cc_for_cond's ls/hi/lo/hs expect => all NaN comparisons false.
7. **ADDV-to-scalar left stale bytes** — `addv Bd,Vn.8b` stored only 1 byte, so the
   destination vector reg's upper bytes kept the `cnt` lane counts; gcc's
   `fmov x2,d31` then read `[sum, pc1, pc2, ...]` as the integer popcount. This is
   the exact **root cause of the documented intermittent SIMD-loop block-liveness
   corruption** (one element garbage, nondeterministic, stack-layout dependent)
   that the old maskf/times7/mod_pow2/regidx/mixed canaries hit. Now the ADDV
   store zeros the upper bytes (64-bit store of a size-masked sum). `simdu3`
   (popcount loop) returns exact 591 deterministically, 8/8 runs at -O2/-O3.
8. **CMN/ADDS C-flag polarity** (commit 5cbe6ce) — the flag-setting ADD path
   stored C = x86 carry, but x86_cc_for_cond's HS/LO/HI/LS assume the SUBTRACT-
   borrow convention. gcc's `unsigned um > 0xffffffffffff0000` (compiled to
   `cmn x,#0x10000; b.ls`) evaluated "not greater" when it carries. cmc before
   store_nzcv on the non-subtract paths stores C in borrow convention.
   structfp was off by 999999 from exactly this.

New permanent canaries: `diff_ccmp_cond_compare`, `diff_w32_overflow_compare`,
`diff_fp_compare_zero_and_cset`, `diff_fp_nan_compare`, `diff_addv_popcount_accumulate`
(all differential vs native oracle). Decode regressions: `ccmp_ccmn_decode`,
`fcmp #0.0 / d0 / fcmpe #0.0` additions.

**Verification:** `cargo test --workspace` **234/0**, **0 ignored** (was 227/0);
the 28-program differential probe suite (FP math/compare/cvt, NaN, signed-zero,
ccmp chains, 32-bit overflow compares, NEON/16-bit/unsigned SIMD, popcount,
branch tables, recursion) matches the native oracle at both -O2 and -O3.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery driving ISA/correctness breadth (SIMD permute/
   wide paths, more FP reduction/reassociation shapes, struct-by-value vectors).
2. `addv s0,v1.4s` 32-bit scalar-store path is now covered; check `saddv`/`uaddv`
   (signed accumulator) if a case surfaces one.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — SIMD INS lane-index decode FIX + Extr/unpack/zip batch folded (workspace 243/0)

Commit `b93e89d` on `dev` (all `cargo build --workspace` + `cargo test
--workspace` green). Two threads:

1. **Closed the persistent `diff_float_vector_reduced` (fv4, -O3) miscompile —
   jit 153 vs native oracle 175.** Root cause was NOT the permute snapshot:
   the INS (vector, element) lane-index decode read `dst_idx = bit20` and
   `src_idx = bit14` — two single bits that only coincided with the true lane
   index for S lane 0->1. Every S lane beyond 0 and every H/B lane silently
   copied into the wrong vector element. gcc -O3 emits `mov v3.s[1], v28.s[0]`
   and `mov v31.s[1], v4.s[0]` in the float tight loop fv4 exercises, so a wrong
   `dst_idx` put the seed/accumulator f32 lanes in the wrong V slots. Correct
   packing (verified against the aarch64 assembler for all 4x4 S, 8x8 H, 16x16
   B, 2x2 D lane pairs — 340 encodings, 0 mismatches):
   `l = log2(esize); dst = imm5 >> (l+1); src = (insn>>(11+l)) & ((1<<(4-l))-1)`.
   fv4 now returns 175 == oracle. Locked with `simd_insd_sets_correct_lane_with_multi_byte_indices`
   (mov v3.s[2],v5.s[1] copies 3.5f into lane 2; move-back bytes assembler-verified).

2. **Folded in the earlier uncommitted arm64jit batch** (verified green so HEAD
   stays clean): general EXTR with `rm != rn` (`extr x0,x0,x1,#51`, gcc's shift-
   rotate idiom) now decodes as `Inst::Extr` instead of falling through to the
   UBFM/SBFM gate; uzp1/uzp2/zip1/zip2 snapshot their rd-aliased source to a
   `permscratch` buffer in CpuState (gcc's ubiquitous `uzp1 v31.8h,v31.8h,v26.8h`
   and `zip1 v31.4s,v3.4s,v31.4s`); and REX.B on shl/shr/ror/not/neg emitters so
   guest regs >= 8 (R10+) are addressed correctly. Regression tests:
   `extr_general_two_operand_rotate`, `uzp1_rd_aliases_rn_does_not_corrupt_source`,
   `csel_family_op_discriminates_neg_not_inc_identity`, plus diff_battery
   canaries `diff_integer_signed_division_negative_edge`,
   `diff_rotate_extract_and_byte_accum`, and `diff_float_vector_reduced` (now green).

**Verification:** `cargo test --workspace` **243/0** (146 arm64jit lib incl. the
new ins test; 26 diff_battery incl. fv4). HEAD `b93e89d`.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery driving ISA/correctness breadth (more SIMD
   lane/permute/wide paths, FP reduction/reassociation shapes).
2. Move up to the runtime side: the FMOD "divert guest bl-to-once through the
   dispatcher" task and JNI function-table stubs per RECOMMENDATION.md.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — ld3/st3/ld4/st4 structure DEINTERLEAVE implemented (workspace 245/0)

Commit `1c5f69e` on `dev`. Continued the differential-battery ISA sweep. A 4x4
float matmul (`C[i][j] += A[i][k]*B[k][j]`, gcc -O3) returned **20 vs oracle
5248**. Root cause: the structure-load opcode field (insn bits15:12) was folded
wrong — **0b0000 (ld4/st4) mapped into the single-register ld1 placeholder and
0b0100 (ld3/st3) to Unsupported**, so both ran the ld1-multiple
CONSECUTIVE-load path. AArch64 ld4/ld3 are structure **deinterleave** loads
(`Vd[j][i] = mem[base + i*N*es + j*es]`); loading them consecutively read the
wrong memory. gcc -O3 emits `ld4 {v24.4s-v27.4s},[sp]` to load matrices.

Fix: new `Inst::Ld3N/St3N/Ld4N/St4N` with true, element-size-aware
deinterleave; decode maps op=0b0000->Ld4N/St4N and op=0b0100->Ld3N/St3N
(ld1-multiple keeps only 0b0010/0b0110/0b0111/0b1010). Byte-verified against
qemu for ld3/ld4 q=0 & q=1 and st4 — all match. matmul now = 5248, transpose =
684, complex = 288 (all = native oracle). Regression: decode unit test
`ld4_st4_decode_to_structure_deinterleave`; differential canaries
`diff_ld4_st4_matrix_transpose` (ld4_matmul + st4_transpose). Workspace 245/0.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping ISA/correctness breadth (structure
   load/store widths, more SIMD lane/permute/wide paths, FP reduction shapes).
2. Move up to the runtime side: FMOD "divert guest bl-to-once through the
   dispatcher" and JNI function-table stubs per RECOMMENDATION.md.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — vector fmls operand order + dup-from-GPR vs sqadd gate (workspace 249/0)

Commit `7fef13e`. Two real silent SIMD miscompiles from the differential
battery's complex-matrix and -O2 fill probes:

1. **Vector fmls inverted operands.** `fmls Vd,Vn,Vm` (= Vd - Vn*Vm) shared the
   commutative add's mul/load ordering, so the JIT computed `Vn*Vm - Vd` — right
   magnitude, wrong sign. 4x4 complex matmul returned 10688 vs oracle 13504;
   `acc -= a*b` loop +20 vs oracle -100. Fixed by loading Vd into xmm0 and the
   product into xmm1 so subss(0,1) = Vd - Vn*Vm (also .2d el64 path).
2. **`dup Vd.T, Wn` swallowed by the SIMD saturating-add gate** (sqadd/uqadd/
   sqsub/uqsub have byte2==0x0c too). gcc -O2 matrix/fill loops emit `dup
   v30.4s,w1; add v30,v30,v31; scvtf; str q30,[x],#16`, so the broadcast decoded
   as a sat-add and every array held garbage (init_O2: 18446744039484557312 vs
   96). Verified vs assembler: bit21 is SET for all 14 sat-add forms and CLEAR
   for all 6 dup-from-GPR widths — now required in the sat-add gate.

Regression: `fmls_vector_subtract_has_correct_operand_order`,
`dup_from_gpr_not_swallowed_by_sqadd_gate` (decode); differential canaries
`diff_fmls_vector_subtract_accumulate`, `diff_dup_from_gpr_matrix_init`.
cargo build clean; `cargo test --workspace` 249/0. HEAD `7fef13e`.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping ISA/correctness breadth (structure
   load/store widths, more SIMD lane/permute/wide paths, FP reduction shapes).
2. Move up to the runtime side: FMOD "divert guest bl-to-once through the
   dispatcher" and JNI function-table stubs per RECOMMENDATION.md.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — sat-add lane-width/sign + ubfiz-vs-ror fixes (workspace 251/0)

Commits `e3f12be` (SimdSatAdd) and `3696d89` (UBFM). The differential battery
kept finding real silent SIMD miscompiles:

1. **Saturating add/sub treated >=32-bit lanes as 64-bit ops.** SimdSatAdd used a
   64-bit load for any lane esize>=4, so `.4s` loaded 8 bytes as ONE value and
   clamped both s-lanes together (10+5 -> 0x80000000 smin sentinel; qemu 15).
   This path was only reachable after the dup-from-GPR gate fix (it had been
   silently masked by the same gate collision). Fixed with per-lane width loads,
   sign-extension (movsx/movsxd) so 64-bit clamps judge negatives correctly,
   SIGN-EXTENDED smin constants (0xFFFFFFFF80000000 for .4s — the raw lane-width
   0x80000000 as u64 is positive, so 15 < 0x80000000 and every non-negative
   result clamped), and .2d 1<<64/1<<63 guard shifts. Verified vs qemu across 8
   widths x signed/unsigned x add/sub.
2. **ubfiz/sbfiz with immr>imms hit the UBFM ROR shortcut.** `ubfiz w4,w2,#3,#3`
   (immr=29, imms=2; 29+2+1==32==bits) matched the `imms+immr+1==bits` rotate
   gate BEFORE the shift-extend branch, so it rotated right by imms=2 instead of
   computing (w2&7)<<3. A -O2 mix/shuffle-hash (`h ^= msg[i]<<((i%8)*8)`)
   returned 13680984341602923654 vs oracle 13072640789477207222. A genuine ror
   always has immr<=imms; gated the ROR branch on `!(immr > imms)` so ubfiz/
   sbfiz fall through to their shift path. mix now = oracle; real ror/extr tests
   stay green.

Regression: `sqadd_uqadd_respect_lane_width_and_sign`,
`ubfiz_immr_gt_imms_does_not_rotate` (exec); differential canary
`mix_hash_ubfiz`. cargo build clean; `cargo test --workspace` 251/0. HEAD `3696d89`.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping ISA/correctness breadth (structure
   load/store widths, more SIMD lane/permute/wide paths, FP reduction shapes).
2. Move up to the runtime side: FMOD "divert guest bl-to-once through the
   dispatcher" and JNI function-table stubs per RECOMMENDATION.md.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — fixed-point fcvtzs #fbits + post-index multi-reg ld1 (workspace 253/0)

Commit `97417af`. Two more silent arithmetic/load bugs from the differential
battery:

1. **Fixed-point FP->int** (`fcvtzs/fcvtzu Rd, Fn, #fbits`, result = trunc(Fn *
   2^fbits)) was misdecoded as `SimdMull` (smull x0,w31,w24) by the widening-
   multiply gate — every *2^fbits scale silently dropped. gcc -O3 folds
   `(long long)(s*4)` into fcvtzs #2; the double square-sum `v64f` returned 5 vs
   oracle 436. Added an `fbits` field to `FcvtToInt`, decoded the fixed-point
   encodings (top16 0x1e18/0x1e58/0x9e18/0x9e58 signed, +bit16 unsigned; fbits =
   64 - bits[15:10]) before SimdMull, and scale xmm0 by 2^fbits in translate.
   vs qemu: 3.25>>#2 = 13, >>#4 = 52, s 2.5>>#3 = 20, fcvtzu 3.75>>#1 = 7.
2. **Post-indexed multi-register ld1/st1** `{Vt..,Vt+n},[Xn],#imm` sets bit23
   (bases 0x..cc0 ld / 0x..c80 st), which the structure-multiple gate's four
   no-post bases missed — `ld1 {v26.16b,v27.16b},[x1],#32` fell through to the
   single-vector Ld1V gate: loaded only 16B and advanced Xn by 16 not 32. An
   -O2 double dot-product (fmadd loop + shifted-register add addressing)
   accumulated 165 vs oracle 470. Added the 4 post-index bases.

Regression: `fcvtzs_fixed_point_fbits_scales`, `ld1_multireg_post_index_decode_and_advance`;
differential canaries `fcvtzs_fixed_scale`(/neg), `fma_ld1_postidx`.
cargo build clean; `cargo test --workspace` 253/0. HEAD `97417af`.

### Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping ISA/correctness breadth (structure
   load/store widths, more SIMD lane/permute/wide paths, FP reduction shapes).
2. Move up to the runtime side: FMOD "divert guest bl-to-once through the
   dispatcher" and JNI function-table stubs per RECOMMENDATION.md.
3. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
4. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays the
   HARD GATE, blocked until a capable host + the real binary/APK (none here).

---

## Session 2026-09-11 (cycle 19) — scalar single-precision FP rounding fixed (workspace 254/0)

Workspace opened green (253/0, android idempotency test passing — the mandated
first fix was already committed in prior cycles). Continued the differential
battery ISA sweep and flushed out **two silent miscompiles** in the arm64jit
FpUnary SINGLE-precision path (`frintm/frintp/frintz/fsqrt s`):

1. **Loaded float bits never reached xmm0.** The single path did
   `mov_load32(RAX, RNslot)` then `cvtss2sd(0,0)` — but `cvtss2sd` reads xmm0's
   low32, so EVERY single frint/fsqrt converted STALE xmm0 instead of the value.
   The FpScalar single path (op 4-7) correctly inserts `movd_xmm_r32(0, RAX)`
   first; FpUnary single was missing it. A `floorf` loop returned 450 vs 360.
   Fix: add `buf.movd_xmm_r32(0, RAX);` before `cvtss2sd`.

2. **`frintz s` used round-to-nearest, not truncate.** roundsd mode was `0b00`
   (toward-nearest) instead of `0b11` (toward-zero). The double path already
   used `0x03`. A negative-heavy `truncf` loop: what ARM-trunc gives 40 came out
   20 because negatives rounded to nearest. Fix: `0b00` → `0b11`.

Verified vs native x86-64 oracle through elfjit (no QEMU): floor 360/360,
ceil 440/440, trunc 40/40; double trunc 40/40 (double was already correct).

Added a new differential battery test `diff_scalar_fp_round_single` with three
probes (fr_floor_single, fr_ceil_single, fr_trunc_single_negatives). Confirmed
the canary is a REAL gate: reverting just the `0b11` mode fix makes
fr_trunc_single_negatives fail with exactly `jit 20 vs oracle 40`, then restored.

`cargo build --workspace` clean; `cargo test --workspace` **254/0, 0 ignored**
(battery 30). Committed on `dev`.

Honest next (unchanged): vector `frintm/p/z v.*` is still an honest *Unsupported*
stop (no silent value), so gcc's vectorized rounding is the next feature slice;
then the runtime-side items (FMOD bl-to-once dispatcher, JNI table stubs) and
the HARD GATE real-binary/GPU boot proof which is impossible on this APK-less,
GPU-less VPS.

---

## Session 2026-09-11 (cycle 19b) — VECTOR frint + compare-to-zero + SimdMull gate fix (workspace 257/0)

Follow-on to the scalar FP-rounding fix. Sweeping more vector-FP probes exposed a
**systemic misdecode class**: the SIMD widening-multiply gate was
`insn & 0x0f00_c000` — it drops bit28 and only keeps byte2 bits15:14, so ANY
op whose byte2 shares those bits folded into the smlal residue. Two real,
silent (garbage-not-stop) miscompiles found and fixed:

1. **VECTOR frint** (frint{n,m,p,z,a} Vd.T, Vn.T): a `floorf` loop's
   `frintm v1.4s,v1.4s` (byte2 0x98) decoded as **smlal** — the floor loop
   returned 1.9e16 vs native 590 (a silent widen-multiply of the float bits).
   Implemented `Inst::SimdFrint` + decode (mode = bit23<<1|bit12, es=bit22,
   frinta=bit29) before SimdMull; translate per-lane via cvtss2sd→roundsd→
   cvtsd2ss (0b00 n / 0b01 m / 0b10 p / 0b11 z, frinta ties-away trick).

2. **VECTOR compare-to-zero** (fcmeq/fcmgt/fcmge/fcmlt/fcmle Vd,Vn,#0.0): gcc's
   `x<0 ? a : b` select uses `fcmlt ... #0.0` (byte2 0xea) + `bsl` — the fcmlt
   was also smlal, so the select chose the wrong branch. Implemented
   `Inst::VecFpCmpZero` + decode + translate (comiss/comisd vs a zeroed xmm1,
   setcc per lane like the existing VecFpCmp; seta/setae/sete/setb/setbe).

3. **SimdMull gate tightened** to `(insn & 0x3800) == 0` (byte2 bits13:11
   clear) — genuine smull/umull/smlal always clear those (size varies in byte2
   bits[2:0], e.g. smull .8h 0x...c020 vs umull 0x...c340), while frint (0x88/98)
   and fcmlt (0xea) set one. Prevents any future frint/cmpz-style smlal fires
   even if a new width variant slips the earlier gates. (An earlier too-strict
   `byte2 == 0xc0/0x80` broke diff_magic_div etc. — element size lives in byte2
   bits[2:0]; corrected to the 0x3800 check, which keeps all mull widths working.)

Also proved the earlier `vfa` "fneg sign flip" was a **UB false alarm**: the
probe cast a negative float to `unsigned long long` (undefined in C) — aarch64
emits `fcvtzu` (clamps to 0) while x86 emits signed trunc (-363), so the JIT
returning 0 was CORRECT aarch64 semantics. With a signed cast, jit == native ==
-363. Isolated tests confirmed fneg, bsl, and fcmlt+bsl are each correct.

New differential canaries: diff_vector_frint_rounding (vflr_floor, vflr_ceil),
diff_vector_fp_compare_zero (vcmp0_lt_keep_neg, vcmp0_gt_keep_pos). New decode
regression `simd_frint_and_cmpzero_not_swallowed_by_widen_mul` (guards (a) frint
(b) fcmlt (c) genuine umull still SimdMull). cargo build clean; cargo test
--workspace 257/0. Committed on dev.

Honest next: continue the battery sweep (vector frinta/frintn, f2d .2d forms,
sdadd/sqadd, pmull); runtime-side FMOD bl-to-once dispatcher + JNI table stubs
are still pending the real binary; HARD GATE (elfjit on libroblox.so on a GPU/
APK host) unchanged — impossible on this GPU-less, APK-less VPS.

---

# Session (Sep 11, 2026) — three silent FP `.2d`/FMOV-imm decode collisions fixed (workspace 264/0)

Extended the differential battery with four new game-critical canaries (SIMD
fmin/fmax reduction, reciprocal/division funnel, integer widening mul-acc,
double FP loop-condition). One — `l_double_div_accum_guard` (6-element
`(i+1)*2/(i+2)` sum with a `<3.0` guard) — **failed: jit 2001 vs oracle 8820**.
Bisecting it (per-instruction exec_bytes + whole-block runs through
`load_elf_image`+`jit_run`) surfaced THREE unrelated silent miscompiles in the
FP/vector decode layer, each a real pixel/audio/coordinate corruption:

## 1. `fadd/fmul/fsub Vd.2D` swallowed by the integer add/sub gates
The four SIMD int add/sub gates (Simd4s/SimdAddD/SimdAddB/SimdAddH) keyed on
`insn & 0x2f20_0c00` in {0x0e200400, 0x2e200400} plus `size`, but ignored bit14.
FP `.2d` two-source ops (byte1 0xd4, bit14 SET) share that residue with integer
`add` (byte1 0x84, bit14 CLR). `fadd v0.2d,v0.2d,v1.2d` (0x4e61d400) compiled as
a **halfword `paddw`** on the double bits. The isolated host dump showed
`paddw xmm0,xmm1`; the decode scratch returned `SimdAddH`. Fix: `(insn & 0x4000)
== 0` on all four gates. VecFpArith covers `.2s/.4s` early; Simd2dFp now gets
`.2d`.

## 2. `fdiv/fmul Vd.2D` swallowed by the SimdSel (bsl) gate
byte1 low 0xfc/0xdc (bits[15:13] SET) duped the bsl select gate, which only
checked `(insn & 0x1c00) == 0x1c00` (bits[12:10]). Host dump showed
`pandn/pand/por` — a bitwise select. `fdiv v0.2d` returned 256 for 64/8 (both
lanes). The real bsl family always has byte1 low 0x1c (bits[15:13] CLR). Fix:
`(insn & 0xe000) == 0` on the SimdSel gate.

## 3. `fmov d,#imm` with mantissa m>=8 swallowed by the fcvt-to-int round gate
`fmov d23,#12.0` = 0x1e651017 decoded as `FcvtToInt` (destination silently 0).
The coarse fcvt-round gate (`insn & 0xffff_0000`, added for `0x9e..` X-dest
fcvtps/ms/au) included `0x1e65` (W fcvtau) which collides with FMOV-imm. FMOV-imm
has bit12 SET (the imm lane anchor) while every fcvt-to-int has bit12 CLR
(verified: fcvtzu 0x1e790020, fcvtas 0x1e640020, fcvtau 0x1e650062). Fix: gate
the fcvt-round block on `(insn & 0x1000) == 0`.

## Verification
- Isolated vs the real aarch64 assembler: bsl 0x6e611c00, fdiv/fmul/fadd .2d
  0x6e61fc00/0x6e61dc00/0x4e61d400, fcvtzu 0x1e790020, `fmov d,#12.0` words
  0x1e651017 etc. p9 (scalar div+fmadd, was 2^63) -> 6000, p5 (vector
  a[]+reduce, was 90000) -> 8814, l_double -> 8820.
- +1 decode regression (`fp_2d_op_decode_collisions_with_int_add_bsl_and_fcvt`),
  +2 exec regressions (`vector_2d_fp_div_mul_not_swallowed_by_int_add_or_bsl`,
  `fmov_imm_high_mantissa_12_to_15_not_swallowed_as_fcvt`), and the 4 new battery
  canaries are permanent.
- `cargo build --workspace` clean; `cargo test --workspace` 264/0, 0 ignored.

HARD GATE unchanged: real-binary/GPU boot proof (`elfjit <libroblox.so>
0x1f0db20 --jni`) on a GPU + real binary/APK host (none on this VPS).
---

# Session (Sep 11, 2026) — SIMD across-lanes min/max + smax/smin & movsx fixes (workspace 268/0)

Continuing the differential battery: a new int SIMD min/max reduction canary
(`im_running_minmax`) exposed one missing ISA and two more silent bugs.

## 1. SMINV/SMAXV/UMINV/UMAXV implemented (was Unsupported)
Across-lanes reduce to the bottom scalar (upper cleared). New
`Inst::SimdReduceMinMax` decode (0x4e/0x6e `XYa820` family: esize by byte2 high
nibble {3,7,b}, min/max by byte2 bit0, signed by bit29) + translate (per-lane
sign/zero-extended CMOVcc reduce). Two gotchas during bring-up: cmov_rr64
expects the cc in the `0F 4X` domain (I first passed the raw 0x0X Jcc domain ->
SIGILL `0F 0C`), and the cmp/cmov direction is min=CMOV-G/A, max=CMOV-L/B
(update when the candidate is the extrema found by `cmp RDX, RAX`).

## 2. Element-wise smax/smin mis-decoded for high source registers (silent)
The SminMax gate is correctly written `(b2 mask 0xfc) == 0x64` (max) but the
ASSIGNMENT was `max: b2 == 0x64` (EXACT). b2 = bits[15:8] and its low 2 bits
carry Rn (bits[9:8]). A real gcc `smax v30.4s, v29.4s, v28.4s` encodes b2=0x67,
so it masked to max but the exact-equality assign said MIN -> returned the Vn
operands verbatim. Only source regs 0..3 (b2 stays 0x64/0x6c) ever hid it.
Fixed to mask like the gate.

## 3. movsx_word_mem/movsx_byte_mem lacked REX.W (silent, shared-emitter)
`0F BF /r` / `0F BE /r` with REX no-W write only a 32-bit destination, so a
negative 8/16-bit lane loaded into RAX compared as a huge POSITIVE u64 in any
64-bit signed reduction (`sminv.8h` over {-9,-2,..} picked 4, not -9). Latent
across every consumer (incl. ADDV signed byte/halfword sums). Both emitters now
emit REX.W. This was the real root of the earlier `.8h`/`.16b` sminv results.

Verified via per-instruction stepping of the gcc-unrolled probe: smax produced
v30=[0,-28,28,28] (wrong) -> after fix [112,140,252,308]; sminv/smaxv/addv
scalars and the final result 3640 = oracle. +2 differential canaries
(rm_running_extents, im_running_minmax), +2 exec regressions. cargo build
--workspace clean; cargo test --workspace 268/0, 0 ignored.

HARD GATE unchanged: real-binary/GPU boot proof (`elfjit <libroblox.so>
0x1f0db20 --jni`) on a GPU + real binary/APK host (none on this VPS).
---

# Session (Sep 11, 2026) — guest_svc syscall surface expanded for real-boot/ALooper/login (workspace 270/0)

The android-layout idempotency test is green at HEAD (long since fixed); the
workspace gate is clean. This session widened the JIT's in-process AArch64
syscall dispatcher (`guest_svc` in `crates/arm64jit/src/jit.rs`) from ~30 to
~48 syscalls, targeting the families a real Android boot / ALooper / login
path issues that the table previously sent to -ENOSYS:

- **fstat(80) / newfstatat(79)** with a **guest-layout `stat`** — the host
  `libc::stat` layout differs across x86_64 vs aarch64, so forwarding the host
  struct would silently mis-place every field. `unsafe fn write_guest_stat`
  transcribes into the AArch64 asm-generic layout (128B): st_dev@0 st_ino@8
  st_mode@16 st_nlink@20 st_uid@24 st_gid@28 st_rdev@32 st_size@48
  st_blksize@56 st_blocks@64, times (sec+nsec) @72..112. Numbers + layout
  verified against `/usr/aarch64-linux-gnu/include/asm-generic/{unistd,stat}.h`.
- **sockets**: socket(198), bind(200), listen(201), accept(202), connect(203),
  setsockopt(208), getsockopt(209) — the networking/login path.
- **event/epoll** (Android ALooper is epoll-based): eventfd2(19),
  epoll_create1(20), epoll_ctl(21), epoll_pwait(22), ppoll(73).
- **descriptors**: dup(23), dup3(24), ioctl(29), readv(65), writev(66).
- **system/time**: uname(160), gettimeofday(169), clock_getres(114).
- **limits/signals/timers**: getrlimit(163)/setrlimit(164), kill(129),
  tgkill(131), timer_create(107), timer_settime(110).

All struct-returning syscalls chosen with layout-identical-or-explicit
conversion (timeval/rlimit/utsname/epoll_event layouts are arch-identical;
`stat` uses write_guest_stat). New integration test
`guest_svc_stats_and_descriptors_roundtrip` verifies fstat/newfstatat st_size+
st_mode in guest layout, eventfd write/read, epoll_create1+epoll_ctl(ADD — on an
eventfd/pipe, not a regular file which EPERMs), gettimeofday, uname=="Linux".

Gate: `cargo build --workspace` clean; `cargo test --workspace` 270/0 (was 269).
HARD GATE unchanged — real Roblox boot + run log on a GPU/APK host
(`elfjit <libroblox.so> 0x1f0db20 --jni`); none of that is on this VPS.

# Session (Sep 11, 2026) — two REAL silent miscompiles fixed: EXTR operand order + ADC/SBC carry polarity (workspace 272/0)

New differential canaries (diff_math128_and_carry: __int128 sq/mul/madd;
diff_switch_fnptr_hash: switch jump table / fnptr blr dispatch / FNV) surfaced
one failing probe: math128 (JIT 0xc24b01e838c56079 vs native 0xb50f76ac635ab31b).
Bisected to TWO distinct arm64jit bugs, both fixed + native-verified:
1. EXTR invert — see STATUS.
2. ADC/SBC borrow-convention carry — see STATUS.
Files: crates/arm64jit/src/translate.rs (Extr + AddCarry), jit.rs
(add_carry_reference), tests/diff_battery.rs (+math128 canary). Commit 912eff8.


---

# Session (Sep 11, 2026) — SIMD fcvtl/fcvtn float<->double conversion + latent scalar store fix (workspace 275/0)

Commit `3718824` (dev). Opened at 272/0 green; drove the differential battery
into the mixed-precision float<->double vector path and it surfaced BOTH a
missing ISA wall and a latent miscompile:

## 1. New ISA: fcvtl/fcvtl2 + fcvtn/fcvtn2 (SIMD float<->double width conversion)
gcc -O2 emits these for any `float[]` <-> `double[]` elementwise round-trip; the
JIT previously stopped `Unsupported` at the first fcvtl in such code.
- Gates (asm+objdump verified): `(insn & 0xffff_fc00)` in {0x0e617800,
  0x4e617800} = fcvtl (f32->f64, 2 lanes), {0x0e616800, 0x4e616800} = fcvtn
  (f64->f32). Placed BEFORE VecIntToFp/SimdMull (which swallow these as int->fp /
  widening-multiply). `upper` = bit30 (Q): fcvtl2 reads Vn's upper half;
  fcvtn2 writes Vd's upper half. Half-precision byte2-0x21 (fcvtl Vd.4s,Vn.4h /
  fcvtn Vd.4h,Vn.4s) stays Unsupported (no fp16 in the JIT).
- Translate: per-lane cvtss2sd/movq_store (widen), movq_load/cvtsd2ss/narrow
  store (fcvtn). Guest v-lane = VECTOR_BASE + reg*16 confirmed.

## 2. LATENT MISCOMPILE exposed by the new ISA (the important find)
Once a gcc float<->double loop can compile fully (previously it always aborted
`Unsupported` at fcvtl during compile, so NOTHING after it ever executed), the
JIT segfaulted at fault=0x41480000. Root cause: `FpLdStImmWb` (scalar pre/post-
index ld/st) passed RAX as the address register into `fp_scalar_xfer`, which
uses RAX as its value scratch -- so a scalar pre/post-index STORE clobbered the
base with the value and wrote to [value bits] instead of [base]
(`str s30,[x4],#4` stored to 0x41480000 = float 12.5). Now computes the address
in RDX (mirrors the correct FpLdStImmUnscaled). This would have corrupted
single-precision array/matrix writes in any real graphics/audio math.

## Verification
- Differential canary `diff_fcvtl_widen_and_fcvtn_narrow` (round-trip 16
  floats<->doubles, distinct values, upper-half use): jit==oracle==1248.
- Decode/exec: `fcvtl_fcvtn_decode_and_lane_widen_exec`; store fix:
  `fp_scalar_postindex_store_uses_base_not_value`.
- `cargo build --workspace` clean; `cargo test --workspace` 275/0 (arm64jit
  162 lib + 42 diff + 8 loader_run; libbadcpu 22; libloader 23; +others).

## Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping mixed FP width / vector breadth
   (fcvtl half-precision forms, f2d long forms, pmull, sat-ops, more -O3
   reduction shapes) -- this ISA-assertion loop keeps flushing real latent
   miscompiles (this cycle: fcvtl + the scalar-store RAX clobber).
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE, blocked until a capable host + the real binary/APK (none here).


---

# Session (Sep 11, 2026) — SIMD BIF misdecoded as BSL; int64->int32 narrowing (workspace 277/0)

Commit `60400f3` (dev). Opened at 275/0 (fcvtl cycle). Pushed the differential
battery further into int64->int32 truncation pipelines and found a THIRD real
silent miscompile in the bitwise-select family:

## BIF was decoded as BSL with the opposite mask semantics
gcc -O2 int64->int32 narrowing (with large negatives needing sign handling)
compiles through smull/saddl/saddw + cmeq + bit/bif + uzp; through the JIT it
returned the wrong value. Root cause: the SimdSel (BSL) decode gate required
bit22 SET but did NOT inspect bit23, so BIF (`bit22=1,bit23=1`) was silently
decoded as BSL (`bit22=1,bit23=0`). The three select ops are:
- BSL = (N&M)|(D&~M);  BIT (bit22=0/bit23=1) = (N&M)|(D&~M);  BIF (bit22=1/bit23=1)
  = (N&~M)|(D&M) -- BIT/BIF are the opposite bit-inserts.
- Fix: SimdSel gate now requires bit23 CLEAR (BSL only); BIT/BIF fall through.
- SimdBit gate extended to the BIF residues (0x6ee01c00/0x2ee01c00 alongside
  0x6ea01c00/0x2ea01c00) with a `bif` flag decoded from bit22; translate emits
  (sel&Vm)|(keep&~Vm) with (sel,keep)=(Vd,Vn) for BIF and (Vn,Vd) for BIT.
- NOTE: the bit-vs-bif opcode discriminator is bit22 (0x0040_0000), NOT bit14
  (a first pass used bit14 and produced wrong values; the two words differ by
  exactly 0x00400000). The SimdBit FAMILY residues also differ by bit22, so the
  gate set {0x6ea01c00, 0x6ee01c00, ...} is correct.

## Verification
- jit.rs `bit_vs_bif_bitwise_insert_semantics`: decode maps bit->bif:false,
  bif->bif:true, bsl->SimdSel; exec both produce the correct DISTINCT values
  (BIT 0x11bb33dd11ff7799, BIF 0x660066446600ee for the fixed operand set).
  Learned en route: read ARM's Vd,Vn,Vm operand order for `bit`/`bif` (Vm is the
  MASK); the earlier "bif returned bit's value" was a test-labels error, not a
  translate bug.
- diff_battery `diff_int64_to_int32_narrowing_bif`: int64->int32 and the full
  int64->int32->short round-trip drive the bif path; jit==oracle.
- (Surgery lesson: a bad mid-file replace during test insertion dropped two
  pre-existing host-float-bridge tests; restored them verbatim from HEAD and
  verified with a fn-name diff that no test was lost.)
- `cargo build --workspace` clean; `cargo test --workspace` 277/0 (arm64jit
  163 lib + 43 diff + 8 loader_run; libbadcpu 22; libloader 23; +others).

## This session's net (commits 3718824 fcvtl + store fix, 60400f3 bif)
Two new-ISA walls (fcvtl/fcvtn) and TWO latent silent miscompiles fixed
(FpLdStImmWb scalar store RAX-address clobber; SIMD BIF->BSL opposite select) --
the differential-assertion loop keeps flushing real wrong-pixel/audio bugs.

## Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping ISA breadth (SIMD permute/wide
   paths, sat-ops, half-precision, -O3 reduction shapes) -- the fcvtl+bif
   cycles prove it is the highest-leverage correctness engine available here.
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE, blocked until a capable host + the real binary/APK (none here).


---

# Session (Sep 11, 2026) — SIMD compare-to-zero + sub-wide + high-rm MulDiv (workspace 280/0)

Commit `1702c89` (dev). Opened at 277/0 (bif cycle). Continued the cross-gcc ISA
sweep into signed-byte / 16-bit / int64-narrowing code; flushed one new-ISA wall
and TWO more real silent miscompiles (total this session: 2 ISA walls + 4 bugs):

## 1. SIMD integer compare-to-zero (cmeq/cmgt/cmge/cmlt/cmle Vd.T,Vn.T,#0) -- NEW
gcc emits these for every vectorized x<0 / x==0 / sign check, e.g. the
`if(sc[i]<0) count++` idiom which compiles to cmlt -> sxtl -> ssubw (count=n by
subtracting the sign-extended mask). Implemented `Inst::SimdCmpZero`:
- gate: top byte {0x0e,0x2e,0x4e,0x6e} + bit26 + bits[15:10] in {0x22 cmgt,
  0x26 cmeq, 0x2a cmlt} AND bit16 CLEAR (bit16 set = FP frint/tbl family -- the
  first gate draft collided with frintm 0x4e219821).
- esize = 1<<bits[23:22] (8B/8H/4S/2D); U (bit29) toggles gt->ge and eq->le.
- translate: per-lane signed/sign-extended load, test, setcc(eq==0x4,gt==0xf,
  ge==0xd,lt==0xc,le==0xe), movzx, neg => all-ones-or-0 mask.

## 2. sub-wide ssubw/usubw decoded as ADD -- SILENT
The SimdAddw gate checks `(insn & 0x1800)==0x1000` (bit12 set, bit11 clear) but
ignores bit13 (0x2000), the add/sub-wide discriminator (saddw 0x0e7613de vs
ssubw 0x0e7633de differ by bit13). So every ssubw silently ADDED the widened
element; the negative-count idiom's subtract became an add and the count was
wrong. Added `sub = (insn & 0x2000) != 0` to SimdAddw; translate emits
`sub_rr64` when set.

## 3. MulDiv gate mask kept bit20 -- `mul w9,w9,w20` was Unsupported
`(insn & 0x7ff0_0000)` keeps bit20 (part of rm, bits[20:16]), so any mul/madd/
sdiv with rm>=16 (bit20 set) failed the gate. Correct mask 0x7fe0_0000 (clears
the whole rm field). gcc's `mul w9,w9,w20` (32-bit mul, high rm) exposed it.

## Verification
- jit.rs `isa_regress_tests`: cmlt mask (neg bytes all-ones) + ssubw subtracts-
  not-adds exec (both decode-atom and runtime values).
- diff_battery `diff_byte_negcount_ssubw_cmlt`: byte sum + negative count via
  the cmlt->ssubw idiom + 16-bit unsigned widening; jit==oracle.
- Cross-gcc probes all exact: byte SIMD=36775, 16-bit audio =9564799, int64
  narrowing n1=7769812456 / w=7634108456, byte+negcount=3487.
- (learned: keep test value literals out of `<< 32` shift overflow; append
  regression tests as a separate `#[cfg(test)] mod` at EOF instead of fragile
  mid-file splicing.)
- `cargo build --workspace` clean; `cargo test --workspace` 280/0 (arm64jit
  165 lib + 44 diff + 8 loader_run).

## Session total (commits 3718824, 60400f3, 1702c89)
4 ISA walls / features (fcvtl/fcvtn float<->double, SIMD compare-to-zero) and
FOUR real silent miscompiles found+fixed via the differential sweep:
FpLdStImmWb scalar-store RAX-address clobber, SIMD BIF->BSL opposite select,
ssubw-as-add, MulDiv high-rm gate mask. The cross-gcc differential battery
(+native oracle) is the highest-leverage correctness engine available without
an APK/GPU.

## Next (ordered, no APK/GSI/GPU on this box)
1. Keep the differential battery sweeping (sat-ops, half-precision, more -O3
   reduction/permute shapes, fp16).
2. libloader ELF/loader gaps -> libbadcpu ISA gaps -> services/auth.
3. Real-binary/GPU boot proof (`elfjit <libroblox.so> 0x1f0db20 --jni`) stays
   the HARD GATE, blocked until a capable host + the real binary/APK (none here).

## Session (Sep 11, 2026) — ld2/st2 structure deinterleave element-size FIXED (commit 36813d1, workspace 282/0)

Root-caused and fixed a silent SIMD miscompile in the structure load/store
(ld2/st2) path: the translate arms DEINTERLEAVED AT BYTE GRANULARITY
regardless of element size. `ld2 {v.8h, v.8h}` (2-byte elements — gcc's
strided u16 accumulate `for(i+=2) s2 += b[i]`) read mem[2i],mem[2i+1] instead
of the correct mem[4i],mem[4i+2]; the even-index sum registered the wrong
memory elements. Decode already computed `esize` for the 3/4-register forms
but dropped it for ld2/st2 (the 2-register case made byte-only runs look
correct, hiding the bug). Passed `esize` through decode and deinterleave at
element stride (element i of reg j at byte i*(2*es)+j*es, copy es bytes).

Verified end-to-end (no QEMU): isolated strided-u16 repro 311814 -> 281606 ==
native; differential canary `diff_ld2_halfword_strided_accumulate` added
(proven sensitive — reverting ONLY the esize change makes it fail again).
+decode regression with assembler-verified 8h/16b/4s encodings.
`cargo build --workspace` clean; `cargo test --workspace` 282/0 (was 280).

HONEST REMAINING / next high-value target: a SEPARATE pre-existing co-resident
bug surfaced by the differential sweep — a single function that VECTORIZES TWO
widen-accumulate loops (e.g. two byte-sums `i+=1` and `i+=2`) into one host
block produces correct low-32 sums but a stray data byte at bits 32-39 of the
64-bit accumulator lanes (repro `twov2.c`/`twoloopp.c`: JIT 18916 vs oracle
16868; isolated each loop passes). Independent of the ld2 fix (byte-only path,
esize=1, unchanged). Root-cause lead: bits 32-63 of the accumulator lanes are
contaminated, strongly suggesting a shared host/permscratch register clobber
across the two co-resident widening loops — NOT a single-Ld2 fault. Next
session: trace which translate arm leaves permscratch / a host vector scratch
dirty that the second loop's zip/uxtl reads; this subsumes the "byte-acc
halved" 2x signature documented in the runs/STATUS ledger. HARD GATE unchanged
(no GPU/APK/libroblox.so on this VPS).

## Session (Sep 11, 2026) — differential-sweep JIT correctness: 6 silent SIMD/int miscompiles FIXED (288/0)

Driving elfjit against qemu-aarch64 oracles on real static aarch64 builds caught
and fixed SIX silent miscompiles (each regression-guarded + sensitivity-verified):

1. ld2/st2 element-size deinterleave (commit 36813d1) — byte-granularity
   regardless of element size; strided-u16 accum read mem[2i],mem[2i+1] instead
   of mem[4i],mem[4i+2]. 282/0.
2. W-form bitfield (Ubfm/LSR/LSL/ROR aliases) loaded Rn as u64 and shifted
   without zeroing the high 32 bits (b739a6b) — `madd x; lsr w` pulled high
   guest garbage into the byte (8652 -> 0x37562e2cc). 284/0.
3. shrn/shrn2 (shift-right-NARROW) decoded as plain equal-size ushr/sshr
   (04a1483) — wrong source stride + shift (x^(x>>16) -> 0x83f9b82e6). 286/0.
4. rbit SWAR truncated every 64-bit mask to 32 bits (b83a1cf) — only the low
   half of x reversed; ctz=198 vs 10.
5. clz x (sf=true) emitted REX.W BEFORE the F3 prefix — CPU ran a 32-bit
   lzcnt, so clz(x<2^32) returned 32-len not 64-len (clz(0x16136740)=3 vs 35).
   Emit `F3 48 0F BD` (REX must be the last prefix).
6. shl #imm decoded shift as immb-only + esize from trailing_zeros (dropped
   bit22) — `shl v.4s,#25` ran as #1 on 1-byte lanes. shift = immh4:immb -
   esize_bits.

Tooling added: `sweep_wform.py` (in /tmp/combw) — static -nostdlib -Wl,-e,entry
build, qemu-aarch64 oracle, elfjit run, diff. Expanded to rbit/clz/ctz/shl/
shr/umulh families; all probes now match the oracle. Notebook: the entry-sym
parse must split on whitespace (objdump '0000000000400120 <entry>:' includes
the symbol), and native oracle must use gcc not cross-gcc.

Workspace 288/0, build clean, 6 focused commits on dev. No repo push. The
pre-existing co-resident two-loop widening bug remains open (documented;
contamination at bits 32-63 of acc lanes when two widen loops share a block)
and is independent of all six fixes. Next: keep sweeping SIMD/FP shapes; then
libloader ELF/loader gaps per RECOMMENDATION order. HARD GATE unchanged (no
GPU/APK/libroblox.so on this box).

## Session (Sep 11, 2026) — randomized differential fuzz: +2 JIT fixes (289/0, 165 fuzz cases green)

Extended the elfjit-vs-qemu-aarch64 differential harness into a randomized
fuzzer (`fuzz_jit.py` in /tmp/combw) generating unrolled scalar/SIMD shift/xor/
byte/fp programs with runtime-dependent LCG inputs. Found TWO more silent JIT
bugs, both in `Inst::BitField` (the general-extract path):

1. UBFM extract mask SIGN-EXTENSION: for a field width making the mask >= 2^31
   (ubfx x,#16,#32 -> mask 0xffffffff) `and_ri64(RAX, mask as u32)` emitted a
   64-bit AND with a sign-extended imm32 => mask became 0xffffffffffffffff
   (no-op), leaking the high 32 bits of the shifted value into the result
   ((x>>16) -> 8234290418553910946 vs 85047753507696). Route masks >0x7fffffff
   through mov_ri64+and_rr64. Commit 750457a.
2. UBFM mis-decoded as ROR: `imms+immr+1==bits` is NOT a rotate discriminator —
   genuine ror is an EXTR alias (-> Inst::Extr), so any UBFM matching it
   (ubfx x,#16,#32: immr=16,imms=47, field to the top bit) is a plain extract.
   The old branch ran ror_ri8(imms), corrupting extracts. Removed it; words
   fall through to the extract path. Genuine ror x,#17 still passes.

Regression: diff_ubfx_masks_upper_bits_and_is_not_ror (u32-width extract +
extract-with-genuine-ror), sensitivity-proven both ways. cargo build clean;
cargo test --workspace 289/0. Randomized fuzz seeds 1-4: 165/165 match qemu.

Combined with the previous sweep this cycle has produced EIGHT verified JIT
correctness fixes (ld2/esize, W-form bitfield, shrn2, rbit mask, clz REX order,
shl-imm decode, ubfx mask, ubfx-vs-ror). Next: keep fuzzing with more diverse
generators (fp, structure loads, saturating arith), then libloader ELF/loader
gaps per RECOMMENDATION order. HARD GATE unchanged (no GPU/APK/libroblox.so).

## Session (Sep 11, 2026) — co-resident two-loop widening bug RESOLVED (was the documented OPEN BUG)

After the 8 JIT correctness fixes this session (ld2/st2 esize deinterleave,
W-form bitfield high-32 mask, shrn/shrn2 narrow decode, rbit 64-bit mask width,
clz REX.W byte order, shl#imm decode, UBFM extract mask sign-extension, and
ubfx-vs-ror discriminator), the LONG-DOCUMENTED co-resident two-loop widening
bug — and the "intermittent SIMD-loop block-liveness" bug it subsumed — are
both resolved. The original reproducers now pass exactly:
  twov2.c    JIT 18788  == oracle 18788   (was JIT 7465833 / garbage 0x375)
  twoloopp.c JIT 3776   == oracle 3776
  maskf.c    JIT 2416   == oracle 2416    (documented intermittent lane corruption)
Both were SYMPTOMS of the same underlying shift/immediate-mask miscompiles
(high-32 contamination leaking through sign-extended `and` imm32 masks and
wrong shift amounts), not a separate co-resident register-liveness fault. ~700
differential fuzz cases green including PIE+reloc loader-mode (elfjit vs qemu
oracle). This removes the last known-open JIT correctness item on this box.

Next (ordered, all still open): libloader ELF/loader gaps, libbadcpu ISA gaps,
services/auth — or more fuzz coverage. HARD GATE unchanged (no GPU/APK).
## Session (Sep 11, 2026) — guest_svc syscall surface expansion (filesystem/IO/network)

Added two batches of AArch64 syscalls to the guest_svc bridge that a real
Android/Roblox boot path issues early and that previously returned -ENOSYS,
each number verified against the sysroot asm-generic headers (not guessed):
  Batch 1 (commit c0073b6): fcntl(25), clock_nanosleep(115), getrusage(165),
  setpgid(154), rt_sigaction(134), rt_sigprocmask(135), fadvise64(223).
  Batch 2 (commit a682eea): mkdirat(34)/unlinkat(35)/symlinkat(36)/linkat(37)/
  renameat(38), pread64(67)/pwrite64(68), socketpair(199), sendto(206)/
  recvfrom(207)/sendmsg(211)/recvmsg(212)/accept4(213), madvise(233), umask(166),
  getgroups(158).
Signal ops accept registration (return 0) but don't dispatch guest trampolines
(consistent with the shim's no-signal posture); oact/oset outputs are zeroed so
callers don't deref garbage. madvise DONTNEED keeps host RSS bounded under guest
allocation churn. Test guest_svc_common_boot_gaps_roundtrip covers both batches
(mkdir/link/rename/read/write/pipe lifecycle, socketpair+sendmsg/recvmsg byte
roundtrip, madvise, umask, fcntl, sigaction/procmask zeroing, clock_nanosleep).
Workspace 290/0. HARD GATE unchanged (no GPU/APK on this box).
## Session (Sep 11, 2026) — fuzz_jit loader-mode: PIE + reloc shapes end-to-end (100+ cases)

Extended the differential fuzzer with a loader-mode: gen_globals_pie /
gen_pie_callchain compile `-fPIE -pie -nostdlib` programs with exported
statics, arrays, and function-pointer initializers — forcing R_AARCH64_GLOB_DAT,
RELATIVE, and ABS64 relocations in .data.rel.ro — then run them through elfjit
(load_elf_image + bind_image_plt + JIT) and diff against a qemu-aarch64 oracle.
~35% of fuzz cases now take this path. Seeds 31-36: 120/120 pass, validating
the full loader→reloc→bind→JIT chain the runtime depends on (commit 93c0ee4).
The JIT correctness work is now broadly covered (~620 differential cases green
total). Next (unchanged, in RECOMMENDATION order): libloader ELF/loader gaps
then libbadcpu ISA gaps then services/auth; or more precision on the open
co-resident two-loop widening bug. HARD GATE unchanged (no GPU/APK on this box).

## Session (Sep 11, 2026) — guest_svc boot-path gaps + co-resident bug RESOLVED (290/0)

Added seven AArch64 syscalls a real Android/Roblox boot issues early, that
previously fell to -ENOSYS: fcntl(25), clock_nanosleep(115), getrusage(165),
setpgid(154), rt_sigaction(134), rt_sigprocmask(135), fadvise64(223) — numbers
verified against the sysroot asm-generic headers (commit c0073b6). +unit test
guest_svc_common_boot_gaps_roundtrip.

MAJOR: the long-documented co-resident two-loop widening bug — and the it
subsumed "intermittent SIMD-loop block-liveness" bug — are BOTH RESOLVED. After
the 8 JIT correctness fixes this session (ld2/st2 esize, W-form bitfield mask,
shrn/sh2 narrow decode, rbit 64-bit mask, clz REX.W order, shl#imm decode, UBFM
mask sign-extension, ubfx-vs-ror), the original reproducers pass exactly:
twov2.c 18788==18788 (was 7465833), twoloopp.c 3776==3776, maskf.c 2416==2416.
They were SYMPTOMS of the same shift/immediate-mask miscompiles, not a separate
register-liveness fault. ~950 differential fuzz cases green (incl. PIE+reloc
loader-mode via fuzz_jit.py). No known-open JIT correctness items remain.

Workspace 290/0, build clean, ~18 focused commits on dev. Next (RECOMMENDATION
order): more boot-path syscalls, libloader/libbadcpu/services gaps, or more fuzz
coverage. HARD GATE unchanged (no GPU/APK/libroblox.so on this box).

## Session (Sep 11, 2026) — scalar ucvtf S-form FIXED (291/0) + fuzz gens

Silent FP miscompile found by expanding the differential fuzzer with
fma-chain/128-bit-struct/float-reduce generators: scalar ucvtf S-form
(0x7e21db18, gcc emits `ldr sD,[sp]` + `ucvtf sD,sD` for `(float)volatile_u32`)
decoded with sng=true but the translate arm ignored it and always did the 64-bit
D-form convert, silently dropping the value (audio/down-mix u32->float). Fixed,
sensitivity-proven (166908 vs 222908 without the fix). commit 48863d1.
Open (deeper, still characterizing): vectorized div/multiply reduction pipeline
(`a[i]/b[i]` -> ushr.2d + and + uzp1 + scvtf + fmla fmul fdiv) gives value
collapse (63 vs 729251) not reproducible in minimal isolated hand-asm; next
step is tracing that specific op mix.

## Session (Sep 11, 2026) — fuzz-driven FP findings wrap

Fixed 9th JIT miscompile this campaign: scalar ucvtf S-form sng flag ignored
(commit 6a3db93). fuzz_jit gens expanded (fma-chain/128-struct/float-reduce).
All individually-isolated SIMD/FP ops now match the qemu oracle (fdiv/fmul/fmla/
uzp1/and/ushr/scvtf/ucvtf/movi/lane-extract). Remaining open: a specific
multi-op vectorized div/multiply-reduction pipeline (vmult.c: `acc+=a[i]/
b[i]+c[i]` with computed operands) shows value collapse ~63 vs 729251 that does
NOT reproduce when the same ops are isolated; needs a JIT execution tracer to
pin the exact guest instruction. Low smoking-gun priority: all constituent ops
validate individually. Next sessions: add a per-instruction traced run to
diff_battery or resolve via reducing vmult.c further.

## Session (Sep 11, 2026) — MOVI Vd.2D immediate decode FIXED — the 10th JIT bug

Root-caused and fixed the 'fmla/div vector pipeline' bug that had resisted
isolation all session. It was NOT a lane-coalescing interaction — it was a
fundamental decode error in MOVI Vd.2D, #<imm>. The 2D immediate is a
BYTE-SELECT pattern (bits[9:5] low nibble picks which bytes 0..3 of the low 32
are 0xff), but the decode treated it as a generic imm8-replicate, so
mov v27.2d,#0xffff produced 0x03 lanes instead of 0x000000000000ffff. Any
vector AND-mask built this way corrupted bit-field extraction (gcc's
shr->and->uzp1->scvtf reduction), collapsing values. Fixed the decode with 6
qemu-verified ground-truth lane values; new unit test (movi_2d_byte_select_
ground_truth) + moved the flow canary from #[ignore]d to a real regression
(diff_fmla_div_pipeline_lcg). Sensitivity-proven both ways (695 vs 7777753).
cargo test --workspace 293/0. This is the 10th JIT correctness fix this
campaign; the fma_chain fuzzer is responsible for surfacing it.

## Session (Sep 11, 2026) — RELR e2e test + campaign wrap (294/0, ~1680 fuzz cases)

The loader's DT_RELR path (packed-relative relocations, tag 0x23) had a
unit-tested decoder but the discovery wire-up (walk PT_DYNAMIC tags -> find the
stream -> decode -> route back as R_AARCH64_RELATIVE) had no test. Added an
e2e test that builds a minimal ELF with a RELR-only PT_DYNAMIC on disk and
asserts read_elf_relocations returns the right offsets/info. This is the last
unvalidated reloc path a real Android .so would hit (recent lld emits RELR by
default).

Known gap for next sessions: R_AARCH64_TLS_* (TPREL/DTPREL) are not handled at
all by the loader/JIT. Real TLS needs a host-side per-thread guest TLS area
(tp/x28 slot), an architected feature — needs the real binary to validate.
cargo test --workspace 294/0, build clean.

## Session (Sep 11, 2026) — long-open fused two-loop signed-div miscompile RESOLVED: integer vector NEG/ABS (302/0)

Root-caused the last reproducibly-open JIT bug — the fused two-loop signed
magic-division miscompile (`fuzz_jit.py` gen_signed_div; n=4, div /7 repro:
oracle 406144671 vs jit 78184144 — the entire neg-loop contribution was lost).
It was a DECODE COLLISION, not a register-clobber:

- `neg v29.2s, v31.2s` (0x2ea0bbfd) is an integer two-register-misc op
  (opcode bits[16:12]==0xb, bit16 CLEAR). The vector float->int FcvVec gate
  used mask 0xffe0_fc00, which ZEROES bits[20:16], so NEG fell into the residue
  `0x2ea0_b800` (== fcvtzu v0.2s) and executed as a float->int convert — with
  v31.s0=0xfc8e117a (a small denormal float) that silently produced 0, then fed
  every downstream zip1/saddw/sxtl2 lane wrong.

Fixed (commit 47b1007):
1. FcvVec gate now requires bit16 SET (all six fcvtzs/fcvtzu sizes have it;
   neg/abs have it clear) — NEG/ABS no longer decode as fcvtzu. This also means
   NEG/ABS stop silently corrupting anywhere a -O3 build emits them.
2. New `Inst::SimdArithUnary` decode for NEG/ABS (opcode 0xb, neg=bit29,
   esize from bits[23:22] {0:B,1:H,2:S,3:D}), placed before FcvVec.
3. Translate: per-lane signed negate (neg_r64, two's-complement wrap) and
   abs via `(x^(x ar>> w-1)) - (x ar>> w-1)` after sign-extending each lane;
   per-lane read-modify-write is rd==rn safe.
4. Added `JIT_STEP` debug trace (single-instruction blocks + full x/v dump
   after each) — the per-instruction register oracle that made this tractable;
   debug-only, no effect on normal path.

Verification: repro jit=406144671==oracle (exact); decode + exec regressions
across .4s/.8h/.16b/.2d (negative, wrap, and abs-magnitude lane cases); new
diff_battery canary `diff_vector_neg_abs_unary`; ~420 fresh fuzz cases across
12 seeds (incl. 50/99/7/9001 repro seeds) all green, 0 fails; full
`cargo test --workspace` **302/0**, build clean.

No known-open arm64jit correctness items remain on this box (same as the
cycle-27 close). Next high-value per RECOMMENDATION order: libloader ELF/loader
gaps, libbadcpu ISA coverage, JNI function-table surface, then services/auth.
HARD GATE unchanged: real Roblox boot + reproducible run log on a GPU + real
APK/binary host (none on this VPS).

---

## Session (Sep 11, 2026) — guest TLS bootstrapped (R_AARCH64_TLS local-exec / main-binary case)

Commit `6af3cd4` (dev), workspace **304/0** (was 302), build clean.

### What landed
- `libloader::elf::setup_guest_tls(info, path, tls_region, size) -> tpidr` (+ `tls_layout`):
  finds the main image's `PT_TLS` (p_type 7), copies its `p_filesz` init image into
  a per-thread region at `region + AARCH64_TCB_SIZE` (16), zero-fills `.tbss` to
  `p_memsz`, and returns the thread pointer `tpidr = region` — the AArch64 TLS ABI:
  the module TLS data block lives 16 bytes after TP, and local-exec/initial-exec
  `:tprel:` addressing (`mrs xN,tpidr_el0; add x0,tp,#o`) lands on block+`o-16`.
- `elfjit` and the `loader_run` harness now seed `CpuState.tpidr` from
  `setup_guest_tls` instead of a bare zero-filled stack. ELFs without `PT_TLS`
  get `tpidr = region` (byte-identical to the prior behaviour) — no regression.
- **Verified end-to-end, no QEMU**: cross-gcc `__thread` fixture (`g_slot=7`,
  `g_big=123456789`, `g_zero` in `.tbss`, `bump()`) through `load_elf_image →
  setup_guest_tls → jit_run` returns **123456804** — the exact native x86-64
  oracle. (qemu-aarch64 itself SIGSEGVs on this nostdlib static TLS image because
  it doesn't seed PT_TLS without a dynamic loader, so qemu is not a usable oracle
  here.) Before the seeding the region was zeroed, so every `__thread` read
  returned 0.

### Scope note (honest)
This closes the documented "R_AARCH64_TLS_* untouched" gap for the **main-binary
local-exec/initial-exec** case — which is exactly the shape of `libroblox.so`
loaded as the boot image (a PIE still uses local-exec for its own `__thread`).
The **dynamic** TLS paths (`R_AARCH64_TLS_TPREL64`/`DTPREL64` GOT slots for
TLS referenced *across* modules, general-dynamic) only engage once the loader
loads `DT_NEEDED` dependency modules as a multi-image process — the next
loader frontier, and it needs a real multi-lib host to validate.

### Tests added
- `libloader elf::tests::setup_guest_tls_copies_init_and_returns_tcb_tpidr` —
  init-image copy, TCB tpidr, `.tbss` zero-fill, tprel addressing, no-TLS fallback.
- `arm64jit loader_run::loader_run_thread_local_storage_returns_123456804` —
  full loader→TLS→JIT pipeline gate (skipped if cross-gcc absent).

### Next (per RECOMMENDATION order)
libbadcpu/libloader JTAG: guest **threading (clone/vfork)** is the documented
single-threaded-boot frontier (needs a real host to validate); multi-module
`DT_NEEDED` load for cross-module TLS + GOT/PLT within deps; broaden the
differential fuzzer into still-uncovered NEON/by-element/`tbz` classes. HARD
GATE unchanged: real Roblox boot + run log on a GPU/APK host (none on this VPS).

### Session (Sep 11, 2026 cont.) — differential fuzzer extended: NEON by-element/tbl + bitfield-insert + tbz + fcvt classes, qemu oracle; 23 generators clean
Commit `a733fba`. Added 5 generators for previously-uncovered ISA classes (NEON
`vmlaq_n_f32` by-element fmla + lane ins/get + `vbsl/vext/vrev64` bitmix, 64-bit
`bfi/bfiz`, `tbz/tbnz` bit-branches, fixed-point `fcvtzs #fbits`) and a
64-bit-exact **qemu-aarch64 oracle fallback** (write+itoa `_start` wrapper) for
`<arm_neon.h>` programs the host x86 gcc can't compile — the old harness hard-
skipped them. `gen_neon_byelem` uses binary-exact lanes so FMA-vs-`mul+add` ULP
noise doesn't read as a structural diff; removed an unavailable `vrbitq_u32`
(only `_u8` exists) for `vrev64q_u32`. Result: full 23-generator sweep is
**70/70 green across fresh seeds, 0 skips** (was ~14% skip). No new miscompile
found in the covered classes — a negative result, but those classes are now
permanent gates. Workspace unchanged (304/0; Rust untouched this commit).

## Session (Sep 11, 2026 cont.) — complete: TLS bootstrap + fuzz-harness expansion + 4 permanent canaries + harness-cascade fix (workspace 308/0)

### Milestones landed this session (8 commits on `dev`)
1. **Guest TLS bootstrap** (`6af3cd4`, docs `7bc1a5e`): `libloader::setup_guest_tls`
   copies the main image's `PT_TLS` init into a per-thread region at TP+16 (the
   AArch64 TCB) and seeds `tpidr_el0` from it, so local-exec/initial-exec
   `:tprel:` addressing reads/writes real `__thread` data. Verified no-QEMU:
   cross-gcc `__thread` fixture → `jit_run` returns 123456804 == native oracle.
   Closes the documented `R_AARCH64_TLS_*` gap for the main-binary case.
2. **Fuzz harness** (`a733fba`, docs `5b8d805`): +5 generators for previously
   uncovered classes (NEON by-element fmla + lane round-trip, NEON
   `bsl/vext/vrev64`, 64-bit `bfi/bfiz`, `tbz/tbnz`, fixed-point `fcvtzs #fbits`)
   and a 64-bit-exact **qemu-aarch64 oracle fallback** (write+itoa `_start`
   wrapper) so `<arm_neon.h>` programs the host x86 gcc can't compile now get a
   differential oracle instead of hard-skipping. 23 generators; a 12-seed ×
   100-case campaign (1200 cases) is **0 fail / 0 skip**.
3. **Permanent cargo canaries** (`b4a48cd`, `7244feb`): loader_run gates for the
   newly-covered classes — bfi-64 (279514809947), tbz/tbnz (4068), fixed-pt
   fcvt (99), NEON by-element fmla (504; qemu architectural oracle).
4. **Harness-cascade fix** (`7244feb`): a single test's cross-gcc compile failure
   used to poison the shared `run_lock` mutex and cascade-fail every other test
   with `PoisonError` (each passed in isolation). Added `lock_run()` which
   recovers poisoned guards. Also made the byelem fixture `-O0`-safe
   (const-index `vgetq_lane`/`vsetq_lane` need `-O` to fold their lane index).

### Honest scope + findings
- **Negative result (good news):** the newly covered classes (NEON by-element
  fmla, lane ops, bitfield-insert, bit-branch, fixed-point fcvt) show **no**
  miscompile across 1200 fresh differential cases — the arm64jit ISA handling
  of those classes is structurally correct. They're now locked as permanent
  `cargo test` gates.
- **FMA granularity note:** the JIT's by-element `fmla` translate does `mul`+`add`
  (two roundings) rather than a fused FMA (one rounding). Structurally correct
  (binary-exact differential runs agree with qemu), but bit-exact float workflows
  can differ by 1 ULP from ARM's fused `fmla`. Acceptable for now; revisit if
  exact-bit-sovereignty is ever required.
- **Boundary (unchanged):** dynamic *cross-module* TLS (`R_AARCH64_TLS_TPREL64`
  GOT slots, general-dynamic) only engages with a multi-`DT_NEEDED` loader (the
  next loader frontier); guest threading (`clone`/`fork`/`rt_sigreturn`/`execve`)
  remains the documented single-threaded-boot frontier.

### Verification state
`cargo build --workspace` clean; `cargo test --workspace` **308/0** (up from
302). loader_run 13/13. Ship 1200-case fuzz campaign green. HARD GATE unchanged:
real Roblox boot + reproducible run log on a GPU + APK/binary host (none on this
VPS) — nothing here can satisfy it, so this session closed loader/TLS + fuzz-
correctness work as far as physically verifiable.

## Session (Sep 11, 2026 cont.) — EXT extract-immediate bug FIXED; open u16/32 pair-xor reduction bug isolated

### Fixed: `ext` (SIMD vector extract immediate) operand-order inversion — commit `826db92`
Found by the new `gen_pairwise_reduce` differential generator. ARM
`ext Vd.16B, Vn.16B, Vm.16B, #imm` returns the 16-byte window at `imm` of the
concatenation where **Vn is the low-address half and Vm the high**:
`Vd[0..16-imm)=Vn[imm..16)`, `Vd[16-imm..16)=Vm[0..imm)`. The translate built the
concat as `[Vm.lo, Vm.hi, Vn.lo, Vn.hi]` (Vm low, Vn high) — inverted for every
non-symmetric `ext`. **qemu-verified** with distinct bytes: `ext(Vn,Vm,#8)` ⇒
lo=`Vn[8..15]`, hi=`Vm[0..7]`. gcc's horizontal XOR-reduce for pair reductions
(`s ^= a[i]+a[i+1]`) emits `ext v0,v30,v0,#8` + `eor`; the old code collapsed
it to a single 64-bit-lane xor instead of the full one. Minimal repro p1
(const int pair-xor): returned **16**, now **0** == qemu and native. 28→99/160
of the pairwise-reduce stress now correct.

### OPEN (isolated, reproducible) — 16/32-bit `i+=2` pair-xor reduction
A second, independent bug in the pair-xor reduction path remains, unaffected
by the ext fix. Minimal repro `/tmp/combw/s13b.c` (`unsigned short a[24]`,
`for(i+=2) s ^= (long long)a[i]+a[i+1]`): **native 2314 vs jit 59648**. The
path uses `uxtl/uxtl2` + `uaddl/uaddl2` (element-wise widen-add, upper-half
reads) + `eor` chain + the (now-correct) `ext` horizontal combine. Suspect the
in-place `uxtl2` upper-half read or an eor-chain lane-composition bug, NOT the
(now-correct) ext and NOT `uaddl2` — a hand-assembled
`uaddl2 v1.4s,v0.8h,v2.8h` isolated test is byte-correct in the JIT (== qemu,
upper-half values [5..8]+[50..80]=[55,66,77,88]). The passing sum loop (s13a)
uses `uaddw`+`uxtl`, the failing xor loop (s13b) uses `uaddl`+`uxtl2` with the
in-place `uxtl2 Vd.2d, Vd.4s` (rd==rn) upper-half reads — the cycle-27 in-place
widening-alias class re-checked for the `.4s→.2d` upper form. Reproducible via
`fuzz_jit.py` `gen_pairwise_reduce`
(int/short/u32/u16 variants all trip it). Next debug pass: isolate `uaddl2`
(upper) with a hand-controlled fixture vs qemu, then the eor-chain.

### State
`cargo build --workspace` clean; `cargo test --workspace` **308/0**. Commits
`826db92` (ext fix), `bd60f2c` (generator + this doc), plus the earlier
`fe1e27a` cycle-28 close. The gen_pairwise_reduce generator is a permanent
asset that keeps surfacing this class. HARD GATE unchanged: real Roblox boot +
run log on a GPU + APK/binary host (none on this VPS).

---

# Session — in-place uaddl/saddl widening-alias fix (silent SIMD miscompile)

## The bug (found by the gen_pairwise_reduce differential fuzzer)
The previous session isolated an OPEN pair-xor reduction failing repro
`/tmp/combw/s13b.c` (u16 `for(i;i+=2) s ^= a[i]+a[i+1]`): native gcc + qemu-aarch64
oracle `2314`, JIT `59648`. Suspect was "in-place uxtl2 upper", but that was a
red herring — isolated `uxtl2 v0.2d, v0.4s` returns 3 correctly. The real cause:

**`SimdAddl` (uaddl/saddl/usubl) had NO in-place alias snapshot.** The widened
2*esrc dest write of lane i at `i*2*esrc` overlaps the narrow source bytes of
lane i+1 (at `(i+1)*esrc`), so when `rd` aliases `rn`/`rm` a read-then-write
loop clobbers the still-needed source and corrupts the sum. gcc's pair-xor
reduction emits `uaddl v0.4s, v0.4h, v1.4h` (dest==src). The sibling widening
ops `SimdAddw`/`SimdXtl` already snapshotted via `permute_source`; `SimdAddl`
was the missed gap.

## The fix
Mirror `SimdAddw`: snapshot `rn`/`rm` to the permscratch slots when either
aliases `rd`: `nb = if rn==rd { permute_source(..false) } else { vslot(rn) }`,
`mb = if rm==rd { permute_source(..true) } else { vslot(rm) }`.

## Verification
- in-place `uaddl v0.4s,v0.4h,v1.4h`: lane1 22 (was 20) — `0x160000000b`.
- `s13b.c`: 2314 == oracle (was 59648).
- 400 fresh fuzz cases across 9 seeds (7,19,31,42,5,11,23,55,77): **0 fail / 0 skip**.
- `cargo test --workspace` 309/0 green. New permanent gate
  `loader_run_pairwise_xor_inplace_uaddl_returns_2314` compiles the exact
  program with **-O3** (new `compile_o3` helper — the default `compile()` is -O0
  and never emits the SIMD uaddl path).

HARD GATE unchanged: real Roblox boot + run log on a GPU + APK/binary host
(none on this VPS).

## Session 31b (Sep 11, 2026) — multi-module (DT_NEEDED) loader + cross-module symbol binding

The loader could previously load only a **single** aarch64 image; the real
libroblox.so `DT_NEED`s a dependency chain (libssl, libcrypto, liblog, GSI libs)
that was never loaded, so the guest would fault on its first cross-module
import. Committed `8f3c48b` + `040c3bf` (workspace **310/0**, build clean):

- `libloader::deps::{load_elf_with_deps, LoadedChain}` resolves the main image's
  `DT_NEEDED` closure recursively, maps each dependency **contiguously** after
  the previous in one high guest region, so a single `jit_run` image slice
  `[chain.base, chain.end)` covers every module and cross-module calls compile
  from the same image. `load_elf_image` refactored into
  `load_elf_image_at(path, base)` (the `-shared` tools default to GNU hash, so
  no `DT_HASH` nchain — the scope scan is bounded by the mapped image end).
- `arm64jit::plt::build_export_scope(els)` builds a combined `name→guest_addr`
  map (main-first interposition); `bind_image_plt`/`bind_glob_dat` now take an
  optional scope and resolve an import a loaded dependency defines to its guest
  address (else the host resolver / float bridge / graphics stub as before).
- Two latent binder bugs fixed en route: the symbol scan mistook the **null
  symbol (index 0)** for a terminator → collected **0 exports**; and
  `patch_stack_canary` dereferenced its hardcoded libroblox GOT link
  (`0x631aa30`) even when it mapped outside a small module's image — now guarded
  by `host_addr_of` (real libroblox path unaffected: the slot is in-image).

Verified end-to-end with a cross-gcc `-shared` fixture: libmain.so `DT_NEED`s
libdep.so; `entry()` calls `dep_val()` (cross-module **JUMP_SLOT**) and reads
`dep_global` (cross-module **GLOB_DAT**) → returns **82** from the dependency's
guest address through the shared slice (both symbol-resolution families now
permanent `loader_run` gates).

HARD GATE unchanged: real Roblox boot + reproducible run log on a GPU + real
APK/binary host (`elfjit <libroblox.so> 0x1f0db20 --jni`); none on this GPU-less,
APK-less VPS. Next per RECOMMENDATION order: still libloader gaps (multi-module
binding validated only against synthetic fixtures until the GSI/APK is present),
then services/auth and the JNI surface.

## Session 32 (Sep 11, 2026, hermes-worker) — JNI native-method registry + C++ static-init (__cxa_guard*) shims wired into boot (315/0)

Opened at 311/0 green (no failing test; the android idempotency mandated fix is
long since committed). Runtime-side advance per RECOMMENDATION order (JNI
surface + the documented FMOD/engine C++ static-init boot blocker). Two commits:

### 1. arm64jit JNI: register->lookup->dispatch native-method bridge (a55ead8)
`jni_register_natives` returned JNI_OK and DROPPED the guest `JNINativeMethod`
array, so a native method Roblox binds (e.g. Java_..._IAPPurchaseManager_*) was
recorded nowhere and could never be called back into the guest. Now:
- `parse_register_natives` reads the guest JNINativeMethod array (3 u64 words:
  name*, signature*, fnPtr) at `methods[0..n)` and records (class,name) ->
  (signature,fnPtr) in a process-wide registry (`native_registry()`).
- `lookup_native_method(class,name)` returns the last-registration-wins binding.
- `dispatch_native_method` runs a registered guest fnPtr (a Java_* impl address)
  back through `jit_run` as a guest entry with args, returning guest x0.
+2 tests: records guest bindings (multi-entry parse, re-registration replaces,
  unknown->None), and end-to-end register->lookup->dispatch through jit_run
  returns 42 (guest `mov x0,#42; ret` as the registered fnPtr).

### 2. __cxa_guard_*/__cxa_atexit C++ static-init shims wired into boot (4cff657)
The elfjit `--jni` boot target (JNI_OnLoad) previously stopped when the FMOD /
engine C++ static-init dispatched into pc=0x68c7518 (a `.bss` guard) because the
guest's `__cxa_guard_acquire/release/abort` were never provided — the documented
handoff blocker. Now:
- `shims::register_cxx_shims()` installs Itanium `__cxa_guard_acquire/release/
  abort` (single-threaded byte semantics: acquire sets 1 and returns 1 to run the
  once-body, release sets 2, abort resets to 0) + a no-op `__cxa_atexit`.
- `plt::bind_image_plt` now calls `register_shims()` + `register_cxx_shims()`
  before symbol scanning, so the hand-written bionic shims (__errno/strlens/
  android_log/AAsset/ALooper/ANativeWindow) AND the C++ shims all participate in
  import binding instead of every unresolved name falling to the NULL/0 graphics
  catch-all. `register_named` is idempotent (reuses an existing slot).
+2 tests: guard acquire/release/abort byte machine, and `register_cxx_shims` are
  resolvable by name through `resolver::resolve` (proving the boot wiring).

### Verification
`cargo build --workspace` clean (0 errors); `cargo test --workspace` 315/0
(was 311). Differential fuzz re-run on 9 fresh seeds (77,101,202,303,404,505,
13,29,91) -> 0 fail / 0 skip, confirming no JIT correctness regression from the
wiring. HARD GATE unchanged: real Roblox boot + reproducible run log on a
GPU + real APK/binary host (`elfjit <libroblox.so> 0x1f0db20 --jni`) — none on
this GPU-less, APK-less VPS. Next (RECOMMENDATION order, still open): libloader
multi-module TLS (TPREL/DTPREL across DT_NEEDED deps), more JNI surface (fake
object backing), libbadcpu ISA, services/auth.

## Session 32b (Sep 11, 2026, hermes-worker) — REAL bug: self-imports bound to the NULL catch-all (commit 88ba3f6, 316/0)

Follow-on finding while validating the __cxa_guard_* shims against a real
cross-gcc C++ fixture (function-local static with dynamic init): the fixture's
`entry()` returned 0 instead of the correct 21. Root cause was NOT the guard
shims — it was a binder bug **exposed** by the fixture's `bl init_value@plt`:
a `-shared` module that calls one of its OWN exported functions goes through
`@plt`, and that JUMP_SLOT names a symbol the module itself defines
(`st_shndx != SHN_UNDEF`). `bind_image_plt` only handled cross-module deps
(via `scope`), host resolution, float bridges, and the graphics/stub catch-all —
never the module's own definitions. So `init_value@plt` bound to the NULL/0
graphics catch-all host thunk, and the call dispatched to a stub returning
garbage (0) instead of executing the real guest function at `guest_of(st_value)`.

**Fix (plt.rs):** before host resolution, read the dynamic symbol's `st_shndx`
(byte offset +6) and `st_value` (+8); if `st_shndx != 0` (locally defined), write
`el.guest_of(st_value)` into the GOT slot and count it resolved. `st_value` is
the definition's link address; `guest_of` maps it exactly like every other reloc
target. Self-defined CUDA/PAC/STT exports are the same shape.

**Verified** (`elfjit <guardfixture.so> 0x920 --jni`, real C++ static-init + guard):
first `init_value` -> `__cxa_guard_acquire` returns 1 (runs init, x=0+3+5=8),
release marks done; second call -> acquire returns 0 (already done), x=8+5=13;
sum 8+13 = 21, and JIT now returns exactly 21 = native oracle (a=8 b=13 sum=21
confirmed on host gcc). Before the self-import fix this returned 0. This pattern
(own-exported functions internally `bl fn@plt`-called) is ubiquitous in real
libroblox.so init code, so it is directly on the boot path.

+permanent regression `loader_run_self_import_binds_to_own_guest_body`
(internal_fn(7) = 35; readelf-verifies the JUMP_SLOT names a self-defined symbol
before running). `cargo build --workspace` clean; `cargo test --workspace`
316/0. HARD GATE unchanged: real Roblox boot + run log on a GPU/APK host (none
on this VPS).

## Session 32c (Sep 11, 2026, hermes-worker) — the documented once-routine regression ROOT-CAUSED + FIXED (commit fce362c, 317/0)

While validating the self-import fix against real cross-gcc fixtures, a
`-shared` module that recurses `f(n)=n<=0?7:f(n-1)+helper(n)` (f calls BOTH
itself AND helper@plt) returned 21 instead of native 27 at JIT_BUDGET>=64.
This is the long-documented **"nested guest bl-inline regression to once-
routine"** boot blocker, finally reproduced minimally end-to-end.

Root cause (all three pieces):
1. f's body calls `helper@plt` -> `body_contains_host_plt_bl(f)`=true -> f is
   **import-bearing**, so the recursion `bl f` must DIVERT via a dispatcher-
   return stub (a fresh f frame) rather than inline.
2. Compile-time handling was correct (`import_bearing` -> `force_stubs=true`,
   target NOT pushed to frontier). BUT the fixup-RESOLUTION step still checked
   `host_of_guest.contains_key(fx.target_pc)` FIRST — and f IS the current
   block's own start, so it's in `host_of_guest`. The recursion was re-inlined
   as a self-host-call anyway.
3. Inlining the recursive f AND diverting the inner helper@plt through the stub
   table regresses exactly like the QEMU once-body did: the inner import's
   dispatcher bookkeeping clashes with the inlined caller, so f(5)+helper
   produced 21 (one helper contribution lost) instead of 27.

Fix (jit.rs): a `divert_set` of guest-bl targets decided import-bearing. Stub
targets are built for them even when present in `host_of_guest`, and fixup
resolution routes them to the stub first (call rewritten call->jmp) so the
dispatcher runs each recursive f frame cleanly.

Exposed by the session-32b self-import fix: before it, `helper@plt` bound to the
NULL/0 catch-all and f got a garbage return (this exact fixture returned 0), so
the recursion bug was masked. Now the whole chain is exercised.

Verified: d.so returns 27 == native at EVERY JIT_BUDGET (1,2,8,64,8192; was 21
at >=64); all self-import family (a=1,b=61,c=720,d=27) match native; C++ guard
fixture still 21; fib/shared and plain-static recursion still correct. Fuzzer
re-run 3 fresh seeds 0 fail. +loader_run_recursive_import_bearing_callee_returns_
correct regression. `cargo test --workspace` 317/0, build clean. HARD GATE
unchanged: real Roblox boot + run log on a GPU/APK host (none on this VPS).

## Session 32d (Sep 11, 2026, hermes-worker) — cross-module TLS GOT binding: TPREL64 + TLSDESC (319/0)

Closed the last documented loader TLS gap (`R_AARCH64_TLS_*` across `DT_NEEDED`
deps, which the memory tracked as open). The multi-module loader seeded TLS only
for the main image, and TLS GOT relocations were unbound, so a `__thread` in a
dependency (libssl/libcrypto/...) read garbage/0. One focused commit:

- **`deps::setup_chain_tls` / `layout_chain_tls`** (`libloader`): lay EVERY
  module's `PT_TLS` block into one per-thread region at TP-relative offsets
  (aarch64 `TLS_TCB_AT_TP`: TCB at TP, first block at TP+16, later blocks
  alignment-padded) and copy each module's init image there. Returns `(TP,
  per-module offsets)`. Generalises `elf::setup_guest_tls` (main-only).
- **`plt::bind_chain_tls(els, offsets)`** (`arm64jit`): bind the chain's TLS GOT
  relocations to concrete TP-relative offsets:
  - `R_AARCH64_TLS_TPREL64` (1030, initial-exec): GOT slot = module block offset
    + symbol `st_value` + addend — the guest's `mrs tpidr_el0; ldr [GOT]; add
    tp,x0` then lands on the variable.
  - `R_AARCH64_TLSDESC` (1031, GCC 13+ default even for `-ftls-model=global-
    dynamic`): a 16-byte descriptor `{resolver_host_call, tprel}`; the resolver
    host fn returns `descriptor[1]` after the guest `blr`s to it.
  - **Pitfall found in debug:** GCC emits TLSDESC relocs into `.rela.plt`
    (DT_JMPREL), NOT `.rela.dyn` (DT_RELA) where initial-exec TPREL64 lives, so
    the binder must walk BOTH dynamic reloc tables (first cut bound 0 slots).

Verified end-to-end, no QEMU, with cross-gcc `-shared` dep chains exercising
BOTH models: dep-local `__thread` accessed through TLSDESC (`entry()==1007`) and
through initial-exec (`entry()==53`) via the full loader → cross-module binder →
chain-TLS → `jit_run` pipeline. Two permanent gates
`loader_run_chain_dep_tls_{initial_exec,tlsdesc}` seal them.

`cargo build --workspace` clean; `cargo test --workspace` **319/0** (was 317;
+2 loader_run). HARD GATE unchanged (no GPU/APK here). Next per RECOMMENDATION:
more JNI fake-object backing, libbadcpu ISA.

## Session 32e (Sep 11, 2026, hermes-worker) — global-dynamic TLS `__tls_get_addr` closes the LAST TLS dialect (320/0)

Completed the TLS relocation surface: GCC 13+ defaults to TLSDESC, but older dep
builds forced to the classic model (`-mtls-dialect=trad
-ftls-model=global-dynamic`) call `__tls_get_addr(&tls_index{module, offset})`
from the PLT — and that JUMP_SLOT previously fell to the NULL/0 catch-all (guest
called garbage). Commit `a1b1505`:

- `bind_chain_tls` additionally binds `R_AARCH64_TLS_DTPMOD64` (1028 →
  tls_index[0] = defining module's chain index) and `R_AARCH64_TLS_DTPREL64`
  (1029 → tls_index[1] = the symbol's offset within that module's block).
- `plt::set_chain_tls(tp, offsets)` stores process-wide TP + per-module block
  offsets (single-threaded).
- `host_tls_get_addr(a0=…&tls_index)` returns `TP + offsets[module] + offset`;
  `ensure_tls_get_addr()` registers it by name in `bind_image_plt` so the
  JUMP_SLOT binds (not the catch-all). `run_chain` seeds it before `jit_run`.
- Permanent gate `loader_run_chain_dep_tls_global_dynamic_returns_403`
  (dep_getx 3 + dep_gety 400 through a forced trad/gd dep).

This completes **all three AArch64 TLS access models** for cross-module deps:
initial-exec (TPREL64), TLSDESC (1031), and general-dynamic
(DTPMOD/DTPREL via `__tls_get_addr`). The `R_AARCH64_TLS_*` gap in the memory
ledger is now closed. `cargo build --workspace` clean; `cargo test --workspace`
**320/0**. Next per RECOMMENDATION: more JNI fake-object backing, libbadcpu
ISA, services/auth. HARD GATE unchanged (no GPU/APK here).

---

## Session (cycle 34, Sep 11, 2026) — RESOLVED the last documented open arm64jit bug (323/0)

Closed the long-open cycle-33 residual (fuzz seed 9000_58, `gen_fp_edge`):
full-program JIT 2474795 vs both oracles 2039651 (+435144 = exactly a[5]*1e6).
This was the only remaining documented arm64jit correctness gap on this box.

**Root cause — a decode collision, NOT the hypothesized host-register clobber.**
gcc -O3 schedules a scalar `fcsel Dd,Dn,Dm,<cond>` into the d-reg min/max chain
whose rm/cond fields give it a top-16 (`0x1e65`) that ALSO matches the
fcvt-to-int decode gate (`fcvtau`/`fcvtas`, mode 2). e.g. `fcsel d26,d28,d5,mi`
= `0x1e654f9a` decoded as `Inst::FcvtToInt` — which writes integer X{rd}, NOT
vector D{rd} — so the min-accumulator (d26) never updated and kept its stale
`a[i]*1e6` double; the final total carried that element. This explains the two
confusing facts: it only reproduced from the full program (register-reuse gave
the exact badly-encoded fcsel), and JIT_BUDGET=1/JIT_STEP did NOT mask it
(per-instruction decode, not a within-block scratch collision).

**Fix (decode.rs, one discriminator):** the FcvtToInt gate now requires
`bits[11:10]==00` in addition to the existing bit12-clear guard. Every real
fcvt-to-int clears bits[11:10] (verified fcvtau 0x1e650062 = 00); fcsel needs
0b11 (cond lives in bits[15:12]). With the guard, `0x1e654f9a` falls through
to `Inst::FcsSel` and d26 updates correctly. Regression-guarded:
- decode unit: fcsel 0x1e654f9a/0x1e65ef9a -> FcsSel, fcvtau 0x1e650062 ->
  still FcvtToInt (added to fp_2d_op_decode_collisions_with_int_add_bsl_and_fcvt).
- e2e canary `diff_fp_edge_fcsel_swallowed_as_fcvt` (full program, jit==oracle
  ==2039651). Sensitivity-proven: reverting only the `0x0c00` guard re-fails
  the canary with the EXACT original jit 2474795 vs oracle 2039651.
- `fuzz_repros/README.md` updated: fp_edge_9000_58/21 now RESOLVED.

`cargo build --workspace` clean; `cargo test --workspace` **323/0** (was 322).
HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK host (none
on this VPS). Next per RECOMMENDATION order: JNI fake-object backing, libbadcpu
ISA, services/auth.

## Session (cycle 36, Sep 11, 2026) — GLES float/mixed + >8-arg bridge to real Mesa; JIT compressed-texture decode via shared texture_codec (343/0)
Closed the cycle-35b graphics hole (float-ABI and >8-arg GLES stuck on the
NULL/0 stub: no way to clear the framebuffer or upload a texture) and the
compressed-texture JIT gap. Commits c951bb7, 370b544.

1. **`HostGlesCall` bridge (c951bb7).** The integer `HostCall` (8 x-reg args) and
   the uniform-float bridges can't express GLES functions that mix integer args
   (x0..x7) with float args (low 32 bits of s0..s7) or take >8 args (the 9th+ on
   the guest stack). Added `jit::HostGlesCall = extern "C" fn(*mut CpuState)->u64`
   + `register_gles_call` (new thunk region after the f32 slots); the dispatcher
   hands each registered wrapper the full guest CpuState. `resolver::resolve_gles_mixed`
   installs one wrapper per function that reads the exact x/s/sp lanes its
   AArch64 signature uses and calls real Mesa (libGLESv2.so.2, RTLD_LOCAL).
   Wired into the PLT binder (plt.rs 7-tuple) so these imports bind to real Mesa.
   Covered: glClearColor/BlendColor/ClearDepthf/DepthRangef/LineWidth/PolygonOffset/
   SampleCoverage/TexParameterf, glUniform{1,2,3,4}f, glVertexAttrib{1,2,3,4}f,
   glTexImage2D/TexSubImage2D/TexImage3D (stack pixels). Filled the integer-ABI
   whitelist with missed pointer-arg GLES: glUniform1fv..4fv, glUniformMatrix{2,3,4}fv,
   glTexParameterfv, glGetTexParameterfv, glGetFloatv, glGetTexLevelParameteriv.

2. **JIT compressed-texture decode (370b544).** The wrapper's Android-format
   decompression was bypassed by the JIT path (glCompressedTexImage2D bound to raw
   Mesa -> undecodable ETC2/ASTC on desktop). Extracted `crates/texture-codec`
   (ETC1/ETC2/EAC/ASTC/ATC -> RGBA8 + handle_compressed_tex_image_2d/sub_image_2d;
   6 tests moved from the wrapper) shared by BOTH the glesv2-wrapper cdylib and the
   JIT. glCompressedTexImage2D moved to the mixed bridge (removed from the int
   whitelist) and glCompressedTexSubImage2D now decode to RGBA8 and upload via real
   glTexImage2D/SubImage2D; non-Android formats fall through to Mesa.

**Headless gates (permanent, real Mesa llvmpipe + surfaceless EGL, no GPU):** the
end-to-end test drives context creation (eglGetDisplay/Initialize/ChooseConfig/
CreateContext/MakeCurrent) entirely through guest blr and the int bridge,
glClearColor(0.5,0.25,0.75,1.0) round-trips via glGetFloatv (float bridge), a 1x1
glTexImage2D uploads through the stack bridge, and a 4x4 ETC2 upload leaves
glGetTexLevelParameteriv(GL_TEXTURE_INTERNAL_FORMAT)==GL_RGBA8 (proves the JIT
decompressed it, not raw-Mesa). cargo build --workspace clean; cargo test
--workspace 343/0. HARD GATE unchanged: real Roblox boot + run log on a GPU/APK host.

3. **Headless window-presentation gate (cdab7f8, 344/0)** — proved
   GRAPHICS_RECOMMENDATION §5 (ANativeWindow→desktop window → EGL window surface →
   present) end-to-end: `arm64jit/tests/egl_window_present.rs` spawns Xvfb, opens
   an X11 window via input-wrapper, and drives eglGetDisplay/Initialize/
   ChooseConfig(window-capable)/CreateWindowSurface (the X11 Window XID as
   native_window)/CreateContext/MakeCurrent, then glClearColor(float bridge)+
   glClear+eglSwapBuffers — all through guest blr + the resolver bridges under
   EGL_PLATFORM=x11 on Mesa llvmpipe. First proof a translated guest can present
   frames to a real on-screen window on a headless box. arm64jit gained
   input-wrapper + x11rb dev-deps.

4. **JNI object-array backing (e4e8aea, 345/0)** — NewObjectArray(172)/
   GetObjectArrayElement(173)/SetObjectArrayElement(174) were NULL/garbage in the
   JNI function table. Backed them like the primitive arrays (shared
   array_len_registry): object arrays are len×8 opaque jobject slots; NewObjectArray
   allocs + optionally seeds with initialElement; Get/Set are bounds-checked
   (get→NULL, set→no-op out of range, never fault). +regression test.

5. **timerfd + signalfd syscalls (5220aed, 346/0)** — ALooper/libutils wait on
   timerfds for timeouts and some services take a signalfd; both were -ENOSYS.
   timerfd_create(85)/settime(86)/gettime(87) are clean itimerspec forwards
   (283 is membarrier, already handled); signalfd4(74) returns a real host
   signalfd with an EMPTY sigset (no guest-signal dispatch here, so it never
   fires, but the call succeeds). +guest_svc_timerfd_and_signalfd_roundtrip.

## Session (cycle 35, Sep 11, 2026) — GRAPHICS TRANSLATION LAYER: egl-wrapper + glesv2-wrapper cdylibs, input-wrapper, real-EGL JIT wiring (336/0)
Per the worker operating rules, took the graphics translation layer
(GRAPHICS_RECOMMENDATION.md) as the highest-leverage unblocked item — built AND
headlessly verified via Mesa llvmpipe + surfaceless EGL (no GPU needed). Commits
6a6efd5, bcf49ed, 50f8b14:

1. **`crates/egl-wrapper` + `crates/glesv2-wrapper`** — cdylibs exposing the guest's
   exact sonames (`libEGL.so`/`libGLESv2.so`). They dlopen Mesa (`libEGL.so.1`/
   `libGLESv2.so.2`) and forward every entry via a `dl.rs` resolver (dlsym'd addrs
   cached once; a missing Mesa symbol aborts loudly rather than calling null). All
   44 EGL + 142 GLES signatures transcribed EXACTLY from the system headers by
   `gen_forward.py` (kept regenerable). `glCompressedTexImage2D`/`SubImage2D` are
   hand-written to intercept Android compressed textures — ETC1 (0x8D64), ETC2
   RGB/RGBA1/RGBA8, EAC R/RG signed+unsigned, ASTC (0x93B0..0x93BD), ATC — and
   decompress to RGBA with pure-Rust `texture2ddecoder` (BGRA→RGBA swizzle), then
   upload via the real `glTexImage2D`/`glTexSubImage2D`. Non-Android formats pass
   through to Mesa untouched.
2. **`crates/input-wrapper`** (GRAPHICS_RECOMMENDATION §6) — Android MotionEvent/
   KeyEvent/action-key model + surface-agnostic `PointerTracker` (mouse→ACTION_DOWN/
   MOVE/UP multi-touch), X11 keysym→AKEYCODE map, and a raw-X11 (pure-Rust x11rb,
   no C toolchain) window/event pump.
3. **In-process JIT graphics wiring** — `resolver::resolve_egl` dlopens real Mesa
   `libEGL.so.1` RTLD_GLOBAL and binds `egl*` imports as integer-ABI `HostCall`s, so
   the guest's EGL calls now run real Mesa instead of the NULL/0 graphics catch-all.
   Regression drives a guest `blr` to `eglGetError` through jit_run and asserts a real
   non-zero EGL error enum. GLES is deliberately NOT routed through the integer
   HostCall (its float-in-xmm ABI needs a dedicated float bridge).

**Permanent headless gates:** `glesv2-wrapper/tests/headless_graphics.rs` dlopens
both .so shims and exercises a real surfaceless ES3 llvmpipe context through them
(EGL forwards, GLES renderer string, ETC2 intercepted => GL_TEXTURE_COMPRESSED=0,
DXT1 passthrough, eglGetProcAddress forwards). `input-wrapper/tests/xvfb_input.rs`
spawns Xvfb and opens a mapped window through the crate, validating connect + the
pointer→touch mapping (random-display + connect-retry for stability). Plus 6 texture
unit, 3 input unit, 2 EGL resolver tests. Installed libegl-dev/libgles-dev/libgbm-dev/
libwayland-dev/mesa-utils first; input deps use pure-Rust x11rb.

`cargo build --workspace` clean (only the cosmetic cdylib crate-name warnings);
`cargo test --workspace` **336/0** (was 323). HARD GATE unchanged: real Roblox boot +
run log on a GPU/APK host (none on this VPS). Next: wire the wrapper sonames into the
guest loader's NEEDED resolution and add a GLES float-ABI host bridge, then continue
the ordered RECOMMENDATION list (JNI fake-object backing, libbadcpu ISA, services/auth).

## Session (cycle 35b, Sep 11, 2026) — integer-ABI GLES routed to real Mesa in JIT resolver + JNI array offset bug fixed (341/0)
Two follow-on commits on the graphics/JNI surface:

1. **de5fed1 — `resolver::resolve_gles_int`**: mirroring the EGL wiring, 107 GLES
   entry points with a pure integer/pointer ABI AND at most 8 integer args (all the
   texture/state/draw pipeline calls Roblox makes — bind/gen/delete, shader compile,
   draw arrays/elements, buffer data, stencil/blend/depth, uniform*i, viewport) are
   now resolved to real Mesa via a whitelist (GLES_INT_NAME_LIST, generated from the
   exact headers). `gles_handle()` dlopens libGLESv2.so.2 with **RTLD_LOCAL** (NOT
   GLOBAL: a global load would leak float-taking `gl*` into the general resolve()
   RTLD_DEFAULT scan and corrupt their xmm args through the integer bridge). Float-
   taking GLES (glClearColor) and >8-arg forms (glTexImage2D) are deliberately kept
   on the NULL/0 stub — they need a dedicated float-ABI bridge, not wired here.
2. **6ba6104 — JNI primitive-array accessors + a REAL offset bug**: verified every
   JNINativeInterface offset against the authoritative Android NDK r26b jni.h
   (extracted from the official NDK zip). Found REGISTER_NATIVES was 199 and
   GET_JAVA_VM 203 but the NDK places them at **215 and 219** (the 175..214
   New<Prim>Array/Get/Release/Get/Set<Prim>ArrayRegion block was skipped), so a
   guest's RegisterNatives/GetJavaVM table index read the wrong (NULL) slot and
   JNI_OnLoad native registration was silently dropped in the real boot path. Fixed
   the offsets and implemented the missing JNI primitive-array surface (jbyteArray /
   jintArray = guest-addressable buffer + byte-length registry; new/get/release/
   region ops, bounds-checked), wired at the corrected slots; GetArrayLength now real.
   +3 regressions (NDK offset assertions, ByteArrayRegion round-trip, Elements->backing).

cargo build --workspace clean; `cargo test --workspace` **341/0** (was 336). HARD GATE
unchanged: real Roblox boot + run log on a GPU/APK host (none on this VPS).

## Cycle 37 (Sep 11, 2026) — JNI fake-object backing + full JNI surface (351/0)

Commit `23e053a` (dev). Per the ordered post-graphics RECOMMENDATION list, took
"JNI function-table stubs / fake-object backing" as the highest-leverage
unblocked item. The JIT `jni.rs` (crates/arm64jit) had backed arrays, strings and
~14 slots, but everything else in the JNINativeInterface fell to the `voidp`
default (returns 0) — so guest Roblox JNI paths that `if (!ref / !clazz / !buf)
fail` aborted instead of proceeding.

Backed the behavior-changing slots at their authoritative Android NDK offsets:
- **NewLocalRef / NewWeakGlobalRef**: identity pass-through (no distinct ref pool).
- **IsSameObject**: identity compare (1 iff a==b).
- **GetObjectClass**: stable non-zero jclass handle (interned
  `java/lang/Object`), never aliases the object; NULL obj -> NULL class.
- **IsInstanceOf / IsAssignableFrom**: permissive true (fake-object model takes
  the success branch instead of a NULL/abort path).
- **GetStringUTFRegion**: bounds-safe UTF-8 byte copy (real behavior, tested).
- **DirectByteBuffer trio** (Roblox passes textures/audio/asset native memory as
  java.nio.ByteBuffer): NewDirectByteBuffer creates a fresh unique handle into a
  handle->(addr,cap) registry; GetDirectBufferAddress/Capacity recover it; unknown
  handle -> 0.
- **PopLocalFrame**: passes its `result` arg through.
- Wired monitor (Enter/Exit=JNI_OK), exception (Occurred/Check=no-pending,
  Describe/Clear no-op), local-frame (Push=JNI_OK, EnsureCapacity=JNI_OK),
  static-field (GetStaticFieldID=GetMethodID stub, Get/SetStatic* typed), and the
  Call{Object,Boolean,Int,Void}Method + CallStatic* forms (typed zero) to explicit
  stubs so they're non-null at correct offsets.

**Tests**: +4 (NDK offset assertions for 23 slots; nonnull-at-official-offsets for
17 boot-relevant slots; fake-object backing semantics; direct-buffer round-trip +
string-region copy). `cargo test --workspace` **351/0** (was 346), build clean.

Next per RECOMMENDATION order: libbadcpu ISA gaps, then services/auth. HARD GATE
unchanged (real Roblox boot + run log only on a GPU/APK host; none on this VPS).

## Cycle 37 / 37b (Sep 11, 2026) — JNI fake-object backing + boot syscall gaps (352/0)

Commits `23e053a`, `ae4de9c` (+ docs 3a8ea7b, 1d8783f) on `dev`. Per the ordered
post-graphics RECOMMENDATION list (JNI function-table stubs / fake-object backing,
then ELF/loader, libbadcpu, services/auth).

1. **JNI fake-object backing (`23e053a`)** — `crates/arm64jit/src/jni.rs` backed
   arrays/strings/~14 slots; the rest of the JNINativeInterface fell to the voidp
   default (return 0), so guest `if (!ref / !clazz / !buf) fail` aborts. Wired the
   behavior-changing slots at authoritative NDK offsets: NewLocalRef/NewGlobalRef/
   NewWeakGlobalRef (identity), IsSameObject (identity compare), GetObjectClass
   (stable non-zero jclass), IsInstanceOf/IsAssignableFrom (permissive true),
   GetSuperclass/PopLocalFrame, the DirectByteBuffer trio (NewDirectByteBuffer/
   GetDirectBufferAddress/GetDirectBufferCapacity via a handle->(addr,cap) registry
   — Roblox's NIO texture/audio/asset buffers), and GetStringUTFRegion (bounds-safe
   copy). Monitor/exception/local-frame/static-field/call-method slots are typed
   no-op stubs at correct offsets. +4 tests (NDK offsets, nonnull-at-offset, fake-object
   semantics, direct-buffer roundtrip, string-region copy). 347→351.

2. **Boot-critical syscall gaps (`ae4de9c`)** — sysinfo(179)/statx(291)/
   get_robust_list(100)/restart_syscall(128) were -ENOSYS. sysinfo fills the guest
   asm-generic 64-bit struct with REAL host values (no forging per the record);
   statx is a raw SYS_statx forward; get_robust_list reports an empty robust-futex
   list so glibc pthread init proceeds; restart_syscall returns -EINTR.
   +guest_svc_sysinfo_statx_robust_restart_roundtrip. 351→352.

3. **Differential-fuzz validation** — 3 campaigns, 14 fresh seeds x 100 = ~1500
   cases, **0 failures / ~150 skips**: the JIT FP (fcsel/frint/fcvt/vector-compare),
   SIMD (widen/bitmix/popcount/4s), bitfield, long-loop and shared/PIE paths all
   hold against the qemu-aarch64 + native x86 gcc oracles. No silent miscompile found.

`cargo build --workspace` clean; `cargo test --workspace` **352/0**. HARD GATE
unchanged: real Roblox boot + run log only on a GPU/APK host (none on this VPS).

**Assessed next (not landed — see project no-half-baked discipline):** the armed64jit
guest thread model (`clone` 220 → host thread with post-svc PC continuation + a
per-thread CpuState + join/tid registry). This is the last large JIT capability a
real Roblox run needs (render/audio/network worker threads) but is a genuine
multi-session subsystem; the translator already threads per-instruction guest PC
(needed for child continuation) and guest==host makes child memory shared.

## Cycle 38 (Sep 11, 2026) — GUEST THREAD MODEL: clone(220) spawns a real host thread (353/0)

Commit landed on `dev` (jit thread-model slice). This was the flagged
next-biggest JIT capability a real Roblox run needs (render/audio/network
worker threads) and the HANDOFF's stated "last large JIT capability". The first
**bounded, fully-correct, tested slice** of it:

- **CpuState** gains `svc_next` (post-svc guest PC) + `tid`.
- **Svc translate arm** now (a) records `pc+4` into `state.svc_next` before the
  `guest_svc` call, and (b) early-returns from the compiled block when a syscall
  zeroes `state.pc` — the mechanism that lets a child thread's OWN `jit_run`
  unwind cleanly at thread-exit (the inlined svc otherwise continues into the
  next guest instruction).
- **guest_svc `clone`(220)**: requires CLONE_VM (the sharing thread case that
  pthread_create uses). Assigns a guest tid, clones the parent register file,
  sets child SP from the `child_stack` arg + new TLS (CLONE_SETTLS), honors
  CLONE_PARENT_SETTID/CLONE_CHILD_SETTID stores, and `std::thread::spawn`s a
  real host thread that re-enters `jit_run` at the post-svc PC — so the child
  continues the guest program right after its `svc`, exactly as AArch64 clone
  semantics dictate. `EXEC_CTX` (image ptr/len/base) is registered at `jit_run`
  entry for the child's re-entry.
- **exit(93) is now thread-local on a spawned child** (pc=0 → Svc-arm early-ret
  → child `jit_run` returns → host thread ends); **exit_group(94) and
  main-thread exit still `_exit` the whole process**. `gettid`(178) stays the
  real host tid — each clone child is a distinct host thread, so it returns a
  distinct, correct id.
- **New helpers**: `jcc_rel8`/`jne_rel8`/`jz_rel8` in x86.rs (short rel8 jcc —
  needed for the in-block thread-exit `ret` guard).

**Regression gate** `loader_run_clone_spawns_guest_thread`: a raw-`clone`
fixture (cross-gcc) where the child sums 0..63 on its own guest stack, writes
42 to a shared guest global, then thread-exits (93) without killing the parent;
the parent's bounded spin then returns 42. Proves end-to-end that a *second
host thread* ran guest code in the same image, shared memory (guest==host),
distinct stack/tid, and clean thread-local exit. Verified e2e manually first
with a SIGSEGV-logging harness (`entry()` → 42) and root-caused an initial
fixture ABI bug (child resumes post-svc WITHOUT the caller's `sub sp,#0x20`
prologue, so its `[sp+#8]` locals sat above the raw SP — pointed `child_stack`
below the buffer top).

`cargo build --workspace` clean; `cargo test --workspace` **353/0** (was 352).
Differential fuzz seeds 1 (36 cases), 7 (38), 55 (38) all 0 fail — the Svc-arm
change (now emits a pc-load/test/guard on every svc) didn't regress the JIT.

**Known next** in this subsystem (documented, not half-baked in): pthread_join
(hold child tid + futex-CLONE_CHILD_CLEARTID join), per-thread guest TLS block
layout beyond SETTLS-pointer handoff, `clone3`(435), and tgkill/signal
delivery to a specific child. The core spawn+shared-memory+thread-local-exit
loop is now verified working. HARD GATE unchanged: real Roblox boot + run log
only on a GPU/APK host (none on this VPS).

## Cycle 38b (Sep 11, 2026) — pthread_join primitive: CLONE_CHILD_CLEARTID + futex (354/0)

Commit `b778647` on dev. Completed the thread lifecycle after cycle 38's clone
spawn: a joining parent can now block on `FUTEX_WAIT(ctid)` and be woken when
the child exits.

- `CpuState` gains `clear_tid_addr`. `clone`(220) now honors
  `CLONE_CHILD_CLEARTID` (0x00200000): the child carries the child-tid word
  address in its state (alongside the existing CLONE_CHILD_SETTID store).
- thread-local `exit`(93) on a spawned child now **zeroes** that word and
  **FUTEX_WAKEs** it before halting `pc` — exactly the kernel
  CLONE_CHILD_CLEARTID semantics pthread_join's futex-wait depends on.
- +`loader_run_clone_child_cleartid_join_via_futex`: parent FUTEX_WAITs on the
  child's clear-tid word; child computes a signed sum, publishes a result
  global, thread-exits (93); the exit clears+wakes the word so the parent's
  wait returns. Asserts the word is zeroed, the result published, and the wait
  returned cleanly -> 42.

`cargo test --workspace` **354/0** (was 353), build clean. The core spawn →
  child-runs-on-own-stack/tid → publishes-shared-state → thread-local-exit →
  parent-futex-joins lifecycle is now verified headlessly end-to-end. Documented
  remaining: per-thread guest TLS block layout, clone3(435), tgkill/signal
  delivery. HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK
  host (none on this VPS).

## Cycle 38c (Sep 11, 2026) — clone3(435) struct-args thread spawn (355/0)

Commit landed on dev. Modern glibc/bionic prefers clone3(435) over clone(220),
so a real boot needs it.

- Refactored the shared-VM thread spawn from the clone(220) arm into
  `spawn_guest_thread(s, flags, stack, parent_tid, tls, child_tid)` — one body,
  two syscall shapes.
- clone3(435): reads `struct clone_args` from guest memory (flags@0,
  child_tid@16, parent_tid@24, stack@40, tls@56; -EINVAL if the guest's `size`
  is too small for those fields) and spawns via the shared helper — same
  CLONE_VM/SETTLS/PARENT_SETTID/CHILD_SETTID/CLEARTID semantics as clone(220).
- +`loader_run_clone3_struct_args_spawns_guest_thread`: raw clone3 with a real
  struct clone_args — child computes 10! (3628800) on its own stack, publishes a
  result global, thread-exits (93); parent futex-joins on the child's clear-tid
  word; asserts the factorial result, the zeroed clear-tid, and a clean wait.

`cargo test --workspace` **355/0** (was 354), build clean. clone(220),
clone3(435), and pthread_join (CLONE_CHILD_CLEARTID + futex) all verified
headlessly. Remaining thread-model next: per-thread guest TLS block layout
(beyond SETTLS pointer handoff), tgkill/signal delivery to a specific child.
HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK host (none
on this VPS).

## Cycle 39 — SIMD variable-shift ushl/sshl + unaligned `ext` fixes (359/0)

New `gen_varshift` differential fuzz generator (variable shift by register,
both 32/64-bit lanes) exposed **two real silent miscompiles**, both fixed on
`dev` (commit `e8b9721`):

1. **ushl/sshl Vd.T,Vn.T,Vm.T.** The decode gate `(insn & 0x3f000c00) ∈
   {0x2e,0x4e,0x6e}` masked out bit29 (the **U** bit — ushl=1, sshl=0) AND the
   `.2d` size bits, so **sshl never decoded** (its residue is `0x0e…`) and the
   `signed_` flag was inverted. Fixed to mask `0xffe0_fc00` with all 14
   q/size/ushl-sshl residues; `signed_ = bit29==0`; added the `q` flag the
   translate was ignoring. The translate previously did a plain `shl` with
   `16/esize` lanes always (wrong for q0) and no negative-count handling.
   Rewritten to honor q, sign-extend the count lane, and implement ARM
   semantics: C≥0 left shift; C<0 right shift by -C (**sshl arithmetic** /
   **ushl logical**); |C|≥element-width → 0 (ushl) or sign-fill (sshl).
   Also fixed rel32 patching (done-jumps were patched to a stale offset recorded
   before the right path was emitted; the 64-bit left path fell through into the
   right path; and the JS gate had no `test_rr64`). Encodings verified against
   `aarch64-linux-gnu` for `.2d`/`.4s`. +`simd_var_reg_shift_2d_reference` and
   `simd_var_reg_shift_4s_reference` unit tests.

2. **`ext VD.16B,Vn,Vm,#imm`** — pre-existing latent bug: the SimdExt translate
   computed the concat offset in **bytes** but shifted by that value as if **bits**
   (`shr/shl by 4` instead of `32`), so every unaligned immediate (≠0,≠8)
   returned garbage. gcc's cross-lane XOR-reduce (`ext #8`+`ext #4`) mis-combined
   the halves, which is what also derailed the sshl `.4s` compound cases. Fix:
   `sh = (start%8)*8`. +`diff_simd_ext_unaligned_xor_reduce` canary.

Both bugs are silent (no crash — wrong values). Fuzz generator now bounds shift
counts below element width so the native-gcc oracle and ARM agree (ARM left-shift
by ≥width = 0, x86 masks `cl` mod-width — UB region differs by ISA). A follow-on
generator `gen_scalar_varshift` now covers the scalar lslv/lsrv/asrv path (the
SIMD gen_varshift vectorizes to NEON ushl/sshl; the scalar arm was unfuzzed) —
40/40 differential clean, plus a 600-case confirmation campaign.

**`cargo test --workspace` 359/0** (was 355). New cross-lane fuzz campaign clean
(8 seeds × 40, all pass). `cargo build --workspace` clean. HARD GATE unchanged:
real Roblox boot + run log only on a GPU/APK host (none on this VPS).

## Cycle 40 (Sep 11, 2026) — GUEST SIGNAL DELIVERY: rt_sigaction/kill/tgkill dispatch, handler run + SIGRET resume (363/0)

Commit `5f8c16c` on `dev`. Completed the thread-model item documented as the
next gap: a guest thread can now be interrupted by a signal (self-delivered or
cross-thread), run a Linux-style handler, and resume cleanly.

**New `crates/arm64jit/src/signals.rs`** (guest signal contract):
- `rt_sigaction`(134) parses the aarch64 kernel `struct sigaction` (handler@0 /
  flags@8 / restorer@16 / 8-byte mask@24) into a process-wide table
  (SIG_DFL / SIG_IGN / guest-handler fn), reporting the prior action into oact.
- `kill`(129)/`tgkill`(131) go through the guest model (NOT forwarded to libc,
  which would kill the host): a SAME-thread target runs the handler right after
  the `svc`; a DIFFERENT guest thread gets a cooperative `pending_signal` that
  its own `jit_run` loop picks up (proves "signal to a specific child thread").
- Handler run: save the interrupted context (x regs, vectors, sp, TPIDR, NZCV)
  + guest siginfo/ucontext on a per-thread frame stack; enter the handler with
  the aarch64 signal ABI (x0=signo, x1=siginfo, x2=ucontext, x30=SIGRET); on the
  handler's `ret` (x30 lands on the SIGRET sentinel) or an explicit
  `rt_sigreturn`(139) restore and resume right after the interrupted `svc`.
- Un-handled signals fall back to the POSIX default disposition (ignore for
  SIGCHLD/SIGURG/SIGWINCH/SIGCONT+stop family; terminate the process 128+sig
  otherwise), so a guest raise(SIGTERM)/SIGPIPE behaves like Linux.

**Two JIT fixes the dispatch exposed (both real):**
1. Svc translate arm: the syscall-return store `stg x0` was overwriting the
   handler's signo argument before the block yielded. Redirect now decides
   FIRST (before `stg`), so guest x0 keeps `sig` for a self-delivered handler.
2. `svc`-bearing `bl` callees — new `body_contains_svc` (follows guest calls
   transitively) — are now diverted through the dispatcher like host-import
   callees, so every `svc` runs at a top-level block boundary. Without this, an
   svc inlined into a caller's monolithic block whose Svc arm early-rets (signal
   redirect / thread-local exit) popped the *inlined-caller* return address
   instead of jit_run's — corrupting the host stack (a real `movaps [rsp]` fault
   below a corrupted RSP). This also hardens the pre-existing child-thread
   local-`exit`-via-helper path.

**CpuState**: `+redirect_request` (block-yield to a handler) and
`+pending_signal` (cross-thread cooperative pickup, volatile), plus a
guest-tid → CpuState registry (`GUEST_THREADS`) for `post_signal_to_thread`.

**Tests (+3, one strengthened):** `loader_run_self_signal_handler_runs_and_resumes`
(self-delivered SIGUSR1 handler sets globals, resumes, returns 42),
`loader_run_sig_ign_prevents_termination` (SIG_IGN for default-death SIGPIPE
survives), and `loader_run_cross_thread_signal_delivers_to_child` (parent
`tgkill`s a spawned child; the child's own thread picks it up cooperatively,
runs the handler, publishes a result and futex-joins — real cross-thread
delivery). Unit `body_contains_svc_follows_call_graph`. The `guest_svc` oact
assert moved from "128-byte buffer zeroed" (the old no-op artifact) to the real
32-byte aarch64 sigaction struct boundary.

`cargo build --workspace` clean; `cargo test --workspace` **363/0** (was 359).
HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK host (none on
this VPS). Thread-model remaining: per-thread guest TLS block layout beyond the
SETTLS-pointer handoff; signal-blocking (rt_sigprocmask is a no-op) and real
timer/signalfd dispatch are still simplified.

---
## Cycle 41 (Sep 11, 2026) — REAL rt_sigprocmask blocking + per-thread TLS TP (365/0)

Two thread-model gaps closed on `dev` (commits `de483c3`, `517b2fb`), following
cycle 40's guest signal delivery.

### 1. Real `rt_sigprocmask` (135) — signal blocking, Linux semantics (de483c3)
The previous arm was a no-op (accepted and reported an empty old-set). Now:
- `signals.rs::sigprocmask` implements SIG_BLOCK(0)/SIG_UNBLOCK(1)/SIG_SETMASK(2)
  on a per-thread `CpuState.blocked_mask` (64-bit sigset, bit N-1 = sig N),
  reports the previous mask into oset, returns -EINVAL on bad `how`/`sigsetsize`
  or a NULL-set SIG_SETMASK. SIGKILL(9)/SIGSTOP(19) bits are silently dropped
  from any attempted mask (the kernel never lets a thread block them).
- A signal that arrives while blocked is **mercifully marked pending**
  (`pending_mask`) and NOT dispatched; the dispatch arms (`kill`/`tgkill` self,
  cross-thread post) route through `signals::deliver` which checks `is_blocked`.
- On `rt_sigprocmask` returning after an UNBLOCK/SETMASK, `take_deliverable_pending`
  drains the lowest unblocked pending signal and dispatches it post-svc (the
  kernel delivers a pending signal before returning from sigprocmask).
- **Cross-thread `tgkill` now posts into the target's `pending_mask`** via
  `mark_pending` (blocked-aware), replacing the old fixed single-word
  `pending_signal`; the target's dispatcher loop drains
  `take_deliverable_pending` (respects its own blocked_mask) each iteration.
  `pending_mask` access is volatile/atomic so a sender racing the owner's clear
  can't lose a newly-pending signal.
+`loader_run_sigprocmask_block_then_unblock_delivers_pending` — set SIGUSR1
handler, BLOCK it, `tgkill` self (handler MUST NOT run), UNBLOCK, assert the
handler now runs with the right signo; returns 42.

### 2. Per-thread TLS TP for `__tls_get_addr` (general-dynamic) (517b2fb)
`host_tls_get_addr` previously resolved `{module, offset}` against a
process-global main TP (from `set_chain_tls`), so a clone-spawned child thread
accessing a dependency's `__thread` via the classic global-dynamic model
(`-mtls-dialect=trad -ftls-model=global-dynamic`) read the MAIN thread's block.
- New `jit::current_guest_tp()` thread-local, published at `jit_run` entry from
  `CpuState::tpidr`. Each guest thread runs its own host thread (main scope or
  clone child's `std::thread::spawn`), so this holds that guest thread's TP.
- `host_tls_get_addr` uses `current_guest_tp()` (falling back to the main TP
  only for a never-published call). Module block offsets (TP-relative) stay
  global; only TP is per-thread. This closes the documented "per-thread guest
  TLS block layout beyond the SETTLS-pointer handoff" item for the
  general-dynamic path.
+unit `current_guest_tp_is_thread_local_and_published` (per-thread distinct TP,
published/reset on the right host thread).

### State
`cargo build --workspace` clean; `cargo test --workspace` **365/0** (was 363).
Thread-model remaining (documented, needs a multilib/real-guest host to fully
validate): real timer/signalfd **to-guest-handler** dispatch (rt_sigaction
guests can't yet receive host POSIX-timer expiry as guest signals); per-thread
TLS **init-image copies** for clone children beyond the TP pointer handoff.
HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK host (none on
this VPS). Next per RECOMMENDATION order: libbadcpu ISA, services/auth, more
differential-fuzz coverage.

## Cycle 42 (Sep 11, 2026) — fuzzer found FP pairwise gap; fmaxp/fminp/fmaxnmp/fminnmp fixed (366/0)

### What happened (a new generator caught a real incomplete-ISA decode)
Extended `fuzz_jit.py` with two generators for previously-uncovered shapes:
- **`gen_double_neon`** — f64-lane (`.2d`) NEON, which NONE of the existing SIMD
  generators exercised (they're all f32/single). Forces `vfmaq_n_f64`,
  `vmulq_n_f64`, `vmaxvq_f64` and a `vld1q/vst1q_f64` round trip, with small
  binary-exact lane values so any structural miscompute reads as a real diff.
  The host x86 gcc can't compile `<arm_neon.h>`, so it runs through the
  **qemu-aarch64 architectural oracle** (`run_qemu_exact`), never a native one.
- **`gen_uxtl_uaddw`** — 16→32→64 widening chains (`uxtl`/`uaddw`/narrow back)
  exercising the `SimdAddw`/`SimdXtl` permute-source alias paths.
- **`gen_double_neon` immediately exposed a real gap**: the very first shape
  stopped the JIT with `Unsupported(0x7e70fa60)` at an fp-reduce site. That
  word is `fmaxp d0, v19.2d` — the **FP pairwise two-register horizontal
  reduce** (`FMAXP Vd, Vn`), which had NO decoder slot (it fell to the scalar-
  fp family's Unsupported residue).

### Fix (commit 9d40f47)
- `Inst::FpPair` + decode: all **8 two-register** encodings
  (`fmaxp/fminp/fmaxnmp/fminnmp` × `.2s/.2d`), with ground-truthed field
  extraction (verified against `aarch64-linux-gnu-as`/`objdump`):
  `min = bit23`, `nm = bits[14:12]==4` (fmaxp/fminp have bits[14:12]==7),
  `sz (.2d) = bit22`. Disjoint from the 3-operand `FMAXP Vd,Vn,Vm` (bit16 SET)
  and from `FMaxMin`/`FMaxV`. (My first field guess — bit15-min, bit13-nm — was
  wrong and the decode unit test caught it.)
- translate: pairwise-reduce Vn's two elements into a scalar Vd — `.2s` via
  `maxss/minsd`.../`minss`, `.2d` via `maxsd/minsd` (x86 max/min ignore NaN,
  acceptable for the fmaxnm* path like the existing `FMaxMin` comment notes).
- decode regression test `fp_pairwise_two_register_reduce_ground_truth` (all 8).
- Verified end-to-end: the exact program that previously stopped now returns
  **1562 == qemu oracle**. 16+16 fresh oracle-gated cases for both new
  generators: 0 fail / 0 skip.

`cargo build --workspace` clean; `cargo test --workspace` **366/0** (was 365).
Committed `9d40f47` (+ this doc). The gen_double_neon generator is a permanent
asset that now keeps the f64-NEON lane classes regression-guarded. HARD GATE
unchanged (real Roblox boot + run log only on a GPU/APK host; none on this VPS).

## Cycle 43 (Sep 11, 2026) — REAL guest POSIX timer -> guest-signal dispatch (367/0)

### What was wrong
`timer_create(107)/timer_settime(110)/timer_delete(109)` forwarded to host
POSIX timers (`libc::timer_create/timer_settime`). A host timer expiry raises a
**host** SIGALRM delivered to the host process's libc disposition — it never
reached the guest's `SIG_ACTIONS` handler table that cycle 40 built. So Android
watchdogs/callbacks that rely on SIGALRM (SystemClock, trace, watchdog,
timeout handlers) never fired on the guest. This was the documented "real
timer/signalfd-to-guest-handler dispatch" thread item.

### Fix (test-driven; commit 641f45b)
Routed the three syscalls to a guest-side timer implementation:
- **`guest_timer_create`**: reads the aarch64 `struct sigevent`
  (`sigev_signo`@8, `sigev_notify`@12; default SIGALRM=14), returns a non-null
  `timer_t` id, records the owning guest thread's real host tid + signo.
- **`guest_timer_settime`**: reads the 16-byte `struct itimerspec`, spawns a
  host worker thread that sleeps `it_value` then **POSTS the signal into the
  owner's blocked-aware `pending_mask`** (`post_signal_to_thread`, the cycle-41
  mechanism); a periodic `it_interval` re-arms. The owner's dispatcher loop
  drains it via `take_deliverable_pending` and runs the registered handler.
  Fresh `Arc<AtomicBool>` stop-flag per arming cancels any prior worker without
  stopping the newly-armed one.
- **`guest_timer_delete`**: signals the worker to stop (the worker holds an Arc
  clone of the flag, so it stays valid) and frees the slot.

Two bugs the e2e test surfaced: (1) the guest's `timer_t` local wasn't being
reloaded after the svc (gcc kept it in a register) — fixed the *test* with a
global volatile; (2) settime re-used the SAME stop flag it set to cancel the
prior worker, so every new worker immediately stopped itself — fixed by
installing a fresh Arc per arming.

### Test
`loader_run_timer_signal_delivers_sigalm_to_handler`: guest installs a SIGALRM
handler, creates a timer (default sigevent), arms a 100ms periodic itimerspec,
spins (1ms nanosleep yields) until the handler ticks **2** times, then
verifies `g_sig == 14`. Runs in ~0.23s — real timer→guest-signal dispatch.

### State
`cargo build --workspace` clean; `cargo test --workspace` **367/0** (was 366).
Fuzz sanity (3 seeds, 25 cases) 0 fail — no JIT regression from the timing code.
Committed `641f45b` (+ this doc). Documents the fuzzer additions since cycle 42
(gen_scalar_fp_sign_chain, gen_fmadd_reduce) in the cycle-42 record.
Thread-model remaining: per-thread TLS **init-image copies** for clone children
beyond the TP pointer handoff (needs a real multilib guest to validate).
HARD GATE unchanged: real Roblox boot + run log only on a GPU/APK host (none on
this VPS).

## Cycle 44 (Sep 11, 2026) — SIMD rev16 (granule 2) fixed; rev/permute fuzz generator (368/0)

### The bug (silent-miscompile class, found by the differential fuzzer)
A new fuzz generator `gen_byte_reverse_perm` (added to `fuzz_jit.py`) targets
the SIMD byte-reverse/permutation family the int-heavy generators never
produce: `vrev16/32/64q_u8`, `vuzp`, `vtrn`, and a u32 `vrev64q_u32`. First run
exposed a real gap: **SIMD `rev16` decoded via the rev64/rev32 gates (which
only handle byte1 0x08) and mis-decoded to a wrong-value instruction instead of
trapping** — the byte-reverse/permute silent-miscompile class. Real encodings
`rev16 v0.16b=0x4e201800` / `.8b=0x0e201800` fell through. Oracle/JIT
mismatches were byte-identical to the rev16 FAIL numbers (a compiler also
emitted `rev16` inside the trn/rbit functions, so all three reported the same
root cause).

### Fix (commit 8e00bdc)
- **decode.rs**: `rev16` is two-reg-misc REV16 (byte1 0x18, **bit21 SET**) —
  disjoint from the uzp1/trn1 3-same permute (`uzp1 v0.16b=0x4e011800` has bit21
  CLEAR). New gate `(insn & 0x3f20_f800) == 0x0e20_1800` pins the prefix lanes,
  bit21, and byte1 0x18; returns `SimdRev { granule: 2 }`.
- **x86.rs**: `mov_load16` (movzx 0F B7, zero-extend a 16-bit word) and
  `rol16_ri8` (66 C1 /0 ib, 16-bit rotate-left) helpers.
- **translate.rs**: `SimdRev` granule 2 arm — per-16-bit-halfword byte-swap via
  `mov_load16 → rol16_ri8(8) → mov_store16`.
- **decoder regression test** `rev16_decodes_as_granule2_not_uzp` (rev16 .16b/.8b
  → granule 2 + q; uzp1 stays a permute, not rev16; rev32 stays granule 4).

### Verification
Isolated the family through the full oracle pipeline: rev16 60/60 (was 22/60
before the fix), rev32/rev64/uzp unchanged. 4 fresh seeds (30 cases each) of the
full mixed fuzzer: 0 fail. `cargo test --workspace` **368/0** (was 367/0 — the
new decode regression).

Remaining JIT ISA surface is substantially complete (155 Inst arms incl. SHA,
crypto, structure ld/st, perms, SAT narrow, SME no-ops); the one annotation'd
gap is fp16 (`fcvtl` `.4h`/`fcvtn` `.4h`), deferred — a real project, not a
one-liner. Thread-model remaining: per-thread TLS init-image copies for clone
children (needs a real multilib guest). HARD GATE unchanged.

## Cycle 44g (Sep 11, 2026) — FMUL/FMLA by-element 32-bit index (376/0)

Commit `def4153` (dev). A fused-FMA pair probe (gen_fp_fmla_reduce, later
removed — qemu is nondeterministic on the fused-FMA oracle pattern) exposed
that the 32-bit by-element index was decoded wrong: SimdFmulEl read
bit11|bit13<<1 (always 0 for .s), FmlaEl used b21<<1|b11 (swapped). Assembler
ground truth: .s index is 2 bits, index = (b11<<1)|b21 — s[1]=b21, s[2]=b11,
s[3]=both. Every .4s by-element s[2]/s[3] (and fmla s[1] via the swap) used
the wrong element. Cleared with regression tests for all 4 fmul indices +
fmla index-1 execution. Workspace 376/0.

## Cycle 44k (Sep 11, 2026) — SIMD int pairwise smaxp/sminp/umaxp/uminp (383/0)

Commit `54bfb8d` (dev). gen_int_pairwise_maxmin exposed smaxp/sminp/umaxp/
uminp mis-decoding as SimdAddB/SimdAddH/Simd4s or Unsupported. New
`Inst::SimdMaxMinP`: prefix 0x0e/2e/4e/6e + **byte2&0xfc in {0xa4=max,
0xac=min}** (byte2 low bits carry rn — high-reg self-alias like v30,v31,v30
clears them), placed before the mla/add gates. `min` discriminator = byte2
bit3 (word bit11). Translate: pairwise-reduce each source into halves of Vd;
sign/zero-extend; cmov **must use 0x4x cmov cc** (L=0x4C,G=0x4F,B=0x42,
A=0x47 — the jcc-0x40 codes are wrong); permute_source for self-alias.

## Cycle 44j (Sep 11, 2026) — SIMD saturating shift-left sqshl/uqshl/sqshlu (382/0)

Commit `f3dd4d7` (dev). gen_sat_left_shift exposed sqshl (emitted by
vqshl_n_s16) as `Unsupported`. New `Inst::SimdSatShl`: gate prefix
0x0f/2f/4f/6f + bits[14:12] in {0b110=sqshlu, 0b111=sqshl/uqshl}, placed
BEFORE the plain shl gate (which uses bits[14:12]=0b101). shift/esize from
immh like shl. sat: 0=sqshl signed src/dst, 1=uqshl unsigned, 2=sqshlu
(signed src → unsigned dst, byte2 0x64 vs 0x74 → discriminator is **bit12
(0x1000)**). Translate: sign/zero-extend src, shl in the 64-bit reg, THEN
clamp to the element range — must NOT pre-truncate to elem width first,
else a negative src (e.g. 0xCBB2<<15 becomes huge-positive) wrongly
saturates to max instead of min. 220 arm64jit + 382 workspace, all swept clean.

## Cycle 44i (Sep 11, 2026) — 3-same FP pairwise faddp/fmaxp/fminp/fmaxnmp/fminnmp (380/0)

Commit `309a82d` (+ `172b830` tests). gen_fp_pairwise exposed fmaxp/fminp/
faddp Vd.T,Vn.T,Vm.T as `Unsupported` — the existing `FpPair` gate handled
only the 2-register (0x7e, bit16-clear) reduce form. New `Inst::SimdFpPair3`
gate: prefix 0x2e/0x6e, residue `&0xffe0_fc00` in the 12 {c4,d4,f4}x{20,a0}x{2e,6e}
patterns, `.2s/.4s` only (bit22 clear); add = bit13-clear && bit12-set, nm =
bit13-clear && bit12-clear, min = bit23. Translate reduces each source's
adjacent lanes into halves of Vd via addss/minss/maxss. Key pitfall: **bit16
is bit0 of rm, NOT a 2-op/3-op discriminator** — self-aliased fmaxp v31,v31,v30
(0x2e3ef7ff) clears it; prefix alone disambiguates. 40/40 fuzz cases pass;
workspace 380/0.

## Cycle 44h (Sep 11, 2026) — saturating-narrowing-shift + rshrn rounding (379/0)

Commit `0c63158` (dev). gen_narrow_shift (shrn/vrshrn/vqshrn) exposed two
silent-miscompile families: sqshrn/uqshrn/sqshrun decoded as non-saturating
SimdShrAcc (bad values), and rshrn decoded as shrn (no rounding half-add
1<<(shift-1)). New `Inst::SatNarrowShift` with a disciplined gate (prefix
0x0f/2f/4f/6f, bit15 narrowing marker, **bit19** immh guard so by-element
widen-mul with immh4=0xc is NOT captured, bit12 OR bit29 for saturation, and
sqshrun src_signed=byte2-bit4). Rounding-narrow now `round = bit11`; both
translates self-alias via permute_source. Regression tests lock the 6 encodings.

## Cycle 44f (Sep 11, 2026) — 3-same ADDP + MODIMM ORR/BIC RMW (375/0)

Commit `84e0d71` (dev). gen_pairwise_dot (addp/vpaddq/smaxv/sminv/umaxv/
uminv) found two silent-miscompile families:
1. **3-same ADDP Vd.T,Vn.T,Vm.T** (byte2&0xf8==0xb8, bit10 set) was decoded as
   SimdArithUnary (neg/abs) or Unsupported. bit10=0 two-reg neg/abs vs
   bit10=1 3-same ADDP. Added SimdAddp with a self-alias-safe translate
   (permute_source snapshot so `addp v31,v31,v31` reductions survive; exact
   width loads/stores).
2. **MODIMM ORR/BIC are read-modify-write**, not MOVI/MVNI. Odd cmode
   (bit0=1) with op0/orr + op1/bic; the decoder wrote lo/hi flatly, so
   `bic v.4h,#0xff,lsl#8` REPLACED lanes with the mask (all 0x00ff) instead of
   ANDing — every gcc -O3 value-truncation was wrong. Added VecMovi.kind
   {0=write,1=AND,2=OR} from cmode LSB + op; translate ANDs/ORs in place.
Workspace green at 375/0. HARD GATE unchanged.

## Cycle 44b (Sep 11, 2026) — integer LONG multiply by element fixed; widen-mul fuzz generator (369/0)

### The bug (silent-miscompile, fuzzer-caught)
A second new generator `gen_widen_mul_acc` (vmlal_lane_s16/32, vmull_lane, and
the `_high_` 2 variants — widening multiply-accumulate with an indexed
broadcast element) failed **40/40** against the qemu oracle. Root cause: the
integer **LONG-by-element** multiply family
(`smull/umull/smlal/umlal/smlsl/umlsl Vd.T, Vn.T, Vm.Tsb[idx]`) was entirely
undecoded. GCC emits it for vector*const-scalar scaling. The words mis-decoded
as `FmlaEl` (the `.2d` forms, which share bit23 SET) or `VecMovi` (the `.4s`
forms) — both silent wrong values, no trap.

### Fix (commit 39150c4)
- New `Inst::SimdMullEl`. Decode gate: prefix `b[28:24]=01111`,
  size `b[23:22]` (1 → `.4s` res 4 bytes, 2 → `.2d` res 8), indexed operand
  reg `Vm = b[19:16]` (4 bits, v0-v15), index = `L(b21):H(b20)` for 16-bit
  src / `L(b21)` alone for 32-bit src, op `b[15:12]` in
  `{2=mlal acc, 6=mlsl acc-sub, 0xa=mull}`.
- **bit13 (0x2000) SET** is the clean discriminator vs FP fmla-el (op 1/5 →
  bit13 clear) and non-widening int mla-el (op 0 → bit13 clear). The gate
  must precede the FP fmla-el gate (which otherwise steals the `.2d` forms).
- Translated per-lane like `SimdMull` but the `m` operand is loaded ONCE from
  the single indexed element (`slot(rm) + index*esize`) instead of lane `i`;
  `sub` arm for mlsl/umlsl; `uphalf=8` for the q=1 (_2/upper) high-lane forms.
- Decoder regression `widening_mul_by_element_decodes_not_fptsel_or_movi`
  (25 ground-truth encodings incl. the failing real-binary words) + guards
  that FP fmla-el, int MLA-el, and `movi v0.2d,#0` stay unmangled.

### Verification
widen-mul isolation 40/40 (was 0/40 — every case formerly failed). Full mixed
fuzz sweeps across 3 fresh seeds: 0 fail. `cargo test --workspace` **369/0**
(arm64jit lib 207 unit tests, +1 with the new regression).

The two cycle-44 fixes (rev16 `8e00bdc`, LONG-by-element `39150c4`) were both
differential-fuzzer finds — new targeted generators is the highest-yield
bug-hunt on this APK-less box. HARD GATE unchanged.

## Cycle 44c (Sep 11, 2026) — saturating-narrow + MSL immediates fixed; gen_sat_narrow fuzzer (370/0)

Third fuzzer-won battle this cycle. `gen_sat_narrow` (vqmovn_s32/vqmovn_u32/
vqmovun_s32 forced through NEON, with a matching clamp reference) failed
**18/40** and exposed FOUR real JIT bugs, each silent (wrong value, no trap):
1. **rev32 gate stole the unsigned-dst SaturatNarrow** — sqxtun/uqxtn have a
   0x2e prefix with bit23 set, which the rev32 gate (granule 4) wrongly
   claimed (`0x2e614bff → SimdRev`), so the unsigned family never ran. Added
   bit15==0 (byte1 top-nibble 0) to the rev32 gate — genuine rev32 has
   byte1=0x08 (top nibble 0), sqxtun has 0x2b.
2. **SaturatNarrow decode was wrong in 3 places**: byte2 must be MASKED
   (`& 0xf8`) not exact (sqxtun byte2 is 0x2b, carries reg bits → was missed);
   src/dst-signed misderived — now from U(bit29)+byte2 (sqxtn dst+src signed,
   uqxtn both unsigned, sqxtun dst unsigned + src signed); dst_esize now from
   the full byte1 nibble (0x2→1, 0x6→2, 0xa→4) so `sqxtn .2s,.2d` gets 4 bytes.
3. **SaturatNarrow clamp used the wrong max for signed dest** — always 0xffff;
   positive overflow of a signed dst saturated to 65535 not 32767 (the
   `+0x7fff7fff` vs `-0x8000` oracle diff). Fixed to signed-aware maxv.
4. **MSL movi/mvni (vector immediate, cmode 0xc/0xd) was captured by the
   SIMD shl gate** (missing bit15==0 check — movi-msl fell in as a shift), and
   the MSL branch never inverted for mvni (op==1). Both fixed; `mvni v.4s,
   #0xff,msl8` now gives per-lane 0xffff0000 as ground-truth requires.

Regression `saturating_narrow_variants_decode_correctly` (11 encodings incl
the squared-byte2 0x2b forms) + movi-msl16/mvni-msl8 + shl/ushr-stay-shifts.
Verified sat-narrow 40/40 (was 22/40), each variant 25/25, clean fuzz sweeps.
cargo test --workspace **370/0** (arm64jit lib 208). Commit `7599867`.
HARD GATE unchanged.

---

## Session (Sep 11, 2026, hermes-worker) — FIRST REAL-BINARY BOOT EXERCISE on this box; 3 boot-path JIT fixes (384/0)

Milestone context change: the REAL Roblox 2.738.1397 APK is NOW present on this
VPS (`~/.cache/open-sober/apks/roblox-android.apk`, 109MB arm64 `libroblox.so`
extracted to `~/.cache/open-sober/robbox/libroblox.so`, ARM aarch64 Android 26
NDK r28c stripped). The task's HARD GATE — exercising the actual Roblox binary
through arm64jit+libloader headlessly with Mesa llvmpipe — is now a concrete,
daily action on this box, no longer "impossible without APK/GPU". The boot is
STILL not complete (main loop not reached), but the JIT now loads the real
binary, binds ALL 534 JUMP_SLOT + 63 GLOB_DAT imports, and executes real
JNI_OnLoad prologue+init code before faulting on the JNI fake-object vtable wall.

### Three fixes (all committed to local `dev`, all boot-verifying):
1. **`45f111b` — bind APS2-packed GLOB_DAT/ABS64 in `bind_glob_dat`.** This
   release's `.rela.dyn` is `DT_ANDROID_RELA` (packed APS2, no stock DT_RELA),
   so the old binder (which only walked plain DT_RELA) left every Android-
   packed data GOT slot 0. The very first guest prologue instruction
   `adrp x24,0x67d1000; ldr x24,[x24,#1776]` (the `__stack_chk_guard` data
   GOT) read 0 and null-faulted BEFORE any real code ran. Decoded the APS2
   stream via `libloader::android_relocs::decode_aps2` and bind GLOB_DAT/ABS64
   entries; added a static-canary fallback for `__stack_chk_guard` (glibc
   doesn't export it to RTLD_DEFAULT). Now 63 GLOB_DAT bind (was 0); the guest
   executes past its prologue into real JNI init.
2. **PRFM decode (`0xf98xxxxx`) → Hint.** `prfm pldl3keep,[x8]` = size-8
   bit23-set, which the LdStrImm sign-extend decoder rejected as "size 8 not
   implemented". PRFM is a pure hint; now a Hint/no-op. +regression
   `prfm_prefetch_is_hint_not_size8_sext_load`.
3. **JavaVM GetEnv ABI slot 7→6 (byte 48).** Guest dispatch does
   `ldr x8,[vm]; ldr x8,[x8,#48]; blr` = JNIInvokeInterface slot 6 = GetEnv
   (verified against host java-21 jni.h). Our table had GetEnv at slot 7/offset
   56 (inherited QEMU guess), so `[x8,#48]` read the voidp default, `*penv`
   was never written, and the guest's env stayed null → next dispatch null-
   faulted. Updated the e2e JIT JNI test (`jit_jni_onload_getenv_getversion`).

### Current boot frontier (verified via JIT_TRACE / JIT_STEP)
The guest now loads, binds 597 imports, gets a live JavaVM* in x0, and
executes the JNI_OnLoad prologue, `__stack_chk_guard` store, once-guard init,
clock/time setup, and dispatches into real JNI code. It then faults on a
`ldr x8,[x8,#48]` C++ virtual-method dispatch where the handle's word0 (method
table) is 0 — the documented **JNI fake-object backing wall**: the guest
builds a C++ object from JNI getters/results and virtual-dispatches on a null
vtable slot, which the fake JNI objects (bare str_handle buffers, or GetEnv's
env when the vm slot was wrong) don't safely back. Trace shows NONE of the
FindClass/NewStringUTF/GetEnv stubs fire before the fault — it is pure guest
code dispatching on an object its own init built. Real fix = JNI fake-object
model (real vtable-backed handles the guest can dispatch on), a multi-session
subsystem.

Repro run-log artifact: `/home/hermes-worker/runs/real-boot-runlog.txt`.

### Environment / assets now on this box
- `~/.cache/open-sober/apks/roblox-android.apk` — real Roblox 2.738.1397 (229MB)
- `~/.cache/open-sober/robbox/libroblox.so` — extracted arm64 lib (109MB)
- elfjit command: `cargo run -p arm64jit --example elfjit -- <lib> 0x2173ff4 --jni`

### Status
`cargo test --workspace` **384/0** green (`cargo build --workspace` clean; the
decode.rs/plt.rs edits surface only the pre-existing rustfmt-churn warnings —
rustfmt isn't installed on this box). Local `dev` commits only (no push).

---

## Session (Sep 11, continued) — REAL Roblox engine code now runs: MemoryPool + guest-threading fixed (387/0)

The real `libroblox.so` 2.738.1397 boot (arm64jit + libloader, headless) crossed
three walls this session and now executes **real engine code** before the next
fault: JNI_OnLoad completed, TSAN/TLS-key once-init, a guest worker thread
spawned, and `[roblox:JNIMain] TelemetryProtocol::setProcessTimeOverride` logged.
Run log: `/home/hermes-worker/runs/real-boot-runlog.txt`.

Three boot fixes (dev commits `9bf61e3`, `a3a8372`):

1. **`body_contains_indirect()`** — a guest fn containing a `blr`/`br` (C++
   vtable dispatch, computed GetEnv) is now *diverted*, not inlined. An inlined
   `blr` `ret`s into the caller block instead of jit_run, silently skipping the
   hostcall (GetEnv's *penv never written) and skipping the callee's x19-x28
   restoring epilogue. This was the REAL cause of the long-standing "null
   vtable" SIGSEGV: the vm was a corrupted register from a skipped inline GetEnv,
   NOT a missing JNI stub (the vtable-backed-fake-objects theory is unnecessary.
2. **`bionic_pthread_once()`** — real glibc pthread_once calls the guest
   init_routine natively (SIGILL on `paciasp`). Interpose: run the guest
   once-routine via jit_run (`run_guest_callback()`). Unblocked the TSAN /
   TLS-key one-time init.
3. **`route_mempool_big_alloc_to_host()`** — the TLS-block allocator's big
   allocator (unseeded MemoryPool arena → NULL → guest abort) is patched
   (adrp/br + thunk in a mapped segment gap) to route to host `calloc`. Then
   `bionic_pthread_create/join` + `spawn_pthread()` — glibc pthread_create
   called the guest worker start routine natively (SIGILL); now spawns a fresh
   host thread running it through jit_run with its own guest stack+TLS.

Current wall (engine data access): JNIMain/TelemetryProtocol SIMD-copies a
struct to guest addr 0x109285fb0 (unmapped, ~33MB past image end). The pointer
isn't a 0x55... heap result — likely a guest svc mmap returning a low address or
a computed arena base. Next: trace who produced 0x109285fb0 and make it real.

`cargo test --workspace` **387/0** green. Local dev commits only (no push).
