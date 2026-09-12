# SH22 (2026-09-12) — the engine's OWN frame-fn presents a real, correctly-colored frame. The SH21 "black window" wall is broken.

## One-line result
Driving the real engine frame-fn `0x105b32c00` (with fabricated renderer/view
+ a clear-state x4 object) now produces a **visible, correctly-colored 1280x720
frame** through the GLES bridge on a live Mesa llvmpipe context: 18430/18432
sampled pixels = the exact `--renderframe-color` (raw RGB 102,51,242 =
0.4,0.2,0.95), 99.987% non-black, `frame-fn returned Ok` + `post-frame swap
Ok(0x1)`, stable exit 124, zero heap abort. Run-log:
`runs/sh22-color-frame-from-engine-framefn.txt`; frame artifact:
`runs/sh22-color-frame-from-engine-framefn.{png,rgb}`.

## Root cause of SH21's black window (corrected from disassembly, not guessing)
SH21/19 seeded the 8 engine-GLES dispatch slots with per-slot function-name
*guesses* (slot0=glClearColor, slot2=glClearDepthf) and concluded "the clear
loop clears depth/stencil-style buffers, not color". Disassembly of the
clear-state sub-fn `0x5b32e08` shows that was **wrong on both slots**:

```
per-buffer clear loop 0x5b32ec8:
    mov w0,#0x1800          ; 0x1800 = GL_COLOR  (glClearBuffer buffer enum)
    mov w1, w22             ; drawbuffer index i (0..3)
    mov x2, x23             ; = clearstate+4 + i*0x10  (f32 value ptr)
    bl 0x5b3a1d8            ; slot2 (0x106d3b300)
depth/stencil branch 0x5b32f38:
    mov w0,#0x1801          ; 0x1801 = GL_DEPTH
    ... bl 0x5b3a1d8        ; slot2 again
```

So **slot2 is `glClearBufferfv(GLenum buffer, GLint drawbuffer, const GLfloat
*value)`** — 0x1800/0x1801 are the GL_COLOR/GL_DEPTH *buffer* enums for
glClearBufferfv, NOT glClear mask bits. Seeding slot2 with the `glClearDepthf`
float bridge (which ignores x0/x1/x2 and reads a nonexistent s0) meant the
engine's real color clear was being routed to a function that cleared nothing →
black window. Likewise **slot0 = `glDrawBuffers`** (the preamble builds
`{GL_COLOR_ATTACHMENT0..3}` or, for the default FB, `{GL_BACK}=0x405` arrays and
dispatches slot0), not glClearColor.

## The fixes
1. `resolver.rs`: add `glClearBufferfv` + `glDrawBuffers` to
   `GLES_INT_NAME_LIST` — both are pure integer/pointer ABI (<=8 args, no float
   s-regs), safe through the integer HostCall; they were returning None before,
   so the bridge couldn't seed the slots.
2. `elfjit.rs --renderframe-seedgles`: correct slot names slot0=glDrawBuffers,
   slot2=glClearBufferfv.
3. `elfjit.rs --renderframe-drive`: set `[objB+140]=0` so the default-framebuffer
   path takes `glDrawBuffers(1,{GL_BACK})` (was 1 → the 4-color-attachment
   variant, wrong for the window FB; tolerance of the error kept it non-fatal but
   the GL_BACK path is the correct one).

## Verified (reproducible, real-binary)
```
[elfjit:renderthunk] REAL ctx vtable=0x106731ae0 egl: display=.. surface=.. context=..
[elfjit:renderframe-seedgles] slot 0 (glDrawBuffers)  <- bridge 0x7f0000003010
[elfjit:renderframe-seedgles] slot 2 (glClearBufferfv) <- bridge 0x7f0000003018
[elfjit:renderframe-drive] engine frame-fn 0x105b32c00 returned Ok(0x..)
[elfjit:renderframe-drive] post-frame swap returned Ok(0x1)
```
Frame pixel decomposition (independent, numpy-free): 18432 samples, 18430 at
raw RGB(102,51,242) = fractions (0.4,0.2,0.95) = the exact `--renderframe-color
0.40,0.20,0.95,1`; 1 white + 1 black pixel (window edge/grab-boundary); total
black 116/921600 (0.013%). No channel swap (the SH21 capture script mislabeled
grab byte order as b,g,r; raw rgb24 is r,g,b).

## Remaining frontier (unchanged shape)
The frame is still *harness-driven*: the fake renderer/view/clear-state objects
are fabricated and the engine's frame-fn is invoked once from the host before a
manual swap. The engine's real main-loop producer still never enqueues a render
task, so it does not yet drive frames natively. The mechanical reverse of slot0
(glDrawBuffers) and slot2 (glClearBufferfv) removes the last guess-blocker in
the engine's own clear path. Baselines unchanged: `--jni` exit 0; stable idle
main loop exit 124. Next: drive the engine's own render-loop recipe
(vtable[16] bind -> frame-fn -> vtable[24] swap) in natural order from the
engine's real thread, or reverse the second clear-source object at the main-fn
x22 (0x105b32d5c `ldr q0,[x22]; str q0,[x27]`).