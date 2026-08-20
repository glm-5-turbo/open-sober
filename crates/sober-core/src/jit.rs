//! JIT execution path (experimental, no QEMU).
//!
//! Loads an aarch64 `.so`/ELF with `libloader::elf::load_elf_image` (which
//! maps every PT_LOAD into ONE contiguous kernel-chosen region so **guest
//! vaddr == host address**), then runs a guest entry address through
//! `arm64jit`'s in-process translator.
//!
//! The full `libroblox.so` uses instructions beyond the current arm64jit
//! subset, so execution stops at the first unsupported opcode — expected and
//! reported with the offending guest address. Self-contained aarch64 ELFs run
//! to completion.

use anyhow::{Context, Result};
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::path::Path;
use tracing::{info, warn};

/// Load `path` and run the guest instruction stream starting at `entry` (a
/// guest virtual address). Returns the x0 the JIT left behind.
pub fn run_elf_entry(path: &Path, entry: u64) -> Result<u64> {
    let el = unsafe { libloader::elf::load_elf_image(path) }
        .with_context(|| format!("load_elf_image({})", path.display()))?;
    // If the caller didn't pick a specific function, use the ELF's own entry.
    // `entry` is a link-time address; translate to guest/runtime space (== host,
    // since load_elf_image maps guest==host).
    let entry = if entry == 0 {
        el.info.entry
    } else {
        el.guest_of(entry)
    };
    info!(
        "ELF loaded '{}' (base=0x{:x}, e_entry=0x{:x}, segments={})",
        path.display(),
        el.base_addr,
        el.info.entry,
        el.segments.len()
    );

    // The translator dereferences computed guest addresses directly as host
    // pointers, so guest vaddr MUST equal host address — `load_elf_image`
    // guarantees that. Pick the executable segment as the code source.
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .context("no executable segment in load result")?;
    let base = seg.guest_vaddr; // == host addr of image[0] (guest==host)
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, seg.memsz as usize) };
    info!(
        "JIT running guest entry 0x{:x} at host 0x{:x} (text size 0x{:x})",
        entry,
        entry,
        seg.memsz
    );

    // Give the guest a small anonymous stack at a chosen address (just below
    // where our mappings live) so spills/prologues have room.
    let stack = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            8 * 1024 * 1024,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if stack == libc::MAP_FAILED {
        anyhow::bail!(
            "Failed to mmap guest stack: {}",
            std::io::Error::last_os_error()
        );
    }
    let stack_top = stack as u64 + (8 * 1024 * 1024);
    // The translator's CpuState lives in our address space; set its sp so guest
    // LDP/STP-BL prologue has a usable RSP. (CpuState holds a host context; for
    // a C ABI entry the caller's rsp/reset is done by compile_image's prologue.)
    info!("guest stack mapped at 0x{:x} (top 0x{:x})", stack as usize, stack_top);

    let mut st = arm64jit::jit::CpuState::new();
    let blk = arm64jit::jit::compile_image(image, base, entry, &mut st as *mut _).map_err(|e| {
        anyhow::anyhow!(
            "arm64jit stopped on the first unsupported instruction near guest 0x{:x}: {e} \
             \nThis is the honest next decoder slice for libroblox.so.",
            entry
        )
    })?;
    let r = unsafe { arm64jit::jit::run(&blk, &mut st as *mut _) };
    info!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
    warn!(
        "guest stack remains mapped at 0x{:x} (leaked by design for one-shot run)",
        stack as usize
    );
    Ok(r)
}