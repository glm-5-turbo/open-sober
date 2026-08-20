# Open Sober — Agent Handoff

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
