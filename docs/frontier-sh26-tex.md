# Frontier: SH26 — Real TEXTURED render through the engine's own geometry wrapper

The engine's OWN coherent geometry draw path (wrapper 0x5b35288 -> primitive-setup
0x5b353d0 -> indexed glDrawElements) now renders a REAL TEXTURED triangle. The
`--renderframe-tex` lever adds to the SH25 coherent-renderer harness a 2x2 RGBA
checkerboard texture (texels RED/GREEN/BLUE/WHITE) whose fragment shader samples it
via a UV derived from `gl_FragCoord`. Every texture/uniform/shader call dispatches
through the JIT GLES bridge (@plt -> host-thunk slots).

## What this proves

The GLES texture+uniform+shader bridge surface the engine's real textured draws need
is end-to-end functional on the live llvmpipe context:

- `glGenTextures(1,&tex)` -> tex_id (int bridge, out-slot id).
- `glActiveTexture(GL_TEXTURE0)` + `glBindTexture(GL_TEXTURE_2D,tex)`.
- `glTexParameteri(MIN/MAG_FILTER, GL_NEAREST)` (no mipmap required; each probed
  quadrant yields one exact texel color).
- `glTexImage2D` — **9 args** (pixels rides the guest stack, arg 8 at [sp+0]); the
  PLT stub is a leaf (`adrp/ldr/add/br`, never pushes sp), so a fake sp whose [0]
  holds the pixels pointer is read by the bridge's `gs_stack`. Uploads the 2x2 RGBA.
- `glGetUniformLocation(program,"uTex")` -> 0, `glUniform1i(uTex, 0)`.

The textured fragment shader (`precision mediump float;` is REQUIRED in GLSL ES 1.00
for a local `vec2` — without it Mesa errors "No precision specified ... for type
'vec2'") then samples the checkerboard. The engine's own wrapper draws it.

## Verification (runs/sh26-tex.txt, pixel analysis on runs/sh26-tex.rgb)

Three on-triangle quadrant probes read back three DIFFERENT colors — impossible for a
constant/solid shader, i.e. the sampled texture is definitively what rendered:

```
readback centroid(WHITE) @(640,360)   = RGBA(255,255,255,255)
readback quad-(1,0)(GREEN) @(900,150) = RGBA(0,255,0,255)
readback quad-(0,0)(RED)   @(300,150) = RGBA(255,0,0,255)
compile_status vs=0x1 fs=0x1 link_status=0x1 ; uTex loc=0x0<-unit0
geometry wrapper returned Ok(0x0) ; post-draw swap Ok(0x1) ; exit 124
```

Captured frame (1280x720): background blue (0,0,76)=505,728 px, plus the textured
triangle interior split into the 4 checkerboard colors RED=155,909 / GREEN=155,899 /
WHITE=52,001 / BLUE=51,947 (~415,756 px ≈ 45.1%, matching the SH25 footprint, now
4-colored). Reproducible: `runs/capture_tex.sh`; run-log `runs/sh26-tex.txt`.

## New debug aid

When a shader/program fails to compile, the harness now dumps its info log via the
int bridge (`glGetShaderInfoLog`/`glGetProgramInfoLog` resolve through
`resolve_gles_int`). This is what surfaced the missing `precision mediump float;`.

## Notes / honest scope

- Still harness-driven on a time base: the harness fabricates the coherent renderer +
  GL resources and drives the engine's own geometry wrapper; the engine's real
  main-loop producer still never enqueues a render task (long-standing structural
  wall). The texture here is harness-created via @plt (not bound through the engine's
  16-slot GLES dispatch table, whose slot 11+ mapping for texture is not yet
  disasm-pinned — left unseeded to avoid routing a real engine dispatch to a guessed
  function).
- `uTex` loc came back 0 fine here; a real engine will assign its own sampler
  locations, which the SAME `glGetUniformLocation`/`glUniform1i` bridge covers.
- Baselines unchanged: `--jni` exit 0 (JNI_OnLoad 0x10006); stable idle exit 124;
  untextured `--renderframe-triangle` still solid-red full triangle. Workspace 473/0.