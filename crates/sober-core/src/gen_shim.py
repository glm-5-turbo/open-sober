#!/usr/bin/env python3
"""
Generate bionic_shim.S — complete LIBC-versioned symbol shim for libroblox.so.

Produces:
  bionic_shim.S  — assembly trampolines with lazy dispatch table resolution
  bionic_init.c  — lazy fill stubs for the first call to each symbol

Each trampoline checks if the dispatch table entry is NULL and lazily resolves
it via a helper function. This avoids PLT circularity: dlsym calls in the helper
go through glibc (which is loaded before us) and don't trigger our symbols.

Usage:
    gen_shim.py <libroblox.so> <output.S> <output.c>
"""

import subprocess, sys, os


BIONIC_ONLY = {'__system_property_get', 'android_set_abort_message', 'arc4random_buf'}
RENAME_MAP = {'__errno': '__errno_location'}
CHK_TO_BASE = {'__strlen_chk': 'strlen', '__strchr_chk': 'strchr',
               '__FD_SET_chk': 'FD_SET', '__FD_ISSET_chk': 'FD_ISSET',
               '__FD_CLR_chk': 'FD_CLR'}
SPECIAL_STUBS = {'__assert2', '__strncpy_chk2', '__bf_c_resolve', '__bf_init_data'}

# Unwind stubs referenced from libc++.so under LIBC_R version.
# These DWARF unwind functions live in libgcc_s.so.1 (GCC_3.0/3.3 on glibc).
# The bionic shim provides them so libc++.so's symbol lookup succeeds.
UNWIND_STUBS = [('_Unwind_RaiseException', 'LIBC_R', 'int'),
                ('_Unwind_DeleteException', 'LIBC_R', 'void'),
                ('_Unwind_SetGR', 'LIBC_R', 'void'),
                ('_Unwind_SetIP', 'LIBC_R', 'void'),
                ('_Unwind_GetLanguageSpecificData', 'LIBC_R', 'void*'),
                ('_Unwind_GetIP', 'LIBC_R', 'unsigned long'),
                ('_Unwind_GetRegionStart', 'LIBC_R', 'unsigned long'),
                ('_Unwind_GetTextRelBase', 'LIBC_R', 'unsigned long'),
                ('_Unwind_Backtrace', 'LIBC_R', 'int'),
                ('_Unwind_FindEnclosingFunction', 'LIBC_R', 'void*'),
                ('_Unwind_Find_FDE', 'LIBC_R', 'void*'),
                ('_Unwind_GetCFA', 'LIBC_R', 'unsigned long'),
                ('_Unwind_GetIPInfo', 'LIBC_R', 'unsigned long'),
                ('_Unwind_GetDataRelBase', 'LIBC_R', 'unsigned long'),
                ('_Unwind_Resume', 'LIBC_R', 'void'),
                ]

# Symbols provided by bridge_libc.c, NOT by the bionic shim.
# These are dl* functions which Bionic puts in libc but glibc puts in libdl.
# If the bionic shim intercepts these, its own __bf_c_resolve() (which calls
# dlsym to resolve symbols) will infinite-recurse through the shim's PLT.
BRIDGE_LIBC_SYMS = {'dlopen', 'dlsym', 'dlclose', 'dlerror', 'dladdr'}
DATA_ALIAS = {'__sF': '_IO_2_1_stderr_'}


