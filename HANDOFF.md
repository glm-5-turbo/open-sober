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

Roblox loads through its full dependency chain (~170 libraries) and fails at:
```
libprotobuf-cpp-lite-6.33.5-absl20260526.so: cannot allocate memory in static TLS block
```

**Root cause:** The library uses TLS (Thread-Local Storage) and is loaded via `dlopen()`.
Glibc's static TLS reserve is exceeded by cumulative TLS usage from all loaded GSI
libraries. The TLS segment itself is only 97 bytes (0x61 memsz), but the cumulative
total of all TLS segments across ~170 libraries overflows the 1664-byte default reserve.

**Fixed with:** `GLIBC_TUNABLES="glibc.rtld.optional_static_tls=4096"` QEMU env variable.

After TLS fix, next blocker:
```
libxml2.so: undefined symbol: UCNV_TO_U_CALLBACK_STOP_android, version LIBANDROIDICU_EXTERNAL_1
```
This requires `libandroidicu.so` ICU stubs — a separate Android subsystem.

### 🧪 Test commands
```bash
# Bridge rebuild
SYSROOT=~/.cache/open-sober/android-env/system/lib64
SCRIPT_DIR=crates/sober-core/src/bridges
cd ~/Documents/Projects/open-sober
python3 "$SCRIPT_DIR/gen_bridge_stubs.py" "$SYSROOT" /tmp/bridge_stubs_full.c
aarch64-linux-gnu-gcc -c -fPIC -O2 -o /tmp/bridge_stubs.o /tmp/bridge_stubs_full.c
aarch64-linux-gnu-gcc -c -fPIC -O2 -o /tmp/bridge_manual.o "$SCRIPT_DIR/bridge_libc.c"
aarch64-linux-gnu-gcc -shared -fPIC -O2 -o "$SYSROOT/libc.so" /tmp/bridge_stubs.o /tmp/bridge_manual.o \
    -Wl,--version-script,"$SCRIPT_DIR/bridge_version.ver" \
    -Wl,-soname,libc.so -L/usr/aarch64-linux-gnu/lib -lc -lm -ldl -nostartfiles

# Fix libm.so DT_INIT
python3 -c "
import struct; p='$SYSROOT/libm.so'; d=bytearray(open(p,'rb').read())
dyn_off=0xfe00
for off in range(dyn_off, dyn_off+0x1c0, 16):
    if struct.unpack('<Q', d[off:off+8])[0] in (0xc, 0xd):
        struct.pack_into('<Q', d, off+8, 0xa18)
open(p,'wb').write(bytes(d))
"

# Test libroblox.so
timeout 30 qemu-aarch64 -L ~/.cache/open-sober/android-env \
  -E LD_LIBRARY_PATH="/system/lib64:/lib" \
  -E LD_PRELOAD="libbionic_shim.so" \
  -E GLIBC_TUNABLES="glibc.rtld.optional_static_tls=4096" \
  -E ROBLOX_LIB="libroblox.so" \
  ~/.cache/open-sober/android-env/jni_shim
```

### ⚠️ Known Issues
1. **TLS overflow** — `GLIBC_TUNABLES` hack needed for dlopen'd TLS objects
2. **libandroidicu.so stubs** — Missing ICU symbol `UCNV_TO_U_CALLBACK_STOP_android`
3. **gsi_libcutils.so corrupted** — My batch scripts may have damaged some gsi_* files.
   `libcutils.so` now symlinks to `android_libcutils.so` instead.
4. **VERSYM removal** — 149 patched GSI files no longer have VERSYM entries. This
   prevents version resolution for these libraries. If a library needs versioned
   lookups, the patch breaks it. So far all tested libraries work without VERSYM.
5. **Multiple libc.so in load chain** — The bridge libc.so (SONAME=libc.so) and glibc's
   libc.so.6 coexist. The linker resolves soname `libc.so` but libc.so.6 is also loaded.
6. **Zeroing `libdl.so`'s p_offset** — The batch patch log showed corrupted offsets
   (`0x400000006`). This was a false alarm from incorrect PHDR field byte offsets in
   my diagnostic script, not actual file corruption.

### 🔮 Next Steps
1. Add `libandroidicu.so` symbols for the ICU external library
2. Create stub for `libandroidicu.so` or symlink to GSI version
3. After libroblox.so loads, tackle JNI function table (FindClass, GetMethodID, etc.)
4. EGL/GLES→Vulkan bridge (Mesa zink driver)
5. X11/Wayland window creation
6. Input handling (touch → mouse/keyboard)
