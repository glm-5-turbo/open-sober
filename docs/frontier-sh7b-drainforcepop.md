# SH7b (2026-09-12) — CORRECTION to SH7: the finite-timeout never reached the pop-loop; forced pop-loop is the real crossing (and now dispatches, faults on the sentinel)

## One-line result
SH7's `--drain-poll` claim that a finite wait timeout makes the engine task-deque's
**pop-loop** (0x2856f94) run is **WRONG**. Measured: under `--drain-poll <ms>` the
drain's post-wait `tbz w24,#0` (0x102856f7c) fires **~128k times** but the pop-loop
**0x102856f94 is entered 0 times**. The finite timeout only hot-loops the drain's
maintenance heartbeat (0x10285371c with x4=2/3 — the drain struct's own vtable
dispatch), never the node-pop path (x4=4). The real gate is the drain's own
wait-result latch. Forcing `mov w24,w0` (0x102856f4c) -> `mov w24,#1` AND NOP'ing
the tbz at 0x102856f7c (`--drain-force-pop`) **makes the pop-loop run for the first
time** — it CAS-pops and dispatches, faulting on the sentinel as a task node (the
long-anticipated controlled first crossing). Run-log:
`/home/hermes-worker/runs/drain-forcepop-crossing.txt`.

## Why finite-timeout alone is not enough (the correction)
Generic-wait 0x284d014 maps the HOST futex's return through `cmn x0,#1`
(0x284d0a4): it only treats an EXACT `x0==-1` as "timed out under deadline". The
host futex returns **-110 (ETIMEDOUT)** on timeout (native Linux semantics:
`__linux futex` returns `-ETIMEDOUT`), which is `!= -1`, so generic-wait falls to
0x284d0ec -> returns **w0=0 ("woken")**. The drain's `tbz w24,#0` (0x2856f7c)
therefore always branches back to the maintenance re-loop and the pop-loop
(0x2856f94, reached only when the tbz falls through AND the version matches) is
dead. So `--drain-poll`'s 8ms timeout produced a hot maintenance heartbeat, which
SH7 misread as "the drain pops + dispatches the deque". The dispatched
0x10285371c x4=2/3 calls are the drain STRUCT's own callback (from
`[x19+104]+112` vtable), not node pops.

## --drain-force-pop (the real crossing, opt-in)
Adds two guest patches (both gated by `--drain-force-pop` so plain `--drain-poll`
stays stable):
1. `0x102856f4c`: `mov w24,w0` (0x2a0003f8) -> `mov w24,#1` (0x52800018).
2. `0x102857f7c`: `tbz w24,#0,<back>` (0x3607fbb8) -> NOP (0xd503201f), so the
   drain ALWAYS falls through to the version-check (0x2856f80) -> pop-loop
   (0x2856f94).

Result (proven): the pop-loop now runs; it CAS-pops the deque head and dispatches
`[node+112]&~0x3f -> [vt+40]` with `[node+40]`/`[node+32]`/w4=4. During the idle
deque the head is the SENTINEL (the drain's own struct), so the drain dispatches
the sentinel as a task node → the handler (0x10285371c) walks the sentinel's
garbage task content and faults (strlen on guest-text pointer 0x1005af2, lr
0x10222f330). This is the documented "zeroed/foreign node faults — a controlled
first crossing to capture" milestone: **the engine's real task-deque pop + dispatch
machinery now executes.**

## --deque-node-live <vt-hex> (new, targeting the LIVE drainer)
The SH7 handoff correctly noted `--deque-node` injected only into the parked
consumers (tids 1/2, lr==0x10284d134) — never tid 0, which is the live drainer
under --drain-poll. `--deque-node-live` instead:
- Recovers tid 0's deque root from its live x20 while pc is in the drain body
  (0x102856e40..0x1028570a4) — stable across samples and host-readable.
- Recons the live deque: `[root]=headcell`, `[headcell]=packed head` (low48=node,
  high16=tag), `[root+8]=tag`; head node internals (`[node+112]=vt`,
  `[vt+40]=handler`, `[node+40]`, `[node+32]`). Confirms the live head is the
  sentinel (vt=0x106829f00 -> [vt+40]=0x10285371c).
- Does NOT wait for an empty window (the drain keeps the deque non-empty by
  re-enqueuing), so it swaps the task node over the live head and checks whether
  the drain pops it.

Note: because the drain re-enqueues every popped node, the node staying at head is
NOT itself proof of non-consumption; the discriminating signal is the type-4
dispatch of OUR node through its vtable. Under --drain-force-pop the run faults
during the sentinel dispatch, so a real injected node's dispatch has not yet been
isolated — that is the next lever.

## Repro
```
cargo build -p arm64jit --example elfjit
# baseline (stable idle, no patches):  exit 124
JIT_DRIVE_LIFECYCLE=1 timeout 14 ./target/debug/examples/elfjit \
  ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni --startapp 0x258b144 \
  --kicker 0x106863af8 --drain-poll 8
# --drain-force-pop (pop-loop runs, dispatches sentinel, faults):  exit 134
JIT_DRIVE_LIFECYCLE=1 timeout 14 ... --drain-poll 8 --drain-force-pop
```
Run-log: `/home/hermes-worker/runs/drain-forcepop-crossing.txt` (exit 134, SIGSEGV
guestpc=0x7f0000002068 tid 0, pop-loop + sentinel dispatch reached).
Workspace 467/0. Baseline (no --drain-poll) unchanged: --jni-only clean exit 0.

## Files
- `crates/arm64jit/examples/elfjit.rs`: `--drain-force-pop` (wait-latch + tbz
  force, the TRUE pop-loop crossing), `--deque-node-live <vt>` (live-drainer
  deque injection + recon). Both opt-in; plain `--drain-poll` unchanged.