# Frontier: SH29 — REAL ETC2 texture renders through the engine's geometry wrapper

Extends SH27's compressed-texture prove from ETC1 to **ETC2** — the actual format
Android Roblox textures use (`GL_COMPRESSED_RGB8_ETC2`=0x9274). `--renderframe-etc2`
is `--renderframe-etc` but relays the same hand-crafted 8x8 four-block texture with
internalformat 0x9274. Because ETC2 RGB modes 1/2 are bit-identical to ETC1
individual/differential, the same blocks are valid ETC2; the bridge dispatches them
through `decode_etc2_rgb` and re-uploads. The textured fragment shader samples it,
and the three on-triangle probes read back the same distinct decoded colors:

```
readback centroid(WHITE)   @(640,360) = RGBA(255,255,255,255)
readback quad-(1,0)(GREEN) @(900,150) = RGBA(2,255,2,255)
readback quad-(0,0)(RED)   @(300,150) = RGBA(255,2,2,255)
compile_status vs=1 fs=1 link=1 ; geometry wrapper Ok(0x0) ; swap Ok(0x1) ; exit 124
```

Captured runs/sh29-etc2.{rgb,png}: clear-blue bg + triangle interior 4-colored from
decoded ETC2 (RED 155,909 / GREEN 155,899 / WHITE 52,001 / BLUE 51,947 — pixel
identical to the ETC1 capture, as expected for bit-identical modes).

## New regression additions

`crafted_etc1_solid_blocks_decode_to_expected_colors` (texture-codec) now also
decompresses the same blob as `GL_COMPRESSED_RGB8_ETC2` and asserts the four
quadrant colors match, and that the ETC2 pixel buffer equals the ETC1 one.

## Reproducible

`runs/capture_etc2.sh`; run-log runs/sh29-etc2.txt. `--renderframe-etc` (ETC1) and
`--renderframe-tex` (RGBA) unchanged. Baselines unchanged (--jni exit 0; idle 124).
Workspace 474/0.