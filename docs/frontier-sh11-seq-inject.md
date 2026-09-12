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
logging — the drain consumes the node and re-parks, so the type-4 handler call
count is still 0 in these runs. Two ways forward:
1. Confirm the probe actually executes (or route `[vt+40]` to a real engine
   render/tick handler) so the crossing reaches egl*/gl*.
2. Investigate whether the drain's post-pop re-enqueue returns to the idle
   sentinel (headcell drained to empty) vs a perpetual ours-node loop.
Baselines unchanged and re-verified: `--jni` clean exit 0; stable idle
(StartApp main loop) exit 124; workspace green 468/0.

## Files
- crates/arm64jit/examples/elfjit.rs: deferred force-pop, head-node clone,
  `[node+0]` zeroing, post-placement arm, `block_cache_drop_region` call.
- crates/arm64jit/src/jit.rs: new `pub fn block_cache_drop_region(lo, hi)`.
- Run-log: /home/hermes-worker/runs/deque-seq5.txt (exit 124, node popped).