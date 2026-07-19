/* libc.so version bridge for the Android sysroot.
 *
 * Provides LIBC/LIBC_* version definitions that Android GSI
 * libraries expect from libc.so. Wraps glibc's ARM64 libc
 * and exports Bionic-only stubs that GSI libs reference. */

#include <stddef.h>
#include <stdint.h>
#include <time.h>
#include <stdio.h>

/* ===== Bionic-only stubs that GSI libraries reference from libc.so ===== */

int __system_property_get(const char *key, char *value) {
    (void)key;
    if (value) value[0] = '\0';
    return 0;
}

/* __system_property_set — set a system property (stub, no-op) */
int __system_property_set(const char *key, const char *value) {
    (void)key; (void)value;
    return 0;
}

/* __system_property_area_serial — returns global serial counter for system properties.
 * Bionic uses this for property change notifications. Real value doesn't matter for us. */
unsigned int __system_property_area_serial(void) { return 0; }

/* __system_property_find — find a property by name, return NULL (don't have properties) */
void *__system_property_find(const char *name) { (void)name; return NULL; }

/* __system_property_wait — wait for property change (Bionic-only).
 * Returns 0 immediately (no properties available). */
void __system_property_wait(void) { }

/* __system_property_read_callback — read a property with callback (Bionic-only).
 * Used by libcutils.so under LIBC_O. Stub: does nothing.
 * Uses .symver to export under @@LIBC_O (non-default version). */
int _bf_sysprop_read_callback_impl(void *pi, void (*callback)(void*, const char*, const char*, void*), void *cookie);
__asm__(".symver _bf_sysprop_read_callback_impl, __system_property_read_callback@@LIBC_O");
int _bf_sysprop_read_callback_impl(void *pi, void (*callback)(void*, const char*, const char*, void*), void *cookie) {
    (void)pi; (void)callback; (void)cookie;
    return 1;
}

/* __system_property_foreach — iterate over system properties.
 * libcutils.so uses this to enumerate properties. */
void __system_property_foreach(void (*callback)(void *pi, void *cookie), void *cookie) {
    (void)callback; (void)cookie;
}

/* __system_properties_init — initialize system properties. */
void __system_properties_init(void) { }

/* __system_properties_zygote_reload — reload after zygote fork. */
void __system_properties_zygote_reload(void) { }

/* __system_property_serial — returns serial for a named property.
 * Used by liblog.so for property change detection. */
unsigned int __system_property_serial(const void *pi) { (void)pi; return 0; }

