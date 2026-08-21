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

    // Bootstrap a guest runtime the JIT can actually run against:
    //  (1) guest stack — a real writable region whose host pointer is also a
    //      valid guest address (guest==host), SP (x31) at the top.
    //  (2) TLS base — point `tpidr` at a writable region so `mrs tpidr_el0`
    //      returns a usable, writable thread pointer (FS/GS-style).
    const STACK_SIZE: usize = 8 * 1024 * 1024;
    let stack = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            STACK_SIZE,
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
    let stack_top = stack as u64 + STACK_SIZE as u64;
    const TLS_SIZE: usize = 64 * 1024;
    let tls = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            TLS_SIZE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if tls == libc::MAP_FAILED {
        anyhow::bail!(
            "Failed to mmap guest tls: {}",
            std::io::Error::last_os_error()
        );
    }

    let mut st = arm64jit::jit::CpuState::new();
    st.set(31, stack_top); // x31 = SP (top; stack grows down)
    st.tpidr = tls as u64;
    info!(
        "guest stack 0x{:x} tls 0x{:x} — running via jit_run dispatcher (handles blr/br/ret + svc)",
        stack_top,
        tls as u64
    );

    // Use the PC-driven dispatcher: compiles reachable regions and re-enters on
    // `blr`/`br`/`ret`, so real (blr-heavy, syscall-making) Roblox init code can
    // actually execute rather than stopping at the first indirect call.
    let r = arm64jit::jit::jit_run(image, base, entry, &mut st as *mut _).map_err(|e| {
        anyhow::anyhow!(
            "arm64jit run_loop stopped near guest 0x{:x}: {e}",
            st.pc
        )
    })?;
    info!("JIT(no-QEMU) entry() -> {} (0x{:x})", r, r);
    warn!(
        "guest stack mapped at 0x{:x} and tls at 0x{:x} (leaked by design for one-shot run)",
        stack_top,
        tls as u64
    );
    Ok(r)
}