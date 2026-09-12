//! Integration spike: load an aarch64 ELF (static non-PIE OR PIE/ET_DYN) with
//! libloader's `load_elf_image`, which lays every PT_LOAD into one contiguous
//! kernel-chosen mapping so **guest vaddr == host address**, then run the entry
//! function through the in-process arm64jit translator — NO QEMU.
//!
//! Build a test ELF with:
//!   cat > t.c <<'EOF'
//!   int entry(void){ return 42; }
//!   EOF
//!   aarch64-linux-gnu-gcc -static -nostdlib -Wl,-e,entry t.c -o tiny.elf
//!
//! Run with: cargo run -p arm64jit --example elfjit -- /path/to/tiny.elf [entry-guest-addr-hex]
//!
//! Because guest==host, the `entry` you pass is BOTH the guest virtual address
//! of the first instruction and (==) its host address; ADRP/ADR of globals and
//! guest loads/stores dereference the correct host pointers directly.

use arm64jit::jit::{CpuState, jit_run};
use arm64jit::shims::set_anativewindow_xid;
use input_wrapper::x11;

// Diagnostic: on a host SIGSEGV inside a translated block, print the guest PC
// (CpuState.pc, offset 256) + a few guest regs read from the CpuState (RBX).
// elfjit is a diagnostic binary, so this stays in.
unsafe fn install_fault_debug() {
    extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
        unsafe {
            let uc = ctx as *const libc::ucontext_t;
            let rbx = (*uc).uc_mcontext.gregs[libc::REG_RBX as usize];
            let rip = (*uc).uc_mcontext.gregs[libc::REG_RIP as usize];
            let pc = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(256) as *const u64) } else { 0 };
            let x0 = if (rbx as usize) & 7 == 0 { *(rbx as *const u64) } else { 0 };
            let x1 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(8) as *const u64) } else { 0 };
            let x2 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(16) as *const u64) } else { 0 };
            let x3 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(24) as *const u64) } else { 0 };
            let x4 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(32) as *const u64) } else { 0 };
            let x5 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(40) as *const u64) } else { 0 };
            let x6 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(48) as *const u64) } else { 0 };
            let x7 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(56) as *const u64) } else { 0 };
            let x8 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(64) as *const u64) } else { 0 };
            let x9 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(72) as *const u64) } else { 0 };
            let sp = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(248) as *const u64) } else { 0 };
            let fault = (*info).si_addr() as u64;
            // Dump the raw host bytes around the faulting translated x86 so the
            // memory-op (e.g. a `mov rax,[rax+0x30]` = guest `ldr x8,[x8,#48]`)
            // can be identified precisely even though CpuState.pc is coarse.
            let mut raw = [0u8; 48];
            std::ptr::copy_nonoverlapping(rip.wrapping_sub(24) as *const u8, raw.as_mut_ptr(), 48);
            let hex = raw.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            let x10 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(80) as *const u64) } else { 0 };
            let x19 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(152) as *const u64) } else { 0 };
            let x20 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(160) as *const u64) } else { 0 };
            let x21 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(168) as *const u64) } else { 0 };
            let x22 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(176) as *const u64) } else { 0 };
            let x23 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(184) as *const u64) } else { 0 };
            let x28 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(224) as *const u64) } else { 0 };
            let x29 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(232) as *const u64) } else { 0 };
            let x30 = if (rbx as usize) & 7 == 0 { *(rbx.wrapping_add(240) as *const u64) } else { 0 };
            let name = if sig == libc::SIGSEGV { "SIGSEGV" } else if sig == libc::SIGILL { "SIGILL" } else { "SIGFAULT" };
            let tid = unsafe { libc::syscall(libc::SYS_gettid) };
            // Does the faulting host thread actually run a guest CpuState?
            // Compare the ucontext RBX against the registered CpuState pointer
            // for this host tid. If they match, the guest reg dump is real; if
            // not, RBX is an arbitrary host value and the dump is garbage.
            let reg_state = arm64jit::jit::guest_state_of_host(tid as i64);
            let state_matches = reg_state != 0 && reg_state == rbx as u64;
            // Guest tid of the matching registered state (0 if unregistered).
            let reg_guest_tid = if reg_state != 0 {
                unsafe { *(reg_state as *const u64).wrapping_add(848 / 8) } // CpuState.tid field
            } else {
                u64::MAX
            };
            // Enumerate all registered guest threads: (host_tid, guest_tid, state).
            let thr = arm64jit::jit::dump_guest_threads()
                .iter()
                .map(|(h, g, s)| format!("({h}:{g},{s:#x})"))
                .collect::<Vec<_>>()
                .join(" ");
            let in_jit = arm64jit::jit::in_jit_run();
            // Diagnostic: dump the LocalStorageManager static-map global on fault
            // to confirm whether our boot-time seed persisted.
            let lsm_global = unsafe { *(0x10726f8c0u64 as *const u64) };
            // Full host x86-64 register file (SysV). The guest regs above are read
            // through rbx==CpuState base, so if rbx itself is corrupt the guest
            // dump is an artifact; the host frame disambiguates a real guest fault
            // from a handler/spurious read.
            let g = |i: usize| (*uc).uc_mcontext.gregs[i];
            let (rax, rcx, rdx, rsi, rdi, rbp, rsp, r8, r9, r10, r11, r12, r13, r14, r15, fl) = (
                g(libc::REG_RAX as usize), g(libc::REG_RCX as usize), g(libc::REG_RDX as usize),
                g(libc::REG_RSI as usize), g(libc::REG_RDI as usize), g(libc::REG_RBP as usize),
                g(libc::REG_RSP as usize), g(libc::REG_R8 as usize), g(libc::REG_R9 as usize),
                g(libc::REG_R10 as usize), g(libc::REG_R11 as usize), g(libc::REG_R12 as usize),
                g(libc::REG_R13 as usize), g(libc::REG_R14 as usize), g(libc::REG_R15 as usize),
                g(libc::REG_EFL as usize),
            );
            let s = format!(
                "\n[{name}] tid={tid} fault={fault:#x} rip={rip:#x} guestpc={pc:#x} rbx_matches_gueststate={state_matches} (reg_state={reg_state:#x}, tid={reg_guest_tid})\n  x0={x0:#x} x1={x1:#x} x2={x2:#x} x3={x3:#x} x4={x4:#x}\n  x5={x5:#x} x6={x6:#x} x7={x7:#x} x8={x8:#x} x9={x9:#x} sp={sp:#x}\n  x10={x10:#x} x19={x19:#x} x20={x20:#x} x21={x21:#x} x22={x22:#x}\n  x23={x23:#x} x28={x28:#x} x29={x29:#x} lr(x30)={x30:#x} lsm_map_global=0x{lsm_global:x}\n  HOST rax={rax:#x} rbx={rbx:#x} rcx={rcx:#x} rdx={rdx:#x} rsi={rsi:#x} rdi={rdi:#x}\n  HOST rbp={rbp:#x} rsp={rsp:#x} r8={r8:#x} r9={r9:#x} r10={r10:#x} r11={r11:#x}\n  HOST r12={r12:#x} r13={r13:#x} r14={r14:#x} r15={r15:#x} eflags={fl:#x}\n  GUEST_THREADS {thr} in_jit_run={in_jit}\n  raw[]= {hex}\n"
            );
            let b = s.as_bytes();
            libc::write(2, b.as_ptr() as *const libc::c_void, b.len());
            // Native frame-pointer backtrace (SysV: rbp chain, [rbp]=prev rbp,
            // [rbp+8]=return addr). Classifies every ret addr as host-JIT vs
            // guest-text vs libc so we see WHICH dispatcher path jumped to guest.
            let mut btd = String::from("\n  BT:");
            let mut fp: u64 = rbp as u64;
            let classify = |ra: u64| -> String {
                if ra >= 0x100000000 && ra < 0x120000000 {
                    format!("GUEST({ra:#x})")
                } else if ra >= 0x7f0000000000 && ra < 0x7f8000000000 {
                    format!("HOST({ra:#x})")
                } else if ra >= 0x7f0000000000 {
                    format!("HOST({ra:#x})")
                } else {
                    format!("{ra:#x}")
                }
            };
            for _ in 0..24 {
                if fp & 7 != 0 || fp < 0x400000 || fp >> 56 != 0 {
                    break;
                }
                let ra = unsafe { *(fp.wrapping_add(8) as *const u64) };
                if ra == 0 {
                    break;
                }
                btd.push_str(&format!(" -> {}", classify(ra)));
                let nfp = unsafe { *(fp as *const u64) };
                if nfp <= fp || nfp - fp > 0x4000 {
                    break;
                }
                fp = nfp;
            }
            btd.push('\n');
            libc::write(2, btd.as_bytes().as_ptr() as *const libc::c_void, btd.len());
        }
        std::process::abort();
    }
    for sig in [libc::SIGSEGV, libc::SIGILL] {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handler as usize;
        sa.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(sig, &sa, std::ptr::null_mut());
    }
}

