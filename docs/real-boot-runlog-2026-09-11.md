# Real Roblox libroblox.so boot — canonical run log (this cycle)

Date: 2026-09-11, on the dev VPS.
Command:
    cd /home/hermes-worker/runs/open-sober
    cargo build -p arm64jit --example elfjit
    timeout 120 ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 --jni

Binary: Roblox 2.738.1397 `lib/arm64-v8a/libroblox.so` (extracted via
`sober-core::apk::extract_libs`), 101MB ELF ARM aarch64, Android 26, NDK r28c.
Guest image base 0x100000000, spans [0x100000000, 0x107333c3c).

## What this log demonstrates

The real Roblox engine's **main-thread JNI_OnLoad runs to completion and returns
0x10006 (JNI_VERSION_1_6, the canonical success value)** every run, after:

1. load_elf_image + writable guest-tail reservation (crashpad telemetry static
   table walks ~36MB past the last PT_LOAD bss);
2. 534 R_AARCH64_JUMP_SLOT + 63 GLOB_DAT bound to host thunks;
3. static seeding of never-constructed C++ singletons (empty .init_array):
   - TLS big-alloc 0x1d9801c -> host calloc(x1),
   - LSM map allocator 0x1d97744 -> host calloc(x0),
   - LSM static hash-map 0x726f8c0 seeded two-level empty map (+NOP init store
     0x1d975f8),
   - JNICallProtocol singleton ptr 0x7333948 -> object 0x7333950;
4. JNIMain logging reached: `TelemetryProtocol::setProcessTimeOverride`,
   `DeviceStaticParams is null.`;
5. JNI_OnLoad returns 0x10006.

## Full output (representative run)

[plt] patched __stack_chk_guard GOT 0x631aa30 (0x0) -> 0x5628c32de568
[plt] bound 534 JUMP_SLOT + 63 GLOB_DAT/ABS64 (0 unresolved), 15 unresolved
PLT imports bound: 534 to host thunks (0 unbound)
[mempool] big-alloc 0x1d9801c routed to host calloc (thunk @ 0x1062d9000)
loaded '/home/hermes-worker/.cache/open-sober/robbox/libroblox.so': is_pie=true base_load_vaddr=0x0 e_entry=0x100000000
  segment guest=[0x100000000,0x1062d8190) host=same prot=r-x
  segment guest=[0x1062dc1c0,0x1067d3000) host=same prot=rw-
  segment guest=[0x1067d67c0,0x107333c3c) host=same prot=rw-
[tail] reserved 402653184B guest RW tail @0x107334000
[lsm-map] allocator 0x1d97744 routed to host calloc(x0) (thunk @ 0x1062d9800)
[lsm-map] NOP'd LSM init store at 0x101d975f8 (guest 0x1d975f8) to keep seeded empty map
[lsm-map] seeded static empty LocalStorageManager map: global 0x10726f8c0 -> bucket array 0x... (4194304 buckets)
[JNICall-singleton] seeded ptr 0x107333948 -> object 0x107333950 (zeroed bss ~ PTHREAD_MUTEX_INITIALIZER at +8)
[lsm-map] readback global 0x10726f8c0 = 0x... (must be non-zero)
running entry guest=0x102173ff4 host=0x102173ff4 (segment base guest=0x100000000 size=0x7333c3c)
guest sp=0x... tls(tpidr)=0x...
JNI boot: x0 = JavaVM* 0x...
[roblox:JNIMain] TelemetryProtocol::setProcessTimeOverride: %ld
[roblox:JNIMain] DeviceStaticParams is null.
JIT(no-QEMU) entry() -> 65542 (0x10006)
[then, after main returns, a worker guest thread spawned during JNI_OnLoad
 (pthread_create start_routine=0x284d168, tid=1) crashes deterministically:
[SIGSEGV] tid=<host> fault=0xffffffffa90357f7 rip=0x10284d6be guestpc=0x747361636461
 ... register file decodes to ASCII libc symbol-name strings ("pthread_*",
 "memset", "pthread_cond_*", "broadcast"): the guest branch/call landed on
 symbol-name data in .dynstr. This is the NEXT wall (worker-thread JIT call
 through a GOT/PLT slot or vtable entry whose target is a symbol-name string,
 not committed code). It is deterministic and predates this cycle's changes.]

## Next wall (also in STATUS.md / HANDOFF.md top)

The spawned worker guest thread (tid=1, from Roblox's pthread_create during
JNI_OnLoad) faults deterministically with a register file of symbol-name
strings (guestpc "adcast" = .dynstr "broadcast"). 15 GLOB_DAT were left
unresolved (0x67ca950, 0x67cf398-0x67cf3f0, 0x67d0140/48) — candidates for the
indirect-branch-into-string. Next: bind/seed those GOT slots, or a vtable entry,
so the worker's indirect call lands on real code.
## Append (cycle SH, 2026-09-12) — decoder 100% closed; boot boundary re-verified

arm64jit decode now reports **ZERO Unsupported / ZERO PANIC across the whole
libroblox.so .text span** (11,437 -> 0). The decoder is permanently done.

Fresh run confirms the boot wall is unchanged and now precisely characterized.

**Idle wait function (guessed name: engine lamport-style futex barrier), entry
0x10284d018:**

- Preamble: `bl 0x102b9e9b0(x0=1, x1=x19)` — the ARM64 atomics/re-arm helper
  (ldaddal when a 0x10683b000+0xa58 flag is set, else ldaxr/stlxr add; x0 is
  added to [x1]). This is the `x19` counter the parked futex word sits at.
- `cmp x21, x0, lsr#32 / b.ne 0x10284d0ec` — compare a version/hi word.
- `cmn x20, #-1 / b.eq 0x10284d114` — if x20 (target/progress bound) == -1,
  take the **infinite wait path** 0x10284d114; else compute a bounded wait
  (x20*1000 ns, msub magic) and go through the same futex.
- **Parked site lr=0x10284d134** (all three threads): below `bl 0x1062d62d0`
  (futex syscall thunk). The regs (x0=0x62 syscall FUTEX, x1=0x7fdf..8cc =
  `x19+4` futex word, x2=0x89, x3=w21=0xF4240 expected, x6=-1) show a
  `FUTEX_WAIT_BITSET`/WAKE on the per-thread futex word at `x19+4` awaiting
  the guest's "go" value.
- Loop body: after futex returns, `mov w20,wzr; b 0x10284d0f0` -> `mov x0,#-1;
  mov x1,x19; bl 0x102b9e9b0` (re-arm: add -1, i.e. decrement) -> ret.
- The `0x10284d0e4 cmp x8,x20 / b.cs 0x10284d13c` at the top is the bounded-
  wait variant that proceeds to "work" when a progress counter reaches x20.

**Why --futex-kick cannot advance it:** the kicker writes values 0x1528+
(its own counter) to the futex word, but the guest latches await a specific
"go" semantics: 0xF4240 is the spare state, and a *released* thread re-arms
(x19 -= 1) then re-parks until the guest's OWN producer flips the latch to a
"work available" value. The kicker is a spurious releaser; it can neither
produce guest work nor teach the producer to run. To cross this gate the JIT
must surface the engine's producer path (the thread that enqueues onto these
per-thread latches) — the true next lever, now that decode is 100%.

Thread census at idle: guest tid 0 (owner, host 852481), tid 1 (852482),
tid 2 (852483) ALL park at lr=0x10284d134 on their own futex word. 3 threads,
all waits; the producer thread is not among them (either never spawned or
blocked earlier upstream — next trace: which thread spawns/feeds these
latches).
