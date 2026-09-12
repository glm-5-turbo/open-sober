# SH9 (2026-09-12) — drain SELF-NODE-SKIP discovered (sentinel-repoint is structurally futile); foreign-node path yields a controlled guest dispatch

## One-line result
Reversed the drain pop-loop `0x2856e40..0x28570a4` from disassembly and found a
**self-node-skip guard** the SH8 model missed: at `0x2856fc8` the drain compares
the popped node to `[consumer+104]` and, if equal, RETURNS (`b.eq 0x28570a4`)
**without dispatching it**. The idle sentinel IS that self node (the drain's own
struct, vtable 0x106829f00), so repointing the sentinel's `[node+112]`
(`--deque-probe`, SH8) can never make the probe fire — the sentinel never
reaches the type-4 dispatch at `0x2857008`. The correct lever is the **foreign
node** path (`--deque-node-live`): a node whose address != `[consumer+104]`
passes the skip guard and reaches the real dispatch. This cycle: `--deque-node-live
probe` (auto-build a host-thunk PROBE vtable: `[vt+40]`=registered host thunk,
`[vt+16]`=ctx) so the foreign node's dispatch is observable. Result: the pop-loop
NOW dispatches in a REAL guest thread (tid 0, `rbx_matches_gueststate=true`,
`in_jit_run=true`) — a controlled first crossing through the foreign-node path —
faulting only because the node's handler is our probe (host-thunk), not a real
engine render callback. This corrects SH8's "why the probe never fires" (race
with in-flight dispatcher) — the true cause is the self-node-skip guard.

## The drain pop-loop, decoded (ground truth, file vaddr = guest - 0x100000000)
```
0x2856f94   ldr  x23,[x20]          ; x20 = deque root, x23 = headcell
0x2856f98   ldar x24,[x23]          ; packed head (low48=node, high16=tag)
0x2856f9c   ands x22,x24,#0xffff_ffff_ffff ; node = low48
0x2856fa0   b.eq 0x2856ee8          ; empty -> wait
            ... CAS pop (2b9e6e0) ...
0x2856fc8   ldr  x8,[x19,#104]      ; SELF-NODE-SKIP GUARD: x8 = [consumer+104]
0x2856fcc   cmp  x8,x22             ; popped node == self?
0x2856fd0   b.eq 0x28570a4          ; YES -> RETURN, never dispatch (sentinel skipped)
0x2856fd4   ldr  x8,[x22,#112]      ; vt = [node+112] & ~0x3f
0x2856fdc   and  x9,x8,#~0x3f
0x2856fe0   ldr  x8,[x9,#40]        ; handler = [vt+40]
0x2856fe4   cbz  x10               ; guard [node+40]!=0 (x10=[node+40])
0x2856fe8   cbz  x8                ; guard handler!=0
0x2856fec   ldr  x10,[x22,#32]
0x2856ff0   ldr  x0,[x9,#16]
0x2856ff4   mov  x1,x19             ; consumer
0x2856ff8   mov  x3,x22             ; node
0x2856ffc   mov  w4,#4
0x2857000   mov  x5,#0
0x2857004   and  x2,x10,#~1         ; x2 = [node+32]&~1
0x2857008   blr  x8                 ; DISPATCH handler(vt+16, consumer, node+32&~1, node, 4, 0)
0x285700c   ; re-enqueue popped node via the real producer 0x285682c, loop
```
The skip guard at 0x2856fc8 is the piece SH8's model omitted and is the whole
reason `--deque-probe` (sentinel repoint) can never cross: the sentinel is
`[consumer+104]` itself.

## --deque-node-live probe (elfjit, opt-in)
Before, `--deque-node-live <vt>` required a real render vtable (unknown). Now
pass the literal `probe`: it allocates a fake vtable with `[vt+40]=registered
host-thunk` (the auto `probe` fn) + `[vt+16]=0xdeadbeef` and injects a FOREIGN
calloc'd node (np != [consumer+104]) with `[node+112]=that vt`, `[node+40]=1`,
packed into the live headcell. The drain's next CAS-pop takes our node, passes
the skip guard (foreign), and dispatches it through `[vt+40]=probe_addr` — the
probe host-thunk. Logs: `PROBE vtable (vt=0x..., [vt+40]=0x...)`.

Observed run: the forced-pop loop DISPATCHES our foreign node in a real guest
thread (tid 0, `rbx_matches_gueststate=true`, `in_jit_run=true`) — guestpc lands
at 0x7f0000002068 (a host-thunk region address the JIT dispatches). This is a
controlled first crossing through the foreign-node dispatch path (vs SH8's
untracked-host-thread fault). It still faults because the handler is our probe
host-thunk, not a real engine render callback — i.e. the crossing mechanism now
works and the remaining need is a REAL render/tick vtable for `[vt+40]` (+ a
coherent node payload for the handler it leads to).

## Next lever (unchanged shape, now with the correct mechanism)
1. Identify a REAL render/tick vtable (what the engine sets `[node+112]` to for
   a frame task), and a coherent payload — then `--deque-node-live <vt>` reaches
   egl*/gl*.
2. Invoke the REAL producer 0x285682c as a guest call with a valid task node
   (recover scheduler `this` = the [x1+104] object) — the cleanest, keeps the
   framework push path running.
Baselines unchanged: `--jni` clean exit 0; stable idle (StartApp main loop)
exit 124 via `--drain-poll`; `--deque-node-live`+`--drain-force-pop` opt-in.

## Files
- crates/arm64jit/examples/elfjit.rs: `--deque-node-live probe` (auto host-thunk
  probe vtable); `--deque-probe` unchanged (documented as structurally futile
  against the skip guard, kept for the ABI evidence).
- Run-log: /home/hermes-worker/runs/deque-nodelive-probe-crossing.txt (exit 134).