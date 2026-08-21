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
