#!/usr/bin/env python3
"""
Unpack Android packed relocations (APS2 format) into standard RELA entries.

Uses the exact decoder from Android's linker_reloc_iterators.h (for_all_packed_relocs).

Usage: python3 unpack_rela.py <so_file>

The decoder handles APS2 packed RELA found in Android GSI shared libraries.
After decoding, the standard 24-byte Elf64_Rela entries are appended to the file
with a new PT_LOAD segment added to map them. DT_RELA/DT_RELASZ in the dynamic
section are updated to point to the new location.
"""
import struct
import sys
import os
import subprocess

# Android linker flag constants
RELOCATION_GROUPED_BY_INFO_FLAG = 1
RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG = 2
RELOCATION_GROUPED_BY_ADDEND_FLAG = 4
RELOCATION_GROUP_HAS_ADDEND_FLAG = 8


def read_sleb128(data, offset):
    """Read SLEB128 value. Returns (value, new_offset)."""
    result = 0
    shift = 0
    last_byte = 0
    while True:
        if offset >= len(data):
            raise ValueError("SLEB128 ran out of data at offset %d" % offset)
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7f) << shift
        shift += 7
        last_byte = byte
        if not (byte & 0x80):
            break
    if shift < 64 and (last_byte & 0x40):
        result |= - (1 << shift)
    return result, offset


def get_reloc_type_name(r_type):
    names = {
        0: 'R_AARCH64_NONE',
        257: 'R_AARCH64_ABS64',
        1024: 'R_AARCH64_COPY',
        1025: 'R_AARCH64_GLOB_DAT',
        1026: 'R_AARCH64_JUMP_SLOT',
        1027: 'R_AARCH64_RELATIVE',
        1028: 'R_AARCH64_TLS_DTPMOD',
        1029: 'R_AARCH64_TLS_DTPREL',
        1030: 'R_AARCH64_TLS_TPREL',
        1031: 'R_AARCH64_TLSDESC',
        1032: 'R_AARCH64_IRELATIVE',
    }
    return names.get(r_type, 'R_AARCH64_%d' % r_type)


def decode_aps2(data):
    """Decode APS2 packed relocation data into standard RELA entries.

    Uses the exact decoder function from Android linker_reloc_iterators.h
    (for_all_packed_relocs). Returns list of (r_offset, r_info, r_addend) tuples.
    """
    if data[:4] != b'APS2':
        print("ERROR: Not APS2 format, got %s" % data[:4])
        return None

    offset = 4
    try:
        num_relocs, offset = read_sleb128(data, offset)
        r_offset = read_sleb128(data, offset)[0]
        offset = read_sleb128(data, offset)[1]
        print("APS2: %d entries, initial r_offset=0x%x" % (num_relocs, r_offset))

        entries = []
        r_info = 0
        r_addend = 0
        idx = 0

        while idx < num_relocs:
            group_size, offset = read_sleb128(data, offset)
            group_flags, offset = read_sleb128(data, offset)
            if group_size == 0 and group_flags == 0:
                break

            group_r_offset_delta = 0
            if group_flags & RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG:
                group_r_offset_delta, offset = read_sleb128(data, offset)
            if group_flags & RELOCATION_GROUPED_BY_INFO_FLAG:
                r_info, offset = read_sleb128(data, offset)

            group_flags_reloc = group_flags & (RELOCATION_GROUP_HAS_ADDEND_FLAG |
                                               RELOCATION_GROUPED_BY_ADDEND_FLAG)
            if group_flags_reloc == (RELOCATION_GROUP_HAS_ADDEND_FLAG |
                                     RELOCATION_GROUPED_BY_ADDEND_FLAG):
                r_addend += read_sleb128(data, offset)[0]
                offset = read_sleb128(data, offset)[1]
            elif group_flags_reloc == 0:
                r_addend = 0

            for i in range(group_size):
                if group_flags & RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG:
                    r_offset += group_r_offset_delta
                else:
                    delta, offset = read_sleb128(data, offset)
                    r_offset += delta

                if not (group_flags & RELOCATION_GROUPED_BY_INFO_FLAG):
                    r_info, offset = read_sleb128(data, offset)

                if group_flags_reloc == RELOCATION_GROUP_HAS_ADDEND_FLAG:
                    addend_delta, offset = read_sleb128(data, offset)
                    r_addend += addend_delta

                entries.append((r_offset, r_info, r_addend))

            idx += group_size

        print("Decoded %d entries" % len(entries))
        return entries

    except ValueError as e:
        print("ERROR: %s" % e)
        return None


