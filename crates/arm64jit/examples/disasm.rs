// Disassemble guest regions of libroblox.so around the boot-wall addresses to
// name the functions. Diagnostic example; uses arm64jit::decode.
use arm64jit::decode::decode;

fn main() {
    let path = std::env::args().nth(1).expect("usage: disasm <elf> <guest-hex-addr...>");
    let el = libloader::elf::load_elf_image(std::path::Path::new(&path)).expect("load");
    let seg = el.segments.iter().find(|s| s.prot.execute).expect("text seg");
    let base = seg.guest_vaddr;
    let full_end = el.segments.iter().fold(0u64, |m, s| m.max(s.guest_vaddr + s.memsz));
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, (full_end - base) as usize) };
    for a in std::env::args().skip(2) {
        // accept an absolute guest address 0x102206bb0 or a link addr 0x2206bb0
        let mut guest = u64::from_str_radix(a.trim_start_matches("0x"), 16).unwrap();
        if guest < base {
            guest = el.guest_of(guest);
        }
        println!("=== around guest 0x{guest:x} ===");
        let start = guest.saturating_sub(0x28);
        for off in (0..0x50u64).step_by(4) {
            let pc = start + off;
            if pc < base || pc - base + 4 > image.len() as u64 {
                println!("0x{pc:x}: <out of range>");
                continue;
            }
            let o = (pc - base) as usize;
            let w = u32::from_le_bytes([image[o], image[o + 1], image[o + 2], image[o + 3]]);
            let inst = decode(w);
            let marker = if pc == guest { " <== HERE" } else { "" };
            println!("0x{pc:x}: {:08x}  {}{}", w, format!("{:?}", inst).replace('\n', " "), marker);
        }
    }
}