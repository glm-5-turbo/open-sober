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

## Current Status (July 20, after session 7)

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

### 🟢 QEMU JIT crash is fixed (Session 6 breakthrough)

The CF_NO_GOTO_TB + tb_set_jmp_target no-op patches in the custom QEMU (`~/.cache/open-sober/qemu-patched`) are sufficient when JNI_OnLoad is called **directly** (no code copy). The code copy introduced adrp fix bugs that caused SIGILL.

### 🟡 QEMU JIT _dl_mcount profiling bottleneck

The "spin loop" is NOT a hang — it's a **QEMU JIT performance bottleneck** in glibc's `_dl_mcount` function in QEMU's internal ld-linux-aarch64 dynamic linker.

Using `-d exec` traces, the hottest guest PC is **`0x1e2d8` in ld-linux** — the core of an `strcmp` loop that is the `_dl_mcount` call-record maintenance. This function is called for EVERY PLT-resolved trampoline invocation and iterates a growing linked list of call records, doing strcmp on each entry. With 785 trampoline entries being called, this becomes an O(n²) bottleneck.

**Root cause:** glibc's `_rtld_global.dl_profile` field (offset 0xCA0 from `_rtld_global`) has the value **0x40** — profiling is ENABLED by default. Every PLT resolution enters the strcmp loop.

**Fix applied:** Runtime noping of `_dl_mcount` with mprotect + write 0xd65f03c0 (ret) at the entry point. The mprotect forces QEMU's TCG JIT cache to invalidate the page, making the patch take effect at runtime. Note that disk-patching ld-linux doesn't work because QEMU user-mode loads the guest binary through its own internal loader, not the guest's ld-linux.

### 🟡 Current Blocker — JNI_OnLoad crashes with pc=0x0

After fixing the _dl_mcount bottleneck and pre-resolving all 785 trampoline entries, we reach JNI_OnLoad but crash with `pc=0x0` (jump to NULL). This happens during the first few function calls inside JNI_OnLoad.

**Root cause:** The 128KB runtime noping of `_dl_mcount` corrupts the PLT fixup code following `_dl_mcount` in ld-linux. When a GSI library calls through lazy PLT resolution during JNI_OnLoad, the fixup returns 0. With `RTLD_NOW` on libroblox, its PLT is resolved at load time, but GSI libraries (transitive depdendencies) may use lazy binding.

**Attempts that didn't work:**
- Disk-patching ld-linux: QEMU user-mode uses its own internal loader, not the guest's ld-linux
- Zeroing dl_profile at runtime: QEMU's TCG cache has already translated the check with the cached 0x40 value
- 4-byte only noping + 1-page mprotect: not enough TCG cache flush; process too slow to produce output

**What does work:** 128KB noping `mprotect(0x10000) + write ret` — aggressively flushes QEMU's JIT cache. The corrupted PLT fixup returns 0 for any lazy PLT resolution.

### 🎯 Recommended Next Steps

1. **Fix the pc=0x0 crash** — either prevent GSI libraries from needing lazy PLT resolution (use RTLD_NOW for all of them) OR handle the lazy PLT resolution differently
2. **Alternative: use QEMU's `-one-insn-per-tb`** — avoids TB chaining entirely, which eliminates goto_tb SMC issues but may be slower
3. **Alternative: rebuild patched QEMU** — add a proper SMC invalidation handler that catches host-side writes to guest code pages

### What was fixed in session 7

1. **Full 785-entry pre-resolve** — replaced 359-entry static table with data-driven loop over all 785 symbol names (782/785 resolved via dlsym, 3 Bionic-only fallbacks)
2. **Diagnosed dl_mcount profiling** — discovered rtld_global.dl_profile = 0x40 (profiling ENABLED). Clarified QEMU user-mode uses its own ld-linux, not the guest one
3. **Refined noping approach** — 128KB noping + mprotect works but corrupts PLT fixup code. 4-byte noping with correct mprotect range is the right approach but needs larger cache flush

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