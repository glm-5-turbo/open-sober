#!/usr/bin/env python3
"""Patch ANDROID_RELR DT tags to standard RELR for glibc compatibility.

Finds the .dynamic section via ELF program headers (PT_DYNAMIC segment)
instead of hardcoded offsets."""
import struct
import sys

if len(sys.argv) < 2:
    print(f"Usage: {sys.argv[0]} <so_file>")
    sys.exit(1)

path = sys.argv[1]
with open(path, 'r+b') as f:
    data = bytearray(f.read())

    # Parse ELF header
    if data[0:4] != b'\x7fELF':
        print("Not an ELF file")
        sys.exit(1)

    is_64bit = data[4] == 2  # ELFCLASS64
    is_le = data[5] == 1      # ELFDATA2LSB

    if not is_le:
        print("Only little-endian supported")
        sys.exit(1)

    if not is_64bit:
        print("Only 64-bit ELF supported")
        sys.exit(1)

    # 64-bit ELF: e_phoff at offset 32 (8 bytes), e_phnum at offset 56 (2 bytes), e_phentsize at offset 54 (2 bytes)
    e_phoff = struct.unpack('<Q', data[32:40])[0]
    e_phnum = struct.unpack('<H', data[56:58])[0]
    e_phentsize = struct.unpack('<H', data[54:56])[0]

    # Find PT_DYNAMIC segment
    dyn_vaddr = None
    dyn_filesz = None
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', data[off:off+4])[0]
        if p_type == 2:  # PT_DYNAMIC
            if is_64bit:
                p_vaddr = struct.unpack('<Q', data[off+16:off+24])[0]
                p_filesz = struct.unpack('<Q', data[off+32:off+40])[0]
            else:
                p_vaddr = struct.unpack('<I', data[off+8:off+12])[0]
                p_filesz = struct.unpack('<I', data[off+20:off+24])[0]
            dyn_vaddr = p_vaddr
            dyn_filesz = p_filesz
            # Now we need to figure out the file offset from vaddr
            # Find the PT_LOAD that contains this vaddr
            for j in range(e_phnum):
                off2 = e_phoff + j * e_phentsize
                p_type2 = struct.unpack('<I', data[off2:off2+4])[0]
                if p_type2 == 1:  # PT_LOAD
                    if is_64bit:
                        p_offset2 = struct.unpack('<Q', data[off2+8:off2+16])[0]
                        p_vaddr2 = struct.unpack('<Q', data[off2+16:off2+24])[0]
                        p_filesz2 = struct.unpack('<Q', data[off2+32:off2+40])[0]
                    else:
                        p_offset2 = struct.unpack('<I', data[off2+4:off2+8])[0]
                        p_vaddr2 = struct.unpack('<I', data[off2+8:off2+12])[0]
                        p_filesz2 = struct.unpack('<I', data[off2+16:off2+20])[0]

                    if p_vaddr2 <= dyn_vaddr < p_vaddr2 + p_filesz2:
                        dyn_file_off = dyn_vaddr - p_vaddr2 + p_offset2
                        break
            break

    if dyn_file_off is None:
        print("Could not find .dynamic section")
        sys.exit(1)

    print(f".dynamic at file offset 0x{dyn_file_off:x}, size 0x{dyn_filesz:x}")

    # Scan .dynamic for Android-specific DT tags that need conversion
    android_tags = {
        # ANDROID_RELR → RELR
        0x6fffe000: (0x24, 'DT_ANDROID_RELR', 'DT_RELR'),
        0x6fffe001: (0x23, 'DT_ANDROID_RELRSZ', 'DT_RELRSZ'),
        0x6fffe003: (0x25, 'DT_ANDROID_RELRENT', 'DT_RELRENT'),
        # ANDROID_RELA → RELA (AArch64-specific tags)
        0x60000011: (0x07, 'DT_ANDROID_RELA', 'DT_RELA'),
        0x60000012: (0x08, 'DT_ANDROID_RELASZ', 'DT_RELASZ'),
        0x60000013: (0x09, 'DT_ANDROID_RELAENT', 'DT_RELAENT'),
    }

    changes = 0
    for entry_off in range(dyn_file_off, dyn_file_off + dyn_filesz, 16):
        if entry_off + 16 > len(data):
            break
        tag = struct.unpack('<Q', data[entry_off:entry_off+8])[0]
        if tag in android_tags:
            new_tag, old_name, new_name = android_tags[tag]
            struct.pack_into('<Q', data, entry_off, new_tag)
            print(f"  {old_name} → {new_name}")
            changes += 1

    f.seek(0)
    f.write(bytes(data))
    f.truncate()
    print(f"Patched {changes} DT tags in {path}")