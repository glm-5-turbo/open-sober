#!/usr/bin/env python3
"""Generate LIBC-versioned C forwarding stubs for bridge libc.so.

Each stub declares the glibc function as extern (unversioned reference),
defines a tail-call wrapper, and relies on the version script
(bridge_version.ver LIBC { global: *; }) to tag it with @@LIBC.

Usage:
    python3 gen_bridge_stubs.py <lib_to_analyze> <output.c>
"""

import subprocess
import sys


def get_undef_symbols(lib_path):
    result = subprocess.run(
        ['aarch64-linux-gnu-readelf', '-sW', '--wide', lib_path],
        capture_output=True, text=True)
    symbols = []
    for line in result.stdout.split('\n'):
        if 'UND' not in line:
            continue
        line = line.strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) < 8:
            continue
        raw_name = parts[-2] if (parts[-1].startswith('(') and parts[-1].endswith(')')) else parts[-1]
        version = None
        name = raw_name
        if '@@' in raw_name:
            name, version = raw_name.split('@@', 1)
        elif '@' in raw_name:
            name, version = raw_name.split('@', 1)
        if not version:
            continue
        if not version.startswith('LIBC') and version != 'LIBDL_ANDROID':
            continue
        sym_type = 'FUNC' if 'FUNC' in line else ('OBJECT' if 'OBJECT' in line else 'OTHER')
        symbols.append({'name': name, 'version': version, 'type': sym_type})
    return symbols


SKIP = {
    'dlopen', 'dlsym', 'dlclose', 'dladdr', 'dlerror',
    '__cxa_finalize', '__cxa_atexit', '__register_atfork',
    '_Unwind_RaiseException', '_Unwind_DeleteException',
    '_Unwind_SetGR', '_Unwind_SetIP',
    '_Unwind_GetLanguageSpecificData', '_Unwind_GetIP',
    '_Unwind_GetRegionStart', '_Unwind_Resume',
    '__cxa_thread_atexit_impl',
    'android_set_abort_message', '__system_property_get',
    '__assert2', '__strncpy_chk2',
}


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <lib_to_analyze> <output.c>")
        sys.exit(1)

    lib_path = sys.argv[1]
    output_path = sys.argv[2]

    symbols = get_undef_symbols(lib_path)
    print(f"Found {len(symbols)} LIBC-versioned UNDEF symbols")

    to_gen = [s for s in symbols if s['name'] not in SKIP and s['type'] == 'FUNC']
    objs = [s for s in symbols if s['name'] not in SKIP and s['type'] == 'OBJECT']
    skipped = [s for s in symbols if s['name'] in SKIP]

    print(f"  Stubs generated: {len(to_gen)}")
    print(f"  Data (in bridge_libc.c): {len(objs)}")
    print(f"  Skipped: {len(skipped)}")

    lines = [
        '/* AUTO-GENERATED LIBC forwarding stubs */',
        f'/* {len(to_gen)} functions from {lib_path} */',
        '/* Each: extern decl → tail-call wrapper → version script tags @@LIBC */',
        '',
        '/* WARNING: All wrapper functions use void(void) signature.',
        ' * On ARM64, all args pass in registers x0-x7, and tail-calls',
        ' * via b instruction preserve them. The version script assigns',
        ' * these to the LIBC version tag automatically.',
        ' */',
        '',
    ]

    for sym in to_gen:
        name = sym['name']
        version = sym['version']
        lines += [
            f'extern void _bf_{name}_glibc(void);',
            f'void _bf_{name}_wrapper(void) __asm__("{name}");',
            f'void _bf_{name}_wrapper(void)',
            '{',
            f'    _bf_{name}_glibc();',
            '}',
            '',
        ]

    output = '\n'.join(lines)
    with open(output_path, 'w') as f:
        f.write(output)

    print(f"Wrote {output_path}")


if __name__ == '__main__':
    main()