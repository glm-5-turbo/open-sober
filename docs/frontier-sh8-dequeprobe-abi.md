# SH8 (2026-09-12) — dispatch ABI reversed + `--deque-probe` vtable-repoint lever; sentinel-as-task still faults (honest)

## One-line result
Fully reversed the engine task-deque consumer's POP-LOOP dispatch ABI from live
disasm (verified against libroblox 0x2856fd4..0x2857008) and pinned it with a
regression test; added an opt-in `--deque-probe` elfjit lever that live-repoints
BOTH parked consumers' sentinel `[node+112]` vtable to a host-thunk we control —
PROVEN working (logs `REPOINTED sentinel ... [node+112]: 0x106829f00->0x...`)
but the dispatched handler never fires (`probe count=0`): the forced pop-loop
faults in an UNTRACKED host thread (reading host-thunk slot addr 0x7f0000000090
= slot 18 as a pointer) before our handler runs. Exit 134. This is the precise
next wall, not a crossed barrier.

## The dispatch ABI (the durable win — regression-pinned)
Consumer pop (drain 0x2856f94) after CAS-popping the head node:
```
node = low48([headcell])
vt      = [node+112] & ~0x3f
handler = [vt+40]
guard:  [node+40] != 0  &&  handler != 0
handler([vt+16], consumer, [node+32]&~1, node, w4=4, x5=0)
```
Ground truth (objdump/qemu-verified instruction stream):
- 0x2856fd4 `ldr x8,[x22,#112]`    ; x22 = node
- 0x2856fdc `and x9,x8,#~0x3f  `   ; vt
- 0x2856fe0 `ldr x8,[x9,#40]  `    ; handler
- 0x2856fe4 `cbz x10,+68  `        ; guard [node+40] (@ 0x2856fd8 ldr x10,[x22,#40])
- 0x2856fe8 `cbz x8,+36  `         ; guard handler
- 0x2856fec `ldr x10,[x22,#32]`    ; x2 source
- 0x2856ff0 `ldr x0,[x9,#16]  `    ; x0 = [vt+16]
- 0x2856ff4 `mov x1,x19      `     ; consumer
- 0x2856ff8 `mov x3,x22      `     ; node
- 0x2856ffc `mov w4,#4       `     ; type code 4
- 0x2857000 `mov x5,#0       `
- 0x2857004 `and x2,x10,#1   `     ; x2 = [node+32]&~1
- 0x2857008 `blr x8          `     ; dispatch
Regression: `deque_dispatch_node_layout_matches_engine_abi` pins the offsets
(node+112 vt base, node+40 guard, node+32 arg, vt+16 a0, vt+40 handler, w4=4,
x5=0) and the round-trip, so any render-task injector builds nodes the running
drain interprets correctly. Workspace 468/0.

## --deque-probe <ctx-qw-hex> (elfjit, opt-in)
Arm this by repointing the LIVE sentinel's `[node+112]` -> a fake vtable we
allocate (host heap, `[vt+16]=ctx`, `[vt+40]=registered host-thunk`). The 
engine's own pop-loop then dispatches that NODE through OUR handler + real ABI
args — the "discriminating type-4 dispatch of OUR node" SH7b demanded. Only
engages with `--drain-force-pop`; plain `--drain-poll`/`--jni`/stable-idle
paths byte-for-byte unchanged (verified: JNI exit 0, stable-idle exit 124,
zero SIGSEGV; workspace green 468/0).

## Why it doesn't fire (the honest wall)
`REPOINTED` both sentincls (headcells 0x10682b338 / 0x10682a638), but then an
UNTracked host thread (guestpc=0x0, rbx_matches=false) faults reading
`0x7f0000000090` (= HOST_THUNK_BASE + slot 18) as a pointer. Under forced-pop
the engine dispatches the SENTINEL AS A TASK (the drain re-enqueues every popped
node; the sentinel is the drain's OWN struct, [node+112]=0x106829f00 -> [vt+40]
= the engine dispatcher 0x10285371c). Repointing [node+112] midway races the
in-flight dispatcher: the dispatch handler walks the sentinel's OTHER garbage
task fields (which still point at slot-18 thunk addresses etc.) before our
vtable is through. So the sentinel-as-task dispatch crashes regardless — the
vtable repoint alone cannot detour the engine's own handler side-effects.

## Next lever (what this cycle proves must change)
A real render task needs a node whose vt+40 handler is a REAL engine render
callback AND whose task payload (the fields the handler walks) is coherent — the
sentinel's is not. The two SH7b candidates remain, now with the ABI pinned:
1. Invoke the REAL producer 0x285682c as a guest call with a valid task node
   (recover scheduler `this` = drain_struct [x1+104]) so the framework's own
   push path runs — the cleanest, avoids hand-fabricating the task.
2. Build a full task node (vt+40 = a real render/tick vtable we must locate,
   payload coherent) and inject/cross before force-pop races it.

## Files
- crates/arm64jit/examples/elfjit.rs: `--deque-probe <ctx>` (live sentinel
  vtable repoint to host-thunk) — opt-in.
- crates/arm64jit/src/jit.rs: `deque_dispatch_node_layout_matches_engine_abi`
  (ABI pin regression).
- Run-logs: /home/hermes-worker/runs/boot-probe-{1..5,final}.txt
  (REPOINTED both sentinels; exit 134).