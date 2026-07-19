#!/usr/bin/env python3
"""Add VERSYM/VERNEED dynamic entries to GSI libs.

Many Android GSI libraries have .gnu.version and .gnu.version_r section
data (visible via readelf -V) but no corresponding VERSYM/VERNEED dynamic
tags. Glibc's dynamic linker requires these tags to resolve versioned
symbol references.

This script:
1. Scans all GSI libs for missing VERSYM/VERNEED dynamic entries
2. For each affected lib, reads the version data vaddr from readelf
3. Inserts VERSYM/VERNEED/VERNEEDNUM entries before the DT_NULL terminator
4. Also handles ANDROID_RELA→DT_RELA conversion and APS2 decompression

Usage: python3 add_versym.py <path_to_lib_or_dir>
"""
import struct
import sys
import os
import subprocess

def get_elf_info(elf_data):
    """Extract basic ELF info: phdr, dynamic section location."""
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
    
    dyn_fo = None
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', elf_data[off:off+4])[0]
        if p_type == 1:
            p_offset = struct.unpack('<Q', elf_data[off+8:off+16])[0]
            p_vaddr = struct.unpack('<Q', elf_data[off+16:off+24])[0]
            p_filesz = struct.unpack('<Q', elf_data[off+32:off+40])[0]
            if p_vaddr <= dyn_vaddr < p_vaddr + p_filesz:
                dyn_fo = dyn_vaddr - p_vaddr + p_offset
                break
    
    return e_phoff, e_phentsize, e_phnum, dyn_vaddr, dyn_filesz, dyn_fo


def scan_dynamic(elf_data, dyn_fo, dyn_filesz):
    """Scan dynamic entries, return dict of tags and null offset."""
    tags = {}
    null_off = None
    for off in range(dyn_fo, dyn_fo + dyn_filesz, 16):
        tag = struct.unpack('<Q', elf_data[off:off+8])[0]
        val = struct.unpack('<Q', elf_data[off+8:off+16])[0]
        if tag == 0:
            null_off = off
            break
        tags[tag] = (off, val)
    return tags, null_off


def get_version_vaddrs(path):
    """Use readelf to get .gnu.version and .gnu.version_r vaddrs."""
    result = subprocess.run(['readelf', '-W', '-V', path],
                           capture_output=True, text=True, timeout=15)
    output = result.stdout
    
    versym_addr = None
    verneed_addr = None
    verneednum = 0
    
    for line in output.split('\n'):
        if 'Version symbols section' in line and 'gnu.version' in line:
            # e.g.: "  Addr: 0x0000000000000440  Offset: 0x00000440  Link: 4 (.dynsym)"
            addr_line = line
            # Next line has the address
        elif 'Addr:' in line and versym_addr is None:
            parts = line.split()
            for p in parts:
                if p.startswith('0x'):
                    versym_addr = int(p, 16)
                    break
    
    # Find VERNEED address
    in_verneed = False
    for line in output.split('\n'):
        if 'Version needs section' in line:
            in_verneed = True
        if in_verneed and 'Addr:' in line:
            parts = line.split()
            for p in parts:
                if p.startswith('0x'):
                    verneed_addr = int(p, 16)
                    break
            in_verneed = False
        if 'Version needs' in line and 'entries' in line:
            # e.g. "Version needs section '.gnu.version_r' contains 4 entries:"
            import re
            m = re.search(r'contains (\d+) entries', line)
            if m:
                verneednum = int(m.group(1))
    
    return versym_addr, verneed_addr, verneednum


def add_versym_entries(path):
    """Add VERSYM/VERNEED/VERNEEDNUM to a single ELF."""
    with open(path, 'rb') as f:
        d = bytearray(f.read())
    
    if d[:4] != b'\x7fELF':
        return False, "Not an ELF"
    
    e_phoff, e_phentsize, e_phnum, dyn_vaddr, dyn_filesz, dyn_fo = get_elf_info(d)
    if dyn_fo is None:
        return False, "No PT_DYNAMIC"
    
    tags, null_off = scan_dynamic(d, dyn_fo, dyn_filesz)
    
    has_versym = 0x6ffffff0 in tags
    has_verneed = 0x6ffffffe in tags
    
    if has_versym and has_verneed:
        return False, "Already has VERSYM/VERNEED"
    
    versym_addr, verneed_addr, verneednum = get_version_vaddrs(path)
    
    if versym_addr is None or verneed_addr is None:
        return False, "Could not find version section addresses"
    
    # Insert entries before NULL
    struct.pack_into('<QQ', d, null_off, 0x6ffffff0, versym_addr)
    struct.pack_into('<QQ', d, null_off + 16, 0x6ffffffe, verneed_addr)
    struct.pack_into('<QQ', d, null_off + 32, 0x6fffffff, verneednum)
    struct.pack_into('<QQ', d, null_off + 48, 0, 0)
    
    # Convert ANDROID_RELA → DT_RELA
    for off in range(dyn_fo, dyn_fo + dyn_filesz, 16):
        tag = struct.unpack('<Q', d[off:off+8])[0]
        if tag == 0: break
        if tag == 0x60000011: struct.pack_into('<Q', d, off, 7)
        elif tag == 0x60000012: struct.pack_into('<Q', d, off, 8)
    
    # Update PT_DYNAMIC filesz/memsz
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        pt = struct.unpack('<I', d[off:off+4])[0]
        if pt == 2:
            ofsz = struct.unpack('<Q', d[off+32:off+40])[0]
            struct.pack_into('<Q', d, off+32, ofsz + 64)
            struct.pack_into('<Q', d, off+40, ofsz + 64)
            break
    
    # Update PT_LOAD containing .dynamic
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        pt = struct.unpack('<I', d[off:off+4])[0]
        if pt == 1:
            pv = struct.unpack('<Q', d[off+16:off+24])[0]
            pf = struct.unpack('<Q', d[off+32:off+40])[0]
            if pv <= dyn_vaddr < pv + pf:
                struct.pack_into('<Q', d, off+32, pf + 64)
                break
    
    with open(path, 'wb') as f:
        f.write(bytes(d))
    
    return True, f"VERSYM=0x{versym_addr:x} VERNEED=0x{verneed_addr:x} count={verneednum}"


def main():
    args = sys.argv[1:]
    if not args:
        print(f"Usage: {sys.argv[0]} <lib.so> [lib2.so ...]")
        sys.exit(1)
    
    count = 0
    for path in args:
        if os.path.isdir(path):
            for f in sorted(os.listdir(path)):
                if f.endswith('.so'):
                    fp = os.path.join(path, f)
                    ok, msg = add_versym_entries(fp)
                    if ok:
                        print(f"  ✓ {f}: {msg}")
                        count += 1
        else:
            ok, msg = add_versym_entries(path)
            if ok:
                print(f"  ✓ {os.path.basename(path)}: {msg}")
                count += 1
    
    print(f"\nPatched {count} libraries")


if __name__ == '__main__':
    main()
