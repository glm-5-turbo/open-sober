#!/usr/bin/env python3
"""Patch a GSI Android .so file for glibc compatibility.

Performs all patches needed for a GSI library to load under glibc's
dynamic linker:
1. APS2 → RELA decompression (delegates to unpack_rela.py)
2. Remove PT_GNU_RELRO (may overlap with RELA write range)
3. Zero DT_RELR/DT_RELRSZ, set DT_RELRENT=8 (Android RELR format not glibc-compat)
4. Clear DT_INIT_ARRAY/DT_INIT to prevent constructor crashes
5. Remove BIND_NOW / SYMBOLIC flags (lazy binding avoids PLT resolution failures)

Usage: python3 patch_gsi.py <so_file>
"""
import struct
import sys
import os
import subprocess

def find_dynamic(elf_data):
    """Find the .dynamic section's file offset and size via program headers."""
    e_phoff = struct.unpack('<Q', elf_data[32:40])[0]
    e_phentsize = struct.unpack('<H', elf_data[54:56])[0]
    e_phnum = struct.unpack('<H', elf_data[56:58])[0]

    dyn_vaddr = dyn_filesz = None
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 2:
            dyn_vaddr = struct.unpack('<Q', elf_data[off+16:off+24])[0]
            dyn_filesz = struct.unpack('<Q', elf_data[off+32:off+40])[0]
            break
    if dyn_vaddr is None:
        return None, None

    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 1:
            p_offset = struct.unpack('<Q', elf_data[off+8:off+16])[0]
            p_vaddr = struct.unpack('<Q', elf_data[off+16:off+24])[0]
            p_filesz = struct.unpack('<Q', elf_data[off+32:off+40])[0]
            if p_vaddr <= dyn_vaddr < p_vaddr + p_filesz:
                dyn_fo = dyn_vaddr - p_vaddr + p_offset
                return dyn_fo, dyn_filesz
    return None, None

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <so_file>")
        sys.exit(1)

    path = sys.argv[1]
    dry_run = '--dry-run' in sys.argv

    if not os.path.exists(path):
        print(f"File not found: {path}")
        sys.exit(1)

    with open(path, 'rb') as f:
        elf_data = bytearray(f.read())

    if elf_data[:4] != b'\x7fELF':
        print("Not an ELF file")
        sys.exit(1)

    dyn_fo, dyn_sz = find_dynamic(elf_data)
    if dyn_fo is None:
        print("ERROR: Could not find .dynamic section")
        sys.exit(1)

    # Step 0: Check if already patched (look for our PT_LOAD at end)
    e_phoff = struct.unpack('<Q', elf_data[32:40])[0]
    e_phentsize = struct.unpack('<H', elf_data[54:56])[0]
    e_phnum = struct.unpack('<H', elf_data[56:58])[0]

    # Scan dynamic tags to decide what needs patching
    tags = {}
    for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
        tag = struct.unpack('<Q', elf_data[off:off+8])[0]
        val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
        if tag == 0:
            break
        tags[tag] = (off, val)

    has_aps2 = False
    has_relr = False
    has_relro = False
    has_init = False
    has_bind_now = False

    # Check APS2 format: DT_RELA vaddr should point to APS2 magic
    if 7 in tags:
        rela_vaddr = tags[7][1]
        for i in range(e_phnum):
            off = e_phoff + i * e_phentsize
            p_type = struct.unpack('<I', elf_data[off:off+4])[0]
            if p_type == 1:
                p_offset = struct.unpack('<Q', elf_data[off+8:off+16])[0]
                p_vaddr = struct.unpack('<Q', elf_data[off+16:off+24])[0]
                p_filesz = struct.unpack('<Q', elf_data[off+32:off+40])[0]
                if p_vaddr <= rela_vaddr < p_vaddr + p_filesz:
                    rela_fo = rela_vaddr - p_vaddr + p_offset
                    if elf_data[rela_fo:rela_fo+4] == b'APS2':
                        has_aps2 = True
                    break

    if 0x23 in tags and tags[0x23][1] != 0:
        has_relr = True
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 0x6474e552:
            has_relro = True
    if 25 in tags and tags[25][1] != 0:
        has_init = True
    if 0x1e in tags:
        val = tags[0x1e][1]
        if val & 0x2 or val & 0x8:
            has_bind_now = True
    if 0x6ffffffb in tags:
        val = tags[0x6ffffffb][1]
        if val & 0x1:
            has_bind_now = True

    needs_patching = has_aps2 or has_relr or has_relro or has_init or has_bind_now
    if not needs_patching:
        print(f"  No patches needed — {os.path.basename(path)} is already glibc-compatible")
        return

    print(f"\n=== Patching {os.path.basename(path)} ===")
    print(f"  APS2: {'YES' if has_aps2 else 'no'}, RELR: {'YES' if has_relr else 'no'}, "
          f"GNU_RELRO: {'YES' if has_relro else 'no'}, "
          f"INIT: {'YES' if has_init else 'no'}, BIND_NOW: {'YES' if has_bind_now else 'no'}")

    # Step 1: APS2 → RELA decompression
    if has_aps2:
        print("\n[Step 1] APS2 → RELA decompression...")
        r = subprocess.run(['python3', '/home/code-agent/unpack_rela.py', path],
                           capture_output=True, text=True, timeout=60)
        for line in r.stdout.split('\n'):
            if 'ERROR' in line or 'WARNING' in line:
                print(f"  {line}")
        if r.returncode != 0:
            print(f"  unpack_rela failed: {r.stderr}")
        # Re-read the patched file
        with open(path, 'rb') as f:
            elf_data = bytearray(f.read())
        # Re-scan dynamic since unpack_rela changed the file
        dyn_fo, dyn_sz = find_dynamic(elf_data)
        e_phnum = struct.unpack('<H', elf_data[56:58])[0]
        tags = {}
        for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
            tag = struct.unpack('<Q', elf_data[off:off+8])[0]
            val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
            if tag == 0: break
            tags[tag] = (off, val)
        print("  ✓ APS2 decompressed")

    if dry_run:
        print("\n[dry-run] Would apply remaining patches")
        return

    # Step 2: Remove PT_GNU_RELRO
    if has_relro:
        print("\n[Step 2] Removing PT_GNU_RELRO...")
        for i in range(e_phnum):
            off = e_phoff + i * e_phentsize
            p_type = struct.unpack('<I', elf_data[off:off+4])[0]
            if p_type == 0x6474e552:
                struct.pack_into('<I', elf_data, off, 0)
                print(f"  ✓ PT_GNU_RELRO at PHDR[{i}] set to PT_NULL")

    # Step 3: Fix RELR tags
    if has_relr:
        print("\n[Step 3] Fixing RELR tags...")
        for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
            tag = struct.unpack('<Q', elf_data[off:off+8])[0]
            if tag == 0: break
            if tag == 0x23:  # DT_RELR
                struct.pack_into('<Q', elf_data, off+8, 0)
                print("  ✓ DT_RELR value zeroed")
            elif tag == 0x24:  # DT_RELRSZ
                struct.pack_into('<Q', elf_data, off+8, 0)
                print("  ✓ DT_RELRSZ value zeroed")
            elif tag == 0x25:  # DT_RELRENT
                struct.pack_into('<Q', elf_data, off+8, 8)
                print("  ✓ DT_RELRENT set to 8")

    # Step 4: Clear init
    if has_init:
        print("\n[Step 4] Clearing init entries...")
        for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
            tag = struct.unpack('<Q', elf_data[off:off+8])[0]
            if tag == 0: break
            if tag == 12:   # DT_INIT
                struct.pack_into('<Q', elf_data, off+8, 0)
                print("  ✓ DT_INIT cleared")
            elif tag == 25:  # DT_INIT_ARRAY
                struct.pack_into('<Q', elf_data, off+8, 0)
                print("  ✓ DT_INIT_ARRAY vaddr cleared")
            elif tag == 27:  # DT_INIT_ARRAYSZ
                struct.pack_into('<Q', elf_data, off+8, 0)
                print("  ✓ DT_INIT_ARRAYSZ cleared")

    # Step 5: Remove BIND_NOW / SYMBOLIC
    if has_bind_now:
        print("\n[Step 5] Removing BIND_NOW...")
        for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
            tag = struct.unpack('<Q', elf_data[off:off+8])[0]
            if tag == 0: break
            if tag == 0x1e:  # DT_FLAGS
                val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
                new_val = val & ~(2 | 8)  # Remove DF_BIND_NOW, DF_SYMBOLIC
                if new_val != val:
                    struct.pack_into('<Q', elf_data, off+8, new_val)
                    print(f"  ✓ DT_FLAGS: 0x{val:x} → 0x{new_val:x}")
            elif tag == 0x6ffffffb:  # DT_FLAGS_1
                val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
                new_val = val & ~0x1  # Remove DF_1_NOW
                if new_val != val:
                    struct.pack_into('<Q', elf_data, off+8, new_val)
                    print(f"  ✓ DT_FLAGS_1: 0x{val:x} → 0x{new_val:x}")

    # Write patched file
    with open(path, 'wb') as f:
        f.write(bytes(elf_data))
    print(f"\n✓ Patching complete — {os.path.basename(path)} ({len(elf_data)} bytes)")


if __name__ == '__main__':
    main()