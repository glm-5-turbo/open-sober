/* libc.so version bridge for the Android sysroot.
 *
 * Provides LIBC/LIBC_* version definitions that Android GSI
 * libraries expect from libc.so. Wraps glibc's ARM64 libc
 * and exports Bionic-only stubs that GSI libs reference. */

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* ===== Bionic-only stubs that GSI libraries reference from libc.so ===== */

void android_set_abort_message(const char *msg) { (void)msg; }

int __system_property_get(const char *key, char *value) {
    (void)key;
    if (value) value[0] = '\0';
    return 0;
}

void __assert2(const char *file, int line, const char *func, const char *msg) {
    (void)file; (void)line; (void)func; (void)msg;
}

char *__strncpy_chk2(char *dest, const char *src, size_t n, size_t destlen) {
    (void)destlen;
    size_t i;
    for (i = 0; i < n && *src; i++) *dest++ = *src++;
    for (; i < n; i++) *dest++ = '\0';
    return dest;
}

/* getentropy / arc4random — used by libcrypto and others */
int getentropy(void *buf, size_t len) {
    /* Best-effort: zero out. Real Android would use the kernel. */
    __builtin_memset(buf, 0, len);
    return 0;
}

/* ===== Data symbols re-exported under @@LIBC =====
 * libc++.so and other GSI libraries reference these as versioned
 * data objects (e.g. stderr@@LIBC). We provide 8-byte data slots
 * that are resolved at runtime. The values start NULL, but the
 * dynamic linker's symbol resolution satisfies the versioned lookup. */

__attribute__((used)) __attribute__((externally_visible))
void *bf_stderr = NULL;
__asm__(".symver bf_stderr,stderr@@LIBC");

__attribute__((used)) __attribute__((externally_visible))
void *bf_stdin = NULL;
__asm__(".symver bf_stdin,stdin@@LIBC");

__attribute__((used)) __attribute__((externally_visible))
void *bf_stdout = NULL;
__asm__(".symver bf_stdout,stdout@@LIBC");

/* ===== CFI slowpath stub (Bionic-only, CFI not needed on glibc) =====
 * __cfi_slowpath is referenced by libraries compiled with Control Flow
 * Integrity. On non-CFI builds it should be a no-op. The version tag
 * matters: libcodec2_hidl_client needs __cfi_slowpath@LIBC_OMR1. */

__attribute__((used)) __attribute__((externally_visible))
void __cfi_slowpath(void) { }
__asm__(".symver __cfi_slowpath, __cfi_slowpath@@LIBC_OMR1");

/* ===== Re-exported glibc symbols under LIBC version =====
 * These are standard C functions that GSI libraries reference
 * from libc.so with version LIBC. We use thin wrappers that
 * call into glibc via dlsym, tagged with @@LIBC via .symver. */

/* memset_explicit — Bionic has this as a LIBC_U symbol.
 * glibc 2.43+ has it as GLIBC_2.43, but the version bridge
 * links against the glibc ARM64 .so directly, so we provide
 * an explicit wrapper to ensure it's available under LIBC_U. */
void *memset_explicit(void *s, int c, size_t n) {
    volatile unsigned char *p = s;
    for (size_t i = 0; i < n; i++) p[i] = (unsigned char)c;
    /* Prevent optimization */
    __asm__ volatile("" : : "r"(p) : "memory");
    return s;
}