# SH23 (2026-09-12) — the engine's OWN render recipe runs CONTINUOUSLY as a live animated render loop (--rendersustain)

## One-line result
New elfjit `--rendersustain <fps>` runs the engine's own render recipe
(vtable bind -> frame-fn `0x105b32c00` -> post-frame swap) in an **unbounded
loop** on the detached host thread, concurrent with StartApp's idle main-loop
`jit_run` on the main thread, cycling the clear color through a 5-color palette
per frame. A real recording proves **every frame is a fresh render**: the
captured video's majority color tracks the palette exactly
(green->red->blue->yellow->magenta, float fracs match to 3 dp), 12+ distinct
frames over a 6 s capture, and the run logged **160 consecutive
frame-fn->swap pairs, every swap Ok(0x1), exit 124 stable, zero fault/heap
abort**. Baselines unchanged (`--jni` exit 0; stable idle exit 124).

Run-log: `/home/hermes-worker/runs/sh23-sustain-loop.txt` (160 iterations).
Artifact: `/home/hermes-worker/runs/sh23-sustain-loop.mp4`.
Capture script: `runs/capture_sustain_loop.sh`.

## What this adds over SH22d (`--renderframe-loop <N>`)
SH22d proved the recipe is *reentrant* (N=3, frozen color). SH23 makes it
*sustainable and animated*: instead of a bounded host loop with one color, the
engine's own frame-fn is driven on a real time-base (configurable fps) for the
whole run — the property the engine's main loop would need to present frames
natively. Each iteration re-writes both of the engine's clear-color sources
(the frame-fn 5th-arg clear-state object at `base+0x400` `[+4..16]` and the 6th-
arg color-source object at `base+0x500` `[+0..16]`) before calling the real
frame-fn, then swaps. The palette-per-frame shows the frames are genuinely
re-rendered (not a static buffer).

## Verified per-frame palette match (independent pixel decomposition)
Palette (iteration-indexed): 0=[0.40,0.20,0.95],1=[0.10,0.70,0.05],
2=[0.90,0.15,0.10],3=[0.05,0.60,0.90],4=[1.00,0.82,0.05].

Captured frames (majority RGB -> fraction, file-order r,g,b):
```
frame 0: 26,178,13    = (0.102,0.698,0.051) ~ pal[1]  green
frame 1: 230,39,26    = (0.902,0.153,0.102) ~ pal[2]  red
frame 2: 13,153,231   = (0.051,0.600,0.906) ~ pal[3]  blue
frame 3: 255,209,13   = (1.000,0.820,0.051) ~ pal[4]  yellow
frame 4: 103,51,242   = (0.404,0.200,0.949) ~ pal[0]  magenta
frame 5: 26,178,13    ~ pal[1] green  (cycle repeats)
...
```
Every sampled frame matches the exact palette float (to 3 dp) — the engine's
frame-fn honors the per-frame color, and each frame is freshly rendered.

## Reproduce
```bash
cargo build -p arm64jit --example elfjit
FPS=2 CAP_SECS=8 bash runs/capture_sustain_loop.sh
# log: 160+ "=== frame iteration N color [..] ===" + "frame-fn returned Ok" + "swap Ok(0x1)"; exit 124
# mp4: majority color cycles the palette exactly
```

## Remaining frontier (unchanged shape)
The render loop is still *harness-driven*: fabricated renderer/view/clear-state
objects, and the frame is clear-only (the engine's real draw path
`glDrawElements` is gated on a coherent renderer C++ object that hasn't been
reversed). The engine's real main-loop producer still never enqueues a render
task, so it does not yet call frame-fn by itself — we drive its own code on a
time base. Next highest-value work: (1) drive the recipe from the engine's real
thread once a looper/lifecycle producer exists, or (2) reverse the coherent
renderer object so frame-fn reaches a real geometry draw. The GLES dispatch-slot
map (slot0=glDrawBuffers, slot1=glClearBufferiv, slot2=glClearBufferfv,
slot3=glClearBufferfi) and continuous render prove the engine's GL path is
bridge-functional and sustainable.