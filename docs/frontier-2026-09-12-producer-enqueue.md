# Producer-enqueue wall — precise contract (2026-09-12, cycle SH4)

Ill-posed flag #2s: this cycle closed the "0xF4240 go-token" misreading and
pinned the consumer's real proceed-gate from first-principles disassembly.
467 tests pass (workspace green). Boot unchanged (stable idle main loop, exit
124). The wall is now exactly one thing: **enqueue a real task node into the
engine's per-thread lock-free task-deque head, then signal.**

## What the parked threads actually are

All 3 guest threads are CONSUMERS, not producers. They park inside a generic
futex wait-with-timeout at 0x10284d018 (reached via `blr` — vtable-dispatched,
ZERO static BL callers), which is the queue's "empty ⇒ sleep" primitive:

    wait(obj=Q, expected_seq=x21, timeout_ns=x22)  0x10284d018
      atomic_add(&Q.refcount, +1)  (helper 0x2b9e9b0 = LDADDAL)
      if (expected >> 32) != (old >> 32)  -> already-changed, skip futex
      else futex(Q+4, WAIT_BITSET|PRIVATE, val=low32(expected))   # the park
      ... on timeout: clock_gettime deadline recheck loop ...
      atomic_add(&Q.refcount, -1); return

The shifted epoch `Q>>32` is a self-syncing VERSION COUNTER: a producer bumps
it and a consumer that captured an OLD value sees the mismatch and proceeds.
The low 32 bits are a refcount the consumer nudges +1/-1 around the futex.
The futex word is at `Q+4` (= the field prior-handoffs called `x19+4`).

## The consumer drain loop (task deque), fns 0x285682c / 0x2856f44

After the wait returns "changed", the consumer (same function family, vtable
dispatch at [vt+40] when non-null) drains a LIST of tasks:

    loop:
      x23 = [x20]                      # head pointer cell
      x24 = ldar [x23]                 # atomic head (packed ptr+validity)
      if (low48(x24) == 0)  goto EMPTY # nothing to do -> wait(obj, ...)
      ... process one task via blr [vt+40], then loop ...

`x20` is set up as `per-cpu slot base` computed with `umaddl x24, w9, w8, x23`
where `x23 = [x17+8]` and `w9 = sched_getcpu() & 0xf` — i.e. the deque is a
**per-CPU lock-free task queue** (head at `[x24+0x10]`). The head is 0 on this
box because the Android framework producer (render choreographer / looper /
input) that posts tasks does not exist here.

## Why version+latch poking is NOT a producer (confirmed, `--futex-bump`)

Prior `--futex-kick`/`--futex-set` only wrote the LATCH (Q+4). This cycle
added `--futex-bump`, which ALSO increments the VERSION word `[Q]>>32`
(real produce = bump version + set latch + FUTEX_WAKE). Empirically:

    it=0 BUMP [Q]=0x7f55..788 ver 0x0->0x1   (per-thread, host-heap Q)
    ... compiles STALL at 1668, JIT_STATS heartbeat stops, all 3 re-park --
    exit 124 ------------------------------------------------

Why: the consumer re-reads `[Q]>>32` each iteration, so the host's bumped value
becomes the NEW expected — the gate is a MOVING epoch, not a discrete "go".
There is no host-writable scalar that makes an empty deque produce work. The
only lever is a REAL task node in the deque head, which requires the Android
framework task format (or a GPU/render host). This is framework-owned, not a
JIT/loader/decoder gap.

## Status vs HARD GATE

Boot-stabilization (load → JNI_OnLoad 0x10006 → StartApp → stable headless
idle main loop) HOLDS and is reproducible. Every translation surface for a
frame is complete and gated (decoder 100%, Mesa llvmpipe egl/gl, texture-codec
ETC1/2+ASTC+EAC+ATC→RGBA, GLES float/mixed bridge, ANativeWindow→real X11 XID).
The single remaining barrier is the framework task-producer enqueue.

