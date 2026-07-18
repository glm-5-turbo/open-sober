#include <stdio.h>
#include <stddef.h>
#include <stdint.h>

// Android logging - Bionic's liblog is a thin wrapper around write()
int __android_log_print(int prio, const char *tag, const char *fmt, ...) { return 0; }

// Additional Bionic libc symbols not in NDK's static libc
void* __memcpy_chk(void* d, const void* s, size_t n, size_t os) { 
    extern void* memcpy(void*,const void*,size_t);
    return memcpy(d,s,n); 
}
void* __memset_chk(void* d, int c, size_t n, size_t os) {
    extern void* memset(void*,int,size_t);
    return memset(d,c,n); 
}
size_t __strlen_chk(const char* s, size_t os) {
    extern size_t strlen(const char*);
    return strlen(s); 
}
