// SPDX-License-Identifier: MIT
//
// Android packed relocations (APS2) — decode + RELATIVE-application.
//
// Real Roblox/Android APK shared libraries (and the GSI system libs they
// depend on) often ship their ELF relocations in Android's *packed* form
// (`DT_ANDROID_RELA`, tag 0x60000011) rather than a standard `DT_RELA`, and
// the ELF may carry `R_AARCH64_RELATIVE` data relocations in `.data` /
// `.data.rel.ro` that a loader must apply before the code can dereference
// globals (vtables, pointer globals). The QEMU path worked around this with
// an external `unpack_rela.py`; this module brings the same decoding into the
// Rust loader so the JIT `load_elf_image` path can materialize the packed
// reloc stream and apply RELATIVE relocations in-process.
//
// The decoder is a faithful port of Android's `for_all_packed_relocs` /
// `Unpacker` (AOSP linker), whose exact byte format was validated against the
// real 2.726.1142 libroblox.so in Session 14 via `unpack_rela.py`.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// Program-header / dynamic-section constants (self-contained here so the
// module does not depend on elf.rs's private constants).
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const DT_NULL: u64 = 0;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_ANDROID_RELA: u64 = 0x6000_0011;
const DT_ANDROID_RELASZ: u64 = 0x6000_0012;

/// AArch64 relocation type for `*(P) = B + A` (place-based absolute).
pub const R_AARCH64_RELATIVE: u64 = 1027;

// APS2 group-flag bits (Android linker_reloc_iterators.h).
const GROUPED_BY_INFO: i64 = 1;
const GROUPED_BY_OFFSET_DELTA: i64 = 2;
const GROUPED_BY_ADDEND: i64 = 4;
const GROUP_HAS_ADDEND: i64 = 8;

/// A decoded relocation in standard ELF form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rela {
    /// Link-time virtual address of the word being relocated (target).
    pub r_offset: u64,
    /// Encoded symbol-index `<31:8>` | relocation type `<7:0>`.
    pub r_info: u64,
    /// The addend.
    pub r_addend: i64,
}

impl Rela {
    /// The relocation type: the low 32 bits of `r_info`.
    pub fn r_type(&self) -> u64 {
        self.r_info & 0xffff_ffff
    }
}

/// Decode one SLEB128 value from `data` starting at `*offset`, advancing it.
fn read_sleb128(data: &[u8], offset: &mut usize) -> anyhow::Result<i64> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *offset >= data.len() {
            anyhow::bail!("SLEB128 ran out of data at offset {}", *offset);
        }
        let byte = data[*offset];
        *offset += 1;
        result |= ((byte & 0x7f) as i64).wrapping_shl(shift);
        let next_shift = shift.checked_add(7).unwrap_or(64);
        if byte & 0x80 == 0 {
            // Terminal byte: its bit6 is the value's sign (bit6 of the last
            // payload byte; ignore for continuation bytes, which aren't the
            // sign-determining one).
            if next_shift < 64 && (byte & 0x40) != 0 {
                result |= -(1i64 << next_shift);
            }
            break;
        }
        shift = next_shift;
    }
    Ok(result)
}

