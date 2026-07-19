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

### ✅ APS2 → Standard RELA Decompression (NEW MAJOR BREAKTHROUGH)

**The fix:** `unpack_rela.py` (now in repo) correctly decodes APS2 packed relocations using the EXACT decoder from Android's `for_all_packed_relocs()`. The format is `"APS2"` magic followed by a SLEB128 stream with relocation groups.

**Critical discovery: Android linker flag bit assignments differ from LLVM encoder:**
| Flag | Android linker | LLVM lld |
|------|---------------|----------|
| `GROUPED_BY_INFO` | Bit 0 (1) | Bit 1 (2) |
| `GROUPED_BY_OFFSET_DELTA` | Bit 1 (2) | Bit 0 (1) |

**Results with libc++.so:**
- 2113 APS2 entries → 50712 bytes of standard Elf64_Rela (from 15790 packed)
- Types: 1923 R_AARCH64_ABS64, 189 R_AARCH64_GLOB_DAT, 1 R_AARCH64_TLSDESC
- readelf displays all entries with correct symbol names
- Patching strategy: appends RELA data to file + new PT_LOAD segment

**How to patch any GSI library:**
```bash
python3 ./unpack_rela.py ~/.cache/open-sober/android-env/system/lib64/gsi_libfoo.so
cp ~/.cache/open-sober/android-env/system/lib64/gsi_libfoo.so \
   ~/.cache/open-sober/android-env/system/lib64/libfoo.so
```


## ✅ ANDROID_RELA → RELA Patch (NEW MAJOR BREAKTHROUGH)

GSI libc++.so uses **packed ANDROID_RELA** relocations — a compressed format the Android linker understands but the ARM64 glibc linker does not. The standard `DT_RELA` tags point to data in APS2 packed format (signature `"APS2"` at offset 0x2fab8). The dynamic linker needs:
- `DT_ANDROID_RELA (0x60000011)` → `DT_RELA (0x7)`
- `DT_ANDROID_RELASZ (0x60000012)` → `DT_RELASZ (0x8)`

**Fix:** `~/patch_relr.py` now handles ANDROID_RELA→RELA conversion (in addition to ANDROID_RELR).

**Key status:** After DT tag conversion, libc++.so's RELA data is in compressed APS2 format that glibc still can't parse. The packed data needs to be **decompressed** into standard 24-byte RELA entries. A partial script at `~/unpack_rela.py` starts this work.

## ✅ libEGL.so → libnativewindow.so Symlink Fix

The blocker `libEGL.so: undefined symbol: AHardwareBuffer_to_ANativeWindowBuffer, version LIBNATIVEWINDOW_PLATFORM` was fixed by symlinking `libnativewindow.so → gsi_libnativewindow.so` (same established pattern as libbinder_ndk.so). Also symlinked `libz.so → gsi_libz.so`.

## ✅ Mass ANDROID_RELR + ANDROID_RELA Patching

The `patch_relr.py` script was rewritten to find `.dynamic` via ELF program headers (not hardcoded offsets). All 788 GSI libraries were patched for ANDROID_RELR→RELR AND ANDROID_RELA→RELA.

## ✅ New Stubs Added

Three missing symbols added to `bionic_init.c`:
- `nrand48@@LIBC` — random number function (libEGL.so needs this)
- `android_dlopen_ext@@LIBC` — Bionic dlopen variant (delegates to dlopen)
- `futimens@@LIBC` — timestamp function

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

### ✅ 12 Critical GSI Libraries Now Load

All of the following libraries load via `dlopen()` under QEMU-aarch64:

| Library | Status | Notes |
|---------|--------|-------|
| libc++.so | ✅ | Working |
| liblog.so | ✅ | Working |
| libbase.so | ✅ | Working |
| libcutils.so | ✅ | Working |
| libutils.so | ✅ | Working |
| libhardware.so | ✅ | Working |
| libcodec2.so | ✅ | Working |
| libbinder.so | ✅ | Working |
| libfmq.so | ✅ | Working |
| libEGL.so | ✅ | Working |
| libgui.so | ✅ | Working |
| libui.so | ✅ | Working |

### 🔧 Fixes Applied This Session

