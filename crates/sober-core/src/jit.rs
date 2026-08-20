//! JIT execution path (experimental, no QEMU).
//!
//! Loads an aarch64 `.so`/ELF with `libloader::elf::load_elf`, then runs a
//! guest entry address through `arm64jit`'s in-process translator.
//!
//! PIE mapping: guest virtual addresses (e_entry, ADRP targets) are translated
//! to host addresses via `LoadedElf::host_addr_of`, so both static ELFs and
//! PIE/ET_DYN shared objects (like the real `libroblox.so`) load correctly.
//!
//! The full `libroblox.so` uses instructions beyond the current arm64jit
//! subset, so execution stops at the first unsupported opcode — expected and
//! reported with the guest address for honest iteration.

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, warn};

/// Load `path` and run the guest instruction stream starting at `entry` (a
/// guest virtual address; 0 means the ELF's own e_entry). Returns the x0 the
/// JIT left behind.
pub fn run_elf_entry(path: &Path, entry: u64) -> Result<u64> {
    let el = unsafe { libloader::elf::load_elf(path) }
        .with_context(|| format!("load_elf({})", path.display()))?;
    let entry = if entry == 0 { el.info.entry } else { entry };

    info!(
        "ELF loaded '{}' (is_pie={}, guest entry=0x{:x}, base_load_vaddr=0x{:x}, segments={})",
        path.display(),
        el.info.is_pie,
        entry,
        el.info.base_load_addr,
        el.segments.len()
    );

    // PIE-aware mapping: find the executable segment, slice its host-mapped
    // bytes as the compile `image`, and translate the chosen guest `entry`
    // into that segment's host address.
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .context("no executable segment in load result")?;
    let base = seg.guest_vaddr;
    let host_entry = el
        .host_addr_of(entry)
        .context(format!(
            "guest entry 0x{:x} outside any loaded segment",
            entry
        ))?;
    let image =
        unsafe { std::slice::from_raw_parts(seg.vaddr as *const u8, seg.memsz as usize) };

    info!(
        "JIT running guest entry 0x{:x} at host 0x{:x} (text size 0x{:x})",
        entry, host_entry, seg.memsz
    );

    let mut st = arm64jit::jit::CpuState::new();
    // Give the guest a valid SP (x31) pointing at a fresh stack so prologues
    // that push via stp/??[sp,#-N]! don't fault. 2 MiB anonymous.
    let stack = {
        let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let size = 2 * 1024 * 1024;
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(p as isize, -1, "mmap guest stack");
            (p as usize, size, ps)
        }
    };
    st.x[31] = (stack.0 + stack.1) as u64; // SP = top of stack (grows down)
    let _ = stack.2;

    let blk = arm64jit::jit::compile_image(image, base, entry, &mut st as *mut _).map_err(|e| {
        anyhow::anyhow!(
            "arm64jit stopped on an unsupported instruction near guest 0x{:x} (host 0x{:x}): {e} \
             \nThis is the honest next decoder slice for libroblox.so.",
            entry,
            host_entry
        )
    })?;
    let r = unsafe { arm64jit::jit::run(&blk, &mut st as *mut _) };
    info!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
    warn!("guest stack remains mapped at 0x{:x} (leaked by design for one-shot run)", stack.0);
    Ok(r)
}