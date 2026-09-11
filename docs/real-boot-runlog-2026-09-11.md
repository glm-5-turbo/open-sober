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