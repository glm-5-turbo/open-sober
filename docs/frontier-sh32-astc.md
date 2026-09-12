# Frontier: SH32 — REAL ASTC texture through the engine's geometry wrapper

Scales the two-attrib textured-quad path (SH30) onto a REAL **ASTC** compressed
texture (`GL_COMPRESSED_RGBA8_ASTC_4x4 = 0x93B0`), completing the compressed-texture
live-path prove coverage: ETC1 (SH27) + ETC2-RGB (SH29) + ETC2-RGBA8/EAC (SH31) +
**ASTC 4x4 (SH32)**. `--renderframe-astc` uploads the quad's texture through
`glCompressedTexImage2D(0x93B0)`; the GLES bridge decompresses via `decode_astc` and
re-uploads as RGBA8.

## Why ASTC is the load-bearing format

Desktop OpenGL has **no native ASTC decode** (unlike ETC2, which Mesa can software-
decode). Per GRAPHICS_RECOMMENDATION §8.2, ASTC is the one Android format where our
interception is **required** rather than redundant: without the bridge decompressing
ASTC→RGBA8, an ASTC upload on a host whose desktop GL can't decode it would either
fail or upload garbage. This cycle proves that required path end-to-end.

## The crafted 8x8 ASTC texture — Khronos LDR void-extent

Each 16-byte block is an **ASTC LDR void-extent block** (the spec's solid-color
encoding), verified against Khronos `DataFormat/astc.txt` "Void-Extent Blocks":
- `buf[0] = 0xFC` (low 8 bits of the 9-bit block-mode "111111100"); `buf[1] = 0x01`
  (bit 8 set; **bit 9 = Dynamic-Range flag = 0 ⇒ LDR**).
- Block color components are **UNORM16 at bytes 8(R)/10(G)/12(B)/14(A)**. Setting the
  value as `<g> << 8`, the 8-bit channel = the high byte `buf[9]/[11]/[13]/[15]`
  (exact truncation). `texture2ddecoder`'s LDR path reads exactly those high bytes,
  so a solid color+alpha is fully determined by bytes 9/11/13/15.

The 4 blocks (row-major, 8x8) carry gray alphas **255 / 190 / 128 / 64** (and
R=G=B=alpha). The fragment shader maps the DECODED ALPHA to RGB gray-scale
(`vec4(t.aaa, 1.0)`), so the 4 quadrant readbacks read the 4 distinct ASTC block
alphas as gray lobes — robust because window framebuffers often discard alpha.

## Verification (runs/sh32-astc.txt, runs/sh32-astc.{rgb,png})

```
[elfjit:renderframe] uploading 8x8 ASTC 4x4 (0x93B0) LDR void-extent 4-block via glCompressedTexImage2D
[elfjit:renderframe-quad] readback: BL=RGBA(255,255,255,255) BR=RGBA(190,190,190,255) TR=RGBA(64,64,64,255) TL=RGBA(128,128,128,255)
[elfjit:renderframe-quad] post-draw swap returned Ok(0x1) ; exit 124
```

The 4 quadrant readbacks read the **4 exact ASTC void-extent alphas as gray levels**
(255/190/64/128). Pixel analysis: GRAY255 x186641 / GRAY128 x186587 / GRAY64 x186581
/ GRAY190 x186571 (4 near-equal quadrants) on the clear-blue `(0,0,0.3)` background.
Wrapper Ok(0x0), swap Ok(0x1), exit 124 stable, zero crash.

## New hermetic regression

`texture-codec::astc_ldr_void_extent_blocks_decode_expected_color_and_alpha` — the
same 64 bytes via `decompress(0x93B0, 8, 8)`: asserts each block decodes to a solid
RGB=alpha of 255/190/128/64, proving the ASTC enum + LDR void-extent decode match the
spec layout (UNORM16 at bytes 8/10/12/14, high-byte truncation) without the graphics
stack.

Reproducible: `runs/capture_astc.sh`. Workspace 476/0; baselines unchanged
(`--jni` exit 0; idle 124; quad/etc2a/etc/etc2/tex modes intact).

Honest framing: still harness-driven on a time base (fabricated renderer + GL
resources; the engine's own main-loop producer still never enqueues a render task).
The compressed-texture live-prove matrix is now complete for the formats Roblox uses
(ETC1/ETC2-RGB/ETC2-RGBA8-EAC/ASTC-4x4); EAC R/RG (0x9270-73) and ATC (0x8C92/93)
remain code-complete but not live-proven (lowest priority — ATC is legacy-Adreno-only
and EAC mono is rarely used by Roblox). Remaining graphics frontier: the engine's
main-loop producer enqueuing a render task so frames run natively (the long-standing
structural wall).