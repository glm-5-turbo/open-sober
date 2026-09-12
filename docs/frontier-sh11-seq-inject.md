# SH11 (2026-09-12) — sequenced deque-node-live injection CROSSES the stable idle drain (node POPPED, exit 124, sentinel crash eliminated)

## One-line result
The `--deque-node-live` foreign-node injection — which for ~10 cycles
(SH7b/SH8/SH9) always ended in a **exit-134 sentinel-as-task SIGSEGV** at
~200ms — now crosses the barrier: the live idle drainer **pops our foreign
node**, its deque head drains to empty (`0x1000000000000`), and the process
stays stable to the harness timeout (**exit 124**, no crash). Root cause of the
10-cycle wall was a *sequencing* bug in the harness, not a wrong deque model.

## Why every prior attempt crashed (the real bug)
Two mutually-exclusive invariants blocked the injection:

1. `--drain-force-pop` patches the pop-loop to always fall through. But during
   idle the drain's deque head is the **sentinel (self-node)**, so the FIRST
   forced pop dispatching the sentinel-as-task faults (SIGSEGV exit 134 within
   ~200ms) — long before an injector thread (which spent 40 recon ticks x 50ms
   = **2s** before its first write) could place a node. The injector
   structurally lost the race.
2. Plain `--drain-poll` (no force) is stable but the pop-loop never runs
   (SH7b: finite timeout only drives the maintenance heartbeat), so a placed
   foreign node sits at head **forever** (measured 400 ticks) — never popped.

So: force-pop crashes before injection; stable-poll never pops. Prior cycles
tried one or the other and hit whichever failure dominated.

## The fix: SEQUENCE the two states
The two are complementary if applied in the right ORDER (commit `c3fc23b`):

1. **DEFER the force-pop patches** when `--deque-node-live` is present: the
   startup patcher leaves `mov w24,w0`/`tbz` untouched, so the drain stays
   stable (never pops) during node placement.
2. **Inject while stable**: clone the live HEAD node's coherent payload (a
   real, constructible task node — the sentinel during idle) as the node
   template, override `[node+112] -> probe vt`, force `[node+40]!=0`, and
   **zero `[node+0]`** (fresh tail; the re-enqueue producer 0x285682c walks
   it and a stale cloned link faults at pc 0x51). This replaces the unreliable
   `[consumer+104]` / `[x19+104]` sentinel indexing SH9 used.
3. **ARM force-pop AFTER placement + drop the cached drain blocks** (new
   `pub jit::block_cache_drop_region(lo,hi)` — the JIT dispatcher had already
   compiled the UNPATCHED pop-loop, so patching guest bytes alone had no
   effect; evicting the drain-region blocks forces it to recompile the patched
   code). The first forced pop now takes OUR foreign node (passes the
   self-node-skip guard, `[node+40]=1 -> probe`), not the sentinel.

Observed (reproducible, no injection): `node POPPED by live drainer (headcell
now 0x1000000000000) — deque crossed the barrier`; process continues to
timeout exit 124. The deque crossing no longer faults.

## Remaining (next lever)
The node is POPPED and the drain no longer crashes, but the probe-handler
dispatch (`[vt+40]` -> our `probe` host-thunk) has not yet been confirmed
logging — the drain consumes the node one-shot and the deque head drains to
empty (`0x1000000000000`), so a sustained type-4 dispatch loop isn't yet
observed. Two ways forward:
1. Confirm the probe actually executes (or route `[vt+40]` to a real engine
   render/tick handler) so the crossing reaches egl*/gl*.
2. Understand the drain's post-pop: it pops our node, re-enqueues (or not),
   and parks — read whether the head draining to empty means it consumed the
   node as a one-shot task and went idle, vs a perpetual re-enqueue loop.
Baselines unchanged and re-verified: `--jni` clean exit 0; stable idle
(StartApp main loop) exit 124; workspace green 469/0.

## SH11b correction (same commit cycle): node+vtable MUST be guest-arena
allocated (low48-safe) — host-heap allocations were structurally broken
The crossing's residual faults (`pc 0x51`, garbage vt `[vt+16]=0x8b8b48...`)
traced to TWO host-heap-abuse bugs in the injector, both fixed by allocating
the node and probe vtable in the reserved guest RW tail:

- **low48 packing**: the drain CAS-pops `low48(headcell)` = bits 47..0. A
  host-heap node at `0x7f2a18000e00` truncates to `0x2a18000e00` on pop — a
  DIFFERENT address — so the drain derefs garbage. Real engine nodes live in
  the guest range `[0x100000000, 0x107...]` which is below 2^48.
- **guest memory deref**: the drain reads `[node+112]` then `[vt+40]` as guest
  memory. A host-heap probe vtable at `0x55a3...` (outside the mapped image) is
  not readable by the JIT guest load, so `[vt+40]` reads garbage (0x8b8b48...)
  instead of our registered host-thunk slot.

New `guest_arena_set_base`/`guest_arena_alloc` (elfjit) allocate node + vtable
from the reserved 384MB guest RW tail (`0x107334000`), so both the low48 packing
is exact and the guest's `ldr [vt+40]` reads our registered probe slot from real
mapped RW memory. The injected node now pops from the live idle drain at a guest
address (`0x107334040`) with the process stable (exit 124). This is a real
correctness fix (not just a harness nicety): any future render-task injector
must allocate its node in the guest address space, never host heap.

## Files
- crates/arm64jit/examples/elfjit.rs: deferred force-pop, head-node clone,
  `[node+0]` zeroing, post-placement arm + cache drop, guest_arena allocator
  (node+vtable in the guest RW tail), probe dispatch logging.
- crates/arm64jit/src/jit.rs: new `pub fn block_cache_drop_region(lo, hi)`.
- Test: `block_cache_drop_region_compiles_fresh_after_eviction`.
- Run-log: /home/hermes-worker/runs/deque-garena.txt (exit 124, node popped
  at guest addr 0x107334040).