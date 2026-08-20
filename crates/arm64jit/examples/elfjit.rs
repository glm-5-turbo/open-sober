//! Integration spike: load a tiny static aarch64 ELF with libloader's ELF
//! loader (which maps PT_LOAD segments at their virtual addresses, so guest
//! vaddr == host vaddr), then run its entry function through the in-process
//! arm64jit translator — NO QEMU.
//!
//! Build a test ELF with:
//!   cat > t.c <<'EOF'
//!   int entry(void){ return 42; }
//!   EOF
//!   aarch64-linux-gnu-gcc -static -nostdlib -Wl,-e,entry t.c -o tiny.elf
//!
//! Run with: cargo run -p arm64jit --example elfjit -- /path/to/tiny.elf

use arm64jit::jit::{compile_image, run, CpuState};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: elfjit <aarch64-elf> [entry-addr-hex]");
    let el = unsafe { libloader::elf::load_elf(std::path::Path::new(&path)) }
        .expect("load_elf");

    let base = el.base_addr as u64;
    let entry = el.info.entry;
    println!(
        "loaded '{}': base=0x{:x} entry=0x{:x} entry_offset=0x{:x} segments={}",
        path,
        base,
        entry,
        el.info.entry_offset,
        el.segments.len()
    );

    // For a non-PIE statically-linked ELF the loader maps segments at their
    // vaddr, so host address == guest vaddr at the entry point.
    let host = entry;
    let image = unsafe { std::slice::from_raw_parts(host as *const u8, 0x1000) };

    let mut st = CpuState::new();
    let blk = compile_image(image, host, host, &mut st as *mut CpuState).expect("compile_image");
    let r = unsafe { run(&blk, &mut st as *mut CpuState) };
    println!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
}