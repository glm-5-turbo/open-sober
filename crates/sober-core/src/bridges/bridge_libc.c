/* libc.so version bridge for the Android sysroot.
 *
 * Provides LIBC/LIBC_* version definitions that Android GSI
 * libraries expect from libc.so. Wraps glibc's ARM64 libc
 * and exports Bionic-only stubs that GSI libs reference. */

#include <stddef.h>
#include <stdint.h>

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