# SH21 (2026-09-12) — reverse: the engine frame-fn's clear-color object is its **5th arg x4**, not x2. Providing it makes the engine's OWN clear-state sub-fn execute GL (glColorMask + per-buffer clear dispatches) through the bridge. Workspace 471/0.

## One-line result
The engine's own frame-fn 0x105b32c00 now reaches its real CLEAR path:
with a fabricated x4 clear-state object, the clear-state sub-fn 0x105b32e08
fires glColorMask(all-1) + a per-buffer clear dispatch loop (4 buffer bits)
+ glGetError — all through the host-thunk GLES bridge on the real recovered
context. Before this, SH20's drive stopped SILENTLY after the GL preamble
(glBindFramebuffer/glViewport/glScissor) and never issued any clear.

## The reverse (corrects SH20's false premise)
SH20's --renderframe-drive comment guessed the clear-color struct ptr was the
frame-fn's 3rd arg (x2). That is WRONG. Disassembly:

```
frame-fn 0x105b32c00 prologue:  0x105b32c30  mov x20, x4        ; x4 -> x20
clear gate:                      0x105b32d44  cbz x20, +24      ; x4==0 -> skip clear
                                 0x105b32d48  ldr w1,[x20]
                                 0x105b32d4c  cbz w1, +16       ; [x4]==0 -> skip clear
                                 0x105b32d50  mov x0, x19       ; (renderer)
                                 0x105b32d58  bl 0x105b32e08    ; clear-state sub-fn
clear-state sub-fn 0x105b32e08:  0x105b32e1c  mov x21, x2       ; x2 = x4
clear color read:                0x105b32f84  ldp s0, s1,[x21,#4]   ; [x4+4],[x4+8]
                                 0x105b32f88  ldp s2, s3,[x21,#12]  ; [x4+12],[x4+16]
```

So the clear color RGBA float4 lives at **[x4+4..16]**, and the sub-fn is
gated on **x4 != 0 AND [x4] != 0**. SH20 passed x4 = 0 (unset), so the gate
at 0x105b32d44 cbz x20 skipped the whole clear path — the window stayed black.

## The fix (elfjit --renderframe-drive)
Fabricate a clear-state object in the 8KiB scratch and pass it as the frame-fn's
5th arg x4:
- [+0]  = 0xF  (w20 = per-buffer clear bitmask; also the !=0 gate). 0xF = all 4 buffer bits.
- [+4..+20] = RGBA float4 clear color (default 0.4,0.2,0.95,1; overridable via
  new `--renderframe-color r,g,b,a`).

Also fabricate a 6th-arg (x5 -> x22) color-source object at base+0x500 (RGBA
float4 at [+0..16]; `0x105b32c28 mov x22,x5`; main-fn `0x105b32d5c cbz x22` /
`ldr q0,[x22]`). Providing it has no observable change yet (the color clear is
still gated behind the slot/buffer semantics below), but it is part of the
coherent clear-object reverse.

## JIT_TRACE evidence (new hostcalls that never fired before this fix)
The drive's call sequence, with the x4 object present (0xF):

```
hostcall@glBindFramebuffer x0=0x8d40 x1=0x0            (bind default fb)
hostcall@glViewport        0,0,1280,720
hostcall@glScissor         0,0,1280,720
hostcall@glColorMask       x0=1 x1=1 x2=1  x30=0x105b32e44  <-- sub-fn, NEW
hostcall@glGetError        (b.ne OOM-check at 0x105b32e48)
... per-buffer clear loop at 0x105b32ec8 (mov w0,#0x1800; bl slot2-stub; glGetError)
     iterates the 4 bits of w20=[clearobj+0]
```
glColorMask + the buffer-clear loop at 0x105b32ec8/0x105b32ed8 are entirely NEW
hostcalls enabled by the x4 object. The drive still returns cleanly:
`engine frame-fn 0x105b32c00 returned Ok(...)` + `post-frame swap Ok(0x1)`,
stable exit 124. Run-log: runs/sh21-clearstate-x4.txt.

## Honest remaining wall (a visible COLOR frame still not achieved)
The window still captures black. The per-buffer clear loop dispatches slot2
(seeded as glClearDepthf — a GUESS; the 8 slot->function names in
--renderframe-seedgles are heuristic) with integer arg 0x1800, so it clears
depth/stencil-style buffers, NOT the color buffer. Achieving a visible colored
frame needs the coherent renderer/clear-state reverse to identify:
1. the TRUE function of the slot that 0x105b32ec8 dispatches (and slot0/2's
   real names — the seed names are guesses, so the "clear" dispatch may be
   mis-routed);
2. which buffer-bit of w20 maps to GL_COLOR_BUFFER (and whether glClearColor's
   float4 should feed a separate glClear).
3. the second clear-state object the main fn derefs at 0x105b32d5c (x22,
   `cbz x22; ldr q0,[x22]; str q0,[x27]`) — likely the color-clear source that
   would make a real glClear(GL_COLOR_BUFFER_BIT) use the seeded color.

Baselines unchanged: --jni clean exit 0; stable idle main loop exit 124;
workspace 471/0.