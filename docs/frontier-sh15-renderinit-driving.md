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

## SH16 additions (2026-09-12) — the abort reason now SURFACES + render-init reaches eglCreateWindowSurface with a FAILED window

Two findings closed this cycle (workspace still green, test total 470 → **470/0**):

1. **The terminate reason was masked by a SECOND unshimmed print.** The libc++ terminate
   handler writes its body with **`vfprintf`** — not just `fwrite` (which SH15 shimmed).
   The guest hands glibc's `vfprintf` its bionic `FILE*` (=0x130 stderr) + an AAPCS64
   `va_list`; glibc derefs it as a host `FILE_`/host va_list → SIGSEGV before ANY text
   of the real exception showed. New **`vfprintf` shim** (`bionic_vfprintf`) diverts
   guest/bionic streams to fd 2, decoding the AArch64 va_list (`__stack/__gr_top/
   __gr_offs`). Now the reason is fully visible:
   ```
   libc++abi: terminating due to uncaught exception of type std::runtime_error:
   Error creating context: eglCreateWindowSurface ...
   ```
   Regression `vfprintf_va_list_decoder_renders_terminate_message` pins the decoder.

2. **The real binary's render-init now reaches `eglCreateWindowSurface`** (the SH14
   gateway) when the run wires a real window: run under `JIT_DRIVE_LIFECYCLE=1`
   (which calls `wire_real_window` → sets DISPLAY/EGL_PLATFORM=x11 → XID 0x200000):
   ```
   ANativeWindow_fromSurface -> ANativeWindow_acquire -> eglGetDisplay
     -> eglInitialize -> eglChooseConfig(x3) -> eglCreateContext -> eglCreateWindowSurface
   ```
   Which then FAILS: the call lands with `x2=0x0` (native window arg NULL) because the
      render-init's context reads its native window from the framework-built context
      object (`[ctx+24]`) which the isolated-thread harness cannot populate (x0 is a bare
      scratch buffer; passing the live `*0x1067d16f0` corrupts it since render-init's
      prologue stores INTO *x0). eglCreateWindowSurface returns NULL → runtime_error →
      terminate — now VISIBLE instead of a silent SIGSEGV. With NO window wired the abort
      is the eglInitialize error; WITH the window the rund goes one full EGL stage farther
      to the surface. The exact next lever is supplying a coherent native window handle in
      the render-init's context — through the ALooper/framework path or by seeding the
      guest window global render-init reads.

   ## SH16b — the native window comes via x1: render-init's FULL real EGL chain now SUCCEEDS headlessly (window surface + context made current)

   Root-causing the x2=0 from above: at the real call site 0x105b2ea90 the caller does
   `ldp x8, x1, [x0, #344]` then `bl render-init` — so the render-init's **x1 param is
   the ANativeWindow** (loaded from `[parent+352]`), which the prologue moves to x22 and
   stores to `[ctx+24]` (0x105b3a340 `str x22,[x19,#24]`), i.e. exactly the window field
   `eglCreateWindowSurface`'s wrapper (0x105b3b194 `ldp x2,x8,[x0,#24]`) reads as its
   native-window arg. The harness was passing `s3.x[1]=0` ⇒ win=0 ⇒ eglCreateWindowSurface
   returned NULL. **Passing the wired XID as x1** (`s3.x[1] = anativewindow_xid()` = the
   real X11 XID 0x200000 that Mesa's x11 EGL platform expects as its native window):

   ```
   hostcall@ANativeWindow_fromSurface
   hostcall@ANativeWindow_acquire
   hostcall@eglGetDisplay
   hostcall@eglInitialize
   hostcall@eglChooseConfig (x3)
   hostcall@eglGetConfigAttrib
   hostcall@eglCreateContext
   hostcall@eglCreateWindowSurface  x2=0x200000   <-- the real window
   hostcall@eglMakeCurrent          surface, surface, context
   hostcall@eglQuerySurface (x2)
   hostcall@eglSwapInterval
   [elfjit:renderinit] returned Ok(0x0)
   ```

   The REAL Roblox binary now executes its complete render-init EGL setup against Mesa
   llvmpipe + a real Xvfb X11 window **headlessly on this GPU-less VPS** and returns
   successfully (exit 124 = the engine main loop still idles afterward, no crash). This
   crosses the SH14-identified gateway (eglCreateWindowSurface/eglMakeCurrent) that was
   declared framework-gated. Worklog: /home/hermes-worker/runs/sh16-renderinit-window-x1-success-runlog.txt.
   Next: the engine's frame loop (gl* calls) once the main-loop producer enqueues a render
   task — the long-standing idle-futex/ALooper wall, now with a live EGL context behind it.

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