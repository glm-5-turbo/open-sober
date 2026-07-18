// Provide Bionic (LIBC) versioned symbols by wrapping glibc
extern void *__dlopen(const char *file, int mode) __asm__("dlopen");
extern void *__dlsym(void *handle, const char *name) __asm__("dlsym");
extern int __dlclose(void *handle) __asm__("dlclose");
extern char *__dlerror(void) __asm__("dlerror");

// Redefine with LIBC version
__asm__(".symver dlopen,dlopen@@LIBC");
void *dlopen(const char *file, int mode) { return __dlopen(file, mode); }

__asm__(".symver dlsym,dlsym@@LIBC");  
void *dlsym(void *handle, const char *name) { return __dlsym(handle, name); }

__asm__(".symver dlclose,dlclose@@LIBC");
int dlclose(void *handle) { return __dlclose(handle); }

__asm__(".symver dlerror,dlerror@@LIBC");
char *dlerror(void) { return __dlerror(); }
