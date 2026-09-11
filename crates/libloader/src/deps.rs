// SPDX-License-Identifier: MIT
//
// Multi-module (DT_NEEDED) loader.
//
// The single-module loader (`elf::load_elf_image`) maps one aarch64 image at a
// fixed guest base — enough for a `-nostdlib` test binary, but a real shared
// library (Roblox's libroblox.so) `DT_NEED`s a dependency chain (libssl,
// libcrypto, liblog, the Android/GSI libs ...). Without loading those deps the
// guest faults resolving its first cross-module import.
//
// `load_elf_with_deps` loads the main image, then walks its `DT_NEEDED` list
// (recursively), placing every dependency *contiguously* right after the main
// image in the same high guest region. Because guest == host under
// `load_elf_image_at`, a single `jit_run` image slice spanning
// `[chain.base(), chain.end())` covers every module, so a guest `blr` from one
// module into another module's exported function is compiled from the same
// slice. Symbol scope for cross-module resolution is the caller's job (see
// `arm64jit::plt::build_export_scope`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::elf::{load_elf_image_at, LoadedElf};

/// A loaded module chain: entry 0 is the main image, then each dependency in
/// load (breadth-first) order. Every module is mapped into one contiguous high
/// guest region so `jit_run` can be given a single image slice covering all of
/// them.
#[allow(unused)]
#[derive(Debug)]
pub struct LoadedChain {
    /// Loaded modules in load order; `entries[0]` is the main image.
    pub entries: Vec<LoadedElf>,
}

impl LoadedChain {
    /// The main (entry-0) image.
    #[allow(unused)]
    pub fn main(&self) -> &LoadedElf {
        &self.entries[0]
    }

    /// Lowest guest address across all loaded modules (the main image's base,
    /// since dependencies are placed contiguously after it).
    #[allow(unused)]
    pub fn base(&self) -> u64 {
        self.entries
            .iter()
            .map(|e| e.segments.iter().map(|s| s.guest_vaddr).min().unwrap_or(0))
            .min()
            .unwrap_or(0)
    }

    /// End (exclusive) of the highest module mapping — the top of the
    /// contiguous guest image. The `jit_run` slice is `[base, end)`.
    #[allow(unused)]
    pub fn end(&self) -> u64 {
        self.entries
            .iter()
            .flat_map(|e| e.segments.iter())
            .map(|s| s.guest_vaddr.saturating_add(s.memsz))
            .max()
            .unwrap_or(0)
    }
}

/// Size of the AArch64 thread control block (the 16 bytes TP points at, before
/// every module's TLS block). Any TLS block starts at `TP + AARCH64_TCB_SIZE`.
const CHAIN_TCB_SIZE: u64 = 16;

/// Compute each chain module's *static-TLS block offset* (TP-relative, in load
/// order), so `bind_tls` can turn `R_AARCH64_TLS_TPREL64` / `R_AARCH64_TLSDESC`
/// relocations into concrete thread-relative offsets.
///
/// AArch64 uses the TLS_TCB_AT_TP model: TP (the `tpidr_el0` value returned by
/// `setup_chain_tls`) points at a 16-byte TCB, and every module's TLS block
/// lives at a positive, alignment-padded offset from it. A module with no
/// `PT_TLS` gets offset 0 (no block). The first TLS-bearing module's block
/// starts immediately after the TCB at `TP + 16`; each later block is aligned
/// to its `p_align` and placed after the previous block's end. This mirrors
/// glibc's static-TLS layout for TLS_TCB_AT_TP targets.
#[allow(unused)]
pub fn layout_chain_tls(chain: &LoadedChain) -> Vec<u64> {
    let mut offsets = Vec::with_capacity(chain.entries.len());
    let mut next = CHAIN_TCB_SIZE;
    for el in &chain.entries {
        let tls = el
            .info
            .phdrs
            .iter()
            .find(|p| p.p_type == crate::elf::PT_TLS_PUB)
            .map(|p| (p.p_memsz, p.p_align.max(1)));
        match tls {
            None | Some((0, _)) => {
                offsets.push(0);
            }
            Some((memsz, align)) => {
                let start = align_up_u64(next, align);
                offsets.push(start);
                next = start + memsz;
            }
        }
    }
    offsets
}

