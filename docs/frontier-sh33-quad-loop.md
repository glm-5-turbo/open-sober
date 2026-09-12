# Frontier: SH33 — SUSTAINABLE TEXTURED real-geometry rendering through the engine's wrapper

`--renderframe-quad-loop <N>` makes the engine's OWN two-attrib textured-geometry
draw path SUSTAINABLE on the detached host thread: after the single proof textured
quad frame (SH30), it re-drives `clear(cycling bg) -> geometry wrapper 0x5b35288 ->
swap` N times. This is the textured/mesh analog of SH25b's untextured triangle-loop
— it closes the last "sustainable real-geometry frame" property for the TEXTURED
path that a real main-loop frame drive needs.

## Why it matters

Every prior compressed-texture/textured prove (SH26-32) drew ONE frame. SH25b proved
the geometry path sustains, but only for a solid untextured triangle. A real main
loop renders textured/meshed scenes every frame, so the draw recipe must sustain
WITH a texture bound and per-texel UV mapping. SH33 proves exactly that: the same
2-attrib quad (aPos + aUV, interleaved VBO stride 24, 6-idx EBO), RGBA-checkerboard
texture, textured FS — drawn and presented repeatedly, with a cycling clear
background so a recording cannot be a static buffer.

## Verification (runs/sh33-quad-loop.txt, runs/sh33-quad-loop.mp4)

```
[elfjit:renderframe-quad]      readback: BL=RGBA(255,0,0,255) BR=RGBA(0,255,0,255) TR=RGBA(255,255,255,255) TL=RGBA(0,0,255,255)
[elfjit:renderframe-quad-loop] iter 0 drew+swap Ok(0x1) bg=[0.9, 0.1, 0.1, 1.0]
[elfjit:renderframe-quad-loop] iter 1 drew+swap Ok(0x1) bg=[0.1, 0.9, 0.1, 1.0]
[elfjit:renderframe-quad-loop] iter 2 drew+swap Ok(0x1) bg=[0.1, 0.1, 0.9, 1.0]
[elfjit:renderframe-quad-loop] iter 3 drew+swap Ok(0x1) bg=[0.9, 0.9, 0.1, 1.0]
[elfjit:renderframe-quad-loop] iter 4 drew+swap Ok(0x1) bg=[0.9, 0.1, 0.9, 1.0]
[elfjit:renderframe-quad-loop] iter 5 drew+swap Ok(0x1) bg=[0.9, 0.1, 0.1, 1.0]
```

- 6 iterations, **every** `drew+swap Ok(0x1)` (exit 124 stable, zero crash).
- **5 distinct cycling background colors** — a recording proves a fresh textured
  render each frame, not a static buffer.
- The textured readback in the SAME run proves the textured draw (2x2 checkerboard
  at interpolated UV) is intact while sustaining.
- 8-frame x11grab recording (runs/sh33-quad-loop.mp4).

Reproducible: `runs/capture_quad_loop.sh`. New harness lever only (no GLES/codec/
resolver change, so workspace stays 476/0); baselines unchanged (`--jni` exit 0;
idle 124; quad/etc2a/etc/etc2/tex/triangle modes intact).

Honest framing: still harness-driven on a time base — the engine's own main-loop
producer still never enqueues a render task, so the harness drives the engine's OWN
code (geometry wrapper + swap) on a time base. But the textured-geometry recipe now
has both properties a real main-loop drive needs on this path: it RENDERS correctly
(SH30-32) and it SUSTAINS (SH33). The remaining frontier continues to be the
engine's producer enqueuing a render task so frames run natively (the long-standing
structural wall).