def get_needed_symbols(lib_path, extra_lib_dirs=None):
    """Get LIBC-versioned UNDEF symbols from a library and optionally from extra libs."""
    result = subprocess.run(
        ["aarch64-linux-gnu-objdump", "-T", lib_path],
        capture_output=True, text=True, check=True)
    symbols = []
    found_vers = set()
    for line in result.stdout.splitlines():
        ver_m = None
        for v in ['LIBC', 'LIBC_N', 'LIBC_O', 'LIBC_P', 'LIBC_Q', 'LIBC_R',
                       'LIBC_S', 'LIBC_T', 'LIBC_U', 'LIBC_V', 'LIBC_OMR1',
                       'LIBC_36', 'LIBC_37', 'LIBDL_ANDROID']:
            if f'({v})' in line:
                ver_m = v
                break
        if not ver_m:
            continue
        cols = line.split()
        sym = cols[-1]
        is_data = ' DO ' in line
        symbols.append((sym, ver_m, is_data))

    # Also scan extra libraries for LIBC-versioned UNDEF symbols they import.
    # These are transitive dependencies (libc++.so, libbase.so, etc.)
    # that need LIBC-versioned symbols resolved through the bionic shim.
    if extra_lib_dirs:
        seen_syms = {s for s, v, d in symbols}
        for libdir in extra_lib_dirs:
            if not os.path.isdir(libdir):
                continue
            for fname in sorted(os.listdir(libdir)):
                if not fname.endswith('.so') or fname.startswith('gsi_'):
                    continue
                libpath = os.path.join(libdir, fname)
                if not os.path.isfile(libpath):
                    continue
                try:
                    r2 = subprocess.run(
                        ["aarch64-linux-gnu-objdump", "-T", libpath],
                        capture_output=True, text=True, timeout=30)
                except:
                    continue
                for line in r2.stdout.splitlines():
                    ver_m = None
                    for v in ['LIBC', 'LIBC_N', 'LIBC_O', 'LIBC_P', 'LIBC_Q', 'LIBC_R',
                                   'LIBC_S', 'LIBC_T', 'LIBC_U', 'LIBC_V', 'LIBC_OMR1',
                                   'LIBC_36', 'LIBC_37', 'LIBDL_ANDROID']:
                        if f'({v})' in line:
                            ver_m = v
                            break
                    if not ver_m:
                        continue
                    if '*UND*' not in line:
                        continue
                    cols = line.split()
                    sym = cols[-1]
                    if sym not in seen_syms:
                        is_data = ' DO ' in line
                        symbols.append((sym, ver_m, is_data))
                        seen_syms.add(sym)
    return symbols


def generate_shim(symbols, output_path):
    """Generate .S file with lazy-resolving trampolines."""
    sym_list = list(symbols)
    all_stubs = sorted(BIONIC_ONLY | SPECIAL_STUBS | BRIDGE_LIBC_SYMS)
    func_syms = [(s, v) for s, v, d in sym_list if not d and s not in all_stubs]
    data_syms = [(s, v) for s, v, d in sym_list if d]

    lines = []
    lines.append('/* Auto-generated Bionic symbol shim with lazy dispatch. DO NOT EDIT.')
    lines.append(f' * {len(func_syms)} trampolines, {len(data_syms)} data objs.')
    lines.append(' */')
    lines.append('')

    # Section: data for function pointers (zero-initialized)
    lines.append('.section .data.bionic_tramp, "aw"')
    lines.append('.balign 8')
    lines.append('.globl __bf_tramp_table')
    lines.append('.type __bf_tramp_table, %object')
    lines.append(f'.size __bf_tramp_table, {len(func_syms) * 8}')
    lines.append('__bf_tramp_table:')
    lines.append(f'.zero {len(func_syms) * 8}')
    lines.append('')

    # Section: resolver helper (text)
    lines.append('.section .text.bionic_resolver, "ax"')
    lines.append('.balign 4')
    lines.append('.globl __bf_resolve_and_call')
    lines.append('.type __bf_resolve_and_call, %function')
    lines.append('__bf_resolve_and_call:')
    lines.append('    /* x0 = trampoline index, x1 = address of glibc func */')
    lines.append('    /* This is called from each lazy trampoline for first-use resolution */')
    lines.append('    /* We use a simple C function pointer table populated via dlsym */')
    lines.append('    /* Arguments: x0 = entry index */')
    lines.append('    stp x29, x30, [sp, #-16]!')
    lines.append('    mov x29, sp')
    lines.append('    /* We call __bf_c_resolve(index) which does dlsym and fills table */')
    lines.append('    bl __bf_c_resolve')
    lines.append('    /* Restore and jump to the resolved function */')
    lines.append('    ldp x29, x30, [sp], #16')
    lines.append('    /* x0 now holds the function pointer, jump to it */')
    lines.append('    br x0')
    lines.append('')

    # Section: trampolines
    lines.append('.section .text.bionic_tramp, "ax"')
    lines.append('')
    for i, (sym, ver) in enumerate(func_syms):
        target = RENAME_MAP.get(sym, CHK_TO_BASE.get(sym, sym))
        internal = f'__bf_{sym}'
        lines.append(f'.balign 4')
        lines.append(f'.globl {internal}')
        lines.append(f'.type {internal}, %function')
        lines.append(f'{internal}:')
        # Lazy dispatch: load entry, check NULL, if NULL call resolver
        lines.append(f'    adrp x16, :got:__bf_tramp_table')
        lines.append(f'    ldr x16, [x16, #:got_lo12:__bf_tramp_table]')
        lines.append(f'    ldr x17, [x16, #{i*8}]')
        lines.append(f'    cbz x17, 1f')
        lines.append(f'    br x17')
        lines.append(f'1:  mov x0, #{i}')
        lines.append(f'    bl __bf_resolve_and_call')
        lines.append(f'    br x0')
        # Above will crash if resolver fails (x0 = 0) — intentional
        lines.append(f'.symver {internal}, {sym}@@{ver}')
        lines.append('')

    # Data objects
    lines.append('.section .data.bionic_data, "aw"')
    lines.append('')
    for sym, ver in data_syms:
        internal = f'__bf_data_{sym}'
        lines.append(f'.balign 8')
        lines.append(f'.globl {internal}')
        lines.append(f'.type {internal}, %object')
        lines.append(f'.size {internal}, 8')
        lines.append(f'{internal}:')
        lines.append(f'    .dword 0')
        lines.append(f'.symver {internal}, {sym}@@{ver}')
        lines.append('')

    output = '\n'.join(lines)
    with open(output_path, 'w') as f:
        f.write(output)
    print(f"Generated ASM: {output_path} ({len(func_syms)} lazy trampolines)")
    return func_syms, data_syms


