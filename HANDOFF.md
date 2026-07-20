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

## Current Status (July 19, after session 6)

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

### 🟢 CF_NO_GOTO_TB patch works with direct call

Session 6 discovered: The CF_NO_GOTO_TB + tb_set_jmp_target no-op patches in the custom QEMU (`~/.cache/open-sober/qemu-patched`) are sufficient when JNI_OnLoad is called **directly** (no code copy). The code copy workaround introduced adrp fix bugs that caused SIGILL.

Key findings:
- **Direct call to JNI_OnLoad → SIGILL fixed** (no host crash, no guest crash)
- **Code copy approach → still crashes with SIGILL** (adrp/branch fix has bugs)
- **100% CPU for 5+ min** with no syscalls — libroblox enters a spin loop after init returns
- The init deadlock (condvar wait) is bypassed via BSS pre-init + condvar shim
- But the code after init returns has an infinite loop

### 🟡 Current Blocker — Libroblox init spin loop

**Symptom:** After JNI_OnLoad enters and the guard check returns immediately, libroblox enters a tight computation loop at 100% CPU, makes **zero syscalls**, and never reaches any JNI calls even after 5+ minutes.

**Evidence:**
- `strace` shows no futex/write/read after `entering JNI_OnLoad...`
- No `FindClass`, `GetMethodID`, or `RegisterNatives` log lines appear
- QEMU process shows 100% CPU on one core
- WITH QEMU_RESERVED_VA: same spin loop behavior
- No SIGSEGV handler fires (no dangling pointers or bad accesses)

**Hypothesis:** JNI_OnLoad calls code that spins on a BSS variable that was supposed to be set during the one-time init. Since we bypassed the init (guard=1), the variable was never written. Candidates:
- A "initialization complete" flag checked in a loop
- A function pointer table that was supposed to be populated by init
- An internal Roblox sync primitive (not pthread — no syscalls)

**Best fix to try next:** Don't set guard=1. Instead, let the init function run, but let the **condvar shim** handle the deadlock. The init function calls `bl 26c0c7c` which does mutex_lock + cond_wait. With condvar shim (`mov w0,#0; ret` at tramp[39,96]), cond_wait returns immediately and the init function completes normally — writing the guard byte and storing the JavaVM* itself.

### What session 5 discovered about JNI_OnLoad

- The "0% CPU hang" was a **one-time init guard** using `pthread_mutex_lock` + `pthread_cond_wait` at function `0x26c0c7c`. The main thread waits on a condvar that no other thread ever signals (no Java runtime).
- Function at `0x5e17fb8` checks BSS at `0x6a26e48` for an object pointer. When set, it calls through what it expects is a vtable (but is actually vm_table from our JavaVM stub).
- Multiple guard bytes at VA `0x6a26e30-0x6a26e48` control different initialization paths.

### What was fixed (software side)

1. **Enhanced JNI logging stubs** — FindClass, GetMethodID, RegisterNatives, ThrowNew all log with call counts
2. **Mutex sanitization** — wrappers for Bionic→glibc pthread_mutex_t ABI mismatch (kind field at offset 16, __owner at offset 8 leaking into __count)
3. **Condvar shim** — raw ARM `mov w0,#0; ret` replaces trampoline entries 39 (pthread_cond_wait) and 96 (pthread_cond_timedwait), preventing all condvar deadlocks
4. **SIGSEGV handler fixed** — bad-address faults chain to old handler instead of setting x0=&g_env (which made crashes worse)
5. **BSS pre-init** — guard bytes set to 1 at `0x6a26e30-0x6a26e48`, JavaVM* stored at `0x6a26e48`
6. **Init call patches** — code copy replaces `bl 26c0c7c` with `mov w0, #1` at offset +0x1a98

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