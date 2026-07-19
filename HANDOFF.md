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

## Current Status (July 19, session 4 — breakthrough)

### ✅ Complete (earlier sessions)

1. **Custom QEMU built** — Static 41MB binary at `/tmp/qemu-10.2.1/build/qemu-aarch64` (VDSO disabled). Source at `/tmp/qemu-10.2.1/`.
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — intercepts `pthread_mutex_lock`/`trylock`/`timedlock` via trampoline table. Sanitizes `__kind` bits 2-6 (lock_full trigger) and clears stale `__owner` values.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled with safe stubs.
4. **QEMU bridge wiring** (`qemu.rs`) — `setup_bridges()` builds version bridges (libc.so, libm.so, libdl.so with LIBC_* version tags) and copies glibc into sysroot.
5. **Canary GOT patching** — `mprotect` + GOT write for `__stack_chk_guard` RELRO fix. Verified address at `base + 0x6473438`.

### ✅ Fixed in session 3

6. **SIGSEGV handler for QEMU JIT bugs + RELRO writes** — Handles three cases:
   - *Self-write (pc == fault_addr)*: toggles page `PROT_NONE` → `PROT_RWX` to force QEMU JIT cache invalidation
   - *Other RELRO fault*: mprotects page and advances PC by 4
   - *NULL/-1 address*: chains to old handler (genuine bug, not JIT issue)
   - Loop limit of 500 faults prevents infinite loops

7. **Pre-mprotect of all GSI RELRO pages** — Reads `/proc/self/maps`, finds all `r--p` pages from GSI libraries (excluding glibc), mprotects them to `rw-p`. This prevents MOST RELRO write faults before they happen. (~464 pages fixed)

8. **Mutex sanitization wrappers installed in jni_shim.c directly**
   - Previous issue: `__bf_init_data` was never called (not a constructor), and calling `__bf_c_resolve` (which uses `dlsym(RTLD_NEXT)`) crashed under QEMU.
   - Fix: jni_shim.c now pre-resolves ~18 key functions into the trampoline table via `dlsym(RTLD_DEFAULT)` (which works under QEMU), and installs custom sanitize-and-lock wrapper functions directly at trampoline indices 38, 124, 780.

9. **Canary check patched out** — The `ldr x8, [x24]` at JNI_OnLoad+0x30 is replaced with `mov x8, xzr` in the code copy. The canary load was returning 0 despite the GOT value being non-zero (likely a QEMU JIT bug on that specific `ldr` instruction). Bypassing the canary entirely avoids this crash.

10. **JNI_OnLoad code copy with adrp fix** — The QEMU JIT bug is a HOST-level SIGSEGV when JIT-compiling certain code pages (~0x1f64000 offset in libroblox.so). JNI_OnLoad is at offset 0x1f64e58, right in the bad zone. The fix copies 128KB of code to a fresh mmap'd page, fixes all adrp instructions to target original absolute addresses, and calls the copy instead. This works around QEMU's JIT crash.

### ✅ Fixed in session 4

11. **BL/B/B.cond/CBZ/TBZ offset fix for code copy** — The code copy was only fixing `adrp` instructions (data access). BL/B/B.cond/CBZ/TBZ instructions that target code outside the 128KB copy range had wrong offsets because the copy is at a different virtual address than the original. The fix recalculates all PC-relative branch offsets to point to the ORIGINAL target address. This is critical for calls to PLT, GSI library functions, and other libroblox functions outside the copied range.

12. **adrp detection fix (all immlo variants)** — The adrp instruction has 4 variants depending on `immlo` (bits 30-29): 0x90, 0xB0, 0xD0, 0xF0. The old code only detected `immlo=00` (0x90), missing 75% of adrp instructions. The fix uses `(ins & 0x1F000000) == 0x10000000 && bit31==1` to catch all 4 variants. **9010 adrp instructions** are now fixed in the 128KB copy (up from 2154).

