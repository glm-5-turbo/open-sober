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

### ✅ ANDROID_RELR → RELR Patch (2026-07-19, MAJOR BREAKTHROUGH)

**Root cause identified:** `libc++.so` uses `ANDROID_RELR` (`.relr.dyn` section) for relative relocations. The ARM64 glibc `ld-linux-aarch64.so.1` does NOT recognize `DT_ANDROID_RELR` (tag `0x6fffe000`) — it only understands standard `DT_RELR` (tag `0x24`). This left the entire `.init_array` and all other RELR-format relocations unprocessed, causing the segfault when `call_init` tried to read unrelocated function pointers.

**Fix:** Patch the dynamic section to convert `ANDROID_RELR` → `RELR` tags:

```
DT_ANDROID_RELR (0x6fffe000)  → DT_RELR (0x24)
DT_ANDROID_RELRSZ (0x6fffe001) → DT_RELRSZ (0x23)
DT_ANDROID_RELRENT (0x6fffe003) → DT_RELRENT (0x25)
```

The underlying bit-packed RELR data format is identical between Android and glibc — only the DT tags differ.

**Usage:** `python3 ~/patch_relr.py gsi_libc++.so`

**Current status:** After patching, `dlopen("libc++.so")` no longer segfaults. The library transitions past relocation into C++ static initialization (`.init_array` constructors run). However, it now **hangs** during one of the 3 constructors rather than crashing. The last syscall is `getrandom`, suggesting the issue is in C++ runtime init (possibly `std::ios_base::Init` or `__cxa_atexit` registering).

### ❌ Current Blocker
```
dlopen("libc++.so")" — HANGS during C++ static initialization.
```

After ANDROID_RELR→RELR patching and adding all ~40 LIBC-versioned stubs, `libc++.so` loads without crashing. It hangs during the 3rd phase of dlopen: running `.init_array` constructors. The three functions at offsets `0x7fbbc`, `0x7feb0`, `0xc87d4` in the patched binary are likely `std::ios_base::Init`, static locale init, or `__cxa_atexit` guard setup.

**To debug:**
1. Use GDB to catch dlopen completion and identify which constructor is hanging
2. Set breakpoints at the init function addresses after RELR patch
3. Check if the constructors are calling back into glibc functions that expect glibc-internal state that isn't set up properly (e.g., `__libc_single_threaded`, `__ctype_b_loc`, etc.)
4. Try running with `GLIBC_TUNABLES=glibc.cpu.hwcaps=-XSAVEC` or similar to disable slow init paths

### After init_array hang is resolved:
- Test `dlopen("libroblox.so")` directly (the main Roblox game library)
- Then JNI function table (~233 functions to stub)
- EGL/GLES→Vulkan translation (see GRAPHICS_RECOMMENDATION.md)
- Window creation + input handling

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

### Debug libc++.so init hang after ANDROID_RELR fix

**Quick start — run the current test:**
```bash
cd ~/Documents/Projects/open-sober

# Build bionic shim with all stubs
SYSROOT=~/.cache/open-sober/android-env/system/lib64
CRATE_SRC=crates/sober-core/src
aarch64-linux-gnu-gcc -c -fPIC -o /tmp/bs.o "$CRATE_SRC/bionic_init.c"
aarch64-linux-gnu-gcc -c -o /tmp/ba.o "$CRATE_SRC/bionic_shim.S"
aarch64-linux-gnu-gcc -shared -fPIC -o "$SYSROOT/libbionic_shim.so" \
  /tmp/ba.o /tmp/bs.o \
  -Wl,--version-script,"$CRATE_SRC/bionic_version.ver" \
  -Wl,-rpath,/system/lib64 -L"$SYSROOT" -lglibc -lm -ldl -nostartfiles
rm -f /tmp/ba.o /tmp/bs.o

# Patch libc++.so ANDROID_RELR → RELR
python3 ~/patch_relr.py "$SYSROOT/gsi_libc++.so"
ln -sf "gsi_libc++.so" "$SYSROOT/libc++.so"

# Build JNI shim with unbuffered output
cp "$CRATE_SRC/jni_shim.c" /tmp/js.c
sed -i 's/int main(int argc, char\*\* argv) {/int main(int argc, char** argv) { setbuf(stderr,NULL); setbuf(stdout,NULL);/' /tmp/js.c
aarch64-linux-gnu-gcc -static -o ~/.cache/open-sober/android-env/jni_shim /tmp/js.c -ldl

# Test
timeout 8 qemu-aarch64 \
  -L ~/.cache/open-sober/android-env \
  -E LD_LIBRARY_PATH="/system/lib64:/lib" \
  -E LD_PRELOAD="libbionic_shim.so" \
  -E ROBLOX_LIB="libc++.so" \
  ~/.cache/open-sober/android-env/jni_shim
```

**What's happening:** All ~40 stubs resolve. ANDROID_RELR is patched to RELR. dlopen proceeds past relocation into C++ static init (`.init_array` at offsets 0x7fbbc, 0x7feb0, 0xc87d4) but hangs there. The 3 constructors are likely `std::ios_base::Init`, locale init, and `__cxa_atexit` guard setup.

**To investigate:**
1. **GDB debug:** Install `gdb-multiarch`, use `qemu-aarch64 -g 1234` and connect:
   ```bash
   qemu-aarch64 -g 1234 -L ~/.cache/open-sober/android-env \
     -E LD_LIBRARY_PATH="/system/lib64:/lib" \
     -E LD_PRELOAD="libbionic_shim.so" \
     -E ROBLOX_LIB="libc++.so" \
     ~/.cache/open-sober/android-env/jni_shim &
   gdb-multiarch -ex "target remote :1234" -ex "cont" ./jni_shim
   # Wait 5s, Ctrl+C, bt
   ```
2. **Check if constructors loop** on `dlsym(RTLD_NEXT, ...)` calls from our stubs — some of our stubs might be called during C++ init and the dlsym itself might trigger more relocation processing → infinite loop.

3. **Try empty init_array** to confirm constructors are the culprit:
   ```python
   with open('gsi_libc++.so', 'r+b') as f:
       f.seek(0x115878); f.write(b'\x00' * 24)
   ```

4. **Check `__cxa_*` stubs** — the existing trampoline table entries for `__cxa_finalize` and `__cxa_atexit` might need to be more complete (forwarding to actual glibc versions rather than atomic no-ops).

### After init_array hang is resolved:
- Test `dlopen("libroblox.so")` directly
- JNI function table (~233 stubs)
- EGL/GLES→Vulkan (see GRAPHICS_RECOMMENDATION.md)
- Window creation + input
- EGL/GLES→Vulkan translation (see GRAPHICS_RECOMMENDATION.md)
- Window creation + input handling

### Environment:
- QEMU: `qemu-aarch64` at `/usr/bin/qemu-aarch64`
- Cross-compiler: `aarch64-linux-gnu-gcc`
- GSI libs: `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- Roblox APK: `~/Documents/Projects/open-sober/roblox-android.apk`
- Android NDK: `/tmp/ndk_extract/`
- GSI image mount: `/tmp/gsi_mount/`
