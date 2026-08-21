//! Resolve the real `libroblox.so` `R_AARCH64_JUMP_SLOT` (PLT) imports against
//! host x86-64 libc/libm via the arm64jit guest->host bridge, patching each GOT
//! slot to the resolved thunk's guest address.
//!
//! This turns `libroblox.so`'s PLT stubs (which `br x16` to a GOT slot) into
//! real host calls: after this pass, a translated `br x16` whose GOT points at a
//! resolved thunk invokes the matching host function. Imports that cannot be
//! found on the host (Android/bionic/JNI shims) are reported so the runtime
//! layer can bind them next.
//!
//! Usage: cargo run -p arm64jit --example resolveimports -- [libroblox.so]

use arm64jit::resolver;
use libloader::elf::load_elf_image;
use std::path::PathBuf;

const PT_DYNAMIC: u32 = 2;
const DT_NULL: i64 = 0;
const DT_PLTRELSZ: i64 = 2;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_JMPREL: i64 = 23;
const R_AARCH64_JUMP_SLOT: u64 = 1026;

#[inline(always)]
unsafe fn rd16(p: usize) -> u16 {
    unsafe { std::ptr::read_unaligned(p as *const u16) }
}
#[inline(always)]
unsafe fn rd32(p: usize) -> u32 {
    unsafe { std::ptr::read_unaligned(p as *const u32) }
}
#[inline(always)]
unsafe fn rd64(p: usize) -> u64 {
    unsafe { std::ptr::read_unaligned(p as *const u64) }
}
#[inline(always)]
unsafe fn wr64(p: usize, v: u64) {
    unsafe { std::ptr::write_unaligned(p as *mut u64, v) }
}

/// Read a NUL-terminated C string at host pointer `p`, at most 256 bytes.
unsafe fn read_cstr_host(mut p: usize) -> String {
    let mut v: Vec<u8> = Vec::new();
    for _ in 0..256 {
        let b = unsafe { *(p as *const u8) };
        if b == 0 {
            break;
        }
        v.push(b);
        p += 1;
    }
    String::from_utf8_lossy(&v).into_owned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "/home/code-agent/.cache/open-sober/libs/libroblox.so".to_string());
    let el = unsafe { libloader::elf::load_elf_image(&PathBuf::from(&path)) }.expect("load_elf_image");

    // host helper: guest -> readable host pointer
    let host = |g: u64| -> usize { el.host_addr_of(g).expect("guest not mapped") as usize };

    // ELF header is at the start of the file; find programmer via e_phoff.
    // The header lives at host address of the lowest guest vaddr (offset 0).
    let min_guest = el.segments.iter().map(|s| s.guest_vaddr).min().unwrap();
    let ehdr = host(min_guest);
    let e_phoff = unsafe { rd64(ehdr + 0x20) } as usize;
    let e_phentsize = unsafe { rd16(ehdr + 0x36) } as usize;
    let e_phnum = unsafe { rd16(ehdr + 0x38) } as usize;

    // PT_DYNAMIC link-time vaddr (from phdr) -> its host pointer.
    let mut dyn_link = 0u64;
    for i in 0..e_phnum {
        let ph = ehdr + e_phoff + i * e_phentsize;
        let pt = rd32_phdr(ph);
        if pt == PT_DYNAMIC {
            dyn_link = unsafe { rd64(ph + 0x10) };
            break;
        }
    }
    assert_ne!(dyn_link, 0, "no PT_DYNAMIC");

    // Walk dynamic entries (host pointers) to find the relocation tables.
    let dynp = host(el.guest_of(dyn_link));
    let (mut jmprel_link, mut pltrelsz, mut symtab_link, mut strtab_link) = (0u64, 0u64, 0u64, 0u64);
    let mut i = 0usize;
    loop {
        let tag = unsafe { rd64(dynp + i * 16) } as i64;
        let val = unsafe { rd64(dynp + i * 16 + 8) };
        if tag == DT_NULL {
            break;
        }
        match tag {
            DT_PLTRELSZ => pltrelsz = val,
            DT_STRTAB => strtab_link = val,
            DT_SYMTAB => symtab_link = val,
            DT_JMPREL => jmprel_link = val,
            _ => {}
        }
        i += 1;
        if i > 512 {
            break;
        }
    }
    assert_ne!(pltrelsz, 0, "no PLT relocs");
    let nsyms = (pltrelsz / 24) as usize;

    // host addresses of the symbol/string/reloc tables.
    let strtab_h = host(el.guest_of(strtab_link));
    let symtab_h = host(el.guest_of(symtab_link));
    let jmprel_h = host(el.guest_of(jmprel_link));

    let mut rows: Vec<String> = Vec::new();
    let (mut resolved, mut needs_shim) = (0usize, 0usize);
    for n in 0..nsyms {
        let r = jmprel_h + n * 24;
        let r_offset = unsafe { rd64(r) }; // link-time address of the GOT slot
        let r_info = unsafe { rd64(r + 8) };
        if r_info & 0xffff_ffff != R_AARCH64_JUMP_SLOT {
            continue;
        }
        let sym_idx = (r_info >> 32) as usize;
        let sym = symtab_h + sym_idx * 24;
        let st_name = unsafe { rd32(sym) } as usize; // offset into strtab
        let name = unsafe { read_cstr_host(strtab_h + st_name) };

        match arm64jit::resolver::resolve(name.as_bytes()) {
            Some(thunk_guest) => {
                // Patch the GOT slot at guest r_offset to the host thunk's guest addr
                let got_host = host(el.guest_of(r_offset));
                unsafe { wr64(got_host, thunk_guest) };
                rows.push(format!("  {name:<42} -> host (GOT patched)"));
                resolved += 1;
            }
            None => {
                rows.push(format!("  {name:<42} (shim needed)"));
                needs_shim += 1;
            }
        }
    }

    println!("== {path} PLT import classification ==");
    println!("resolved-to-host NOW: {resolved}  |  bionic/Android shim needed: {needs_shim}  (total {})", resolved + needs_shim);
    println!("(first 90):");
    for row in rows.iter().take(90) {
        println!("{row}");
    }
}

#[inline(always)]
fn rd32_phdr(p: usize) -> u32 {
    unsafe { std::ptr::read_unaligned(p as *const u32) }
}