// SPDX-License-Identifier: MIT
//
// ELF loader — reads ELF headers, loads segments via mmap with correct
// permissions, and parses NEEDED entries for dependency resolution.
//
// This mirrors the functionality of Sober's libloader for loading the
// translated ARM binary (which is an x86-64 ELF) into memory.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// ELF constants and structures (minimal — just what we need)
// ---------------------------------------------------------------------------

/// ELF magic number: \x7fELF
#[allow(unused)]
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class: 64-bit
#[allow(unused)]
const ELFCLASS64: u8 = 2;

/// ELF data encoding: little-endian
#[allow(unused)]
const ELFDATA2LSB: u8 = 1;

/// ET_DYN — shared object / Position-Independent Executable
#[allow(unused)]
const ET_DYN: u16 = 3;

/// ET_EXEC — executable
#[allow(unused)]
const ET_EXEC: u16 = 2;

/// Program header types
#[allow(unused)]
const PT_NULL: u32 = 0;
#[allow(unused)]
const PT_LOAD: u32 = 1;
#[allow(unused)]
const PT_DYNAMIC: u32 = 2;
#[allow(unused)]
const PT_INTERP: u32 = 3;
#[allow(unused)]
const PT_NOTE: u32 = 4;
#[allow(unused)]
const PT_PHDR: u32 = 6;
#[allow(unused)]
const PT_GNU_STACK: u32 = 0x6474e551;
#[allow(unused)]
const PT_GNU_RELRO: u32 = 0x6474e552;

/// Program header flags
#[allow(unused)]
const PF_X: u32 = 1; // Execute
#[allow(unused)]
const PF_W: u32 = 2; // Write
#[allow(unused)]
const PF_R: u32 = 4; // Read

/// Dynamic section tags relevant to us
#[allow(unused)]
const DT_NEEDED: u64 = 1;
#[allow(unused)]
const DT_STRTAB: u64 = 5;
#[allow(unused)]
const DT_STRSZ: u64 = 10;
#[allow(unused)]
const DT_NULL: u64 = 0;

/// ELF 64-bit header — 64 bytes total
#[allow(unused)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF 64-bit program header — 56 bytes
#[allow(unused)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// ELF 64-bit dynamic entry — 16 bytes
#[allow(unused)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Dyn {
    pub d_tag: u64,
    pub d_val: u64, // or d_ptr
}

/// Memory protection flags derived from ELF program header flags.
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemProt {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemProt {
    /// Convert to POSIX mprotect flags (PROT_READ | PROT_WRITE | PROT_EXEC).
    #[allow(unused)]
    pub fn to_posix(self) -> libc::c_int {
        let mut flags = 0;
        if self.read {
            flags |= libc::PROT_READ;
        }
        if self.write {
            flags |= libc::PROT_WRITE;
        }
        if self.execute {
            flags |= libc::PROT_EXEC;
        }
        flags
    }

    /// Convert from ELF p_flags.
    #[allow(unused)]
    pub fn from_elf(p_flags: u32) -> Self {
        Self {
            read: (p_flags & PF_R) != 0,
            write: (p_flags & PF_W) != 0,
            execute: (p_flags & PF_X) != 0,
        }
    }
}

/// Information about an ELF binary extracted from its headers.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ElfInfo {
    /// Path to the ELF file.
    pub path: PathBuf,
    /// Entry point virtual address.
    pub entry: u64,
    /// Entry point relative to base (entry - base_load_addr).
    pub entry_offset: u64,
    /// Program headers.
    pub phdrs: Vec<Elf64Phdr>,
    /// Base load address (lowest PT_LOAD vaddr).
    pub base_load_addr: u64,
    /// Whether this is a PIE/shared object.
    pub is_pie: bool,
    /// List of NEEDED shared library names.
    pub needed_libs: Vec<String>,
    /// Whether the binary requires an interpreter (PT_INTERP).
    pub has_interp: bool,
    /// Path to the interpreter if present.
    pub interp: Option<String>,
}

