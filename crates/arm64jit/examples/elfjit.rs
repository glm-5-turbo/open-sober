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

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: elfjit <aarch64-elf> [entry-guest-addr-hex]");
    let entry_arg = std::env::args().nth(2);

    let el = unsafe { libloader::elf::load_elf_image(std::path::Path::new(&path)) }
        .expect("load_elf_image");

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
    let len = seg.memsz as usize;

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
        let v = if arg == "buf" {
            let b = Box::leak(vec![0x7fu8; 256].into_boxed_slice());
            if i == 0 {
                let base = b.as_ptr() as u64;
                st.set(0, base);
                st.set(1, base + 32);
            }
            b.as_ptr() as u64
        } else {
            u64::from_str_radix(arg.trim_start_matches("0x"), 16)
                .unwrap_or_else(|e| panic!("bad x{i} hex: {e}"))
        };
        if !(arg == "buf" && i == 0) {
            st.set(i, v);
        }
    }

    // PC-driven dispatcher: compiles reachable regions and re-enters on
    // indirect branch (`blr`) / `br` / `ret`, so real (blr-heavy) Roblox code
    // can actually *execute* rather than stopping at the first blr.
    match jit_run(image, base, entry, &mut st as *mut CpuState) {
        Err(e) => {
            eprintln!("arm64jit run_loop stopped: {e}");
            std::process::exit(1);
        }
        Ok(r) => println!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r),
    }
}
