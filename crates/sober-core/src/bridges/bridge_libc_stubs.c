/* Minimal libc version stubs for common glibc functions.
 *
 * GSI Android libraries reference standard C functions from libc.so
 * with version LIBC (e.g. "free@@LIBC"). Real glibc provides these
 * as "free@@GLIBC_2.17". This library re-exports the most common
 * ones under the LIBC version tag via .symver directives.
 *
 * Built as a separate .so that's placed before glibc in the search
 * order (via LD_LIBRARY_PATH order or as libc.so's dependency).
 *
 * Compile: aarch64-linux-gnu-gcc -shared -fPIC -o libc_wrapper.so \
 *   bridge_libc_stubs.c -ldl -Wl,--version-script,bridge_version.ver
 */

#define _GNU_SOURCE
#include <stddef.h>
#include <dlfcn.h>

/* Each LIBC-tagged symbol is a thin wrapper that dlsym's the real glibc function.
 * This ensures we never need to link directly against glibc. */

#define DECLARE_LIBC_STUB(ret, name, args, call_args) \
  static ret name##_real args; \
  __asm__(".symver name##_real," #name "@@LIBC"); \
  __attribute__((constructor)) static void _resolve_##name(void) { \
    name##_real = dlsym(RTLD_NEXT, #name); \
  }

/* Memory management */
DECLARE_LIBC_STUB(void, free, (void *p), (p))
DECLARE_LIBC_STUB(void *, malloc, (size_t s), (s))
DECLARE_LIBC_STUB(void *, calloc, (size_t n, size_t s), (n, s))
DECLARE_LIBC_STUB(void *, realloc, (void *p, size_t s), (p, s))

/* Strings */
DECLARE_LIBC_STUB(size_t, strlen, (const char *s), (s))
DECLARE_LIBC_STUB(int, strcmp, (const char *a, const char *b), (a, b))
DECLARE_LIBC_STUB(int, strncmp, (const char *a, const char *b, size_t n), (a, b, n))

/* Memory */
DECLARE_LIBC_STUB(void *, memcpy, (void *d, const void *s, size_t n), (d, s, n))
DECLARE_LIBC_STUB(int, memcmp, (const void *a, const void *b, size_t n), (a, b, n))
DECLARE_LIBC_STUB(void *, memchr, (const void *s, int c, size_t n), (s, c, n))

/* I/O */
DECLARE_LIBC_STUB(int, fprintf, (void *f, const char *fmt, ...), (f, fmt))
DECLARE_LIBC_STUB(size_t, fwrite, (const void *d, size_t s, size_t n, void *f), (d, s, n, f))
DECLARE_LIBC_STUB(int, fputc, (int c, void *f), (c, f))
DECLARE_LIBC_STUB(int, fflush, (void *f), (f))

/* Time */
DECLARE_LIBC_STUB(int, clock_gettime, (int id, void *tp), (id, tp))

/* System */
DECLARE_LIBC_STUB(void, abort, (void), ())
DECLARE_LIBC_STUB(long, syscall, (long n, ...), (n))
DECLARE_LIBC_STUB(int, isatty, (int fd), (fd))
DECLARE_LIBC_STUB(int, fileno, (void *f), (f))

/* pthread (from glibc's libpthread / libc) */
DECLARE_LIBC_STUB(int, pthread_mutex_lock, (void *m), (m))
DECLARE_LIBC_STUB(int, pthread_mutex_unlock, (void *m), (m))
DECLARE_LIBC_STUB(int, pthread_cond_wait, (void *c, void *m), (c, m))
DECLARE_LIBC_STUB(int, pthread_cond_broadcast, (void *c), (c))

/* Logging */
DECLARE_LIBC_STUB(void, openlog, (const char *id, int opt, int f), (id, opt, f))
DECLARE_LIBC_STUB(void, syslog, (int pri, const char *f, ...), (pri, f))
DECLARE_LIBC_STUB(void, closelog, (void), ())