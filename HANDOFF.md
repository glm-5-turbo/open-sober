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

## Current Status (July 19, session 3 - late)

### ✅ Done (this session)

1. **Rewrote jni_shim.c** — Removed broken adrp-based JNI_OnLoad pre-copy (the copy+fix approach had incorrect address calculations). Replaced with a simple SIGSEGV handler that:
   - Toggles page permissions (PROT_NONE → PROT_RWX) to force QEMU JIT cache invalidation
   - Handles self-referencing writes (code writing to its own page) by flushing the cache
   - Pre-mprotects all r--p RELRO pages to rw-p using /proc/self/maps
   - Detects NULL/bad-address faults (returned MAP_FAILED errors)
   - Has an infinite-loop limit of 500 faults

2. **Fixed bionic_shim's __bf_init_data** — The function wasn't a constructor so the mutex wrapper trampolines were never installed. Added `__bf_install_mutex_wrappers()` which is called explicitly from jni_shim after the SIGSEGV handler is active. It installs `sanitize_mutex` wrappers at trampoline indices 38, 124, 780.

3. **Fixed bulk mutex owner scan** — The previous session's dl_iterate_phdr scan was too aggressive and hit RELRO-protected pages. Removed the bulk scan entirely — per-mutex sanitization at lock time (via the bionic_shim trampolines) is the safe approach.

### 🔴 Current Blocker

After fixing RELRO page faults and mutex assertions, JNI_OnLoad now crashes immediately with:
```
pc=0xffffffffffffffff fault=0xffffffffffffffff
```
The program counter itself is at -1. This means a function call returned -1 (MAP_FAILED or similar error) and the code tried to call through it as a function pointer. This is likely one of:

1. A `dlsym` or `mmap` call inside a GSI library that failed, and the error return is used as a callback pointer
2. A JNI stub that returns a bad value used as a function pointer
3. An ELF constructor in a GSI library that calls an unimplemented function returning -1

The current jni_shim aborts on this with the bad-address check, but we need to understand why this call chain returns -1.

### Next Steps

1. **Trace the -1 PC crash** — Use QEMU's `-strace` or `-d cpu_reset` to find which function call leads to the `0xffffffffffffffff` PC. It's likely a GSI library init function crashing.
2. **Identify missing stub** — Find which function is returning -1 and add a proper stub (likely one of the JNI function table entries).
3. **EGL/GLES graphics stubs** — Mesa+zink approach. Not yet started.
4. **Window + input** — SDL2-based. Not yet started.

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom `/tmp/qemu-10.2.1/build/qemu-aarch64` (VDSO disabled) + system `/usr/bin/qemu-aarch64`
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs, patched for ANDROID_RELR/RELA)
- **Bionic shim:** Built at `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** Built at `~/.cache/open-sober/android-env/jni_shim`