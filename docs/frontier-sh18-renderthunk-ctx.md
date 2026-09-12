# SH18 (2026-09-12) — render-init THUNK drive recovers the engine's REAL ctx object; corrected SH17's "don't drive the thunk" note; presents a real colored frame through the engine's own path

## One-line result
SH17 wrongly recorded the render-init **THUNK** 0x105b3a280 as "DON'T drive it
(SIGSEGV, shifts parent/win args)". That was a harness-arg bug, not a real
failure. This cycle reversed the thunk's exact signature, drove it correctly,
and recovered the engine's **real, guest-addressable 0x48-byte ctx object** —
with its own engine-populated vtable (0x106731ae0) and live EGL handles at the
canonical offsets — then presented a real colored frame through the engine's
own swap path on that real ctx. This is the bridge to frontier lever (2)
(drive the engine's frame-render machinery with a coherent, engine-owned
object). Workspace 470/0; baseline boot unchanged.

## The thunk signature (v2.738.1397 disasm)
```
thunk 0x105b3a280(x0=win, x1=parent):
    x21=x0; x20=x1; x19 = big_alloc(0x48)      ; bl 0x1d96768 (mempool big-alloc -> routed to host calloc)
    inner(0x105b3a2d8)(x0=x19 /*ctx*/, x1=x21 /*win*/, x2=x20 /*parent*/)
    ret x0=x19                                  ; the REAL ctx
```
The engine's own frame-render callers (0x5b2b214 via `ldr x0,[x23]; str x0,[x19,#344];
mov x1,xzr; bl 0x105b3a280`, and 0x5b2ea90 via `ldp x8,x1,[x0,#344]; mov x0,x8; bl 0x105b3a280`)
use exactly this: `bl thunk; ldr x8,[ctx]; ldr x8,[x8,#16]; blr x8`. SH17's crash came
from passing scratch-as-x0 (so inner took win=scratch) — the correct drive is
**thunk(x0=win=XID, x1=parent=0)**.

## What a correct drive proves (reproducible, captured)
```
[elfjit:renderinit] driving THUNK 0x105b3a280 ... x0=0x200000 x1=0x0
[elfjit:renderinit] returned Ok(0x7f6a40001d90)
[elfjit:renderthunk] REAL ctx 0x7f6a40001d90 vtable=0x106731ae0 egl: display=0x7f6a4008b5e0 surface=0x7f6a400a8fb0 context=0x7f6a400a6090
[elfjit:renderthunk] ctx vtable[0..5] = 0x105b3b288 0x105b3b334 0x105b3b358 0x105b3b408 0x101dc7428  — [vt+16](idx2)=disp target
[elfjit:renderframe] swap returned Ok(0x1) (eglSwapBuffers)      ; engine's swap on the real ctx
[elfjit:renderclear] glClearColor via bridge Ok / glClear via bridge Ok
[elfjit:renderclear] swap returned Ok(0x1)                       ; real colored frame
```
Captured window (ffmpeg x11grab, decomposed): dominant pixel RGB=(51,76,229) =
the requested blue 0.2,0.3,0.9 — see /home/hermes-worker/runs/sh18-thunk-blue.png.

## The recovered ctx object layout (fully mapped)
- `[ctx+0]`  = vtable 0x106731ae0 (engine-populated at runtime; the .data.rel.ro
  section has NO static RELATIVE relocs — the engine writes it itself, so live
  dump was required and now confirmed non-zero for the first 5 slots).
- `[ctx+16]` = share/parent-ctx shot (eglMakeCurrent's draw/read surface uses
  `csel x8,ctx,x8,eq` when [ctx+16]==0 -> ctx itself).
- `[ctx+24]` = ANativeWindow (the native window, used for ANativeWindow_get*).
- `[ctx+32]` = EGLDisplay, `[ctx+40]` = EGLSurface, `[ctx+48]` = EGLContext.
- `[vt+0]` 0x105b3b288 = destructor (resets current -> DestroyContext ->
  DestroySurface -> (Terminate -> ANativeWindow_release)) — the teardown method.
- `[vt+8]` 0x105b3b334 = dtor wrapper (then bl 0x626b6d0, the operator-delete
  path).
- `[vt+16]` 0x105b3b358 = **make-current-if-not-bound** method: `eglGetCurrentContext;
  cmp ctx,[ctx+48]; b.eq skip; eglMakeCurrent([display],[surface],[surface],[context])`
  — this is the method the engine's render callers dispatch to rebind the GL
  context before drawing. A correct entry for driving the engine's own render.
- `[vt+24]` 0x105b3b408 = the swap thunk (eglSwapBuffers).

## Next lever (now genuinely reachable)
Frontier lever (2) is opened: with the real engine ctx in hand (and its bind
method + swap on the vtable), we can drive the engine's OWN render-loop
recipe in a natural order — vtable[16] bind (eglMakeCurrent) -> frame-render fn
0x105b32c00 (glViewport/glScissor/glClearColor/glClear/glDrawElements, driven
via [x0]->[vt+16]->blr) -> vtable[24] swap — instead of force-driving glClear
directly. The remaining gap is the renderer C++ object that frame-fn 0x105b32c00
takes as x0 (its caller passes a renderer whose +24/+40 sub-objects carry the
clear/viewport state, +224/232/236/238 the mask flags); and ultimately the
ALooper/lifecycle producer that would drive this whole loop from the engine's
main thread (the SH14-capped deque wall). Baselines: --jni exit 0; stable idle
exit 124; workspace 470/0.

## Repro
```bash
cargo build -p arm64jit --example elfjit
timeout 45 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=1000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe --renderclear "0.2,0.3,0.9,1" \
  --kicker 0x106863af8
# expect REAL ctx + vtable dump, swap Ok(0x1), green/blue frame; exit 124
```
(old path unchanged: `--renderinit 0x105b3a2d8` without `--renderthunk` drives
the inner fn into the scratch buffer, as SH16/17.)
Run-logs: /home/hermes-worker/runs/sh18-renderthunk.txt,
sh18-renderthunk2.txt, sh18-thunk-frame.txt;
frame artifact: /home/hermes-worker/runs/sh18-thunk-blue.png;
capture script: /home/hermes-worker/runs/capture_thunk_frame.sh.