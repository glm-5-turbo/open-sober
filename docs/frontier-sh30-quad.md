# Frontier: SH30 — REAL TWO-ATTRIB TEXTURED QUAD through the engine's geometry wrapper

Scales the (fully reversed) coherent renderer onto a real **two-vertex-attribute
(pos + UV) textured quad** — the shape of real Roblox geometry. `--renderframe-quad`
fabricates a coherent renderer whose primitive SETUP 0x5b353d0 now runs its
multi-primitive loop (one vertex attrib per primitive) to set up BOTH `aPos` and
`aUV`, and the fragment shader samples a 2x2 checkerboard at the REAL interpolated
vertex UV (not `gl_FragCoord`).

## What primitive-setup does (from disasm, so the fabrication is exact)

`0x5b353d0` iterates the container's primitive list (`[container+72]..[+80]`,
stride 0x18), and for EACH primitive binds the array buffer, enables ONE vertex
attrib and calls glVertexAttribPointer:

- attrib location w22 = primitive[+12] (the GL attrib slot it maps to; 0/1 pass
  through, 2/3 offset the base — for a mesh's first two attribs just 0 and 1).
- vb idx = primitive[+0]; byte offset = primitive[+4]; format idx = primitive[+8].
- format table @ 0x10cecf8c: each 12B entry {size i32, type i32, normalized u8}.
  `format[3]={4,GL_FLOAT}` (aPos) and `format[1]={2,GL_FLOAT}` (aUV).
- glVertexAttribPointer(loc, size, type, norm, stride=container stride_tbl[vb]=24,
  offset=stride*mult + primitive[+4]).

So a two-attrib vertex is TWO primitives sharing one VBO:
```
prim0 = { vb:0, off:0,   fmt:3({4,GL_FLOAT}), attrib:0 }   aPos
prim1 = { vb:0, off:16,  fmt:1({2,GL_FLOAT}), attrib:1 }   aUV
```
VBO = `[x,y,z,w, u,v]` x4 vertices (stride 24), EBO = 6 u32 indices (2 triangles).

## Verification (runs/sh30-quad.txt, runs/sh30-quad.{rgb,png})

```
program compiled+linked (aPos->0, aUV->1, aUV varying); texture 2x2 RED/GREEN/BLUE/WHITE
geometry wrapper 0x5b35288 returned Ok(0x0)
readback: BL(320,180)=RED(255,0,0) BR(960,180)=GREEN(0,255,0)
          TR(960,540)=WHITE(255,255,255) TL(320,540)=BLUE(0,0,255)
post-draw swap returned Ok(0x1) ; exit 124
```

The 4 quadrants read the 4 texel colors at the REAL interpolated vertex UVs
(v0 uv=(0,0) blue?  no — v0 uv=(0,0) bottoms-left RED, v1 (1,0) GREEN bottom-right,
v2 (1,1) WHITE top-right, v3 (0,1) BLUE top-left). Pixel analysis: WHITE 186,641 /
BLUE 186,587 / RED 186,581 / GREEN 186,571 (+ clear-blue background outside the
quad's ±0.9 corners). Reproducible runs/capture_quad.sh.

Honest framing: still harness-driven on a time base (fabricated renderer + GL
resources, the engine's real main-loop producer still never enqueues a render task).
But this proves the engine's OWN primitive-setup multi-attrib loop sets up two real
vertex attribs and dispatches a per-texel UV-mapped textured draw — the geometry
shape every real Roblox mesh needs. Baselines unchanged (--jni exit 0; idle 124;
triangle/tex/etc modes intact); workspace 474/0.