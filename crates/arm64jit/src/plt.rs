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
    if dyn_link == 0 {
        // Static (no dynamic segment) or otherwise no imports: nothing to bind.
        return (0, 0);
    }

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
    if pltrelsz == 0 {
        // Dynamic segment present but no PLT/JUMP_SLOT relocations at all.
        // GLOB_DAT relocations may still exist in the main DT_RELA, so bind
        // them before bailing (a module with only exported-data globals has
        // zero PLT calls yet depends on the main GOT for correctness).
        let glob = bind_glob_dat(el);
        eprintln!(
            "[plt] (no JUMP_SLOT) bound {} GLOB_DAT/ABS64, {} unresolved",
            glob.0, glob.1
        );
        return (0, 0);
    }

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

    // __stack_chk_guard: the guest prologue (`adrp xN, …; ldr xN,[xN,#off]; ldr
    // …,[xN]; stur …,[x29,#-8]`) reads a *data* GOT slot for libc's canary. The
    // Android NDK build emits NO relocation for this slot (only JUMP_SLOTs — no
    // .rela.dyn / GLOB_DAT / RELATIVE), so it stays 0x0 and the guest null-faults
    // on its first `ldr x8,[x0]`. The QEMU path (jni_shim.c) fixed this by
    // writing a live canary address into the GOT; mirror it here.
    patch_stack_canary(el, &wr64);

    // Bind GLOB_DAT relocations in the main DT_RELA section. Jump-slot (PLT)
    // handling above only covers DT_JMPREL; exported-data / function-pointer
    // globals referenced from `-fPIC` module code go through the *main* GOT via
    // R_AARCH64_GLOB_DAT, and if left un-patched the guest `adrp;ldr x,[GOT]`
    // reads 0 and derefs/calls NULL. See bind_glob_dat for the exact semantics.
    let glob = bind_glob_dat(el);
    eprintln!(
        "[plt] bound {resolved} JUMP_SLOT + {} GLOB_DAT/ABS64 ({} unresolved), {} unresolved",
        glob.0, unresolved, glob.1
    );

    (resolved, unresolved)
}

/// Bind `R_AARCH64_GLOB_DAT` (1025) relocations in the main `DT_RELA` section.
///
/// JUMP_SLOT (DT_JMPREL) covers PLT *function calls*; but `-fPIC` module code
/// that references an *exported* global (a data variable or a function pointer)
/// goes through the main GOT via GLOB_DAT: the compiler emits
/// `adrp x0,GOT; ldr x0,[x0,#off]` to fetch the symbol's *runtime address*,
/// then dereferences or calls through it. `R_AARCH64_GLOB_DAT` says
/// `*(r_offset) = S` (S = the symbol's load address). Left un-patched the guest
/// loads 0 and either calls address 0 or null-derefs.
///
/// The symbol's runtime value for *this* module = `el.guest_of(st_value)` (the
/// loader maps guest==host, so `ldr xN,[GOT]` then `[xN]`/`blr xN` resolves back
/// into the mapped image). For an *undefined/imported* symbol, the value is the
/// host address: for a data object use `dlsym` raw (guest==host addressable); for
/// a function/notype use the resolver's host-call thunk so a guest call to the
/// slot dispatches to the real host fn. Returns `(bound, unresolved)`.
fn bind_glob_dat(el: &LoadedElf) -> (usize, usize) {
    const DT_NULL: i64 = 0;
    const PT_DYNAMIC: u8 = 2;
    const DT_RELA: i64 = 7;
    const DT_RELASZ: i64 = 8;
    const DT_SYMTAB: i64 = 6;
    const DT_STRTAB: i64 = 5;
    const DT_RELAENT: i64 = 9;
    const R_AARCH64_GLOB_DAT: u64 = 1025;
    const R_AARCH64_ABS64: u64 = 257;
    const SHN_UNDEF: u16 = 0;
    // ELF symbol type bits (st_info & 0xf).
    const STT_OBJECT: u8 = 1;

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
    fn rd16(p: usize) -> u16 {
        unsafe { std::ptr::read_unaligned(p as *const u16) }
    }
    #[inline]
    fn wr64(p: usize, v: u64) {
        unsafe { std::ptr::write_unaligned(p as *mut u64, v) };
    }

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
    if dyn_link == 0 {
        return (0, 0); // static ELF
    }

    let dynp = host(el.guest_of(dyn_link));
    let (mut rela, mut relasz, mut relaent, mut symtab_ref, mut strtab_ref) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut i = 0usize;
    loop {
        let tag = rd64(dynp + i * 16) as i64;
        let val = rd64(dynp + i * 16 + 8);
        if tag == DT_NULL {
            break;
        }
        match tag {
            DT_RELA => rela = val,
            DT_RELASZ => relasz = val,
            DT_RELAENT => relaent = val,
            DT_SYMTAB => symtab_ref = val,
            DT_STRTAB => strtab_ref = val,
            _ => {}
        }
        i += 1;
        if i > 4096 {
            break;
        }
    }
    if rela == 0 || relasz == 0 || symtab_ref == 0 {
        return (0, 0);
    }
    let entsz = if relaent != 0 { relaent as usize } else { 24 };

    // GLOB_DAT lives in the *main* reloc table, which also holds the RELATIVE
    // entries the loader already applied. We only process GLOB_DAT here.
    let rela_h = host(el.guest_of(rela));
    let symtab_h = host(el.guest_of(symtab_ref));
    let strtab_h = host(el.guest_of(strtab_ref));

    let n = (relasz as usize) / entsz;
    let (mut bound, mut unresolved) = (0usize, 0usize);
    for k in 0..n {
        let r = rela_h + k * entsz;
        let r_offset = rd64(r);
        let r_info = rd64(r + 8);
        let r_addend = rd64(r + 16) as i64; // Elf64_Rela.r_addend @ +16
        let stype = (r_info & 0xffff_ffff) as u64;
        // R_AARCH64_GLOB_DAT (1025) and R_AARCH64_ABS64 (257) are both
        // "write the symbol's runtime address here". GLOB_DAT is the GOT-slot
        // form (addend 0), ABS64 the data-initializer form (addend = symbol
        // offset for a defined symbol). Both belong to the same resolver
        // family and are left unbound by the loader (which only applies
        // RELATIVE), so a guest global/function-pointer reads link-time
        // garbage. Handle both.
        if stype != R_AARCH64_GLOB_DAT && stype != R_AARCH64_ABS64 {
            continue;
        }
        let sym_idx = (r_info >> 32) as usize;
        let sym = symtab_h + sym_idx * 24;
        let st_info = unsafe { *(sym as *const u8).add(4) };
        let st_shndx = rd16(sym + 6); // Elf64_Sym.st_shndx @ +6
        let st_value = rd64(sym + 8); // Elf64_Sym.st_value @ +8
        let st_name = rd32(sym) as usize;

        let slot_guest = el.guest_of(r_offset) as usize;
        if slot_guest == 0 {
            unresolved += 1;
            continue;
        }

        let value: Option<u64> = if st_shndx != SHN_UNDEF {
            // Defined in this module: runtime address = guest addr of st_value
            // plus the addend (ABS64 has addend = offset to the symbol; GLOB_DAT
            // typically 0).
            let va = el.guest_of(st_value);
            if va == 0 { None } else { Some(va.wrapping_add(r_addend as u64)) }
        } else {
            // Imported symbol: resolve its host address.
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
            if st_info & 0xf == STT_OBJECT {
                // Data object: dlsym gives the raw host (guest==host) addr.
                match std::ffi::CString::new(name) {
                    Ok(c) => {
                        let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c.as_ptr()) };
                        if p.is_null() { None } else { Some((p as u64).wrapping_add(r_addend as u64)) }
                    }
                    Err(_) => None,
                }
            } else {
                // Function / notype import: host-call thunk (callable).
                crate::resolver::resolve(&name).map(|a| a.wrapping_add(r_addend as u64))
            }
        };

        match value {
            Some(v) => {
                wr64(slot_guest, v);
                bound += 1;
            }
            None => {
                unresolved += 1;
                if std::env::var_os("JIT_TRACE").is_some() {
                    eprintln!("[plt:glob_dat] unresolved {:#x}", r_offset);
                }
            }
        }
    }
    (bound, unresolved)
}

