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

## Current Status (July 20, after session 8)

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

The Phase 1a progressive patch (`patch_jni_onload_phase1`) only NOPs the clock/time
init, so JNI_OnLoad runs partial native init. The next agent should:

**Phase A — Port the patched QEMU into the repo (critical long-term fix)**

The custom QEMU at `/home/code-agent/.cache/open-sober/qemu-patched` was built
from /tmp/qemu-10.2.1/ (now deleted). The patches applied were:
1. CF_NO_GOTO_TB — prevents chained TB linking in TCG, fixing SMC crashes
2. tb_set_jmp_target no-op — related to goto_tb patching

Without the QEMU patches, the SMC (self-modifying code) crashes return. The
patched QEMU must be preserved or rebuilt from source. Check:
- `~/Documents/qemu-10.2.1/build/qemu-aarch64` (may exist from original build)
- Or rebuild from upstream QEMU 10.2.1 tarball with the two patches reapplied

**Phase B — Fix nativeSetAssetPath hang (Session 11 blocker)**

The hang is inside `nativeSetAssetPath` (binary offset `0x273de0c`) which is
called from JNI_OnLoad at `0x1f64eb8`. The function calls a helper at `0x273dd4c`
which calls `FindClass(env, x1)` where `x1` is uninitialized. The caller should:

1. **Confirm the hang point** — Replace the raw ARM condvar shim with a C-based
   one that writes to stderr each time it's called, then sleeps 100ms and returns 0.
   Key: the shim must be a proper ARM C function (not a host pointer) to work
   with the bionic trampoline table and PLT GOT patching.

2. **Use QEMU's gdbstub** (`-g 1234`) to attach `aarch64-linux-gnu-gdb` and get
   a proper backtrace when the process hangs. The SIGALRM handler's `x29`/`x30`
   register reads are unreliable under QEMU — gdbstub provides accurate frame
   unwinding.

3. **Strace the syscalls at hang time.** At the hang point, no futex call
   appears in `-strace` output, suggesting either:
   a) The condvar is being shimmed by the raw ARM shim (returns immediately, so
      no syscall is made)
   b) The process is in a tight loop that doesn't involve any syscalls

4. **If the hang is in nativeSetAssetPath's helper** (function at `0x273dd4c`):
   The helper calls `FindClass` via JNI with uninitialized `x1`. Our
   `stub_FindClass` handles any pointer but `strdup(garbage)` might fault.
   Fix: Set up `x1` to point to a valid class name string before the call,
   or NOP the helper call (`bl 0x273dd4c` at offset `0x273de2c`).

5. **Alternative: Full nativeSetAssetPath bypass.**
   NOPing `bl 0x273de0c` at JNI_OnLoad offset `0x1f64eb8` in addition to the
   clock init NOP. Note: this requires a SINGLE mprotect covering both NOP
   targets (they're on the same or adjacent pages), and each page restore
   must use the proper permissions.

6. **Build proper JNI stubs** for the 3 discovered classes and 13 methods.
   The next step after fixing the hang is to handle RegisterNatives properly
   (the registration function at `0x1f6594c` stores method IDs in BSS at
   `base + 0x64d9000` region, but our stubs return `track_ptr` values which
   the calling code stores as valid pointers).

**Phase C — Full JNI_OnLoad enablement**

Once the basic JNI stubs and condvar shim are working:

1. **Remove the Phase 1a NOP** (stop NOPing the clock init call).
2. **Fix the remaining crash** — when all NOPs are removed, the binary may
   hit a SIGSEGV from a different code path.
3. **Expand JNI stubs** to handle all classes/methods that JNI_OnLoad needs.
4. **Properly RegisterNatives** — call intercepted native methods with the
   correct signatures.

**Phase D — Integrate with the Rust orchestrator**

Once the C-based JNI shim works stably, update `qemu.rs` to use it as the
main entry point for `open-sober play --apk roblox.apk`.
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