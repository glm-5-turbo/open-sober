#!/usr/bin/env python3
"""
Automatic stub generator for missing Android NDK/GSI libraries.

Usage: python3 auto_stub.py <sysroot_lib64_dir>

Scans libraries in the sysroot for NEEDED entries that are missing,
and generates minimal stubs for them. Re-runs dlopen and discovers
more missing libs iteratively until the full chain is resolved.
"""

import subprocess, sys, os, tempfile, re

CROSS_CC = "aarch64-linux-gnu-gcc"

def get_needed_libs(lib_path):
    """Return set of library sonames needed by a shared library."""
    needed = set()
    try:
        out = subprocess.check_output(
            ["readelf", "-d", lib_path], stderr=subprocess.DEVNULL, text=True)
        for line in out.splitlines():
            m = re.search(r'NEEDED.*\[([^\]]+)\]', line)
            if m:
                needed.add(m.group(1))
    except: pass
    return needed

def find_lib(sysroot, soname):
    """Find a library by soname in the sysroot."""
    for root, dirs, files in os.walk(sysroot):
        for f in files:
            if f == soname:
                return os.path.join(root, f)
        for d in dirs:
            p = os.path.join(root, d, soname)
            if os.path.exists(p):
                return p
    return None

def find_missing_libs(sysroot):
    """Find all NEEDED entries not satisfied in sysroot."""
    all_libs = set()
    present = set()

    # Find all .so files
    for root, dirs, files in os.walk(sysroot):
        for f in files:
            if f.endswith(".so"):
                all_libs.add(f)
                present.add(f)
        for d in dirs:
            for f in os.listdir(os.path.join(root, d)):
                if f.endswith(".so"):
                    all_libs.add(f)
                    present.add(f)

    # Also check symlinks
    for root, dirs, files in os.walk(sysroot):
        for f in files:
            fp = os.path.join(root, f)
            if os.path.islink(fp):
                present.add(f)

    # Find all NEEDED entries
    needed = set()
    for root, dirs, files in os.walk(sysroot):
        for f in files:
            if f.endswith(".so"):
                needed |= get_needed_libs(os.path.join(root, f))

    # Also check libroblox
    roblox = os.path.expanduser("~/.cache/open-sober/libs/lib/libroblox.so")
    if os.path.exists(roblox):
        needed |= get_needed_libs(roblox)

    missing = needed - present
    return sorted(missing)

def create_stub(lib_name, output_dir):
    """Create a minimal stub .so for a missing library."""
    safe_name = lib_name.replace(".so", "").replace("-", "_").replace(".", "_")

    # Handle versioned sonames: libfoo.so.1 -> libfoo.so_1
    safe_name = re.sub(r'[^a-zA-Z0-9_]', '_', safe_name)

    stub_c = f"""
/* Auto-generated stub for {lib_name} */
typedef long unsigned int size_t;
void __stub_init(void) {{ }}
"""
    out_path = os.path.join(output_dir, lib_name)

    # Use nostdlib and ffreestanding to avoid conflicts
    result = subprocess.run(
        [CROSS_CC, "-shared", "-fPIC", "-o", out_path,
         "-x", "c", "-", "-nostdlib", "-ffreestanding"],
        input=stub_c, capture_output=True, text=True, timeout=30)

    if result.returncode == 0 and os.path.exists(out_path):
        return True
    return False

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <sysroot_lib64_dir>")
        sys.exit(1)

    sysroot = sys.argv[1]
    if not os.path.isdir(sysroot):
        print(f"Error: {sysroot} not found")
        sys.exit(1)

    round = 0
    while True:
        round += 1
        missing = find_missing_libs(sysroot)
        if not missing:
            print(f"\nRound {round}: No missing libraries! Dependency chain resolved.")
            break

        print(f"\nRound {round}: {len(missing)} missing libraries")
        for lib in missing:
            print(f"  Creating stub: {lib}")
            if create_stub(lib, sysroot):
                print(f"    ✓ {lib}")
            else:
                print(f"    ✗ {lib} FAILED")

        if round >= 10:
            print("Too many rounds — might be circular dependency")
            break

    print("\nDone. All dependencies should now be satisfied.")

if __name__ == "__main__":
    main()