def generate_c_init(func_syms, data_syms, output_path):
    """Generate C file: lazy resolver + stub_init + bionic stubs."""
    real_tramp = [(s, v) for s, v in func_syms if s not in (BIONIC_ONLY | SPECIAL_STUBS | BRIDGE_LIBC_SYMS)]
    all_stubs = sorted(BIONIC_ONLY | SPECIAL_STUBS)

    lines = []
    lines.append('/* Auto-generated bionic shim C helpers. DO NOT EDIT.')
    lines.append(f' * {len(real_tramp)} lazy-resolve entries, {len(data_syms)} data ptrs.')
    lines.append(' */')
    lines.append('#define _GNU_SOURCE')
    lines.append('#include <stddef.h>')
    lines.append('#include <dlfcn.h>')
    lines.append('')

    lines.append('/* Dispatch table and data — defined in bionic_shim.S */')
    lines.append(f'extern void *__bf_tramp_table[{len(real_tramp)}];')
    lines.append('__attribute__((weak)) void *__bf_c_resolve(int index);')
    for d_sym, _ in data_syms:
        lines.append(f'extern void *__bf_data_{d_sym};')
    lines.append('')

    # Bionic-only stubs
    lines.append('/* ===== Bionic-only stubs ===== */')
    lines.append('')
    lines.append('/* __system_property_get */')
    lines.append('__asm__(".symver __system_property_get,__system_property_get@@LIBC");')
    lines.append('int __system_property_get(const char *key, char *value) {')
    lines.append('    (void)key; if (value) value[0] = \'\\0\'; return 0;')
    lines.append('}')
    lines.append('')

    lines.append('/* android_set_abort_message */')
    lines.append('__asm__(".symver android_set_abort_message,android_set_abort_message@@LIBC");')
    lines.append('void android_set_abort_message(const char *msg) { (void)msg; }')
    lines.append('')

    lines.append('/* arc4random_buf */')
    lines.append('__asm__(".symver arc4random_buf,arc4random_buf@@LIBC");')
    lines.append('void arc4random_buf(void *buf, size_t n) {')
    lines.append('    long fd;')
    lines.append('    register long x0 asm("x0") = -100;')
    lines.append('    register const char *x1 asm("x1") = "/dev/urandom";')
    lines.append('    register long x2 asm("x2") = 0; register long x8 asm("x8") = 56;')
    lines.append('    asm volatile("svc #0" : "=r"(x0) : "r"(x0), "r"(x1), "r"(x2), "r"(x8));')
    lines.append('    fd = x0;')
    lines.append('    if (fd >= 0) {')
    lines.append('        register long r0 asm("x0") = fd;')
    lines.append('        register void *r1 asm("x1") = buf;')
    lines.append('        register size_t r2 asm("x2") = n;')
    lines.append('        register long r8_rd asm("x8") = 63;')
    lines.append('        asm volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r8_rd));')
    lines.append('        register long c0 asm("x0") = fd;')
    lines.append('        register long r8_cl asm("x8") = 57;')
    lines.append('        asm volatile("svc #0" : : "r"(c0), "r"(r8_cl));')
    lines.append('    }')
    lines.append('}')
    lines.append('')

    lines.append('/* __assert2 */')
    lines.append('__asm__(".symver __assert2,__assert2@@LIBC");')
    lines.append('void __assert2(const char *f, int l, const char *fn, const char *msg) {')
    lines.append('    (void)f; (void)l; (void)fn; (void)msg;')
    lines.append('    register long r0 asm("x0") = 2;')
    lines.append('    register const char *r1 asm("x1") = "Assertion failed\\n";')
    lines.append('    register long r2 asm("x2") = 16;')
    lines.append('    register long r8 asm("x8") = 64;')
    lines.append('    asm volatile("svc #0" : : "r"(r0), "r"(r1), "r"(r2), "r"(r8));')
    lines.append('    asm volatile("udf #0");')
    lines.append('}')
    lines.append('')

    lines.append('/* __strncpy_chk2 */')
    lines.append('__asm__(".symver __strncpy_chk2,__strncpy_chk2@@LIBC");')
    lines.append('char *__strncpy_chk2(char *d, const char *s, size_t n, size_t dl) {')
    lines.append('    (void)dl; size_t i; char *p = d;')
    lines.append('    for (i = 0; i < n && *s; i++) *p++ = *s++;')
    lines.append('    for (; i < n; i++) *p++ = \'\\0\';')
    lines.append('    return d;')
    lines.append('}')
    lines.append('')

    # _Unwind_* DWARF unwinding stubs — referenced from libc++.so under LIBC_R
    lines.append('')
    # Lazy resolver — called from assembly when a trampoline entry is NULL
    lines.append('/* ===== Lazy dispatch-table resolver ===== */')
    lines.append('void* __bf_c_resolve(int index) {')
    lines.append('    if (index < 0 || index >= ' + str(len(real_tramp)) + ')')
    lines.append('        return NULL;')

    # Map index -> symbol name and target
    rename_target = {}
    for sym, _ in real_tramp:
        t = RENAME_MAP.get(sym, CHK_TO_BASE.get(sym, sym))
        rename_target[sym] = t

    for i, (sym, _) in enumerate(real_tramp):
        target = RENAME_MAP.get(sym, CHK_TO_BASE.get(sym, sym))
        lines.append(f'    if (index == {i}) {{ __bf_tramp_table[{i}] = (void*)(unsigned long long)__bf_tramp_table[{i}]; }}')
        lines.append(f'    if (index == {i}) __bf_tramp_table[{i}] = dlsym(RTLD_NEXT, "{target}");')
        lines.append(f'    if (index == {i}) return __bf_tramp_table[{i}];')

    lines.append('    return NULL;')
    lines.append('}')
    lines.append('')

    lines.append('')
    # Data object init — called from JNI shim
    lines.append('/* ===== Init function (called from JNI shim) ===== */')
    lines.append('__attribute__((visibility("default")))')
    lines.append('void __bf_init_data(void) {')
    lines.append('    void *self = dlopen(NULL, RTLD_LAZY);')
    lines.append('    if (!self) return;')
    for d_sym, _ in data_syms:
        glibc_sym = DATA_ALIAS.get(d_sym, d_sym)
        lines.append(f'    if (!__bf_data_{d_sym})')
        lines.append(f'        *(void **)(__bf_data_{d_sym}) = dlsym(self, "{glibc_sym}");')
    lines.append('    dlclose(self);')
    lines.append('}')

    with open(output_path, 'w') as f:
        f.write('\n'.join(lines))
    print(f"Generated C init: {output_path}")


def main():
    if len(sys.argv) < 4:
        print("Usage: gen_shim.py <libroblox.so> <output.S> <output.c> [extra_lib_dir...]")
        sys.exit(1)

    extra_dirs = sys.argv[4:] if len(sys.argv) > 4 else None
    symbols = get_needed_symbols(sys.argv[1], extra_dirs)
    if not symbols:
        print("Error: no LIBC-versioned symbols found")
        sys.exit(1)

    print(f"Found {len(symbols)} LIBC-versioned symbols")
    func_syms, data_syms = generate_shim(symbols, sys.argv[2])
    generate_c_init(func_syms, data_syms, sys.argv[3])


if __name__ == '__main__':
    main()