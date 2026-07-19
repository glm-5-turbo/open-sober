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
    # DETECT: DT_INIT_ARRAY/DT_FINI_ARRAY tag presence alone causes glibc call_init
    # to crash (it checks l_info presence, not value). Even with value=0 and
    # DT_INIT_ARRAYSZ=0, the tag existing makes it jump to base+0 = SIGILL.
    if 25 in tags or 26 in tags or 27 in tags or 28 in tags:  # INIT/FINI ARRAY/ARRAYSZ
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
    # CRITICAL: Glibc's call_init checks l_info[DT_INIT_ARRAY] presence (not value),
    # so zeroing the value is NOT enough — it still sees the tag and jumps to base+0.
    # We must replace the ENTIRE dynamic entry (tag + value) with DT_NULL (0, 0).
    # However, replacing entries with DT_NULL can terminate the .dynamic scan early
    # (DT_NULL is the terminator), so if INIT/FINI entries come BEFORE VERSYM/VERNEED,
    # we must restructure: move nulled INIT/FINI entries to after VERSYM/VERNEED.
    init_entries = []  # (file_offset, name) for INIT/FINI entries
    versym_entries = []  # (file_offset, name, tag, val) for VERSYM/VERNEED/VERNEEDNUM
    other_entries = []  # (file_offset, tag, val) for everything else
    null_offset = None  # file offset of the DT_NULL terminator

    # First pass: read all entries up to the terminator
    for off in range(dyn_fo, dyn_fo + dyn_sz, 16):
        tag = struct.unpack('<Q', elf_data[off:off+8])[0]
        val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
        if tag == 0:
            null_offset = off
            break

        init_name = {25: "DT_INIT_ARRAY", 26: "DT_FINI_ARRAY", 27: "DT_INIT_ARRAYSZ", 28: "DT_FINI_ARRAYSZ"}.get(tag)
        versym_name = {0x6ffffff0: "VERSYM", 0x6ffffffc: "VERDEF", 0x6ffffffd: "VERDEFNUM", 0x6ffffffe: "VERNEED", 0x6fffffff: "VERNEEDNUM"}.get(tag)

        if init_name:
            init_entries.append((off, init_name))
        elif versym_name:
            versym_entries.append((off, versym_name, tag, val))
        elif tag == 12:  # DT_INIT — set value to 0 in place (safe)
            struct.pack_into('<Q', elf_data, off+8, 0)
            other_entries.append((off, tag, 0))
        else:
            other_entries.append((off, tag, val))

    if init_entries:
        print(f"  Found {len(init_entries)} INIT/FINI entries, {len(versym_entries)} version entries")

        # If any VERSYM entries come AFTER the INIT entries, the scan will
        # terminate early when we null the INIT entries. Restructure.
        last_init_off = max(off for off, _ in init_entries)
        if versym_entries and min(off for off, _, _, _ in versym_entries) > last_init_off:
            print("  ▲ VERSYM/VERNEED after INIT/FINI — restructuring .dynamic section")

            # Build new dynamic layout: other + versym + nulled_INIT + NULL
            new_section = bytearray()
            # Fix flags during restructure too
            for _, tag, val in other_entries:
                if tag == 0x1e:  # DT_FLAGS
                    val = val & ~(2 | 8)  # Clear DF_BIND_NOW, DF_SYMBOLIC
                elif tag == 0x6ffffffb:  # DT_FLAGS_1
                    val = val & ~1  # Clear DF_1_NOW
                new_section.extend(struct.pack('<QQ', tag, val))
            for _, _, tag, val in versym_entries:
                new_section.extend(struct.pack('<QQ', tag, val))
            for _ in init_entries:
                new_section.extend(struct.pack('<QQ', 0, 0))  # DT_NULL
            new_section.extend(struct.pack('<QQ', 0, 0))  # DT_NULL terminator

            # Compute how many extra bytes needed
            old_end = null_offset + 16  # past the original NULL
            new_size = len(new_section)
            old_size = null_offset - dyn_fo + 16
            extra = new_size - old_size

            if extra > 0:
                # Need to insert bytes — rewrite everything from dynamic onward
                print(f"  Inserting {extra} bytes in .dynamic section")
                # Build the new file: data before .dynamic + new dynamic + data after .dynamic
                before_dyn = bytearray(elf_data[:dyn_fo])
                after_dyn = bytearray(elf_data[old_end:])
                elf_data = before_dyn + new_section + after_dyn

                # Update PT_DYNAMIC filesz/memsz
                for i in range(e_phnum):
                    poff = e_phoff + i * e_phentsize
                    pt = struct.unpack('<I', elf_data[poff:poff+4])[0]
                    if pt == 2:
                        struct.pack_into('<Q', elf_data, poff+32, new_size)
                        struct.pack_into('<Q', elf_data, poff+40, new_size)
                        break

                # Update containing PT_LOAD filesz if needed
                for i in range(e_phnum):
                    poff = e_phoff + i * e_phentsize
                    pt = struct.unpack('<I', elf_data[poff:poff+4])[0]
                    if pt == 1:
                        p_vaddr = struct.unpack('<Q', elf_data[poff+16:poff+24])[0]
                        p_filesz = struct.unpack('<Q', elf_data[poff+32:poff+40])[0]
                        if p_vaddr <= dyn_vaddr < p_vaddr + p_filesz:
                            new_p_filesz = p_filesz + extra
                            struct.pack_into('<Q', elf_data, poff+32, new_p_filesz)
                            print(f"  PT_LOAD[{i}] filesz: 0x{p_filesz:x} → 0x{new_p_filesz:x}")
                            break

                # Update segment endings for subsequent LOAD segments
                # (their offsets shift by extra bytes)
                for i in range(e_phnum):
                    poff = e_phoff + i * e_phentsize
                    pt = struct.unpack('<I', elf_data[poff:poff+4])[0]
                    if pt == 1:
                        p_offset = struct.unpack('<Q', elf_data[poff+8:poff+16])[0]
                        p_filesz = struct.unpack('<Q', elf_data[poff+32:poff+40])[0]
                        # If this segment starts AFTER our insertion point, shift it
                        if p_offset > dyn_fo:
                            struct.pack_into('<Q', elf_data, poff+8, p_offset + extra)
            else:
                # The new section fits in place — just write it
                elf_data[dyn_fo:dyn_fo + new_size] = new_section

            for _, name in init_entries:
                print(f"  ✓ {name} → DT_NULL (moved after VERSYM)")
            for _, name, _, _ in versym_entries:
                print(f"  ✓ {name} preserved")
        else:
            # VERSYM is before INIT entries — simple nulling is safe
            for off, name in init_entries:
                struct.pack_into('<QQ', elf_data, off, 0, 0)
                print(f"  ✓ {name} → DT_NULL")
    else:
        # No INIT/FINI entries — also null DT_INIT if present
        pass  # already handled in the first pass loop

    # Update has_init flag for the rest of the function
    has_init = bool(init_entries)

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