/// A loaded ELF segment in memory.
#[allow(unused)]
#[derive(Debug)]
pub struct LoadedSegment {
    /// Guest virtual address from the ELF program header (p_vaddr), i.e. the
    /// address the *guest* program believes this segment lives at. For a static
    /// non-PIE this equals `vaddr`/`host`; for a PIE it is the offset (usually
    /// 0-based) the code was linked at.
    pub guest_vaddr: u64,
    /// Host virtual address where this segment was actually mapped.
    pub vaddr: u64,
    /// Size of the segment in memory.
    pub memsz: u64,
    /// File descriptor used for mmap (may be -1 if anonymous).
    pub fd: i32,
    /// Memory protection.
    pub prot: MemProt,
}

/// Result of loading an ELF binary into memory.
#[allow(unused)]
#[derive(Debug)]
pub struct LoadedElf {
    /// Base address where the ELF was loaded.
    pub base_addr: usize,
    /// Information about the ELF binary.
    pub info: ElfInfo,
    /// The individual loaded segments.
    pub segments: Vec<LoadedSegment>,
}

impl LoadedElf {
    /// Map a *guest* virtual address (an address in the ELF's own address
    /// space, e.g. `e_entry` for a PIE/ET_DYN) to the *host* address where that
    /// byte actually lives after `load_elf` mapped the segments.
    ///
    /// For a static non-PIE the guest vaddr == the host address (the loader
    /// maps segments at their link-time vaddr); for a PIE the loader maps at a
    /// kernel-chosen address, and this is where guest and host diverge.
    /// Returns `None` if `guest` does not fall inside any loaded segment.
    #[allow(unused)]
    pub fn host_addr_of(&self, guest: u64) -> Option<u64> {
        for seg in &self.segments {
            let gstart = seg.guest_vaddr;
            let gend = gstart.checked_add(seg.memsz)?;
            if guest >= gstart && guest < gend {
                // Same byte offset within the segment, translated to host.
                return Some(seg.vaddr + (guest - gstart));
            }
        }
        None
    }

    /// The first PT_LOAD segment whose protection allows execution (`r-x`),
    /// and the host range `[start, end)` it occupies. This is the "text"
    /// segment the JIT should translate code out of.
    #[allow(unused)]
    pub fn text_segment(&self) -> Option<(u64 /*host start*/, u64 /*host end*/)> {
        self.segments
            .iter()
            .find(|s| s.prot.execute)
            .map(|s| (s.vaddr, s.vaddr + s.memsz))
    }

    /// Translate a link-time (ELF-file) address into the guest/runtime address
    /// used after `load_elf_image`. For a non-PIE the identity usually holds
    /// (guest base == link base); for a PIE it adds the relocation delta. This
    /// is how callers turn a symbol address (e.g. an offset into libroblox.so)
    /// into the `entry` value `compile_image` expects.
    #[allow(unused)]
    pub fn guest_of(&self, link_addr: u64) -> u64 {
        // load_elf_image maps so that guest == base + (link - min_vaddr).
        // Equivalent: guest = link_addr + (self.base_addr - self.info.base_load_addr).
        let base_load = self.info.base_load_addr;
        self.base_addr as u64 + link_addr.wrapping_sub(base_load)
    }
}

