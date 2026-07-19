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

## Current Status (July 19, after sessions 5+5b)

### ✅ Complete (all sessions)

1. **Custom QEMU** built from `/tmp/qemu-10.2.1/` — patched copy at `~/.cache/open-sober/qemu-patched`.
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — trampoline-based mutex interceptors.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled.
4. **QEMU bridge wiring** (`qemu.rs`) — version bridges for libc/libm/libdl.
5. **Canary GOT patching** — stack_chk_guard write via mprotect.
6. **SIGSEGV handler** — RELRO faults, self-write JIT bugs, NULL deref handling.
7. **Pre-mprotect RELRO** — ~464 pages made RW before JNI_OnLoad.
8. **Pre-resolved trampoline table** — 358 functions via RTLD_DEFAULT.
9. **Canary check patched out** in code copy.
10. **JNI_OnLoad code copy** — 128KB copy with adrp fix + BL/B.cond/CBZ/TBZ offset fix.
11. **Init guard deadlock FIXED** — code patch replaces `bl 26c0c7c` (mutex+condvar) with `mov w0,#1; nop`.
12. **Raw ARM condvar shim** — `mov w0,#0; ret` in mmap'd RWX page, replaces `pthread_cond_wait` trampoline.

### 🟡 Current Blocker — QEMU JIT page-boundary SMC crash on x86-64 host

**QEMU 10.2.1 TCG JIT** writes to its code buffer during translation (before jump patching). When the write lands at the end of a host page (`0x...ffb8` or `0x...fff8`), x86-64 Self-Modifying Code detection triggers a host SIGSEGV that QEMU cannot recover from. This corrupts the JIT cache and causes guest SIGILL.

**Patches applied but insufficient:**
- `CF_NO_GOTO_TB` — prevents goto_tb chaining writes
- `tb_set_jmp_target` no-op — prevents indirect jump patching writes
- The write comes from TCG code **generation** (literal pool fixup, relocations), not from jump patching

**Findings:**
- `-one-insn-per-tb`, `-d nochain`, `-tb-size`, `-cpu max` all still crash
- split-wx IS configured (`tcg_splitwx_diff` is set) but the RX view still gets corrupted
- The crash is in QEMU itself (also affects Debian's system `qemu-aarch64`)
- 100% failure rate in last session's runs (6/6), earlier it was ~30%
- The memory layout changes from the expanded trampoline table (bigger BSS) likely made it deterministic

**The user says:** They will handle the QEMU JIT bug themselves. Don't try to patch QEMU further.

### What session 5 discovered about JNI_OnLoad

- The "0% CPU hang" was a **one-time init guard** using `pthread_mutex_lock` + `pthread_cond_wait` at function `0x26c0c7c`. The main thread waits on a condvar that no other thread ever signals (no Java runtime).
- Function at `0x5e17fb8` checks BSS at `0x6a26e48` for an object pointer. When set, it calls through what it expects is a vtable (but is actually vm_table from our JavaVM stub).
- Multiple guard bytes at VA `0x6a26e30-0x6a26e48` control different initialization paths.

### What was fixed (software side)

1. **Code copy patch** in `jni_shim.c`: `bl 26c0c7c` → `mov w0, #1` at copy offset +0x1a98
2. **BSS pre-init**: guard bytes set to 1 at `0x6a26e30-0x6a26e48`
3. **Condvar shim**: raw ARM `mov w0,#0; ret` replaces trampoline entries 39 (pthread_cond_wait) and 96 (pthread_cond_timedwait), preventing all condvar deadlocks
4. **Null-deref safety** in SIGSEGV handler (partial — under QEMU user-mode `uc_mcontext` maps to host x86-64 registers, not guest ARM)

### 🎯 Next Steps (after QEMU JIT is fixed)

1. **Let JNI_OnLoad run to completion** — with init deadlock bypassed, JNI_OnLoad makes real progress. 30+ second runs expected under QEMU JIT slowdown.
2. **Implement RegisterNatives properly** — log registered methods and implement key ones.
3. **Once JNI_OnLoad returns** — Roblox main loop (render, network, scripts), then optimize the 10-15% emulation slowdown.

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

### Key Source Files

- `crates/sober-core/src/jni_shim.c` — Main JNI shim: loads libroblox.so, code copy with adrp/branch fix, SIGSEGV handler, trampoline pre-resolve, condvar shim
- `crates/sober-core/src/bionic_init.c` — Bionic shim C code: trampoline resolver (`__bf_c_resolve`)
- `crates/sober-core/src/bionic_shim.S` — Auto-generated assembly trampolines (785 entries)
- `crates/sober-core/src/qemu.rs` — QEMU launcher: builds bridges, sets up environment, spawns QEMU

### Environment

- **GPU:** NVIDIA RTX 3060 Mobile + Intel Iris Xe (Mesa drivers active)
- **OS:** Ubuntu 26.04 LTS
- **QEMU:** Custom from `/tmp/qemu-10.2.1/` source; also system `/usr/bin/qemu-aarch64` (both crash with same SMC bug)
- **Cross-compiler:** `aarch64-linux-gnu-gcc` (gcc-15)
- **GSI ARM64 libs:** At `~/.cache/open-sober/android-env/system/lib64/` (788 libs)
- **Bionic shim:** `~/.cache/open-sober/android-env/system/lib64/libbionic_shim.so`
- **JNI shim:** `~/.cache/open-sober/android-env/jni_shim`