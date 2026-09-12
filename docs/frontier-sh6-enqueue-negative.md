## SH6 (2026-09-12, elfjit `--deque-node`) — host enqueue PROVEN NOT sufficient; deque model corrected from assembly

Implemented a real host-side producer in elfjit (`--deque-node <vtable>`):
for each parked consumer it recovers the deque, allocates a task node, links
it into the deque, sets `[node+112]=vtable`, bumps the Q' version epoch and
FUTEX_WAKEs. Two distinct enqueue strategies were tried and BOTH are robustly
NOT consumed (`popped=false` every check, compiles flat, no crash):

1. **node into [headcell] = slot+0** (the SH5b "head cell"): not consumed.
2. **node into HEAD=slot+0x10 and TAIL=slot+0x18, [node]=0** (the ring shown by
   the slot dump): not consumed.

In both, the head field stays pointing at our node for the whole 16s run — the
parked consumers never CAS-pop it. Combined with the version-epoch bump + wake,
this is a hard negative: **placing a node in the deque + bumping the epoch does
not make the consumers drain.** This corrects the SH5b "circular intrusive list
with sentinel at the drain struct" model, which located the enqueue point wrong.

### The version-TIMEOUT gate (why the pop-loop is dead during idle) — new

Wait(0x284d014) return semantics (from its epilogue at 0x284d0ec/0x284d134/
0x284d160): **w20=0** on the futex-woken / version-changed / already-changed
paths; **w20=1 ONLY on the deadline(timeout)-elapsed path** (cset w20,cs). The
drain at 0x2856f7c then:
```
  tbz w24,#0 → 2856ef0     # wait returned 0 (woken/version-changed) → wait-loop
  2856f80/90: cmp x21,[Q]>>32; b.ne 28570a4   # version changed → RETURN
  2856f94: DRAIN POP LOOP                     # reached ONLY on timeout + version match
```
So the pop-loop is entered ONLY when the wait TIMED OUT and the version still
matches the caller's captured x21. On this box the consumers park with an
effectively infinite timeout and only wake when a producer bumps the version,
which makes the drain RETURN (b.ne 28570a4) — so the pop-loop never runs during
idle, and a host bump just makes the drain exit so the caller re-captures the
new version and re-parks. Placing a node even WITHOUT bumping (natural-timeout
poll) is also not consumed (infinite timeout → never polls). **The deque is
drained only by a real (framework) producer that re-enters the drain loop —
host writes to the ring are not sufficient.**

(Three negative enqueues total: slot+0, slot+0x10/0x18±bump, slot+0x10/0x18
no-bump — all `popped=false`.)

### Corrected deque layout (from full producer/drain disassembly)

The deque is a **pointer-ring arena** per consumer, at a stable guest-bss base
(0x10682a638, 0x10682b338 — the two real ones; the 3rd consumer's headcell is a
host-heap garbage-ASCII cell, ignore it):

```
slot base (from dump):
  +0x00 = sentinel (== drain_struct / consumer obj host-heap ptr)
  +0x08 = 0
  +0x10 = HEAD field       (value = 0x10682a640 == slot+8, the ring's node 0)
  +0x18 = TAIL field       (value = 0x10682a640)
  (empty: HEAD == TAIL == slot+8, the self-referential first node; [node]=0)
```

- **Producer / enqueue 0x285682c** `(this=x0, task=x1, mode=w2, cbarb=x3)`:
  optional cb `[this+24]` (returns bit0 ⇒ done); else `slot = [this+8] +
  sched_getcpu()&0xf * 0x4a140`, then a tagged-CAS push against HEAD=slot+0x10:
  walk `ldar[head] → [node]` to find the tail (next==0), CAS-link the new node
  (helpers 0x2b9e760 CAS / 0x2b9e6e0 pop); the ring is node-next-linked.
- **Consumer / drain 0x2856e40** `(root=x0, consumer=x1, timeout=x2)`: the
  steady loop (2856f94) does `x23=[x0]; x24=ldar[[x0]...]; x22=x24&low48; if
  x22==0 EMPTY→wait`; else CAS-pop (head←node.next, helper 0x2b9e6e0), then
  dispatch the popped node via `[node+112]&~0x3f → vt → [vt+40]` (+ `[node+40]`,
  `[node+32]` arg) and **re-enqueue the node via `bl 0x285682c`** (2857020).
  After each VM-dispatch it wakes `futex(node+0xc, 0x8a, 1)` (2857060-78).
- **The version gate is why a node+bump is not enough:** the drain is gated by
  the version-epoch wait (generic wait 0x284d014 parks in
  `futex(Q+4, WAIT_BITSET, low32(epoch))`). On wake, 2856f90/2856f80 check
  `cmp x21, [Q]>>32`; **if the version CHANGED the drain RETURNS** (b.ne →
  28570a4) instead of entering the pop-loop at 2856f94 (which only runs when
  the version still MATCHES the caller's captured x21). So a host version-bump
  makes the drain return; the pop-loop is only reachable when a NEW drain entry
  re-captures the (now-bumped) version as its expected x21 and finds work.
  Empirically the caller does not re-enter the drain into the pop-loop after an
  external bump — hence both enqueue attempts sit unconsumed.

### What this pins (next-cycle levers, ordered)

The lever is NOT "place node + bump epoch" (disproven twice). Real candidates:
1. **Invoke the real producer 0x285682c as a guest call** with a valid task node
   — let the framework's own push path run (needs the scheduler `this` obj at
   `[this+8]`=slot-array, recoverable from the drain_struct at `[x1+104]`).
2. **Synthesize a full drain re-entry** so the pop-loop at 2856f94 runs with a
   version that matches the new expected — i.e. bump version, let the drain
   return, and re-enter it via its caller with a fresh captured epoch + the node
   already in HEAD.
3. Reverse what the caller does after the drain returns (the loop around
   0x284eb80 / the drain's caller) to find what re-enters the drain.

Run-log: /home/hermes-worker/runs/deque-node-v3.txt (node into slot+0x10/0x18,
negative), deque-node-v2.txt (slot+0), deque-node-verify.txt, deque-node-
enqueue-vt.txt. Workspace 467/0; boot unchanged (stable idle main loop,
exit 124). elfjit `--deque-node <vtable>` is the faithful reuseable harness.