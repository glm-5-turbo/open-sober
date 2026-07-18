#define _GNU_SOURCE
#include <dlfcn.h>
#include <string.h>
#include <stdio.h>

// LIBC versioned symbols that libroblox.so needs
// These just delegate to glibc

asm(".symver dlopen,dlopen@@LIBC");
asm(".symver dlsym,dlsym@@LIBC");
asm(".symver dlclose,dlclose@@LIBC");
asm(".symver dlerror,dlerror@@LIBC");

// Override dlerror to be useful
__attribute__((visibility("default"))) char *dlerror(void) {
    extern char *__real_dlerror(void);
    return __real_dlerror ? __real_dlerror() : (char*)"unknown error";
}