/// Decode an APS2-packed relocation stream into standard RELA entries.
///
/// The stream begins with the 4-byte magic `APS2`, then (all SLEB128):
/// `num_relocs`, a running `r_offset`, then a series of groups. Returns the
/// materialized `Vec<Rela>`. The format mirrors AOSP
/// `linker_reloc_iterators.h` `for_all_packed_relocs`.
pub fn decode_aps2(data: &[u8]) -> anyhow::Result<Vec<Rela>> {
    if data.len() < 4 || &data[0..4] != b"APS2" {
        anyhow::bail!("not APS2 packed reloc data (bad magic)");
    }
    let mut off = 4usize;

    let num_relocs = read_sleb128(data, &mut off)?;
    if num_relocs < 0 {
        anyhow::bail!("negative APS2 relocation count {}", num_relocs);
    }
    let num_relocs = num_relocs as usize;
    let mut r_offset: i64 = read_sleb128(data, &mut off)?;
    let mut r_info: u64 = 0;
    let mut r_addend: i64 = 0;

    let mut entries: Vec<Rela> = Vec::with_capacity(num_relocs.min(1 << 20));
    let mut idx = 0usize;

    while idx < num_relocs {
        let group_size = read_sleb128(data, &mut off)?;
        let group_flags = read_sleb128(data, &mut off)?;
        if group_size == 0 && group_flags == 0 {
            break;
        }
        if group_size < 0 {
            anyhow::bail!("negative APS2 group size {}", group_size);
        }

        let mut group_r_offset_delta: i64 = 0;
        if group_flags & GROUPED_BY_OFFSET_DELTA != 0 {
            group_r_offset_delta = read_sleb128(data, &mut off)?;
        }
        if group_flags & GROUPED_BY_INFO != 0 {
            r_info = read_sleb128(data, &mut off)? as u64;
        }

        let grouped_reloc = group_flags
            & (GROUP_HAS_ADDEND | GROUPED_BY_ADDEND);
        if grouped_reloc == (GROUP_HAS_ADDEND | GROUPED_BY_ADDEND) {
            r_addend += read_sleb128(data, &mut off)?;
        } else if grouped_reloc == 0 {
            r_addend = 0;
        }

        for _ in 0..group_size {
            if group_flags & GROUPED_BY_OFFSET_DELTA != 0 {
                r_offset += group_r_offset_delta;
            } else {
                r_offset += read_sleb128(data, &mut off)?;
            }
            if group_flags & GROUPED_BY_INFO == 0 {
                r_info = read_sleb128(data, &mut off)? as u64;
            }
            if grouped_reloc == GROUP_HAS_ADDEND {
                r_addend += read_sleb128(data, &mut off)?;
            }
            entries.push(Rela {
                r_offset: r_offset as u64,
                r_info,
                r_addend,
            });
        }
        idx += group_size as usize;
    }

    if entries.len() != num_relocs {
        anyhow::bail!(
            "APS2 declared {} relocations but decoded {}",
            num_relocs,
            entries.len()
        );
    }
    Ok(entries)
}

