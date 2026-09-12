# Frontier: SH28 — the real engine GLES dispatch-table content, captured live

`--renderframe-seedgles` now dumps the 16 raw GLES dispatch-slot values
(`BSS 0x106d3b2f0 + 8*N`) that the ENGINE's own GL-init left in the table, and
`dladdr`-resolves each host address to a symbol, BEFORE we overwrite them. This
pins the real slot->function map without code archaeology.

## Why it works — and the 0x7f00000000 slots

Slots 0-2 read as `0x7f0000003010/18/20` = OUR host-thunk bridge slots. That is not
an artifact: SH3 routed `eglGetProcAddress` to return host-thunk slots for the
names in the GLES bridge whitelist, so when the engine's real GL-init resolved
glDrawBuffers / glClearBufferiv / glClearBufferfv through `eglGetProcAddress` it
stored OUR bridge slots into its dispatch table. The names NOT in the whitelist
resolved to real Mesa, which is why 3-15 are raw Mesa addresses with clean
`dladdr` names. Confirms SH3's interception reaches the engine's own table.

## The engine's REAL dispatch-table content (live, resolved)

```
slot  0 = 0x7f0000003010 (bridge)  glDrawBuffers          (SH22 name correct)
slot  1 = 0x7f0000003018 (bridge)  glClearBufferiv        (SH22 name correct)
slot  2 = 0x7f0000003020 (bridge)  glClearBufferfv        (SH22 name correct)
slot  3 = Mesa  glClearBufferfi
slot  4 = Mesa  glUniformBlockBinding
slot  5 = Mesa  glBindBufferBase
slot  6 = Mesa  glBindBufferRange
slot  7 = Mesa  glGetUniformBlockIndex
slot  8 = Mesa  glGetActiveUniformBlockiv
slot  9 = Mesa  glDrawElementsInstanced
slot 10 = Mesa  glDrawArraysInstanced
slot 11 = 0                      (unfilled at this point)
slot 12 = 0                      (unfilled)
slot 13 = Mesa  glGetProgramBinary
slot 14 = Mesa  glProgramBinary
slot 15 = Mesa  glProgramParameteri
```

## Corrects the mental model

The real engine renderer's GLES dispatch is a MODERN GLES3 pipeline: uniform
buffer objects (glUniformBlockBinding/glBindBufferBase/glBindBufferRange),
instanced draws (glDrawElementsInstanced/glDrawArraysInstanced), program binaries
(glGetProgramBinary/glProgramBinary/glProgramParameteri), and block introspection
(glGetUniformBlockIndex/glGetActiveUniformBlockiv). This is substantially richer
than the simple clear/draw map the harness drives (slots 0-2 = clear path, 9/10 =
indexed/array draw). When the engine-self-driven render eventually dispatches
through these slots, each must resolve through a bridge too — but those functions
are still being called via real Mesa here (valid; host Mesa is the ABI target).

The harness does NOT re-seed based on this snapshot: its driven wrappers (SH22-27)
dispatch through the slots the harness itself seeds (0-7 clear, 9/draw + 10/array)
and render correctly. Do not overwrite a working harness seed with the engine's
raw table. This snapshot is diagnostic context for the future engine-driven render.

## Note

Some slots are shared across code paths and may hold different functions at
different times (e.g. slot 9 was glDrawElementsInstanced here vs the SH24 draw
wrapper dispatching glDrawElements). Treat this as one authoritative point-in-time
snapshot of the engine's GL init, not an exclusive mapping.

Reproducible: any `--renderframe-seedgles` run prints the snapshot before seeding
(e.g. runs/sh28-slotsnap.txt). Baselines unchanged.