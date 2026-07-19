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

## Current Status (July 19, session 4 final — JIT bug FIXED at root cause)

### ✅ Complete (all sessions)

1. **Custom QEMU built + PATCHED** — Static 41MB binary from `/tmp/qemu-10.2.1/`. **Patched copy** at `~/.cache/open-sober/qemu-patched` with `CF_NO_GOTO_TB` fix (see below).
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — trampoline-based mutex interceptors.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled. Name-tracked pointers for FindClass/GetMethodID uniqueness.
4. **QEMU bridge wiring** (`qemu.rs`) — version bridges for libc/libm/libdl.
5. **Canary GOT patching** — stack_chk_guard write via mprotect.
6. **SIGSEGV handler** — RELRO faults, self-write JIT bugs, NULL deref detection.
7. **Pre-mprotect RELRO** — ~464 pages made RW before JNI_OnLoad.
8. **Pre-resolved trampoline table** — 20+ functions resolved via RTLD_DEFAULT.
9. **Canary check patched out** in code copy.
10. **JNI_OnLoad code copy** — 128KB copy with adrp fix (all 4 immlo variants, 9010 instructions).
11. **BL/B/B.cond/CBZ/TBZ offset fix** — recalculation for calls outside copy range.
12. **GOT + init_array pre-mprotect** — explicit ranges for libroblox.so.
13. **PROT_NONE→PROT_RW double-mprotect** — forces QEMU TLB flush.
14. **Name-tracked JNI stubs** — unique pointers per class/method name.

### ✅ QEMU JIT BUG — ROOT CAUSE FIXED

**Root cause:** QEMU's TCG `goto_tb` mechanism. When QEMU chains TBs via direct `jmp` instructions, `tb_target_set_jmp_target()` writes a 4-byte displacement to the JIT code buffer via `qatomic_set((int32_t *)jmp_rw, ...)`. If this `jmp` lands at the end of a host page (`...fffb8`), x86_64 SMC detection triggers a host SIGSEGV that QEMU cannot recover from.

**Patch** (`accel/tcg/cpu-exec-common.c`): `cflags |= CF_NO_GOTO_TB;` unconditionally. TBs exit through hash lookup instead of direct jmp patching.

**5/5 runs stable** with patched QEMU (vs ~2-3/5 before).

### 🟡 Current Blocker — JNI_OnLoad initializing very slowly (100% CPU, hasn't returned)

With patched QEMU, JNI_OnLoad runs at 100% CPU for minutes. 99.6% unique TB addresses confirms it's actively executing new code across ~30 GSI libraries, not looping. Under QEMU JIT (~5-10% native speed), the massive init sequence takes very long.

### 🎯 Next Steps

1. **Let JNI_OnLoad run to completion** — try 10-minute timeout
2. **Check strace for futex/condvar waits** — if threads are spawned, need threading support
3. **Implement RegisterNatives properly** — log and implement key native methods
4. **Once JNI_OnLoad returns** — Roblox main loop (render, network, scripts)

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
Rebuild QEMU after patching: `cd /tmp/qemu-10.2.1/build && ninja qemu-aarch64 && cp qemu-aarch64 ~/.cache/open-sober/qemu-patched`

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