/// Locate the relocation stream declared by `PT_DYNAMIC` and materialize it.
///
/// Prefers `DT_ANDROID_RELA`/`DT_ANDROID_RELASZ` (APS2 packed) and falls back
/// to plain `DT_RELA`/`DT_RELASZ` (already-standard entries). Returns
/// `Ok(None)` when the ELF declares no relocation table.
pub fn read_elf_relocations(path: &Path) -> anyhow::Result<Option<Vec<Rela>>> {
    let mut f = std::fs::File::open(path)?;

    // --- ELF header ---
    let mut ehdr = [0u8; 64];
    f.read_exact(&mut ehdr)?;
    if &ehdr[0..4] != b"\x7fELF" {
        anyhow::bail!("not an ELF file: {}", path.display());
    }
    let e_phoff = u64::from_le_bytes(ehdr[32..40].try_into().unwrap());
    let e_phentsize = u16::from_le_bytes(ehdr[54..56].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(ehdr[56..58].try_into().unwrap()) as usize;

    // --- Program headers ---
    let mut phdrs = vec![0u8; e_phnum * e_phentsize];
    f.seek(SeekFrom::Start(e_phoff))?;
    f.read_exact(&mut phdrs)?;

    let phdr = |i: usize| &phdrs[i * e_phentsize..(i + 1) * e_phentsize];
    let p_type = |i: usize| {
        u32::from_le_bytes(phdr(i)[0..4].try_into().unwrap())
    };

    // --- PT_DYNAMIC ---
    let mut dyn_fo = None; // PT_DYNAMIC p_offset is already a FILE offset
    let mut dyn_filesz = 0usize;
    for i in 0..e_phnum {
        if p_type(i) == PT_DYNAMIC {
            dyn_fo = Some(u64::from_le_bytes(phdr(i)[8..16].try_into().unwrap()));
            dyn_filesz = u64::from_le_bytes(phdr(i)[32..40].try_into().unwrap()) as usize;
            break;
        }
    }
    let Some(dyn_fo) = dyn_fo else {
        return Ok(None); // no dynamic section -> no relocs
    };

    let mut dyn_data = vec![0u8; dyn_filesz];
    f.seek(SeekFrom::Start(dyn_fo))?;
    f.read_exact(&mut dyn_data)?;

    // --- Walk DT_* entries ---
    let mut rela_vaddr: Option<u64> = None;
    let mut relasz: Option<u64> = None;
    let mut source_android = false;
    for chunk in dyn_data.chunks_exact(16) {
        let tag = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let val = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        if tag == DT_NULL {
            break;
        }
        match tag {
            DT_ANDROID_RELA => {
                rela_vaddr = Some(val);
                source_android = true;
            }
            DT_ANDROID_RELASZ => relasz = Some(val),
            DT_RELA if rela_vaddr.is_none() => rela_vaddr = Some(val),
            DT_RELASZ if relasz.is_none() => relasz = Some(val),
            _ => {}
        }
    }

    let (rela_vaddr, relasz) = match (rela_vaddr, relasz) {
        (Some(v), Some(s)) => (v, s),
        _ => return Ok(None),
    };

    let rela_fo = vaddr_to_file_offset(&phdrs, e_phentsize, e_phnum, rela_vaddr)
        .context_no_dyn()?;
    let mut stream = vec![0u8; relasz as usize];
    f.seek(SeekFrom::Start(rela_fo))?;
    f.read_exact(&mut stream)?;

    if stream.len() >= 4 && &stream[0..4] == b"APS2" {
        tracing::info!(
            "{}: decoding {} bytes of APS2 packed relocations ({})\n",
            path.display(),
            stream.len(),
            if source_android { "DT_ANDROID_RELA" } else { "DT_RELA" }
        );
        return Ok(Some(decode_aps2(&stream)?));
    }

    // Fall through: the stream is already standard Elf64_Rela (24 bytes each).
    if stream.len() % 24 != 0 {
        anyhow::bail!(
            "{}: non-packed reloc stream size {} not a multiple of 24",
            path.display(),
            stream.len()
        );
    }
    let mut out = Vec::with_capacity(stream.len() / 24);
    for c in stream.chunks_exact(24) {
        out.push(Rela {
            r_offset: u64::from_le_bytes(c[0..8].try_into().unwrap()),
            r_info: u64::from_le_bytes(c[8..16].try_into().unwrap()),
            r_addend: i64::from_le_bytes(c[16..24].try_into().unwrap()),
        });
    }
    Ok(Some(out))
}

/// Apply `R_AARCH64_RELATIVE` relocations to an already-mapped JIT image.
///
/// `load_elf_image` maps a PIE at `base_addr` with **guest vaddr == host
/// address**, so a RELATIVE relocation `*(P) = B + A` is realized by writing
/// `load_bias + addend` (8 bytes, LE) at the host address of `r_offset`,
/// where `load_bias = base_addr - base_load_addr` is the ELF's runtime load
/// bias (see [`crate::elf::LoadedElf::guest_of`]). Targets that fall outside
/// any loaded segment are skipped. Callers must map the target silently —
/// RELATIVE targets live in `.data`/`.data.rel.ro`, which must currently be
/// writable. Returns the number of relocations applied.
pub fn apply_relatives(
    relas: &[Rela],
    base_addr: usize,
    base_load_addr: u64,
    target_of: impl Fn(u64) -> Option<u64>,
) -> usize {
    // runtime load bias B such that guest_of(link) == B + link.
    let load_bias = (base_addr as u64).wrapping_sub(base_load_addr);
    let mut applied = 0;
    for r in relas {
        if r.r_type() != R_AARCH64_RELATIVE {
            continue;
        }
        let value = load_bias.wrapping_add(r.r_addend as u64);
        if let Some(host) = target_of(r.r_offset) {
            // SAFETY: `host` is the address of `r_offset` inside the mapped
            // JIT image (guest==host), which the caller guarantees is current
            // writable for this window.
            unsafe {
                (host as *mut u64).write_unaligned(value);
            }
            applied += 1;
        }
    }
    applied
}

/// Small helper so the module compiles without `anyhow::Context` import churn.
trait ContextNoDyn<T> {
    fn context_no_dyn(self) -> anyhow::Result<T>;
}
impl<T> ContextNoDyn<T> for Option<T> {
    fn context_no_dyn(self) -> anyhow::Result<T> {
        self.ok_or_else(|| anyhow::anyhow!("DT_* vaddr not covered by a PT_LOAD"))
    }
}

/// Convert an ELF virtual address to a file offset using PT_LOAD headers.
fn vaddr_to_file_offset(
    phdrs: &[u8],
    e_phentsize: usize,
    e_phnum: usize,
    vaddr: u64,
) -> Option<u64> {
    let phdr = |i: usize| &phdrs[i * e_phentsize..(i + 1) * e_phentsize];
    for i in 0..e_phnum {
        let p_type = u32::from_le_bytes(phdr(i)[0..4].try_into().unwrap());
        if p_type != PT_LOAD {
            continue;
        }
        let p_offset = u64::from_le_bytes(phdr(i)[8..16].try_into().unwrap());
        let p_vaddr = u64::from_le_bytes(phdr(i)[16..24].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(phdr(i)[32..40].try_into().unwrap());
        if p_vaddr <= vaddr && vaddr < p_vaddr + p_filesz {
            return Some(vaddr - p_vaddr + p_offset);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-built APS2 stream (independently written byte-by-byte per the
    /// APS2 spec) decoding to three R_AARCH64_RELATIVE relocations with
    /// non-contiguous offsets and distinct addends. Guards against the
    /// decoder ("does it just mirror itself?"): the expected triples are
    /// computed here by hand, not produced by the encoder.
    #[test]
    fn aps2_golden_vector_decodes_hand_computed_relocs() {
        // magic + num_relocs=3 + init r_offset=0x2000
        //   group(3, flags=GROUP_HAS_ADDEND)
        //     +0x0000 -> 0x2000, info=0x403(RELATIVE), addend=+0x30
        //     +0x0008 -> 0x2008, info=0x403,            addend=+0x10 (0x40)
        //     -0x1008 -> 0x1000, info=0x403,            addend=+0x10 (0x50)
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"APS2");
        b.push(0x03); // num_relocs = 3
        b.extend_from_slice(&[0x80, 0xC0, 0x00]); // SLEB +0x2000 (3 bytes: 0x2000's
                                                  // bit13 would set final-byte bit6, so a
                                                  // sign-0 terminator byte is required)
        b.push(0x03); // group_size = 3
        b.push(0x08); // group_flags = GROUP_HAS_ADDEND only
        b.push(0x00); // delta 0x0000  (per-reloc offset, no offset-delta group)
        b.extend_from_slice(&[0x83, 0x08]); // info 0x403 (RELATIVE)
        b.push(0x30); // addend +0x30
        b.push(0x08); // delta +0x0008 -> 0x2008
        b.extend_from_slice(&[0x83, 0x08]);
        b.push(0x10); // addend +0x10 -> 0x40
        b.extend_from_slice(&[0xF8, 0x5F]); // SLEB -0x1008 -> 0x1000
        b.extend_from_slice(&[0x83, 0x08]);
        b.push(0x10); // addend +0x10 -> 0x50

        let relas = decode_aps2(&b).expect("decode golden APS2 stream");
        assert_eq!(
            relas,
            vec![
                Rela { r_offset: 0x2000, r_info: 0x403, r_addend: 0x30 },
                Rela { r_offset: 0x2008, r_info: 0x403, r_addend: 0x40 },
                Rela { r_offset: 0x1000, r_info: 0x403, r_addend: 0x50 },
            ]
        );
        for r in &relas {
            assert_eq!(r.r_type(), R_AARCH64_RELATIVE);
        }
    }

    /// Grouped-by-offset-delta + grouped-by-info encoding (the common shape a
    /// real linker emits: a run of same-type relocs at a fixed stride).
    #[test]
    fn aps2_offset_delta_group_replicates_stride() {
        // 4 relocs at 0x1000, 0x1008, 0x1010, 0x1018, same info(0x403).
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"APS2");
        b.push(0x04); // num_relocs
        b.extend_from_slice(&[0x80, 0x20]); // SLEB 0x1000
        b.push(0x04); // group_size = 4
        // flags = OFFSET_DELTA(2) | INFO(1) | GROUP_HAS_ADDEND(8) = 0xB
        b.push(0x0B);
        b.push(0x08); // group_r_offset_delta = +8
        b.extend_from_slice(&[0x83, 0x08]); // r_info = 0x403
        for _ in 0..4 {
            b.push(0); // per-reloc addend delta (GROUP_HAS_ADDEND) = 0
        }
        let relas = decode_aps2(&b).expect("decode delta-group");
        // In offset-delta grouping the running r_offset is `header + i*delta`,
        // so the relocs land at 0x1008, 0x1010, 0x1018, 0x1020.
        let expected: Vec<Rela> = (0..4)
            .map(|i| Rela {
                r_offset: 0x1008 + 8 * i,
                r_info: 0x403,
                r_addend: 0,
            })
            .collect();
        assert_eq!(relas, expected);
    }

    #[test]
    fn sleb128_handles_negative_values() {
        let mut off = 0usize;
        // -4104 == the SLEB bytes [0xF8, 0x5F] (see golden vector above).
        let v = read_sleb128(&[0xF8, 0x5F], &mut off).unwrap();
        assert_eq!(v, -4104);
        assert_eq!(off, 2);
    }

    #[test]
    fn rejects_non_aps2_magic() {
        assert!(decode_aps2(b"NOPE1234567890123456").is_err());
    }
}