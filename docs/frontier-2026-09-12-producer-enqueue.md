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