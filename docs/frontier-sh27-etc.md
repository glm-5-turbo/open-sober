# Frontier: SH27 — REAL compressed-ETC1 texture renders through the engine's geometry wrapper

Adds to SH26's textured triangle a `--renderframe-etc` mode: the same coherent
geometry draw path (wrapper 0x5b35288 -> primitive-setup -> indexed glDrawElements)
now renders a REAL COMPRESSED ETC1 texture. A hand-crafted 8x8 ETC1 texture (4
solid 4x4 blocks, individual mode, table codeword 0, all selectors 0) is uploaded
via `glCompressedTexImage2D(GL_ETC1_RGB8_OES=0x8d64)`; the GLES bridge intercepts
it (the mixed/float `w_glCompressedTexImage2D`), decompresses ETC1->RGBA with
`texture-codec`, and re-uploads via a real `glTexImage2D`. The textured fragment
shader samples it; three on-triangle probes read back the distinct decoded colors.

## The crafted ETC1 block

Every block is `[R,G,B, 0,0,0,0,0]`: individual mode (diff bit clear), table
codeword 0 -> modifier +2 (ETC1_MODIFIER_TABLE[0][0]), all selectors 0. Decode
splits each color byte into two 4-bit sub-block bases replicated as `c*0x11` and
adds +2, clamped: decoded channel = `(c*0x11)+2`. `c=(t-2)>>4` reproduces the
target: `enc(255)=0xff` -> 255, `enc(2)=0x00` -> 2. Colors chosen all distinct
with one channel 255: red `(255,2,2)`, green `(2,255,2)`, blue `(2,2,255)`,
white `(255,255,255)`.

## Verification (runs/sh27-etc.txt + pixel analysis on runs/sh27-etc.rgb)

```
texture tex_id=0x1 bound+uploaded uTex loc=0x0<-unit0
compile_status vs=0x1 fs=0x1 link_status=0x1
readback centroid(WHITE)   @(640,360) = RGBA(255,255,255,255)   <- decoded ETC1 white
readback quad-(1,0)(GREEN) @(900,150) = RGBA(2,255,2,255)       <- decoded ETC1 green
readback quad-(0,0)(RED)   @(300,150) = RGBA(255,2,2,255)       <- decoded ETC1 red
geometry wrapper Ok(0x0) ; post-draw swap Ok(0x1) ; exit 124
```

Captured frame: clear-blue bg + the triangle interior 4-colored from the decoded
ETC1 blocks: RED=155,909 / GREEN=155,899 / WHITE=52,001 / BLUE=51,947 (~45.1% of
frame, the SH25 footprint now compressed-textured). The rounded decoded channels
exactly match the engineered `(c*0x11)+2` prediction, which confirms the ETC1
decode ran (not Mesa falling through or a passthrough).

## New regression test

`crafted_etc1_solid_blocks_decode_to_expected_colors` (texture-codec) pins: a 4x4
red block decodes to `(255,2,2)` for all 16 texels, and the full 8x8 (4 blocks
row-major top-first) decodes to the red/green/blue/white quadrant pattern — the
exact blob the harness uploads. Workspace 474/0 (was 473).

## Notes / honest scope

- Proves the compressed-texture INTERCEPTION live path for the ETC1/ETC2 family
  (same `handle_compressed_tex_image_2d` decompress+re-upload handles ETC1/ETC2/
  EAC/ASTC via texture2ddecoder — ETC1 confirms the mechanism; ETC2/ASTC decode
  correctness is covered by texture2ddecoder's own suite, our decompress routes
  through it).
- Harness-driven on a time base (fabricated coherent renderer + GL resources, the
  engine wrapper driven directly); the engine's main-loop producer still never
  enqueues a render task (long-standing structural wall). Texture uploaded via
  @plt through the bridge, not the engine's 16-slot dispatch table (slot 11+ map
  for texture still not disasm-pinned; left unseeded to avoid routing a real
  engine dispatch to a guessed function).
- Four new harness PLT stubs pinned (guest = file+0x100000000): glActiveTexture
  0x1062d75e0, glBindTexture 0x1062d75f0, glGetUniformLocation 0x1062d7900,
  glUniform1i 0x1062d7910, glTexParameteri 0x1062d7960, glGenTextures 0x1062d7980,
  glTexImage2D 0x1062d79a0, glCompressedTexImage2D 0x1062d7990.
- Baselines unchanged: --jni exit 0 (JNI_OnLoad 0x10006); stable idle exit 124;
  untextured triangle still solid-red; --renderframe-tex still works. Reproducible:
  runs/capture_etc.sh.