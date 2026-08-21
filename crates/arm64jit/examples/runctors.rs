//! Run the real `libroblox.so` `.init_array` constructor chain through the JIT
//! plus the real `guest_svc` syscall bridge (NO QEMU).
//!
//! `libroblox.so` is a shared library (e_entry=0): its first on-execute code is
//! the `.init_array` constructors (3484 of them at link vaddr 0x630bfc0, size
//! 0x6ce0), which a real `dlopen` runs before `JNI_OnLoad`. Running part of that
//! chain is the most "boot-like" work achievable without the Android/JNI runtime,
//! and it surfaces exactly which libc/import the runtime must supply.
//!
//! Usage: cargo run -p arm64jit --example runctors -- <libroblox.so> [max_ctors]

use arm64jit::jit::{CpuState, jit_run};
use std::path::Path;

const INIT_ARRAY_LINK: u64 = 0x630bfc0; // link-time vaddr of .init_array
const INIT_ARRAY_SIZE: usize = 0x6ce0;  // bytes -> 0x6ce0/8 = 3484 pointers

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: runctors <libroblox.so> [max_ctors]");
    let max = std::env::args()
        .nth(2)
        .map(|a| a.parse::<usize>().unwrap_or(64))
        .unwrap_or(64);

    let el = unsafe { libloader::elf::load_elf_image(Path::new(&path)) }.expect("load_elf_image");

    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .expect("no exec segment");
    let base = seg.guest_vaddr;
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, seg.memsz as usize) };

    // .init_array lives in the mapping (guest==host). Constructors are stored as
    // absolute link-time addresses in a PIE .init_array; compute guest of each.
    let ia_guest = el.guest_of(INIT_ARRAY_LINK);
    println!(
        "init_array guest=0x{:x} (n={}) text base=0x{:x} size=0x{:x}",
        ia_guest,
        INIT_ARRAY_SIZE / 8,
        base,
        seg.memsz
    );

    const STACK_SIZE: usize = 8 * 1024 * 1024;
    let stack = Box::leak(vec![0u8; STACK_SIZE].into_boxed_slice());
    let sp = stack.as_ptr() as u64 + STACK_SIZE as u64;
    let tls = Box::leak(vec![0u8; 65536].into_boxed_slice());

    let mut st = CpuState::new();
    st.set(31, sp);
    st.tpidr = tls.as_ptr() as u64;

    let ptrs: &[u64] =
        unsafe { std::slice::from_raw_parts(ia_guest as *const u64, INIT_ARRAY_SIZE / 8) };
    let nmax = INIT_ARRAY_SIZE / 8;
    let cap = nmax.min(max);
    println!("running up to {cap} of {nmax} constructors (pc-driven, real svc)");
    for i in 0..cap {
        let slot = ptrs[i];
        if slot == 0 {
            continue;
        }
        // Each slot is a link-time virtual address of a constructor function.
        let pc = el.guest_of(slot);
        match jit_run(image, base, pc, &mut st as *mut CpuState) {
            Ok(r) => {
                if std::env::var("RUNCTORS_TRACE").is_ok() {
                    println!("ctor[{i}] @0x{:x} -> 0x{:x} sp=0x{:x}", pc, r, st.x[31]);
                }
            }
            Err(e) => {
                println!("ctor[{i}] @0x{:x}: STOP at pc 0x{:x} -> {e}", pc, st.pc);
                println!("constructors completed before this point: {i}");
                break;
            }
        }
    }
    println!("done; attempted {cap} ctors, final sp=0x{:x}", st.x[31]);
}

/// The load base for guest_of relocation (kept imported even if unused in debug).
#[allow(dead_code)]
fn _rel(_el: &libloader::elf::LoadedElf) -> u64 {
    0
}