# SH14 (2026-09-12) — deque-injection path is STRUCTURALLY capped; located the real render-init (0x105b3a2d8) and proved it's framework-gated. New tool: JIT_REGION_WATCH.

## One-line result
13 cycles of deque-injection (SH5–SH13) cannot reach egl*/gl* — not because of
a wrong node, but because the drain's pop-loop **hardcodes w4=4** (task type)
and the type-4 "maintenance" handler (0x10285371c) can only dispatch through
runtime-built BSS globals the Android framework must populate (all 0 on this
box). In parallel I located the engine's REAL render-init function —
**0x105b3a2d8** (thunk 0x105b3a280) which calls the full
eglGetDisplay→eglInitialize→eglCreateContext→**eglCreateWindowSurface**→eglMakeCurrent
sequence — and confirmed via a new `JIT_REGION_WATCH` diagnostic that the boot
**never reaches it** (framework-gated upstream). The deque lever is a documented
dead-end; the render lever is real but behind the same framework producer wall.

## Why the deque path is structurally capped (disassembly-proven)
The drain pop-loop (0x2856f94 → dispatch at 0x2857008) calls the node vtable's
handler with the fixed ABI `[vt+40]([vt+16], consumer, [node+32]&~1, node, **w4=4**, 0)`.
`w4=4` is a constant baked into the pop-loop — no injected node content can make
the dispatcher choose a different task type, because the dispatch is
`cmp w4,#1..4` (0x102853748..768) on the drain-supplied constant, NOT on node data.
The type-4 handler 0x10285371c then reads the working object from
`[consumer+104]→[+96]` (x22) and `blr`s through BSS dispatch globals at
**0x1068262e8 / 0x106826300 / 0x106826308** (adrp→ldr→br at 0x102853784/85c/8a4/9cc)
which I confirmed statically are **all 0** in the ELF (they're populated at
runtime by the Android framework's producer path). So:
- SH13b already showed arg2 sweep can't help (render isn't a linear arg2 index).
- SH14 now shows even the whole deque-vtable-substitution approach can't help:
  the type is baked to 4 and the maintenance handler's forward edge is a
  framework-owned BSS function pointer.
**Conclusion for future cycles: stop investing in the deque-injection dispatch
path. Documented dead-end.**

## The real render-init (the actual frame path)
Walking backward from the egl GOT slots (readelf + adrp→ldr→blr static scan):
- eglGetDisplay ← bl at 0x105b3a3b0 (inside fn 0x105b3a2d8)
- eglInitialize ← 0x105b3a3c4, eglCreateContext ← 0x105b3a400,
  eglMakeCurrent ← 0x105b3a480/0x105b3b2c0/..., eglChooseConfig ← 0x105b3b6f0,
  eglCreateWindowSurface ← **0x105b3b1a0**
- eglGetProcAddress is used dynamically (70 call sites in 0x105b3a68c..) — the
  engine resolves most gl* through it, so the resolver's eglGetProcAddress
  bridge must stay correct.
- fn 0x105b3a2d8 (prologue: sub sp,#96; stp x29,x30,...) is entered via thunk
  0x105b3a280, which is called from 0x105b2b214 and 0x105b2ea98.
**But it reads a runtime-built context global: fn 0x105b3a2d8 starts with
`ldr x25,[x25,#222*8]` where the adrp base is 0x1067d1000 → global 0x1067d16f0,
statically 0.** So the render-init needs a framework-constructed device/context
obj before it can even query EGL — same producer gate as everything else.

## New diagnostic tool: JIT_REGION_WATCH=<lo-hex>-<hi-hex>
`jit.rs` run-loop now logs (dedup by pc) the first time any block whose guest pc
is in [lo,hi) is entered. Verified: reports StartApp entries and correctly
reports ZERO for the render-init region.
```
JIT_REGION_WATCH=0x105b3a000-0x105b4000   # render-init -> 0 hits (never reached)
JIT_REGION_WATCH=0x10258b100-0x10258b200  # StartApp     -> 3 hits (reachable)
```

## What this means for the frontier
The boot is stable and the window layer is WIRED (real Xvfb X11 XID=0x200000).
The single remaining wall is: **the engine's render-init (0x105b3a2d8) is only
reachable after the Android framework producer populates its context globals /
ALooper app-command lifecycle.** Every candidate driver we control (deque-node,
deque-vtable, arg2) is behind the same gate. The next productive levers are the
ones already documented in SH7/N/P — build/fabricate the framework-driven
context object the render-init derefs, or drive the ALooper app-command
lifecycle so a real producer posts the work item — NOT more deque node surgery.

## Repro
```bash
cargo build -p arm64jit --example elfjit
# baseline clean:        elfjit <libroblox.so> 0x2173ff4 --jni            (exit 0)
# stable idle main loop: elfjit ... --jni --startapp 0x258b144 --kicker 0x106863af8  (exit 124)
# region watch: JIT_REGION_WATCH=0x105b3a000-0x105b4000 ... (0 hits = render-init not reached)
```
Workspace: **469 passed / 0 failed** (unchanged; the region-watch addition is
pure diagnostic, no behaviour change).