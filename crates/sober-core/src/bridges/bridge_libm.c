/* libm.so version bridge for the Android sysroot.
 *
 * Provides LIBC_* version definitions that GSI libs expect from libm.so.
 * Wraps glibc's ARM64 libm (libm_glibc.so).
 *
 * Build:
 *   aarch64-linux-gnu-gcc -shared -fPIC -o libm.so bridge_libm.c \
 *       -Wl,--version-script,bridge_version.ver \
 *       -Wl,-soname,libm.so \
 *       -L. -lm_glibc
 */

/* Empty — all symbols come from the wrapped glibc libm or the bionic shim */