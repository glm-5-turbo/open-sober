//! Integration spike: load an aarch64 ELF (static non-PIE OR PIE/ET_DYN) with
//! libloader's ELF loader, then run its entry function through the in-process
//! arm64jit translator — NO QEMU.
//!
//! This exercises the PIE mapping fix: guest virtual addresses (e_entry, ADRP
//! targets) are translated through `LoadedElf::host_addr_of` into the real host
//! address where the segment was mapped, so both static executables and shared
//! objects / position-independent executables work the same way.
//!
//! Build a test static ELF with:
//!   cat > t.c <<'EOF'
//!   int entry(void){ return 42; }
//!   EOF
//!   aarch64-linux-gnu-gcc -static -nostdlib -Wl,-e,entry t.c -o tiny.elf
//!
//! Build a test PIE/shared object with:
//!   aarch64-linux-gnu-gcc -shared -o libpie.so pie.c
//!
//! Run: cargo run -p arm64jit --example elfjit -- /path/to/elf

use arm64jit::jit::{compile_image, run, CpuState};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: elfjit <aarch64-elf> [entry-guest-addr-hex]");
    let entry_arg = std::env::args().nth(2);

    let el = unsafe { libloader::elf::load_elf(std::path::Path::new(&path)) }
        .expect("load_elf");

    // Guest entry: the ELF's own e_entry unless overridden.
    let entry = entry_arg
        .map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).expect("entry addr"))
        .unwrap_or(el.info.entry);

    println!(
        "loaded '{}': is_pie={} base_load_vaddr=0x{:x} e_entry=0x{:x}",
        path,
        el.info.is_pie,
        el.info.base_load_addr,
        el.info.entry
    );
    for s in &el.segments {
        println!(
            "  segment guest=[0x{:x},0x{:x}) host=[0x{:x},0x{:x}) prot={}{}{}",
            s.guest_vaddr,
            s.guest_vaddr + s.memsz,
            s.vaddr,
            s.vaddr + s.memsz,
            if s.prot.read { 'r' } else { '-' },
            if s.prot.write { 'w' } else { '-' },
            if s.prot.execute { 'x' } else { '-' },
        );
    }

    // ---- PIE-aware mapping ----
    // Find the executable ("text") segment. `base` = guest vaddr of its start,
    // `image` = the host-mapped bytes of that segment, `entry` = the guest addr
    // to begin executing.
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .expect("no executable segment");
    let base = seg.guest_vaddr;
    let host_start = seg.vaddr;
    let host_end = seg.vaddr + seg.memsz;
    // Translate the chosen guest entry to host via the segment offset.
    let host_entry = el.host_addr_of(entry).expect("entry not in any segment");

    println!(
        "running entry guest=0x{:x} host=0x{:x} (segment base guest=0x{:x} host=0x{:x} size=0x{:x})",
        entry, host_entry, base, host_start, seg.memsz
    );

    // `image` is the host bytes of the text segment. compile_image expects
    // `image[0]` to be the instruction whose guest address is `base`, and
    // `entry` the guest address of the first instruction to run.
    let image = unsafe { std::slice::from_raw_parts(host_start as *const u8, seg.memsz as usize) };
    let mut st = CpuState::new();
    match compile_image(image, base, entry, &mut st as *mut CpuState) {
        Err(e) => {
            // compile_image errors when it hits an instruction the translator
            // can't handle; the error string embeds the offending guest pc.
            eprintln!("arm64jit stopped at/after guest 0x{:x}: {e}", entry);
            std::process::exit(1);
        }
        Ok(blk) => {
            let r = unsafe { run(&blk, &mut st as *mut CpuState) };
            println!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
        }
    }
}