## Run (reproducible)

    cargo build -p arm64jit --example elfjit
    JIT_DRIVE_LIFECYCLE=1 JIT_THREADS=1 JIT_STATS=1 \
      timeout 15 ./target/debug/examples/elfjit \
        ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
        --jni --startapp 0x258b144 --kicker 0x106863af8 \
        --futex-kick 5 --futex-bump
    # expect: BUMP lines (version increment), compiles flat 1668, exit 124.
    # Run-log: /home/hermes-worker/runs/futex-bump2.txt

## Precise producer/consumer contract (cycle SH5, from full disassembly)

A full disassembly of the scheduler (producer 0x285682c, consumer drain
0x2856e40, generic wait 0x284d014) + a live host-side deque probe replaced the
vaguer "x19/Q+4" notes. file vaddr = guest − 0x100000000.

- **Producer / enqueue = 0x285682c** (`this`=x0, task=x1, mode=w2, cbarb=x3):
  optional callback at `[this+24]` runs first when non-null (returns bit0 ⇒
  done). Else it computes the **per-CPU slot base** `[this+8] +
  sched_getcpu()*0x4a140` and pushes the task onto that slot's lock-free
  tapered queue via tagged-CAS helpers (pop 0x2b9e6e0, push 0x2b9e720, refcnt
  0x2b9e760). The deque is **per-CPU MPSC**: head atomic at `slot+0x10`
  (packed low48=node ptr, high16=tag), tail at `slot+0x18`. Node link = `[node]`.
- **Consumer drain = 0x2856e40** (`root`=x0): recovers per-CPU head from
  `ldar [[root]]`. The steady pop loop (0x2856f94): `x24=ldar[[root]]`;
  empty iff `low48(x24)==0`; else CAS-pop node and process it via
  `[node+112]&~0x3f → [vt+40]` (and `[node+40]`), then the callback at
  `[node+32]`. After processing it wakes `futex(node+0xc, 0x8a=WAKE_BITSET|PRIVATE, 1)`.
- **Generic wait-with-timeout = 0x284d014**(`Q`, `epoch`, `timeout_ns`): saves
  caller callee-saved regs at `stp x20,x19,[sp,#64]`; refcount `atomic([Q],+1)`
  via 0x2b9e9b0 (x0=1); returns early if `epoch != [Q]>>32`; else
  `futex([Q]+4, 0x89=WAIT_BITSET|PRIVATE, val=low32(epoch))`; on EAGAIN a
  `clock_gettime` deadline loop; then `atomic([Q],-1)`.
- **Waiter-frame recovery (the enqueue prerequisite):** at park (inside the
  futex `bl syscall`, x30=0x10284d134 is the *in-wait return-into-fn*, not the
  caller), the DRAIN's callee-saved regs live on the waiter stack:
  `[sp+64]` = drain `x20` = deque root, `[sp+72]` = drain `x19` = consumer
  struct; the drain's saved `x30` is at `[sp+32]`. Confirmed live with
  `JIT_DEQUE_PROBE=1` (elfjit): each parked consumer's `[root]` resolves to a
  **stable guest-bss head-cell** (0x10682a6x38, 0x10682b338 for two per-CPU
  slots) — the exact address a host producer must push onto.
- **The deque is a circular INTRUSIVE list, not a 0-terminated head (corrected
  SH5b):** the head node is a **self-referential sentinel** at the drain struct
  itself: `head.node == drain_struct`, and `[struct+112]=vt=0x106829f00` with
  `[vt+40]` = a REAL guest handler (0x10285371c). So the consumers are NOT
  waiting on a "head==0 empty" — the sentinel is always present; they park in
  the generic `Q'` wait (`[Q']`=refc=1 / epoch high-32=0, futex at `Q'+4`)
  waiting for a producer to (a) insert a task node into the circular list and
  (b) bump `[Q']>>32` epoch + FUTEX_WAKE.
- **Host enqueue plan (correct for the circular list):** allocate a node, link
  it into the circular intrusive list around the sentinel (the drain struct),
  set `[node+112]=0x106829f00` so the dispatch resolves to the real handler
  `[vt+40]=0x10285371c` (a zeroed node instead faults deref'ing `[0x28]`), then
  bump `[Q']>>32` epoch and FUTEX_WAKE on `[Q']+4`. The loop's work is the real
  engine task dispatched at `[vt+40]`.