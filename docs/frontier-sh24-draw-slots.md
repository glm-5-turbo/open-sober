# SH24 (2026-09-12) — complete the GLES dispatch-slot map to the geometry draw path

## One-line result
Reversed the engine's real GEOMETRY draw path and completed the 16-slot GLES
dispatch table that --renderframe-seedgles covers. The clear path uses slots
0-7 (SH22 map); the real draw path uses slots 9/10 = glDrawElements /
glDrawArrays via wrapper 0x5b35288. These were left UNSEEDED (raw-Mesa, SH19
bug class). Now seeded + regression-pinned. Workspace 473/0.

## What the clear-only frame was hiding
The frame-fn 0x105b32c00 (driven by --rendersustain / --renderframe-drive) is a
CLEAR-ONLY function: it dispatches the per-buffer clear path (slots 0-7) and
returns at 0x5b32dcc — it never touches geometry. The engine's actual geometry
draw is a SEPARATE function pair in the same render pass:

- wrapper 0x5b35288: `(renderer=x0, w1, w2, w3, w4, w5)` — calls the
  primitive-setup fn then dispatches the indexed (slot 9, glDrawElements) or
  array (slot 10, glDrawArrays) draw, each through a GLES dispatch-table stub.
- primitive-setup 0x5b353d0: iterates the renderer's primitive list, binds
  vertex buffers (glBindBuffer GL_ARRAY_BUFFER), enables vertex attrib arrays
  and sets glVertexAttribPointer directly via @plt — then the wrapper draws.

So a real geometry draw needs BOTH the coherent renderer C++ object (the
multi-cycle frontier) AND the draw dispatch slots 9/10 to route through the
bridge. SH19/SH22 proved the clear slots 0-7 were raw-Mesa and needed seeding;
the draw slots 9/10 have the exact same bug — an unseeded slot makes the driven
draw `br` out-of-image.

## Full 16-slot GLES dispatch table (BSS 0x106d3b2f0, stub 0x5b3a1c0+0xc*N : `adrp x8,6d3b000;ldr xK,[x8,#752+8*N];br xK`)
```
slot 0  glDrawBuffers           (clear preamble: GL_COLOR_ATTACHMENT0..3 / GL_BACK)
slot 1  glClearBufferiv         (GL_STENCIL=0x1802)
slot 2  glClearBufferfv         (GL_COLOR=0x1800 / GL_DEPTH=0x1801 per-buffer loop)
slot 3  glClearStencil
slot 4  glColorMask
slot 5  glDepthMask
slot 6  glStencilMask
slot 7  glViewport
slot 8  (unused by wrapper; vertex/pixel store)
slot 9  glDrawElements          (wrapper 0x5b352f4 bl 0x5b3a22c — indexed draw)
slot 10 glDrawArrays            (wrapper 0x5b35368 bl 0x5b3a238 — array draw)
slot 11+ texture / uniform / shader dispatch (unreached by clear; see below)
```
Verified from disassembly: stubs at 0x5b3a1c0..0x5b3a274 each `ldr xK,[x8,#752+8*N]`,
N = slot index. Wrapper 0x5b35288 uses slot 9 (0x5b352f4 bl 0x5b3a22c) and
slot 10 (0x5b35368 bl 0x5b3a238).

## Fix
--renderframe-seedgles `seed_names` extended from 8 to 10 entries: slots 8/9
(now also glDrawElements + glDrawArrays) resolve through the integer GLES
bridge (both are pure int-ABI, <=8 args). New regression
`draw_slots_gl_draw_elements_arrays_resolve_via_int_bridge` pins both resolve
with a trailing NUL (the seedgles calling convention) and rejects mixed-wrapping.

## Verify
- Workspace 473/0 (was 472; +1 regression).
- `--jni` clean exit 0; stable idle exit 124 (baselines unchanged).
- Clear/sustain render path unchanged (slots 0-7 seeded as before).

## Frontier (unchanged shape, now one wall shorter)
The draw dispatch slots route through the bridge, so the ONLY remaining blocker
to a real geometry draw is the coherent renderer C++ object that wrapper
0x5b35288 / primitive-setup 0x5b353d0 take as x0 — its primitive-list /
vertex-buffer / VAO layout (the multi-cycle frontier). Slots 11+ (texture /
uniform / shader dispatch) still need reversing before a textured/shaded draw.