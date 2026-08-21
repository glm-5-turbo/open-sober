//! Fold the import resolver + host shims into the loaded-image boot path.
//!
//! `bind_image_plt` walks a loaded ELF's `DT_JMPREL` JUMP_SLOT relocations and
//! patches each GOT slot to the matching host thunk, so translated Roblox code
//! can `blr` to libc/libm/host-shim functions in-process. This is the JIT
//! equivalent of the dynamic linker working through `libloader`'s in-memory
//! image: after this, every import the guest references resolves to a real host
//! x86-64 routine (or a benign graphics/audio/media fallback stub).

use libloader::elf::LoadedElf;

/// Walk `DT_JMPREL` and bind every `R_AARCH64_JUMP_SLOT` GOT slot to a host
/// thunk guest address. Returns `(resolved, total_relocs)`.
///
/// Resolution order mirrors `resolveimports`: 1) native host symbol (libc/libm,
/// with an explicit `libm.so.6` fallback), 2) hand-written bionic shim
/// (`__errno`/`__strlen_chk`/`__android_log_print`/... ), 3) float-ABI bridge
/// (float64/float32), 4) catch-all graphics/audio/media fallback stub.
pub fn bind_image_plt(el: &LoadedElf) -> (usize, usize) {
    const DT_NULL: i64 = 0;
    const PT_DYNAMIC: u8 = 2; // NOT 6 (PT_PHDR=6); 2 is the dynamic segment
    const DT_STRTAB: i64 = 5;
    const DT_SYMTAB: i64 = 6;
    const DT_JMPREL: i64 = 23;
    const DT_PLTRELSZ: i64 = 2;
    const R_AARCH64_JUMP_SLOT: u64 = 1026;

    let host = |g: u64| -> usize { el.host_addr_of(g).expect("guest not mapped") as usize };
    #[inline]
    fn rd64(p: usize) -> u64 {
        unsafe { std::ptr::read_unaligned(p as *const u64) }
    }
    #[inline]
    fn rd32(p: usize) -> u32 {
        unsafe { std::ptr::read_unaligned(p as *const u32) }
    }
    #[inline]
    fn wr64(p: usize, v: u64) {
        unsafe { std::ptr::write_unaligned(p as *mut u64, v) };
    }
    #[inline]
    fn rd16(p: usize) -> u16 {
        unsafe { std::ptr::read_unaligned(p as *const u16) }
    }

    // ELF header of the first (lowest) segment: file offset 0 maps there.
    let min_guest = el
        .segments
        .iter()
        .map(|s| s.guest_vaddr)
        .min()
        .expect("no segments");
    let ehdr = host(min_guest);
    let e_phoff = rd64(ehdr + 0x20) as usize;
    let e_phentsize = rd16(ehdr + 0x36) as usize;
    let e_phnum = rd16(ehdr + 0x38) as usize;

    let mut dyn_link = 0u64;
    for i in 0..e_phnum {
        let ph = ehdr + e_phoff + i * e_phentsize;
        if rd32(ph) == PT_DYNAMIC as u32 {
            dyn_link = rd64(ph + 0x10); // p_vaddr
            break;
        }
    }
    assert_ne!(dyn_link, 0, "no PT_DYNAMIC");

    let dynp = host(el.guest_of(dyn_link));
    let (mut jmprel, mut pltrelsz, mut symtab_ref, mut strtab_ref) = (0u64, 0u64, 0u64, 0u64);
    let mut i = 0usize;
    loop {
        let tag = rd64(dynp + i * 16) as i64;
        let val = rd64(dynp + i * 16 + 8);
        if tag == DT_NULL as i64 {
            break;
        }
        match tag {
            DT_JMPREL => jmprel = val,
            DT_PLTRELSZ => pltrelsz = val,
            DT_SYMTAB => symtab_ref = val,
            DT_STRTAB => strtab_ref = val,
            _ => {}
        }
        i += 1;
        if i > 4096 {
            break;
        }
    }
    assert_ne!(pltrelsz, 0, "no PLT relocs");

    let jmprel_h = host(el.guest_of(jmprel));
    let symtab_h = host(el.guest_of(symtab_ref));
    let strtab_h = host(el.guest_of(strtab_ref));

    let nsyms = (pltrelsz as usize) / 24;
    let (mut resolved, mut unresolved) = (0usize, 0usize);
    let mut pending: Vec<(Vec<u8>, u64)> = Vec::new(); // (name, guest r_offset)

    for n in 0..nsyms {
        let r = jmprel_h + n * 24;
        let r_offset = rd64(r);
        let r_info = rd64(r + 8);
        let stype = (r_info & 0xffff_ffff) as u32;
        if stype != R_AARCH64_JUMP_SLOT as u32 {
            continue;
        }
        let sym_idx = (r_info >> 32) as usize;
        let sym = symtab_h + sym_idx * 24;
        let st_name = rd32(sym) as usize;
        let mut name = Vec::new();
        {
            let mut p = strtab_h + st_name;
            for _ in 0..256 {
                let c = unsafe { *(p as *const u8) };
                if c == 0 {
                    break;
                }
                name.push(c);
                p += 1;
            }
        }
        match (
            crate::resolver::resolve(&name),
            crate::resolver::resolve_float(&name),
            crate::resolver::resolve_float32(&name),
        ) {
            (Some(a), _, _) => {
                wr64(host(el.guest_of(r_offset)), a);
                resolved += 1;
            }
            (None, Some(a), _) => {
                wr64(host(el.guest_of(r_offset)), a);
                resolved += 1;
            }
            (None, None, Some(a)) => {
                wr64(host(el.guest_of(r_offset)), a);
                resolved += 1;
            }
            (None, None, None) => pending.push((name, r_offset)),
        }
    }

    if !pending.is_empty() {
        let names: Vec<&[u8]> = pending.iter().map(|(n, _)| n.as_slice()).collect();
        crate::shims::register_graphics_stubs(&names);
        for (name, r_offset) in &pending {
            if let Some(a) = crate::resolver::resolve(name) {
                wr64(host(el.guest_of(*r_offset)), a);
                resolved += 1;
            } else {
                unresolved += 1;
            }
        }
    }

    (resolved, unresolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Guarded against the real Roblox binary so the suite stays hermetic when
    // the asset isn't cached; when present this proves the boot-path binder
    // resolves the full PLT JUMP_SLOT set you'd otherwise hit at runtime.
    #[test]
    fn bind_image_plt_real_roblox_binds_all() {
        let path = "/home/code-agent/.cache/open-sober/libs/libroblox.so";
        if !std::path::Path::new(path).exists() {
            eprintln!("skipping: no cached libroblox.so");
            return;
        }
        let el = unsafe { libloader::elf::load_elf_image(std::path::Path::new(path)) }
            .expect("load_elf_image");
        let (bound, unbound) = bind_image_plt(&el);
        eprintln!("bound {bound}, unbound {unbound}");
        assert!(bound >= 500, "expected most of 537 JUMP_SLOT imports bound, got {bound}");
        assert_eq!(unbound, 0, "every import should bind via resolve/stub");
    }
}