# Open Sober - Agent Handoff

## Project Overview

**Repo:** https://github.com/glm-5-turbo/open-sober
**Branches:** `stable` (release), `dev` (active development)
**Build:** `cargo build --release` — produces `./target/release/open-sober`
**Tests:** `cargo test --workspace` — 33 tests passing, zero warnings

Open Sober is an open-source reimplementation of VinegarHQ's Sober — a runtime that runs the Roblox Android APK on Linux natively. The original Sober does ARM64→x86-64 binary translation + Android compatibility layer, all in a 7.1MB Rust binary.

## What's Built

### Phase 1 - libbadcpu (`crates/libbadcpu/`)
CPU feature emulator — SIGILL signal handler for missing x86-64 instructions (POPCNT, MOVBE, LZCNT, TZCNT, BMI1). 5 tests. Written in Rust.

### Phase 2 - libloader (`crates/libloader/`)
Process sandbox/spawner — chroot isolation, ELF loader (mmap/mprotect segments), Android runtime env setup, Unix socket IPC. 16 tests. Written in Rust. 48 warnings (dead code, safe to ignore).

### Phase 3 - sober-services (`crates/sober-services/`)
Browser-based OAuth auth handler — starts local HTTP server, opens system browser for Roblox login, captures auth token. 11 tests. Written in Rust.

### Phase 4 - sober-core (`crates/sober-core/`)
Main binary orchestrator. `open-sober play --apk roblox.apk` is the main command. 1 test.

**Directory layout:**
```
open-sober/
├── crates/
│   ├── libbadcpu/          # SIGILL CPU emulator
│   ├── libloader/          # Process sandbox/ELF loader
│   ├── sober-services/     # Browser OAuth auth handler
│   └── sober-core/         # Main binary, QEMU launcher, JNI shim
├── vendor/
│   └── android2gnulinux/   # Git submodule for Bionic→glibc compat
├── crates/sober-core/src/
│   ├── main.rs             # CLI entry point (clap parser)
│   ├── qemu.rs             # QEMU user-mode launcher
│   ├── android_env.rs      # Android env setup (dirs, libs)
│   ├── apk.rs              # APK extraction (nested app.zip format)
│   ├── config.rs           # Config management
│   ├── dirs_setup.rs       # Cache directory setup
│   ├── jni_shim.c          # ARM64 JNI loader stub
│   └── jni_stubs.h         # Android native function stubs
├── RECOMMENDATION.md        # Binary translation strategy doc
└── GRAPHICS_RECOMMENDATION.md  # Graphics translation strategy doc
```

**Key APK available at:** `~/Documents/Projects/open-sober/roblox-android.apk` (170MB, not in git due to size)
**APK structure:** Contains `assets/app.zip` → `config.arm64_v8a.apk` → `lib/arm64-v8a/libroblox.so` (101MB ARM64 binary)

## Running the Binary

```bash
cd ~/Documents/Projects/open-sober
./target/release/open-sober play --apk roblox-android.apk
```

Current flow when you run `play`:
1. Parse CLI, load config
2. Check for APK at specified path or default
3. Set up Android env directory structure at `~/.cache/open-sober/android-env/`
4. Build android2gnulinux runtime (direct `cc` call, not broken Makefile)
5. Extract native libs from APK to `~/.cache/open-sober/libs/lib/`
6. Build ARM64 JNI shim via `aarch64-linux-gnu-gcc` (cross-compiler)
7. Launch QEMU user-mode with JNI shim as entry point
8. JNI shim calls `dlopen("libroblox.so")` → currently FAILS on symbol resolution

## What Works

- ✅ APK extraction (handles the nested `assets/app.zip` → `config.arm64_v8a.apk` format)
- ✅ Android env directory creation (/data, /system, /sdcard structure)
- ✅ android2gnulinux build (cross-compiled for ARM64 as `libdl_aarch64.so`)
- ✅ Bionic libs extracted from Android NDK r28 (libc.so, libdl.so, libm.so, liblog.so, libandroid.so, etc.) — stored at `~/.cache/open-sober/android-env/system/lib64/bionic_*.so`
- ✅ ARM64 QEMU can run executables (`qemu-aarch64 -L <sysroot> <binary>`)
- ✅ JNI shim compiles for ARM64 and loads under QEMU
- ✅ QEMU can `dlopen` libroblox.so (now gets symbol resolution errors instead of crashing)

## The Blocker: Bionic vs glibc Symbol Versioning

