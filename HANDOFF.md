# Open Sober - Agent Handoff

## ⚠️ CRITICAL RULES — READ FIRST

1. **NO WORKTREES.** Do NOT create git worktrees. Ever. The last agent created 8 worktrees and they all conflicted with each other.
2. **NO BRANCHES.** Work directly on `dev` branch. No feature branches, no topic branches.
3. **CLAUDE.md** at repo root has these rules — read it.
4. **Commit directly to `dev`**, push, and let the user decide when to merge to `stable`.
5. If you need to research something, do it inline or in a temp dir outside the repo.
6. **`cargo check --workspace`** before committing. **`cargo test --workspace`** before pushing.

## Project Overview

**Repo:** https://github.com/glm-5-turbo/open-sober
**Branches:** `stable` (release), `dev` (active development — work here)
**Build:** `cargo build --release`
**Tests:** `cargo test --workspace` — 33 tests passing, zero warnings

Open Sober is an open-source reimplementation of VinegarHQ's Sober — a runtime that runs the Roblox Android APK on Linux natively.

## What's Built

### Phase 1 - libbadcpu (`crates/libbadcpu/`)
CPU feature emulator — SIGILL handler for missing x86-64 instructions (POPCNT, MOVBE, LZCNT, TZCNT, BMI1). 5 tests.

### Phase 2 - libloader (`crates/libloader/`)
Process sandbox/spawner — chroot isolation, ELF loader, Android runtime env setup, Unix socket IPC. 16 tests.

### Phase 3 - sober-services (`crates/sober-services/`)
Browser-based OAuth auth handler. 11 tests.

### Phase 4 - sober-core (`crates/sober-core/`)
Main binary orchestrator. `open-sober play --apk roblox.apk` is the main command. 1 test.

**Directory layout:**
```
open-sober/
├── crates/
│   ├── libbadcpu/          # SIGILL CPU emulator
│   ├── libloader/          # Process sandbox/ELF loader
│   ├── sober-services/     # Browser OAuth auth handler
│   └── sober-core/         # Main binary, QEMU launcher, JNI shim, bionic shim
│       └── src/
│           ├── bridges/
│           │   ├── bridge_version.ver    # 21 LIBC_* variant definitions
│           │   ├── bridge_libc.c         # Weak stubs + Bionic stubs
│           │   ├── bridge_libdl.c        # LIBDL_ANDROID version
│           │   ├── bridge_libm.c         # Empty version shim
│           │   ├── build_bridges.sh      # Cross-compiles bridges
│           │   └── auto_stub.py          # Auto-generates missing lib stubs
│           ├── bionic_shim.S         # 392 ARM64 trampolines (auto-gen'd)
│           ├── bionic_init.c         # C stubs for Bionic-only symbols
│           ├── bionic_version.ver    # LIBC/N/O version tags for shim
│           ├── gen_shim.py           # Generates bionic_shim.S from libroblox.so
│           ├── jni_shim.c            # ARM64 JNI loader stub
│           └── qemu.rs               # QEMU launcher, builds shims/bridges
├── vendor/
│   └── android2gnulinux/   # 32-bit Bionic→glibc bridge (not used for ARM64)
├── RECOMMENDATION.md
└── GRAPHICS_RECOMMENDATION.md
```

**Key APK:** `~/Documents/Projects/open-sober/roblox-android.apk` (170MB, not in git)
**APK structure:** `assets/app.zip` → `config.arm64_v8a.apk` → `lib/arm64-v8a/libroblox.so` (101MB)

## Status: Bionic → glibc Symbol Bridge

### ✅ What Works