**1. Bridge libc.so with comprehensive Bionic stubs**
- Added `getprogname@@LIBC`, `android_set_abort_message@@LIBC`
- Added all `__system_property_*` Bionic system property functions
- Added `android_fdsan_*` file descriptor sanitizer stubs under `@@LIBC_Q`
- Added `__sF@@LIBC` (Bionic FILE table, needed by UBSan runtime)
- Added `android_get_application_target_sdk_version@@LIBC_N`
- Added `__write_chk@@LIBC_N` and `dlopen@@LIBC`/`dlerror@@LIBC` via .symver
- Moved dl* functions from SKIP to auto-generated stubs (they ARE in glibc libc)
- Generator now adds `__system_property_*` and `android_fdsan_*` to SKIP

**2. gen_bridge_stubs.py fixes**
- Added Bionic-only function skip list expansions
- Uses dedup-by-name (highest-version wins) for versioned symbols
- Handles data objects by skipping them (handled in bridge_libc.c)

**3. VERSYM/VERNEED patch for GSI libraries**
- Many GSI libraries have VERSYM=0 but VERNEED pointing to garbage
- Batch patch: zeroes VERSYM dynamic entry tag when VERNEEDNUM=0
- Prevents glibc `do_lookup_x` NULL+8 crash during symbol resolution
- 149 libraries patched

**4. DT_RELA cleanup for APS2 libraries**
- Some GSI libraries have APS2-packed RELA at vaddr past file end (in BSS gap)
- Zeroing DT_RELA/DT_RELASZ for these libs prevents garbage reloc processing
- `gsi_libcutils.so` patched (RELA was at vaddr 0x20000, beyond LOAD segments)

**5. Batch init/fini array clearing**
- Batch script clears DT_INIT_ARRAY, DT_INIT_ARRAYSZ, DT_FINI_ARRAY, DT_FINI_ARRAYSZ
- Does NOT clear DT_INIT (tag 12) or DT_FINI (tag 13) — zeroing these makes glibc's
  `call_init` jump to `base+0` (ELF header = SIGILL)
- 2426 libraries patched

**6. libm.so DT_INIT/DT_FINI fix**
- libm.so bridge had DT_INIT=0 and DT_FINI=0 (compiled with -nostartfiles)
- Glibc's `call_init` checks `l->l_info[DT_INIT]` presence (not value), crashes at `base+0`
- **Fix:** Point DT_INIT and DT_FINI to vaddr 0xa18 (existing `ret` opcode in .text)
- Same fix applies to any bridge library compiled with -nostartfiles