def vaddr_to_file_offset(elf_data, e_phoff, e_phnum, e_phentsize, vaddr):
    """Convert a virtual address to file offset using PT_LOAD program headers."""
    for j in range(e_phnum):
        off = e_phoff + j * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 1:
            po = struct.unpack('<Q', elf_data[off+8:off+16])[0]
            pv = struct.unpack('<Q', elf_data[off+16:off+24])[0]
            pf = struct.unpack('<Q', elf_data[off+32:off+40])[0]
            if pv <= vaddr < pv + pf:
                return vaddr - pv + po
    return None


def main():
    if len(sys.argv) < 2:
        print("Usage: %s <so_file>" % sys.argv[0])
        sys.exit(1)

    path = sys.argv[1]
    dry_run = '--dry-run' in sys.argv

    if not os.path.exists(path):
        print("File not found: %s" % path)
        sys.exit(1)

    with open(path, 'rb') as f:
        elf_data = f.read()

    if elf_data[:4] != b'\x7fELF':
        print("Not an ELF file")
        sys.exit(1)

    # Parse ELF headers
    e_phoff = struct.unpack('<Q', elf_data[32:40])[0]
    e_phnum = struct.unpack('<H', elf_data[56:58])[0]
    e_phentsize = struct.unpack('<H', elf_data[54:56])[0]

    # Find dynamic section
    dyn_vaddr = None
    dyn_filesz = None
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 2:
            dyn_vaddr = struct.unpack('<Q', elf_data[off+16:off+24])[0]
            dyn_filesz = struct.unpack('<Q', elf_data[off+32:off+40])[0]
            break

    dyn_fo = vaddr_to_file_offset(elf_data, e_phoff, e_phnum, e_phentsize, dyn_vaddr)
    if dyn_fo is None:
        print("ERROR: Could not find dynamic section")
        sys.exit(1)

    # Find DT_RELA and DT_RELASZ
    rela_vaddr = None
    relasz = None
    for entry_off in range(dyn_fo, dyn_fo + dyn_filesz, 16):
        tag = struct.unpack('<Q', elf_data[entry_off:entry_off+8])[0]
        val = struct.unpack('<Q', elf_data[entry_off+8:entry_off+16])[0]
        if tag == 0: break
        if tag == 7: rela_vaddr = val
        elif tag == 8: relasz = val

    if rela_vaddr is None or relasz is None:
        print("No DT_RELA found")
        sys.exit(1)

    print("DT_RELA vaddr=0x%x, DT_RELASZ=%d" % (rela_vaddr, relasz))

    # Convert to file offset
    rela_fo = vaddr_to_file_offset(elf_data, e_phoff, e_phnum, e_phentsize, rela_vaddr)
    if rela_fo is None:
        print("ERROR: Could not find file offset for RELA")
        sys.exit(1)

    print("RELA at file offset 0x%x" % rela_fo)

    # Read APS2 and decode
    aps2_data = elf_data[rela_fo:rela_fo + relasz]
    if aps2_data[:4] != b'APS2':
        # Check if already standard RELA
        ro = struct.unpack('<Q', aps2_data[:8])[0]
        ri = struct.unpack('<Q', aps2_data[8:16])[0]
        if 0 < ro < 0x200000 and (ri & 0xffffffff) in (1027, 1025, 257):
            print("Already standard RELA format. Nothing to do.")
            return
        print("WARNING: Not APS2 format at expected offset. Trying anyway...")

    entries = decode_aps2(aps2_data)
    if entries is None:
        sys.exit(1)

    # Print statistics
    from collections import Counter
    type_counts = Counter(e[1] & 0xffffffff for e in entries)
    print("\nRelocation type distribution:")
    for t, c in sorted(type_counts.items(), key=lambda x: -x[1]):
        print("  %s: %d" % (get_reloc_type_name(t), c))
    offsets = [e[0] for e in entries]
    print("Offset range: 0x%x - 0x%x" % (min(offsets), max(offsets)))

    if dry_run:
        print("\nFirst 5 entries:")
        for i, (ro, ri, ra) in enumerate(entries[:5]):
            print("  [%d] offset=0x%x info=0x%x type=%s addend=0x%x" %
                  (i, ro, ri, get_reloc_type_name(ri & 0xffffffff), ra))
        print("Would write %d bytes of standard RELA" % (len(entries) * 24))
        return

    # Build standard RELA entries (24 bytes each for Elf64_Rela)
    new_rela = bytearray()
    for r_offset, r_info, r_addend in entries:
        new_rela.extend(struct.pack('<Q', r_offset))
        new_rela.extend(struct.pack('<Q', r_info))
        new_rela.extend(struct.pack('<q', r_addend))

    new_relasz = len(new_rela)
    print("\nNew RELA: %d bytes (%d entries)" % (new_relasz, len(entries)))

    # Strategy: Append new RELA data after the file, add a new PT_LOAD segment
    # to map it, update DT_RELA/DT_RELASZ.

    new_file = bytearray(elf_data)

    # Find the highest existing segment end for vaddr placement
    page_size = 0x4000  # AArch64 page size
    max_end = 0
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', new_file[off:off+4])[0]
        if p_type == 1:
            p_vaddr = struct.unpack('<Q', new_file[off+16:off+24])[0]
            p_memsz = struct.unpack('<Q', new_file[off+40:off+48])[0]
            end = p_vaddr + p_memsz
            if end > max_end:
                max_end = end

    new_rela_vaddr = (max_end + page_size - 1) & ~(page_size - 1)
    print("New vaddr: 0x%x (past end 0x%x)" % (new_rela_vaddr, max_end))

    # Pad file to make the new file offset page-aligned (matching vaddr alignment)
    while len(new_file) % page_size != 0:
        new_file.append(0)

    new_rela_fo = len(new_file)
    print("New file offset: 0x%x" % new_rela_fo)

    # Verify alignment constraint: (vaddr - offset) % align == 0
    if (new_rela_vaddr - new_rela_fo) % page_size != 0:
        print("WARNING: vaddr and file offset alignment mismatch!")

    # Append RELA data
    new_file.extend(new_rela)

    # Add new PT_LOAD segment
    # PT_PHDR filesz may or may not have room; increment e_phnum regardless
    # (the kernel/loader doesn't strictly check PT_PHDR filesz)
    new_phdr_off = e_phoff + e_phnum * e_phentsize
    struct.pack_into('<H', new_file, 56, e_phnum + 1)
    struct.pack_into('<I', new_file, new_phdr_off+0, 1)        # PT_LOAD
    struct.pack_into('<I', new_file, new_phdr_off+4, 4)        # PF_R
    struct.pack_into('<Q', new_file, new_phdr_off+8, new_rela_fo)  # p_offset
    struct.pack_into('<Q', new_file, new_phdr_off+16, new_rela_vaddr)  # p_vaddr
    struct.pack_into('<Q', new_file, new_phdr_off+24, new_rela_vaddr)  # p_paddr
    struct.pack_into('<Q', new_file, new_phdr_off+32, new_relasz)  # p_filesz
    struct.pack_into('<Q', new_file, new_phdr_off+40, new_relasz)  # p_memsz
    struct.pack_into('<Q', new_file, new_phdr_off+48, page_size)   # p_align
    print("Added PT_LOAD[%d]: offset=0x%x vaddr=0x%x filesz=%d" %
          (e_phnum, new_rela_fo, new_rela_vaddr, new_relasz))

    # Also update PT_PHDR filesz if it exists
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        if struct.unpack('<I', new_file[off:off+4])[0] == 6:
            old_sz = struct.unpack('<Q', new_file[off+32:off+40])[0]
            new_sz = old_sz + e_phentsize
            struct.pack_into('<Q', new_file, off+32, new_sz)
            struct.pack_into('<Q', new_file, off+40, new_sz)
            print("Updated PT_PHDR filesz: 0x%x -> 0x%x" % (old_sz, new_sz))
            break

    # Update dynamic section: DT_RELA vaddr and DT_RELASZ
    # Note: dyn_fo was computed from the ORIGINAL file, still valid
    # since we appended (didn't shift).
    for entry_off in range(dyn_fo, dyn_fo + dyn_filesz, 16):
        if entry_off + 16 > len(new_file): break
        tag = struct.unpack('<Q', new_file[entry_off:entry_off+8])[0]
        val = struct.unpack('<Q', new_file[entry_off+8:entry_off+16])[0]
        if tag == 0: break
        if tag == 7:
            struct.pack_into('<Q', new_file, entry_off+8, new_rela_vaddr)
            print("  DT_RELA: 0x%x -> 0x%x" % (val, new_rela_vaddr))
        elif tag == 8:
            struct.pack_into('<Q', new_file, entry_off+8, new_relasz)
            print("  DT_RELASZ: %d -> %d" % (val, new_relasz))

    # Write the patched file
    with open(path, 'wb') as f:
        f.write(bytes(new_file))
    print("\nWritten %d bytes" % len(new_file))

    # Verify
    r = subprocess.run(['aarch64-linux-gnu-readelf', '-d', path],
                       capture_output=True, text=True, timeout=10)
    for line in r.stdout.split('\n'):
        if 'RELA' in line or 'JMPREL' in line:
            print("  " + line.strip())

    r2 = subprocess.run(['aarch64-linux-gnu-readelf', '-r', '--use-dynamic', path],
                        capture_output=True, text=True, timeout=10)
    relocs = [l for l in r2.stdout.split('\n') if 'R_AARCH64' in l]
    print("  .rela.dyn entries: %d (plus .rela.plt entries shown by readelf)" % len(entries))
    if relocs:
        print("  First: " + relocs[0].strip())
        print("  Last:  " + relocs[-1].strip())


if __name__ == '__main__':
    main()