1. **Bionic shim** (`libbionic_shim.so`) — 392 ARM64 assembly trampolines forwarding to glibc. All 405 LIBC-versioned symbols covered. Zero missing. Zero performance overhead per call (direct branch after first resolution).
2. **JNI shim** — loads bionic shim, fills dispatch table via dlsym, initializes data objects (`__stack_chk_guard`, `stderr`, `environ`, etc.)
3. **Version bridges** — libc.so, libm.so, libdl.so define all LIBC_* variants that GSI libraries check. ~800 weak stubs with correct per-symbol version tags (LIBC_N/O/P/Q/R).
4. **libdl.so bridge** — defines LIBDL_ANDROID version with `android_get_exported_namespace` stub.
5. **Auto stub generator** — `auto_stub.py` discovers missing NEEDED libraries and creates minimal stubs.
6. **Full Git commit** — all changes committed on branch `worktree-close-the-gap`.

### 📈 dlopen Progress

dlopen("libroblox.so") passes version checks for:
- ✅ **libc.so** — 21 LIBC variants, 872 weak stubs
- ✅ **libm.so, libdl.so** — version definitions, LIBDL_ANDROID
- ✅ **libc++.so** — 170 symbols with correct version tags
- ✅ **libbase.so, liblog.so, libcutils.so, libutils.so**
- ✅ **GSI libs symlinked:** libandroid.so, libOpenSLES.so, libmediandk.so
- ✅ **Stubs created:** libandroidicu.so, libicu.so, tethering connectivity, nativeloader, statspull, statssocket, adb pairing

### ✅ LIBBINDER_NDK Fixed

The previous blocker was `android.hardware.common-V2-ndk.so` needing `AParcel_getDataPosition` with version `LIBBINDER_NDK` from `libbinder_ndk.so`. The issue:

- **The stub `libbinder_ndk.so`** (69KB, auto-generated) had NO version definitions — it only exported `libbinder_ndk_init`. The dynamic linker needs `LIBBINDER_NDK` as a version tag in `libbinder_ndk.so`'s VERDEF table.
- **The fix:** Replaced the stub with a symlink → `gsi_libbinder_ndk.so` (the real GSI library, 189KB), which has proper version definitions for `LIBBINDER_NDK`, `LIBBINDER_NDK30`–`LIBBINDER_NDK37`, and `LIBBINDER_NDK_PLATFORM`, and exports `AParcel_getDataPosition` under `LIBBINDER_NDK`.
- All of `gsi_libbinder_ndk.so`'s dependencies (`libbinder.so`, `liblog.so`, `libutils.so`, `libc++.so`, `libc.so`, `libm.so`, `libdl.so`) are already symlinked to real GSI versions — no cascade.

**GSI symlink strategy (established pattern):**
| Unversioned stub → Real GSI lib | Status |
|---|---|
| `libbinder.so` → `gsi_libbinder.so` | ✅ Already set |
| `libc++.so` → `gsi_libc++.so` | ✅ Already set |
| `liblog.so` → `android_liblog.so` | ✅ Already set |
| `libutils.so` → `gsi_libutils.so` | ✅ Already set |
| **`libbinder_ndk.so` → `gsi_libbinder_ndk.so`** | **✅ Fixed** |

### ❓ Remaining unknowns
- Whether `ld-linux-aarch64.so.1`, `libc.so.6`, `libm.so.6` NEEDED references in real GSI libs cause issues at runtime. These are glibc base references and should be served through the bionic shim, but may need stubs.
- `auto_stub.py` picks these up and creates stubs, but they loop infinitely (stubs reference nothing, so their NEEDED entries still appear missing).

### 🔬 The Problem (version namespace complexity)

GSI libraries have ~60+ unique version namespaces. The glibc dynamic linker checks per-library VERDEF tables, not globally — a symbol from LD_PRELOAD doesn't satisfy a versioned reference against another library. **The correct approach is to use the real GSI libraries directly, not to stub them**, as demonstrated by the `libbinder_ndk.so` fix.

## Recommended Path Forward

**Phase A (DONE):** Bionic shim + libc/libm/libdl version bridges.

**Phase B (DONE):** Iteratively resolved GSI library dependency chain. Key strategy change from the HANDOFF's original recommendation: instead of creating versioned stubs for every library (which requires replicating hundreds of version tags and symbols), the correct approach is to **replace auto-generated stubs with symlinks to the real GSI libraries**. This preserves their VERDEF tables, symbol versions, and dependency chains intact.