/// Write a live canary pointer into the guest `__stack_chk_guard` GOT slot.
///
/// The slot holds the *address* of the canary variable; the guest then does
/// `ldr x8,[xN]` to read the canary bytes, storing them on its stack to check
/// against a later `ldr`. We point it at libc's real `__stack_chk_guard` so the
/// value matches what `__stack_chk_fail` expects, falling back to a stable
/// static canary if the host symbol can't be resolved. The slot address is a
/// per-build constant (see caller notes); `LoadedElf` guards sanity.
fn patch_stack_canary(el: &LoadedElf, wr64: &impl Fn(usize, u64)) {
    // Guest vaddr of the `__stack_chk_guard` data GOT slot for the reference
    // build. Verified (Session xx): JNI_OnLoad's prologue
    //   adrp x24, 0x631a000 ; ldr x24,[x24,#2608] ; ldr x8,[x24] ; stur x8,[x29,#-8]
    // reads this slot, which is 0x0 in the file (no relocation). Note objdump
    // prints `#2608` in DECIMAL = 0xa30, so the slot is 0x631a000 + 0xa30.
    // For PIE the loaded guest address of a link-time slot is `guest_of(link)`;
    // because the runtime maps guest==host (contig), that value doubles as the
    // host addr.
    const CANARY_GOT_LINK: u64 = 0x631a000 + 0xa30; // = 0x631aa30
    let host_addr = el.guest_of(CANARY_GOT_LINK) as usize;
    if host_addr == 0 {
        eprintln!("[plt] canary slot link {:#x} maps to 0; skipping", CANARY_GOT_LINK);
        return;
    }
    let cur = unsafe { std::ptr::read_unaligned(host_addr as *const u64) };

    static CANARY: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);
    let canary_val = CANARY.load(std::sync::atomic::Ordering::Relaxed);
    let canary_addr = if canary_val != 0 {
        &canary_val as *const u64 as u64
    } else {
        // Prefer libc's real canary so __stack_chk_fail and our value agree.
        let libc_guard =
            unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"__stack_chk_guard\0".as_ptr() as *const _) };
        let addr = if !libc_guard.is_null() {
            libc_guard as u64
        } else {
            // Static fallback: a stable non-zero canary byte pattern.
            let canary = 0x2f_2a_1a_0a_0e_0f_10_11u64;
            // If libc guard exists but reads 0, seed it too (mirror jni_shim).
            if !libc_guard.is_null() {
                unsafe { std::ptr::write_unaligned(libc_guard as *mut u64, canary) };
            }
            CANARY.store(canary, std::sync::atomic::Ordering::Relaxed);
            &CANARY as *const _ as u64
        };
        CANARY.store(addr, std::sync::atomic::Ordering::Relaxed);
        addr
    };

    if cur & !0x0000_0000_ffff_ffffu64 == 0 {
        // Only write when the slot doesn't already reference a real page, so we
        // never clobber a legitimately-bound canary or an unrelated GOT slot.
        wr64(host_addr, canary_addr);
        eprintln!("[plt] patched __stack_chk_guard GOT {:#x} ({:#x}) -> {:#x}", CANARY_GOT_LINK, cur, canary_addr);
    }
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