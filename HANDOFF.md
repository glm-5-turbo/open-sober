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

## Current Status (July 19, session 5 — One-time init deadlock FIXED, QEMU JIT crash intermittent)

### ✅ Complete (all sessions)

1. **Custom QEMU built + PATCHED** — Static 41MB binary from `/tmp/qemu-10.2.1/`. **Patched copy** at `~/.cache/open-sober/qemu-patched` with `CF_NO_GOTO_TB` fix + `tb_set_jmp_target` no-op.
2. **pthread_mutex_t ABI fix** (`bionic_init.c`) — trampoline-based mutex interceptors.
3. **Complete JNI function table** (`jni_shim.c`) — all 256 JNIEnv slots filled. Name-tracked pointers for FindClass/GetMethodID uniqueness.
4. **QEMU bridge wiring** (`qemu.rs`) — version bridges for libc/libm/libdl.
5. **Canary GOT patching** — stack_chk_guard write via mprotect.
6. **SIGSEGV handler** — RELRO faults, self-write JIT bugs, NULL deref returns valid JNIEnv*.
7. **Pre-mprotect RELRO** — ~464 pages made RW before JNI_OnLoad.
8. **Pre-resolved trampoline table** — 358 functions resolved via RTLD_DEFAULT (up from 20).
9. **Canary check patched out** in code copy.
10. **JNI_OnLoad code copy** — 128KB copy with adrp fix (all 4 immlo variants, 9010 instructions).
11. **BL/B/B.cond/CBZ/TBZ offset fix** — recalculation for calls outside copy range.
12. **GOT + init_array pre-mprotect** — explicit ranges for libroblox.so.
13. **PROT_NONE→PROT_RW double-mprotect** — forces QEMU TLB flush.
14. **Name-tracked JNI stubs** — unique pointers per class/method name.
15. **Init guard deadlock FIXED** — code copy patch replaces `bl 26c0c7c` (mutex+condvar) with `mov w0,#1; nop`, bypassing the futex deadlock.
16. **NULL deref safety** — SIGSEGV handler returns valid JNIEnv* pointer when code dereferences NULL.

### ✅ QEMU JIT BUG — Root cause patched but intermittent crashes remain

**Root cause:** QEMU's TCG `goto_tb` mechanism writes to the JIT code buffer. When the write address is at the end of a host page (`...fffb8`), x86_64 SMC detection triggers a host SIGSEGV, corrupting the JIT cache and causing guest SIGILL.

**Patches:**
- `CF_NO_GOTO_TB` (`cpu-exec-common.c`): prevents goto_tb chaining
- `tb_set_jmp_target` no-op (`cpu-exec.c`): prevents indirect jump patching writes to JIT buffer

**Intermittent:** Even with both patches, TCG code generation itself (literal pool fixup, etc.) writes to the JIT buffer during translation. These writes can also trigger SMC at page boundaries (~30% of runs).

### 🟡 Session 5 Progress — JNI_OnLoad init deadlock bypassed

**What was discovered:**
- The `0% CPU` hang from session 4 was NOT a thread spawn — it was a one-time init guard using `pthread_mutex_lock` + `pthread_cond_wait` at function `26c0c7c`. The main thread calls into this init, which waits on a condvar that no other thread ever signals.
- Function at `5e17fb8` checks BSS[0x6a26e48] for a JavaVM* pointer. When set, it treats the pointer as an object with a vtable and calls `vtable[6]` (which happens to be GetEnv from our JavaVM stub).
- Multiple guard bytes at VA 0x6a26e30-0x6a26e48 control different initialization paths.
- NULL dereference in JNI_OnLoad (`ldr x0, [sp, #16]` returns NULL, then `ldr x8, [x0]` faults).

**What was fixed:**
1. Code copy patch: `bl 26c0c7c` → `mov w0, #1` at copy offset +0x1a98
2. BSS pre-init: set guard bytes to 1 at 0x6a26e30-0x6a26e48  
3. SIGSEGV handler: NULL deref returns valid JNIEnv* (`&g_env`) in x0 + jni_table in x8
4. Expanded trampoline pre-resolution: 358 entries (all GSI lib symbols)

**Results:** Intermittent — when QEMU doesn't crash on JIT page writes, JNI_OnLoad runs without crashing but still hasn't returned (timeout at 10s). When QEMU hits the page-boundary SMC issue, SIGILL kills the process.

### 🎯 Next Steps (Priority Order)

1. **Fix QEMU JIT page-boundary crash for good** — Option: build QEMU with `--disable-tcg` and use a different JIT backend, or increase JIT page alignment, or use `mmap(MAP_NORESERVE)` to avoid SMC detection. The crash at `0x...ffb8` is from TCG code generation writing to the JIT buffer, not just from tb_set_jmp_target.
2. **Let JNI_OnLoad run to completion** — with the init deadlock bypassed, JNI_OnLoad makes real progress. 30+ second runs may be needed due to QEMU JIT slowdown.
3. **Implement RegisterNatives properly** — log and implement key native methods
4. **Once JNI_OnLoad returns** — Roblox main loop (render, network, scripts)

### QEMU Patch Details

Current patches in `/tmp/qemu-10.2.1/accel/tcg/`:

**`cpu-exec-common.c` (line 53):**
```c
cflags |= CF_NO_GOTO_TB;
```

**`cpu-exec.c` (line 600):**
```c
void tb_set_jmp_target(TranslationBlock *tb, int n, uintptr_t addr) {
    /* no-op */
    (void)tb; (void)n; (void)addr;
    tb->jmp_target_addr[n] = addr;
}
```

Build: `cd /tmp/qemu-10.2.1/build && make -j$(nproc) qemu-aarch64 && cp qemu-aarch64 ~/.cache/open-sober/qemu-patched`

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
- BL/B/B.cond/CBZ/TBZ offset fix  
- Mutex sanitization wrappers disabled (direct glibc call works)
- SIGSEGV handler for RELRO writes and JIT self-write bugs
- Pre-mprotect of RELRO pages

### 🎯 Next Steps (In Priority Order)

1. **Figure out why JNI_OnLoad hangs** — It spins up threads and waits. Options:
   - Use `-strace` to see what syscalls are blocking
   - Check if any required symbols are missing (Android runtime classes, JNI calls)
   - Enable JNI stub logging to see what JNI calls Roblox makes during init
   - The program might need more JNI stubs implemented (FindClass for specific classes, GetMethodID for specific methods)

2. **Re-enable mutex sanitization safely** — The `sanitize_and_lock` wrapper triggers a QEMU JIT bug on re-entry. The fix might be:
   - Use `__attribute__((noinline))` to prevent TCG cross-linking
   - Move the sanitize code inline into jni_shim.c instead of calling through bionic_shim trampoline
   - Simply skip sanitization if glibc's mutex works correctly (the Bionic ABI differences might not matter with glibc)

3. **Implement more JNI stubs** — Based on what JNI_OnLoad calls, add real implementations for key JNI methods:
   - `FindClass` needs to return the right class objects
   - `GetMethodID`/`GetStaticMethodID` might need real implementations
   - `RegisterNatives` needs to handle function registration

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