# Frontier: SH31 — REAL ETC2-RGBA8/EAC texture through the engine's geometry wrapper

Scales the two-attrib textured-quad path (SH30) onto a REAL **ETC2-RGBA8/EAC**
compressed texture (`GL_COMPRESSED_RGBA8_ETC2_EAC = 0x9278` — the real Android
RGBA-EAC format), completing the compressed-texture live-path prove coverage:
ETC1 (SH27) + ETC2-RGB (SH29) + **ETC2-RGBA8/EAC (SH31)**. `--renderframe-etc2a`
uploads the quad's texture through `glCompressedTexImage2D(0x9278)`, the GLES
bridge decompresses it via `decode_etc2_rgba8`, and re-uploads as RGBA8.

## Why ETC2-RGBA8 specifically

- Its 16-byte block = an **8-byte EAC alpha sub-block + an 8-byte ETC2-RGB
  sub-block**. The RGB half is `decode_etc2_rgb_block` (identical to the SH29
  proven 0x9274 path), but the **EAC alpha half is brand-new code** no prior
  cycle rendered live. Proving a distinct *alpha* per texel renders is the only
  way to confirm the EAC sub-block executes (the RGB-only 0x9274 path cannot
  prove it).
- Alpha is the format's whole point (Roblox alpha-masked/ui textures), and is
  the channel a `decode_etc2_rgb`-only shortcut would silently drop.

## The crafted 8x8 EAC texture

4 x 16-byte blocks, row-major top-first. Each block = `[A,0x00, ..6 pad,
 ETC2-RGB-block]`:
- EAC alpha sub-block: `data[0]=A`, `data[1]=0x00` → `decode_etc2_a8_block`'
  multiplier==0 path (`data[1] & 0xf0 == 0`) → **every texel alpha = A**.
- RGB sub-block: the SH29-proven ETC2 colors (modes 1/2, bit-identical to ETC1):
  red `(255,2,2)`, green `(2,255,2)`, blue `(2,2,255)`, white.

Alphas per block = **255 / 190 / 128 / 64** (plainly distinct, wide range).

## Fragment shader maps alpha -> RGB

Window framebuffers (EGL/llvmpipe window surfaces) often have **no alpha
channel**, which would zero-out a naive alpha proof in readback/capture. So the
etc2a FS outputs `vec4(t.aaa, 1.0)` — the DECODED ALPHA becomes the RGB gray
level. The 4 block alphas render as 4 distinct gray lobes, robustly
window-capturable.

## Verification (runs/sh31-etc2a.txt, runs/sh31-etc2a.{rgb,png})

```
[elfjit:renderframe-etc2a] uploading 8x8 ETC2-RGBA8 (0x9278) 4-block EAC via glCompressedTexImage2D
[elfjit:renderframe-quad] readback: BL=RGBA(255,255,255,255) BR=RGBA(190,190,190,255) TR=RGBA(64,64,64,255) TL=RGBA(128,128,128,255)
[elfjit:renderframe-quad] post-draw swap returned Ok(0x1) ; exit 124
```

The 4 quadrant readbacks read the **4 exact EAC block alphas as gray levels**
(255/190/64/128) — each probe maps (via NEAREST on the interpolated quad UV) to a
different block, and each reads its distinct decoded alpha. Pixel analysis:
GRAY255 x186641 / GRAY128 x186587 / GRAY64 x186581 / GRAY190 x186571 (4
near-equal quadrants) on the clear-blue `(0,0,0.3)` background. Wrapper Ok(0x0),
swap Ok(0x1), exit 124 stable, zero crash.

## New hermetic regression

`texture-codec::etc2_rgba8_eac_solid_blocks_decode_expected_alpha` — the same 64
bytes, decoded via `decompress(0x9278, 8, 8)`: asserts each block's RGB matches
the SH29 prediction AND its alpha byte matches the per-block EAC value (255/190/
128/64), and that in-block texels are uniform. Pins the EAC-alpha half of
`decode_etc2_rgba8` without the graphics stack.

Reproducible: `runs/capture_etc2a.sh`. Workspace 475/0; baselines unchanged
(`--jni` exit 0; idle 124; quad/tex/etc/etc2 modes intact).

Honest framing: still harness-driven on a time base (fabricated renderer + GL
resources; the engine's own main-loop producer still never enqueues a render
task). The last Android compressed formats still code-complete but not live-proven
are **ASTC (0x93B0+)**, **EAC R/RG (0x9270-73)** and **ATC (0x8C92/93)** — ASTC is
the load-bearing one (desktop GL has no native ASTC decode; our interception is
required there).