/// Initialise a single contiguous per-thread TLS region for the *whole
/// chain*: copy each module's `PT_TLS` init image into `tls_region +
/// layout_chain_tls(chain)[i]`, zero-fill its `.tbss`, and return `(TP,
/// offsets)` — the thread pointer the guest's `mrs tpidr_el0` should yield,
/// plus the per-module TP-relative block offsets the JIT needs to bind TLS
/// relocations.
///
/// This is the multi-module generalisation of `elf::setup_guest_tls` (which
/// seeds only the *main* image). A real `libroblox.so` chain's dependencies
/// (libssl/libcrypto/...) carry `__thread` TLS of their own; without seeding
/// every block, a dependency's `mrs tpidr_el0; :gottprel:` access lands on
/// zeroed/foreign bytes.
#[allow(unused)]
pub fn setup_chain_tls(
    chain: &LoadedChain,
    tls_region: *mut u8,
    tls_region_size: usize,
) -> anyhow::Result<(u64, Vec<u64>)> {
    let offsets = layout_chain_tls(chain);
    for (el, &off) in chain.entries.iter().zip(&offsets) {
        if off == 0 {
            continue; // no PT_TLS block for this module
        }
        if let Some((p_offset, p_filesz, p_memsz)) = crate::elf::tls_layout(&el.info) {
            anyhow::ensure!(
                off as usize + p_memsz as usize <= tls_region_size,
                "chain module {} PT_TLS does not fit in {}-byte TLS region",
                el.info.path.display(),
                tls_region_size
            );
            let block = unsafe { tls_region.add(off as usize) };
            unsafe { std::ptr::write_bytes(block, 0, p_memsz as usize) };
            if p_filesz > 0 {
                let mut f = std::fs::File::open(&el.info.path)?;
                use std::io::{Read as _, Seek as _};
                f.seek(std::io::SeekFrom::Start(p_offset))?;
                let dst = unsafe { std::slice::from_raw_parts_mut(block, p_filesz as usize) };
                f.read_exact(dst)?;
            }
        }
    }
    Ok((tls_region as u64, offsets))
}

/// Highest mapped guest address across the chain so far — the next dependency
/// is laid out right after it (page-aligned) to keep the whole chain one
/// contiguous image.
fn chain_max_end(chain: &[LoadedElf]) -> u64 {
    chain
        .iter()
        .flat_map(|e| e.segments.iter())
        .map(|s| s.guest_vaddr.saturating_add(s.memsz))
        .max()
        .unwrap_or(0)
}

fn align_up_u64(v: u64, a: u64) -> u64 {
    (v + a - 1) & !(a - 1)
}

/// Resolve a `DT_NEEDED` library name to a file path.
///
/// Search order: `search_dirs` in the order given, then the current directory.
/// (A caller that wants a particular sysroot passes it as `search_dirs`; the
/// main image's own directory is the natural default for test fixtures.)
fn resolve_needed(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in search_dirs {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
    }
    let cwd = Path::new(name);
    if cwd.is_file() {
        return Some(cwd.to_path_buf());
    }
    None
}

/// Load the main image and, recursively, every `DT_NEEDED` dependency it (and
/// its dependencies) imports, laying each contiguously after the previous in
/// one high guest region.
///
/// `search_dirs` is the directory list used to resolve `DT_NEEDED` names for
/// the *main* image and all transitive deps. See [`LoadedChain`] for the
/// layout contract (single contiguous slice).
#[allow(unused)]
pub fn load_elf_with_deps(main: &Path, search_dirs: &[PathBuf]) -> Result<LoadedChain> {
    let mut entries: Vec<LoadedElf> = Vec::new();
    let mut loaded: HashSet<PathBuf> = HashSet::new();

    // Main image first, at the auto-chosen base (0x100000000 for a PIE).
    let main_elf = load_elf_image_at(main, None).context("load main image")?;
    let main_canon = std::fs::canonicalize(main).unwrap_or_else(|_| main.to_path_buf());
    loaded.insert(main_canon);
    entries.push(main_elf);

    // Breadth-first over NEEDED. We borrow the parsed needed_libs up front so
    // we can drop the borrow while loading (each load_elf_image_at is
    // independent).
    let mut queue: Vec<Vec<String>> = vec![entries[0].info.needed_libs.clone()];
    let mut qi = 0usize;
    while qi < queue.len() {
        let needed = queue[qi].clone();
        for name in &needed {
            let Some(dep_path) = resolve_needed(name, search_dirs) else {
                anyhow::bail!(
                    "cannot resolve DT_NEEDED dependency '{name}' of {} (searched {:?})",
                    main.display(),
                    search_dirs
                );
            };
            let canon = std::fs::canonicalize(&dep_path)
                .unwrap_or_else(|_| dep_path.to_path_buf());
            if !loaded.insert(canon.clone()) {
                continue; // already loaded (dedup, incl. the main image)
            }
            // Lay this dependency right after the highest mapping so the whole
            // chain stays one contiguous guest image.
            let base = align_up_u64(chain_max_end(&entries), 0x1000) as usize;
            let dep = load_elf_image_at(&dep_path, Some(base))
                .with_context(|| format!("load dependency {}", dep_path.display()))?;
            queue.push(dep.info.needed_libs.clone());
            entries.push(dep);
        }
        qi += 1;
    }

    Ok(LoadedChain { entries })
}