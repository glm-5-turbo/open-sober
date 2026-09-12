# SH19 (2026-09-12) — pinned the frame-fn 0x105b32c00 clear-path wall to the engine's
# raw-Mesa GLES dispatch table; added a `--renderframe-drive` slot dump. Workspace 470/0.

## One-line result
The engine's OWN frame-fn 0x105b32c00 clear path does not dispatch because the
8 GLES function-pointer slots (guest BSS 0x106d3b2f0 .. 0x106d3b328) it reads via
`br`-stubs hold **raw Mesa host addresses** (0x7fa7… = addr of the x86 Mesa
function), NOT our registered host-thunk slots (HOST_THUNK_BASE 0x7f00_0000_0000).
A guest `br` to a raw Mesa address is not a dispatchable host-call slot, so
`jit_run` treats it as a jump outside the guest image and stops
(`run_loop: pc 0x7fa7c2b2ea80 outside image`). This is the SH3/SH3b-class bug
(raw Mesa vs bridge slot) but for the engine's RUNTIME-built frame line-dispatch
table, which its real GL init was supposed to populate through the bridge.

## The 8 slots and the stubs that read them (v2.738.1397 disasm)
The frame-fn's clear-state sub-fn (0x5b32e08) calls indirect GL dispatchers by
`bl` to tiny `adrp x8, 6d3b000; ldr xN,[x8,#752 + 8*k]; br xN` stubs at
0x5b3a1c0 / 0x5b3a1cc / 0x5b3a1d8 / 0x5b3a1e4 / ... (0x10 stride). `adrp 6d3b000`
(page guest 0x106d3b000) plus byte offset 752.. maps to:
- slot k=0 guest 0x106d3b2f0 (br x2)  <- stub 0x5b3a1c0
- slot k=1 guest 0x106d3b2f8 (br x3)  <- stub 0x5b3a1cc
- slot k=2 guest 0x106d3b300 (br x3)  <- stub 0x5b3a1d8
- slot k=3 guest 0x106d3b308 (br x3)  <- stub 0x5b3a1e4
- slots 4..7: 0x310/0x318/0x320/0x328 (stubs 0x5b3a1f0..0x5b3a214)
Elfjit's new `--renderframe-drive` slot dump prints these 8 at drive time.

## Measured slot contents under a headless drive (reproducible)
```
[elfjit:renderframe-drive] gles-dispatch slot 0 guest 0x106d3b2f0 = 0x7fa7c2b2ea80
... (all 8 are 0x7fa7…, i.e. HOST addresses, not 0x7f0000000000 host-thunk)
[elfjit:renderframe-drive] frame-fn stopped: run_loop: pc 0x7fa7c2b2ea80 outside image
```
Important: an earlier wrong read (locations 0x1067d12f0) was a diagnostic bug
(the stack-canary region, DER-cert bytes) — the CORRECT slots are the
0x106d3b2f0 family loaded by `adrp 6d3b000` (the disasm literal). Fixed in SH19.

## Why this is the same bridging bug as SH3/SH3b, now for the engine's own table
SH3 routed `eglGetProcAddress` and SH3b routed `GLOB_DAT` *function* slots through
the host-thunk GLES bridge so a guest `blr`/`br` to a returned pointer dispatches
through our bridge (float bridge + compressed-texture interception). The frame-fn's
dispatch table is the engine's OWN line/clear GL table (equivalent of the functions
the clear sub-fn needs: glClearColor / glClear / glColorMask / glDepthMask /
glStencilMask / viewport state) and it was populated at render-init/GL-init with
RAW Mesa addresses — a path that did not go through our bridge. When the guest
`br`s to that raw address, jit_run cannot dispatch it (it only treats
HOST_THUNK_BASE-range slots as host calls) and aborts out-of-image.

## Next lever (now precise)
Seed the 8 BSS slots (guest 0x106d3b2f0..0x106d3b328) with host-thunk GLES bridge
slots (the `resolve_gles_mixed(name)` slots for glClearColor/glClear/
glColorMask/glDepthMask/glStencilMask/glClearDepthf/glClearStencil) so the frame's
clear path dispatches through the bridge; or reproduce the engine's GL-init writes
that fill them (the multi-week renderer-object reverse). Baselines unchanged:
`--jni` exit 0; stable idle startapp exit 124; workspace 470/0. Run-log:
/home/hermes-worker/runs/sh19-slotdump2.txt; /home/hermes-worker/runs/sh19-slotdump.txt
(first, wrong-address version).