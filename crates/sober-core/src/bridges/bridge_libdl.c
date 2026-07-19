/* libdl.so version bridge for the Android sysroot.
 *
 * Provides LIBC_* version definitions that GSI libs expect from libdl.so.
 * All dlopen/dlsym/etc. symbols come from the linker or bionic shim.
 *
 * Build:
 *   aarch64-linux-gnu-gcc -shared -fPIC -o libdl.so bridge_libdl.c \
 *       -Wl,--version-script,bridge_version.ver \
 *       -Wl,-soname,libdl.so \
 *       -nostdlib
 */

/* Empty — dlfcn symbols are resolved at runtime by the linker/loader */