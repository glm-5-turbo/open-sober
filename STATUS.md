# Open-Sober Status — Ongoing Autonomous Development

## SH13 (Sep 12, 2026): REAL engine vtable dispatch — the engine's native task-processor runs our injected nodes (~124 pops, exit 124, zero crash); block-cache grows past probe baseline

Work on the `dev` branch (HEAD 1a0ffbb+), workspace 469/0. SH12
confirmed+sustained type-4 dispatch through OUR host-thunk probe. SH13 steps it
forward: instead of the probe, inject with the REAL sentinel vtable
(`--deque-node-live 0x106829f00`, whose `[vt+40]=0x10285371c` is the engine's
own drain-node task-processor).

- **Mechanically verified + stable:** the engine's native dispatch machinery now
  consumes our injected foreign nodes — ~124 pops in 16s, process stable to
  harness timeout (exit 124), zero crash (a faulting dispatcher would give exit
  134). The run has NO probe logging, i.e. the dispatch goes through the engine's
  real processor.
- **Block-cache grows past probe:** `JIT_STATS=1` shows ~2147 compiles /
  7,361,652 hits vs the probe's flat ~434 — the obfuscated dispatch table
  (`0x102853a04..9b4`) keeps compiling+running real engine regions the host-thunk
  probe never touched.
- **Not yet render:** the real processor type-dispatches on `w4` (injected nodes
  always get `w4=4`, task-maintenance) through an obfuscated hash table; hostcall
  histogram is still syscall + pthread/JNI/mem, **zero egl*/gl***. The next
  lever: construct a node whose `[node+32]`/dispatch-index reaches a render/tick
  handler in that table, or give the maintenance path real framework state.

### Repro (reproducible, headless)
```bash
cargo build -p arm64jit --example elfjit
JIT_DRIVE_LIFECYCLE=1 timeout 16 ./target/debug/examples/elfjit \
  ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144 \
  --kicker 0x106863af8 --drain-poll 100 --deque-node-live 0x106829f00
# expect: repeated "NODE 0x1073.. POPPED by live drainer", NO probe logging, exit 124
```
Run-log: `/home/hermes-worker/runs/sh13-realvt-runlog.txt`.
Doc: `docs/frontier-sh13-realvt-dispatch.md`. Baselines unchanged:
`--jni` clean exit 0; stable idle exit 124.

## SH12 (Sep 12, 2026): type-4 dispatch CONFIRMED + SUSTAINED through the real engine idle drain — foreign nodes now pop AND dispatch continuously (107 dispatches/104 pops across 105 node addrs in 14s, exit 124, zero crashes)

Work on the `dev` branch (HEAD 3b37deb), workspace 469/0. The SH11 residual
(object) is closed:

- **Root cause: vtable handler offset off-by-one.** Both probe vtable builders
  wrote `[vt+40]` at `add(4)` = byte 0x20 instead of `add(5)` = byte 40. The
  drain does `ldr [vt,#40]`, read 0, guard failed, node consumed WITHOUT
  dispatch. Fix `a2448fc`: `add(5)` in both sites.
- **Verified:** the real idle drainer pops our injected guest-arena node and
  type-4 dispatches it through our host-thunk handler with the exact engine ABI
  (`x0(vt+16)=0xdeadbeef`, `x3(node)=<our node>`, `w4=4`, `x5=0`).
- **Sustained (`3b37deb`):** re-inject a fresh node per pop → continuous stream.

Run-logs: `/home/hermes-worker/runs/sh12-probe-runlog.txt`,
`/home/hermes-worker/runs/sh12-sustain-runlog.txt`.
Doc: `docs/frontier-sh12-dispatch-confirmed.md`.