Remaining Phase B work:
1. Clean up `auto_stub.py` to skip glibc base libs (`ld-linux-aarch64.so.1`, `libc.so.6`, `libm.so.6`) and extend auto-detect of GSI libs to auto-symlink when possible
2. Remove the `libbinder_ndk.so` stub backup

**Phase C (NEXT):** Once `libroblox.so` loads, the next blockers will be:
- **JNI function table** (~233 JNI functions to stub — FindClass, GetMethodID, NewStringUTF, etc.)
- **EGL/GLES→Vulkan** (Mesa zink driver — see GRAPHICS_RECOMMENDATION.md)
- **Window creation** (X11/Wayland)
- **Input handling** (touch → mouse/keyboard)

## Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** qemu-aarch64 10.2.1 installed
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **Android NDK r28:** Extracted at `/tmp/ndk_extract/` and `/tmp/ndk.zip`
- **GSI ARM64 image:** Downloaded at `/tmp/gsi_arm64.zip`, mounted at `/tmp/gsi_mount/`
- **Bionic libs from GSI:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- **Roblox APK:** At `~/Documents/Projects/open-sober/roblox-android.apk`
- **GitHub token:** Authenticated as `glm-5-turbo`, repo `open-sober`
## Latest Runtime Test Results (2026-07-19)

### ✅ All Symbol Resolution Blockers Fixed

The following new stubs were added to `bionic_init.c` + `bionic_version.ver`:

**`@@LIBC_R` (NEW version block):**
- `_Unwind_RaiseException`, `_Unwind_DeleteException`, `_Unwind_SetGR`, `_Unwind_SetIP`
- `_Unwind_GetLanguageSpecificData`, `_Unwind_GetIP`, `_Unwind_GetRegionStart`, `_Unwind_Resume`

**`@@LIBC` string/number conversion:**
- `strtold`, `wcstod`, `wcstof`, `wcstold`
- `wcstol`, `wcstoll`, `wcstoul`, `wcstoull`
- `setlocale`, `sendfile`, `setbuf`, `swprintf`

**`@@LIBC_O`:**
- `strtof_l`, `strtod_l`

**`@@LIBC` filesystem/syscall:**
- `chdir`, `pathconf`, `truncate`, `remove`, `link`, `symlink`
- `fchmodat`, `openat`, `fdopendir`, `unlinkat`, `utimensat`

### ❌ Current Blocker
`dlopen("libc++.so")` segfaults in `.init_array` (C++ static constructor) at address 0x7fbbc.

All symbols now resolve cleanly (RTLD_NOW succeeds for the symbol phase). The segfault happens during libc++.so's 3 static constructors (`.init_array` has 3 entries). The crash address 0x7fbbc matches the first init_array entry.

**Hypothesis:** libc++.so's C++ static initializers manage global C++ objects (ios_base::Init, locale, etc.) that need working `new`/`delete` or `__cxa_*` runtime support. The bionic shim provides `__cxa_finalize@@LIBC` and `__cxa_atexit@@LIBC` but these may not be sufficient for full C++ runtime init under GSI libc++.

**To debug:**
1. Install `gdb-multiarch` and use QEMU's `-g` flag for GDB server:
   ```
   qemu-aarch64 -g 1234 -L ... ./jni_shim
   gdb-multiarch -ex "target remote :1234" ./jni_shim
   ```
2. Or try with `LD_BIND_NOW=1` and `LD_DEBUG=all` or `GLIBC_TUNABLES=glibc.rtld.dynamic_sort=1`
3. The three init array entries are at offsets 0x7fbbc, 0x7feb0, 0xc87d4 in libc++.so

### Key Architecture Notes
- `bionic_init.c` pattern for LIBC-versioned stubs:
  - `__attribute__((used)) __attribute__((externally_visible))` on the impl
  - `.symver(name_impl, symbol@@VERSION)` 
  - dlsym(RTLD_NEXT, ...) to call the real glibc version
