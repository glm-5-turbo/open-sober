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

## Current Status (July 19, session 4 - early)

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

### 🟢 What Now Works

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

To rebuild jni_shim.c:
```bash
SYSROOT=~/.cache/open-sober/android-env
CRATE=~/Documents/Projects/open-sober/crates/sober-core/src
aarch64-linux-gnu-gcc -o "$SYSROOT/jni_shim" "$CRATE/jni_shim.c" -ldl
```

To rebuild bionic_shim.so (needed if bionic_init.c changes):
```bash
aarch64-linux-gnu-gcc -c -o /tmp/bionic_stubs.o "$CRATE/bionic_init.c" -fPIC
aarch64-linux-gnu-gcc -shared -fPIC -o "$SYSROOT/system/lib64/libbionic_shim.so" \
  /tmp/bionic_symbols.o /tmp/bionic_stubs.o \
  -Wl,--version-script,"$CRATE/bionic_version.ver" \
  -Wl,-rpath,/system/lib64 -L "$SYSROOT/system/lib64" \
  -lglibc -lm -ldl -nostartfiles
```

(The bionic_symbols.o object file persists from a prior build — the assembly trampolines in bionic_shim.S rarely change.)

### 🔴 Current Blocker

**QEMU JIT bug on multiple GSI library code pages**

The QEMU JIT bug is NOT limited to the libroblox.so code page at 0x1f64000. Once JNI_OnLoad's prologue executes (via the code copy), it calls sub-functions in other GSI libraries (`libc++`, `libandroidicu`, etc.), and those libraries' code pages also trigger the same QEMU JIT bug (host-level SIGSEGV during JIT compilation).

The crash now appears as `qemu: uncaught target signal 11` WITHOUT any SIGSEGV handler output, confirming it's a host-level crash inside QEMU's JIT, not deliverable as a guest signal.

The SIGSEGV handler catches guest-level faults (page permission errors, bad addresses), but CANNOT catch QEMU's internal JIT crashes.

### 🎯 Next Steps (In Priority Order)

The QEMU JIT bug is the primary blocker. Options:

1. **Patch QEMU source** — The QEMU source is at `/tmp/qemu-10.2.1/`. Find and fix the JIT compilation bug. The bug causes SIGSEGV when JIT-compiling specific AArch64 instruction sequences. Options:
   - Look at `linux-user/` and `accel/tcg/` directories in the QEMU source
   - The bug might be in TCG code generation for specific ARM64 instructions (adrp+ldr combinations?)
   - Try disabling TCG optimizations in QEMU's configure step (`--disable-tcg` won't help but there might be other flags)
   - Patch `accel/tcg/translator.c` or `target/arm/translate.c`

2. **Build QEMU with debugging flags** — Rebuild with `--enable-debug` to get more info on where the JIT crashes.

3. **Try QEMU's `-tb-size` option** — Reduce the translation block cache size to force more frequent re-translations.

4. **Aggressive code copying** — When a code page triggers the JIT bug, use QEMU's `-strace` to learn which libraries/pages fail, then copy ALL of them. This is a losing battle if the bug is widespread.

5. **Upgrade QEMU** — Try Ubuntu 26.04's `qemu-user` package (might be newer than 10.2.1).

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