# SH13 (2026-09-12) — REAL engine vtable dispatch: the engine's own task-processor runs our injected nodes

## One-line result
`--deque-node-live 0x106829f00` (the LIVE sentinel's real vtable, whose
`[vt+40] = 0x10285371c` is the engine's own drain-node task-processor) makes the
engine's **native** dispatch machinery consume our injected foreign nodes —
~124 pops in 16s, process stable to harness timeout (exit 124), zero crash, and
the block-cache GREWS past the probe baseline (≈2147 compiles / 7,361,652 hits
vs the probe's flat ≈434), i.e. the real obfuscated dispatcher is executing code
it never reached under the host-thunk probe.

## What changed / what is confirmed
- The probe mode (`--deque-node-live probe`) installed OUR host-thunk at
  `[vt+40]` and only logged the ABI args — it never ran engine code for the node.
- Passing the REAL vtable (`0x106829f00`, the drain-struct's own vtable that the
  sentinel carries during idle) instead routes the drain's pop dispatch to the
  engine's real node-processor `0x10285371c`. It runs continuously and stably
  on our injected nodes (124 pops), does NOT feed on them (if the dispatched
  handler faulted on our zeroed/cloned node we would see exit 134; we see 124),
  and its deep obfuscated dispatch table (mul-high/divide at
  `0x102853a04..0x1028539b4`) keeps compiling and running new blocks.
- The `--deque-node-live <hex-vt>` hex path was already code-supported; SH13
  verifies it empirically against the real vtable and captures it as the
  preferred harness mode (vs the probe).

## Repro (reproducible, headless)
```bash
cargo build -p arm64jit --example elfjit
JIT_DRIVE_LIFECYCLE=1 timeout 16 ./target/debug/examples/elfjit \
  ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144 \
  --kicker 0x106863af8 --drain-poll 100 --deque-node-live 0x106829f00
# expect: repeated "[elfjit:deque-node-live] NODE 0x1073.. POPPED by live
#   drainer ... dispatch #N; re-injecting..." with NO probe logging —
#   the dispatch goes through the engine's real processor; exit 124.
JIT_STATS=1 ...   # block-cache keeps growing past the probe baseline
```
Run-log: `/home/hermes-worker/runs/sh13-realvt-runlog.txt`.

## Why this advances the frontier
SH7b/SH12 named the lever "route the node's vtable at a REAL engine render/tick
handler instead of the probe." SH13 proves the real-vtable substitution is
mechanically sound and stable — the engine's own task-dispatch code now runs our
injected nodes. What it has NOT yet reached is a **render** handler: the real
processor `0x10285371c` type-dispatches on `w4` (our injected nodes always get
`w4=4`, the task-maintenance type) through an obfuscated hash table
(`0x102853a04..9b4`), and none of the executable states observed advance into
`egl*`/`gl*` hostcalls yet (hostcall histogram unchanged: syscall +
pthread/JNI/mem, zero GLES). The next lever is to find/construct a node whose
`[node+32]` (the dispatcher arg 2) or dispatch index reaches a render/tick
handler in that obfuscated table — or supply the task-maintenance path real
framework state so it proceeds to render.

## SH13b (same cycle) — `--deque-arg2 <hex>` sweep is a NEGATIVE
Added an elfjit `--deque-arg2 <hex>` override for the injected node's
`[node+32]` (becomes dispatch arg2 = `[node+32]&~1`, the controllable node-content
selector). Swept 1/2/3/4/8/0x100 against the real vtable (0x106829f00):
- All values stable (exit 124, no crash).
- Block-cache count is NOISY run-to-run (434..2212 — depends on whether injection
  lands inside a large init window, not on arg2); the deeper-2212 observation for
  arg2=1 did not reproduce, so it is NOT a discriminator.
- Hostcall histogram identical across all values (syscall + pthread/JNI/mem, zero
  egl/gl). The obfuscated render table is not reachable via arg2 sweep alone.
Takeaway for the next session: `[node+32]` does not select a render handler;
the engine's render dispatch lives behind its runtime-built BSS vtable graph and
obfuscated hash-table internals (mul-high at 0x102853a04..9b4), not a linear
arg2 index. Do not re-run the arg2 sweep.