13. **Temporarily disabled mutex sanitization wrappers** — The `sanitize_and_lock` wrapper function re-entered glibc's `pthread_mutex_lock`, which triggered a QEMU JIT bug specifically on the second call. Without the wrapper (directly calling glibc's mutex functions), the code runs stably under QEMU for extended periods. The wrapper is guarded by `if(0)` — needs re-enabling with a fix.
    - Also pre-resolved `pthread_mutex_lock` (trampoline index 38) to bypass the lazy resolver (`__bf_c_resolve` uses buggy `dlsym(RTLD_NEXT)` under QEMU).

### 🟢 Current State — BREAKTHROUGH

**JNI_OnLoad now runs stably for 30+ seconds without crashing!**

Run the jni_shim:
```bash
ANDROID_ROOT=~/.cache/open-sober/android-env
/usr/bin/qemu-aarch64 \
  -L "$ANDROID_ROOT" \
  -E LD_LIBRARY_PATH=/system/lib64 \
  -E LD_PRELOAD=/system/lib64/libbionic_shim.so \
  -E ROBLOX_LIB=/system/lib64/libroblox.so \
  "$ANDROID_ROOT/jni_shim"
```

### 🟡 Current Blocker — JNI_OnLoad doesn't return (hangs)

**JNI_OnLoad runs stably but doesn't return.** After 30+ seconds, the function is still executing (0% CPU — sleeping/waiting).

JNI_OnLoad likely:
1. Spawns worker threads and waits for them to initialize
2. Tries to communicate with the Android Java runtime (which doesn't exist)
3. Is waiting on a futex/condition variable that will never be signaled

The QEMU JIT crash that previously blocked progress is now **fully worked around** with the combination of:
- Code copy with adrp fix (all 4 immlo variants)
- BL/B/B.cond/CBZ/TBZ offset fix  
- Mutex sanitization wrappers disabled (direct glibc call works)
- SIGSEGV handler for RELRO writes and JIT self-write bugs
- Pre-mprotect of RELRO pages

### 🎯 Next Steps (In Priority Order)

1. **Figure out why JNI_OnLoad hangs** — It spins up threads and waits. Options:
   - Use `-strace` to see what syscalls are blocking
   - Check if any required symbols are missing (Android runtime classes, JNI calls)
   - Enable JNI stub logging to see what JNI calls Roblox makes during init
   - The program might need more JNI stubs implemented (FindClass for specific classes, GetMethodID for specific methods)

2. **Re-enable mutex sanitization safely** — The `sanitize_and_lock` wrapper triggers a QEMU JIT bug on re-entry. The fix might be:
   - Use `__attribute__((noinline))` to prevent TCG cross-linking
   - Move the sanitize code inline into jni_shim.c instead of calling through bionic_shim trampoline
   - Simply skip sanitization if glibc's mutex works correctly (the Bionic ABI differences might not matter with glibc)

3. **Implement more JNI stubs** — Based on what JNI_OnLoad calls, add real implementations for key JNI methods:
   - `FindClass` needs to return the right class objects
   - `GetMethodID`/`GetStaticMethodID` might need real implementations
   - `RegisterNatives` needs to handle function registration

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom `/tmp/qemu-10.2.1/build/qemu-aarch64` (VDSO disabled, build from source at `/tmp/qemu-10.2.1/`) + system `/usr/bin/qemu-aarch64`
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs, patched for ANDROID_RELR/RELA)
- **Bionic shim:** Built at `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** Built at `~/.cache/open-sober/android-env/jni_shim`
- **Android NDK extracted:** `/tmp/ndk_extract/` and `/tmp/ndk.zip`

### Key Source Files

- `crates/sober-core/src/jni_shim.c` — Main JNI shim: loads libroblox.so, copies JNI_OnLoad to bypass QEMU bug, installs mutex sanitization wrappers, has RELRO page SIGSEGV handler
- `crates/sober-core/src/bionic_init.c` — Bionic shim C code: trampoline resolver (`__bf_c_resolve`), mutex sanitization, data object resolution
- `crates/sober-core/src/bionic_shim.S` — Auto-generated assembly trampolines (785 entries)
- `crates/sober-core/src/qemu.rs` — QEMU launcher: builds bridges, sets up environment, spawns QEMU process