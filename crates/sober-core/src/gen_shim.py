#!/usr/bin/env python3
"""
Generate bionic_shim.S — complete LIBC-versioned symbol shim for libroblox.so.

Produces a combined .S output that provides every @LIBC, @LIBC_N, @LIBC_O
symbol from Roblox's native library. Each trampoline dispatches through a
function-pointer table filled at init time via dlsym(RTLD_NEXT, ...).

This avoids PLT circularity: the trampolines reference the table, not symbol
names that could resolve to our own versioned exports.

Usage:
    gen_shim.py <libroblox.so> <output.S>
"""

import subprocess, sys, os


# Bionic-specific symbols with no glibc equivalent — have stubs in C code
BIONIC_ONLY = {
    '__system_property_get',
    'android_set_abort_message',
    'arc4random_buf',
}

# Rename map: Bionic name -> glibc name
RENAME_MAP = {
    '__errno': '__errno_location',
}

# _chk symbols that alias to base functions (glibc provides them as inlines)
CHK_TO_BASE = {
    '__strlen_chk': 'strlen',
    '__strchr_chk': 'strchr',
    '__FD_SET_chk': 'FD_SET',
    '__FD_ISSET_chk': 'FD_ISSET',
    '__FD_CLR_chk': 'FD_CLR',
}

# Special stubs with C implementations
SPECIAL_STUBS = {'__assert2', '__strncpy_chk2'}

# Data objects that need different glibc symbol names
DATA_ALIAS = {
    '__sF': '_IO_2_1_stderr_',
}


def get_needed_symbols(lib_path):
    """Extract (symbol_name, version_tag, is_data) from ARM64 ELF."""
    result = subprocess.run(
        ["aarch64-linux-gnu-objdump", "-T", lib_path],
        capture_output=True, text=True, check=True
    )
    symbols = []
    for line in result.stdout.splitlines():
        ver_m = None
        for v in ['LIBC_N', 'LIBC_O', 'LIBC']:
            if f'({v})' in line:
                ver_m = v
                break
        if not ver_m:
            continue
        cols = line.split()
        sym = cols[-1]
        is_data = ' DO ' in line
        symbols.append((sym, ver_m, is_data))
    return symbols


def generate_shim(symbols, output_path):
    """Generate a single .S file with all trampolines + dispatch table + C stubs.

    Layout:
      1. .data section: function pointer table (__bf_tramp_table), indexed by symbol
      2. .text section: trampolines for each function
         Each trampoline: adrp + add (table base) + ldr (slot) + br
      3. .text section: bionic-only C stubs (via .cfile or inline asm)
      4. .data section: data object stubs
      5. .text section: init constructor that fills the table via dlsym(RTLD_NEXT)
    """
    sym_list = list(symbols)  # preserve order for indexing
    func_syms = [(s, v) for s, v, d in sym_list if not d]
    data_syms = [(s, v) for s, v, d in sym_list if d]

    # Build index map
    all_stubs = sorted(BIONIC_ONLY | SPECIAL_STUBS)
    tramp_syms = [(s, v) for s, v in func_syms if s not in all_stubs]
    func_index = {}
    for i, (s, _) in enumerate(tramp_syms):
        func_index[s] = i

    lines = []
    lines.append('/* Auto-generated Bionic symbol shim. DO NOT EDIT.')
    lines.append(f' * {len(tramp_syms)} trampolines, {len(data_syms)} data objs, {len(all_stubs)} C stubs.')
    lines.append(' */')
    lines.append('')

    # ===== Dispatch table =====
    lines.append('.section .data.bionic_tramp, "aw"')
    lines.append('.balign 8')
    lines.append('.globl __bf_tramp_table')
    lines.append('.type __bf_tramp_table, %object')
    lines.append(f'.size __bf_tramp_table, {len(tramp_syms) * 8}')
    lines.append('__bf_tramp_table:')
    lines.append(f'.zero {len(tramp_syms) * 8}')
    lines.append('')

    # ===== Function trampolines =====
    lines.append('.section .text.bionic_tramp, "ax"')
    lines.append('')
    for i, (sym, ver) in enumerate(tramp_syms):
        rename_to = RENAME_MAP.get(sym) or CHK_TO_BASE.get(sym)
        target_sym = rename_to or sym
        internal = f'__bf_{sym}'
        lines.append(f'.balign 4')
        lines.append(f'.globl {internal}')
        lines.append(f'.type {internal}, %function')
        lines.append(f'{internal}:')
        lines.append(f'    adrp x16, :got:__bf_tramp_table')
        lines.append(f'    ldr x16, [x16, #:got_lo12:__bf_tramp_table]')
        lines.append(f'    ldr x16, [x16, #{(i * 8)}]')
        lines.append(f'    br x16')
        lines.append(f'.symver {internal}, {sym}@@{ver}')
        lines.append('')

    # ===== Data object stubs =====
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

    # ===== Glue: init constructor in C via inline asm =====
    # We embed the init function as a C string compiled with the .S
    # But actually we'll just generate a C file separately.
    # The .S file is the trampoline-only part.

    output = '\n'.join(lines)
    with open(output_path, 'w') as f:
        f.write(output)
    print(f"Generated ASM: {output_path}")
    print(f"  {len(tramp_syms)} trampolines via dispatch table")
    print(f"  {len(data_syms)} data stubs")
    return tramp_syms, data_syms


