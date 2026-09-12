# SH17 (2026-09-12) — real Roblox RENDERS a real colored FRAME headlessly; live EGL context + swap + GLES-bridge clear all succeed on this GPU-less VPS

## One-line result
Building on SH16b (real render-init 0x105b3a2d8 completes its EGL chain and returns
Ok(0x0)), this cycle drove the frame-present path on top of that live context. The real
binary now, through the JIT: **glClearColor → glClear → eglSwapBuffers all succeed on its
own live Mesa-llvmpipe EGL context against a real Xvfb X11 window**, and the captured
window is a **solid green canvas** (the RGB we set) — real rendered pixels presented
headlessly by the running engine's own render path. Workspace 470/0.

## The two new harness levers (emit+tested, both opt-in, baselines unchanged)
1. **`--renderframe`**: after `--renderinit` returns Ok(0x0), dump the live EGL handles
   the render-init wrote into the scratch context buffer, then `jit_run` the engine's OWN
   swap fn **0x105b3b408** (`ldp x8,x1,[x0,#32]; mov x0,x8; b eglSwapBuffers`) with
   x0=scratch → `eglSwapBuffers([+32]=display,[+40]=surface)` returns **Ok(0x1)=EGL_TRUE**
   on the same host thread (context stays current). First real present through the real
   engine code path.
2. **`--renderclear <r,g,b,a>`**: draw a colored clear through the JIT's **GLES float
   bridge** on that live context — drive `glClearColor@plt 0x1062d7710` (float lanes
   s0..s3 = v[0..6]) then `glClear@plt 0x1062d7740` (GL_COLOR_BUFFER_BIT=0x4000 in x0),
   then swap again. All three return via the bridge. Captured the live window with
   `ffmpeg -f x11grab`: the frame is a **solid green** canvas (the 0.1,0.7,0.2,1 color).

## Reverse-engineering that makes it possible
- render-init's inner fn 0x105b3a2d8 stores its real EGL handles into the context object
  at fixed offsets: `str x0,[x19,#32]` (eglDisplay), `str x1,[x19,#40]` (surface),
  `str x0,[x19,#48]` (context) — verified by reading live values back from the scratch
  buffer the harness passes as x0 (0x7ff3.. display/surface/context, real Mesa objects).
- Because the harness passes a guest-writable scratch buffer as the inner fn's x0 (the
  "bare scratch, prologue STORES into *x0" pattern from SH16), that same buffer holds the
  live handles the swap fn reads — so no thunk-return plumbing is needed.
- **Abandoned path (documented so we don't re-run):** driving the render-init THUNK
  0x105b3a280 (which allocates the real 0x48-byte ctx via bl 0x1d96768 then inner-inits
  it and returns ctx in x0) to recover the "real" ctx pointer → SIGSEGV at the inner call
  because the thunk shifts its parent/window args across the inner boundary differently
  than the direct drive. The scratch-reuse approach is cleaner and works.

## Frontier (what a real session now needs)
The engine's own main-loop producer still never enqueues a render task onto its idle
futex/ALooper, so the engine itself never issues glClearColor/glClear/draws in its loop —
this cycle's colored clear + swap were driven by the harness on the live context, not by
the engine's main loop. Next levers (unchanged shape, now with a fully-working render
pipeline behind the wall):
1. Drive the ALooper app-command lifecycle so StartApp's real producer enqueues a render
   task (SH14/SH7/N/P), letting the engine's OWN frame loop run glViewport/glClearColor/
   glClear/glDrawElements/eglSwapBuffers natively — now all verified reachable via the
   bridge once a real task lands.
2. Alternatively drive the engine's render-loop function (frame-clear region 0x105b32c40-…,
   glClearColor@0x105b32f8c/glClear@0x105b32fdc, glViewport@0x105b32ca4, glDrawElements@
   0x105b35334) directly with a coherent render-state object, once its object layout is
   reversed. The GLES float bridge + compressed-texture interception (SH3/SH3b) are ready.

## Repro (headless, reproducible)
```bash
cargo build -p arm64jit --example elfjit
# baseline unchanged: --jni exit 0; --startapp stable idle exit 124
# present the engine's surface (eglSwapBuffers via engine's own swap fn):
timeout 40 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=800 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a2d8 --renderframe --kicker 0x106863af8
# expect: renderinit Ok(0x0) -> renderframe swap returned Ok(0x1); exit 124
# render a GREEN frame (clear+swap through the GLES bridge), then capture the window:
bash /home/hermes-worker/runs/capture_frame.sh   # --renderclear 0.1,0.7,0.2,1
```
Run-logs: `/home/hermes-worker/runs/sh17-renderframe2.txt` (swap Ok), 
`/home/hermes-worker/runs/sh17-frame-clearcap.txt` (clear+swap), 
capture script `/home/hermes-worker/runs/capture_frame.sh`.
Frame artifact: `/home/hermes-worker/runs/sh17-frame-green.png` (solid green 1280x720).
Baselines: `--jni` clean exit 0; stable idle main loop exit 124; workspace green 470/0.