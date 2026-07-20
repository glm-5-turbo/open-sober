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

### 🟡 Current Blocker — JNI_OnLoad hangs at 100% CPU

After successfully entering JNI_OnLoad, the QEMU process consumes 100% CPU and does not return. No JNI stub log messages appear (FindClass, GetMethodID etc.).

**Diagnosis so far (Session 8):**
1. **Disassembled JNI_OnLoad** — found the one-time init function at `offset 0x1f65a60` which checks a guard byte at `base + 0x6a26e40`.
2. **Analyzed the init function at 0x26c0c7c** — calls `pthread_mutex_lock`, checks flags, and if in "waiting" state, calls `pthread_cond_wait@plt` in a **spurious wakeup loop** that waits until some other thread changes the state. This loops forever because no other thread exists in QEMU user-mode.
3. **Patched PLT GOT entries** — patched `pthread_cond_wait` and `pthread_cond_timedwait` GOT entries at `base + 0x6473628` and `base + 0x6473630` to point to our condvar shim. Verified the patch works.
4. **Pre-set init guard** — set the guard byte at `base + 0x6a26e40` to 1 _before_ entering JNI_OnLoad, so the init function returns immediately. Also pre-init the JavaVM* at `base + 0x6a26e48`.
5. **CONFIRMED guard fix works** — log shows `[jni_shim] pre-init guard=1 at 0x...`
6. **Still hangs** — the hang is NOT in the one-time init (we skip it with guard=1). It's somewhere ELSE inside JNI_OnLoad (possibly a tight CPU loop in another init function called afterward).

**Likely candidate:** The function at `0x1f65a60` is just the FIRST of several functions called by JNI_OnLoad. After it returns (which we force), JNI_OnLoad continues to call:
   - `bl 1f6594c` (at offset 0x1f64eb0)
   - `bl 273de0c` (nativeSetAssetPath, at 0x1f64eb8)
   - `bl 1f65a54` (at 0x1f64ec0)
   - Various JNI function table calls (`blr x8`)
   - `bl 1f667dc` (at 0x1f64f58)
   - `bl 1f66a24` (at 0x1f65000)

**Next recommended step:** Use QEMU's `-d in_asm -D log` to record which guest code addresses are being executed during the hang, then cross-reference with libroblox.so's symbol table to identify which function is looping.

**What was fixed in session 8**

1. **`disable_mcount_profiling()` extracted as early function** — patches `_dl_mcount` to `ret` + zeros `dl_profile` before anything else in `main()`
2. **Precise 64KB mprotect + 4-byte ret** — the 64KB range on ld-linux text forces TCG re-translation; the 4-byte write patches only the entry point, not adjacent PLT fixup code
3. **Verified working** — confirmed via strace: `[jni_shim] noped _dl_mcount at 0x...`, `_dl_mcount entry now: 0xd65f03c0`, full log shows we reach JNI_OnLoad successfully
4. **PLT GOT condvar shim patching** — patches `pthread_cond_wait` and `pthread_cond_timedwait` GOT entries in libroblox.so's PLT to our immediate-return shim
5. **One-time init guard pre-initialization** — sets guard byte at `base+0x6a26e40` to 1 before JNI_OnLoad, skipping the condvar-based initialization that hangs under QEMU

### Key Source Files

- `crates/sober-core/src/jni_shim.c` — Main JNI shim (~620 lines)
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

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom from `/tmp/qemu-10.2.1/` — patched at `~/.cache/open-sober/qemu-patched`
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- **Bionic shim:** `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** `~/.cache/open-sober/android-env/jni_shim`