/// How a single `--kicker` drives its target guest global.
#[derive(Clone, Copy)]
enum KickerMode {
    /// `pthread_cond_broadcast` the address every tick.
    Broadcast,
    /// Write an exact u64 value every tick (`--kicker 0xADDR=0xVAL`).
    Fixed(u64),
    /// Historical lifecycle pulse: write 1 through the first gate, then 2.
    Pulse,
}

fn main() {
    unsafe {
        install_fault_debug();
    }
    let path = std::env::args()
        .nth(1)
        .expect("usage: elfjit <aarch64-elf> [entry-guest-addr-hex]");
    let entry_arg = std::env::args().nth(2);

    let el = unsafe { libloader::elf::load_elf_image(std::path::Path::new(&path)) }
            .expect("load_elf_image");

        // Fold the import resolver + host shims into the boot path: bind every PLT
        // JUMP_SLOT GOT slot to a host thunk so translated Roblox `blr`s hit real
        // host functions (libc/libm/float/bionic/graphics-stub) instead of stalling.
        let (nbound, nunresolved) = arm64jit::plt::bind_image_plt(&el, None);
        if nbound > 0 {
            println!("PLT imports bound: {nbound} to host thunks ({} unbound)", nunresolved);
        }

    // Route the TLS-block allocator's big-allocation path to host calloc so the
    // unseeded MemoryPool empty-free-list returns a real buffer instead of a
    // NULL+abort. Site 0x1d9801c is the big allocator of Roblox v2.738.1397's
    // per-thread TLS block (reachable from 0x1d96a40's empty free-list tail).
    match arm64jit::jit::route_mempool_big_alloc_to_host(el.guest_of(0x1d9801c), 0x1_0000_0000) {
        Ok(tp) => {
            println!("[mempool] big-alloc 0x1d9801c routed to host calloc (thunk @ {tp:#x})");
            let p = tp as *const u8;
            let hex: Vec<String> = (0..20).map(|i| unsafe { format!("{:02x}", *p.add(i)) }).collect();
            println!("[mempool] thunk bytes: {} (JIT-readable via guest image)", hex.join(" "));
        }
        Err(e) => println!("[mempool] warn: big-alloc patch skipped: {e}"),
    }

    // Guest entry: the ELF's own e_entry (already relocated to guest space by
    // load_elf_image) unless a link-time address is supplied, in which case we
    // translate it to guest/runtime space with guest_of().
    let entry = match entry_arg {
        Some(h) => {
            let link = u64::from_str_radix(h.trim_start_matches("0x"), 16)
                .unwrap_or_else(|e| panic!("bad entry hex: {e}"));
            el.guest_of(link)
        }
        None => el.info.entry,
    };

    println!(
        "loaded '{}': is_pie={} base_load_vaddr=0x{:x} e_entry=0x{:x}",
        path, el.info.is_pie, el.info.base_load_addr, el.info.entry
    );
    for s in &el.segments {
        println!(
            "  segment guest=[0x{:x},0x{:x}) host=same prot={}{}{}",
            s.guest_vaddr,
            s.guest_vaddr + s.memsz,
            if s.prot.read { "r" } else { "-" },
            if s.prot.write { "w" } else { "-" },
            if s.prot.execute { "x" } else { "-" }
        );
    }

    // Pick the executable (text) segment to translate code out of.
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .expect("no executable segment");
    let base = seg.guest_vaddr; // == host addr of image[0] (guest==host)

    // Use the FULL mapped span (every PT_LOAD + inter-segment gaps, which
    // load_elf_image lays into ONE contiguous anonymous region at the fixed
    // base) as the valid-pc extent. The guest may legitimately branch/call
    // into higher sections (data-backed trampolines, .bss-slotted function
    // pointers) that live past the r-x slice; bounding `run_loop` to only the
    // text slice wrongly flags those as "outside image". Compute the extent as
    // the largest guest_vaddr+memsz across segments (the whole mmap is zero-
    // filled), relative to this text-segment base.
    let full_end = el
        .segments
        .iter()
        .fold(0u64, |m, s| m.max(s.guest_vaddr + s.memsz));
    let len = (full_end - base) as usize;

    // Reserve a writable guest tail past the ELF's mapped span. Real Roblox
    // `nativeInitCrashpad` walks a link-time `& bss` telemetry table base by a
    // slot index that reaches tens of MB past the last PT_LOAD `.bss` end; on
    // real Android that adjacent memory is mapped anonymous, our loader maps
    // only the ELF span. Reserve 384MB of RW headroom so the deep table writes
    // (and other large guest tables/arenas) have real backing instead of
    // SIGSEGV. MAP_FIXED at a page-aligned address after base+len is safe
    // (host heap/stack live elsewhere); must start page-aligned or mmap EINVALs.
    let tail_start = (full_end as usize + 0xfff) & !0xfff;
    const TAIL_SIZE: usize = 384 * 1024 * 1024;
    match libloader::elf::reserve_guest_tail(tail_start, TAIL_SIZE) {
        Ok(_s) => println!("[tail] reserved {TAIL_SIZE}B guest RW tail @0x{tail_start:x}"),
        Err(e) => eprintln!("[tail] warn: guest-tail reserve skipped: {e}"),
    }

    // Route the LocalStorageManager static hash-map's bucket-array allocator
    // (`0x1d97744`, receives its byte size in x0) to host calloc, so the
    // unseeded per-object MemoryPool empty free-list returns a real zeroed
    // buffer. The map's lazy init then stores a valid non-NULL bucket array
    // into its header global (0x726f8c0) instead of NULL, so the hash-lookup
    // reader (0x1d99e30: `ldr x8,[0x726f8c0]; ...; ldar x8,[x8]; ldr x0,[x8,idx<<3]`)
    // finds a real map instead of derefing a NULL bucket.
    match arm64jit::jit::route_allocator_x0_to_calloc(el.guest_of(0x1d97744), 0x1_0000_0000) {
        Ok(tp) => println!("[lsm-map] allocator 0x1d97744 routed to host calloc(x0) (thunk @ {tp:#x})"),
        Err(e) => eprintln!("[lsm-map] warn: allocator route skipped: {e}"),
    }

    // Disable the LSM map's lazy-init store that would clobber our seed.
    //
    // The init at 0x1d975f8 does `str x0,[x22,#2240]` writing its (routed)
    // allocator result into the map global 0x726f8c0, then memsets and builds a
    // two-level bucket structure into that discarded buffer. The reader
    // (0x1d99e40/0x1d99e4c/0x1d99e50) instead requires the global to point at a
    // bucket array whose every slot (key>>29) is a pointer to a zeroed
    // sub-array (indexed by (key>>16)&0x1fff); a bare all-zero calloc leaves
    // bucket slots NULL and the reader derefs NULL. `seed_static_empty_map`
    // below constructs exactly the required layout, so NOP the init store to
    // keep that seed authoritative. Guest insn -> NOP (0xd503201f).
    let init_store = el.guest_of(0x1d975f8);
    {
        let page = init_store & !0xfff;
        if unsafe { libc::mprotect(page as *mut libc::c_void, 4096, libc::PROT_READ | libc::PROT_WRITE) } == 0 {
            unsafe { *(init_store as *mut u32) = 0xd503_201fu32 };
            unsafe { libc::mprotect(page as *mut libc::c_void, 4096, libc::PROT_READ | libc::PROT_EXEC) };
            println!("[lsm-map] NOP'd LSM init store at 0x{init_store:x} (guest 0x{:x}) to keep seeded empty map", 0x1d975f8);
        } else {
            eprintln!("[lsm-map] warn: could not mprotect init-store page RW");
        }
    }

    // Seed the LocalStorageManager C++ static hash-map whose .bss base global
    // (guest 0x10726f8c0 = 0x726f000+0x8c0) is 0 because its constructor never
    // ran (.init_array is empty). The reader (0x1d99e40) does
    //   ldr x8,[0x726f8c0]; add x8,x8,key>>29<<3; ldar x8,[x8]; ldr x0,[x8,idx<<3]
    // and faults on the NULL bucket (`key` is a heap/guest pointer, so
    // key>>29 lands up to ~0xb00 buckets in). Seed a real zeroed region as the
    // bucket array, with every bucket slot pointing at a shared (also-zeroed,
    // non-overlapping) sub-array slot, so any lookup reads 0 -> "not found".
    // (Fallback: if the guest later overwrites it with a real map, all the better.)
    unsafe fn seed_static_empty_map(map_global_guest: u64) {
        const BUCKETS: usize = 0x400000; // key>>29: pointers ~0x56.. give idx ~0x2b2a7; grant headroom
        const SLOT_STRIDE: usize = 8;
        const SLOT_REGION: usize = 0x10000; // (key>>16)&0x1fff max index * 8
        let buckets_bytes = BUCKETS * SLOT_STRIDE;
        let total = buckets_bytes + SLOT_REGION;
        let buf = Box::leak(vec![0u8; total].into_boxed_slice());
        let bufp = buf.as_mut_ptr();
        let base = bufp as u64;
        let sub = base + buckets_bytes as u64;
        // Every bucket slot points at `sub` (a zeroed shared sub-array), so
        // `ldar x8,[bucket[key>>29]]` returns a non-null pointer and
        // `ldr x0,[x8, idx<<3]` reads 0 -> cbz -> return NULL (not found).
        let slots = std::slice::from_raw_parts_mut(bufp.cast::<u64>(), BUCKETS);
        for s in slots {
            *s = sub;
        }
        *(map_global_guest as *mut u64) = base;
        println!("[lsm-map] seeded static empty LocalStorageManager map: global 0x{map_global_guest:x} -> bucket array 0x{base:x} ({} buckets, shared zero sub @0x{sub:x})", BUCKETS);
    }
    fn link_to_guest(el0: &libloader::elf::LoadedElf, link: u64) -> u64 {
        el0.guest_of(link)
    }
    unsafe { seed_static_empty_map(link_to_guest(&el, 0x726f000 + 0x8c0)) };

    // Seed the JNICallProtocol-ish refcounted-singleton pointer at guest
    // 0x107333948 (link-time 0x7333000+0x948). The once-init atomic store
    // (0x2b9e890) normally writes the object address 0x7333950 into that slot;
    // under the JIT the .init_array never runs so it stays 0, and the acquire
    // path (0x21daf00) locks `this+8` (a pthread_mutex at object+0x8) — with
    // `this` NULL it calls pthread_mutex_lock(0x8) and faults. The object
    // itself is zeroed bss (a valid PTHREAD_MUTEX_INITIALIZER at +8), so just
    // wiring the pointer releases the lock into the zeroed (== initial, unlocked)
    // mutex.
    let singleton_slot = link_to_guest(&el, 0x7333000 + 0x948); // [ptr] slot
    let singleton_obj = link_to_guest(&el, 0x7333000 + 0x950); // object base
    unsafe { *((singleton_slot) as *mut u64) = singleton_obj };
    println!(
        "[JNICall-singleton] seeded ptr 0x{singleton_slot:x} -> object 0x{singleton_obj:x} (zeroed bss ~ PTHREAD_MUTEX_INITIALIZER at +8)"
    );
    // Read back the seed to confirm it landed where the guest reads it.
    let g_chk = link_to_guest(&el, 0x726f000 + 0x8c0);
    let v_chk = unsafe { *(g_chk as *const u64) };
    println!("[lsm-map] readback global 0x{g_chk:x} = 0x{v_chk:x} (must be non-zero)", );

    println!(
        "running entry guest=0x{:x} host=0x{:x} (segment base guest=0x{:x} size=0x{:x})",
        entry, entry, base, len
    );

    // image = the executable segment's bytes. Because guest==host, the `base`
    // passed to compile_image is the guest address of image[0] and the `entry`
    // is the guest address of the first instruction to run.
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, len) };
    let mut st = CpuState::new();

    // Optional x0/x1/x2 init. Pass `buf` in position 3 to allocate a
    // writable 256-byte host buffer (guest==host, so its address is a valid
    // guest pointer) and put its address in x0; also x1=x0+32. Even when the
    // guest is a real binary we don't yet bootstrap (no TLS/stack), this lets
    // small aarch64 test functions run through the dispatcher.
    for (i, arg) in std::env::args().skip(3).take(3).enumerate() {
        if arg == "--jni" || arg == "buf" || arg == "--startapp" {
            let v = if arg == "buf" {
                let b = Box::leak(vec![0x7fu8; 256].into_boxed_slice());
                if i == 0 {
                    let base = b.as_ptr() as u64;
                    st.set(0, base);
                    st.set(1, base + 32);
                }
                b.as_ptr() as u64
            } else {
                0 // --jni / --startapp aren't x-register values; handled separately
            };
            if arg == "buf" && i == 0 {
                continue;
            }
            let _ = v;
        } else {
            let v = u64::from_str_radix(arg.trim_start_matches("0x"), 16)
                .unwrap_or_else(|e| panic!("bad x{i} hex: {e}"));
            st.set(i, v);
        }
    }

    // Bootstrap a guest runtime the binary can actually use -------------
    // (1) Guest stack: allocate a real writable region (guest==host addressing,
    //     so its host pointer is a valid guest pointer) and point SP at the top.
    // (2) TLS base: point CpuState.tpidr at a writable region so `mrs tpidr_el0`
    //     returns a non-zero, writable base (FS/GS-style thread pointer).
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let stack = Box::leak(vec![0u8; STACK_SIZE].into_boxed_slice());
    // Lay out a real kernel-style initial stack (argc/argv/envp/auxv) so glibc
    // IFUNCs resolve to scalar paths instead of reading garbage auxv into SMP
    // (which drove the JIT into an unsupported `str za` wall). No SME/SVE bits.
    let mut auxv = arm64jit::boot::standard_auxv(
        &el,
        arm64jit::boot::HWCAP_FP | arm64jit::boot::HWCAP_ASIMD,
        0,
    );
    let sp = arm64jit::boot::layout_initial_stack(
        stack.as_ptr() as *mut u8,
        STACK_SIZE,
        Some(&[0u8; 0]), // argv[0] (empty) — keeps argc==1 like a real shell exec
        &[],
        &mut auxv,
    );
    st.set(31, sp); // x31 = SP (points at argc on the initial stack)
    const TLS_SIZE: usize = 1024 * 64;
    let tls = Box::leak(vec![0u8; TLS_SIZE].into_boxed_slice());
    // Seed the guest TLS region from the image's PT_TLS (local-exec/initial-exec
    // thread-locals) and point tpidr_el0 at the AArch64 TCB (16 bytes before the
    // module's TLS block). `__thread` globals then read/write real data.
    st.tpidr = libloader::elf::setup_guest_tls(
        &el.info,
        std::path::Path::new(&path),
        tls.as_ptr() as *mut u8,
        TLS_SIZE,
    )
    .expect("setup_guest_tls");
    // Publish the main thread's TLS block as the template that spawned guest
    // threads (pthread_create/clone children) clone per-thread, so their
    // `__thread` locals and TP-indexed tables match the main thread instead of
    // a bare zeroed buffer.
    arm64jit::jit::publish_guest_tls_template(tls.as_ptr() as u64, TLS_SIZE);
    println!("guest sp=0x{:x} tls(tpidr)=0x{:x}", sp, st.tpidr);

    // JNI boot mode: hand the guest a guest-visible JavaVM* in x0 (as the Android
    // runtime would). Pass `--jni` to set x0 = vm. If x0/x1/x2 were already
    // supplied via positional args they win (we don't clobber a caller's x0).
    if std::env::args().any(|a| a == "--jni") && st.x[0] == 0 {
        let (_env, vm) = arm64jit::jni::build_jni();
        st.x[0] = vm; // JNI_OnLoad(JavaVM* vm, void* reserved) -> x0 = vm
        println!("JNI boot: x0 = JavaVM* 0x{:x}", vm);
    }

    // PC-driven dispatcher: compiles reachable regions and re-enters on
    // indirect branch (`blr`) / `br` / `ret`, so real (blr-heavy) Roblox code
    // can actually *execute* rather than stopping at the first blr.
    match jit_run(image, base, entry, &mut st as *mut CpuState) {
        Err(e) => {
            eprintln!("arm64jit run_loop stopped: {e}");
            std::process::exit(1);
        }
        Ok(r) => {
            // stderr is unbuffered; if this line appears BEFORE the SIGSEGV dump,
            // the fault is in post-run teardown, not the boot loop itself.
            eprintln!("[elfjit] jit_run returned Ok({r:#x}) — entering post-run phase");
            println!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
        }
    }
    // Let spawned worker guest threads (pthread_create/clone children started
    // during boot) run to completion before the process exits, so jit_run on a
    // detached child isn't torn down mid-translation (which surfaces as a
    // SIGSEGV reading a freed child CpuState as 'registers'). Wait for the
    // active-guest-thread count to return to the baseline (main only).
    let baseline = 1;
    for _ in 0..400 {
        if arm64jit::jit::active_guest_threads() <= baseline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    // --startapp <link-addr>: after JNI_OnLoad completes, drive the next real
    // boot stage — the Java side's `nativeAppBridgeV2StartAppWithParams` (the
    // entry that creates the engine main loop + EGL/GLES context). We chain it
    // as a fresh guest entry after the registration phase, giving it the same
    // JNIEnv in x0 plus fake but VALID (non-null, dereferenceable) jobject /
    // jstring handles, exactly as the real JVM would. Captures how far the real
    // binary gets into StartApp (main-loop / graphics init) before the next wall.
    if let Some(hex) = {
        let args: Vec<String> = std::env::args().collect();
        args.iter()
            .position(|a| a == "--startapp")
            .and_then(|i| args.get(i + 1).cloned())
    } {
        // Reuse the singleton env/vm; build a fake-but-valid jobject (a 5-word
        // object header) and a jstring handle containing the StartApp params JSON.
        let (env_ptr, _vm) = arm64jit::jni::build_jni();
        let activity = arm64jit::jni::new_fake_object(); // non-null jobject
        let params = arm64jit::jni::new_string_utf_handle(b"{\"key\":\"\"}");
        let link = u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("bad --startapp hex"));
        let start_app = el.guest_of(link);
        eprintln!("[elfjit] driving StartApp @ guest {start_app:#x} after JNI_OnLoad (env={env_ptr:#x} jobject={activity:#x} params={params:#x})");
        let mut s2 = arm64jit::jit::CpuState::new();
        s2.tpidr = arm64jit::jit::current_guest_tp();
        // Continue on the boot-phase guest stack (real SP), not a fresh 0 —
        // StartApp's prologue `sub sp,#0xf0` would otherwise wrap to 0xffff..10
        // and the frame-write faults. The Java side enters natives on whatever
        // thread is current; elfjit reuses the main guest thread's SP.
        s2.x[31] = st.x[31]; // guest SP
        s2.x[0] = env_ptr;
        s2.x[1] = activity;
        s2.x[2] = params;
        // Concurrent guest-thread state sampler (JIT_THREADS=1). StartApp's
        // `jit_run` parks the main thread forever (the engine main-loop
        // lifecycle-await), so a post-run sampler would never run. Instead
        // spawn a detached host sampler that polls `snapshot_threads()`
        // every ~200 ms for a bounded window, dumping each parked thread's
        // hostcall slot (pc), guest call-site (x30/lr) and wait-object args
        // (x0..x2). This pins the boot wall to the exact guest function that
        // blocks and what it awaits. Runs concurrently with the jit_run.
        if std::env::var_os("JIT_THREADS").is_some() {
            std::thread::spawn(|| {
                for it in 0..100 {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    let (c, h) = arm64jit::jit::block_cache_stats();
                    eprintln!("[elfjit:stats] it={it} compiles={c} hits={h}");
                    let snaps = arm64jit::jit::snapshot_threads();
                    for t in &snaps {
                        let at = arm64jit::resolver::name_of_call_addr(t.pc)
                            .unwrap_or_else(|| format!("{:#x}", t.pc));
                        eprintln!(
                            "  host_tid={} guest_tid={} pc={at} lr={:#x} x0={:#x} x1={:#x} x2={:#x} x19={:#x}[*={:#x}] x29={:#x} sp={:#x}",
                            t.host_tid, t.guest_tid, t.lr, t.x0, t.x1, t.x2, t.x19,
                            // deref predicate pointer if it looks valid (guest rw segment)
                            if (0x100000000..0x108000000).contains(&t.x19) && t.x19 & 7 == 0 { unsafe { *(t.x19 as *const u64) } } else { 0 },
                            t.x29, t.sp
                        );
                    }
                }
            });
        }
        // Host-side lifecycle kicker (experimental): the engine owner parks
        // busy-polling a guest global (`ldar x8,[x8]; cmp #1; b.eq`) until the
        // Java layer's app-command sets it. On this box there is no Java side,
        // so `--kicker 0x<guest-hex-global>=<value-hex>` spawns a detached host
        // thread that writes the value to that guest global repeatedly WHILE
        // jit_run is parked, to test whether releasing the awaited predicate
        // lets StartApp proceed past the rendezvous toward the looper.
        // Host-side lifecycle kicker (experimental): the engine owner parks
        let mut kickers: Vec<(u64, KickerMode)> = Vec::new();
        let args: Vec<String> = std::env::args().collect();
        let mut i = 0;
        while i < args.len() {
            if let Some(k) = args[i].strip_prefix("--kicker") {
                let spec = if k.is_empty() {
                    i += 1;
                    if i >= args.len() { panic!("--kicker needs a value"); }
                    args[i].clone()
                } else {
                    k.trim_start_matches('=').to_string()
                };
                let (addr_s, val_s) = spec.split_once('=').unwrap_or((spec.trim_start_matches("0x"), "1"));
                let addr = u64::from_str_radix(addr_s.trim_start_matches("0x"), 16).expect("bad kicker addr");
                let val_l = val_s.trim_start_matches("0x").to_ascii_lowercase();
                // `=bcast` broadcasts the pthread_cond at that address; `=0xVAL`
                // writes the exact u64 value repeatedly; a bare `--kicker ADDR`
                // (no explicit `=`) keeps the historical 1->2 lifecycle pulse.
                let mode = if val_l == "bcast" {
                    KickerMode::Broadcast
                } else if spec.contains('=') {
                    let v = u64::from_str_radix(val_s.trim_start_matches("0x"), 16).expect("bad kicker val");
                    KickerMode::Fixed(v)
                } else {
                    KickerMode::Pulse
                };
                kickers.push((addr, mode));
            }
            i += 1;
        }
        for (addr, mode) in kickers {
            std::thread::spawn(move || {
                let what = match mode {
                    KickerMode::Broadcast => "pthread_cond_broadcast".to_string(),
                    KickerMode::Fixed(v) => format!("write 0x{v:x}"),
                    KickerMode::Pulse => "pulse 1->2".to_string(),
                };
                eprintln!("[elfjit:kicker] host thread drives 0x{addr:x} ({what})");
                let bc: unsafe extern "C" fn(*const u8) -> i32 = unsafe {
                    std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, c"pthread_cond_broadcast".as_ptr()))
                };
                for it in 0..400 {
                    unsafe {
                        match mode {
                            KickerMode::Broadcast => {
                                bc(addr as *const u8);
                            }
                            KickerMode::Fixed(v) => {
                                *((addr) as *mut u64) = v;
                            }
                            KickerMode::Pulse => {
                                // PULSE: hold 1 through the first gate (init poll
                                // wants *pred==1), then set 2 — the wait loops while
                                // *pred==1 (cd7c b.eq) and proceeds only when
                                // *pred !=1 and !=0 (cd84 cbz-on-zero); 2 is the
                                // terminal "done" state.
                                let v = if it < 60 { 1u64 } else { 2u64 };
                                *((addr) as *mut u64) = v;
                            }
                        }
                        if it % 100 == 0 {
                            eprintln!("[elfjit:kicker] t={it} guest_global 0x{addr:x}=%{:#x}", *((addr) as *const u64));
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            });
        }
        // Synthetic app-command feed (JIT_DRIVE_LIFECYCLE): a host thread pushes
        // Android lifecycle commands into the ALooper app-command queue, so a
        // GameActivity main loop that reaches `ALooper_pollOnce` dispatches
        // APP_CMD_START then APP_CMD_RESUME (the two commands that precede a real
        // EGL context / first frame on Android) instead of spinning on the empty
        // queue. `post_app_command` is the same channel the ALooper shim drains.
        if std::env::var_os("JIT_DRIVE_LIFECYCLE").is_some() {
            use arm64jit::shims::post_app_command;
            std::thread::spawn(|| {
                for (it, cmd) in [
                    arm64jit::shims::APP_CMD_START,
                    arm64jit::shims::APP_CMD_RESUME,
                    arm64jit::shims::APP_CMD_INIT_WINDOW,
                ]
                .iter()
                .enumerate()
                {
                    eprintln!("[elfjit:appcmd] posting APP_CMD_{it} ({cmd})");
                    post_app_command(*cmd);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });
        }
        // Real desktop X11 window for the ANativeWindow layer (GRAPHICS_-
        // RECOMMENDATION §5.3). Under JIT_DRIVE_LIFECYCLE we bring up an Xvfb
        // X server, open a 1280x720 window (the framebuffer
        // ANativeWindow_getWidth/Height report), set the display env Mesa's x11
        // EGL platform reads, and register the window XID as the guest's
        // ANativeWindow handle. Then eglCreateWindowSurface(dpy, config, win,
        // ...) builds on a genuine window, not the fallback sentinel. The
        // connection is leaked so the window outlives the thread scope.
        if std::env::var_os("JIT_DRIVE_LIFECYCLE").is_some() {
            std::thread::spawn(|| {
                let display_num = 220 + (std::process::id() % 50) as usize;
                let display = format!(":{display_num}");
                let mut xvfb = None;
                for _ in 0..20 {
                    if std::path::Path::new(&format!("/tmp/.X11-unix/X{display_num}")).exists() {
                        break;
                    }
                    if xvfb.is_none() {
                        xvfb = std::process::Command::new("Xvfb")
                            .arg(&display)
                            .arg("-screen").arg("0").arg("1280x720x24")
                            .arg("-nolisten").arg("tcp")
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                            .ok();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                let mut xid = 0u64;
                for _ in 0..40 {
                    if let Ok((conn, win)) = x11::open_window_sized(Some(&display), 1280, 720) {
                        Box::leak(Box::new(conn)); // keep the window alive for the boot
                        xid = win as u64;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                if xid != 0 {
                    unsafe {
                        std::env::set_var("DISPLAY", &display);
                        std::env::set_var("EGL_PLATFORM", "x11");
                    }
                    set_anativewindow_xid(xid);
                    eprintln!("[elfjit:anativewindow] wired real X11 window XID=0x{xid:x} on {display} as the guest ANativeWindow");
                } else {
                    eprintln!("[elfjit:anativewindow] could not open an X11 window (Xvfb absent?) — keeping the sentinel ANativeWindow");
                    if let Some(mut c) = xvfb {
                        let _ = c.kill();
                    }
                }
            });
        }
        // Per-thread futex latch kicker (--futex-kick <period-ms>). The engine
        // main-loop idle barrier (cycle L) is a REAL per-thread futex: each
        // guest thread parks in guest_svc's FUTEX_WAIT_BITSET on its OWN latch
        // (uaddr = x1 = x19+4, awaited val 0xF4240) at call-site lr=0x10284d134
        // — a wait-until-changed tick/frame barrier. A host-side producer must
        // CHANGE the latch value and FUTEX_WAKE it to release the wait, else
        // the loop re-parks (a plain WAKE is a spurious wake; the value is
        // still the awaited one, so the futex immediately re-blocks). This was
        // unreachable by the static --kicker (which only writes fixed guest
        // globals). The sampler already exposes each parked thread's x1, so we
        // locate the per-thread latch live and write a value != awaited before
        // waking — advancing the loop one tick per kick into egl*/gl*.
        if let Some(hex) = {
            let args: Vec<String> = std::env::args().collect();
            args.iter()
                .position(|a| a == "--futex-kick")
                .and_then(|i| args.get(i + 1).cloned())
        } {
            let period_ms: u64 = hex.trim().parse().expect("--futex-kick needs integer period-ms");
            const IDLE_FUTEX_CALLSITE: u64 = 0x10284d134; // guest lr when parked in the idle barrier
            std::thread::spawn(move || {
                eprintln!("[elfjit:futexkick] driving per-thread idle futex latch every {period_ms} ms");
                for it in 0..6000 {
                    std::thread::sleep(std::time::Duration::from_millis(period_ms));
                    for t in arm64jit::jit::snapshot_threads() {
                        if t.lr != IDLE_FUTEX_CALLSITE {
                            continue;
                        }
                        let latch = t.x1; // per-thread futex uaddr (== x19+4)
                        // The latch must be host-addressable (guest==host map).
                        if latch < 0x100000000 || latch >> 56 != 0 {
                            continue;
                        }
                        // A futex uaddr is a 4-byte `int` (4-aligned) — read as a
                        // c_int, never as a u64 (the 4-aligned address misaligns).
                        let old = unsafe { *(latch as *const libc::c_int) };
                        // Version-counter futex: the waiter captures *latch as
                        // its "expected" value and blocks WHILE *latch is
                        // unchanged. Releasing it requires writing a NEW value
                        // (increment the version — never reuse the previous or
                        // the next waiter captures that same value and
                        // re-blocks; a fixed write is a self-defeating one-off).
                        // Gate is the exact idle call-site.
                        let nv = old.wrapping_add(1);
                        unsafe { *(latch as *mut libc::c_int) = nv };
                        unsafe {
                            libc::syscall(
                                libc::SYS_futex,
                                latch as usize,
                                libc::FUTEX_WAKE as i64,
                                1i64,
                                0usize,
                            );
                        }
                        if it % 50 == 0 {
                            eprintln!(
                                "[elfjit:futexkick] it={it} guest_tid={} latch={latch:#x} old={old:#x}->{nv:#x}",
                                t.guest_tid
                            );
                        }
                    }
                }
            });
        }
        // Disable the gate-2 re-arm store: the owner's cond-wait loop at
    // 0x102b4cd50/0x102b4cd84 re-parks while *x19==1 and, on seeing that
    // pred has become 0, RE-ARMS it back to 1 (`mov x8,#1; str x8,[x19]` at
    // 0x102b4cdb0/0x102b4cdb4) so the terminal value driven from the host
    // never sticks. NOP the re-arm store so our value persists. JIT_DRIVE_*
    // mode only.
    if std::env::var_os("JIT_DRIVE_LIFECYCLE").is_some() {
        let rearm = el.guest_of(0x102b4cdb4);
        let page = rearm & !0xfff;
        if unsafe { libc::mprotect(page as *mut libc::c_void, 4096, libc::PROT_READ | libc::PROT_WRITE) } == 0 {
            unsafe { *(rearm as *mut u32) = 0xd503_201fu32 }; // NOP
            unsafe { libc::mprotect(page as *mut libc::c_void, 4096, libc::PROT_READ | libc::PROT_EXEC) };
            eprintln!("[kernel:NOP re-arm store 0x{rearm:x} (gate-2) under JIT_DRIVE_LIFECYCLE");
        }
    }

    match arm64jit::jit::jit_run(image, base, start_app, &mut s2 as *mut CpuState) {
            Err(e) => eprintln!("[elfjit] StartApp stopped: {e}"),
            Ok(r) => eprintln!("[elfjit] StartApp returned Ok({r:#x})"),
        }
        // Let any game-start workers run before exiting (or rather: keep the
        // process alive long enough for a real main loop to iterate/block).
        for _ in 0..4000 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if std::env::var_os("JIT_STATS").is_some() {
                let (c, h) = arm64jit::jit::block_cache_stats();
                // Compiles growing = StartApp is advancing through new init code;
                // flat compiles + rising hits = it is recycling cached hot blocks.
                eprintln!("[elfjit] stats: compiles={c} hits={h}");
            }
            // Periodic guest-thread state sampler (JIT_THREADS=1): while the
            // engine parks in the lifecycle-await, dump each registered guest
            // thread's live registers — its hostcall slot (pc), the guest call
            // site (lr = x30), and the wait-object args (x0..x2) — so the
            // boot wall is pinned to a precise guest function & release path.
            if std::env::var_os("JIT_THREADS").is_some() {
                let snaps = arm64jit::jit::snapshot_threads();
                let mut lines = format!("[elfjit] guest threads {}", snaps.len());
                for t in &snaps {
                    let at = arm64jit::resolver::name_of_call_addr(t.pc)
                        .unwrap_or_else(|| format!("{:#x}", t.pc));
                    lines.push_str(&format!(
                        "\n  host_tid={} guest_tid={} pc={at} lr={:#x} x0={:#x} x1={:#x} x2={:#x} x19={:#x}[*={:#x}] x29={:#x} sp={:#x}",
                        t.host_tid, t.guest_tid, t.lr, t.x0, t.x1, t.x2, t.x19,
                        if (0x100000000..0x108000000).contains(&t.x19) && t.x19 & 7 == 0 { unsafe { *(t.x19 as *const u64) } } else { 0 },
                        t.x29, t.sp
                    ));
                }
                eprintln!("{lines}");
            }
            if std::env::var_os("ELFJIT_EXIT_WHEN_IDLE").is_some()
                && arm64jit::jit::active_guest_threads() <= baseline
            {
                eprintln!("[elfjit] guest idle; exiting");
                break;
            }
        }
    }
    let (compiles, hits) = arm64jit::jit::block_cache_stats();
    if compiles > 0 || hits > 0 {
        eprintln!("[elfjit] block-cache: {compiles} compiles / {hits} hits");
    }
}