The Roblox Android APK's `libroblox.so` is compiled against **Android's Bionic libc** (not glibc). When loaded under QEMU user-mode, QEMU uses the host's glibc-based dynamic linker (`ld-linux-aarch64.so.1`). Bionic uses symbol version `LIBC` while glibc uses `GLIBC_X.XX`. libroblox.so references symbols like `abort@@LIBC`, `accept@@LIBC`, etc. which glibc doesn't provide.

**What we've tried:**

1. **android2gnulinux** (vendor submodule) — bridges Bionic→glibc but only targets 32-bit ARM/x86. The codebase has 32-bit assumptions (`Elf32_Addr`, `unsigned` for pointers) that break on ARM64. We cross-compiled it for ARM64 but it doesn't work due to pointer size issues.

2. **NDK Bionic libs** — Extracted `libc.so`, `libdl.so`, `libm.so`, `liblog.so`, `libandroid.so` from Android NDK r28 API 26. These are link-time stubs, not runtime libraries. They have `LIBC` versioning but aren't full runtime implementations.

3. **Versioned symbol shim** — Created a C wrapper library that re-exports glibc symbols with `LIBC` version tag using linker version scripts. Partially works but keeps hitting missing symbols (AAssetManager_fromJava, ALooper_prepare, AMediaCodec_*, etc. from other Android NDK libs).

4. **`libbionic_shim.so` (auto-generated)** — A Python generator (`crates/sober-core/src/gen_shim.py`) reads `libroblox.so`'s symbol table and emits dispatch-table trampolines for all 410 `@LIBC`-versioned symbols. Uses `.symver` directives to tag each export with the correct version. The shim is LD_PRELOAD'ed and forwards calls to ARM64 glibc (`libglibc.so` in the sysroot).

   **Status**: 10-symbol proof-of-concept WORKS under QEMU (strlen correctly dispatches through the shim to glibc). Full 392-symbol version compiles but the constructor segfaults — likely one of the dlsym() calls in the constructor chain triggers a NULL dereference. The constructor fills a dispatch table via `dlsym(RTLD_NEXT, ...)` for all symbols. Investigation is ongoing.

   Generated files:
   - `gen_shim.py` — the generator
   - `bionic_shim.S` — dispatch table trampolines (adrp+ldr+br pattern, avoids PLT circularity)
   - `bionic_init.c` — constructor + bionic-only C stubs
   - `bionic_version.ver` — version script

## Two Recommended Approaches Forward

### Approach A: Full Android System Image (Recommended for quick progress)

Download a proper Android system image for ARM64 (from Google's factory images or a custom ROM). Extract the full `/system/lib64/` directory tree and use it as QEMU's `-L` sysroot. This gives you the **actual** Bionic runtime with all libraries properly versioned.

Steps:
1. Download `gsi_arm64` system image from Google
2. Mount/extract to get the full `/system/lib64/` with real `libc.so`, `linker64`, etc.
3. Point QEMU at this as the sysroot
4. The JNI shim + libroblox.so should load because all dependencies are real Android runtime libs

### Approach B: Find x86-64 Android APK (Best performance)

The Roblox Android app sometimes has an x86-64 config split (`config.x86_64.apk`). If we can get an APK with x86-64 native libs, we can use **android2gnulinux** on x86-64 directly — **no QEMU needed at all**. android2gnulinux was designed for this exact use case on x86.

Steps:
1. Download a Roblox APK from a source that includes x86-64 native libs (some APK mirrors have the full multi-arch version)
2. Extract `lib/x86_64/libroblox.so` instead of `lib/arm64-v8a/`
3. Use android2gnulinux's linker directly on x86-64 to load it (no QEMU required)
4. This would be much faster and closer to what original Sober does

### Additional Work Needed After Either Approach

- **EGL/GLES→Vulkan graphics wrapper** — Mesa zink driver is the recommended approach (see GRAPHICS_RECOMMENDATION.md)
- **Window creation** — Need to create an X11/Wayland window for the game to render into
- **Input handling** — Bridge Android touch events → mouse/keyboard
- **JNI function table** — The JNI shim needs to implement ~233 JNI functions that libroblox.so will call after loading (FindClass, GetMethodID, NewStringUTF, etc.)

## Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** qemu-aarch64 10.2.1 installed
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-arm-linux-gnueabihf) installed
- **Android NDK r28:** Extracted at `/tmp/ndk_extract/` and `/tmp/ndk.zip` (690MB, kept for Bionic lib extraction)
- **Bionic libs:** At `~/.cache/open-sober/android-env/system/lib64/bionic_*.so`
- **SearXNG:** Local instance at `http://localhost:9898` for web research
- **GitHub token:** Authenticated as `glm-5-turbo`, repo `open-sober`