def generate_c_init(tramp_syms, data_syms, output_path):
    """Generate C init file: constructor fills dispatch table + data objects."""
    all_stubs = set(BIONIC_ONLY | SPECIAL_STUBS)
    # Filter out stubs
    real_tramp = [(s, v) for s, v in tramp_syms if s not in all_stubs]

    lines = []
    lines.append('/* Auto-generated Bionic shim C init. DO NOT EDIT.')
    lines.append(f' * {len(real_tramp)} function pointers, {len(data_syms)} data pointers.')
    lines.append(' */')
    lines.append('')
    lines.append('#define _GNU_SOURCE')
    lines.append('#include <stddef.h>')
    lines.append('#include <stdint.h>')
    lines.append('#include <dlfcn.h>')
    lines.append('')

    # Extern the dispatch table and data objects
    lines.append('/* Defined in bionic_shim.S */')
    lines.append(f'extern void *__bf_tramp_table[{len(real_tramp)}];')
    lines.append('')
    for d_sym, _ in data_syms:
        lines.append(f'extern void *__bf_data_{d_sym};')
    lines.append('')

    # Bionic-only C stubs
    lines.append('/* ===== Bionic-only function stubs (no glibc equivalent) ===== */')
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

    lines.append('/* arc4random_buf — via syscalls to avoid libc dependency */')
    lines.append('__asm__(".symver arc4random_buf,arc4random_buf@@LIBC");')
    lines.append('void arc4random_buf(void *buf, size_t n) {')
    lines.append('    long fd;')
    lines.append('    register long x0 asm("x0") = -100;')
    lines.append('    register const char *x1 asm("x1") = "/dev/urandom";')
    lines.append('    register long x2 asm("x2") = 0;')
    lines.append('    register long x8 asm("x8") = 56;')
    lines.append('    asm volatile("svc #0" : "=r"(x0) : "r"(x0), "r"(x1), "r"(x2), "r"(x8));')
    lines.append('    fd = x0;')
    lines.append('    if (fd >= 0) {')
    lines.append('        register long r0 asm("x0") = fd;')
    lines.append('        register void *r1 asm("x1") = buf;')
    lines.append('        register size_t r2 asm("x2") = n;')
    lines.append('        register long r8 asm("x8") = 63;')
    lines.append('        asm volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r8));')
    lines.append('        register long c0 asm("x0") = fd;')
    lines.append('        register long r8c asm("x8") = 57;')
    lines.append('        asm volatile("svc #0" : : "r"(c0), "r"(r8c));')
    lines.append('    }')
    lines.append('}')
    lines.append('')

    lines.append('/* __assert2 */')
    lines.append('__asm__(".symver __assert2,__assert2@@LIBC");')
    lines.append('void __assert2(const char *file, int line, const char *func, const char *failed) {')
    lines.append('    (void)file; (void)line; (void)func; (void)failed;')
    lines.append('    register long r0 asm("x0") = 2;')
    lines.append('    const char *msg = "Assertion failed\\n";')
    lines.append('    register const char *r1 asm("x1") = msg;')
    lines.append('    register long r2 asm("x2") = 16;')
    lines.append('    register long r8 asm("x8") = 64;')
    lines.append('    asm volatile("svc #0" : : "r"(r0), "r"(r1), "r"(r2), "r"(r8));')
    lines.append('    asm volatile("udf #0");')
    lines.append('}')
    lines.append('')

    lines.append('/* __strncpy_chk2 — Bionic fortified strncpy */')
    lines.append('__asm__(".symver __strncpy_chk2,__strncpy_chk2@@LIBC");')
    lines.append('char *__strncpy_chk2(char *dst, const char *src, size_t n, size_t dest_len) {')
    lines.append('    (void)dest_len;')
    lines.append('    char *d = dst; const char *s = src; size_t i;')
    lines.append('    for (i = 0; i < n && *s; i++) *d++ = *s++;')
    lines.append('    for (; i < n; i++) *d++ = \'\\0\';')
    lines.append('    return dst;')
    lines.append('}')
    lines.append('')

    # Constructor
    lines.append('/* ===== Constructor: fill dispatch table and init data ===== */')
    lines.append('__attribute__((constructor))')
    lines.append('static void init_bionic_shim(void) {')
    lines.append('    void *self = dlopen(NULL, RTLD_LAZY);')
    lines.append('    if (!self) return;')
    lines.append('')

    for i, (sym, _) in enumerate(real_tramp):
        target = RENAME_MAP.get(sym, sym)
        # Try to get glibc's version via RTLD_NEXT. If RTLD_NEXT fails,
        # fall back to dlsym(RTLD_DEFAULT, ...)
        lines.append(f'    __bf_tramp_table[{i}] = dlsym(RTLD_NEXT, "{target}");')
        lines.append(f'    if (!__bf_tramp_table[{i}])')
        lines.append(f'        __bf_tramp_table[{i}] = dlsym(self, "{target}");')

    lines.append('')
    lines.append('    dlclose(self);')
    lines.append('')

    # Data objects
    for d_sym, _ in data_syms:
        glibc_sym = DATA_ALIAS.get(d_sym, d_sym)
        lines.append(f'    *(void **)(__bf_data_{d_sym}) = dlsym(RTLD_NEXT, "{glibc_sym}");')

    lines.append('}')

    output = '\n'.join(lines)
    with open(output_path, 'w') as f:
        f.write(output)
    print(f"Generated C init: {output_path}")


def main():
    if len(sys.argv) < 3:
        print("Usage: gen_shim.py <libroblox.so> <output.S> <output.c>")
        sys.exit(1)

    lib_path = sys.argv[1]
    asm_output = sys.argv[2]
    c_output = sys.argv[3]

    if not os.path.exists(lib_path):
        print(f"Error: {lib_path} not found")
        sys.exit(1)

    symbols = get_needed_symbols(lib_path)
    if not symbols:
        print("Error: no LIBC-versioned symbols found")
        sys.exit(1)

    print(f"Found {len(symbols)} LIBC-versioned symbols in {lib_path}")
    ver_counts = {}
    for _, v, _ in symbols:
        ver_counts[v] = ver_counts.get(v, 0) + 1
    print(f"  Versions: {ver_counts}")

    tramp_syms, data_syms = generate_shim(symbols, asm_output)
    generate_c_init(tramp_syms, data_syms, c_output)


if __name__ == '__main__':
    main()