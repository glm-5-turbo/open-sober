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

## Current Status (July 19, session 2 - late)

### ✅ Done

1. **Custom QEMU built** — Static 41MB binary at `/tmp/qemu-10.2.1/build/qemu-aarch64` (VDSO disabled). Source at `/tmp/qemu-10.2.1/`.

2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — intercepts `pthread_mutex_lock`/`trylock`/`timedlock` via trampoline table. Sanitizes `__kind` bits 2-6 (lock_full trigger) and clears stale `__owner` values.

3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled with safe stubs.

4. **QEMU bridge wiring** (`qemu.rs`) — `setup_bridges()` builds version bridges (libc.so, libm.so, libdl.so with LIBC_* version tags) and copies glibc into sysroot.

5. **Canary GOT patching** — `mprotect` + GOT write for `__stack_chk_guard` RELRO fix. Verified working.

6. **SIGSEGV handler** for QEMU JIT bug — one-shot handler skips past faulting instruction on QEMU's bad code pages.

7. **Mutex owner scan** — `dl_iterate_phdr` scan clears stale Bionic mutex owners across ALL loaded GSI libraries (~35K owners in libroblox.so alone).

8. **Rebuilt jni_shim.c** — complete file with JNI table, canary fix, SIGSEGV handler, and mutex scan all integrated.

### 🔴 Blockers

**Primary: `_dl_fixup` assertion crash**
The mutex owner scan's heuristic is too aggressive. It clears `__lock == 0 && __owner != 0 && __kind 0..3` patterns, but this matches PLT GOT entries too (especially unresolved lazy PLT entries). This corrupts the dynamic linker's relocation tables, causing:
```
Inconsistency detected by ld.so: dl-runtime.c: 63: _dl_fixup: Assertion `ELFW(R_TYPE)(reloc->r_info) == ELF_MACHINE_JMP_SLOT' failed!
```
**Fix:** Tighten the heuristic (check `__count` field at offset 4, check alignment, check `__spins` field at offset 20).

**Secondary: QEMU JIT code page bug**
QEMU 10.2.1 crashes (host-level SIGSEGV) when JIT-compiling code from specific text pages of libroblox.so at offset ~0x1f64000. The SIGSEGV handler skip-workaround is working but the `_dl_fixup` crash from the mutex scan should be fixed first — it might reveal more QEMU-dependent crashes after.

### 🟢 Next Steps (Priority Order)

1. **Tighten mutex owner scan heuristic** — Add `__count` field check (offset 4, should be 0 for unlocked mutex), `__kind` check stricter (only 0-3), and skip addresses that look like GOT/PLT entries.
2. **Re-test full JNI shim** — Once the scan is fixed, run the integrated jni_shim with all fixes.
3. **Debug remaining JNI_OnLoad crashes** — Likely more missing symbols, uninitialized globals, or JNI stub issues.
4. **EGL/GLES graphics stubs** — Mesa+zink approach. Not yet started.
5. **Window + input** — SDL2-based. Not yet started.

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom `/tmp/qemu-10.2.1/build/qemu-aarch64` (VDSO disabled) + system `/usr/bin/qemu-aarch64`
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs, patched for ANDROID_RELR/RELA)
- **Android NDK extracted:** `/tmp/ndk_extract/` and `/tmp/ndk.zip`
- **Bionic shim:** Built at `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** Built at `~/.cache/open-sober/android-env/jni_shim`