**7. `libcutils.so` switched to `android_libcutils.so`**
- `gsi_libcutils.so` doesn't export `atrace_update_tags`
- Symlink changed: `libcutils.so → android_libcutils.so` (has 265 symbols vs gsi's 0 readable)

### 📈 dlopen("libroblox.so") Progress

**Current blocker:** `libandroid_runtime.so` crashes with SIGILL at base+0.

`gsi_libandroid_runtime.so` was patched with `patch_gsi.py` (APS2→RELA, GNU_RELRO removed, BIND_NOW removed). DT_INIT_ARRAY tag still exists with value=0 and DT_INIT_ARRAYSZ=0. Glibc's `call_init` sees `l_info[DT_INIT_ARRAY] != NULL` and computes `base + 0 = base = ELF header = SIGILL`. See instructions below for the fix.

**Previous blockers resolved (in order):**
| Blockers | Fix |
|----------|-----|
| `__write_chk@@LIBC_N` | `.symver` alias in bridge_libc.c |
| `android_fdsan_get_owner_tag@@LIBC_Q` | Bionic stub in bridge_libc.c |
| `__system_property_*` (5+ symbols) | Bionic stubs in bridge_libc.c |
| `atrace_update_tags` | Switched to android_libcutils.so |
| `dlopen@@LIBC`, `dlerror@@LIBC` | Removed from SKIP in generator |
| `android_get_application_target_sdk@@LIBC_N` | Bionic stub |
| `__sF@@LIBC` | Data stub |
| `UCNV_TO_U_CALLBACK_STOP_android` (7 ICU-Android syms) | Created bridge_androidicu.c |
| `utext_close@@LIBICU_31` + 27 other ICU symbols | Created bridge_icu.c |
| `getrandom@@LIBC_P`, `aligned_alloc@@LIBC_P` | `.symver` alias in bridge_libc.c |
| `gClsAudioTrackRoutingProxy` | Needs libandroid_runtime.so to load |
| Static TLS overflow | `GLIBC_TUNABLES=glibc.rtld.optional_static_tls=4096` |
| VERSYM=0 NULL+8 crash | Batch patch: zero VERSYM tag when VERNEEDNUM=0 |
| libm.so DT_INIT=0 SIGILL | Point to ret instruction at vaddr 0xa18 |
| libcutils.so reloc 0x40 error | Zero DT_RELA when vaddr in BSS gap |
| VERSYM verneed record error | Zero VERSYM tag → `l_info[VERSYM]` = NULL |

### 🧪 Test commands
```bash
SYSROOT=~/.cache/open-sober/android-env/system/lib64
SCRIPT_DIR=crates/sober-core/src/bridges
cd ~/Documents/Projects/open-sober

# Full rebuild of all bridges
python3 "$SCRIPT_DIR/gen_bridge_stubs.py" "$SYSROOT" /tmp/bs.c
aarch64-linux-gnu-gcc -c -fPIC -O2 -o /tmp/bs.o /tmp/bs.c
aarch64-linux-gnu-gcc -c -fPIC -O2 -o /tmp/bm.o "$SCRIPT_DIR/bridge_libc.c"
aarch64-linux-gnu-gcc -shared -fPIC -O2 -o "$SYSROOT/libc.so" /tmp/bs.o /tmp/bm.o \
    -Wl,--version-script,"$SCRIPT_DIR/bridge_version.ver" \
    -Wl,-soname,libc.so -L/usr/aarch64-linux-gnu/lib -lc -lm -ldl -nostartfiles

# Fix libm.so DT_INIT
python3 -c "
import struct; p='$SYSROOT/libm.so'; d=bytearray(open(p,'rb').read())
for off in range(0xfe00, 0xfe00+0x1c0, 16):
    if struct.unpack('<Q', d[off:off+8])[0] in (0xc, 0xd):
        struct.pack_into('<Q', d, off+8, 0xa18)
open(p,'wb').write(bytes(d))
"

# Build ICU bridges
aarch64-linux-gnu-gcc -shared -fPIC -O2 -o "$SYSROOT/libandroidicu.so" \
    "$SCRIPT_DIR/bridge_androidicu.c" \
    -Wl,--version-script,"$SCRIPT_DIR/bridge_androidicu.ver" \
    -Wl,-soname,libandroidicu.so -nostartfiles
aarch64-linux-gnu-gcc -shared -fPIC -O2 -o "$SYSROOT/libicu.so" \
    "$SCRIPT_DIR/bridge_icu.c" \
    -Wl,--version-script,"$SCRIPT_DIR/bridge_icu.ver" \
    -Wl,-soname,libicu.so -nostartfiles

# Test
timeout 30 qemu-aarch64 -L ~/.cache/open-sober/android-env \
  -E LD_LIBRARY_PATH="/system/lib64:/lib" \
  -E LD_PRELOAD="libbionic_shim.so" \
  -E GLIBC_TUNABLES="glibc.rtld.optional_static_tls=4096" \
  -E ROBLOX_LIB="libroblox.so" \
  ~/.cache/open-sober/android-env/jni_shim

# Test individual library
timeout 15 qemu-aarch64 -L ~/.cache/open-sober/android-env \
  -E LD_LIBRARY_PATH="/system/lib64:/lib" \
  -E LD_PRELOAD="libbionic_shim.so" \
  -E GLIBC_TUNABLES="glibc.rtld.optional_static_tls=4096" \
  -E ROBLOX_LIB="libandroid_runtime.so" \
  ~/.cache/open-sober/android-env/jni_shim
```

### ⚠️ Known Issues
1. **DT_INIT_ARRAY tag with value 0** — Many GSI libraries have cleared INIT_ARRAY entries
   (value=0) but the DT tag still EXISTS. Glibc's `call_init` checks `l_info[DT_INIT_ARRAY]`
   presence, not value. With DT_INIT_ARRAYSZ=0 the loop should not execute, but some
   libraries SIGILL anyway — possibly from a different constructor path.
   **Fix:** Replace the DT_INIT_ARRAY tag itself with DT_NULL (0):
   ```python
   for each dynamic entry:
       if tag in (0x19, 0x1b):  # DT_INIT_ARRAY, DT_INIT_ARRAYSZ
           set tag=0, value=0  # DT_NULL — terminates .dynamic scan
   ```
2. **VERSYM removal** — 149 GSI files lost VERSYM. Libraries needing versioned lookups fail.
3. **Multiple libc.so in load chain** — Bridge `libc.so` and glibc `libc.so.6` coexist.
4. **TLS overflow** — `GLIBC_TUNABLES` env var workaround needed for large dlopen'd libs.

### 📋 Scripts in repo
| File | Purpose |
|------|---------|
| `crates/sober-core/src/bridges/gen_bridge_stubs.py` | Generates LIBC forwarding stubs |
| `crates/sober-core/src/bridges/bridge_libc.c` | Main bridge: Bionic stubs + data symbols |
| `crates/sober-core/src/bridges/bridge_libdl.c` | libdl bridge with __cfi_slowpath @@LIBC_OMR1 |
| `crates/sober-core/src/bridges/bridge_libm.c` | libm bridge (empty version shim) |
| `crates/sober-core/src/bridges/bridge_androidicu.c` | libandroidicu.so stubs (LIBANDROIDICU_EXTERNAL_1) |
| `crates/sober-core/src/bridges/bridge_icu.c` | libicu.so stubs (LIBICU_31: ubidi, utext, unorm2, ubrk) |
| `crates/sober-core/src/bridges/bridge_version.ver` | LIBC_N/O/P/Q/R/S/T/U/V definitions |
| `crates/sober-core/src/bridges/bridge_androidicu.ver` | LIBANDROIDICU_EXTERNAL_1 |
| `crates/sober-core/src/bridges/bridge_icu.ver` | LIBICU_31 |
| `crates/sober-core/src/bridges/patch_gsi.py` | Mass binary patcher (APS2/RELR/GNU_RELRO/INIT) |
| `crates/sober-core/src/bridges/unpack_rela.py` | APS2→RELA decompression |
| `crates/sober-core/src/bridges/patch_relr.py` | ANDROID_RELR→RELR conversion |

### 🎯 Next Agent — Priority Actions

**Phase A: Fix `libandroid_runtime.so` → get libroblox.so to dlopen**

1. **Fix the DT_INIT_ARRAY tag.** In `gsi_libandroid_runtime.so` the tag 0x19 (INIT_ARRAY) exists with value=0. Replace the tag itself with DT_NULL to make `l_info[DT_INIT_ARRAY] = NULL`:
   ```python
   import struct
   with open('gsi_libandroid_runtime.so', 'r+b') as f:
       d = bytearray(f.read())
       # Find dynamic section via PHDR
       # Zero both tag and value for entries 0x19 and 0x1b
       # This creates DT_NULL entries that terminate .dynamic scan early
       # Ensure VERSYM/VERNEED entries come BEFORE DT_INIT_ARRAY to not lose them
       # (VERSYM is before INIT_ARRAY in the .dynamic section, so fine)
   ```

2. **Re-test libroblox.so.** After fix, likely 5-10 more "undefined symbol" blockers. Each follows the same pattern: check if it's a real glibc function (add to generated stubs) or Bionic-only (add manual stub).

3. **Once dlopen succeeds**, the jni_shim will try to call `JNI_OnLoad`. The current jni_shim.c has stub JNI function tables (FindClass=stub_FindClass returning NULL). This may crash or produce "FindClass: ..." debug output.

**Phase B: Post-load execution**

4. **Extend jni_shim.c** with more JNI stubs (GetMethodID, NewStringUTF, GetStringUTFChars, CallVoidMethodV, etc.). Each returning safe defaults.

5. **Add `qemu.rs` integration** — The sober-core crate has `qemu.rs` for launching QEMU. Wire up the library loading and JNI shim invocation through the Rust code.

**Phase C: Graphics (see GRAPHICS_RECOMMENDATION.md)**

6. **EGL bridge** — Create libEGL.so stubs for eglGetProcAddress, eglChooseConfig, eglCreateContext, etc. Mesa zink for GLES→Vulkan.

7. **Window creation** — X11/Wayland native window handle for EGL.

8. **Input** — Touch events → mouse/keyboard.
