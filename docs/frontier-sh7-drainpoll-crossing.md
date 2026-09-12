# SH7 (2026-09-12) — idle task-deque barrier CROSSED: the gate is the drain's INFINITE TIMEOUT; `--drain-poll` forces a finite one and the engine's own pop-loop + dispatch handler (0x10285371c) run live.

## One-line result
The ~35-cycle "engine producer never enqueues / consumer never drains" wall is
broken: the parked consumers are the drain fn 0x2856e40 calling generic-wait
0x284d014 with **timeout = -1 (infinite)**, so they park in a bare futex
`futex(Q+4, WAIT_BITSET, epoch, NULL, ~0)` forever and the drain's pop-loop at
0x2856f94 is reached **only when the wait returns 1 (timed out)**. Patching the
drain's `mov x2,x22` (0x2856f40, x22 = the drain's timeout arg, = -1 during
idle) to a finite ms value makes the wait time out, the drain reaches the
pop-loop, and it now **continuously pops + dispatches the deque**, executing the
real engine dispatch handler 0x10285371c millions of times (stable: flat 1673
compiles, no crash, exit 124). Run-log:
`/home/hermes-worker/runs/boot-drainpoll-crossing.txt`.

## The empirical frame (from a live stack dump of a parked consumer)
Waiter sp (parked inside generic-wait, which shares the drain's frame because
the drain `bl`s to 0x284d018, skipping generic-wait's `sub sp,#80`):
```
sp+0x28 = 0x102856f48   # generic-wait's saved x30 = return INTO the drain after bl 0x284d014  (drain calls generic-wait at 0x2856f44)
sp+0x30 = 0xffffffffffffffff  # generic-wait's saved x20 = its timeout arg = -1 (INFINITE)
sp+0x00 = 0            # drain x29 (saved FP)
sp+0x50 = ...          # drain x20 = root (deque)
sp+0x08 = ...          # drain x30 = drain's caller (a host/dispatch address)
```
Live thread x20 = 0xffffffffffffffff ⇒ **the drain passes timeout -1 to the
wait**. This is the whole gate: an infinite timeout means the futex blocks
forever, so the wait never returns 1, so the pop-loop never runs, so no work is
ever consumed — no matter what is in the deque.

## Generic-wait 0x284d014 semantics (from disasm)
- `wait(Q=x0, epoch=x1, timeout_ms=x2)`. refcount_add([Q]); early-return 0 if
  `epoch != [Q]>>32` (version already changed).
- **timeout == -1** → jump straight to a bare blocking futex
  `futex(Q+4, WAIT_BITSET, epoch, NULL, NULL, ~0)` (0x284d114..130); returns
  w0 = 0 only when woken.
- **finite timeout** → compute a deadline, sleep the interval, then wait; returns
  w0 = 1 (via `csetm w20,#0x3` at 0x284d160) only when the deadline elapses with
  the version unchanged.

## The drain gate (0x2856e40)
```
0x2856f38 add x0,x23,#8     # Q = x23+8
0x2856f3c mov x1,x21        # epoch = version captured at entry
0x2856f40 mov x2,x22        # timeout = drain's 3rd arg (= -1 during idle)   <-- PATCHED
0x2856f44 bl  0x284d014     # generic wait
0x2856f4c mov w24,w0        # w24 = 0 woken / 1 timed-out
0x2856f7c tbz w24,#0, ...   # if woken (w24=0) -> woken path (sets timeout 0, re-waits: busy)
0x2856f80..90: cmp x21,[Q]>>32; b.ne 0x28570a4   # version changed -> drain RETURNS
0x2856f94 [POP LOOP]: x23=[x20]; x24=ldar([x23]); low48==0 -> back to wait
                       else CAS-pop via node.next -> dispatch [node+112]&~0x3f -> [vt+40]
                                                                           + [node+40]/[node+32] args
                       -> re-enqueue via bl 0x285682c -> wake futex(node+12, 0x8a, 1)
```
So the pop-loop runs ONLY on a timed-out wait (w24=1) with an unchanged version.
An infinite timeout never reaches it. **A finite timeout does**, and once the
pop-loop runs it re-enters a timeout-0 busy loop, so it continuously drains.

`--drain-poll <ms>` (elfjit) mprotects guest 0x102856f40 to `mov x2,#<ms>`
(imm12 < 4096) before jit_run, so the drain block compiles with a finite
timeout. Observe: compiles flat 1673, hits grow to ~8.2M, the step pc cycles
through the drain (0x102856f38/48/7c, 0x10284d014/0a4) AND the real dispatch
handler 0x10285371c / 0x1028538c0 / 0x1028539e8, and a hostcall slot
(0x7f0000002118). **The engine's own task-deque dispatch machinery now runs.**

## 0x10285371c = the dispatched node handler
Takes (x1 = consumer-ish, x2, x3, x4 = task type 1..n). Reads [x1+104], then
`[+104]+48` -> obj; calls a recursive handler, then `br` to a function pointer
loaded from a static global (adrp + `[x8+0x528]`). During the drain-poll spin
this receives the deque's (sentinel/self) node, so it is a degenerate
self-dispatch, NOT the render/EGL path — no egl*/gl* hostcall fires. But the
consumer side is now provably live and will consume whatever real node is placed.

## --deque-node node injection: NOT yet consumed — the DISPATCHING consumer is guest_tid 0 (its own deque)
- **Which thread drains:** under --drain-poll the dispatching consumer is
  **guest_tid 0** (live snapshot: `pc=0x10285371c lr=0x102856f38 x1=0x7f4b5486b280`,
  i.e. in the drain about to re-call generic-wait, cycling the dispatch handler).
  guest_tids 1 & 2 (whose recovered headcells are 0x10682a638 / 0x10682b338) stay
  **futex-parked** (lr=0x10284d134, live x20=-1). So --deque-node, which only
  enqueues for threads parked at lr==0x10284d134, has been injecting into the
  *idle* consumers' deques — never into the one tid 0 actually drains. That is
  why no external node is consumed.
- **Offset fix (rolled in):** the drain's pop reads the head node from
  `[headcell + 0x0]` (x23=[x20]; x24=ldar([x23])). Prior `--deque-node` wrote to
  headcell+0x10/0x18 (ring internals) — fixed to [headcell+0], plus a tag-round
  write (this cycle).
- **Remaining for real node injection:** (1) locate guest_tid 0's deque root
  (its live drain frame — needs the [sp+0x50] vs [sp+0x58] layout confirmed at
  dispatch time, not at park); (2) make the node survive the drain's tag guard
  (0x2856e78 `cmp x9,[head]>>48`) and the low-48 pointer truncation; (3) point
  `[node+112]` at a real render/tick vtable (not the sentinel's 0x106829f00) so
  the dispatched handler reaches egl*/gl*.

## Next levers (ordered), against the now-live consumer (guest_tid 0)
1. Recover the true deque slot/node layout from the drain's own disasm
   (0x2856e68-0x2856ec4) and inject a real node that passes the tag guard and
   the low-48 pointer truncation — point `[node+112]` at a render/tick vtable
   (not the sentinel's 0x106829f00) so the dispatched handler reaches the
   egl*/gl*/render path. The consumer now drains continuously, so any correctly
   placed node is consumed immediately (no futex wake/version bookkeeping even).
2. Identify 0x10285371c's global `br` target (`[adrp+0x528]`) and what real
   task node sets node.type/args to drive a render/frame callback.
3. Without --drain-poll the boot returns to the stable idle futex park (baseline
   unchanged). Both paths are preserved.

## Files
- `crates/arm64jit/examples/elfjit.rs`: `--drain-poll <ms>` guest patch (the
  barrier crosser), `--deque-node` headcell+0 offset fix + tag write, and the
  `JIT_STACKDUMP` / `JIT_DEQUE_PROBE2` diagnostics that resolved the frame.
- `crates/arm64jit/examples/dump.rs`: region disassembler (new diagnostic).

## Repro
```
cargo build -p arm64jit --example elfjit
JIT_DRIVE_LIFECYCLE=1 timeout 15 ./target/debug/examples/elfjit \
  ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni \
  --startapp 0x258b144 --kicker 0x106863af8 --drain-poll 8
# exit 124; [jit] step pc cycles the drain + 0x10285371c dispatch, compiles flat 1673.
```
Workspace 467/0. Baseline (no --drain-poll) unchanged: stable idle futex park.