- `bionic_version.ver` defines version blocks: LIBC_R, LIBC, LIBC_N, LIBC_O, LIBC_P
- The bridge `libc.so` provides the VERDEF table (22 LIBC variants) while glibc symbols keep their original GLIBC_2.17 versions
- The bionic shim (LD_PRELOAD'd) provides the @@LIBC-versioned aliases
- Each stub has a `return (ret)0` fallback if dlsym returns NULL (safe when function is resolved but not called)
- `_Unwind_*` stubs try `RTLD_DEFAULT` fallback if `RTLD_NEXT` returns NULL (libgcc_s not yet loaded)

## 🎯 Next Agent — Your Priority Task

### Debug libc++.so .init_array crash

All 8 `_Unwind_*` (`@@LIBC_R`) and ~30 other LIBC-versioned symbols are now stubbed. The symbol resolution phase of `dlopen("libc++.so")` completes — no more "undefined symbol" errors.

**New blocker:** `libc++.so` crashes during its 3 static C++ constructors (`.init_array` entries at offsets 0x7fbbc, 0x7feb0, 0xc87d4). The segfault is at address 0x7fbbc (the first init function address itself), suggesting an unrelocated pointer or missing C++ runtime symbol.

**To investigate:**

1. **Install gdb-multiarch** for QEMU GDB debugging:
   ```
   sudo apt install gdb-multiarch
   qemu-aarch64 -g 1234 -L ~/.cache/open-sober/android-env \
     -E LD_LIBRARY_PATH="/system/lib64:/lib" \
     -E LD_PRELOAD="libbionic_shim.so" \
     -E ROBLOX_LIB="libc++.so" \
     ~/.cache/open-sober/android-env/jni_shim &
   gdb-multiarch -ex "target remote :1234" ~/.cache/open-sober/android-env/jni_shim
   ```

2. **Check if `__cxa_*` functions need more complete stubs.** libc++.so's constructors may need working `__cxa_atexit`, `__cxa_finalize`, or `__cxa_guard_*` that go beyond simple dlsym forwarding. The trampoline table entries for these may be incomplete or wrong.

3. **Check if the `--whole-archive` bridge libc.so is causing symbol conflicts** with the real glibc `libc.so.6`. Consider rebuilding the bridge without `--whole-archive`:
   ```
   aarch64-linux-gnu-gcc -shared -fPIC -o libc.so bridge_libc.c \
     -Wl,--version-script,bridge_version.ver \
     -Wl,-soname,libc.so \
     -L. -lc_glibc -lm -ldl
   ```

4. **Check if `libgcc_s.so.1` needs to be in the sysroot** for the `_Unwind_*` stubs' `dlsym(RTLD_NEXT)` to resolve:
   ```
   cp /usr/aarch64-linux-gnu/lib/libgcc_s.so.1 ~/.cache/open-sober/android-env/system/lib64/
   ```

5. **Try simpler _Unwind stubs** that are true no-ops (no dlsym call at all) to isolate whether the crash is from the _Unwind stubs or from other init code.

### If init_array crash is resolved:
- Test `dlopen("libroblox.so")` directly (the main Roblox game library)
- Then JNI function table (~233 functions to stub)
- EGL/GLES→Vulkan translation (see GRAPHICS_RECOMMENDATION.md)
- Window creation + input handling

### Do NOT create worktrees or feature branches. Work on `dev` directly.
- EGL/GLES→Vulkan translation (see GRAPHICS_RECOMMENDATION.md)
- Window creation + input handling

### Environment:
- QEMU: `qemu-aarch64` at `/usr/bin/qemu-aarch64`
- Cross-compiler: `aarch64-linux-gnu-gcc`
- GSI libs: `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- Roblox APK: `~/Documents/Projects/open-sober/roblox-android.apk`
- Android NDK: `/tmp/ndk_extract/`
- GSI image mount: `/tmp/gsi_mount/`