/// Load an aarch64 ELF for the JIT path in a way that guarantees the property
/// the arm64jit translator relies on: **guest virtual address == host
/// address**, so ADRP/ADR of globals and every guest load/store dereference
/// the correct host pointer directly.
///
/// It maps ONE contiguous anonymous region (page-aligned) at a fixed JIT base,
/// then lays each PT_LOAD into it at `guest = JIT_BASE + (p_vaddr - min_vaddr)`,
/// zero-fills .bss, and applies per-segment mprotect. Because guest addresses
/// are computed as `JIT_BASE + (link_vaddr - base_load_addr)`, every link-time
/// address (e_entry, function offsets, ADRP-relative globals) resolves to a
/// guest address that is also the real host address of the byte. `LoadedElf`
/// therefore maps guest -> host as the identity.
///
/// Use [`LoadedElf::guest_of`] to translate a link-time (ELF-file) address into
/// the guest/runtime address used for entry points.
///
/// `JIT_BASE` is `0x400000` for non-PIE (matches the ELF's own link addresses)
/// and a fixed high base for PIE/ET_DYN (whose link vaddrs start at 0x0, which
/// would collide with the NULL page).
///
/// This avoids the fragile per-segment `MAP_FIXED` in `load_elf`, which for a
/// packed ET_DYN (like the real 117MB libroblox.so) lets a later segment's
/// `MAP_FIXED` target overlap the previous huge text mapping and SIGSEGV.
///
/// SAFETY: mmap/mprotect/copy from file. The returned mapping is RWX where the
/// ELF requests and lives until process exit (deliberate leak for a one-shot
/// run; callers needing to unmap should track `base_addr`).
#[allow(unused)]
pub fn load_elf_image(path: &Path) -> Result<LoadedElf> {
    let info = parse_elf(path)?;
    let _file = File::open(path)?;

    // Flatten all PT_LOAD into [link_min_vaddr, link_max_end).
    let mut loads: Vec<&Elf64Phdr> = info
        .phdrs
        .iter()
        .filter(|p| p.p_type == PT_LOAD)
        .collect();
    anyhow::ensure!(!loads.is_empty(), "no PT_LOAD segments in {}", path.display());
    loads.sort_by_key(|p| p.p_vaddr);

    let min_vaddr = loads.iter().map(|p| p.p_vaddr).min().unwrap_or(0);
    let max_end = loads
        .iter()
        .map(|p| p.p_vaddr.saturating_add(p.p_memsz))
        .max()
        .unwrap_or(0);
    anyhow::ensure!(max_end > min_vaddr, "empty load image");

    // Choose a fixed guest/JIT base that does not collide with host mappings.
    // For non-PIE use the link base (== min_vaddr, e.g. 0x400000); for PIE use
    // a fixed high region since the link origin is 0x0 (NULL page).
    let jit_base = if info.is_pie {
        0x1_0000_0000usize // 0x100000000
    } else {
        align_down_u64(min_vaddr, 0x1000) as usize
    };
    let base_page = align_down_u64(min_vaddr, 0x1000);
    let span = align_up_u64(max_end - base_page, 0x1000) as usize;

    let addr = unsafe {
        libc::mmap(
            jit_base as *mut libc::c_void,
            span,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
            -1,
            0,
        )
    };
    if addr == libc::MAP_FAILED {
        anyhow::bail!(
            "Failed to mmap JIT image ({} bytes @0x{:x}): {}",
            span,
            jit_base,
            std::io::Error::last_os_error()
        );
    }
    let base = addr as usize;

    // Lay each segment at base + (p_vaddr - min_vaddr). Guest vaddr of every
    // byte == its host address (identity mapping), which is what the JIT needs.
    let mut segments: Vec<LoadedSegment> = Vec::new();
    for ph in &loads {
        let offset_in_image = (ph.p_vaddr - base_page) as usize;
        let host_ptr = base as u64 + (ph.p_vaddr - min_vaddr);
        let filesz = ph.p_filesz as usize;

        if filesz > 0 {
            // Read the segment's file bytes into the mapped slot.
            let mut f = std::fs::File::open(path)?;
            use std::io::{Read as _, Seek as _};
            f.seek(std::io::SeekFrom::Start(ph.p_offset))?;
            let write_at = (addr as *mut u8).wrapping_add(offset_in_image);
            let buf = unsafe { std::slice::from_raw_parts_mut(write_at, filesz) };
            f.read_exact(buf)
                .context("Failed to read segment into JIT image")?;
        }
        // Zero-fill .bss (memsz > filesz) — fresh mmap is already zeroed.

        // Apply final protection for this segment's [host, host+memsz).
        let prot_addr = align_down_u64(host_ptr, 0x1000) as usize;
        let prot_end = align_up_u64(host_ptr + ph.p_memsz, 0x1000) as usize;
        let posix = MemProt::from_elf(ph.p_flags).to_posix();
        if unsafe { libc::mprotect(prot_addr as *mut libc::c_void, prot_end - prot_addr, posix) } != 0
        {
            anyhow::bail!(
                "Failed to mprotect {}: {}",
                prot_addr,
                std::io::Error::last_os_error()
            );
        }

        segments.push(LoadedSegment {
            guest_vaddr: host_ptr, // guest == host
            vaddr: host_ptr,
            memsz: ph.p_memsz,
            fd: -1,
            prot: MemProt::from_elf(ph.p_flags),
        });
    }

    // Relocate the recorded entry to guest space so callers can run it directly.
    let mut info = info;
    if info.entry >= min_vaddr {
        info.entry = base as u64 + (info.entry - min_vaddr);
    }

    info!(
        "Loaded JIT ELF '{}' as single {}MB image at host base 0x{:x} (guest==host, base_load=0x{:x})",
        path.display(),
        span / (1024 * 1024),
        base,
        min_vaddr
    );

    Ok(LoadedElf {
        base_addr: base,
        info,
        segments,
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Open and parse an ELF file, returning header information.
///
/// Reads the ELF header and program headers to produce an `ElfInfo`.
/// Does NOT load any segments into memory.
#[allow(unused)]
pub fn parse_elf(path: &Path) -> Result<ElfInfo> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open ELF file: {}", path.display()))?;

    // Read and validate ELF header
    let ehdr = read_ehdr(&mut file)?;
    validate_elf_header(&ehdr)
        .context("Invalid ELF header")?;

    // Read program headers
    let phdrs = read_phdrs(&mut file, &ehdr)?;

    // Determine base load address (lowest PT_LOAD vaddr)
    let base_load_addr = phdrs
        .iter()
        .filter(|ph| ph.p_type == PT_LOAD)
        .map(|ph| ph.p_vaddr)
        .min()
        .unwrap_or(0);

    // Check for PT_INTERP
    let interp = phdrs
        .iter()
        .find(|ph| ph.p_type == PT_INTERP)
        .and_then(|ph| {
            // Read the interpreter path from the file
            read_interp(&mut file, ph).ok()
        });

    // Parse NEEDED entries from the dynamic section
    let needed_libs = parse_needed(&mut file, &ehdr, &phdrs)?;

    let is_pie = ehdr.e_type == ET_DYN;
    let entry_offset = ehdr.e_entry.wrapping_sub(base_load_addr);

    info!("Parsed ELF: {} (PIE={}, entry=0x{:x}, base=0x{:x})",
        path.display(), is_pie, ehdr.e_entry, base_load_addr);

    Ok(ElfInfo {
        path: path.to_path_buf(),
        entry: ehdr.e_entry,
        entry_offset,
        phdrs,
        base_load_addr,
        is_pie,
        needed_libs,
        has_interp: interp.is_some(),
        interp,
    })
}

/// Load an ELF binary into memory using mmap.
///
/// Maps each PT_LOAD segment with the correct permissions, handling
/// page alignment and zero-fill for .bss sections.
///
/// SAFETY: This function calls mmap(2) which creates memory mappings.
/// The caller must ensure the file descriptor is valid and the
/// mappings don't conflict with existing mappings.
#[allow(unused)]
pub unsafe fn load_elf(path: &Path) -> Result<LoadedElf> {
    let info = parse_elf(path)?;
    let file = File::open(path)
        .with_context(|| format!("Failed to open ELF for mmap: {}", path.display()))?;
    let fd = file.as_raw_fd();

    // Find a suitable base address for loading.
    // For PIE binaries, we let the kernel choose (addr = 0 means
    // kernel picks). For non-PIE, we must use the fixed address.
    //
    // SAFETY: We mmap the first PT_LOAD segment to find a base,
    // then map the rest relative to it.
    let base_hint = if info.is_pie {
        std::ptr::null_mut() // Let kernel choose
    } else {
        info.base_load_addr as *mut libc::c_void
    };

    let mut segments: Vec<LoadedSegment> = Vec::new();
    let mut base_addr: usize = 0;

    for phdr in &info.phdrs {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let prot = MemProt::from_elf(phdr.p_flags);
        let vaddr_page = align_down_u64(phdr.p_vaddr, 0x1000);
        let _offset_page = align_down_u64(phdr.p_offset, 0x1000);
        let adjust_u64 = phdr.p_vaddr - vaddr_page;
        let adjust = adjust_u64 as usize;
        let map_size = align_up_u64(phdr.p_memsz + adjust_u64, 0x1000);

        // Map the segment
        // SAFETY: mmap(2) system call. The addresses and permissions
        // come from the ELF headers.
        let map_addr = if info.is_pie && base_addr == 0 {
            // First segment — let kernel pick
            let addr = libc::mmap(
                base_hint,
                map_size as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE, // Start RW for copy
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );

            if addr == libc::MAP_FAILED {
                anyhow::bail!("Failed to mmap base segment: {}", std::io::Error::last_os_error());
            }

            base_addr = (addr as usize).wrapping_sub(vaddr_page as usize);
            addr
        } else if info.is_pie {
            // Subsequent segment — map near base
            let target = base_addr.wrapping_add(vaddr_page as usize) as *mut libc::c_void;
            libc::mmap(
                target,
                map_size as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                -1,
                0,
            )
        } else {
            // Non-PIE — fixed address
            let target = vaddr_page as *mut libc::c_void;
            libc::mmap(
                target,
                map_size as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_FIXED | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if map_addr == libc::MAP_FAILED {
            anyhow::bail!("Failed to mmap segment at vaddr 0x{:x}: {}",
                phdr.p_vaddr, std::io::Error::last_os_error());
        }

        // Set base_addr on first segment
        if base_addr == 0 {
            base_addr = (map_addr as usize).wrapping_sub(vaddr_page as usize);
        }

        // Read segment data from file
        let data_addr = map_addr.wrapping_byte_add(adjust);
        let data_size = phdr.p_filesz as usize;

        if data_size > 0 {
            // Seek to the correct file offset and read
            let mut file_clone = File::open(path)?;
            file_clone.seek(SeekFrom::Start(phdr.p_offset))?;

            let buf = std::slice::from_raw_parts_mut(data_addr as *mut u8, data_size);
            file_clone.read_exact(buf)
                .context("Failed to read segment data from ELF file")?;
        }

        // Apply final protection
        // SAFETY: mprotect(2) changes memory protections.
        let prot_addr = align_down_u64(data_addr as u64, 0x1000) as usize;
        let map_end = (data_addr as u64) + phdr.p_memsz;
        let prot_size = align_up_u64(map_end - prot_addr as u64, 0x1000) as usize;
        let posix_prot = prot.to_posix();

        if libc::mprotect(prot_addr as *mut libc::c_void, prot_size, posix_prot) != 0 {
            anyhow::bail!("Failed to mprotect segment at 0x{:x}: {}",
                phdr.p_vaddr, std::io::Error::last_os_error());
        }

        segments.push(LoadedSegment {
            guest_vaddr: phdr.p_vaddr,
            vaddr: map_addr as u64 + adjust as u64,
            memsz: phdr.p_memsz,
            fd,
            prot,
        });

        debug!("Loaded PT_LOAD segment: vaddr=0x{:x}, filesz={}, memsz={}, prot={:?}",
            phdr.p_vaddr, phdr.p_filesz, phdr.p_memsz, prot);
    }

    info!("Loaded ELF '{}' at base 0x{:x} with {} segments",
        path.display(), base_addr, segments.len());

    Ok(LoadedElf {
        base_addr,
        info,
        segments,
    })
}

/// Read /proc/self/maps to understand the current memory layout.
///
/// Returns a vector of memory mapping entries from the kernel.
#[allow(unused)]
pub fn read_process_maps() -> Result<Vec<MemoryMapEntry>> {
    let maps = std::fs::read_to_string("/proc/self/maps")
        .context("Failed to read /proc/self/maps")?;

    let mut entries = Vec::new();
    for line in maps.lines() {
        if let Ok(entry) = MemoryMapEntry::parse(line) {
            entries.push(entry);
        }
    }

    debug!("Read {} memory map entries", entries.len());
    Ok(entries)
}

/// A single entry from /proc/self/maps.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct MemoryMapEntry {
    /// Start address of the mapping.
    pub start: u64,
    /// End address (exclusive).
    pub end: u64,
    /// Read permission.
    pub read: bool,
    /// Write permission.
    pub write: bool,
    /// Execute permission.
    pub execute: bool,
    /// Whether the mapping is shared.
    pub shared: bool,
    /// Whether the mapping is private.
    pub private: bool,
    /// Offset into the backing file.
    pub offset: u64,
    /// Device major:minor.
    pub dev: String,
    /// Inode number.
    pub inode: u64,
    /// Pathname (if any).
    pub pathname: Option<String>,
}

impl MemoryMapEntry {
    /// Parse a single line from /proc/self/maps.
    ///
    /// Format: address           perms offset  dev   inode   pathname
    ///         00400000-00452000 r-xp 00000000 08:02 173521  /usr/bin/...
    #[allow(unused)]
    pub fn parse(line: &str) -> Result<Self> {
        // Format: start-end perms offset dev:dev inode [path]
        let line = line.trim();
        if line.is_empty() {
            anyhow::bail!("Empty line");
        }

        let mut parts = line.splitn(6, ' ');
        let addr_part = parts.next().context("Missing address")?;
        let perm_part = parts.next().context("Missing permissions")?;
        let offset_part = parts.next().context("Missing offset")?;
        let dev_part = parts.next().context("Missing device")?;
        let inode_part = parts.next().context("Missing inode")?;
        let path_part = parts.next();

        // Parse addresses: "00400000-00452000"
        let addr_split: Vec<&str> = addr_part.split('-').collect();
        if addr_split.len() != 2 {
            anyhow::bail!("Invalid address format: {}", addr_part);
        }
        let start = u64::from_str_radix(addr_split[0], 16)
            .context("Invalid start address")?;
        let end = u64::from_str_radix(addr_split[1], 16)
            .context("Invalid end address")?;

        // Parse permissions: "r-xp"
        let perm_bytes = perm_part.as_bytes();
        let read = !perm_bytes.is_empty() && perm_bytes[0] == b'r';
        let write = perm_bytes.len() > 1 && perm_bytes[1] == b'w';
        let execute = perm_bytes.len() > 2 && perm_bytes[2] == b'x';
        let shared = perm_bytes.len() > 3 && perm_bytes[3] == b's';
        let private = perm_bytes.len() > 3 && perm_bytes[3] == b'p';

        // Parse offset
        let offset = u64::from_str_radix(offset_part, 16)
            .context("Invalid offset")?;

        // Parse inode
        let inode = inode_part.parse::<u64>()
            .context("Invalid inode")?;

        Ok(Self {
            start,
            end,
            read,
            write,
            execute,
            shared,
            private,
            offset,
            dev: dev_part.to_string(),
            inode,
            pathname: path_part.map(|s| s.to_string()),
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read the ELF header from a file.
#[allow(unused)]
fn read_ehdr(file: &mut File) -> Result<Elf64Ehdr> {
    file.seek(SeekFrom::Start(0))
        .context("Failed to seek to ELF header")?;

    let mut ehdr = Elf64Ehdr {
        e_ident: [0u8; 16],
        e_type: 0,
        e_machine: 0,
        e_version: 0,
        e_entry: 0,
        e_phoff: 0,
        e_shoff: 0,
        e_flags: 0,
        e_ehsize: 0,
        e_phentsize: 0,
        e_phnum: 0,
        e_shentsize: 0,
        e_shnum: 0,
        e_shstrndx: 0,
    };

    let size = size_of::<Elf64Ehdr>();
    let buf = unsafe {
        std::slice::from_raw_parts_mut(&mut ehdr as *mut _ as *mut u8, size)
    };

    file.read_exact(buf)
        .context("Failed to read ELF header")?;

    Ok(ehdr)
}

/// Validate the ELF header.
#[allow(unused)]
fn validate_elf_header(ehdr: &Elf64Ehdr) -> Result<()> {
    if ehdr.e_ident[0..4] != ELF_MAGIC {
        anyhow::bail!("Not an ELF file (bad magic)");
    }
    if ehdr.e_ident[4] != ELFCLASS64 {
        anyhow::bail!("Not a 64-bit ELF (class={})", ehdr.e_ident[4]);
    }
    if ehdr.e_ident[5] != ELFDATA2LSB {
        anyhow::bail!("Not little-endian ELF");
    }
    if ehdr.e_type != ET_EXEC && ehdr.e_type != ET_DYN {
        anyhow::bail!("Unsupported ELF type: {}", ehdr.e_type);
    }
    if ehdr.e_phentsize as usize != size_of::<Elf64Phdr>() {
        anyhow::bail!("Unexpected program header size: {}", ehdr.e_phentsize);
    }
    Ok(())
}

/// Read program headers from an ELF file.
#[allow(unused)]
fn read_phdrs(file: &mut File, ehdr: &Elf64Ehdr) -> Result<Vec<Elf64Phdr>> {
    file.seek(SeekFrom::Start(ehdr.e_phoff))
        .context("Failed to seek to program headers")?;

    let count = ehdr.e_phnum as usize;
    let mut phdrs = Vec::with_capacity(count);

    for i in 0..count {
        let mut phdr = Elf64Phdr {
            p_type: 0,
            p_flags: 0,
            p_offset: 0,
            p_vaddr: 0,
            p_paddr: 0,
            p_filesz: 0,
            p_memsz: 0,
            p_align: 0,
        };

        let size = size_of::<Elf64Phdr>();
        let buf = unsafe {
            std::slice::from_raw_parts_mut(&mut phdr as *mut _ as *mut u8, size)
        };

        file.read_exact(buf)
            .with_context(|| format!("Failed to read program header {}", i))?;

        phdrs.push(phdr);
    }

    Ok(phdrs)
}

/// Read the interpreter path from PT_INTERP.
#[allow(unused)]
fn read_interp(file: &mut File, phdr: &Elf64Phdr) -> Result<String> {
    file.seek(SeekFrom::Start(phdr.p_offset))
        .context("Failed to seek to PT_INTERP")?;

    let size = phdr.p_filesz as usize;
    let mut buf = vec![0u8; size];
    file.read_exact(&mut buf)
        .context("Failed to read interpreter path")?;

    // The string is null-terminated
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let s = String::from_utf8(buf[..end].to_vec())
        .context("Invalid UTF-8 in interpreter path")?;

    debug!("ELF interpreter: {}", s);
    Ok(s)
}

/// Parse NEEDED library entries from the dynamic section.
#[allow(unused)]
fn parse_needed(
    file: &mut File,
    _ehdr: &Elf64Ehdr,
    phdrs: &[Elf64Phdr],
) -> Result<Vec<String>> {
    // Find PT_DYNAMIC
    let dyn_phdr = match phdrs.iter().find(|ph| ph.p_type == PT_DYNAMIC) {
        Some(ph) => ph,
        None => return Ok(Vec::new()), // No dynamic section
    };

    // Find PT_LOAD for base address
    let base = phdrs
        .iter()
        .filter(|ph| ph.p_type == PT_LOAD)
        .map(|ph| ph.p_vaddr)
        .min()
        .unwrap_or(0);

    // Read dynamic entries
    let dyn_count = dyn_phdr.p_memsz / size_of::<Elf64Dyn>() as u64;
    file.seek(SeekFrom::Start(dyn_phdr.p_offset))
        .context("Failed to seek to dynamic section")?;

    let mut dyn_entries: Vec<Elf64Dyn> = Vec::with_capacity(dyn_count as usize);
    for _ in 0..dyn_count {
        let mut entry = Elf64Dyn { d_tag: 0, d_val: 0 };
        let size = size_of::<Elf64Dyn>();
        let buf = unsafe {
            std::slice::from_raw_parts_mut(&mut entry as *mut _ as *mut u8, size)
        };
        file.read_exact(buf)
            .context("Failed to read dynamic entry")?;
        dyn_entries.push(entry);
    }

    // Find DT_STRTAB and DT_STRSZ
    let strtab_offset = dyn_entries
        .iter()
        .find(|e| e.d_tag == DT_STRTAB)
        .map(|e| e.d_val)
        .unwrap_or(0);
    let strtab_size = dyn_entries
        .iter()
        .find(|e| e.d_tag == DT_STRSZ)
        .map(|e| e.d_val)
        .unwrap_or(0);

    if strtab_offset == 0 || strtab_size == 0 {
        return Ok(Vec::new());
    }

    // The strtab offset is a virtual address. We need to find its
    // corresponding file offset. Compute it from the PT_LOAD mapping.
    let strtab_file_offset = virt_to_file_offset(phdrs, strtab_offset, base)
        .context("Failed to resolve strtab virtual address to file offset")?;

    // Read the string table
    file.seek(SeekFrom::Start(strtab_file_offset))
        .context("Failed to seek to string table")?;
    let mut strtab = vec![0u8; strtab_size as usize];
    file.read_exact(&mut strtab)
        .context("Failed to read string table")?;

    // Extract NEEDED entries
    let mut needed = Vec::new();
    for entry in &dyn_entries {
        if entry.d_tag == DT_NULL {
            break;
        }
        if entry.d_tag == DT_NEEDED {
            let offset = entry.d_val as usize;
            if offset < strtab.len() {
                let end = strtab[offset..].iter().position(|&b| b == 0).unwrap_or(strtab.len() - offset);
                if let Ok(s) = String::from_utf8(strtab[offset..offset + end].to_vec()) {
                    if !s.is_empty() {
                        needed.push(s);
                    }
                }
            }
        }
    }

    debug!("Found {} NEEDED libraries: {:?}", needed.len(), needed);
    Ok(needed)
}

/// Convert a virtual address to a file offset using program headers.
#[allow(unused)]
fn virt_to_file_offset(phdrs: &[Elf64Phdr], vaddr: u64, _base: u64) -> Option<u64> {
    for ph in phdrs {
        if ph.p_type != PT_LOAD {
            continue;
        }
        let seg_start = ph.p_vaddr;
        let seg_end = ph.p_vaddr + ph.p_filesz;
        if vaddr >= seg_start && vaddr < seg_end {
            let offset = vaddr - seg_start;
            return Some(ph.p_offset + offset);
        }
    }
    None
}

/// Align a value down to the nearest alignment boundary.
#[allow(unused)]
fn align_down(val: u64, align: u64) -> u64 {
    val & !(align - 1)
}

/// Align a value up to the nearest alignment boundary.
#[allow(unused)]
fn align_up(val: u64, align: u64) -> u64 {
    (val + align - 1) & !(align - 1)
}

/// Align a usize up to the nearest alignment boundary.
#[allow(unused)]
fn align_up_usize(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// u64 version of align_down
#[allow(unused)]
fn align_down_u64(val: u64, align: u64) -> u64 {
    val & !(align - 1)
}

/// u64 version of align_up
#[allow(unused)]
fn align_up_u64(val: u64, align: u64) -> u64 {
    (val + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0x1234, 0x1000), 0x1000);
        assert_eq!(align_down(0x1000, 0x1000), 0x1000);
        assert_eq!(align_down(0, 0x1000), 0);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0x1234, 0x1000), 0x2000);
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn test_memory_map_entry_parse() {
        let line = "00400000-00452000 r-xp 00000000 08:02 173521 /usr/bin/cat";
        let entry = MemoryMapEntry::parse(line).unwrap();
        assert_eq!(entry.start, 0x400000);
        assert_eq!(entry.end, 0x452000);
        assert!(entry.read);
        assert!(!entry.write);
        assert!(entry.execute);
        assert!(!entry.shared);
        assert!(entry.private);
        assert_eq!(entry.offset, 0);
        assert_eq!(entry.dev, "08:02");
        assert_eq!(entry.inode, 173521);
        assert_eq!(entry.pathname.as_deref(), Some("/usr/bin/cat"));
    }

    #[test]
    fn test_memory_map_entry_no_path() {
        let line = "7f0000000000-7f0000001000 rwxp 00000000 00:00 0 [heap]";
        let entry = MemoryMapEntry::parse(line).unwrap();
        assert_eq!(entry.pathname.as_deref(), Some("[heap]"));
    }

    #[test]
    fn test_memprot_from_elf() {
        let prot = MemProt::from_elf(PF_R | PF_X);
        assert!(prot.read);
        assert!(!prot.write);
        assert!(prot.execute);

        let prot = MemProt::from_elf(PF_R | PF_W);
        assert!(prot.read);
        assert!(prot.write);
        assert!(!prot.execute);
    }

    #[test]
    fn test_memprot_to_posix() {
        let prot = MemProt { read: true, write: false, execute: true };
        let posix = prot.to_posix();
        assert!(posix & libc::PROT_READ != 0);
        assert!(posix & libc::PROT_EXEC != 0);
        assert!(posix & libc::PROT_WRITE == 0);
    }
}