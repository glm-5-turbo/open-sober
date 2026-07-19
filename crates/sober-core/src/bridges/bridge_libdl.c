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

/* __cfi_slowpath — CFI slowpath stub, needed by some GSI libs
 * under LIBC_OMR1 version, looked up from libdl.so.
 * On non-CFI builds this is a no-op. */
__attribute__((used)) __attribute__((externally_visible))
void __cfi_slowpath(void) { }
__asm__(".symver __cfi_slowpath, __cfi_slowpath@@LIBC_OMR1");