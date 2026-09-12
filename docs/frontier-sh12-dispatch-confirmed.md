# SH12 (2026-09-12) — type-4 dispatch CONFIRMED and SUSTAINED through the real engine idle drain

## One-line result
The `--deque-node-live` foreign-node injection now not only pops stably (SH11)
but **actually dispatches**: the engine's live idle drainer continuously
pops + type-4 dispatches our injected guest-arena nodes through OUR host-thunk
handler — 107 dispatches / 104 pops across 105 distinct node addresses in 14s,
stable to harness timeout (exit 124, zero crashes). This closes the SH11
residual ("probe handler not confirmed executing").

## The bug that blocked confirmation (root cause)
Both probe vtable builders (`--deque-node-live` and `--deque-probe`) wrote the
handler at the wrong offset. They used `(v as *mut u64).add(4)` = byte offset
0x20, but the drain dispatches via **`[vt+40]` = byte 40 = u64 index 5**. So
`ldr [vt,#40]` read 0 (the calloc-zeroed slot):
- the drain's `handler != 0` guard (0x2856f?x) failed,
- the node was popped but **consumed without dispatch**,
- the residual was misread as "dispatch not reachable" when it was a harness
  off-by-one.

Fix (`a2448fc`): `add(5)` in both builders. Verified: dispatch #1 fires with
`x0(vt+16)=0xdeadbeef` (our ctx marker read correctly from the guest-arena
vtable), `x3(node)=0x107334040`, `w4=4` — the exact engine dispatch ABI
pinned in SH7b/SH8.

## Sustaining the stream (`3b37deb`)
The injector previously `return`ed after the first pop, so the drain went idle
(head drained to `0x1000000000000` = empty). Now it re-injects a fresh
guest-arena node (allocating a new one each cycle from the arena) on every pop,
so the drain's pop-loop keeps dispatching. Observed: monotonically increasing
dispatch/pop counts, each cycle a distinct node address, all `w4=4` type-4
dispatches with correct ABI, exit 124.

## Repro
```bash
cargo build -p arm64jit --example elfjit
JIT_DRIVE_LIFECYCLE=1 timeout 14 ./target/debug/examples/elfjit \
  ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144 \
  --kicker 0x106863af8 --drain-poll 100 --deque-node-live probe
# expect: repeated "[elfjit:deque-probe] type-4 dispatch #N: x0=0xdeadbeef ...
#   x3(node)=0x1073... w4=4 x5=0" interleaved with INJECTED/POPPED, exit 124.
```
Run-logs: `/home/hermes-worker/runs/sh12-probe-runlog.txt`,
`/home/hermes-worker/runs/sh12-sustain-runlog.txt`.

## Next lever (unchanged shape, now that dispatch is live)
The drainer now executes OUR handler. The path to egl*/gl* is to route the
node's vtable at a REAL engine render/tick handler instead of the probe, so a
dispatch actually drives the engine's frame/render machinery — OR feed the
real producer (0x285682c) a coherent render task node. Iterate on making the
dispatched handler a real engine callback. Baselines unchanged and re-verified:
`--jni` clean exit 0; stable idle exit 124; workspace 469/0.