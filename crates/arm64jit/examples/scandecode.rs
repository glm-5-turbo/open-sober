// Scan every instruction in an ELF's executable segments and report any that
// the JIT decoder cannot translate (`Inst::Unsupported`), plus any that make
// `decode()` panic (a hard bug — it aborts the whole JIT process). These are
// real ISA gaps the boot would fault on the instant the framework gate advances
// into new code, so surfacing the full list converts "unknown future wall" into
// a concrete, prioritized ledger.
//
// Usage: scandecode <elf>
//   decodes all bytes in PF_X PT_LOADs, reporting Unsupported/panic by vaddr.
use arm64jit::decode::{decode, Inst};

fn main() {
    let path = std::env::args().nth(1).expect("usage: scandecode <elf>");
    let el = libloader::elf::load_elf_image(std::path::Path::new(&path)).expect("load elf");

    let mut unsup: Vec<(u64, u32)> = Vec::new();
    let mut total = 0u64;
    let mut panic_hits = 0u64;

    for seg in &el.segments {
        if !seg.prot.execute {
            continue;
        }
        // seg.vaddr is the host mapping base; memsz bytes are mapped.
        let base = seg.vaddr as u64;
        let size = seg.memsz as usize;
        let image = unsafe { std::slice::from_raw_parts(base as *const u8, size) };
        let guest_base = seg.guest_vaddr;
        let mut off = 0usize;
        while off + 4 <= image.len() {
            let w =
                u32::from_le_bytes([image[off], image[off + 1], image[off + 2], image[off + 3]]);
            total += 1;
            let pc = guest_base + off as u64;
            match std::panic::catch_unwind(|| decode(w)) {
                Ok(Inst::Unsupported(_)) => unsup.push((pc, w)),
                Ok(_) => {}
                Err(_) => {
                    // decode() panicked on this real instruction: hard bug.
                    unsup.push((pc, w));
                    panic_hits += 1;
                }
            }
            off += 4;
        }
    }

    // Distinct opcodes, most-instances first.
    let mut by_insn: std::collections::HashMap<u32, Vec<u64>> =
        std::collections::HashMap::new();
    for (pc, w) in &unsup {
        by_insn.entry(*w).or_default().push(*pc);
    }
    let mut rows: Vec<(u32, usize)> =
        by_insn.iter().map(|(w, ps)| (*w, ps.len())).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));

    println!("total instructions scanned: {total}");
    println!("Unsupported/PANIC decode hits: {}", unsup.len());
    println!("distinct opcodes:             {}", rows.len());
    println!("decode() PANIC hits:          {panic_hits}");

    println!("\ndistinct unsupported/panic opcodes by instance count:");
    for (w, n) in rows {
        println!("  {:08x}  x{n}   first@0x{:x}", w, by_insn[&w][0]);
    }
}