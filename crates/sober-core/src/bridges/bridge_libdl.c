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

/* ===== LIBDL_ANDROID namespace functions =====
 * These are Bionic-specific namespace functions that GSI libraries
 * reference from libdl.so. They don't exist in glibc, so we provide
 * minimal stubs that return sensible defaults. */

/* android_create_namespace — create a linker namespace (Bionic-only).
 * Returns NULL which effectively means "use the default namespace"
 * and will cause the caller to fall back to default behavior. */
void _bf_android_create_ns_stub(void);
__asm__(".symver _bf_android_create_ns_stub, android_create_namespace@@LIBDL_ANDROID");
void _bf_android_create_ns_stub(void) { }

/* android_get_exported_namespace — get an exported namespace by name.
 * Returns NULL (no additional namespaces available). */
void _bf_android_get_exported_ns_stub(void);
__asm__(".symver _bf_android_get_exported_ns_stub, android_get_exported_namespace@@LIBDL_ANDROID");
void _bf_android_get_exported_ns_stub(void) { }

/* android_link_namespaces — link two linker namespaces (Bionic-only). */
void _bf_android_link_ns_stub(void);
__asm__(".symver _bf_android_link_ns_stub, android_link_namespaces@@LIBDL_ANDROID");
void _bf_android_link_ns_stub(void) { }