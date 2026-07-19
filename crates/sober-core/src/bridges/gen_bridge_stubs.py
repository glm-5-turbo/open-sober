#!/usr/bin/env python3
"""Generate LIBC-versioned C stubs from multiple GSI libraries.

Usage:
    python3 gen_bridge_stubs.py <library_or_directory> <output.c>

If a directory is given, all .so files are scanned and the union
of all LIBC-versioned UNDEF symbols is generated.
"""

import subprocess
import sys
import os


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
        if not version.startswith('LIBC'):
            continue
        sym_type = 'FUNC' if 'FUNC' in line else ('OBJECT' if 'OBJECT' in line else 'OTHER')
        symbols.append({'name': name, 'version': version, 'type': sym_type, 'lib': os.path.basename(lib_path)})
    return symbols


SKIP = {
    # dlopen/dlsym/dlclose/dladdr/dlerror — now generated as forwarding stubs
    '_Unwind_RaiseException', '_Unwind_DeleteException',
    '_Unwind_SetGR', '_Unwind_SetIP',
    '_Unwind_GetLanguageSpecificData', '_Unwind_GetIP',
    '_Unwind_GetRegionStart', '_Unwind_Resume',
    '_Unwind_GetDataRelBase', '_Unwind_GetTextRelBase',
    '_Unwind_GetIPInfo',
    '__cxa_thread_atexit_impl',
    'android_set_abort_message', '__system_property_get',
    '__system_property_find', '__system_property_read',
    '__system_property_serial', '__system_property_set',
    '__system_property_area_serial',
    '__system_property_read_callback', '__system_property_wait',
    'android_fdsan_close_with_tag', 'android_fdsan_create_owner_tag',
    'android_fdsan_exchange_owner_tag',
    'android_getaddrinfofornet',
    'android_get_application_target_sdk_version',  # Bionic-specific, defined in bridge_libc.c
    'android_get_device_api_level',
    'android_dlopen_ext',
    'AConnectivityNative_getNetworkBlockedReason',
    'getprogname',
    'nrand48', 'futimens',
    '__assert2', '__strncpy_chk2', '__assert',
    'memset_explicit',  # Defined in bridge_libc.c
    'getentropy',       # Defined in bridge_libc.c
    '__cfi_slowpath',   # Defined in bridge_libc.c (special @LIBC_OMR1)
    'getrandom', 'memfd_create', 'sem_clockwait', 'pthread_cond_clockwait',
    'eventfd_read', 'eventfd_write',
    # Bionic-only system property functions (not in glibc, defined in bridge_libc.c)
    '__system_property_get', '__system_property_find',
    '__system_property_read', '__system_property_serial',
    '__system_property_area_serial', '__system_property_set',
    '__system_properties_init', '__system_properties_zygote_reload',
    '__system_property_foreach',
    # Bionic-only fdsan functions (file descriptor sanitizer)
    'android_fdsan_close_with_tag', 'android_fdsan_create_owner_tag',
    'android_fdsan_exchange_owner_tag', 'android_fdsan_get_owner_tag',
    # Bionic-only malloc debug functions (defined in bridge_libc.c)
    'malloc_backtrace', 'malloc_disable', 'malloc_enable', 'malloc_iterate',
    # LIBC_R version alias for glibc function (defined in bridge_libc.c)
    '__mempcpy_chk',
    # These are handled by bridge_libc.c with explicit .symver aliases
    '__write_chk', 'aligned_alloc', '__fread_chk', '__sendto_chk',
    'strchrnul', 'pthread_setschedprio', 'android_mallopt',
}


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <library_or_directory> <output.c>")
        sys.exit(1)

    path = sys.argv[1]
    output_path = sys.argv[2]

    # Collect all .so files (single file or directory)
    if os.path.isdir(path):
        files = [os.path.join(path, f) for f in os.listdir(path)
                 if f.endswith('.so') and os.path.isfile(os.path.join(path, f))]
    else:
        files = [path]

    print(f"Scanning {len(files)} libraries...")

    # Merge all UNDEF symbols by name+version
    merged = {}
    for lib_path in files:
        syms = get_undef_symbols(lib_path)
        for s in syms:
            key = f"{s['name']}@@{s['version']}"
            if key not in merged:
                merged[key] = s

    symbols = list(merged.values())
    print(f"Found {len(symbols)} unique LIBC-versioned UNDEF symbols")

    to_gen = [s for s in symbols if s['name'] not in SKIP and s['type'] == 'FUNC']
    objs = [s for s in symbols if s['name'] not in SKIP and s['type'] == 'OBJECT']
    skipped = [s for s in symbols if s['name'] in SKIP]

    print(f"  Stubs generated: {len(to_gen)}")
    print(f"  Data (in bridge_libc.c): {len(objs)}")
    print(f"  Skipped: {len(skipped)}")

    # Collect all version tags per symbol for generating .symver aliases
    version_map = {}  # name -> set of version tags
    for sym in to_gen:
        name = sym['name']
        if name not in version_map:
            version_map[name] = set()
        version_map[name].add(sym['version'])

    lines = [
        '/* AUTO-GENERATED LIBC forwarding stubs */',
        f'/* {len(to_gen)} functions from {len(files)} libraries */',
        '/* Each: glibc tail-call wrapper + .symver aliases for LIBC_* version tags */',
        '',
    ]

    # Generate wrappers and version aliases
    seen = set()
    for sym in sorted(to_gen, key=lambda s: (s['name'], s['version'])):
        name = sym['name']
        if name in seen:
            continue
        seen.add(name)
        versions = sorted(version_map.get(name, set()))
        has_libc = 'LIBC' in versions
        default_ver = 'LIBC' if has_libc else versions[0]

        # Generate the wrapper function, renamed to {name}
        # If there's ONLY a non-LIBC version (e.g. LIBC_Q), we still export it
        # as @@LIBC (the version script's default), and also create a .symver
        # alias for the specific version (e.g. reallocarray@LIBC_Q).
        # Note: use single @ for non-default, @@ for default/only version.
        lines += [
            f'void _bf_{name}_wrapper(void) __asm__("{name}");',
            f'void _bf_{name}_wrapper(void)',
            '{',
            f'    extern void _bf_glibc_{name}(void) __asm__("{name}");',
            f'    _bf_glibc_{name}();',
            '}',
        ]

        # Generate .symver aliases for non-LIBC versions.
        # The version script tags everything as @@LIBC by default.
        # We need additional aliases so GSI libs can look up "name@LIBC_Q" etc.
        for ver in versions:
            if ver != 'LIBC':
                ver_lower = ver.replace('LIBC_', '').lower()
                lines += [
                    f'void _bf_{name}_{ver_lower}_alias(void);',
                    f'__asm__(".symver _bf_{name}_{ver_lower}_alias, {name}@@{ver}");',
                    f'void _bf_{name}_{ver_lower}_alias(void)',
                    '{',
                    f'    extern void _bf_glibc_{name}(void) __asm__("{name}");',
                    f'    _bf_glibc_{name}();',
                    '}',
                ]
        lines.append('')

    # Add data objects — NOTE: these use __asm__(".symver") which can cause
    # linker conflicts. Data objects are already handled in bridge_libc.c
    # with the bf_ prefix pattern.
    seen_data = set()
    for sym in objs:
        name = sym['name']
        if name in seen_data:
            continue
        seen_data.add(name)
        version = sym['version']
        lines += [
            f'/* DATA {name}@@{version} — handled in bridge_libc.c */',
            '',
        ]

    output = '\n'.join(lines)
    with open(output_path, 'w') as f:
        f.write(output)

    print(f"Wrote {len(seen)} function stubs + {len(seen_data)} data -> {output_path}")


if __name__ == '__main__':
    main()