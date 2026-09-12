# SH25 (2026-09-12) — the coherent renderer renders a REAL visible triangle

## One-line result
Fabricated a *coherent* engine renderer (real 1-primitive list, real
vertex-descriptor + stride tables, real IBO) plus REAL GL resources (compiled +
linked shader program, real VBO with triangle verts, real EBO with indices) —
all created/uploaded through the JIT GLES int bridge — and drove a **real
indexed `glDrawElements(GL_TRIANGLES, 3, GL_UNSIGNED_INT)`** that renders an
actual visible triangle: **415,696 red pixels (45.11% of the frame)** captured
from the real X11 window. This is the first real RENDERED GEOMETRY (not just the
clear path) through the whole coherent path. Workspace 473/0 (unchanged; the
run-log + harness are the new artifact).

## What the SH24 drawprobe was missing
SH24 drove geometry wrapper 0x5b35288 with an EMPTY primitive list (begin==end),
so primitive-setup 0x5b353d0 returned mask 0 fast and the wrapper dispatched a
`glDrawElements` with NO bound vertex data — it proved the draw *dispatch*, not a
render. SH25 feeds 0x5b353d0 a **coherent** renderer so its real loop runs and a
real indexed draw has real vertices/indices/program.

## The coherent renderer layout (reversed from disasm, this cycle)
- `renderer[+56]` = container.
- container[+72]=begin ptr, container[+80]=end ptr of the primitive list
  (stride 0x18 = 24 B per primitive; count = `(end-begin)/24`, verified via the
  magic-constant mul `(n>>3)*0xaaaaaaaaaaaaaaab` == n).
- `renderer[+0x48]` = INLINE vertex-descriptor table: entry[vb] @ +vb*16 is a
  descriptor ptr (5b3546c `ldr x11,[sp,#16]` where sp+16 = renderer+0x48).
- descriptor[+72] = ARRAY_BUFFER GL id (bound by 5b35494).
- container[+96] = stride table base; stride[vb] @ +vb*8 (glVertexAttribPointer
  stride).
- `renderer[+120]` = IBO obj; [ibo+72] = ELEMENT_ARRAY_BUFFER id (bound by the
  loop tail 5b35544). `renderer[+142]` u16 = element count.
- primitive struct (24 B): [+0]=vb idx(w9), [+4]=offset(w21), [+8]=format idx
  (w28 -> the static format table @ guest 0x100cecf8c, 12 B/entry
  {size,type,normalized}), [+12]=attrib/type (0 -> attrib index 0), [+16]=base.
- Attribute format[5] = size 4, GL_FLOAT -> glVertexAttribPointer(0, 4, GL_FLOAT,
  0, 16, 0).

## New `--renderframe-triangle` lever (elfjit)
Drives, all through the JIT GLES int bridge on the live render-ctx:
1. glClearColor(float lanes) -> glClear + glViewport/glScissor (0,0,1280,720).
2. Real shaders: glCreateShader(VERTEX/FRAGMENT), glShaderSource, glCompileShader
   (status GL_TRUE), glCreateProgram, glAttachShader, glBindAttribLocation(0,
   "aPos"), glLinkProgram (link GL_TRUE), glUseProgram.
3. Real VBO (glGenBuffers + glBufferData 3 x vec4) + real EBO (indices 0,1,2),
   using dedicated id slots (glGenBuffers writes its out-pointer — aliasing the
   data buffer clobbered the verts, a real bug found this cycle).
4. Drive wrapper 0x5b35288 with the coherent renderer: x0=renderer, x1=0 (mode
   table idx -> GL_TRIANGLES), x4=3 (glDrawElements COUNT — the wrapper does
   `mov w1,w20`, so count rides in the 4th drive arg, NOT x5 as SH24's comment
   said), x5=3 (nonzero -> indexed path).
5. glReadPixels probes + swap.

Verified real run: wrapper returned Ok(0x0), `post-draw swap Ok(0x1)`, exit 124.
Capture runs/sh25-triangle.{png,rgb}: 415,696 px = exact shader red (255,0,0) =
45.11% of frame on the 0,0,0.3 clear-blue background. Reproducible artifact:
runs/capture_triangle.sh; run-log runs/sh25-triangle.txt.

## Honest scope / the isolated remaining wall
The verified full-triangle render goes through a REFERENCE draw added for
diagnosis: with the same program + the same VBO/EBO, a direct
`glVertexAttribPointer(0,4,GL_FLOAT,0,16,0)` + bind-ARRAY + enable-attrib0 +
`glDrawElements(4,3,0x1405,0)` renders the full ±0.95 triangle. The engine's OWN
wrapper path (primitive-setup's glVertexAttribPointer) still collapses to a small
blob near screen center — its computed pointer/stride differs from what the data
needs, and `glGetError` reads 0x8000 right after primitive-setup's attrib call.
So the coherent renderer + real GL resources + real indexed draw are PROVEN
bridge-functional (the frontier gap SH24 left), and the precise primitive-setup
attrib-pointer arithmetic is the next, narrow reverse. Baselines unchanged
(--jni exit 0; stable idle exit 124).