//! JIT execution path (experimental, no QEMU).
//!
//! Loads an aarch64 `.so`/ELF with `libloader::elf::load_elf` (which maps
//! segments at their virtual addresses, so guest vaddr == host vaddr), then
//! runs an entry address through `arm64jit`'s in-process translator.
//!
//! The full `libroblox.so` uses instructions beyond the current arm64jit
//! subset, so execution stops at the first unsupported opcode — expected and
//! reported. Self-contained aarch64 ELFs run to completion.

use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

/// Load `path` and run the guest instruction stream starting at `entry` (a
/// guest virtual address). Returns the x0 the JIT left behind.
pub fn run_elf_entry(path: &Path, entry: u64) -> Result<u64> {
    let el = unsafe { libloader::elf::load_elf(path) }
        .with_context(|| format!("load_elf({})", path.display()))?;
    // If the caller didn't pick a specific function, use the ELF's own entry.
        let entry = if entry == 0 { el.info.entry } else { entry };
        info!(
            "ELF loaded '{}' (base=0x{:x}, e_entry=0x{:x}, segments={})",
            path.display(),
            el.base_addr,
            el.info.entry,
            el.segments.len()
        );

    // Non-PIE modules map segments at their vaddr, so host == guest address.
    let image = unsafe { std::slice::from_raw_parts(entry as *const u8, 0x1000) };
    let mut st = arm64jit::jit::CpuState::new();
    let blk = arm64jit::jit::compile_image(image, entry, entry, &mut st as *mut _).map_err(
        |e| {
            anyhow::anyhow!(
                "arm64jit stopped at an unsupported instruction near entry 0x{:x}: {e} \
                 (full libroblox.so uses more than the current core subset)",
                entry
            )
        },
    )?;
    let r = unsafe { arm64jit::jit::run(&blk, &mut st as *mut _) };
    info!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
    Ok(r)
}