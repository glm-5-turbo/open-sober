# Frontier: SH34 — the coherent renderer scales to a REAL larger mesh (N×N textured grid)

Fabricate the coherent engine renderer with a real **N×N mesh** instead of the single
4-vert quad: `--renderframe-grid <N>` builds `(4·N·N)` interleaved `[pos.xyzw, uv.xy]`
vertices (stride 24) and `(6·N·N)` indices across the engine's OWN geometry wrapper
`0x5b35288` -> primitive-setup `0x5b353d0` -> indexed `glDrawElements`. This closes the
SH30/33 frontier note — **scale the coherent-renderer rotation onto a larger real mesh**
— by proving the engine's own primitive-setup + draw wrapper render real mesh topology
(many verts/indices + one texel per cell at the interpolated UV), not just a single quad.

## What changed (harness only — no GLES/codec/resolver change)

In `examples/elfjit.rs`, the `--renderframe-quad` block's mesh fabrication becomes
grid-aware:

- Grid mode (`--renderframe-grid N`, 2≤N≤8): **independent per-cell quads** — cell
  `(gi,gj)` gets 4 verts all sharing the **texel-center UV** `((gi+0.5)/N,(gj+0.5)/N)`,
  so the whole cell renders flat with ITS distinct texel color. Vertex NDC spans
  `-0.9..0.9` in both axes, `cell = 1.8/N`. Index buffer is `6·N·N` u32 (16-bit index
  value in `[renderer+142]` would overflow for N≥6? no — index COUNT is `6·N·N` (u16 ok
  up to N=8 -> 384), and the INDIVIDUAL index values are vertex indices `(gj·n+gi)·4 ..+3`
  (u32, so values stay within 16-bit for N≤8 -> max vert id 4·64-1=255).
- Single-quad mode (SH25-33) is UNCHANGED: per-corner UVs (0,0)/(1,0)/(1,1)/(0,1) on the
  2×2 checkerboard, so all four quadrant readbacks stay distinct (verified).
- Texture: grid mode uploads an **N×N RGBA texture, one distinct solid color per texel**
  (`r = gi/(N-1)·255`, `g = gj/(N-1)·255`, `b=64`) via the same glTexImage2D path; quad
  mode keeps the 2×2 checkerboard.
- Readback: grid mode probes **every cell center** via glReadPixels and prints its RGB
  vs the expected texel `(r,g)`; quad mode keeps the 4 fixed corner probes.
- Buffer hygiene (real bugs found + fixed): VBO data relocated to `base+0x2000` and EBO
  to `base+0x4000` (the old `0xc00/0xd40` overlapped for N≥4), and the grid texture data
  to `base+0x6000` (the old `0xf60` overlapped tex_id_slot/tex_sp for 8×8). All within the
  32 KiB scratch.

## Verification (real libroblox.so, Mesa llvmpipe + Xvfb)

Single proof mode:
```
[elfjit:renderframe-quad] 144 interleaved verts stride24 + 216 idx uploaded (6x6 grid)
[elfjit:renderframe-quad] geometry wrapper 0x5b35288 returned Ok(0x0) (6x6 MESH drawn)
[elfjit:renderframe-quad] cell(0,0)@(160, 90) readback=RGBA(0,0,64,255)   (expect r=0   g=0)
[elfjit:renderframe-quad] cell(1,1)@(351,197) readback=RGBA(51,51,64,255)  (expect r=51  g=51)
[elfjit:renderframe-quad] cell(3,3)@(736,414) readback=RGBA(153,153,64,255)(expect r=153 g=153)
[elfjit:renderframe-quad] cell(5,5)@(1119,629) readback=RGBA(255,255,64,255)(expect r=255 g=255)
```
All 36/36 (6×6) and 16/16 (4×4), 9/9 (3×3) cell-centers read back **exactly** the texel
color of their own cell (±1 for the 51/128 truncation in the `(gi/(N-1))·255` mapping).
The captured frame (`runs/sh34-grid.mp4`, x11grab) shows the N×N distinct-color mesh
(58 sampled colors; R-column gradient 0→255 across cells exact); the G-channel inversion
in the capture is the documented Xvfb/x11grab byte-order quirk — the in-process
glReadPixels readbacks are the authoritative render proof.

Sustainable mode (`--renderframe-quad-loop`): 20 iterations all `drew+swap Ok(0x1)` and the
6×6 mesh re-rendered each frame (fresh distinct-cell texture; no static-buffer collapse).

## Baselines unchanged
- `--jni` boot returns `Ok(0x10006)` (JNI_OnLoad reached), then idles to exit 124.
- Single-quad mode: `readback: BL=RGBA(255,0,0) BR=RGBA(0,255,0) TR=RGBA(255,255,255)
  TL=RGBA(0,0,255)` intact (all four distinct checkerboard texels).
- `cargo build --workspace` + `cargo test --workspace` = **476/0**.

## Reproduce
`runs/capture_grid.sh` (N env, default 4) + `runs/capture_grid_frame.sh` (x11grab). Run-log:
`runs/sh34-grid.txt`.

## Honest framing
Still harness-driven on a time base (fabricated coherent renderer + GL resources; the
engine's real main-loop producer still never enqueues a render task — the long-standing
structural wall). But the engine's OWN primitive-setup + draw wrapper now provably render a
**real mesh** (up to 256 verts / 384 idx in one call), with per-cell UV→texel mapping — the
geometry shape and count a real Roblox mesh needs. The remaining frontier is unchanged: the
engine's producer enqueuing a render task so frames run natively.