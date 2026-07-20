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
10. **Init guard deadlock FIXED** — code patch replaces `bl 26c0c7c` (mutex+condvar) with `mov w0,#1; nop`.
11. **Raw ARM condvar shim** — `mov w0,#0; ret` in mmap'd RWX page, replaces `pthread_cond_wait` trampoline.

### 🟢 QEMU JIT crash is fixed (Session 6 breakthrough)

The CF_NO_GOTO_TB + tb_set_jmp_target no-op patches in the custom QEMU (`~/.cache/open-sober/qemu-patched`) are sufficient when JNI_OnLoad is called **directly** (no code copy). The code copy introduced adrp fix bugs that caused SIGILL.

### 🟡 Current Blocker — QEMU JIT slowness via _dl_mcount O(n²) strcmp loop

**What we found:** The "spin loop" is NOT a hang — it's a **QEMU JIT performance bottleneck** in glibc's `_dl_mcount` function in `ld-linux-aarch64.so.1`.

Using `-d exec` traces, the hottest guest PC is **`0x1e2d8` in ld-linux** — the core of an `strcmp` loop that is the `_dl_mcount` call-record maintenance. This function is called for EVERY PLT-resolved trampoline invocation and iterates a growing linked list of call records, doing strcmp on each entry. With 785 trampoline entries being called, this becomes an O(n²) bottleneck.

**Evidence:**
- Hottest code: `_dl_mcount+0x7834` strcmp loop — 600k iterations in 5 seconds
- All hot PCs are in `ld-linux-aarch64.so.1` (offsets 0x9000-0x1e300 range)
- NOT in `libc.so`, `libm.so`, or `libroblox.so`
- NOT a hang or deadlock — pure CPU-bound O(n²) strcmp

**Attempted fix — noping _dl_mcount:**
- Patching `_dl_mcount` entry with `ret` + noping 128KB of its body **did not help**
- Root cause: QEMU JIT caches translated blocks, so host-side writes to guest memory don't invalidate the TCG translation cache (same SMC issue from earlier sessions)

**Why it happens:**
1. Every trampoline call goes through `__bf_c_resolve` → `dlsym(RTLD_NEXT)` → PLT
2. glibc's PLT calls `_dl_mcount` for profiling
3. `_dl_mcount` does strcmp-based search through a linked list of call records
4. Under QEMU JIT, each strcmp loop iteration is JIT-translated and slow
5. With 785 trampolines × many calls, this dominates

### 🎯 Recommended Next Steps

1. **Run the same test on real ARM64 hardware** — if it completes quickly, the issue is QEMU JIT slowness, not a real bug
2. **Disable `_dl_mcount` at compile time** — rebuild `ld-linux` without profiling support, or use `LD_BIND_NOT=1` / set glibc profiling env vars
3. **Use `-one-insn-per-tb`** to avoid TB chaining (might slow things further but could break the strcmp loop pattern)
4. **Modify bionic_shim.S** to bypass the PLT entirely — call resolved functions directly instead of going through `__bf_c_resolve` → PLT → _dl_mcount
5. **Add JNI_OnLoad to the init calls that get patched** — if we let the init function complete (don't set guard=1, let condvar shim handle the deadlock), the init stores proper BSS state which might reduce the number of trampoline calls needed

### What was fixed in session 6

1. **All 785 trampoline entries pre-filled** — remaining 429 filled with safe default (`__errno_location`) to prevent `__bf_c_resolve` from calling `dlsym(RTLD_NEXT)`
2. **Condvar shim installed** at tramp[39,96] — `mov w0,#0; ret` prevents all condvar deadlocks
3. **Mutex sanitization wrappers** — Bionic→glibc pthread_mutex_t ABI fix (kind field, __owner leak)
4. **SIGSEGV handler improved** — bad-address faults chain to old handler instead of setting x0=&g_env
5. **Removed null-deref hack** — was making crashes worse by setting invalid JNIEnv*
6. **Use direct JNI_OnLoad call** — no code copy needed; QEMU patches handle the JIT issue

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