// Region disassembler: dump [start, start+len] bytes as decoded A64.
// usage: dump <elf> <guest-hex-start> <hex-len-bytes>   (len must be multiple of 4)
use arm64jit::decode::decode;

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump <elf> <guest-hex-start> <len-hex>");
    let start_raw = std::env::args().nth(2).unwrap();
    let len_raw = std::env::args().nth(3).unwrap();
    let el = libloader::elf::load_elf_image(std::path::Path::new(&path)).expect("load");
    let base = el.segments.iter().find(|s| s.prot.execute).expect("text seg").guest_vaddr;
    let full_end = el.segments.iter().fold(0u64, |m, s| m.max(s.guest_vaddr + s.memsz));
    let mut guest = u64::from_str_radix(start_raw.trim_start_matches("0x"), 16).unwrap();
    if guest < base {
        guest = el.guest_of(guest);
    }
    let len = u64::from_str_radix(len_raw.trim_start_matches("0x"), 16).unwrap();
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, (full_end - base) as usize) };
    println!("=== guest 0x{guest:x} + {len:#x} bytes ===");
    let mut i = 0u64;
    while i < len {
        let pc = guest + i;
        if pc < base || pc - base + 4 > image.len() as u64 {
            println!("0x{pc:x}: <out of range>");
            break;
        }
        let o = (pc - base) as usize;
        let w = u32::from_le_bytes([image[o], image[o + 1], image[o + 2], image[o + 3]]);
        let inst = decode(w);
        println!("0x{pc:x}: {:08x}  {}", w, format!("{:?}", inst).replace('\n', " "));
        i += 4;
    }
}