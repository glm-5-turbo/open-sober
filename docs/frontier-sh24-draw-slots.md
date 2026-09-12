# SH24 (2026-09-12) — complete the GLES dispatch-slot map to the geometry draw path

## One-line result
Reversed the engine's real GEOMETRY draw path and completed the 16-slot GLES
dispatch table that --renderframe-seedgles covers. The clear path uses slots
0-7 (SH22 map); the real draw path uses slots 9/10 = glDrawElements /
glDrawArrays via wrapper 0x5b35288. These were left UNSEEDED (raw-Mesa, SH19
bug class). Now seeded + regression-pinned, and a new --renderframe-drawprobe
lever PROVES the engine's own geometry wrapper dispatches glDrawElements
through the bridge. Workspace 473/0.

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

## --renderframe-drawprobe (real-geometry proof)
In addition to completing the map (above) we now DRIVE the engine's own
geometry wrapper 0x5b35288 with a fabricated minimal renderer (empty primitive
list -> primitive-setup 0x5b353d0 returns fast; nonzero [renderer+120]
index-buffer obj + w5=3 count -> INDEXED path). With slots 9/10 seeded, the
wrapper dispatches a REAL glDrawElements through the bridge:

```
[elfjit:renderframe-seedgles] slot 9 (glDrawElements)  <- bridge 0x7f0000002a38
[elfjit:renderframe-seedgles] slot 10 (glDrawArrays)   <- bridge 0x7f0000002a40
hostcall@glBindBuffer pc=0x7f00000029f8 x0=0x8893 ...     x30=0x105b35550 (GL_ELEMENT_ARRAY_BUFFER)
hostcall@glDrawElements pc=0x7f0000002a38 x0=0x4 x1=0x0 x2=0x1405 x30=0x105b352f8  (mode=GL_TRIANGLES, type=GL_UNSIGNED_INT)
[elfjit:renderframe-drawprobe] geometry wrapper 0x5b35288 returned Ok(0x0)
[elfjit:renderframe-drawprobe] post-draw swap returned Ok(0x1)
```

exit 124 stable, zero crash/heap abort. This is the FIRST time the engine's
real geometry draw path (primitive-setup -> buffer-bind -> indexed
glDrawElements) dispatches through the GLES bridge — analogous to SH17/22's
proof for the clear path. Run-log: runs/sh24-drawprobe.txt.

Honest scope: the geometry wrapper ran with a fabricated EMPTY renderer (no
real mesh/buffer/VAO data), so this dispatches a glDrawElements with
mode=GL_TRIANGLES / count routed but no actual vertices — it proves the DRAW
DISPATCH is bridge-functional, not that real geometry renders yet. The next
wall is feeding this primitive-setup path a coherent primitive list + vertex
buffers (the RENDERER C++ object reverse).

## Verify
- Workspace 473/0 (was 472; +1 regression).
- `--jni` clean exit 0; stable idle exit 124 (baselines unchanged).
- Clear/sustain render path unchanged (slots 0-7 seeded as before).
- New repr: `--renderframe-drawprobe` reaches glDrawElements through bridge.

## Frontier (unchanged shape, now two walls shorter)
The draw dispatch slots route through the bridge AND the engine's own geometry
wrapper demonstrably reaches glDrawElements through it. The ONLY remaining
blocker to a real rendered triangle is the coherent renderer C++ object that
wrapper 0x5b35288 / primitive-setup 0x5b353d0 take as x0 — its primitive-list
/ vertex-buffer / VAO layout (the multi-cycle frontier). Slots 11+ (texture /
uniform / shader dispatch) still need reversing before a textured/shaded draw.