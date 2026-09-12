# SH15 (2026-09-12) — CORRECTION to SH14 + the real render-init now DRIVES its real EGL chain headlessly

## One-line result
SH14's central claim is **wrong at runtime**: the render-init context global
`0x1067d16f0` is NOT statically-0-and-framework-gated — StartApp **populates it**
during boot (verified live: `0x562a..` while the engine idles). This reopens the
rendering path SH14 declared a dead-end. Built the `--renderinit` harness and
**drove the real `libroblox.so` render-init fn `0x105b3a2d8` directly**: it now
executes its REAL EGL chain —
`ANativeWindow_acquire` → `eglGetDisplay` → `eglInitialize` → `eglGetError` —
**through the JIT resolver bridges** (headless, llvmpipe), then libc++ aborts
(the terminate reason was masked; surfaced via a new fwrite shim). First real
graphics motion from the real binary on this GPU-less box.

## Corrections to SH14 / STATUS
1. **`0x1067d16f0` (render-init context) is populated at runtime by StartApp** —
   not statically 0. New `JIT_FRAMEWORK_DUMP` diagnostic reads it live during
   the idle loop: `0x562a..` (a host-heap object StartApp builds). So
   "framework-gated, not host-drivable as-is" (SH14) is FALSE for the context
   global — the render-init is drivable.
2. **The deque-maintenance forward-edges `0x1068262e8 / 0x106826300 /
   0x106826308` are ALSO populated at runtime** (`0x10620db24 / 0x102176bfc /
   0x1022199e0` — real .text addresses), not statically 0. (Deque-injection is
   still capped on the `w4=4` task-type, per SH14 — that part stands.)

## What was built (dev branch, workspace 469/0… green, baseline intact)
- **`JIT_FRAMEWORK_DUMP`** (elfjit): a detached sampler thread reads the
  framework globals the render/deque paths depend on, live, every ~500 ms.
- **`--renderinit <guest-addr>`** (elfjit): after StartApp warm-up (default 5s,
  `RENDERINIT_WARMUP_MS` to override), spawns a detached host thread that
  `jit_run`s the real render-init fn (`0x105b3a2d8`, thunk `0x105b3a280`) as a
  fresh guest entry (boot SP + tpidr + a writable scratch `x0`, since the real
  caller passes `[parent+344]` and a 0 x0 NULL-stores in the prologue). Safe to
  run concurrent with StartApp's parked main thread (block cache leaks, never
  munmaps). NOTE: the address is a GUEST address — do NOT `el.guest_of()` it.
- **`dl_iterate_phdr` shim** (shims.rs): the guest passes libc++/engine a GUEST
  AArch64 callback; host glibc executed it as host x86 → SIGILL. Now routed back
  through `jit::run_guest_callback` via a host trampoline. Real gap fixed.
- **`fwrite` shim** (shims.rs): the guest passes bionic `FILE*` (its stderr can
  be a bare `0x130`); host glibc fwrite derefs it → SIGSEGV, masking the abort
  reason. Now diverts guest/non-host streams to host fd 2.

## The real wall (next)
The real render-init runs its EGL chain, then **`libc++abi:` terminate-aborations**
(unsurpressed after the fwrite fix) at a fatal condition — almost certainly a
missing-coherent-framework dependency of the synthetic drive (the context object
fields, the real ANativeWindow/config, a GL probe, or the ALooper lifecycle state
StartApp's framework producer would supply). The abort handler then faults in
`jit_run_inner`'s pass-through hostcall dispatch. Getting past the abort needs
either (a) feeding render-init a complete coherent EGL context/native-window, or
(b) driving the ALooper app-command lifecycle so StartApp's real producer sets up
the state render-init derefs — the SH14/SH7/N/P framework-emulation path. The
glorious part: the real binary's EGL path is now **executing**, not just gated.

## Repro
```bash
cargo build -p arm64jit --example elfjit
# baseline (unchanged): --jni exit 0; --startapp stable idle exit 124
timeout 60 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a2d8 --kicker 0x106863af8
# JIT_TRACE=1 shows the real chain: ANativeWindow_acquire -> eglGetDisplay ->
# eglInitialize -> eglGetError (x30 = render-init call sites), then libc++abi: abort.
```
Run-logs: `/home/hermes-worker/runs/sh15-renderinit-eglchain-runlog.txt`.
Baselines: `--jni` clean exit 0; stable idle main loop exit 124; workspace green.
Next lever: drive the ALooper app-command lifecycle / feed render-init a coherent
ANativeWindow so the engine's abort becomes a real llvmpipe frame.