/* __system_property_read — stub that returns empty values */
int __system_property_read(void *pi, char *name, char *value) {
    (void)pi; (void)name;
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

/* __sF — Bionic FILE table (replaced by _IO_2_1_stdin_ etc. in glibc).
 * libclang_rt.ubsan references this. A NULL pointer won't crash UBSan
 * if it's not actively writing diagnostics. */
__attribute__((used)) __attribute__((externally_visible))
void *bf___sF = NULL;
__asm__(".symver bf___sF, __sF@@LIBC");

/* ===== getprogname (Bionic-only, not in glibc) =====
 * liblog.so and other GSI libraries reference getprogname@@LIBC.
 * This is a Bionic call that reads the program name. We provide
 * a stub that returns "open-sober". We use the __asm__ rename trick
 * so the function is EXPORTED AS "getprogname" directly, and the
 * version script tags it @@LIBC. (Note: using .symver name@@LIBC
 * creates conflicting aliases that BFD ld rejects.) */
const char *getprogname(void) { return "open-sober"; }

/* ===== android_set_abort_message exported under @@LIBC =====
 * liblog.so references android_set_abort_message@@LIBC.
 * The version script bridges/libc_version.ver has LIBC { global: *; }
 * so any symbol we define here is automatically tagged @@LIBC. */
void android_set_abort_message(const char *msg) { (void)msg; }

/* ===== android_fdsan functions (Bionic-only fdsan) =====
 * File descriptor sanitizer functions referenced by libcutils and others
 * under LIBC_Q version. These don't exist in glibc.
 * We use the __asm__ rename trick: the function body has an internal name,
 * and we export only the versioned alias via .symver. This avoids BFD ld
 * conflicts (same-name at same-address issue). */

/* android_fdsan_get_owner_tag@@LIBC_Q */
uint64_t _bf_android_fdsan_get_owner_tag_impl(void);
__asm__(".symver _bf_android_fdsan_get_owner_tag_impl, android_fdsan_get_owner_tag@@LIBC_Q");
uint64_t _bf_android_fdsan_get_owner_tag_impl(void) { return 0; }

/* android_fdsan_create_owner_tag */
uint64_t _bf_android_fdsan_create_owner_tag_impl(void);
__asm__(".symver _bf_android_fdsan_create_owner_tag_impl, android_fdsan_create_owner_tag@@LIBC_Q");
uint64_t _bf_android_fdsan_create_owner_tag_impl(void) {
    return 0;
}

/* android_fdsan_exchange_owner_tag */
uint64_t _bf_android_fdsan_exchange_owner_tag_impl(void);
__asm__(".symver _bf_android_fdsan_exchange_owner_tag_impl, android_fdsan_exchange_owner_tag@@LIBC_Q");
uint64_t _bf_android_fdsan_exchange_owner_tag_impl(void) { return 0; }

/* android_fdsan_close_with_tag */
void _bf_android_fdsan_close_with_tag_impl(void);
__asm__(".symver _bf_android_fdsan_close_with_tag_impl, android_fdsan_close_with_tag@@LIBC_Q");
void _bf_android_fdsan_close_with_tag_impl(void) { }

/* ===== __write_chk@@LIBC_N (glibc function, needs LIBC_N version) =====
 * android_libcutils.so references __write_chk@LIBC_N. The generated
 * stub only exports it as @@LIBC (default version). We need an
 * additional alias under @@LIBC_N for version-specific resolution.
 * The alias points to the same function via a differently-named wrapper
 * to avoid BFD ld name conflicts. */
void _bf_write_chk_libc_n_alias(void);
__asm__(".symver _bf_write_chk_libc_n_alias, __write_chk@@LIBC_N");
void _bf_write_chk_libc_n_alias(void) {
    extern void _bf___write_chk_glibc(void);
    _bf___write_chk_glibc();
}

/* ===== android_get_application_target_sdk_version@@LIBC_N (Bionic-only) =====
 * Returns the target SDK version. android_libcutils.so references this. */
int _bf_android_get_app_target_sdk_impl(void);
__asm__(".symver _bf_android_get_app_target_sdk_impl, android_get_application_target_sdk_version@@LIBC_N");
int _bf_android_get_app_target_sdk_impl(void) { return 34; }  /* Android 14 */

/* ===== CFI slowpath stub (Bionic-only, CFI not needed on glibc) =====
 * __cfi_slowpath is referenced by libraries compiled with Control Flow
 * Integrity. On non-CFI builds it should be a no-op. The version tag
 * matters: libcodec2_hidl_client needs __cfi_slowpath@LIBC_OMR1. */

__attribute__((used)) __attribute__((externally_visible))
void __cfi_slowpath(void) { }
__asm__(".symver __cfi_slowpath, __cfi_slowpath@@LIBC_OMR1");

/* ===== getrandom@@LIBC_P (glibc, needs LIBC_P version alias)
 * libinput.so references getrandom@LIBC_P. */
void _bf_getrandom_libc_p_alias(void);
__asm__(".symver _bf_getrandom_libc_p_alias, getrandom@@LIBC_P");
void _bf_getrandom_libc_p_alias(void) {
    extern void _bf_getrandom_glibc(void);
    _bf_getrandom_glibc();
}

/* ===== aligned_alloc@@LIBC_P (glibc, needs LIBC_P version alias) */
void _bf_aligned_alloc_libc_p_alias(void);
__asm__(".symver _bf_aligned_alloc_libc_p_alias, aligned_alloc@@LIBC_P");
void _bf_aligned_alloc_libc_p_alias(void) {
    extern void _bf_aligned_alloc_glibc(void);
    _bf_aligned_alloc_glibc();
}

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
/* pthread_cond_clockwait — glibc 2.30+ function needed by libandroid.so
 * under LIBC_R. Thin wrapper calling into glibc. */
int _bf_pthread_cond_clockwait_impl(void *cond, void *mutex, int clockid,
                                     const struct timespec *ts) {
    extern int pthread_cond_clockwait(void *, void *, int, const struct timespec *);
    return pthread_cond_clockwait(cond, mutex, clockid, ts);
}
__asm__(".symver _bf_pthread_cond_clockwait_impl, pthread_cond_clockwait@@LIBC_R");

/* __assert — Bionic's assert, maps to glibc's __assert_fail.
 * GSI libraries reference __assert@@LIBC from libc.so. */
void __assert(const char *expr, const char *file, int line) {
    extern void __assert_fail(const char *, const char *, int, const char *);
    __assert_fail(expr, file, line, __func__);
}

/* android_getaddrinfofornet — Bionic-only DNS resolution function.
 * GSI libs reference android_getaddrinfofornet@@LIBC_Q from libc.so. */
/* android_getaddrinfofornet — Bionic-only DNS resolution function.
 * GSI libs reference android_getaddrinfofornet@@LIBC_Q from libc.so.
 * Re-export getaddrinfo under LIBC_Q version. */
int _bf_getaddrinfofornet_impl(const char *node, const char *service,
                                const void *hints, void **res) {
    extern int getaddrinfo(const char *, const char *, const void *, void **);
    return getaddrinfo(node, service, hints, res);
}
__asm__(".symver _bf_getaddrinfofornet_impl, android_getaddrinfofornet@@LIBC_Q");

/* __fread_chk — libroblox.so references __fread_chk@@LIBC_N.
 * Already exported as @@LIBC; provide LIBC_N alias too. */

/* __fread_chk — libroblox.so references this under LIBC_N.
 * The check version __fread_chk has 5 params: ptr, ptrlen, size, nmemb, stream.
 * Provide a thin wrapper matching Bionic's 4-arg signature. */
size_t _bf_fread_chk_libc_n_alias(void *ptr, size_t ptrlen, size_t size, size_t nmemb) {
    return __fread_chk(ptr, ptrlen, size, nmemb, (FILE*)0);
}
__asm__(".symver _bf_fread_chk_libc_n_alias, __fread_chk@@LIBC_N");

/* __sendto_chk — Bionic checked sendto, needed by libroblox.so under LIBC_O.
 * Provide a thin wrapper around glibc's sendto. */
#include <sys/types.h>
#include <sys/socket.h>
ssize_t _bf_sendto_chk_libc_o_alias(int sockfd, const void *buf, size_t len, 
                                     size_t buflen, int flags,
                                     const struct sockaddr *dest_addr, socklen_t addrlen) {
    return sendto(sockfd, buf, len, flags, dest_addr, addrlen);
}
__asm__(".symver _bf_sendto_chk_libc_o_alias, __sendto_chk@@LIBC_O");

/* eventfd_write — needed by libgui.so under LIBC version. */
int _bf_eventfd_write_impl(int fd, unsigned long long value) {
    extern int eventfd_write(int, unsigned long long);
    return eventfd_write(fd, value);
}
__asm__(".symver _bf_eventfd_write_impl, eventfd_write@@LIBC");

/* eventfd_read — needed by libgui.so under LIBC version. */
int _bf_eventfd_read_impl(int fd, unsigned long long *value) {
    extern int eventfd_read(int, unsigned long long *);
    return eventfd_read(fd, value);
}
__asm__(".symver _bf_eventfd_read_impl, eventfd_read@@LIBC");

/* sem_clockwait — needed by libgui.so under LIBC_R version. */
int _bf_sem_clockwait_impl(int *sem, int clockid, const struct timespec *abstime) {
    extern int sem_clockwait(int *, int, const struct timespec *);
    return sem_clockwait(sem, clockid, abstime);
}
__asm__(".symver _bf_sem_clockwait_impl, sem_clockwait@@LIBC_R");
