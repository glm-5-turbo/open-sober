/* Auto-generated bionic shim C helpers. DO NOT EDIT.
 * 785 lazy-resolve entries, 14 data ptrs.
 */
#define _GNU_SOURCE
#include <stddef.h>
#include <dlfcn.h>
#include <pthread.h>

/* Dispatch table and data — defined in bionic_shim.S */
extern void *__bf_tramp_table[785];
__attribute__((weak)) void *__bf_c_resolve(int index);
extern void *__bf_data_stderr;
extern void *__bf_data___sF;
extern void *__bf_data_optarg;
extern void *__bf_data_optind;
extern void *__bf_data_tzname;
extern void *__bf_data_daylight;
extern void *__bf_data_timezone;
extern void *__bf_data_environ;
extern void *__bf_data_in6addr_any;
extern void *__bf_data_stdin;
extern void *__bf_data_stdout;
extern void *__bf_data_in6addr_loopback;
extern void *__bf_data___stack_chk_guard;
extern void *__bf_data_signgam;

/* ===== Bionic-only stubs ===== */

/* __system_property_get */
__asm__(".symver __system_property_get,__system_property_get@@LIBC");
int __system_property_get(const char *key, char *value) {
    (void)key; if (value) value[0] = '\0'; return 0;
}

/* android_set_abort_message */
__asm__(".symver android_set_abort_message,android_set_abort_message@@LIBC");
void android_set_abort_message(const char *msg) { (void)msg; }

/* arc4random_buf */
__asm__(".symver arc4random_buf,arc4random_buf@@LIBC");
void arc4random_buf(void *buf, size_t n) {
    long fd;
    register long x0 asm("x0") = -100;
    register const char *x1 asm("x1") = "/dev/urandom";
    register long x2 asm("x2") = 0; register long x8 asm("x8") = 56;
    asm volatile("svc #0" : "=r"(x0) : "r"(x0), "r"(x1), "r"(x2), "r"(x8));
    fd = x0;
    if (fd >= 0) {
        register long r0 asm("x0") = fd;
        register void *r1 asm("x1") = buf;
        register size_t r2 asm("x2") = n;
        register long r8_rd asm("x8") = 63;
        asm volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r8_rd));
        register long c0 asm("x0") = fd;
        register long r8_cl asm("x8") = 57;
        asm volatile("svc #0" : : "r"(c0), "r"(r8_cl));
    }
}

/* __assert2 */
__asm__(".symver __assert2,__assert2@@LIBC");
void __assert2(const char *f, int l, const char *fn, const char *msg) {
    (void)f; (void)l; (void)fn; (void)msg;
    register long r0 asm("x0") = 2;
    register const char *r1 asm("x1") = "Assertion failed\n";
    register long r2 asm("x2") = 16;
    register long r8 asm("x8") = 64;
    asm volatile("svc #0" : : "r"(r0), "r"(r1), "r"(r2), "r"(r8));
    asm volatile("udf #0");
}

/* __strncpy_chk2 */
__asm__(".symver __strncpy_chk2,__strncpy_chk2@@LIBC");
char *__strncpy_chk2(char *d, const char *s, size_t n, size_t dl) {
    (void)dl; size_t i; char *p = d;
    for (i = 0; i < n && *s; i++) *p++ = *s++;
    for (; i < n; i++) *p++ = '\0';
    return d;
}


/* ===== Lazy dispatch-table resolver ===== */
void* __bf_c_resolve(int index) {
    if (index < 0 || index >= 785)
        return NULL;
    if (index == 0) { __bf_tramp_table[0] = (void*)(unsigned long long)__bf_tramp_table[0]; }
    if (index == 0) __bf_tramp_table[0] = dlsym(RTLD_NEXT, "__cxa_finalize");
    if (index == 0) return __bf_tramp_table[0];
    if (index == 1) { __bf_tramp_table[1] = (void*)(unsigned long long)__bf_tramp_table[1]; }
    if (index == 1) __bf_tramp_table[1] = dlsym(RTLD_NEXT, "__cxa_atexit");
    if (index == 1) return __bf_tramp_table[1];
    if (index == 2) { __bf_tramp_table[2] = (void*)(unsigned long long)__bf_tramp_table[2]; }
    if (index == 2) __bf_tramp_table[2] = dlsym(RTLD_NEXT, "__register_atfork");
    if (index == 2) return __bf_tramp_table[2];
    if (index == 3) { __bf_tramp_table[3] = (void*)(unsigned long long)__bf_tramp_table[3]; }
    if (index == 3) __bf_tramp_table[3] = dlsym(RTLD_NEXT, "strlen");
    if (index == 3) return __bf_tramp_table[3];
    if (index == 4) { __bf_tramp_table[4] = (void*)(unsigned long long)__bf_tramp_table[4]; }
    if (index == 4) __bf_tramp_table[4] = dlsym(RTLD_NEXT, "memcmp");
    if (index == 4) return __bf_tramp_table[4];
    if (index == 5) { __bf_tramp_table[5] = (void*)(unsigned long long)__bf_tramp_table[5]; }
    if (index == 5) __bf_tramp_table[5] = dlsym(RTLD_NEXT, "pthread_mutex_init");
    if (index == 5) return __bf_tramp_table[5];
    if (index == 6) { __bf_tramp_table[6] = (void*)(unsigned long long)__bf_tramp_table[6]; }
    if (index == 6) __bf_tramp_table[6] = dlsym(RTLD_NEXT, "pthread_mutex_destroy");
    if (index == 6) return __bf_tramp_table[6];
    if (index == 7) { __bf_tramp_table[7] = (void*)(unsigned long long)__bf_tramp_table[7]; }
    if (index == 7) __bf_tramp_table[7] = dlsym(RTLD_NEXT, "pthread_once");
    if (index == 7) return __bf_tramp_table[7];
    if (index == 8) { __bf_tramp_table[8] = (void*)(unsigned long long)__bf_tramp_table[8]; }
    if (index == 8) __bf_tramp_table[8] = dlsym(RTLD_NEXT, "__memset_chk");
    if (index == 8) return __bf_tramp_table[8];
    if (index == 9) { __bf_tramp_table[9] = (void*)(unsigned long long)__bf_tramp_table[9]; }
    if (index == 9) __bf_tramp_table[9] = dlsym(RTLD_NEXT, "__memcpy_chk");
    if (index == 9) return __bf_tramp_table[9];
    if (index == 10) { __bf_tramp_table[10] = (void*)(unsigned long long)__bf_tramp_table[10]; }
    if (index == 10) __bf_tramp_table[10] = dlsym(RTLD_NEXT, "strlen");
    if (index == 10) return __bf_tramp_table[10];
    if (index == 11) { __bf_tramp_table[11] = (void*)(unsigned long long)__bf_tramp_table[11]; }
    if (index == 11) __bf_tramp_table[11] = dlsym(RTLD_NEXT, "memchr");
    if (index == 11) return __bf_tramp_table[11];
    if (index == 12) { __bf_tramp_table[12] = (void*)(unsigned long long)__bf_tramp_table[12]; }
    if (index == 12) __bf_tramp_table[12] = dlsym(RTLD_NEXT, "strncmp");
    if (index == 12) return __bf_tramp_table[12];
    if (index == 13) { __bf_tramp_table[13] = (void*)(unsigned long long)__bf_tramp_table[13]; }
    if (index == 13) __bf_tramp_table[13] = dlsym(RTLD_NEXT, "strcmp");
    if (index == 13) return __bf_tramp_table[13];
    if (index == 14) { __bf_tramp_table[14] = (void*)(unsigned long long)__bf_tramp_table[14]; }
    if (index == 14) __bf_tramp_table[14] = dlsym(RTLD_NEXT, "getauxval");
    if (index == 14) return __bf_tramp_table[14];
    if (index == 15) { __bf_tramp_table[15] = (void*)(unsigned long long)__bf_tramp_table[15]; }
    if (index == 15) __bf_tramp_table[15] = dlsym(RTLD_NEXT, "__errno_location");
    if (index == 15) return __bf_tramp_table[15];
    if (index == 16) { __bf_tramp_table[16] = (void*)(unsigned long long)__bf_tramp_table[16]; }
    if (index == 16) __bf_tramp_table[16] = dlsym(RTLD_NEXT, "close");
    if (index == 16) return __bf_tramp_table[16];
    if (index == 17) { __bf_tramp_table[17] = (void*)(unsigned long long)__bf_tramp_table[17]; }
    if (index == 17) __bf_tramp_table[17] = dlsym(RTLD_NEXT, "__open_2");
    if (index == 17) return __bf_tramp_table[17];
    if (index == 18) { __bf_tramp_table[18] = (void*)(unsigned long long)__bf_tramp_table[18]; }
    if (index == 18) __bf_tramp_table[18] = dlsym(RTLD_NEXT, "__read_chk");
    if (index == 18) return __bf_tramp_table[18];
    if (index == 19) { __bf_tramp_table[19] = (void*)(unsigned long long)__bf_tramp_table[19]; }
    if (index == 19) __bf_tramp_table[19] = dlsym(RTLD_NEXT, "read");
    if (index == 19) return __bf_tramp_table[19];
    if (index == 20) { __bf_tramp_table[20] = (void*)(unsigned long long)__bf_tramp_table[20]; }
    if (index == 20) __bf_tramp_table[20] = dlsym(RTLD_NEXT, "clock_gettime");
    if (index == 20) return __bf_tramp_table[20];
    if (index == 21) { __bf_tramp_table[21] = (void*)(unsigned long long)__bf_tramp_table[21]; }
    if (index == 21) __bf_tramp_table[21] = dlsym(RTLD_NEXT, "syscall");
    if (index == 21) return __bf_tramp_table[21];
    if (index == 22) { __bf_tramp_table[22] = (void*)(unsigned long long)__bf_tramp_table[22]; }
    if (index == 22) __bf_tramp_table[22] = dlsym(RTLD_NEXT, "sched_getcpu");
    if (index == 22) return __bf_tramp_table[22];
    if (index == 23) { __bf_tramp_table[23] = (void*)(unsigned long long)__bf_tramp_table[23]; }
    if (index == 23) __bf_tramp_table[23] = dlsym(RTLD_NEXT, "sysconf");
    if (index == 23) return __bf_tramp_table[23];
    if (index == 24) { __bf_tramp_table[24] = (void*)(unsigned long long)__bf_tramp_table[24]; }
    if (index == 24) __bf_tramp_table[24] = dlsym(RTLD_NEXT, "mmap");
    if (index == 24) return __bf_tramp_table[24];
    if (index == 25) { __bf_tramp_table[25] = (void*)(unsigned long long)__bf_tramp_table[25]; }
    if (index == 25) __bf_tramp_table[25] = dlsym(RTLD_NEXT, "mprotect");
    if (index == 25) return __bf_tramp_table[25];
    if (index == 26) { __bf_tramp_table[26] = (void*)(unsigned long long)__bf_tramp_table[26]; }
    if (index == 26) __bf_tramp_table[26] = dlsym(RTLD_NEXT, "munmap");
    if (index == 26) return __bf_tramp_table[26];
    if (index == 27) { __bf_tramp_table[27] = (void*)(unsigned long long)__bf_tramp_table[27]; }
    if (index == 27) __bf_tramp_table[27] = dlsym(RTLD_NEXT, "pthread_attr_init");
    if (index == 27) return __bf_tramp_table[27];
    if (index == 28) { __bf_tramp_table[28] = (void*)(unsigned long long)__bf_tramp_table[28]; }
    if (index == 28) __bf_tramp_table[28] = dlsym(RTLD_NEXT, "pthread_attr_setstacksize");
    if (index == 28) return __bf_tramp_table[28];
    if (index == 29) { __bf_tramp_table[29] = (void*)(unsigned long long)__bf_tramp_table[29]; }
    if (index == 29) __bf_tramp_table[29] = dlsym(RTLD_NEXT, "pthread_create");
    if (index == 29) return __bf_tramp_table[29];
    if (index == 30) { __bf_tramp_table[30] = (void*)(unsigned long long)__bf_tramp_table[30]; }
    if (index == 30) __bf_tramp_table[30] = dlsym(RTLD_NEXT, "pthread_attr_destroy");
    if (index == 30) return __bf_tramp_table[30];
    if (index == 31) { __bf_tramp_table[31] = (void*)(unsigned long long)__bf_tramp_table[31]; }
    if (index == 31) __bf_tramp_table[31] = dlsym(RTLD_NEXT, "pthread_join");
    if (index == 31) return __bf_tramp_table[31];
    if (index == 32) { __bf_tramp_table[32] = (void*)(unsigned long long)__bf_tramp_table[32]; }
    if (index == 32) __bf_tramp_table[32] = dlsym(RTLD_NEXT, "pthread_self");
    if (index == 32) return __bf_tramp_table[32];
    if (index == 33) { __bf_tramp_table[33] = (void*)(unsigned long long)__bf_tramp_table[33]; }
    if (index == 33) __bf_tramp_table[33] = dlsym(RTLD_NEXT, "memset");
    if (index == 33) return __bf_tramp_table[33];
    if (index == 34) { __bf_tramp_table[34] = (void*)(unsigned long long)__bf_tramp_table[34]; }
    if (index == 34) __bf_tramp_table[34] = dlsym(RTLD_NEXT, "pthread_setspecific");
    if (index == 34) return __bf_tramp_table[34];
    if (index == 35) { __bf_tramp_table[35] = (void*)(unsigned long long)__bf_tramp_table[35]; }
    if (index == 35) __bf_tramp_table[35] = dlsym(RTLD_NEXT, "__strncpy_chk");
    if (index == 35) return __bf_tramp_table[35];
    if (index == 36) { __bf_tramp_table[36] = (void*)(unsigned long long)__bf_tramp_table[36]; }
    if (index == 36) __bf_tramp_table[36] = dlsym(RTLD_NEXT, "sched_get_priority_max");
    if (index == 36) return __bf_tramp_table[36];
    if (index == 37) { __bf_tramp_table[37] = (void*)(unsigned long long)__bf_tramp_table[37]; }
    if (index == 37) __bf_tramp_table[37] = dlsym(RTLD_NEXT, "sched_setscheduler");
    if (index == 37) return __bf_tramp_table[37];
    if (index == 38) { __bf_tramp_table[38] = (void*)(unsigned long long)__bf_tramp_table[38]; }
    if (index == 38) __bf_tramp_table[38] = dlsym(RTLD_NEXT, "pthread_mutex_lock");
    if (index == 38) return __bf_tramp_table[38];
    if (index == 39) { __bf_tramp_table[39] = (void*)(unsigned long long)__bf_tramp_table[39]; }
    if (index == 39) __bf_tramp_table[39] = dlsym(RTLD_NEXT, "pthread_cond_wait");
    if (index == 39) return __bf_tramp_table[39];
    if (index == 40) { __bf_tramp_table[40] = (void*)(unsigned long long)__bf_tramp_table[40]; }
    if (index == 40) __bf_tramp_table[40] = dlsym(RTLD_NEXT, "pthread_mutex_unlock");
    if (index == 40) return __bf_tramp_table[40];
    if (index == 41) { __bf_tramp_table[41] = (void*)(unsigned long long)__bf_tramp_table[41]; }
    if (index == 41) __bf_tramp_table[41] = dlsym(RTLD_NEXT, "pthread_cond_signal");
    if (index == 41) return __bf_tramp_table[41];
    if (index == 42) { __bf_tramp_table[42] = (void*)(unsigned long long)__bf_tramp_table[42]; }
    if (index == 42) __bf_tramp_table[42] = dlsym(RTLD_NEXT, "pthread_cond_init");
    if (index == 42) return __bf_tramp_table[42];
    if (index == 43) { __bf_tramp_table[43] = (void*)(unsigned long long)__bf_tramp_table[43]; }
    if (index == 43) __bf_tramp_table[43] = dlsym(RTLD_NEXT, "gettid");
    if (index == 43) return __bf_tramp_table[43];
    if (index == 44) { __bf_tramp_table[44] = (void*)(unsigned long long)__bf_tramp_table[44]; }
    if (index == 44) __bf_tramp_table[44] = dlsym(RTLD_NEXT, "__vsprintf_chk");
    if (index == 44) return __bf_tramp_table[44];
    if (index == 45) { __bf_tramp_table[45] = (void*)(unsigned long long)__bf_tramp_table[45]; }
    if (index == 45) __bf_tramp_table[45] = dlsym(RTLD_NEXT, "atan2f");
    if (index == 45) return __bf_tramp_table[45];
    if (index == 46) { __bf_tramp_table[46] = (void*)(unsigned long long)__bf_tramp_table[46]; }
    if (index == 46) __bf_tramp_table[46] = dlsym(RTLD_NEXT, "vsnprintf");
    if (index == 46) return __bf_tramp_table[46];
    if (index == 47) { __bf_tramp_table[47] = (void*)(unsigned long long)__bf_tramp_table[47]; }
    if (index == 47) __bf_tramp_table[47] = dlsym(RTLD_NEXT, "fprintf");
    if (index == 47) return __bf_tramp_table[47];
    if (index == 48) { __bf_tramp_table[48] = (void*)(unsigned long long)__bf_tramp_table[48]; }
    if (index == 48) __bf_tramp_table[48] = dlsym(RTLD_NEXT, "rand");
    if (index == 48) return __bf_tramp_table[48];
    if (index == 49) { __bf_tramp_table[49] = (void*)(unsigned long long)__bf_tramp_table[49]; }
    if (index == 49) __bf_tramp_table[49] = dlsym(RTLD_NEXT, "strtoll");
    if (index == 49) return __bf_tramp_table[49];
    if (index == 50) { __bf_tramp_table[50] = (void*)(unsigned long long)__bf_tramp_table[50]; }
    if (index == 50) __bf_tramp_table[50] = dlsym(RTLD_NEXT, "time");
    if (index == 50) return __bf_tramp_table[50];
    if (index == 51) { __bf_tramp_table[51] = (void*)(unsigned long long)__bf_tramp_table[51]; }
    if (index == 51) __bf_tramp_table[51] = dlsym(RTLD_NEXT, "pthread_cond_broadcast");
    if (index == 51) return __bf_tramp_table[51];
    if (index == 52) { __bf_tramp_table[52] = (void*)(unsigned long long)__bf_tramp_table[52]; }
    if (index == 52) __bf_tramp_table[52] = dlsym(RTLD_NEXT, "pthread_cond_destroy");
    if (index == 52) return __bf_tramp_table[52];
    if (index == 53) { __bf_tramp_table[53] = (void*)(unsigned long long)__bf_tramp_table[53]; }
    if (index == 53) __bf_tramp_table[53] = dlsym(RTLD_NEXT, "stat");
    if (index == 53) return __bf_tramp_table[53];
    if (index == 54) { __bf_tramp_table[54] = (void*)(unsigned long long)__bf_tramp_table[54]; }
    if (index == 54) __bf_tramp_table[54] = dlsym(RTLD_NEXT, "opendir");
    if (index == 54) return __bf_tramp_table[54];
    if (index == 55) { __bf_tramp_table[55] = (void*)(unsigned long long)__bf_tramp_table[55]; }
    if (index == 55) __bf_tramp_table[55] = dlsym(RTLD_NEXT, "readdir");
    if (index == 55) return __bf_tramp_table[55];
    if (index == 56) { __bf_tramp_table[56] = (void*)(unsigned long long)__bf_tramp_table[56]; }
    if (index == 56) __bf_tramp_table[56] = dlsym(RTLD_NEXT, "closedir");
    if (index == 56) return __bf_tramp_table[56];
    if (index == 57) { __bf_tramp_table[57] = (void*)(unsigned long long)__bf_tramp_table[57]; }
    if (index == 57) __bf_tramp_table[57] = dlsym(RTLD_NEXT, "posix_fallocate");
    if (index == 57) return __bf_tramp_table[57];
    if (index == 58) { __bf_tramp_table[58] = (void*)(unsigned long long)__bf_tramp_table[58]; }
    if (index == 58) __bf_tramp_table[58] = dlsym(RTLD_NEXT, "open");
    if (index == 58) return __bf_tramp_table[58];
    if (index == 59) { __bf_tramp_table[59] = (void*)(unsigned long long)__bf_tramp_table[59]; }
    if (index == 59) __bf_tramp_table[59] = dlsym(RTLD_NEXT, "pthread_equal");
    if (index == 59) return __bf_tramp_table[59];
    if (index == 60) { __bf_tramp_table[60] = (void*)(unsigned long long)__bf_tramp_table[60]; }
    if (index == 60) __bf_tramp_table[60] = dlsym(RTLD_NEXT, "atoll");
    if (index == 60) return __bf_tramp_table[60];
    if (index == 61) { __bf_tramp_table[61] = (void*)(unsigned long long)__bf_tramp_table[61]; }
    if (index == 61) __bf_tramp_table[61] = dlsym(RTLD_NEXT, "srand");
    if (index == 61) return __bf_tramp_table[61];
    if (index == 62) { __bf_tramp_table[62] = (void*)(unsigned long long)__bf_tramp_table[62]; }
    if (index == 62) __bf_tramp_table[62] = dlsym(RTLD_NEXT, "strtod");
    if (index == 62) return __bf_tramp_table[62];
    if (index == 63) { __bf_tramp_table[63] = (void*)(unsigned long long)__bf_tramp_table[63]; }
    if (index == 63) __bf_tramp_table[63] = dlsym(RTLD_NEXT, "localtime");
    if (index == 63) return __bf_tramp_table[63];
    if (index == 64) { __bf_tramp_table[64] = (void*)(unsigned long long)__bf_tramp_table[64]; }
    if (index == 64) __bf_tramp_table[64] = dlsym(RTLD_NEXT, "fseek");
    if (index == 64) return __bf_tramp_table[64];
    if (index == 65) { __bf_tramp_table[65] = (void*)(unsigned long long)__bf_tramp_table[65]; }
    if (index == 65) __bf_tramp_table[65] = dlsym(RTLD_NEXT, "ftell");
    if (index == 65) return __bf_tramp_table[65];
    if (index == 66) { __bf_tramp_table[66] = (void*)(unsigned long long)__bf_tramp_table[66]; }
    if (index == 66) __bf_tramp_table[66] = dlsym(RTLD_NEXT, "fclose");
    if (index == 66) return __bf_tramp_table[66];
    if (index == 67) { __bf_tramp_table[67] = (void*)(unsigned long long)__bf_tramp_table[67]; }
    if (index == 67) __bf_tramp_table[67] = dlsym(RTLD_NEXT, "atan");
    if (index == 67) return __bf_tramp_table[67];
    if (index == 68) { __bf_tramp_table[68] = (void*)(unsigned long long)__bf_tramp_table[68]; }
    if (index == 68) __bf_tramp_table[68] = dlsym(RTLD_NEXT, "mkdir");
    if (index == 68) return __bf_tramp_table[68];
    if (index == 69) { __bf_tramp_table[69] = (void*)(unsigned long long)__bf_tramp_table[69]; }
    if (index == 69) __bf_tramp_table[69] = dlsym(RTLD_NEXT, "__fread_chk");
    if (index == 69) return __bf_tramp_table[69];
    if (index == 70) { __bf_tramp_table[70] = (void*)(unsigned long long)__bf_tramp_table[70]; }
    if (index == 70) __bf_tramp_table[70] = dlsym(RTLD_NEXT, "fread");
    if (index == 70) return __bf_tramp_table[70];
    if (index == 71) { __bf_tramp_table[71] = (void*)(unsigned long long)__bf_tramp_table[71]; }
    if (index == 71) __bf_tramp_table[71] = dlsym(RTLD_NEXT, "asinf");
    if (index == 71) return __bf_tramp_table[71];
    if (index == 72) { __bf_tramp_table[72] = (void*)(unsigned long long)__bf_tramp_table[72]; }
    if (index == 72) __bf_tramp_table[72] = dlsym(RTLD_NEXT, "access");
    if (index == 72) return __bf_tramp_table[72];
    if (index == 73) { __bf_tramp_table[73] = (void*)(unsigned long long)__bf_tramp_table[73]; }
    if (index == 73) __bf_tramp_table[73] = dlsym(RTLD_NEXT, "getenv");
    if (index == 73) return __bf_tramp_table[73];
    if (index == 74) { __bf_tramp_table[74] = (void*)(unsigned long long)__bf_tramp_table[74]; }
    if (index == 74) __bf_tramp_table[74] = dlsym(RTLD_NEXT, "strncpy");
    if (index == 74) return __bf_tramp_table[74];
    if (index == 75) { __bf_tramp_table[75] = (void*)(unsigned long long)__bf_tramp_table[75]; }
    if (index == 75) __bf_tramp_table[75] = dlsym(RTLD_NEXT, "feof");
    if (index == 75) return __bf_tramp_table[75];
    if (index == 76) { __bf_tramp_table[76] = (void*)(unsigned long long)__bf_tramp_table[76]; }
    if (index == 76) __bf_tramp_table[76] = dlsym(RTLD_NEXT, "fopen");
    if (index == 76) return __bf_tramp_table[76];
    if (index == 77) { __bf_tramp_table[77] = (void*)(unsigned long long)__bf_tramp_table[77]; }
    if (index == 77) __bf_tramp_table[77] = dlsym(RTLD_NEXT, "fseeko");
    if (index == 77) return __bf_tramp_table[77];
    if (index == 78) { __bf_tramp_table[78] = (void*)(unsigned long long)__bf_tramp_table[78]; }
    if (index == 78) __bf_tramp_table[78] = dlsym(RTLD_NEXT, "ftello");
    if (index == 78) return __bf_tramp_table[78];
    if (index == 79) { __bf_tramp_table[79] = (void*)(unsigned long long)__bf_tramp_table[79]; }
    if (index == 79) __bf_tramp_table[79] = dlsym(RTLD_NEXT, "fwrite");
    if (index == 79) return __bf_tramp_table[79];
    if (index == 80) { __bf_tramp_table[80] = (void*)(unsigned long long)__bf_tramp_table[80]; }
    if (index == 80) __bf_tramp_table[80] = dlsym(RTLD_NEXT, "fflush");
    if (index == 80) return __bf_tramp_table[80];
    if (index == 81) { __bf_tramp_table[81] = (void*)(unsigned long long)__bf_tramp_table[81]; }
    if (index == 81) __bf_tramp_table[81] = dlsym(RTLD_NEXT, "gmtime");
    if (index == 81) return __bf_tramp_table[81];
    if (index == 82) { __bf_tramp_table[82] = (void*)(unsigned long long)__bf_tramp_table[82]; }
    if (index == 82) __bf_tramp_table[82] = dlsym(RTLD_NEXT, "mktime");
    if (index == 82) return __bf_tramp_table[82];
    if (index == 83) { __bf_tramp_table[83] = (void*)(unsigned long long)__bf_tramp_table[83]; }
    if (index == 83) __bf_tramp_table[83] = dlsym(RTLD_NEXT, "pipe");
    if (index == 83) return __bf_tramp_table[83];
    if (index == 84) { __bf_tramp_table[84] = (void*)(unsigned long long)__bf_tramp_table[84]; }
    if (index == 84) __bf_tramp_table[84] = dlsym(RTLD_NEXT, "write");
    if (index == 84) return __bf_tramp_table[84];
    if (index == 85) { __bf_tramp_table[85] = (void*)(unsigned long long)__bf_tramp_table[85]; }
    if (index == 85) __bf_tramp_table[85] = dlsym(RTLD_NEXT, "pthread_key_create");
    if (index == 85) return __bf_tramp_table[85];
    if (index == 86) { __bf_tramp_table[86] = (void*)(unsigned long long)__bf_tramp_table[86]; }
    if (index == 86) __bf_tramp_table[86] = dlsym(RTLD_NEXT, "abort");
    if (index == 86) return __bf_tramp_table[86];
    if (index == 87) { __bf_tramp_table[87] = (void*)(unsigned long long)__bf_tramp_table[87]; }
    if (index == 87) __bf_tramp_table[87] = dlsym(RTLD_NEXT, "__assert");
    if (index == 87) return __bf_tramp_table[87];
    if (index == 88) { __bf_tramp_table[88] = (void*)(unsigned long long)__bf_tramp_table[88]; }
    if (index == 88) __bf_tramp_table[88] = dlsym(RTLD_NEXT, "strrchr");
    if (index == 88) return __bf_tramp_table[88];
    if (index == 89) { __bf_tramp_table[89] = (void*)(unsigned long long)__bf_tramp_table[89]; }
    if (index == 89) __bf_tramp_table[89] = dlsym(RTLD_NEXT, "__stack_chk_fail");
    if (index == 89) return __bf_tramp_table[89];
    if (index == 90) { __bf_tramp_table[90] = (void*)(unsigned long long)__bf_tramp_table[90]; }
    if (index == 90) __bf_tramp_table[90] = dlsym(RTLD_NEXT, "atoi");
    if (index == 90) return __bf_tramp_table[90];
    if (index == 91) { __bf_tramp_table[91] = (void*)(unsigned long long)__bf_tramp_table[91]; }
    if (index == 91) __bf_tramp_table[91] = dlsym(RTLD_NEXT, "fcntl");
    if (index == 91) return __bf_tramp_table[91];
    if (index == 92) { __bf_tramp_table[92] = (void*)(unsigned long long)__bf_tramp_table[92]; }
    if (index == 92) __bf_tramp_table[92] = dlsym(RTLD_NEXT, "memcpy");
    if (index == 92) return __bf_tramp_table[92];
    if (index == 93) { __bf_tramp_table[93] = (void*)(unsigned long long)__bf_tramp_table[93]; }
    if (index == 93) __bf_tramp_table[93] = dlsym(RTLD_NEXT, "strcpy");
    if (index == 93) return __bf_tramp_table[93];
    if (index == 94) { __bf_tramp_table[94] = (void*)(unsigned long long)__bf_tramp_table[94]; }
    if (index == 94) __bf_tramp_table[94] = dlsym(RTLD_NEXT, "strerror");
    if (index == 94) return __bf_tramp_table[94];
    if (index == 95) { __bf_tramp_table[95] = (void*)(unsigned long long)__bf_tramp_table[95]; }
    if (index == 95) __bf_tramp_table[95] = dlsym(RTLD_NEXT, "pthread_attr_setdetachstate");
    if (index == 95) return __bf_tramp_table[95];
    if (index == 96) { __bf_tramp_table[96] = (void*)(unsigned long long)__bf_tramp_table[96]; }
    if (index == 96) __bf_tramp_table[96] = dlsym(RTLD_NEXT, "pthread_cond_timedwait");
    if (index == 96) return __bf_tramp_table[96];
    if (index == 97) { __bf_tramp_table[97] = (void*)(unsigned long long)__bf_tramp_table[97]; }
    if (index == 97) __bf_tramp_table[97] = dlsym(RTLD_NEXT, "memmove");
    if (index == 97) return __bf_tramp_table[97];
    if (index == 98) { __bf_tramp_table[98] = (void*)(unsigned long long)__bf_tramp_table[98]; }
    if (index == 98) __bf_tramp_table[98] = dlsym(RTLD_NEXT, "strtol");
    if (index == 98) return __bf_tramp_table[98];
    if (index == 99) { __bf_tramp_table[99] = (void*)(unsigned long long)__bf_tramp_table[99]; }
    if (index == 99) __bf_tramp_table[99] = dlsym(RTLD_NEXT, "getpid");
    if (index == 99) return __bf_tramp_table[99];
    if (index == 100) { __bf_tramp_table[100] = (void*)(unsigned long long)__bf_tramp_table[100]; }
    if (index == 100) __bf_tramp_table[100] = dlsym(RTLD_NEXT, "gettimeofday");
    if (index == 100) return __bf_tramp_table[100];
    if (index == 101) { __bf_tramp_table[101] = (void*)(unsigned long long)__bf_tramp_table[101]; }
    if (index == 101) __bf_tramp_table[101] = dlsym(RTLD_NEXT, "localtime_r");
    if (index == 101) return __bf_tramp_table[101];
    if (index == 102) { __bf_tramp_table[102] = (void*)(unsigned long long)__bf_tramp_table[102]; }
    if (index == 102) __bf_tramp_table[102] = dlsym(RTLD_NEXT, "fputs");
    if (index == 102) return __bf_tramp_table[102];
    if (index == 103) { __bf_tramp_table[103] = (void*)(unsigned long long)__bf_tramp_table[103]; }
    if (index == 103) __bf_tramp_table[103] = dlsym(RTLD_NEXT, "strerror_r");
    if (index == 103) return __bf_tramp_table[103];
    if (index == 104) { __bf_tramp_table[104] = (void*)(unsigned long long)__bf_tramp_table[104]; }
    if (index == 104) __bf_tramp_table[104] = dlsym(RTLD_NEXT, "snprintf");
    if (index == 104) return __bf_tramp_table[104];
    if (index == 105) { __bf_tramp_table[105] = (void*)(unsigned long long)__bf_tramp_table[105]; }
    if (index == 105) __bf_tramp_table[105] = dlsym(RTLD_NEXT, "prctl");
    if (index == 105) return __bf_tramp_table[105];
    if (index == 106) { __bf_tramp_table[106] = (void*)(unsigned long long)__bf_tramp_table[106]; }
    if (index == 106) __bf_tramp_table[106] = dlsym(RTLD_NEXT, "sigaltstack");
    if (index == 106) return __bf_tramp_table[106];
    if (index == 107) { __bf_tramp_table[107] = (void*)(unsigned long long)__bf_tramp_table[107]; }
    if (index == 107) __bf_tramp_table[107] = dlsym(RTLD_NEXT, "getpagesize");
    if (index == 107) return __bf_tramp_table[107];
    if (index == 108) { __bf_tramp_table[108] = (void*)(unsigned long long)__bf_tramp_table[108]; }
    if (index == 108) __bf_tramp_table[108] = dlsym(RTLD_NEXT, "pthread_getspecific");
    if (index == 108) return __bf_tramp_table[108];
    if (index == 109) { __bf_tramp_table[109] = (void*)(unsigned long long)__bf_tramp_table[109]; }
    if (index == 109) __bf_tramp_table[109] = dlsym(RTLD_NEXT, "fork");
    if (index == 109) return __bf_tramp_table[109];
    if (index == 110) { __bf_tramp_table[110] = (void*)(unsigned long long)__bf_tramp_table[110]; }
    if (index == 110) __bf_tramp_table[110] = dlsym(RTLD_NEXT, "waitpid");
    if (index == 110) return __bf_tramp_table[110];
    if (index == 111) { __bf_tramp_table[111] = (void*)(unsigned long long)__bf_tramp_table[111]; }
    if (index == 111) __bf_tramp_table[111] = dlsym(RTLD_NEXT, "execv");
    if (index == 111) return __bf_tramp_table[111];
    if (index == 112) { __bf_tramp_table[112] = (void*)(unsigned long long)__bf_tramp_table[112]; }
    if (index == 112) __bf_tramp_table[112] = dlsym(RTLD_NEXT, "_exit");
    if (index == 112) return __bf_tramp_table[112];
    if (index == 113) { __bf_tramp_table[113] = (void*)(unsigned long long)__bf_tramp_table[113]; }
    if (index == 113) __bf_tramp_table[113] = dlsym(RTLD_NEXT, "execve");
    if (index == 113) return __bf_tramp_table[113];
    if (index == 114) { __bf_tramp_table[114] = (void*)(unsigned long long)__bf_tramp_table[114]; }
    if (index == 114) __bf_tramp_table[114] = dlsym(RTLD_NEXT, "getopt_long");
    if (index == 114) return __bf_tramp_table[114];
    if (index == 115) { __bf_tramp_table[115] = (void*)(unsigned long long)__bf_tramp_table[115]; }
    if (index == 115) __bf_tramp_table[115] = dlsym(RTLD_NEXT, "getppid");
    if (index == 115) return __bf_tramp_table[115];
    if (index == 116) { __bf_tramp_table[116] = (void*)(unsigned long long)__bf_tramp_table[116]; }
    if (index == 116) __bf_tramp_table[116] = dlsym(RTLD_NEXT, "geteuid");
    if (index == 116) return __bf_tramp_table[116];
    if (index == 117) { __bf_tramp_table[117] = (void*)(unsigned long long)__bf_tramp_table[117]; }
    if (index == 117) __bf_tramp_table[117] = dlsym(RTLD_NEXT, "epoll_create1");
    if (index == 117) return __bf_tramp_table[117];
    if (index == 118) { __bf_tramp_table[118] = (void*)(unsigned long long)__bf_tramp_table[118]; }
    if (index == 118) __bf_tramp_table[118] = dlsym(RTLD_NEXT, "eventfd");
    if (index == 118) return __bf_tramp_table[118];
    if (index == 119) { __bf_tramp_table[119] = (void*)(unsigned long long)__bf_tramp_table[119]; }
    if (index == 119) __bf_tramp_table[119] = dlsym(RTLD_NEXT, "epoll_ctl");
    if (index == 119) return __bf_tramp_table[119];
    if (index == 120) { __bf_tramp_table[120] = (void*)(unsigned long long)__bf_tramp_table[120]; }
    if (index == 120) __bf_tramp_table[120] = dlsym(RTLD_NEXT, "getsockopt");
    if (index == 120) return __bf_tramp_table[120];
    if (index == 121) { __bf_tramp_table[121] = (void*)(unsigned long long)__bf_tramp_table[121]; }
    if (index == 121) __bf_tramp_table[121] = dlsym(RTLD_NEXT, "setsockopt");
    if (index == 121) return __bf_tramp_table[121];
    if (index == 122) { __bf_tramp_table[122] = (void*)(unsigned long long)__bf_tramp_table[122]; }
    if (index == 122) __bf_tramp_table[122] = dlsym(RTLD_NEXT, "epoll_wait");
    if (index == 122) return __bf_tramp_table[122];
    if (index == 123) { __bf_tramp_table[123] = (void*)(unsigned long long)__bf_tramp_table[123]; }
    if (index == 123) __bf_tramp_table[123] = dlsym(RTLD_NEXT, "getuid");
    if (index == 123) return __bf_tramp_table[123];
    if (index == 124) { __bf_tramp_table[124] = (void*)(unsigned long long)__bf_tramp_table[124]; }
    if (index == 124) __bf_tramp_table[124] = dlsym(RTLD_NEXT, "pthread_mutex_trylock");
    if (index == 124) return __bf_tramp_table[124];
    if (index == 125) { __bf_tramp_table[125] = (void*)(unsigned long long)__bf_tramp_table[125]; }
    if (index == 125) __bf_tramp_table[125] = dlsym(RTLD_NEXT, "ldexp");
    if (index == 125) return __bf_tramp_table[125];
    if (index == 126) { __bf_tramp_table[126] = (void*)(unsigned long long)__bf_tramp_table[126]; }
    if (index == 126) __bf_tramp_table[126] = dlsym(RTLD_NEXT, "strchr");
    if (index == 126) return __bf_tramp_table[126];
    if (index == 127) { __bf_tramp_table[127] = (void*)(unsigned long long)__bf_tramp_table[127]; }
    if (index == 127) __bf_tramp_table[127] = dlsym(RTLD_NEXT, "getaddrinfo");
    if (index == 127) return __bf_tramp_table[127];
    if (index == 128) { __bf_tramp_table[128] = (void*)(unsigned long long)__bf_tramp_table[128]; }
    if (index == 128) __bf_tramp_table[128] = dlsym(RTLD_NEXT, "socket");
    if (index == 128) return __bf_tramp_table[128];
    if (index == 129) { __bf_tramp_table[129] = (void*)(unsigned long long)__bf_tramp_table[129]; }
    if (index == 129) __bf_tramp_table[129] = dlsym(RTLD_NEXT, "freeaddrinfo");
    if (index == 129) return __bf_tramp_table[129];
    if (index == 130) { __bf_tramp_table[130] = (void*)(unsigned long long)__bf_tramp_table[130]; }
    if (index == 130) __bf_tramp_table[130] = dlsym(RTLD_NEXT, "connect");
    if (index == 130) return __bf_tramp_table[130];
    if (index == 131) { __bf_tramp_table[131] = (void*)(unsigned long long)__bf_tramp_table[131]; }
    if (index == 131) __bf_tramp_table[131] = dlsym(RTLD_NEXT, "poll");
    if (index == 131) return __bf_tramp_table[131];
    if (index == 132) { __bf_tramp_table[132] = (void*)(unsigned long long)__bf_tramp_table[132]; }
    if (index == 132) __bf_tramp_table[132] = dlsym(RTLD_NEXT, "sched_getscheduler");
    if (index == 132) return __bf_tramp_table[132];
    if (index == 133) { __bf_tramp_table[133] = (void*)(unsigned long long)__bf_tramp_table[133]; }
    if (index == 133) __bf_tramp_table[133] = dlsym(RTLD_NEXT, "sched_getparam");
    if (index == 133) return __bf_tramp_table[133];
    if (index == 134) { __bf_tramp_table[134] = (void*)(unsigned long long)__bf_tramp_table[134]; }
    if (index == 134) __bf_tramp_table[134] = dlsym(RTLD_NEXT, "getpriority");
    if (index == 134) return __bf_tramp_table[134];
    if (index == 135) { __bf_tramp_table[135] = (void*)(unsigned long long)__bf_tramp_table[135]; }
    if (index == 135) __bf_tramp_table[135] = dlsym(RTLD_NEXT, "uname");
    if (index == 135) return __bf_tramp_table[135];
    if (index == 136) { __bf_tramp_table[136] = (void*)(unsigned long long)__bf_tramp_table[136]; }
    if (index == 136) __bf_tramp_table[136] = dlsym(RTLD_NEXT, "tzset");
    if (index == 136) return __bf_tramp_table[136];
    if (index == 137) { __bf_tramp_table[137] = (void*)(unsigned long long)__bf_tramp_table[137]; }
    if (index == 137) __bf_tramp_table[137] = dlsym(RTLD_NEXT, "strnlen");
    if (index == 137) return __bf_tramp_table[137];
    if (index == 138) { __bf_tramp_table[138] = (void*)(unsigned long long)__bf_tramp_table[138]; }
    if (index == 138) __bf_tramp_table[138] = dlsym(RTLD_NEXT, "writev");
    if (index == 138) return __bf_tramp_table[138];
    if (index == 139) { __bf_tramp_table[139] = (void*)(unsigned long long)__bf_tramp_table[139]; }
    if (index == 139) __bf_tramp_table[139] = dlsym(RTLD_NEXT, "sscanf");
    if (index == 139) return __bf_tramp_table[139];
    if (index == 140) { __bf_tramp_table[140] = (void*)(unsigned long long)__bf_tramp_table[140]; }
    if (index == 140) __bf_tramp_table[140] = dlsym(RTLD_NEXT, "strtoul");
    if (index == 140) return __bf_tramp_table[140];
    if (index == 141) { __bf_tramp_table[141] = (void*)(unsigned long long)__bf_tramp_table[141]; }
    if (index == 141) __bf_tramp_table[141] = dlsym(RTLD_NEXT, "strtoull");
    if (index == 141) return __bf_tramp_table[141];
    if (index == 142) { __bf_tramp_table[142] = (void*)(unsigned long long)__bf_tramp_table[142]; }
    if (index == 142) __bf_tramp_table[142] = dlsym(RTLD_NEXT, "lseek");
    if (index == 142) return __bf_tramp_table[142];
    if (index == 143) { __bf_tramp_table[143] = (void*)(unsigned long long)__bf_tramp_table[143]; }
    if (index == 143) __bf_tramp_table[143] = dlsym(RTLD_NEXT, "ftruncate");
    if (index == 143) return __bf_tramp_table[143];
    if (index == 144) { __bf_tramp_table[144] = (void*)(unsigned long long)__bf_tramp_table[144]; }
    if (index == 144) __bf_tramp_table[144] = dlsym(RTLD_NEXT, "fstat");
    if (index == 144) return __bf_tramp_table[144];
    if (index == 145) { __bf_tramp_table[145] = (void*)(unsigned long long)__bf_tramp_table[145]; }
    if (index == 145) __bf_tramp_table[145] = dlsym(RTLD_NEXT, "lstat");
    if (index == 145) return __bf_tramp_table[145];
    if (index == 146) { __bf_tramp_table[146] = (void*)(unsigned long long)__bf_tramp_table[146]; }
    if (index == 146) __bf_tramp_table[146] = dlsym(RTLD_NEXT, "rename");
    if (index == 146) return __bf_tramp_table[146];
    if (index == 147) { __bf_tramp_table[147] = (void*)(unsigned long long)__bf_tramp_table[147]; }
    if (index == 147) __bf_tramp_table[147] = dlsym(RTLD_NEXT, "unlink");
    if (index == 147) return __bf_tramp_table[147];
    if (index == 148) { __bf_tramp_table[148] = (void*)(unsigned long long)__bf_tramp_table[148]; }
    if (index == 148) __bf_tramp_table[148] = dlsym(RTLD_NEXT, "rmdir");
    if (index == 148) return __bf_tramp_table[148];
    if (index == 149) { __bf_tramp_table[149] = (void*)(unsigned long long)__bf_tramp_table[149]; }
    if (index == 149) __bf_tramp_table[149] = dlsym(RTLD_NEXT, "nanosleep");
    if (index == 149) return __bf_tramp_table[149];
    if (index == 150) { __bf_tramp_table[150] = (void*)(unsigned long long)__bf_tramp_table[150]; }
    if (index == 150) __bf_tramp_table[150] = dlsym(RTLD_NEXT, "sigemptyset");
    if (index == 150) return __bf_tramp_table[150];
    if (index == 151) { __bf_tramp_table[151] = (void*)(unsigned long long)__bf_tramp_table[151]; }
    if (index == 151) __bf_tramp_table[151] = dlsym(RTLD_NEXT, "sigaction");
    if (index == 151) return __bf_tramp_table[151];
    if (index == 152) { __bf_tramp_table[152] = (void*)(unsigned long long)__bf_tramp_table[152]; }
    if (index == 152) __bf_tramp_table[152] = dlsym(RTLD_NEXT, "raise");
    if (index == 152) return __bf_tramp_table[152];
    if (index == 153) { __bf_tramp_table[153] = (void*)(unsigned long long)__bf_tramp_table[153]; }
    if (index == 153) __bf_tramp_table[153] = dlsym(RTLD_NEXT, "fscanf");
    if (index == 153) return __bf_tramp_table[153];
    if (index == 154) { __bf_tramp_table[154] = (void*)(unsigned long long)__bf_tramp_table[154]; }
    if (index == 154) __bf_tramp_table[154] = dlsym(RTLD_NEXT, "pread64");
    if (index == 154) return __bf_tramp_table[154];
    if (index == 155) { __bf_tramp_table[155] = (void*)(unsigned long long)__bf_tramp_table[155]; }
    if (index == 155) __bf_tramp_table[155] = dlsym(RTLD_NEXT, "ptrace");
    if (index == 155) return __bf_tramp_table[155];
    if (index == 156) { __bf_tramp_table[156] = (void*)(unsigned long long)__bf_tramp_table[156]; }
    if (index == 156) __bf_tramp_table[156] = dlsym(RTLD_NEXT, "socketpair");
    if (index == 156) return __bf_tramp_table[156];
    if (index == 157) { __bf_tramp_table[157] = (void*)(unsigned long long)__bf_tramp_table[157]; }
    if (index == 157) __bf_tramp_table[157] = dlsym(RTLD_NEXT, "sendmsg");
    if (index == 157) return __bf_tramp_table[157];
    if (index == 158) { __bf_tramp_table[158] = (void*)(unsigned long long)__bf_tramp_table[158]; }
    if (index == 158) __bf_tramp_table[158] = dlsym(RTLD_NEXT, "recvmsg");
    if (index == 158) return __bf_tramp_table[158];
    if (index == 159) { __bf_tramp_table[159] = (void*)(unsigned long long)__bf_tramp_table[159]; }
    if (index == 159) __bf_tramp_table[159] = dlsym(RTLD_NEXT, "__cmsg_nxthdr");
    if (index == 159) return __bf_tramp_table[159];
    if (index == 160) { __bf_tramp_table[160] = (void*)(unsigned long long)__bf_tramp_table[160]; }
    if (index == 160) __bf_tramp_table[160] = dlsym(RTLD_NEXT, "readlink");
    if (index == 160) return __bf_tramp_table[160];
    if (index == 161) { __bf_tramp_table[161] = (void*)(unsigned long long)__bf_tramp_table[161]; }
    if (index == 161) __bf_tramp_table[161] = dlsym(RTLD_NEXT, "printf");
    if (index == 161) return __bf_tramp_table[161];
    if (index == 162) { __bf_tramp_table[162] = (void*)(unsigned long long)__bf_tramp_table[162]; }
    if (index == 162) __bf_tramp_table[162] = dlsym(RTLD_NEXT, "wmemchr");
    if (index == 162) return __bf_tramp_table[162];
    if (index == 163) { __bf_tramp_table[163] = (void*)(unsigned long long)__bf_tramp_table[163]; }
    if (index == 163) __bf_tramp_table[163] = dlsym(RTLD_NEXT, "localeconv");
    if (index == 163) return __bf_tramp_table[163];
    if (index == 164) { __bf_tramp_table[164] = (void*)(unsigned long long)__bf_tramp_table[164]; }
    if (index == 164) __bf_tramp_table[164] = dlsym(RTLD_NEXT, "__vsnprintf_chk");
    if (index == 164) return __bf_tramp_table[164];
    if (index == 165) { __bf_tramp_table[165] = (void*)(unsigned long long)__bf_tramp_table[165]; }
    if (index == 165) __bf_tramp_table[165] = dlsym(RTLD_NEXT, "__memmove_chk");
    if (index == 165) return __bf_tramp_table[165];
    if (index == 166) { __bf_tramp_table[166] = (void*)(unsigned long long)__bf_tramp_table[166]; }
    if (index == 166) __bf_tramp_table[166] = dlsym(RTLD_NEXT, "sched_yield");
    if (index == 166) return __bf_tramp_table[166];
    if (index == 167) { __bf_tramp_table[167] = (void*)(unsigned long long)__bf_tramp_table[167]; }
    if (index == 167) __bf_tramp_table[167] = dlsym(RTLD_NEXT, "modf");
    if (index == 167) return __bf_tramp_table[167];
    if (index == 168) { __bf_tramp_table[168] = (void*)(unsigned long long)__bf_tramp_table[168]; }
    if (index == 168) __bf_tramp_table[168] = dlsym(RTLD_NEXT, "strcasecmp");
    if (index == 168) return __bf_tramp_table[168];
    if (index == 169) { __bf_tramp_table[169] = (void*)(unsigned long long)__bf_tramp_table[169]; }
    if (index == 169) __bf_tramp_table[169] = dlsym(RTLD_NEXT, "getnameinfo");
    if (index == 169) return __bf_tramp_table[169];
    if (index == 170) { __bf_tramp_table[170] = (void*)(unsigned long long)__bf_tramp_table[170]; }
    if (index == 170) __bf_tramp_table[170] = dlsym(RTLD_NEXT, "strftime");
    if (index == 170) return __bf_tramp_table[170];
    if (index == 171) { __bf_tramp_table[171] = (void*)(unsigned long long)__bf_tramp_table[171]; }
    if (index == 171) __bf_tramp_table[171] = dlsym(RTLD_NEXT, "__strcpy_chk");
    if (index == 171) return __bf_tramp_table[171];
    if (index == 172) { __bf_tramp_table[172] = (void*)(unsigned long long)__bf_tramp_table[172]; }
    if (index == 172) __bf_tramp_table[172] = dlsym(RTLD_NEXT, "frexpf");
    if (index == 172) return __bf_tramp_table[172];
    if (index == 173) { __bf_tramp_table[173] = (void*)(unsigned long long)__bf_tramp_table[173]; }
    if (index == 173) __bf_tramp_table[173] = dlsym(RTLD_NEXT, "ldexpf");
    if (index == 173) return __bf_tramp_table[173];
    if (index == 174) { __bf_tramp_table[174] = (void*)(unsigned long long)__bf_tramp_table[174]; }
    if (index == 174) __bf_tramp_table[174] = dlsym(RTLD_NEXT, "tanf");
    if (index == 174) return __bf_tramp_table[174];
    if (index == 175) { __bf_tramp_table[175] = (void*)(unsigned long long)__bf_tramp_table[175]; }
    if (index == 175) __bf_tramp_table[175] = dlsym(RTLD_NEXT, "atanf");
    if (index == 175) return __bf_tramp_table[175];
    if (index == 176) { __bf_tramp_table[176] = (void*)(unsigned long long)__bf_tramp_table[176]; }
    if (index == 176) __bf_tramp_table[176] = dlsym(RTLD_NEXT, "erff");
    if (index == 176) return __bf_tramp_table[176];
    if (index == 177) { __bf_tramp_table[177] = (void*)(unsigned long long)__bf_tramp_table[177]; }
    if (index == 177) __bf_tramp_table[177] = dlsym(RTLD_NEXT, "acosf");
    if (index == 177) return __bf_tramp_table[177];
    if (index == 178) { __bf_tramp_table[178] = (void*)(unsigned long long)__bf_tramp_table[178]; }
    if (index == 178) __bf_tramp_table[178] = dlsym(RTLD_NEXT, "strstr");
    if (index == 178) return __bf_tramp_table[178];
    if (index == 179) { __bf_tramp_table[179] = (void*)(unsigned long long)__bf_tramp_table[179]; }
    if (index == 179) __bf_tramp_table[179] = dlsym(RTLD_NEXT, "erfcf");
    if (index == 179) return __bf_tramp_table[179];
    if (index == 180) { __bf_tramp_table[180] = (void*)(unsigned long long)__bf_tramp_table[180]; }
    if (index == 180) __bf_tramp_table[180] = dlsym(RTLD_NEXT, "modff");
    if (index == 180) return __bf_tramp_table[180];
    if (index == 181) { __bf_tramp_table[181] = (void*)(unsigned long long)__bf_tramp_table[181]; }
    if (index == 181) __bf_tramp_table[181] = dlsym(RTLD_NEXT, "coshf");
    if (index == 181) return __bf_tramp_table[181];
    if (index == 182) { __bf_tramp_table[182] = (void*)(unsigned long long)__bf_tramp_table[182]; }
    if (index == 182) __bf_tramp_table[182] = dlsym(RTLD_NEXT, "sinhf");
    if (index == 182) return __bf_tramp_table[182];
    if (index == 183) { __bf_tramp_table[183] = (void*)(unsigned long long)__bf_tramp_table[183]; }
    if (index == 183) __bf_tramp_table[183] = dlsym(RTLD_NEXT, "tanhf");
    if (index == 183) return __bf_tramp_table[183];
    if (index == 184) { __bf_tramp_table[184] = (void*)(unsigned long long)__bf_tramp_table[184]; }
    if (index == 184) __bf_tramp_table[184] = dlsym(RTLD_NEXT, "atan2");
    if (index == 184) return __bf_tramp_table[184];
    if (index == 185) { __bf_tramp_table[185] = (void*)(unsigned long long)__bf_tramp_table[185]; }
    if (index == 185) __bf_tramp_table[185] = dlsym(RTLD_NEXT, "cbrtf");
    if (index == 185) return __bf_tramp_table[185];
    if (index == 186) { __bf_tramp_table[186] = (void*)(unsigned long long)__bf_tramp_table[186]; }
    if (index == 186) __bf_tramp_table[186] = dlsym(RTLD_NEXT, "strchr");
    if (index == 186) return __bf_tramp_table[186];
    if (index == 187) { __bf_tramp_table[187] = (void*)(unsigned long long)__bf_tramp_table[187]; }
    if (index == 187) __bf_tramp_table[187] = dlsym(RTLD_NEXT, "clock");
    if (index == 187) return __bf_tramp_table[187];
    if (index == 188) { __bf_tramp_table[188] = (void*)(unsigned long long)__bf_tramp_table[188]; }
    if (index == 188) __bf_tramp_table[188] = dlsym(RTLD_NEXT, "fileno");
    if (index == 188) return __bf_tramp_table[188];
    if (index == 189) { __bf_tramp_table[189] = (void*)(unsigned long long)__bf_tramp_table[189]; }
    if (index == 189) __bf_tramp_table[189] = dlsym(RTLD_NEXT, "remainderf");
    if (index == 189) return __bf_tramp_table[189];
    if (index == 190) { __bf_tramp_table[190] = (void*)(unsigned long long)__bf_tramp_table[190]; }
    if (index == 190) __bf_tramp_table[190] = dlsym(RTLD_NEXT, "nan");
    if (index == 190) return __bf_tramp_table[190];
    if (index == 191) { __bf_tramp_table[191] = (void*)(unsigned long long)__bf_tramp_table[191]; }
    if (index == 191) __bf_tramp_table[191] = dlsym(RTLD_NEXT, "qsort");
    if (index == 191) return __bf_tramp_table[191];
    if (index == 192) { __bf_tramp_table[192] = (void*)(unsigned long long)__bf_tramp_table[192]; }
    if (index == 192) __bf_tramp_table[192] = dlsym(RTLD_NEXT, "nextafterf");
    if (index == 192) return __bf_tramp_table[192];
    if (index == 193) { __bf_tramp_table[193] = (void*)(unsigned long long)__bf_tramp_table[193]; }
    if (index == 193) __bf_tramp_table[193] = dlsym(RTLD_NEXT, "acos");
    if (index == 193) return __bf_tramp_table[193];
    if (index == 194) { __bf_tramp_table[194] = (void*)(unsigned long long)__bf_tramp_table[194]; }
    if (index == 194) __bf_tramp_table[194] = dlsym(RTLD_NEXT, "asin");
    if (index == 194) return __bf_tramp_table[194];
    if (index == 195) { __bf_tramp_table[195] = (void*)(unsigned long long)__bf_tramp_table[195]; }
    if (index == 195) __bf_tramp_table[195] = dlsym(RTLD_NEXT, "ilogb");
    if (index == 195) return __bf_tramp_table[195];
    if (index == 196) { __bf_tramp_table[196] = (void*)(unsigned long long)__bf_tramp_table[196]; }
    if (index == 196) __bf_tramp_table[196] = dlsym(RTLD_NEXT, "FD_SET");
    if (index == 196) return __bf_tramp_table[196];
    if (index == 197) { __bf_tramp_table[197] = (void*)(unsigned long long)__bf_tramp_table[197]; }
    if (index == 197) __bf_tramp_table[197] = dlsym(RTLD_NEXT, "select");
    if (index == 197) return __bf_tramp_table[197];
    if (index == 198) { __bf_tramp_table[198] = (void*)(unsigned long long)__bf_tramp_table[198]; }
    if (index == 198) __bf_tramp_table[198] = dlsym(RTLD_NEXT, "FD_ISSET");
    if (index == 198) return __bf_tramp_table[198];
    if (index == 199) { __bf_tramp_table[199] = (void*)(unsigned long long)__bf_tramp_table[199]; }
    if (index == 199) __bf_tramp_table[199] = dlsym(RTLD_NEXT, "sendto");
    if (index == 199) return __bf_tramp_table[199];
    if (index == 200) { __bf_tramp_table[200] = (void*)(unsigned long long)__bf_tramp_table[200]; }
    if (index == 200) __bf_tramp_table[200] = dlsym(RTLD_NEXT, "recvfrom");
    if (index == 200) return __bf_tramp_table[200];
    if (index == 201) { __bf_tramp_table[201] = (void*)(unsigned long long)__bf_tramp_table[201]; }
    if (index == 201) __bf_tramp_table[201] = dlsym(RTLD_NEXT, "__strcat_chk");
    if (index == 201) return __bf_tramp_table[201];
    if (index == 202) { __bf_tramp_table[202] = (void*)(unsigned long long)__bf_tramp_table[202]; }
    if (index == 202) __bf_tramp_table[202] = dlsym(RTLD_NEXT, "setpriority");
    if (index == 202) return __bf_tramp_table[202];
    if (index == 203) { __bf_tramp_table[203] = (void*)(unsigned long long)__bf_tramp_table[203]; }
    if (index == 203) __bf_tramp_table[203] = dlsym(RTLD_NEXT, "pthread_mutexattr_init");
    if (index == 203) return __bf_tramp_table[203];
    if (index == 204) { __bf_tramp_table[204] = (void*)(unsigned long long)__bf_tramp_table[204]; }
    if (index == 204) __bf_tramp_table[204] = dlsym(RTLD_NEXT, "pthread_mutexattr_settype");
    if (index == 204) return __bf_tramp_table[204];
    if (index == 205) { __bf_tramp_table[205] = (void*)(unsigned long long)__bf_tramp_table[205]; }
    if (index == 205) __bf_tramp_table[205] = dlsym(RTLD_NEXT, "pthread_mutexattr_destroy");
    if (index == 205) return __bf_tramp_table[205];
    if (index == 206) { __bf_tramp_table[206] = (void*)(unsigned long long)__bf_tramp_table[206]; }
    if (index == 206) __bf_tramp_table[206] = dlsym(RTLD_NEXT, "sem_init");
    if (index == 206) return __bf_tramp_table[206];
    if (index == 207) { __bf_tramp_table[207] = (void*)(unsigned long long)__bf_tramp_table[207]; }
    if (index == 207) __bf_tramp_table[207] = dlsym(RTLD_NEXT, "sem_destroy");
    if (index == 207) return __bf_tramp_table[207];
    if (index == 208) { __bf_tramp_table[208] = (void*)(unsigned long long)__bf_tramp_table[208]; }
    if (index == 208) __bf_tramp_table[208] = dlsym(RTLD_NEXT, "sem_wait");
    if (index == 208) return __bf_tramp_table[208];
    if (index == 209) { __bf_tramp_table[209] = (void*)(unsigned long long)__bf_tramp_table[209]; }
    if (index == 209) __bf_tramp_table[209] = dlsym(RTLD_NEXT, "sem_post");
    if (index == 209) return __bf_tramp_table[209];
    if (index == 210) { __bf_tramp_table[210] = (void*)(unsigned long long)__bf_tramp_table[210]; }
    if (index == 210) __bf_tramp_table[210] = dlsym(RTLD_NEXT, "inet_ntop");
    if (index == 210) return __bf_tramp_table[210];
    if (index == 211) { __bf_tramp_table[211] = (void*)(unsigned long long)__bf_tramp_table[211]; }
    if (index == 211) __bf_tramp_table[211] = dlsym(RTLD_NEXT, "inet_pton");
    if (index == 211) return __bf_tramp_table[211];
    if (index == 212) { __bf_tramp_table[212] = (void*)(unsigned long long)__bf_tramp_table[212]; }
    if (index == 212) __bf_tramp_table[212] = dlsym(RTLD_NEXT, "strncasecmp");
    if (index == 212) return __bf_tramp_table[212];
    if (index == 213) { __bf_tramp_table[213] = (void*)(unsigned long long)__bf_tramp_table[213]; }
    if (index == 213) __bf_tramp_table[213] = dlsym(RTLD_NEXT, "pthread_attr_setschedparam");
    if (index == 213) return __bf_tramp_table[213];
    if (index == 214) { __bf_tramp_table[214] = (void*)(unsigned long long)__bf_tramp_table[214]; }
    if (index == 214) __bf_tramp_table[214] = dlsym(RTLD_NEXT, "bind");
    if (index == 214) return __bf_tramp_table[214];
    if (index == 215) { __bf_tramp_table[215] = (void*)(unsigned long long)__bf_tramp_table[215]; }
    if (index == 215) __bf_tramp_table[215] = dlsym(RTLD_NEXT, "getsockname");
    if (index == 215) return __bf_tramp_table[215];
    if (index == 216) { __bf_tramp_table[216] = (void*)(unsigned long long)__bf_tramp_table[216]; }
    if (index == 216) __bf_tramp_table[216] = dlsym(RTLD_NEXT, "gethostname");
    if (index == 216) return __bf_tramp_table[216];
    if (index == 217) { __bf_tramp_table[217] = (void*)(unsigned long long)__bf_tramp_table[217]; }
    if (index == 217) __bf_tramp_table[217] = dlsym(RTLD_NEXT, "__sendto_chk");
    if (index == 217) return __bf_tramp_table[217];
    if (index == 218) { __bf_tramp_table[218] = (void*)(unsigned long long)__bf_tramp_table[218]; }
    if (index == 218) __bf_tramp_table[218] = dlsym(RTLD_NEXT, "puts");
    if (index == 218) return __bf_tramp_table[218];
    if (index == 219) { __bf_tramp_table[219] = (void*)(unsigned long long)__bf_tramp_table[219]; }
    if (index == 219) __bf_tramp_table[219] = dlsym(RTLD_NEXT, "gai_strerror");
    if (index == 219) return __bf_tramp_table[219];
    if (index == 220) { __bf_tramp_table[220] = (void*)(unsigned long long)__bf_tramp_table[220]; }
    if (index == 220) __bf_tramp_table[220] = dlsym(RTLD_NEXT, "__write_chk");
    if (index == 220) return __bf_tramp_table[220];
    if (index == 221) { __bf_tramp_table[221] = (void*)(unsigned long long)__bf_tramp_table[221]; }
    if (index == 221) __bf_tramp_table[221] = dlsym(RTLD_NEXT, "__poll_chk");
    if (index == 221) return __bf_tramp_table[221];
    if (index == 222) { __bf_tramp_table[222] = (void*)(unsigned long long)__bf_tramp_table[222]; }
    if (index == 222) __bf_tramp_table[222] = dlsym(RTLD_NEXT, "vprintf");
    if (index == 222) return __bf_tramp_table[222];
    if (index == 223) { __bf_tramp_table[223] = (void*)(unsigned long long)__bf_tramp_table[223]; }
    if (index == 223) __bf_tramp_table[223] = dlsym(RTLD_NEXT, "usleep");
    if (index == 223) return __bf_tramp_table[223];
    if (index == 224) { __bf_tramp_table[224] = (void*)(unsigned long long)__bf_tramp_table[224]; }
    if (index == 224) __bf_tramp_table[224] = dlsym(RTLD_NEXT, "pthread_kill");
    if (index == 224) return __bf_tramp_table[224];
    if (index == 225) { __bf_tramp_table[225] = (void*)(unsigned long long)__bf_tramp_table[225]; }
    if (index == 225) __bf_tramp_table[225] = dlsym(RTLD_NEXT, "pthread_detach");
    if (index == 225) return __bf_tramp_table[225];
    if (index == 226) { __bf_tramp_table[226] = (void*)(unsigned long long)__bf_tramp_table[226]; }
    if (index == 226) __bf_tramp_table[226] = dlsym(RTLD_NEXT, "exit");
    if (index == 226) return __bf_tramp_table[226];
    if (index == 227) { __bf_tramp_table[227] = (void*)(unsigned long long)__bf_tramp_table[227]; }
    if (index == 227) __bf_tramp_table[227] = dlsym(RTLD_NEXT, "ferror");
    if (index == 227) return __bf_tramp_table[227];
    if (index == 228) { __bf_tramp_table[228] = (void*)(unsigned long long)__bf_tramp_table[228]; }
    if (index == 228) __bf_tramp_table[228] = dlsym(RTLD_NEXT, "clearerr");
    if (index == 228) return __bf_tramp_table[228];
    if (index == 229) { __bf_tramp_table[229] = (void*)(unsigned long long)__bf_tramp_table[229]; }
    if (index == 229) __bf_tramp_table[229] = dlsym(RTLD_NEXT, "wcslen");
    if (index == 229) return __bf_tramp_table[229];
    if (index == 230) { __bf_tramp_table[230] = (void*)(unsigned long long)__bf_tramp_table[230]; }
    if (index == 230) __bf_tramp_table[230] = dlsym(RTLD_NEXT, "wmemcmp");
    if (index == 230) return __bf_tramp_table[230];
    if (index == 231) { __bf_tramp_table[231] = (void*)(unsigned long long)__bf_tramp_table[231]; }
    if (index == 231) __bf_tramp_table[231] = dlsym(RTLD_NEXT, "exp");
    if (index == 231) return __bf_tramp_table[231];
    if (index == 232) { __bf_tramp_table[232] = (void*)(unsigned long long)__bf_tramp_table[232]; }
    if (index == 232) __bf_tramp_table[232] = dlsym(RTLD_NEXT, "pow");
    if (index == 232) return __bf_tramp_table[232];
    if (index == 233) { __bf_tramp_table[233] = (void*)(unsigned long long)__bf_tramp_table[233]; }
    if (index == 233) __bf_tramp_table[233] = dlsym(RTLD_NEXT, "fmod");
    if (index == 233) return __bf_tramp_table[233];
    if (index == 234) { __bf_tramp_table[234] = (void*)(unsigned long long)__bf_tramp_table[234]; }
    if (index == 234) __bf_tramp_table[234] = dlsym(RTLD_NEXT, "log");
    if (index == 234) return __bf_tramp_table[234];
    if (index == 235) { __bf_tramp_table[235] = (void*)(unsigned long long)__bf_tramp_table[235]; }
    if (index == 235) __bf_tramp_table[235] = dlsym(RTLD_NEXT, "log2");
    if (index == 235) return __bf_tramp_table[235];
    if (index == 236) { __bf_tramp_table[236] = (void*)(unsigned long long)__bf_tramp_table[236]; }
    if (index == 236) __bf_tramp_table[236] = dlsym(RTLD_NEXT, "log10");
    if (index == 236) return __bf_tramp_table[236];
    if (index == 237) { __bf_tramp_table[237] = (void*)(unsigned long long)__bf_tramp_table[237]; }
    if (index == 237) __bf_tramp_table[237] = dlsym(RTLD_NEXT, "round");
    if (index == 237) return __bf_tramp_table[237];
    if (index == 238) { __bf_tramp_table[238] = (void*)(unsigned long long)__bf_tramp_table[238]; }
    if (index == 238) __bf_tramp_table[238] = dlsym(RTLD_NEXT, "frexp");
    if (index == 238) return __bf_tramp_table[238];
    if (index == 239) { __bf_tramp_table[239] = (void*)(unsigned long long)__bf_tramp_table[239]; }
    if (index == 239) __bf_tramp_table[239] = dlsym(RTLD_NEXT, "sin");
    if (index == 239) return __bf_tramp_table[239];
    if (index == 240) { __bf_tramp_table[240] = (void*)(unsigned long long)__bf_tramp_table[240]; }
    if (index == 240) __bf_tramp_table[240] = dlsym(RTLD_NEXT, "sinh");
    if (index == 240) return __bf_tramp_table[240];
    if (index == 241) { __bf_tramp_table[241] = (void*)(unsigned long long)__bf_tramp_table[241]; }
    if (index == 241) __bf_tramp_table[241] = dlsym(RTLD_NEXT, "cos");
    if (index == 241) return __bf_tramp_table[241];
    if (index == 242) { __bf_tramp_table[242] = (void*)(unsigned long long)__bf_tramp_table[242]; }
    if (index == 242) __bf_tramp_table[242] = dlsym(RTLD_NEXT, "cosh");
    if (index == 242) return __bf_tramp_table[242];
    if (index == 243) { __bf_tramp_table[243] = (void*)(unsigned long long)__bf_tramp_table[243]; }
    if (index == 243) __bf_tramp_table[243] = dlsym(RTLD_NEXT, "tan");
    if (index == 243) return __bf_tramp_table[243];
    if (index == 244) { __bf_tramp_table[244] = (void*)(unsigned long long)__bf_tramp_table[244]; }
    if (index == 244) __bf_tramp_table[244] = dlsym(RTLD_NEXT, "tanh");
    if (index == 244) return __bf_tramp_table[244];
    if (index == 245) { __bf_tramp_table[245] = (void*)(unsigned long long)__bf_tramp_table[245]; }
    if (index == 245) __bf_tramp_table[245] = dlsym(RTLD_NEXT, "atol");
    if (index == 245) return __bf_tramp_table[245];
    if (index == 246) { __bf_tramp_table[246] = (void*)(unsigned long long)__bf_tramp_table[246]; }
    if (index == 246) __bf_tramp_table[246] = dlsym(RTLD_NEXT, "atof");
    if (index == 246) return __bf_tramp_table[246];
    if (index == 247) { __bf_tramp_table[247] = (void*)(unsigned long long)__bf_tramp_table[247]; }
    if (index == 247) __bf_tramp_table[247] = dlsym(RTLD_NEXT, "strspn");
    if (index == 247) return __bf_tramp_table[247];
    if (index == 248) { __bf_tramp_table[248] = (void*)(unsigned long long)__bf_tramp_table[248]; }
    if (index == 248) __bf_tramp_table[248] = dlsym(RTLD_NEXT, "strtof");
    if (index == 248) return __bf_tramp_table[248];
    if (index == 249) { __bf_tramp_table[249] = (void*)(unsigned long long)__bf_tramp_table[249]; }
    if (index == 249) __bf_tramp_table[249] = dlsym(RTLD_NEXT, "ioctl");
    if (index == 249) return __bf_tramp_table[249];
    if (index == 250) { __bf_tramp_table[250] = (void*)(unsigned long long)__bf_tramp_table[250]; }
    if (index == 250) __bf_tramp_table[250] = dlsym(RTLD_NEXT, "getpeername");
    if (index == 250) return __bf_tramp_table[250];
    if (index == 251) { __bf_tramp_table[251] = (void*)(unsigned long long)__bf_tramp_table[251]; }
    if (index == 251) __bf_tramp_table[251] = dlsym(RTLD_NEXT, "listen");
    if (index == 251) return __bf_tramp_table[251];
    if (index == 252) { __bf_tramp_table[252] = (void*)(unsigned long long)__bf_tramp_table[252]; }
    if (index == 252) __bf_tramp_table[252] = dlsym(RTLD_NEXT, "accept");
    if (index == 252) return __bf_tramp_table[252];
    if (index == 253) { __bf_tramp_table[253] = (void*)(unsigned long long)__bf_tramp_table[253]; }
    if (index == 253) __bf_tramp_table[253] = dlsym(RTLD_NEXT, "epoll_create");
    if (index == 253) return __bf_tramp_table[253];
    if (index == 254) { __bf_tramp_table[254] = (void*)(unsigned long long)__bf_tramp_table[254]; }
    if (index == 254) __bf_tramp_table[254] = dlsym(RTLD_NEXT, "FD_CLR");
    if (index == 254) return __bf_tramp_table[254];
    if (index == 255) { __bf_tramp_table[255] = (void*)(unsigned long long)__bf_tramp_table[255]; }
    if (index == 255) __bf_tramp_table[255] = dlsym(RTLD_NEXT, "expm1");
    if (index == 255) return __bf_tramp_table[255];
    if (index == 256) { __bf_tramp_table[256] = (void*)(unsigned long long)__bf_tramp_table[256]; }
    if (index == 256) __bf_tramp_table[256] = dlsym(RTLD_NEXT, "if_indextoname");
    if (index == 256) return __bf_tramp_table[256];
    if (index == 257) { __bf_tramp_table[257] = (void*)(unsigned long long)__bf_tramp_table[257]; }
    if (index == 257) __bf_tramp_table[257] = dlsym(RTLD_NEXT, "sigaddset");
    if (index == 257) return __bf_tramp_table[257];
    if (index == 258) { __bf_tramp_table[258] = (void*)(unsigned long long)__bf_tramp_table[258]; }
    if (index == 258) __bf_tramp_table[258] = dlsym(RTLD_NEXT, "pthread_sigmask");
    if (index == 258) return __bf_tramp_table[258];
    if (index == 259) { __bf_tramp_table[259] = (void*)(unsigned long long)__bf_tramp_table[259]; }
    if (index == 259) __bf_tramp_table[259] = dlsym(RTLD_NEXT, "fgets");
    if (index == 259) return __bf_tramp_table[259];
    if (index == 260) { __bf_tramp_table[260] = (void*)(unsigned long long)__bf_tramp_table[260]; }
    if (index == 260) __bf_tramp_table[260] = dlsym(RTLD_NEXT, "setjmp");
    if (index == 260) return __bf_tramp_table[260];
    if (index == 261) { __bf_tramp_table[261] = (void*)(unsigned long long)__bf_tramp_table[261]; }
    if (index == 261) __bf_tramp_table[261] = dlsym(RTLD_NEXT, "longjmp");
    if (index == 261) return __bf_tramp_table[261];
    if (index == 262) { __bf_tramp_table[262] = (void*)(unsigned long long)__bf_tramp_table[262]; }
    if (index == 262) __bf_tramp_table[262] = dlsym(RTLD_NEXT, "pthread_condattr_init");
    if (index == 262) return __bf_tramp_table[262];
    if (index == 263) { __bf_tramp_table[263] = (void*)(unsigned long long)__bf_tramp_table[263]; }
    if (index == 263) __bf_tramp_table[263] = dlsym(RTLD_NEXT, "pthread_condattr_setclock");
    if (index == 263) return __bf_tramp_table[263];
    if (index == 264) { __bf_tramp_table[264] = (void*)(unsigned long long)__bf_tramp_table[264]; }
    if (index == 264) __bf_tramp_table[264] = dlsym(RTLD_NEXT, "pthread_condattr_destroy");
    if (index == 264) return __bf_tramp_table[264];
    if (index == 265) { __bf_tramp_table[265] = (void*)(unsigned long long)__bf_tramp_table[265]; }
    if (index == 265) __bf_tramp_table[265] = dlsym(RTLD_NEXT, "sched_get_priority_min");
    if (index == 265) return __bf_tramp_table[265];
    if (index == 266) { __bf_tramp_table[266] = (void*)(unsigned long long)__bf_tramp_table[266]; }
    if (index == 266) __bf_tramp_table[266] = dlsym(RTLD_NEXT, "pthread_setschedparam");
    if (index == 266) return __bf_tramp_table[266];
    if (index == 267) { __bf_tramp_table[267] = (void*)(unsigned long long)__bf_tramp_table[267]; }
    if (index == 267) __bf_tramp_table[267] = dlsym(RTLD_NEXT, "getgid");
    if (index == 267) return __bf_tramp_table[267];
    if (index == 268) { __bf_tramp_table[268] = (void*)(unsigned long long)__bf_tramp_table[268]; }
    if (index == 268) __bf_tramp_table[268] = dlsym(RTLD_NEXT, "getegid");
    if (index == 268) return __bf_tramp_table[268];
    if (index == 269) { __bf_tramp_table[269] = (void*)(unsigned long long)__bf_tramp_table[269]; }
    if (index == 269) __bf_tramp_table[269] = dlsym(RTLD_NEXT, "random");
    if (index == 269) return __bf_tramp_table[269];
    if (index == 270) { __bf_tramp_table[270] = (void*)(unsigned long long)__bf_tramp_table[270]; }
    if (index == 270) __bf_tramp_table[270] = dlsym(RTLD_NEXT, "sigfillset");
    if (index == 270) return __bf_tramp_table[270];
    if (index == 271) { __bf_tramp_table[271] = (void*)(unsigned long long)__bf_tramp_table[271]; }
    if (index == 271) __bf_tramp_table[271] = dlsym(RTLD_NEXT, "fdopen");
    if (index == 271) return __bf_tramp_table[271];
    if (index == 272) { __bf_tramp_table[272] = (void*)(unsigned long long)__bf_tramp_table[272]; }
    if (index == 272) __bf_tramp_table[272] = dlsym(RTLD_NEXT, "timerfd_create");
    if (index == 272) return __bf_tramp_table[272];
    if (index == 273) { __bf_tramp_table[273] = (void*)(unsigned long long)__bf_tramp_table[273]; }
    if (index == 273) __bf_tramp_table[273] = dlsym(RTLD_NEXT, "timerfd_settime");
    if (index == 273) return __bf_tramp_table[273];
    if (index == 274) { __bf_tramp_table[274] = (void*)(unsigned long long)__bf_tramp_table[274]; }
    if (index == 274) __bf_tramp_table[274] = dlsym(RTLD_NEXT, "fputc");
    if (index == 274) return __bf_tramp_table[274];
    if (index == 275) { __bf_tramp_table[275] = (void*)(unsigned long long)__bf_tramp_table[275]; }
    if (index == 275) __bf_tramp_table[275] = dlsym(RTLD_NEXT, "bsearch");
    if (index == 275) return __bf_tramp_table[275];
    if (index == 276) { __bf_tramp_table[276] = (void*)(unsigned long long)__bf_tramp_table[276]; }
    if (index == 276) __bf_tramp_table[276] = dlsym(RTLD_NEXT, "vfprintf");
    if (index == 276) return __bf_tramp_table[276];
    if (index == 277) { __bf_tramp_table[277] = (void*)(unsigned long long)__bf_tramp_table[277]; }
    if (index == 277) __bf_tramp_table[277] = dlsym(RTLD_NEXT, "pthread_exit");
    if (index == 277) return __bf_tramp_table[277];
    if (index == 278) { __bf_tramp_table[278] = (void*)(unsigned long long)__bf_tramp_table[278]; }
    if (index == 278) __bf_tramp_table[278] = dlsym(RTLD_NEXT, "finitef");
    if (index == 278) return __bf_tramp_table[278];
    if (index == 279) { __bf_tramp_table[279] = (void*)(unsigned long long)__bf_tramp_table[279]; }
    if (index == 279) __bf_tramp_table[279] = dlsym(RTLD_NEXT, "cbrt");
    if (index == 279) return __bf_tramp_table[279];
    if (index == 280) { __bf_tramp_table[280] = (void*)(unsigned long long)__bf_tramp_table[280]; }
    if (index == 280) __bf_tramp_table[280] = dlsym(RTLD_NEXT, "remquof");
    if (index == 280) return __bf_tramp_table[280];
    if (index == 281) { __bf_tramp_table[281] = (void*)(unsigned long long)__bf_tramp_table[281]; }
    if (index == 281) __bf_tramp_table[281] = dlsym(RTLD_NEXT, "strcspn");
    if (index == 281) return __bf_tramp_table[281];
    if (index == 282) { __bf_tramp_table[282] = (void*)(unsigned long long)__bf_tramp_table[282]; }
    if (index == 282) __bf_tramp_table[282] = dlsym(RTLD_NEXT, "gmtime_r");
    if (index == 282) return __bf_tramp_table[282];
    if (index == 283) { __bf_tramp_table[283] = (void*)(unsigned long long)__bf_tramp_table[283]; }
    if (index == 283) __bf_tramp_table[283] = dlsym(RTLD_NEXT, "difftime");
    if (index == 283) return __bf_tramp_table[283];
    if (index == 284) { __bf_tramp_table[284] = (void*)(unsigned long long)__bf_tramp_table[284]; }
    if (index == 284) __bf_tramp_table[284] = dlsym(RTLD_NEXT, "strpbrk");
    if (index == 284) return __bf_tramp_table[284];
    if (index == 285) { __bf_tramp_table[285] = (void*)(unsigned long long)__bf_tramp_table[285]; }
    if (index == 285) __bf_tramp_table[285] = dlsym(RTLD_NEXT, "shutdown");
    if (index == 285) return __bf_tramp_table[285];
    if (index == 286) { __bf_tramp_table[286] = (void*)(unsigned long long)__bf_tramp_table[286]; }
    if (index == 286) __bf_tramp_table[286] = dlsym(RTLD_NEXT, "memrchr");
    if (index == 286) return __bf_tramp_table[286];
    if (index == 287) { __bf_tramp_table[287] = (void*)(unsigned long long)__bf_tramp_table[287]; }
    if (index == 287) __bf_tramp_table[287] = dlsym(RTLD_NEXT, "accept4");
    if (index == 287) return __bf_tramp_table[287];
    if (index == 288) { __bf_tramp_table[288] = (void*)(unsigned long long)__bf_tramp_table[288]; }
    if (index == 288) __bf_tramp_table[288] = dlsym(RTLD_NEXT, "if_nametoindex");
    if (index == 288) return __bf_tramp_table[288];
    if (index == 289) { __bf_tramp_table[289] = (void*)(unsigned long long)__bf_tramp_table[289]; }
    if (index == 289) __bf_tramp_table[289] = dlsym(RTLD_NEXT, "setvbuf");
    if (index == 289) return __bf_tramp_table[289];
    if (index == 290) { __bf_tramp_table[290] = (void*)(unsigned long long)__bf_tramp_table[290]; }
    if (index == 290) __bf_tramp_table[290] = dlsym(RTLD_NEXT, "realpath");
    if (index == 290) return __bf_tramp_table[290];
    if (index == 291) { __bf_tramp_table[291] = (void*)(unsigned long long)__bf_tramp_table[291]; }
    if (index == 291) __bf_tramp_table[291] = dlsym(RTLD_NEXT, "recvmmsg");
    if (index == 291) return __bf_tramp_table[291];
    if (index == 292) { __bf_tramp_table[292] = (void*)(unsigned long long)__bf_tramp_table[292]; }
    if (index == 292) __bf_tramp_table[292] = dlsym(RTLD_NEXT, "getcwd");
    if (index == 292) return __bf_tramp_table[292];
    if (index == 293) { __bf_tramp_table[293] = (void*)(unsigned long long)__bf_tramp_table[293]; }
    if (index == 293) __bf_tramp_table[293] = dlsym(RTLD_NEXT, "pread");
    if (index == 293) return __bf_tramp_table[293];
    if (index == 294) { __bf_tramp_table[294] = (void*)(unsigned long long)__bf_tramp_table[294]; }
    if (index == 294) __bf_tramp_table[294] = dlsym(RTLD_NEXT, "pwrite");
    if (index == 294) return __bf_tramp_table[294];
    if (index == 295) { __bf_tramp_table[295] = (void*)(unsigned long long)__bf_tramp_table[295]; }
    if (index == 295) __bf_tramp_table[295] = dlsym(RTLD_NEXT, "fchmod");
    if (index == 295) return __bf_tramp_table[295];
    if (index == 296) { __bf_tramp_table[296] = (void*)(unsigned long long)__bf_tramp_table[296]; }
    if (index == 296) __bf_tramp_table[296] = dlsym(RTLD_NEXT, "fchown");
    if (index == 296) return __bf_tramp_table[296];
    if (index == 297) { __bf_tramp_table[297] = (void*)(unsigned long long)__bf_tramp_table[297]; }
    if (index == 297) __bf_tramp_table[297] = dlsym(RTLD_NEXT, "mremap");
    if (index == 297) return __bf_tramp_table[297];
    if (index == 298) { __bf_tramp_table[298] = (void*)(unsigned long long)__bf_tramp_table[298]; }
    if (index == 298) __bf_tramp_table[298] = dlsym(RTLD_NEXT, "fsync");
    if (index == 298) return __bf_tramp_table[298];
    if (index == 299) { __bf_tramp_table[299] = (void*)(unsigned long long)__bf_tramp_table[299]; }
    if (index == 299) __bf_tramp_table[299] = dlsym(RTLD_NEXT, "utimes");
    if (index == 299) return __bf_tramp_table[299];
    if (index == 300) { __bf_tramp_table[300] = (void*)(unsigned long long)__bf_tramp_table[300]; }
    if (index == 300) __bf_tramp_table[300] = dlsym(RTLD_NEXT, "msync");
    if (index == 300) return __bf_tramp_table[300];
    if (index == 301) { __bf_tramp_table[301] = (void*)(unsigned long long)__bf_tramp_table[301]; }
    if (index == 301) __bf_tramp_table[301] = dlsym(RTLD_NEXT, "statvfs");
    if (index == 301) return __bf_tramp_table[301];
    if (index == 302) { __bf_tramp_table[302] = (void*)(unsigned long long)__bf_tramp_table[302]; }
    if (index == 302) __bf_tramp_table[302] = dlsym(RTLD_NEXT, "mallinfo");
    if (index == 302) return __bf_tramp_table[302];
    if (index == 303) { __bf_tramp_table[303] = (void*)(unsigned long long)__bf_tramp_table[303]; }
    if (index == 303) __bf_tramp_table[303] = dlsym(RTLD_NEXT, "__readlink_chk");
    if (index == 303) return __bf_tramp_table[303];
    if (index == 304) { __bf_tramp_table[304] = (void*)(unsigned long long)__bf_tramp_table[304]; }
    if (index == 304) __bf_tramp_table[304] = dlsym(RTLD_NEXT, "__gnu_strerror_r");
    if (index == 304) return __bf_tramp_table[304];
    if (index == 305) { __bf_tramp_table[305] = (void*)(unsigned long long)__bf_tramp_table[305]; }
    if (index == 305) __bf_tramp_table[305] = dlsym(RTLD_NEXT, "pthread_getschedparam");
    if (index == 305) return __bf_tramp_table[305];
    if (index == 306) { __bf_tramp_table[306] = (void*)(unsigned long long)__bf_tramp_table[306]; }
    if (index == 306) __bf_tramp_table[306] = dlsym(RTLD_NEXT, "sinf");
    if (index == 306) return __bf_tramp_table[306];
    if (index == 307) { __bf_tramp_table[307] = (void*)(unsigned long long)__bf_tramp_table[307]; }
    if (index == 307) __bf_tramp_table[307] = dlsym(RTLD_NEXT, "sincosf");
    if (index == 307) return __bf_tramp_table[307];
    if (index == 308) { __bf_tramp_table[308] = (void*)(unsigned long long)__bf_tramp_table[308]; }
    if (index == 308) __bf_tramp_table[308] = dlsym(RTLD_NEXT, "exp2");
    if (index == 308) return __bf_tramp_table[308];
    if (index == 309) { __bf_tramp_table[309] = (void*)(unsigned long long)__bf_tramp_table[309]; }
    if (index == 309) __bf_tramp_table[309] = dlsym(RTLD_NEXT, "sincos");
    if (index == 309) return __bf_tramp_table[309];
    if (index == 310) { __bf_tramp_table[310] = (void*)(unsigned long long)__bf_tramp_table[310]; }
    if (index == 310) __bf_tramp_table[310] = dlsym(RTLD_NEXT, "fmal");
    if (index == 310) return __bf_tramp_table[310];
    if (index == 311) { __bf_tramp_table[311] = (void*)(unsigned long long)__bf_tramp_table[311]; }
    if (index == 311) __bf_tramp_table[311] = dlsym(RTLD_NEXT, "exp2f");
    if (index == 311) return __bf_tramp_table[311];
    if (index == 312) { __bf_tramp_table[312] = (void*)(unsigned long long)__bf_tramp_table[312]; }
    if (index == 312) __bf_tramp_table[312] = dlsym(RTLD_NEXT, "log10f");
    if (index == 312) return __bf_tramp_table[312];
    if (index == 313) { __bf_tramp_table[313] = (void*)(unsigned long long)__bf_tramp_table[313]; }
    if (index == 313) __bf_tramp_table[313] = dlsym(RTLD_NEXT, "logf");
    if (index == 313) return __bf_tramp_table[313];
    if (index == 314) { __bf_tramp_table[314] = (void*)(unsigned long long)__bf_tramp_table[314]; }
    if (index == 314) __bf_tramp_table[314] = dlsym(RTLD_NEXT, "powf");
    if (index == 314) return __bf_tramp_table[314];
    if (index == 315) { __bf_tramp_table[315] = (void*)(unsigned long long)__bf_tramp_table[315]; }
    if (index == 315) __bf_tramp_table[315] = dlsym(RTLD_NEXT, "fmodf");
    if (index == 315) return __bf_tramp_table[315];
    if (index == 316) { __bf_tramp_table[316] = (void*)(unsigned long long)__bf_tramp_table[316]; }
    if (index == 316) __bf_tramp_table[316] = dlsym(RTLD_NEXT, "log2f");
    if (index == 316) return __bf_tramp_table[316];
    if (index == 317) { __bf_tramp_table[317] = (void*)(unsigned long long)__bf_tramp_table[317]; }
    if (index == 317) __bf_tramp_table[317] = dlsym(RTLD_NEXT, "expf");
    if (index == 317) return __bf_tramp_table[317];
    if (index == 318) { __bf_tramp_table[318] = (void*)(unsigned long long)__bf_tramp_table[318]; }
    if (index == 318) __bf_tramp_table[318] = dlsym(RTLD_NEXT, "powl");
    if (index == 318) return __bf_tramp_table[318];
    if (index == 319) { __bf_tramp_table[319] = (void*)(unsigned long long)__bf_tramp_table[319]; }
    if (index == 319) __bf_tramp_table[319] = dlsym(RTLD_NEXT, "cosf");
    if (index == 319) return __bf_tramp_table[319];
    if (index == 320) { __bf_tramp_table[320] = (void*)(unsigned long long)__bf_tramp_table[320]; }
    if (index == 320) __bf_tramp_table[320] = dlsym(RTLD_NEXT, "pthread_key_delete");
    if (index == 320) return __bf_tramp_table[320];
    if (index == 321) { __bf_tramp_table[321] = (void*)(unsigned long long)__bf_tramp_table[321]; }
    if (index == 321) __bf_tramp_table[321] = dlsym(RTLD_NEXT, "sysinfo");
    if (index == 321) return __bf_tramp_table[321];
    if (index == 322) { __bf_tramp_table[322] = (void*)(unsigned long long)__bf_tramp_table[322]; }
    if (index == 322) __bf_tramp_table[322] = dlsym(RTLD_NEXT, "madvise");
    if (index == 322) return __bf_tramp_table[322];
    if (index == 323) { __bf_tramp_table[323] = (void*)(unsigned long long)__bf_tramp_table[323]; }
    if (index == 323) __bf_tramp_table[323] = dlsym(RTLD_NEXT, "pthread_setname_np");
    if (index == 323) return __bf_tramp_table[323];
    if (index == 324) { __bf_tramp_table[324] = (void*)(unsigned long long)__bf_tramp_table[324]; }
    if (index == 324) __bf_tramp_table[324] = dlsym(RTLD_NEXT, "pthread_getattr_np");
    if (index == 324) return __bf_tramp_table[324];
    if (index == 325) { __bf_tramp_table[325] = (void*)(unsigned long long)__bf_tramp_table[325]; }
    if (index == 325) __bf_tramp_table[325] = dlsym(RTLD_NEXT, "pthread_attr_getstack");
    if (index == 325) return __bf_tramp_table[325];
    if (index == 326) { __bf_tramp_table[326] = (void*)(unsigned long long)__bf_tramp_table[326]; }
    if (index == 326) __bf_tramp_table[326] = dlsym(RTLD_NEXT, "mlock");
    if (index == 326) return __bf_tramp_table[326];
    if (index == 327) { __bf_tramp_table[327] = (void*)(unsigned long long)__bf_tramp_table[327]; }
    if (index == 327) __bf_tramp_table[327] = dlsym(RTLD_NEXT, "dl_iterate_phdr");
    if (index == 327) return __bf_tramp_table[327];
    if (index == 328) { __bf_tramp_table[328] = (void*)(unsigned long long)__bf_tramp_table[328]; }
    if (index == 328) __bf_tramp_table[328] = dlsym(RTLD_NEXT, "isspace");
    if (index == 328) return __bf_tramp_table[328];
    if (index == 329) { __bf_tramp_table[329] = (void*)(unsigned long long)__bf_tramp_table[329]; }
    if (index == 329) __bf_tramp_table[329] = dlsym(RTLD_NEXT, "gethostbyname");
    if (index == 329) return __bf_tramp_table[329];
    if (index == 330) { __bf_tramp_table[330] = (void*)(unsigned long long)__bf_tramp_table[330]; }
    if (index == 330) __bf_tramp_table[330] = dlsym(RTLD_NEXT, "strcat");
    if (index == 330) return __bf_tramp_table[330];
    if (index == 331) { __bf_tramp_table[331] = (void*)(unsigned long long)__bf_tramp_table[331]; }
    if (index == 331) __bf_tramp_table[331] = dlsym(RTLD_NEXT, "sendmmsg");
    if (index == 331) return __bf_tramp_table[331];
    if (index == 332) { __bf_tramp_table[332] = (void*)(unsigned long long)__bf_tramp_table[332]; }
    if (index == 332) __bf_tramp_table[332] = dlsym(RTLD_NEXT, "tolower");
    if (index == 332) return __bf_tramp_table[332];
    if (index == 333) { __bf_tramp_table[333] = (void*)(unsigned long long)__bf_tramp_table[333]; }
    if (index == 333) __bf_tramp_table[333] = dlsym(RTLD_NEXT, "pthread_rwlock_destroy");
    if (index == 333) return __bf_tramp_table[333];
    if (index == 334) { __bf_tramp_table[334] = (void*)(unsigned long long)__bf_tramp_table[334]; }
    if (index == 334) __bf_tramp_table[334] = dlsym(RTLD_NEXT, "pthread_rwlock_init");
    if (index == 334) return __bf_tramp_table[334];
    if (index == 335) { __bf_tramp_table[335] = (void*)(unsigned long long)__bf_tramp_table[335]; }
    if (index == 335) __bf_tramp_table[335] = dlsym(RTLD_NEXT, "pthread_rwlock_rdlock");
    if (index == 335) return __bf_tramp_table[335];
    if (index == 336) { __bf_tramp_table[336] = (void*)(unsigned long long)__bf_tramp_table[336]; }
    if (index == 336) __bf_tramp_table[336] = dlsym(RTLD_NEXT, "pthread_rwlock_unlock");
    if (index == 336) return __bf_tramp_table[336];
    if (index == 337) { __bf_tramp_table[337] = (void*)(unsigned long long)__bf_tramp_table[337]; }
    if (index == 337) __bf_tramp_table[337] = dlsym(RTLD_NEXT, "pthread_rwlock_wrlock");
    if (index == 337) return __bf_tramp_table[337];
    if (index == 338) { __bf_tramp_table[338] = (void*)(unsigned long long)__bf_tramp_table[338]; }
    if (index == 338) __bf_tramp_table[338] = dlsym(RTLD_NEXT, "signal");
    if (index == 338) return __bf_tramp_table[338];
    if (index == 339) { __bf_tramp_table[339] = (void*)(unsigned long long)__bf_tramp_table[339]; }
    if (index == 339) __bf_tramp_table[339] = dlsym(RTLD_NEXT, "tcgetattr");
    if (index == 339) return __bf_tramp_table[339];
    if (index == 340) { __bf_tramp_table[340] = (void*)(unsigned long long)__bf_tramp_table[340]; }
    if (index == 340) __bf_tramp_table[340] = dlsym(RTLD_NEXT, "tcsetattr");
    if (index == 340) return __bf_tramp_table[340];
    if (index == 341) { __bf_tramp_table[341] = (void*)(unsigned long long)__bf_tramp_table[341]; }
    if (index == 341) __bf_tramp_table[341] = dlsym(RTLD_NEXT, "utime");
    if (index == 341) return __bf_tramp_table[341];
    if (index == 342) { __bf_tramp_table[342] = (void*)(unsigned long long)__bf_tramp_table[342]; }
    if (index == 342) __bf_tramp_table[342] = dlsym(RTLD_NEXT, "vasprintf");
    if (index == 342) return __bf_tramp_table[342];
    if (index == 343) { __bf_tramp_table[343] = (void*)(unsigned long long)__bf_tramp_table[343]; }
    if (index == 343) __bf_tramp_table[343] = dlsym(RTLD_NEXT, "openlog");
    if (index == 343) return __bf_tramp_table[343];
    if (index == 344) { __bf_tramp_table[344] = (void*)(unsigned long long)__bf_tramp_table[344]; }
    if (index == 344) __bf_tramp_table[344] = dlsym(RTLD_NEXT, "syslog");
    if (index == 344) return __bf_tramp_table[344];
    if (index == 345) { __bf_tramp_table[345] = (void*)(unsigned long long)__bf_tramp_table[345]; }
    if (index == 345) __bf_tramp_table[345] = dlsym(RTLD_NEXT, "closelog");
    if (index == 345) return __bf_tramp_table[345];
    if (index == 346) { __bf_tramp_table[346] = (void*)(unsigned long long)__bf_tramp_table[346]; }
    if (index == 346) __bf_tramp_table[346] = dlsym(RTLD_NEXT, "ungetc");
    if (index == 346) return __bf_tramp_table[346];
    if (index == 347) { __bf_tramp_table[347] = (void*)(unsigned long long)__bf_tramp_table[347]; }
    if (index == 347) __bf_tramp_table[347] = dlsym(RTLD_NEXT, "getc");
    if (index == 347) return __bf_tramp_table[347];
    if (index == 348) { __bf_tramp_table[348] = (void*)(unsigned long long)__bf_tramp_table[348]; }
    if (index == 348) __bf_tramp_table[348] = dlsym(RTLD_NEXT, "ungetwc");
    if (index == 348) return __bf_tramp_table[348];
    if (index == 349) { __bf_tramp_table[349] = (void*)(unsigned long long)__bf_tramp_table[349]; }
    if (index == 349) __bf_tramp_table[349] = dlsym(RTLD_NEXT, "getwc");
    if (index == 349) return __bf_tramp_table[349];
    if (index == 350) { __bf_tramp_table[350] = (void*)(unsigned long long)__bf_tramp_table[350]; }
    if (index == 350) __bf_tramp_table[350] = dlsym(RTLD_NEXT, "fputwc");
    if (index == 350) return __bf_tramp_table[350];
    if (index == 351) { __bf_tramp_table[351] = (void*)(unsigned long long)__bf_tramp_table[351]; }
    if (index == 351) __bf_tramp_table[351] = dlsym(RTLD_NEXT, "newlocale");
    if (index == 351) return __bf_tramp_table[351];
    if (index == 352) { __bf_tramp_table[352] = (void*)(unsigned long long)__bf_tramp_table[352]; }
    if (index == 352) __bf_tramp_table[352] = dlsym(RTLD_NEXT, "uselocale");
    if (index == 352) return __bf_tramp_table[352];
    if (index == 353) { __bf_tramp_table[353] = (void*)(unsigned long long)__bf_tramp_table[353]; }
    if (index == 353) __bf_tramp_table[353] = dlsym(RTLD_NEXT, "vsscanf");
    if (index == 353) return __bf_tramp_table[353];
    if (index == 354) { __bf_tramp_table[354] = (void*)(unsigned long long)__bf_tramp_table[354]; }
    if (index == 354) __bf_tramp_table[354] = dlsym(RTLD_NEXT, "strftime_l");
    if (index == 354) return __bf_tramp_table[354];
    if (index == 355) { __bf_tramp_table[355] = (void*)(unsigned long long)__bf_tramp_table[355]; }
    if (index == 355) __bf_tramp_table[355] = dlsym(RTLD_NEXT, "mbsrtowcs");
    if (index == 355) return __bf_tramp_table[355];
    if (index == 356) { __bf_tramp_table[356] = (void*)(unsigned long long)__bf_tramp_table[356]; }
    if (index == 356) __bf_tramp_table[356] = dlsym(RTLD_NEXT, "freelocale");
    if (index == 356) return __bf_tramp_table[356];
    if (index == 357) { __bf_tramp_table[357] = (void*)(unsigned long long)__bf_tramp_table[357]; }
    if (index == 357) __bf_tramp_table[357] = dlsym(RTLD_NEXT, "strcoll_l");
    if (index == 357) return __bf_tramp_table[357];
    if (index == 358) { __bf_tramp_table[358] = (void*)(unsigned long long)__bf_tramp_table[358]; }
    if (index == 358) __bf_tramp_table[358] = dlsym(RTLD_NEXT, "strxfrm_l");
    if (index == 358) return __bf_tramp_table[358];
    if (index == 359) { __bf_tramp_table[359] = (void*)(unsigned long long)__bf_tramp_table[359]; }
    if (index == 359) __bf_tramp_table[359] = dlsym(RTLD_NEXT, "wcscoll_l");
    if (index == 359) return __bf_tramp_table[359];
    if (index == 360) { __bf_tramp_table[360] = (void*)(unsigned long long)__bf_tramp_table[360]; }
    if (index == 360) __bf_tramp_table[360] = dlsym(RTLD_NEXT, "wcsxfrm_l");
    if (index == 360) return __bf_tramp_table[360];
    if (index == 361) { __bf_tramp_table[361] = (void*)(unsigned long long)__bf_tramp_table[361]; }
    if (index == 361) __bf_tramp_table[361] = dlsym(RTLD_NEXT, "iswlower_l");
    if (index == 361) return __bf_tramp_table[361];
    if (index == 362) { __bf_tramp_table[362] = (void*)(unsigned long long)__bf_tramp_table[362]; }
    if (index == 362) __bf_tramp_table[362] = dlsym(RTLD_NEXT, "iswspace_l");
    if (index == 362) return __bf_tramp_table[362];
    if (index == 363) { __bf_tramp_table[363] = (void*)(unsigned long long)__bf_tramp_table[363]; }
    if (index == 363) __bf_tramp_table[363] = dlsym(RTLD_NEXT, "iswprint_l");
    if (index == 363) return __bf_tramp_table[363];
    if (index == 364) { __bf_tramp_table[364] = (void*)(unsigned long long)__bf_tramp_table[364]; }
    if (index == 364) __bf_tramp_table[364] = dlsym(RTLD_NEXT, "iswblank_l");
    if (index == 364) return __bf_tramp_table[364];
    if (index == 365) { __bf_tramp_table[365] = (void*)(unsigned long long)__bf_tramp_table[365]; }
    if (index == 365) __bf_tramp_table[365] = dlsym(RTLD_NEXT, "iswcntrl_l");
    if (index == 365) return __bf_tramp_table[365];
    if (index == 366) { __bf_tramp_table[366] = (void*)(unsigned long long)__bf_tramp_table[366]; }
    if (index == 366) __bf_tramp_table[366] = dlsym(RTLD_NEXT, "iswupper_l");
    if (index == 366) return __bf_tramp_table[366];
    if (index == 367) { __bf_tramp_table[367] = (void*)(unsigned long long)__bf_tramp_table[367]; }
    if (index == 367) __bf_tramp_table[367] = dlsym(RTLD_NEXT, "iswalpha_l");
    if (index == 367) return __bf_tramp_table[367];
    if (index == 368) { __bf_tramp_table[368] = (void*)(unsigned long long)__bf_tramp_table[368]; }
    if (index == 368) __bf_tramp_table[368] = dlsym(RTLD_NEXT, "iswdigit_l");
    if (index == 368) return __bf_tramp_table[368];
    if (index == 369) { __bf_tramp_table[369] = (void*)(unsigned long long)__bf_tramp_table[369]; }
    if (index == 369) __bf_tramp_table[369] = dlsym(RTLD_NEXT, "iswpunct_l");
    if (index == 369) return __bf_tramp_table[369];
    if (index == 370) { __bf_tramp_table[370] = (void*)(unsigned long long)__bf_tramp_table[370]; }
    if (index == 370) __bf_tramp_table[370] = dlsym(RTLD_NEXT, "iswxdigit_l");
    if (index == 370) return __bf_tramp_table[370];
    if (index == 371) { __bf_tramp_table[371] = (void*)(unsigned long long)__bf_tramp_table[371]; }
    if (index == 371) __bf_tramp_table[371] = dlsym(RTLD_NEXT, "towupper_l");
    if (index == 371) return __bf_tramp_table[371];
    if (index == 372) { __bf_tramp_table[372] = (void*)(unsigned long long)__bf_tramp_table[372]; }
    if (index == 372) __bf_tramp_table[372] = dlsym(RTLD_NEXT, "towlower_l");
    if (index == 372) return __bf_tramp_table[372];
    if (index == 373) { __bf_tramp_table[373] = (void*)(unsigned long long)__bf_tramp_table[373]; }
    if (index == 373) __bf_tramp_table[373] = dlsym(RTLD_NEXT, "btowc");
    if (index == 373) return __bf_tramp_table[373];
    if (index == 374) { __bf_tramp_table[374] = (void*)(unsigned long long)__bf_tramp_table[374]; }
    if (index == 374) __bf_tramp_table[374] = dlsym(RTLD_NEXT, "wctob");
    if (index == 374) return __bf_tramp_table[374];
    if (index == 375) { __bf_tramp_table[375] = (void*)(unsigned long long)__bf_tramp_table[375]; }
    if (index == 375) __bf_tramp_table[375] = dlsym(RTLD_NEXT, "wcsnrtombs");
    if (index == 375) return __bf_tramp_table[375];
    if (index == 376) { __bf_tramp_table[376] = (void*)(unsigned long long)__bf_tramp_table[376]; }
    if (index == 376) __bf_tramp_table[376] = dlsym(RTLD_NEXT, "wcrtomb");
    if (index == 376) return __bf_tramp_table[376];
    if (index == 377) { __bf_tramp_table[377] = (void*)(unsigned long long)__bf_tramp_table[377]; }
    if (index == 377) __bf_tramp_table[377] = dlsym(RTLD_NEXT, "mbsnrtowcs");
    if (index == 377) return __bf_tramp_table[377];
    if (index == 378) { __bf_tramp_table[378] = (void*)(unsigned long long)__bf_tramp_table[378]; }
    if (index == 378) __bf_tramp_table[378] = dlsym(RTLD_NEXT, "mbrtowc");
    if (index == 378) return __bf_tramp_table[378];
    if (index == 379) { __bf_tramp_table[379] = (void*)(unsigned long long)__bf_tramp_table[379]; }
    if (index == 379) __bf_tramp_table[379] = dlsym(RTLD_NEXT, "mbtowc");
    if (index == 379) return __bf_tramp_table[379];
    if (index == 380) { __bf_tramp_table[380] = (void*)(unsigned long long)__bf_tramp_table[380]; }
    if (index == 380) __bf_tramp_table[380] = dlsym(RTLD_NEXT, "__ctype_get_mb_cur_max");
    if (index == 380) return __bf_tramp_table[380];
    if (index == 381) { __bf_tramp_table[381] = (void*)(unsigned long long)__bf_tramp_table[381]; }
    if (index == 381) __bf_tramp_table[381] = dlsym(RTLD_NEXT, "mbrlen");
    if (index == 381) return __bf_tramp_table[381];
    if (index == 382) { __bf_tramp_table[382] = (void*)(unsigned long long)__bf_tramp_table[382]; }
    if (index == 382) __bf_tramp_table[382] = dlsym(RTLD_NEXT, "strtoll_l");
    if (index == 382) return __bf_tramp_table[382];
    if (index == 383) { __bf_tramp_table[383] = (void*)(unsigned long long)__bf_tramp_table[383]; }
    if (index == 383) __bf_tramp_table[383] = dlsym(RTLD_NEXT, "strtoull_l");
    if (index == 383) return __bf_tramp_table[383];
    if (index == 384) { __bf_tramp_table[384] = (void*)(unsigned long long)__bf_tramp_table[384]; }
    if (index == 384) __bf_tramp_table[384] = dlsym(RTLD_NEXT, "strtold_l");
    if (index == 384) return __bf_tramp_table[384];
    if (index == 385) { __bf_tramp_table[385] = (void*)(unsigned long long)__bf_tramp_table[385]; }
    if (index == 385) __bf_tramp_table[385] = dlsym(RTLD_NEXT, "__cxa_thread_atexit_impl");
    if (index == 385) return __bf_tramp_table[385];
    if (index == 386) { __bf_tramp_table[386] = (void*)(unsigned long long)__bf_tramp_table[386]; }
    if (index == 386) __bf_tramp_table[386] = dlsym(RTLD_NEXT, "strncat");
    if (index == 386) return __bf_tramp_table[386];
    if (index == 387) { __bf_tramp_table[387] = (void*)(unsigned long long)__bf_tramp_table[387]; }
    if (index == 387) __bf_tramp_table[387] = dlsym(RTLD_NEXT, "__cfi_slowpath");
    if (index == 387) return __bf_tramp_table[387];
    if (index == 388) { __bf_tramp_table[388] = (void*)(unsigned long long)__bf_tramp_table[388]; }
    if (index == 388) __bf_tramp_table[388] = dlsym(RTLD_NEXT, "android_fdsan_close_with_tag");
    if (index == 388) return __bf_tramp_table[388];
    if (index == 389) { __bf_tramp_table[389] = (void*)(unsigned long long)__bf_tramp_table[389]; }
    if (index == 389) __bf_tramp_table[389] = dlsym(RTLD_NEXT, "android_fdsan_create_owner_tag");
    if (index == 389) return __bf_tramp_table[389];
    if (index == 390) { __bf_tramp_table[390] = (void*)(unsigned long long)__bf_tramp_table[390]; }
    if (index == 390) __bf_tramp_table[390] = dlsym(RTLD_NEXT, "android_fdsan_exchange_owner_tag");
    if (index == 390) return __bf_tramp_table[390];
    if (index == 391) { __bf_tramp_table[391] = (void*)(unsigned long long)__bf_tramp_table[391]; }
    if (index == 391) __bf_tramp_table[391] = dlsym(RTLD_NEXT, "free");
    if (index == 391) return __bf_tramp_table[391];
    if (index == 392) { __bf_tramp_table[392] = (void*)(unsigned long long)__bf_tramp_table[392]; }
    if (index == 392) __bf_tramp_table[392] = dlsym(RTLD_NEXT, "malloc");
    if (index == 392) return __bf_tramp_table[392];
    if (index == 393) { __bf_tramp_table[393] = (void*)(unsigned long long)__bf_tramp_table[393]; }
    if (index == 393) __bf_tramp_table[393] = dlsym(RTLD_NEXT, "dup");
    if (index == 393) return __bf_tramp_table[393];
    if (index == 394) { __bf_tramp_table[394] = (void*)(unsigned long long)__bf_tramp_table[394]; }
    if (index == 394) __bf_tramp_table[394] = dlsym(RTLD_NEXT, "android_mallopt");
    if (index == 394) return __bf_tramp_table[394];
    if (index == 395) { __bf_tramp_table[395] = (void*)(unsigned long long)__bf_tramp_table[395]; }
    if (index == 395) __bf_tramp_table[395] = dlsym(RTLD_NEXT, "__system_property_find");
    if (index == 395) return __bf_tramp_table[395];
    if (index == 396) { __bf_tramp_table[396] = (void*)(unsigned long long)__bf_tramp_table[396]; }
    if (index == 396) __bf_tramp_table[396] = dlsym(RTLD_NEXT, "__system_property_read_callback");
    if (index == 396) return __bf_tramp_table[396];
    if (index == 397) { __bf_tramp_table[397] = (void*)(unsigned long long)__bf_tramp_table[397]; }
    if (index == 397) __bf_tramp_table[397] = dlsym(RTLD_NEXT, "getprogname");
    if (index == 397) return __bf_tramp_table[397];
    if (index == 398) { __bf_tramp_table[398] = (void*)(unsigned long long)__bf_tramp_table[398]; }
    if (index == 398) __bf_tramp_table[398] = dlsym(RTLD_NEXT, "sleep");
    if (index == 398) return __bf_tramp_table[398];
    if (index == 399) { __bf_tramp_table[399] = (void*)(unsigned long long)__bf_tramp_table[399]; }
    if (index == 399) __bf_tramp_table[399] = dlsym(RTLD_NEXT, "android_dlopen_ext");
    if (index == 399) return __bf_tramp_table[399];
    if (index == 400) { __bf_tramp_table[400] = (void*)(unsigned long long)__bf_tramp_table[400]; }
    if (index == 400) __bf_tramp_table[400] = dlsym(RTLD_NEXT, "nrand48");
    if (index == 400) return __bf_tramp_table[400];
    if (index == 401) { __bf_tramp_table[401] = (void*)(unsigned long long)__bf_tramp_table[401]; }
    if (index == 401) __bf_tramp_table[401] = dlsym(RTLD_NEXT, "remove");
    if (index == 401) return __bf_tramp_table[401];
    if (index == 402) { __bf_tramp_table[402] = (void*)(unsigned long long)__bf_tramp_table[402]; }
    if (index == 402) __bf_tramp_table[402] = dlsym(RTLD_NEXT, "utimensat");
    if (index == 402) return __bf_tramp_table[402];
    if (index == 403) { __bf_tramp_table[403] = (void*)(unsigned long long)__bf_tramp_table[403]; }
    if (index == 403) __bf_tramp_table[403] = dlsym(RTLD_NEXT, "futimens");
    if (index == 403) return __bf_tramp_table[403];
    if (index == 404) { __bf_tramp_table[404] = (void*)(unsigned long long)__bf_tramp_table[404]; }
    if (index == 404) __bf_tramp_table[404] = dlsym(RTLD_NEXT, "getpwuid");
    if (index == 404) return __bf_tramp_table[404];
    if (index == 405) { __bf_tramp_table[405] = (void*)(unsigned long long)__bf_tramp_table[405]; }
    if (index == 405) __bf_tramp_table[405] = dlsym(RTLD_NEXT, "getgrgid");
    if (index == 405) return __bf_tramp_table[405];
    if (index == 406) { __bf_tramp_table[406] = (void*)(unsigned long long)__bf_tramp_table[406]; }
    if (index == 406) __bf_tramp_table[406] = dlsym(RTLD_NEXT, "aligned_alloc");
    if (index == 406) return __bf_tramp_table[406];
    if (index == 407) { __bf_tramp_table[407] = (void*)(unsigned long long)__bf_tramp_table[407]; }
    if (index == 407) __bf_tramp_table[407] = dlsym(RTLD_NEXT, "_Unwind_RaiseException");
    if (index == 407) return __bf_tramp_table[407];
    if (index == 408) { __bf_tramp_table[408] = (void*)(unsigned long long)__bf_tramp_table[408]; }
    if (index == 408) __bf_tramp_table[408] = dlsym(RTLD_NEXT, "_Unwind_Resume");
    if (index == 408) return __bf_tramp_table[408];
    if (index == 409) { __bf_tramp_table[409] = (void*)(unsigned long long)__bf_tramp_table[409]; }
    if (index == 409) __bf_tramp_table[409] = dlsym(RTLD_NEXT, "_Unwind_SetGR");
    if (index == 409) return __bf_tramp_table[409];
    if (index == 410) { __bf_tramp_table[410] = (void*)(unsigned long long)__bf_tramp_table[410]; }
    if (index == 410) __bf_tramp_table[410] = dlsym(RTLD_NEXT, "_Unwind_SetIP");
    if (index == 410) return __bf_tramp_table[410];
    if (index == 411) { __bf_tramp_table[411] = (void*)(unsigned long long)__bf_tramp_table[411]; }
    if (index == 411) __bf_tramp_table[411] = dlsym(RTLD_NEXT, "_Unwind_GetLanguageSpecificData");
    if (index == 411) return __bf_tramp_table[411];
    if (index == 412) { __bf_tramp_table[412] = (void*)(unsigned long long)__bf_tramp_table[412]; }
    if (index == 412) __bf_tramp_table[412] = dlsym(RTLD_NEXT, "_Unwind_GetIP");
    if (index == 412) return __bf_tramp_table[412];
    if (index == 413) { __bf_tramp_table[413] = (void*)(unsigned long long)__bf_tramp_table[413]; }
    if (index == 413) __bf_tramp_table[413] = dlsym(RTLD_NEXT, "_Unwind_GetRegionStart");
    if (index == 413) return __bf_tramp_table[413];
    if (index == 414) { __bf_tramp_table[414] = (void*)(unsigned long long)__bf_tramp_table[414]; }
    if (index == 414) __bf_tramp_table[414] = dlsym(RTLD_NEXT, "realloc");
    if (index == 414) return __bf_tramp_table[414];
    if (index == 415) { __bf_tramp_table[415] = (void*)(unsigned long long)__bf_tramp_table[415]; }
    if (index == 415) __bf_tramp_table[415] = dlsym(RTLD_NEXT, "posix_memalign");
    if (index == 415) return __bf_tramp_table[415];
    if (index == 416) { __bf_tramp_table[416] = (void*)(unsigned long long)__bf_tramp_table[416]; }
    if (index == 416) __bf_tramp_table[416] = dlsym(RTLD_NEXT, "pthread_cond_clockwait");
    if (index == 416) return __bf_tramp_table[416];
    if (index == 417) { __bf_tramp_table[417] = (void*)(unsigned long long)__bf_tramp_table[417]; }
    if (index == 417) __bf_tramp_table[417] = dlsym(RTLD_NEXT, "asinhf");
    if (index == 417) return __bf_tramp_table[417];
    if (index == 418) { __bf_tramp_table[418] = (void*)(unsigned long long)__bf_tramp_table[418]; }
    if (index == 418) __bf_tramp_table[418] = dlsym(RTLD_NEXT, "acoshf");
    if (index == 418) return __bf_tramp_table[418];
    if (index == 419) { __bf_tramp_table[419] = (void*)(unsigned long long)__bf_tramp_table[419]; }
    if (index == 419) __bf_tramp_table[419] = dlsym(RTLD_NEXT, "atanhf");
    if (index == 419) return __bf_tramp_table[419];
    if (index == 420) { __bf_tramp_table[420] = (void*)(unsigned long long)__bf_tramp_table[420]; }
    if (index == 420) __bf_tramp_table[420] = dlsym(RTLD_NEXT, "calloc");
    if (index == 420) return __bf_tramp_table[420];
    if (index == 421) { __bf_tramp_table[421] = (void*)(unsigned long long)__bf_tramp_table[421]; }
    if (index == 421) __bf_tramp_table[421] = dlsym(RTLD_NEXT, "memalign");
    if (index == 421) return __bf_tramp_table[421];
    if (index == 422) { __bf_tramp_table[422] = (void*)(unsigned long long)__bf_tramp_table[422]; }
    if (index == 422) __bf_tramp_table[422] = dlsym(RTLD_NEXT, "unsetenv");
    if (index == 422) return __bf_tramp_table[422];
    if (index == 423) { __bf_tramp_table[423] = (void*)(unsigned long long)__bf_tramp_table[423]; }
    if (index == 423) __bf_tramp_table[423] = dlsym(RTLD_NEXT, "setenv");
    if (index == 423) return __bf_tramp_table[423];
    if (index == 424) { __bf_tramp_table[424] = (void*)(unsigned long long)__bf_tramp_table[424]; }
    if (index == 424) __bf_tramp_table[424] = dlsym(RTLD_NEXT, "_Unwind_DeleteException");
    if (index == 424) return __bf_tramp_table[424];
    if (index == 425) { __bf_tramp_table[425] = (void*)(unsigned long long)__bf_tramp_table[425]; }
    if (index == 425) __bf_tramp_table[425] = dlsym(RTLD_NEXT, "setbuf");
    if (index == 425) return __bf_tramp_table[425];
    if (index == 426) { __bf_tramp_table[426] = (void*)(unsigned long long)__bf_tramp_table[426]; }
    if (index == 426) __bf_tramp_table[426] = dlsym(RTLD_NEXT, "strtof_l");
    if (index == 426) return __bf_tramp_table[426];
    if (index == 427) { __bf_tramp_table[427] = (void*)(unsigned long long)__bf_tramp_table[427]; }
    if (index == 427) __bf_tramp_table[427] = dlsym(RTLD_NEXT, "strtod_l");
    if (index == 427) return __bf_tramp_table[427];
    if (index == 428) { __bf_tramp_table[428] = (void*)(unsigned long long)__bf_tramp_table[428]; }
    if (index == 428) __bf_tramp_table[428] = dlsym(RTLD_NEXT, "strdup");
    if (index == 428) return __bf_tramp_table[428];
    if (index == 429) { __bf_tramp_table[429] = (void*)(unsigned long long)__bf_tramp_table[429]; }
    if (index == 429) __bf_tramp_table[429] = dlsym(RTLD_NEXT, "fabs");
    if (index == 429) return __bf_tramp_table[429];
    if (index == 430) { __bf_tramp_table[430] = (void*)(unsigned long long)__bf_tramp_table[430]; }
    if (index == 430) __bf_tramp_table[430] = dlsym(RTLD_NEXT, "ceil");
    if (index == 430) return __bf_tramp_table[430];
    if (index == 431) { __bf_tramp_table[431] = (void*)(unsigned long long)__bf_tramp_table[431]; }
    if (index == 431) __bf_tramp_table[431] = dlsym(RTLD_NEXT, "floor");
    if (index == 431) return __bf_tramp_table[431];
    if (index == 432) { __bf_tramp_table[432] = (void*)(unsigned long long)__bf_tramp_table[432]; }
    if (index == 432) __bf_tramp_table[432] = dlsym(RTLD_NEXT, "sqrt");
    if (index == 432) return __bf_tramp_table[432];
    if (index == 433) { __bf_tramp_table[433] = (void*)(unsigned long long)__bf_tramp_table[433]; }
    if (index == 433) __bf_tramp_table[433] = dlsym(RTLD_NEXT, "sigprocmask");
    if (index == 433) return __bf_tramp_table[433];
    if (index == 434) { __bf_tramp_table[434] = (void*)(unsigned long long)__bf_tramp_table[434]; }
    if (index == 434) __bf_tramp_table[434] = dlsym(RTLD_NEXT, "__pread_chk");
    if (index == 434) return __bf_tramp_table[434];
    if (index == 435) { __bf_tramp_table[435] = (void*)(unsigned long long)__bf_tramp_table[435]; }
    if (index == 435) __bf_tramp_table[435] = dlsym(RTLD_NEXT, "statfs");
    if (index == 435) return __bf_tramp_table[435];
    if (index == 436) { __bf_tramp_table[436] = (void*)(unsigned long long)__bf_tramp_table[436]; }
    if (index == 436) __bf_tramp_table[436] = dlsym(RTLD_NEXT, "symlink");
    if (index == 436) return __bf_tramp_table[436];
    if (index == 437) { __bf_tramp_table[437] = (void*)(unsigned long long)__bf_tramp_table[437]; }
    if (index == 437) __bf_tramp_table[437] = dlsym(RTLD_NEXT, "strsep");
    if (index == 437) return __bf_tramp_table[437];
    if (index == 438) { __bf_tramp_table[438] = (void*)(unsigned long long)__bf_tramp_table[438]; }
    if (index == 438) __bf_tramp_table[438] = dlsym(RTLD_NEXT, "setrlimit");
    if (index == 438) return __bf_tramp_table[438];
    if (index == 439) { __bf_tramp_table[439] = (void*)(unsigned long long)__bf_tramp_table[439]; }
    if (index == 439) __bf_tramp_table[439] = dlsym(RTLD_NEXT, "dup2");
    if (index == 439) return __bf_tramp_table[439];
    if (index == 440) { __bf_tramp_table[440] = (void*)(unsigned long long)__bf_tramp_table[440]; }
    if (index == 440) __bf_tramp_table[440] = dlsym(RTLD_NEXT, "isatty");
    if (index == 440) return __bf_tramp_table[440];
    if (index == 441) { __bf_tramp_table[441] = (void*)(unsigned long long)__bf_tramp_table[441]; }
    if (index == 441) __bf_tramp_table[441] = dlsym(RTLD_NEXT, "getrusage");
    if (index == 441) return __bf_tramp_table[441];
    if (index == 442) { __bf_tramp_table[442] = (void*)(unsigned long long)__bf_tramp_table[442]; }
    if (index == 442) __bf_tramp_table[442] = dlsym(RTLD_NEXT, "alarm");
    if (index == 442) return __bf_tramp_table[442];
    if (index == 443) { __bf_tramp_table[443] = (void*)(unsigned long long)__bf_tramp_table[443]; }
    if (index == 443) __bf_tramp_table[443] = dlsym(RTLD_NEXT, "kill");
    if (index == 443) return __bf_tramp_table[443];
    if (index == 444) { __bf_tramp_table[444] = (void*)(unsigned long long)__bf_tramp_table[444]; }
    if (index == 444) __bf_tramp_table[444] = dlsym(RTLD_NEXT, "wait");
    if (index == 444) return __bf_tramp_table[444];
    if (index == 445) { __bf_tramp_table[445] = (void*)(unsigned long long)__bf_tramp_table[445]; }
    if (index == 445) __bf_tramp_table[445] = dlsym(RTLD_NEXT, "strsignal");
    if (index == 445) return __bf_tramp_table[445];
    if (index == 446) { __bf_tramp_table[446] = (void*)(unsigned long long)__bf_tramp_table[446]; }
    if (index == 446) __bf_tramp_table[446] = dlsym(RTLD_NEXT, "getrlimit");
    if (index == 446) return __bf_tramp_table[446];
    if (index == 447) { __bf_tramp_table[447] = (void*)(unsigned long long)__bf_tramp_table[447]; }
    if (index == 447) __bf_tramp_table[447] = dlsym(RTLD_NEXT, "__register_frame");
    if (index == 447) return __bf_tramp_table[447];
    if (index == 448) { __bf_tramp_table[448] = (void*)(unsigned long long)__bf_tramp_table[448]; }
    if (index == 448) __bf_tramp_table[448] = dlsym(RTLD_NEXT, "__deregister_frame");
    if (index == 448) return __bf_tramp_table[448];
    if (index == 449) { __bf_tramp_table[449] = (void*)(unsigned long long)__bf_tramp_table[449]; }
    if (index == 449) __bf_tramp_table[449] = dlsym(RTLD_NEXT, "__strlcpy_chk");
    if (index == 449) return __bf_tramp_table[449];
    if (index == 450) { __bf_tramp_table[450] = (void*)(unsigned long long)__bf_tramp_table[450]; }
    if (index == 450) __bf_tramp_table[450] = dlsym(RTLD_NEXT, "arc4random");
    if (index == 450) return __bf_tramp_table[450];
    if (index == 451) { __bf_tramp_table[451] = (void*)(unsigned long long)__bf_tramp_table[451]; }
    if (index == 451) __bf_tramp_table[451] = dlsym(RTLD_NEXT, "__strrchr_chk");
    if (index == 451) return __bf_tramp_table[451];
    if (index == 452) { __bf_tramp_table[452] = (void*)(unsigned long long)__bf_tramp_table[452]; }
    if (index == 452) __bf_tramp_table[452] = dlsym(RTLD_NEXT, "__strncat_chk");
    if (index == 452) return __bf_tramp_table[452];
    if (index == 453) { __bf_tramp_table[453] = (void*)(unsigned long long)__bf_tramp_table[453]; }
    if (index == 453) __bf_tramp_table[453] = dlsym(RTLD_NEXT, "tgammaf");
    if (index == 453) return __bf_tramp_table[453];
    if (index == 454) { __bf_tramp_table[454] = (void*)(unsigned long long)__bf_tramp_table[454]; }
    if (index == 454) __bf_tramp_table[454] = dlsym(RTLD_NEXT, "expm1f");
    if (index == 454) return __bf_tramp_table[454];
    if (index == 455) { __bf_tramp_table[455] = (void*)(unsigned long long)__bf_tramp_table[455]; }
    if (index == 455) __bf_tramp_table[455] = dlsym(RTLD_NEXT, "fdimf");
    if (index == 455) return __bf_tramp_table[455];
    if (index == 456) { __bf_tramp_table[456] = (void*)(unsigned long long)__bf_tramp_table[456]; }
    if (index == 456) __bf_tramp_table[456] = dlsym(RTLD_NEXT, "hypotf");
    if (index == 456) return __bf_tramp_table[456];
    if (index == 457) { __bf_tramp_table[457] = (void*)(unsigned long long)__bf_tramp_table[457]; }
    if (index == 457) __bf_tramp_table[457] = dlsym(RTLD_NEXT, "ilogbf");
    if (index == 457) return __bf_tramp_table[457];
    if (index == 458) { __bf_tramp_table[458] = (void*)(unsigned long long)__bf_tramp_table[458]; }
    if (index == 458) __bf_tramp_table[458] = dlsym(RTLD_NEXT, "lgammaf");
    if (index == 458) return __bf_tramp_table[458];
    if (index == 459) { __bf_tramp_table[459] = (void*)(unsigned long long)__bf_tramp_table[459]; }
    if (index == 459) __bf_tramp_table[459] = dlsym(RTLD_NEXT, "lgammaf_r");
    if (index == 459) return __bf_tramp_table[459];
    if (index == 460) { __bf_tramp_table[460] = (void*)(unsigned long long)__bf_tramp_table[460]; }
    if (index == 460) __bf_tramp_table[460] = dlsym(RTLD_NEXT, "log1pf");
    if (index == 460) return __bf_tramp_table[460];
    if (index == 461) { __bf_tramp_table[461] = (void*)(unsigned long long)__bf_tramp_table[461]; }
    if (index == 461) __bf_tramp_table[461] = dlsym(RTLD_NEXT, "logbf");
    if (index == 461) return __bf_tramp_table[461];
    if (index == 462) { __bf_tramp_table[462] = (void*)(unsigned long long)__bf_tramp_table[462]; }
    if (index == 462) __bf_tramp_table[462] = dlsym(RTLD_NEXT, "strndup");
    if (index == 462) return __bf_tramp_table[462];
    if (index == 463) { __bf_tramp_table[463] = (void*)(unsigned long long)__bf_tramp_table[463]; }
    if (index == 463) __bf_tramp_table[463] = dlsym(RTLD_NEXT, "strlcpy");
    if (index == 463) return __bf_tramp_table[463];
    if (index == 464) { __bf_tramp_table[464] = (void*)(unsigned long long)__bf_tramp_table[464]; }
    if (index == 464) __bf_tramp_table[464] = dlsym(RTLD_NEXT, "setpgid");
    if (index == 464) return __bf_tramp_table[464];
    if (index == 465) { __bf_tramp_table[465] = (void*)(unsigned long long)__bf_tramp_table[465]; }
    if (index == 465) __bf_tramp_table[465] = dlsym(RTLD_NEXT, "__strlcat_chk");
    if (index == 465) return __bf_tramp_table[465];
    if (index == 466) { __bf_tramp_table[466] = (void*)(unsigned long long)__bf_tramp_table[466]; }
    if (index == 466) __bf_tramp_table[466] = dlsym(RTLD_NEXT, "clock_nanosleep");
    if (index == 466) return __bf_tramp_table[466];
    if (index == 467) { __bf_tramp_table[467] = (void*)(unsigned long long)__bf_tramp_table[467]; }
    if (index == 467) __bf_tramp_table[467] = dlsym(RTLD_NEXT, "_Unwind_GetCFA");
    if (index == 467) return __bf_tramp_table[467];
    if (index == 468) { __bf_tramp_table[468] = (void*)(unsigned long long)__bf_tramp_table[468]; }
    if (index == 468) __bf_tramp_table[468] = dlsym(RTLD_NEXT, "_Unwind_FindEnclosingFunction");
    if (index == 468) return __bf_tramp_table[468];
    if (index == 469) { __bf_tramp_table[469] = (void*)(unsigned long long)__bf_tramp_table[469]; }
    if (index == 469) __bf_tramp_table[469] = dlsym(RTLD_NEXT, "_Unwind_Backtrace");
    if (index == 469) return __bf_tramp_table[469];
    if (index == 470) { __bf_tramp_table[470] = (void*)(unsigned long long)__bf_tramp_table[470]; }
    if (index == 470) __bf_tramp_table[470] = dlsym(RTLD_NEXT, "asprintf");
    if (index == 470) return __bf_tramp_table[470];
    if (index == 471) { __bf_tramp_table[471] = (void*)(unsigned long long)__bf_tramp_table[471]; }
    if (index == 471) __bf_tramp_table[471] = dlsym(RTLD_NEXT, "__system_property_area_serial");
    if (index == 471) return __bf_tramp_table[471];
    if (index == 472) { __bf_tramp_table[472] = (void*)(unsigned long long)__bf_tramp_table[472]; }
    if (index == 472) __bf_tramp_table[472] = dlsym(RTLD_NEXT, "__system_property_serial");
    if (index == 472) return __bf_tramp_table[472];
    if (index == 473) { __bf_tramp_table[473] = (void*)(unsigned long long)__bf_tramp_table[473]; }
    if (index == 473) __bf_tramp_table[473] = dlsym(RTLD_NEXT, "__system_property_read");
    if (index == 473) return __bf_tramp_table[473];
    if (index == 474) { __bf_tramp_table[474] = (void*)(unsigned long long)__bf_tramp_table[474]; }
    if (index == 474) __bf_tramp_table[474] = dlsym(RTLD_NEXT, "fnmatch");
    if (index == 474) return __bf_tramp_table[474];
    if (index == 475) { __bf_tramp_table[475] = (void*)(unsigned long long)__bf_tramp_table[475]; }
    if (index == 475) __bf_tramp_table[475] = dlsym(RTLD_NEXT, "timerfd_gettime");
    if (index == 475) return __bf_tramp_table[475];
    if (index == 476) { __bf_tramp_table[476] = (void*)(unsigned long long)__bf_tramp_table[476]; }
    if (index == 476) __bf_tramp_table[476] = dlsym(RTLD_NEXT, "__openat_2");
    if (index == 476) return __bf_tramp_table[476];
    if (index == 477) { __bf_tramp_table[477] = (void*)(unsigned long long)__bf_tramp_table[477]; }
    if (index == 477) __bf_tramp_table[477] = dlsym(RTLD_NEXT, "android_getaddrinfofornet");
    if (index == 477) return __bf_tramp_table[477];
    if (index == 478) { __bf_tramp_table[478] = (void*)(unsigned long long)__bf_tramp_table[478]; }
    if (index == 478) __bf_tramp_table[478] = dlsym(RTLD_NEXT, "__system_property_set");
    if (index == 478) return __bf_tramp_table[478];
    if (index == 479) { __bf_tramp_table[479] = (void*)(unsigned long long)__bf_tramp_table[479]; }
    if (index == 479) __bf_tramp_table[479] = dlsym(RTLD_NEXT, "setprogname");
    if (index == 479) return __bf_tramp_table[479];
    if (index == 480) { __bf_tramp_table[480] = (void*)(unsigned long long)__bf_tramp_table[480]; }
    if (index == 480) __bf_tramp_table[480] = dlsym(RTLD_NEXT, "fallocate");
    if (index == 480) return __bf_tramp_table[480];
    if (index == 481) { __bf_tramp_table[481] = (void*)(unsigned long long)__bf_tramp_table[481]; }
    if (index == 481) __bf_tramp_table[481] = dlsym(RTLD_NEXT, "lstat64");
    if (index == 481) return __bf_tramp_table[481];
    if (index == 482) { __bf_tramp_table[482] = (void*)(unsigned long long)__bf_tramp_table[482]; }
    if (index == 482) __bf_tramp_table[482] = dlsym(RTLD_NEXT, "mkstemp");
    if (index == 482) return __bf_tramp_table[482];
    if (index == 483) { __bf_tramp_table[483] = (void*)(unsigned long long)__bf_tramp_table[483]; }
    if (index == 483) __bf_tramp_table[483] = dlsym(RTLD_NEXT, "chmod");
    if (index == 483) return __bf_tramp_table[483];
    if (index == 484) { __bf_tramp_table[484] = (void*)(unsigned long long)__bf_tramp_table[484]; }
    if (index == 484) __bf_tramp_table[484] = dlsym(RTLD_NEXT, "statfs64");
    if (index == 484) return __bf_tramp_table[484];
    if (index == 485) { __bf_tramp_table[485] = (void*)(unsigned long long)__bf_tramp_table[485]; }
    if (index == 485) __bf_tramp_table[485] = dlsym(RTLD_NEXT, "android_dlwarning");
    if (index == 485) return __bf_tramp_table[485];
    if (index == 486) { __bf_tramp_table[486] = (void*)(unsigned long long)__bf_tramp_table[486]; }
    if (index == 486) __bf_tramp_table[486] = dlsym(RTLD_NEXT, "mallopt");
    if (index == 486) return __bf_tramp_table[486];
    if (index == 487) { __bf_tramp_table[487] = (void*)(unsigned long long)__bf_tramp_table[487]; }
    if (index == 487) __bf_tramp_table[487] = dlsym(RTLD_NEXT, "malloc_info");
    if (index == 487) return __bf_tramp_table[487];
    if (index == 488) { __bf_tramp_table[488] = (void*)(unsigned long long)__bf_tramp_table[488]; }
    if (index == 488) __bf_tramp_table[488] = dlsym(RTLD_NEXT, "fdatasync");
    if (index == 488) return __bf_tramp_table[488];
    if (index == 489) { __bf_tramp_table[489] = (void*)(unsigned long long)__bf_tramp_table[489]; }
    if (index == 489) __bf_tramp_table[489] = dlsym(RTLD_NEXT, "android_get_application_target_sdk_version");
    if (index == 489) return __bf_tramp_table[489];
    if (index == 490) { __bf_tramp_table[490] = (void*)(unsigned long long)__bf_tramp_table[490]; }
    if (index == 490) __bf_tramp_table[490] = dlsym(RTLD_NEXT, "getpwnam");
    if (index == 490) return __bf_tramp_table[490];
    if (index == 491) { __bf_tramp_table[491] = (void*)(unsigned long long)__bf_tramp_table[491]; }
    if (index == 491) __bf_tramp_table[491] = dlsym(RTLD_NEXT, "getgrnam");
    if (index == 491) return __bf_tramp_table[491];
    if (index == 492) { __bf_tramp_table[492] = (void*)(unsigned long long)__bf_tramp_table[492]; }
    if (index == 492) __bf_tramp_table[492] = dlsym(RTLD_NEXT, "__sched_cpucount");
    if (index == 492) return __bf_tramp_table[492];
    if (index == 493) { __bf_tramp_table[493] = (void*)(unsigned long long)__bf_tramp_table[493]; }
    if (index == 493) __bf_tramp_table[493] = dlsym(RTLD_NEXT, "sched_getaffinity");
    if (index == 493) return __bf_tramp_table[493];
    if (index == 494) { __bf_tramp_table[494] = (void*)(unsigned long long)__bf_tramp_table[494]; }
    if (index == 494) __bf_tramp_table[494] = dlsym(RTLD_NEXT, "get_nprocs_conf");
    if (index == 494) return __bf_tramp_table[494];
    if (index == 495) { __bf_tramp_table[495] = (void*)(unsigned long long)__bf_tramp_table[495]; }
    if (index == 495) __bf_tramp_table[495] = dlsym(RTLD_NEXT, "setuid");
    if (index == 495) return __bf_tramp_table[495];
    if (index == 496) { __bf_tramp_table[496] = (void*)(unsigned long long)__bf_tramp_table[496]; }
    if (index == 496) __bf_tramp_table[496] = dlsym(RTLD_NEXT, "setgid");
    if (index == 496) return __bf_tramp_table[496];
    if (index == 497) { __bf_tramp_table[497] = (void*)(unsigned long long)__bf_tramp_table[497]; }
    if (index == 497) __bf_tramp_table[497] = dlsym(RTLD_NEXT, "tgkill");
    if (index == 497) return __bf_tramp_table[497];
    if (index == 498) { __bf_tramp_table[498] = (void*)(unsigned long long)__bf_tramp_table[498]; }
    if (index == 498) __bf_tramp_table[498] = dlsym(RTLD_NEXT, "getline");
    if (index == 498) return __bf_tramp_table[498];
    if (index == 499) { __bf_tramp_table[499] = (void*)(unsigned long long)__bf_tramp_table[499]; }
    if (index == 499) __bf_tramp_table[499] = dlsym(RTLD_NEXT, "strtok_r");
    if (index == 499) return __bf_tramp_table[499];
    if (index == 500) { __bf_tramp_table[500] = (void*)(unsigned long long)__bf_tramp_table[500]; }
    if (index == 500) __bf_tramp_table[500] = dlsym(RTLD_NEXT, "pidfd_open");
    if (index == 500) return __bf_tramp_table[500];
    if (index == 501) { __bf_tramp_table[501] = (void*)(unsigned long long)__bf_tramp_table[501]; }
    if (index == 501) __bf_tramp_table[501] = dlsym(RTLD_NEXT, "tcflush");
    if (index == 501) return __bf_tramp_table[501];
    if (index == 502) { __bf_tramp_table[502] = (void*)(unsigned long long)__bf_tramp_table[502]; }
    if (index == 502) __bf_tramp_table[502] = dlsym(RTLD_NEXT, "tcsendbreak");
    if (index == 502) return __bf_tramp_table[502];
    if (index == 503) { __bf_tramp_table[503] = (void*)(unsigned long long)__bf_tramp_table[503]; }
    if (index == 503) __bf_tramp_table[503] = dlsym(RTLD_NEXT, "inotify_init1");
    if (index == 503) return __bf_tramp_table[503];
    if (index == 504) { __bf_tramp_table[504] = (void*)(unsigned long long)__bf_tramp_table[504]; }
    if (index == 504) __bf_tramp_table[504] = dlsym(RTLD_NEXT, "inotify_add_watch");
    if (index == 504) return __bf_tramp_table[504];
    if (index == 505) { __bf_tramp_table[505] = (void*)(unsigned long long)__bf_tramp_table[505]; }
    if (index == 505) __bf_tramp_table[505] = dlsym(RTLD_NEXT, "inotify_rm_watch");
    if (index == 505) return __bf_tramp_table[505];
    if (index == 506) { __bf_tramp_table[506] = (void*)(unsigned long long)__bf_tramp_table[506]; }
    if (index == 506) __bf_tramp_table[506] = dlsym(RTLD_NEXT, "android_fdsan_get_error_level");
    if (index == 506) return __bf_tramp_table[506];
    if (index == 507) { __bf_tramp_table[507] = (void*)(unsigned long long)__bf_tramp_table[507]; }
    if (index == 507) __bf_tramp_table[507] = dlsym(RTLD_NEXT, "android_reset_stack_guards");
    if (index == 507) return __bf_tramp_table[507];
    if (index == 508) { __bf_tramp_table[508] = (void*)(unsigned long long)__bf_tramp_table[508]; }
    if (index == 508) __bf_tramp_table[508] = dlsym(RTLD_NEXT, "android_fdsan_set_error_level");
    if (index == 508) return __bf_tramp_table[508];
    if (index == 509) { __bf_tramp_table[509] = (void*)(unsigned long long)__bf_tramp_table[509]; }
    if (index == 509) __bf_tramp_table[509] = dlsym(RTLD_NEXT, "dup3");
    if (index == 509) return __bf_tramp_table[509];
    if (index == 510) { __bf_tramp_table[510] = (void*)(unsigned long long)__bf_tramp_table[510]; }
    if (index == 510) __bf_tramp_table[510] = dlsym(RTLD_NEXT, "capget");
    if (index == 510) return __bf_tramp_table[510];
    if (index == 511) { __bf_tramp_table[511] = (void*)(unsigned long long)__bf_tramp_table[511]; }
    if (index == 511) __bf_tramp_table[511] = dlsym(RTLD_NEXT, "setresgid");
    if (index == 511) return __bf_tramp_table[511];
    if (index == 512) { __bf_tramp_table[512] = (void*)(unsigned long long)__bf_tramp_table[512]; }
    if (index == 512) __bf_tramp_table[512] = dlsym(RTLD_NEXT, "setresuid");
    if (index == 512) return __bf_tramp_table[512];
    if (index == 513) { __bf_tramp_table[513] = (void*)(unsigned long long)__bf_tramp_table[513]; }
    if (index == 513) __bf_tramp_table[513] = dlsym(RTLD_NEXT, "android_set_16kb_appcompat_mode");
    if (index == 513) return __bf_tramp_table[513];
    if (index == 514) { __bf_tramp_table[514] = (void*)(unsigned long long)__bf_tramp_table[514]; }
    if (index == 514) __bf_tramp_table[514] = dlsym(RTLD_NEXT, "capset");
    if (index == 514) return __bf_tramp_table[514];
    if (index == 515) { __bf_tramp_table[515] = (void*)(unsigned long long)__bf_tramp_table[515]; }
    if (index == 515) __bf_tramp_table[515] = dlsym(RTLD_NEXT, "mount");
    if (index == 515) return __bf_tramp_table[515];
    if (index == 516) { __bf_tramp_table[516] = (void*)(unsigned long long)__bf_tramp_table[516]; }
    if (index == 516) __bf_tramp_table[516] = dlsym(RTLD_NEXT, "unshare");
    if (index == 516) return __bf_tramp_table[516];
    if (index == 517) { __bf_tramp_table[517] = (void*)(unsigned long long)__bf_tramp_table[517]; }
    if (index == 517) __bf_tramp_table[517] = dlsym(RTLD_NEXT, "__system_properties_zygote_reload");
    if (index == 517) return __bf_tramp_table[517];
    if (index == 518) { __bf_tramp_table[518] = (void*)(unsigned long long)__bf_tramp_table[518]; }
    if (index == 518) __bf_tramp_table[518] = dlsym(RTLD_NEXT, "setgroups");
    if (index == 518) return __bf_tramp_table[518];
    if (index == 519) { __bf_tramp_table[519] = (void*)(unsigned long long)__bf_tramp_table[519]; }
    if (index == 519) __bf_tramp_table[519] = dlsym(RTLD_NEXT, "setmntent");
    if (index == 519) return __bf_tramp_table[519];
    if (index == 520) { __bf_tramp_table[520] = (void*)(unsigned long long)__bf_tramp_table[520]; }
    if (index == 520) __bf_tramp_table[520] = dlsym(RTLD_NEXT, "getmntent");
    if (index == 520) return __bf_tramp_table[520];
    if (index == 521) { __bf_tramp_table[521] = (void*)(unsigned long long)__bf_tramp_table[521]; }
    if (index == 521) __bf_tramp_table[521] = dlsym(RTLD_NEXT, "endmntent");
    if (index == 521) return __bf_tramp_table[521];
    if (index == 522) { __bf_tramp_table[522] = (void*)(unsigned long long)__bf_tramp_table[522]; }
    if (index == 522) __bf_tramp_table[522] = dlsym(RTLD_NEXT, "umount2");
    if (index == 522) return __bf_tramp_table[522];
    if (index == 523) { __bf_tramp_table[523] = (void*)(unsigned long long)__bf_tramp_table[523]; }
    if (index == 523) __bf_tramp_table[523] = dlsym(RTLD_NEXT, "statx");
    if (index == 523) return __bf_tramp_table[523];
    if (index == 524) { __bf_tramp_table[524] = (void*)(unsigned long long)__bf_tramp_table[524]; }
    if (index == 524) __bf_tramp_table[524] = dlsym(RTLD_NEXT, "memset_explicit");
    if (index == 524) return __bf_tramp_table[524];
    if (index == 525) { __bf_tramp_table[525] = (void*)(unsigned long long)__bf_tramp_table[525]; }
    if (index == 525) __bf_tramp_table[525] = dlsym(RTLD_NEXT, "lseek64");
    if (index == 525) return __bf_tramp_table[525];
    if (index == 526) { __bf_tramp_table[526] = (void*)(unsigned long long)__bf_tramp_table[526]; }
    if (index == 526) __bf_tramp_table[526] = dlsym(RTLD_NEXT, "dirfd");
    if (index == 526) return __bf_tramp_table[526];
    if (index == 527) { __bf_tramp_table[527] = (void*)(unsigned long long)__bf_tramp_table[527]; }
    if (index == 527) __bf_tramp_table[527] = dlsym(RTLD_NEXT, "malloc_usable_size");
    if (index == 527) return __bf_tramp_table[527];
    if (index == 528) { __bf_tramp_table[528] = (void*)(unsigned long long)__bf_tramp_table[528]; }
    if (index == 528) __bf_tramp_table[528] = dlsym(RTLD_NEXT, "fstatfs");
    if (index == 528) return __bf_tramp_table[528];
    if (index == 529) { __bf_tramp_table[529] = (void*)(unsigned long long)__bf_tramp_table[529]; }
    if (index == 529) __bf_tramp_table[529] = dlsym(RTLD_NEXT, "_Unwind_GetTextRelBase");
    if (index == 529) return __bf_tramp_table[529];
    if (index == 530) { __bf_tramp_table[530] = (void*)(unsigned long long)__bf_tramp_table[530]; }
    if (index == 530) __bf_tramp_table[530] = dlsym(RTLD_NEXT, "_Unwind_GetDataRelBase");
    if (index == 530) return __bf_tramp_table[530];
    if (index == 531) { __bf_tramp_table[531] = (void*)(unsigned long long)__bf_tramp_table[531]; }
    if (index == 531) __bf_tramp_table[531] = dlsym(RTLD_NEXT, "_Unwind_GetIPInfo");
    if (index == 531) return __bf_tramp_table[531];
    if (index == 532) { __bf_tramp_table[532] = (void*)(unsigned long long)__bf_tramp_table[532]; }
    if (index == 532) __bf_tramp_table[532] = dlsym(RTLD_NEXT, "settimeofday");
    if (index == 532) return __bf_tramp_table[532];
    if (index == 533) { __bf_tramp_table[533] = (void*)(unsigned long long)__bf_tramp_table[533]; }
    if (index == 533) __bf_tramp_table[533] = dlsym(RTLD_NEXT, "sigqueue");
    if (index == 533) return __bf_tramp_table[533];
    if (index == 534) { __bf_tramp_table[534] = (void*)(unsigned long long)__bf_tramp_table[534]; }
    if (index == 534) __bf_tramp_table[534] = dlsym(RTLD_NEXT, "eventfd_write");
    if (index == 534) return __bf_tramp_table[534];
    if (index == 535) { __bf_tramp_table[535] = (void*)(unsigned long long)__bf_tramp_table[535]; }
    if (index == 535) __bf_tramp_table[535] = dlsym(RTLD_NEXT, "process_madvise");
    if (index == 535) return __bf_tramp_table[535];
    if (index == 536) { __bf_tramp_table[536] = (void*)(unsigned long long)__bf_tramp_table[536]; }
    if (index == 536) __bf_tramp_table[536] = dlsym(RTLD_NEXT, "inotify_init");
    if (index == 536) return __bf_tramp_table[536];
    if (index == 537) { __bf_tramp_table[537] = (void*)(unsigned long long)__bf_tramp_table[537]; }
    if (index == 537) __bf_tramp_table[537] = dlsym(RTLD_NEXT, "flock");
    if (index == 537) return __bf_tramp_table[537];
    if (index == 538) { __bf_tramp_table[538] = (void*)(unsigned long long)__bf_tramp_table[538]; }
    if (index == 538) __bf_tramp_table[538] = dlsym(RTLD_NEXT, "execvp");
    if (index == 538) return __bf_tramp_table[538];
    if (index == 539) { __bf_tramp_table[539] = (void*)(unsigned long long)__bf_tramp_table[539]; }
    if (index == 539) __bf_tramp_table[539] = dlsym(RTLD_NEXT, "putchar");
    if (index == 539) return __bf_tramp_table[539];
    if (index == 540) { __bf_tramp_table[540] = (void*)(unsigned long long)__bf_tramp_table[540]; }
    if (index == 540) __bf_tramp_table[540] = dlsym(RTLD_NEXT, "dprintf");
    if (index == 540) return __bf_tramp_table[540];
    if (index == 541) { __bf_tramp_table[541] = (void*)(unsigned long long)__bf_tramp_table[541]; }
    if (index == 541) __bf_tramp_table[541] = dlsym(RTLD_NEXT, "memfd_create");
    if (index == 541) return __bf_tramp_table[541];
    if (index == 542) { __bf_tramp_table[542] = (void*)(unsigned long long)__bf_tramp_table[542]; }
    if (index == 542) __bf_tramp_table[542] = dlsym(RTLD_NEXT, "android_get_exported_namespace");
    if (index == 542) return __bf_tramp_table[542];
    if (index == 543) { __bf_tramp_table[543] = (void*)(unsigned long long)__bf_tramp_table[543]; }
    if (index == 543) __bf_tramp_table[543] = dlsym(RTLD_NEXT, "readv");
    if (index == 543) return __bf_tramp_table[543];
    if (index == 544) { __bf_tramp_table[544] = (void*)(unsigned long long)__bf_tramp_table[544]; }
    if (index == 544) __bf_tramp_table[544] = dlsym(RTLD_NEXT, "mkdtemp");
    if (index == 544) return __bf_tramp_table[544];
    if (index == 545) { __bf_tramp_table[545] = (void*)(unsigned long long)__bf_tramp_table[545]; }
    if (index == 545) __bf_tramp_table[545] = dlsym(RTLD_NEXT, "pthread_gettid_np");
    if (index == 545) return __bf_tramp_table[545];
    if (index == 546) { __bf_tramp_table[546] = (void*)(unsigned long long)__bf_tramp_table[546]; }
    if (index == 546) __bf_tramp_table[546] = dlsym(RTLD_NEXT, "isnanf");
    if (index == 546) return __bf_tramp_table[546];
    if (index == 547) { __bf_tramp_table[547] = (void*)(unsigned long long)__bf_tramp_table[547]; }
    if (index == 547) __bf_tramp_table[547] = dlsym(RTLD_NEXT, "sched_setaffinity");
    if (index == 547) return __bf_tramp_table[547];
    if (index == 548) { __bf_tramp_table[548] = (void*)(unsigned long long)__bf_tramp_table[548]; }
    if (index == 548) __bf_tramp_table[548] = dlsym(RTLD_NEXT, "pthread_getname_np");
    if (index == 548) return __bf_tramp_table[548];
    if (index == 549) { __bf_tramp_table[549] = (void*)(unsigned long long)__bf_tramp_table[549]; }
    if (index == 549) __bf_tramp_table[549] = dlsym(RTLD_NEXT, "nftw");
    if (index == 549) return __bf_tramp_table[549];
    if (index == 550) { __bf_tramp_table[550] = (void*)(unsigned long long)__bf_tramp_table[550]; }
    if (index == 550) __bf_tramp_table[550] = dlsym(RTLD_NEXT, "__pwrite_chk");
    if (index == 550) return __bf_tramp_table[550];
    if (index == 551) { __bf_tramp_table[551] = (void*)(unsigned long long)__bf_tramp_table[551]; }
    if (index == 551) __bf_tramp_table[551] = dlsym(RTLD_NEXT, "basename");
    if (index == 551) return __bf_tramp_table[551];
    if (index == 552) { __bf_tramp_table[552] = (void*)(unsigned long long)__bf_tramp_table[552]; }
    if (index == 552) __bf_tramp_table[552] = dlsym(RTLD_NEXT, "__system_property_wait");
    if (index == 552) return __bf_tramp_table[552];
    if (index == 553) { __bf_tramp_table[553] = (void*)(unsigned long long)__bf_tramp_table[553]; }
    if (index == 553) __bf_tramp_table[553] = dlsym(RTLD_NEXT, "inet_ntoa");
    if (index == 553) return __bf_tramp_table[553];
    if (index == 554) { __bf_tramp_table[554] = (void*)(unsigned long long)__bf_tramp_table[554]; }
    if (index == 554) __bf_tramp_table[554] = dlsym(RTLD_NEXT, "pipe2");
    if (index == 554) return __bf_tramp_table[554];
    if (index == 555) { __bf_tramp_table[555] = (void*)(unsigned long long)__bf_tramp_table[555]; }
    if (index == 555) __bf_tramp_table[555] = dlsym(RTLD_NEXT, "hypot");
    if (index == 555) return __bf_tramp_table[555];
    if (index == 556) { __bf_tramp_table[556] = (void*)(unsigned long long)__bf_tramp_table[556]; }
    if (index == 556) __bf_tramp_table[556] = dlsym(RTLD_NEXT, "faccessat");
    if (index == 556) return __bf_tramp_table[556];
    if (index == 557) { __bf_tramp_table[557] = (void*)(unsigned long long)__bf_tramp_table[557]; }
    if (index == 557) __bf_tramp_table[557] = dlsym(RTLD_NEXT, "fmemopen");
    if (index == 557) return __bf_tramp_table[557];
    if (index == 558) { __bf_tramp_table[558] = (void*)(unsigned long long)__bf_tramp_table[558]; }
    if (index == 558) __bf_tramp_table[558] = dlsym(RTLD_NEXT, "dirname");
    if (index == 558) return __bf_tramp_table[558];
    if (index == 559) { __bf_tramp_table[559] = (void*)(unsigned long long)__bf_tramp_table[559]; }
    if (index == 559) __bf_tramp_table[559] = dlsym(RTLD_NEXT, "memccpy");
    if (index == 559) return __bf_tramp_table[559];
    if (index == 560) { __bf_tramp_table[560] = (void*)(unsigned long long)__bf_tramp_table[560]; }
    if (index == 560) __bf_tramp_table[560] = dlsym(RTLD_NEXT, "tsearch");
    if (index == 560) return __bf_tramp_table[560];
    if (index == 561) { __bf_tramp_table[561] = (void*)(unsigned long long)__bf_tramp_table[561]; }
    if (index == 561) __bf_tramp_table[561] = dlsym(RTLD_NEXT, "tfind");
    if (index == 561) return __bf_tramp_table[561];
    if (index == 562) { __bf_tramp_table[562] = (void*)(unsigned long long)__bf_tramp_table[562]; }
    if (index == 562) __bf_tramp_table[562] = dlsym(RTLD_NEXT, "tdelete");
    if (index == 562) return __bf_tramp_table[562];
    if (index == 563) { __bf_tramp_table[563] = (void*)(unsigned long long)__bf_tramp_table[563]; }
    if (index == 563) __bf_tramp_table[563] = dlsym(RTLD_NEXT, "tdestroy");
    if (index == 563) return __bf_tramp_table[563];
    if (index == 564) { __bf_tramp_table[564] = (void*)(unsigned long long)__bf_tramp_table[564]; }
    if (index == 564) __bf_tramp_table[564] = dlsym(RTLD_NEXT, "android_get_device_api_level");
    if (index == 564) return __bf_tramp_table[564];
    if (index == 565) { __bf_tramp_table[565] = (void*)(unsigned long long)__bf_tramp_table[565]; }
    if (index == 565) __bf_tramp_table[565] = dlsym(RTLD_NEXT, "chown");
    if (index == 565) return __bf_tramp_table[565];
    if (index == 566) { __bf_tramp_table[566] = (void*)(unsigned long long)__bf_tramp_table[566]; }
    if (index == 566) __bf_tramp_table[566] = dlsym(RTLD_NEXT, "perror");
    if (index == 566) return __bf_tramp_table[566];
    if (index == 567) { __bf_tramp_table[567] = (void*)(unsigned long long)__bf_tramp_table[567]; }
    if (index == 567) __bf_tramp_table[567] = dlsym(RTLD_NEXT, "clone");
    if (index == 567) return __bf_tramp_table[567];
    if (index == 568) { __bf_tramp_table[568] = (void*)(unsigned long long)__bf_tramp_table[568]; }
    if (index == 568) __bf_tramp_table[568] = dlsym(RTLD_NEXT, "getpwnam_r");
    if (index == 568) return __bf_tramp_table[568];
    if (index == 569) { __bf_tramp_table[569] = (void*)(unsigned long long)__bf_tramp_table[569]; }
    if (index == 569) __bf_tramp_table[569] = dlsym(RTLD_NEXT, "getgrnam_r");
    if (index == 569) return __bf_tramp_table[569];
    if (index == 570) { __bf_tramp_table[570] = (void*)(unsigned long long)__bf_tramp_table[570]; }
    if (index == 570) __bf_tramp_table[570] = dlsym(RTLD_NEXT, "signalfd");
    if (index == 570) return __bf_tramp_table[570];
    if (index == 571) { __bf_tramp_table[571] = (void*)(unsigned long long)__bf_tramp_table[571]; }
    if (index == 571) __bf_tramp_table[571] = dlsym(RTLD_NEXT, "sigdelset");
    if (index == 571) return __bf_tramp_table[571];
    if (index == 572) { __bf_tramp_table[572] = (void*)(unsigned long long)__bf_tramp_table[572]; }
    if (index == 572) __bf_tramp_table[572] = dlsym(RTLD_NEXT, "sigismember");
    if (index == 572) return __bf_tramp_table[572];
    if (index == 573) { __bf_tramp_table[573] = (void*)(unsigned long long)__bf_tramp_table[573]; }
    if (index == 573) __bf_tramp_table[573] = dlsym(RTLD_NEXT, "mkdirat");
    if (index == 573) return __bf_tramp_table[573];
    if (index == 574) { __bf_tramp_table[574] = (void*)(unsigned long long)__bf_tramp_table[574]; }
    if (index == 574) __bf_tramp_table[574] = dlsym(RTLD_NEXT, "openat");
    if (index == 574) return __bf_tramp_table[574];
    if (index == 575) { __bf_tramp_table[575] = (void*)(unsigned long long)__bf_tramp_table[575]; }
    if (index == 575) __bf_tramp_table[575] = dlsym(RTLD_NEXT, "waitid");
    if (index == 575) return __bf_tramp_table[575];
    if (index == 576) { __bf_tramp_table[576] = (void*)(unsigned long long)__bf_tramp_table[576]; }
    if (index == 576) __bf_tramp_table[576] = dlsym(RTLD_NEXT, "strtold");
    if (index == 576) return __bf_tramp_table[576];
    if (index == 577) { __bf_tramp_table[577] = (void*)(unsigned long long)__bf_tramp_table[577]; }
    if (index == 577) __bf_tramp_table[577] = dlsym(RTLD_NEXT, "wcstol");
    if (index == 577) return __bf_tramp_table[577];
    if (index == 578) { __bf_tramp_table[578] = (void*)(unsigned long long)__bf_tramp_table[578]; }
    if (index == 578) __bf_tramp_table[578] = dlsym(RTLD_NEXT, "wcstoul");
    if (index == 578) return __bf_tramp_table[578];
    if (index == 579) { __bf_tramp_table[579] = (void*)(unsigned long long)__bf_tramp_table[579]; }
    if (index == 579) __bf_tramp_table[579] = dlsym(RTLD_NEXT, "wcstoll");
    if (index == 579) return __bf_tramp_table[579];
    if (index == 580) { __bf_tramp_table[580] = (void*)(unsigned long long)__bf_tramp_table[580]; }
    if (index == 580) __bf_tramp_table[580] = dlsym(RTLD_NEXT, "wcstoull");
    if (index == 580) return __bf_tramp_table[580];
    if (index == 581) { __bf_tramp_table[581] = (void*)(unsigned long long)__bf_tramp_table[581]; }
    if (index == 581) __bf_tramp_table[581] = dlsym(RTLD_NEXT, "wcstof");
    if (index == 581) return __bf_tramp_table[581];
    if (index == 582) { __bf_tramp_table[582] = (void*)(unsigned long long)__bf_tramp_table[582]; }
    if (index == 582) __bf_tramp_table[582] = dlsym(RTLD_NEXT, "wcstod");
    if (index == 582) return __bf_tramp_table[582];
    if (index == 583) { __bf_tramp_table[583] = (void*)(unsigned long long)__bf_tramp_table[583]; }
    if (index == 583) __bf_tramp_table[583] = dlsym(RTLD_NEXT, "wcstold");
    if (index == 583) return __bf_tramp_table[583];
    if (index == 584) { __bf_tramp_table[584] = (void*)(unsigned long long)__bf_tramp_table[584]; }
    if (index == 584) __bf_tramp_table[584] = dlsym(RTLD_NEXT, "swprintf");
    if (index == 584) return __bf_tramp_table[584];
    if (index == 585) { __bf_tramp_table[585] = (void*)(unsigned long long)__bf_tramp_table[585]; }
    if (index == 585) __bf_tramp_table[585] = dlsym(RTLD_NEXT, "setlocale");
    if (index == 585) return __bf_tramp_table[585];
    if (index == 586) { __bf_tramp_table[586] = (void*)(unsigned long long)__bf_tramp_table[586]; }
    if (index == 586) __bf_tramp_table[586] = dlsym(RTLD_NEXT, "link");
    if (index == 586) return __bf_tramp_table[586];
    if (index == 587) { __bf_tramp_table[587] = (void*)(unsigned long long)__bf_tramp_table[587]; }
    if (index == 587) __bf_tramp_table[587] = dlsym(RTLD_NEXT, "pathconf");
    if (index == 587) return __bf_tramp_table[587];
    if (index == 588) { __bf_tramp_table[588] = (void*)(unsigned long long)__bf_tramp_table[588]; }
    if (index == 588) __bf_tramp_table[588] = dlsym(RTLD_NEXT, "chdir");
    if (index == 588) return __bf_tramp_table[588];
    if (index == 589) { __bf_tramp_table[589] = (void*)(unsigned long long)__bf_tramp_table[589]; }
    if (index == 589) __bf_tramp_table[589] = dlsym(RTLD_NEXT, "fchmodat");
    if (index == 589) return __bf_tramp_table[589];
    if (index == 590) { __bf_tramp_table[590] = (void*)(unsigned long long)__bf_tramp_table[590]; }
    if (index == 590) __bf_tramp_table[590] = dlsym(RTLD_NEXT, "truncate");
    if (index == 590) return __bf_tramp_table[590];
    if (index == 591) { __bf_tramp_table[591] = (void*)(unsigned long long)__bf_tramp_table[591]; }
    if (index == 591) __bf_tramp_table[591] = dlsym(RTLD_NEXT, "sendfile");
    if (index == 591) return __bf_tramp_table[591];
    if (index == 592) { __bf_tramp_table[592] = (void*)(unsigned long long)__bf_tramp_table[592]; }
    if (index == 592) __bf_tramp_table[592] = dlsym(RTLD_NEXT, "fdopendir");
    if (index == 592) return __bf_tramp_table[592];
    if (index == 593) { __bf_tramp_table[593] = (void*)(unsigned long long)__bf_tramp_table[593]; }
    if (index == 593) __bf_tramp_table[593] = dlsym(RTLD_NEXT, "unlinkat");
    if (index == 593) return __bf_tramp_table[593];
    if (index == 594) { __bf_tramp_table[594] = (void*)(unsigned long long)__bf_tramp_table[594]; }
    if (index == 594) __bf_tramp_table[594] = dlsym(RTLD_NEXT, "fgetxattr");
    if (index == 594) return __bf_tramp_table[594];
    if (index == 595) { __bf_tramp_table[595] = (void*)(unsigned long long)__bf_tramp_table[595]; }
    if (index == 595) __bf_tramp_table[595] = dlsym(RTLD_NEXT, "getxattr");
    if (index == 595) return __bf_tramp_table[595];
    if (index == 596) { __bf_tramp_table[596] = (void*)(unsigned long long)__bf_tramp_table[596]; }
    if (index == 596) __bf_tramp_table[596] = dlsym(RTLD_NEXT, "fremovexattr");
    if (index == 596) return __bf_tramp_table[596];
    if (index == 597) { __bf_tramp_table[597] = (void*)(unsigned long long)__bf_tramp_table[597]; }
    if (index == 597) __bf_tramp_table[597] = dlsym(RTLD_NEXT, "fsetxattr");
    if (index == 597) return __bf_tramp_table[597];
    if (index == 598) { __bf_tramp_table[598] = (void*)(unsigned long long)__bf_tramp_table[598]; }
    if (index == 598) __bf_tramp_table[598] = dlsym(RTLD_NEXT, "removexattr");
    if (index == 598) return __bf_tramp_table[598];
    if (index == 599) { __bf_tramp_table[599] = (void*)(unsigned long long)__bf_tramp_table[599]; }
    if (index == 599) __bf_tramp_table[599] = dlsym(RTLD_NEXT, "setxattr");
    if (index == 599) return __bf_tramp_table[599];
    if (index == 600) { __bf_tramp_table[600] = (void*)(unsigned long long)__bf_tramp_table[600]; }
    if (index == 600) __bf_tramp_table[600] = dlsym(RTLD_NEXT, "chroot");
    if (index == 600) return __bf_tramp_table[600];
    if (index == 601) { __bf_tramp_table[601] = (void*)(unsigned long long)__bf_tramp_table[601]; }
    if (index == 601) __bf_tramp_table[601] = dlsym(RTLD_NEXT, "__pwrite64_chk");
    if (index == 601) return __bf_tramp_table[601];
    if (index == 602) { __bf_tramp_table[602] = (void*)(unsigned long long)__bf_tramp_table[602]; }
    if (index == 602) __bf_tramp_table[602] = dlsym(RTLD_NEXT, "fstat64");
    if (index == 602) return __bf_tramp_table[602];
    if (index == 603) { __bf_tramp_table[603] = (void*)(unsigned long long)__bf_tramp_table[603]; }
    if (index == 603) __bf_tramp_table[603] = dlsym(RTLD_NEXT, "creat");
    if (index == 603) return __bf_tramp_table[603];
    if (index == 604) { __bf_tramp_table[604] = (void*)(unsigned long long)__bf_tramp_table[604]; }
    if (index == 604) __bf_tramp_table[604] = dlsym(RTLD_NEXT, "stat64");
    if (index == 604) return __bf_tramp_table[604];
    if (index == 605) { __bf_tramp_table[605] = (void*)(unsigned long long)__bf_tramp_table[605]; }
    if (index == 605) __bf_tramp_table[605] = dlsym(RTLD_NEXT, "iswspace");
    if (index == 605) return __bf_tramp_table[605];
    if (index == 606) { __bf_tramp_table[606] = (void*)(unsigned long long)__bf_tramp_table[606]; }
    if (index == 606) __bf_tramp_table[606] = dlsym(RTLD_NEXT, "wcschr");
    if (index == 606) return __bf_tramp_table[606];
    if (index == 607) { __bf_tramp_table[607] = (void*)(unsigned long long)__bf_tramp_table[607]; }
    if (index == 607) __bf_tramp_table[607] = dlsym(RTLD_NEXT, "timegm");
    if (index == 607) return __bf_tramp_table[607];
    if (index == 608) { __bf_tramp_table[608] = (void*)(unsigned long long)__bf_tramp_table[608]; }
    if (index == 608) __bf_tramp_table[608] = dlsym(RTLD_NEXT, "personality");
    if (index == 608) return __bf_tramp_table[608];
    if (index == 609) { __bf_tramp_table[609] = (void*)(unsigned long long)__bf_tramp_table[609]; }
    if (index == 609) __bf_tramp_table[609] = dlsym(RTLD_NEXT, "ppoll");
    if (index == 609) return __bf_tramp_table[609];
    if (index == 610) { __bf_tramp_table[610] = (void*)(unsigned long long)__bf_tramp_table[610]; }
    if (index == 610) __bf_tramp_table[610] = dlsym(RTLD_NEXT, "logb");
    if (index == 610) return __bf_tramp_table[610];
    if (index == 611) { __bf_tramp_table[611] = (void*)(unsigned long long)__bf_tramp_table[611]; }
    if (index == 611) __bf_tramp_table[611] = dlsym(RTLD_NEXT, "scalbn");
    if (index == 611) return __bf_tramp_table[611];
    if (index == 612) { __bf_tramp_table[612] = (void*)(unsigned long long)__bf_tramp_table[612]; }
    if (index == 612) __bf_tramp_table[612] = dlsym(RTLD_NEXT, "scalbnf");
    if (index == 612) return __bf_tramp_table[612];
    if (index == 613) { __bf_tramp_table[613] = (void*)(unsigned long long)__bf_tramp_table[613]; }
    if (index == 613) __bf_tramp_table[613] = dlsym(RTLD_NEXT, "logbl");
    if (index == 613) return __bf_tramp_table[613];
    if (index == 614) { __bf_tramp_table[614] = (void*)(unsigned long long)__bf_tramp_table[614]; }
    if (index == 614) __bf_tramp_table[614] = dlsym(RTLD_NEXT, "scalbnl");
    if (index == 614) return __bf_tramp_table[614];
    if (index == 615) { __bf_tramp_table[615] = (void*)(unsigned long long)__bf_tramp_table[615]; }
    if (index == 615) __bf_tramp_table[615] = dlsym(RTLD_NEXT, "fmaxl");
    if (index == 615) return __bf_tramp_table[615];
    if (index == 616) { __bf_tramp_table[616] = (void*)(unsigned long long)__bf_tramp_table[616]; }
    if (index == 616) __bf_tramp_table[616] = dlsym(RTLD_NEXT, "putc");
    if (index == 616) return __bf_tramp_table[616];
    if (index == 617) { __bf_tramp_table[617] = (void*)(unsigned long long)__bf_tramp_table[617]; }
    if (index == 617) __bf_tramp_table[617] = dlsym(RTLD_NEXT, "rewinddir");
    if (index == 617) return __bf_tramp_table[617];
    if (index == 618) { __bf_tramp_table[618] = (void*)(unsigned long long)__bf_tramp_table[618]; }
    if (index == 618) __bf_tramp_table[618] = dlsym(RTLD_NEXT, "lockf");
    if (index == 618) return __bf_tramp_table[618];
    if (index == 619) { __bf_tramp_table[619] = (void*)(unsigned long long)__bf_tramp_table[619]; }
    if (index == 619) __bf_tramp_table[619] = dlsym(RTLD_NEXT, "__umask_chk");
    if (index == 619) return __bf_tramp_table[619];
    if (index == 620) { __bf_tramp_table[620] = (void*)(unsigned long long)__bf_tramp_table[620]; }
    if (index == 620) __bf_tramp_table[620] = dlsym(RTLD_NEXT, "getservbyname");
    if (index == 620) return __bf_tramp_table[620];
    if (index == 621) { __bf_tramp_table[621] = (void*)(unsigned long long)__bf_tramp_table[621]; }
    if (index == 621) __bf_tramp_table[621] = dlsym(RTLD_NEXT, "regcomp");
    if (index == 621) return __bf_tramp_table[621];
    if (index == 622) { __bf_tramp_table[622] = (void*)(unsigned long long)__bf_tramp_table[622]; }
    if (index == 622) __bf_tramp_table[622] = dlsym(RTLD_NEXT, "regerror");
    if (index == 622) return __bf_tramp_table[622];
    if (index == 623) { __bf_tramp_table[623] = (void*)(unsigned long long)__bf_tramp_table[623]; }
    if (index == 623) __bf_tramp_table[623] = dlsym(RTLD_NEXT, "regexec");
    if (index == 623) return __bf_tramp_table[623];
    if (index == 624) { __bf_tramp_table[624] = (void*)(unsigned long long)__bf_tramp_table[624]; }
    if (index == 624) __bf_tramp_table[624] = dlsym(RTLD_NEXT, "regfree");
    if (index == 624) return __bf_tramp_table[624];
    if (index == 625) { __bf_tramp_table[625] = (void*)(unsigned long long)__bf_tramp_table[625]; }
    if (index == 625) __bf_tramp_table[625] = dlsym(RTLD_NEXT, "__fwrite_chk");
    if (index == 625) return __bf_tramp_table[625];
    if (index == 626) { __bf_tramp_table[626] = (void*)(unsigned long long)__bf_tramp_table[626]; }
    if (index == 626) __bf_tramp_table[626] = dlsym(RTLD_NEXT, "getifaddrs");
    if (index == 626) return __bf_tramp_table[626];
    if (index == 627) { __bf_tramp_table[627] = (void*)(unsigned long long)__bf_tramp_table[627]; }
    if (index == 627) __bf_tramp_table[627] = dlsym(RTLD_NEXT, "freeifaddrs");
    if (index == 627) return __bf_tramp_table[627];
    if (index == 628) { __bf_tramp_table[628] = (void*)(unsigned long long)__bf_tramp_table[628]; }
    if (index == 628) __bf_tramp_table[628] = dlsym(RTLD_NEXT, "getpwuid_r");
    if (index == 628) return __bf_tramp_table[628];
    if (index == 629) { __bf_tramp_table[629] = (void*)(unsigned long long)__bf_tramp_table[629]; }
    if (index == 629) __bf_tramp_table[629] = dlsym(RTLD_NEXT, "android_fdsan_get_owner_tag");
    if (index == 629) return __bf_tramp_table[629];
    if (index == 630) { __bf_tramp_table[630] = (void*)(unsigned long long)__bf_tramp_table[630]; }
    if (index == 630) __bf_tramp_table[630] = dlsym(RTLD_NEXT, "__system_property_foreach");
    if (index == 630) return __bf_tramp_table[630];
    if (index == 631) { __bf_tramp_table[631] = (void*)(unsigned long long)__bf_tramp_table[631]; }
    if (index == 631) __bf_tramp_table[631] = dlsym(RTLD_NEXT, "strtoimax");
    if (index == 631) return __bf_tramp_table[631];
    if (index == 632) { __bf_tramp_table[632] = (void*)(unsigned long long)__bf_tramp_table[632]; }
    if (index == 632) __bf_tramp_table[632] = dlsym(RTLD_NEXT, "fstatat");
    if (index == 632) return __bf_tramp_table[632];
    if (index == 633) { __bf_tramp_table[633] = (void*)(unsigned long long)__bf_tramp_table[633]; }
    if (index == 633) __bf_tramp_table[633] = dlsym(RTLD_NEXT, "readdir_r");
    if (index == 633) return __bf_tramp_table[633];
    if (index == 634) { __bf_tramp_table[634] = (void*)(unsigned long long)__bf_tramp_table[634]; }
    if (index == 634) __bf_tramp_table[634] = dlsym(RTLD_NEXT, "mknod");
    if (index == 634) return __bf_tramp_table[634];
    if (index == 635) { __bf_tramp_table[635] = (void*)(unsigned long long)__bf_tramp_table[635]; }
    if (index == 635) __bf_tramp_table[635] = dlsym(RTLD_NEXT, "open_memstream");
    if (index == 635) return __bf_tramp_table[635];
    if (index == 636) { __bf_tramp_table[636] = (void*)(unsigned long long)__bf_tramp_table[636]; }
    if (index == 636) __bf_tramp_table[636] = dlsym(RTLD_NEXT, "vfork");
    if (index == 636) return __bf_tramp_table[636];
    if (index == 637) { __bf_tramp_table[637] = (void*)(unsigned long long)__bf_tramp_table[637]; }
    if (index == 637) __bf_tramp_table[637] = dlsym(RTLD_NEXT, "getprotobynumber");
    if (index == 637) return __bf_tramp_table[637];
    if (index == 638) { __bf_tramp_table[638] = (void*)(unsigned long long)__bf_tramp_table[638]; }
    if (index == 638) __bf_tramp_table[638] = dlsym(RTLD_NEXT, "__fgets_chk");
    if (index == 638) return __bf_tramp_table[638];
    if (index == 639) { __bf_tramp_table[639] = (void*)(unsigned long long)__bf_tramp_table[639]; }
    if (index == 639) __bf_tramp_table[639] = dlsym(RTLD_NEXT, "strtok");
    if (index == 639) return __bf_tramp_table[639];
    if (index == 640) { __bf_tramp_table[640] = (void*)(unsigned long long)__bf_tramp_table[640]; }
    if (index == 640) __bf_tramp_table[640] = dlsym(RTLD_NEXT, "ctime");
    if (index == 640) return __bf_tramp_table[640];
    if (index == 641) { __bf_tramp_table[641] = (void*)(unsigned long long)__bf_tramp_table[641]; }
    if (index == 641) __bf_tramp_table[641] = dlsym(RTLD_NEXT, "scandir");
    if (index == 641) return __bf_tramp_table[641];
    if (index == 642) { __bf_tramp_table[642] = (void*)(unsigned long long)__bf_tramp_table[642]; }
    if (index == 642) __bf_tramp_table[642] = dlsym(RTLD_NEXT, "alphasort");
    if (index == 642) return __bf_tramp_table[642];
    if (index == 643) { __bf_tramp_table[643] = (void*)(unsigned long long)__bf_tramp_table[643]; }
    if (index == 643) __bf_tramp_table[643] = dlsym(RTLD_NEXT, "rewind");
    if (index == 643) return __bf_tramp_table[643];
    if (index == 644) { __bf_tramp_table[644] = (void*)(unsigned long long)__bf_tramp_table[644]; }
    if (index == 644) __bf_tramp_table[644] = dlsym(RTLD_NEXT, "hasmntopt");
    if (index == 644) return __bf_tramp_table[644];
    if (index == 645) { __bf_tramp_table[645] = (void*)(unsigned long long)__bf_tramp_table[645]; }
    if (index == 645) __bf_tramp_table[645] = dlsym(RTLD_NEXT, "__pread64_chk");
    if (index == 645) return __bf_tramp_table[645];
    if (index == 646) { __bf_tramp_table[646] = (void*)(unsigned long long)__bf_tramp_table[646]; }
    if (index == 646) __bf_tramp_table[646] = dlsym(RTLD_NEXT, "posix_fadvise");
    if (index == 646) return __bf_tramp_table[646];
    if (index == 647) { __bf_tramp_table[647] = (void*)(unsigned long long)__bf_tramp_table[647]; }
    if (index == 647) __bf_tramp_table[647] = dlsym(RTLD_NEXT, "ftruncate64");
    if (index == 647) return __bf_tramp_table[647];
    if (index == 648) { __bf_tramp_table[648] = (void*)(unsigned long long)__bf_tramp_table[648]; }
    if (index == 648) __bf_tramp_table[648] = dlsym(RTLD_NEXT, "process_vm_readv");
    if (index == 648) return __bf_tramp_table[648];
    if (index == 649) { __bf_tramp_table[649] = (void*)(unsigned long long)__bf_tramp_table[649]; }
    if (index == 649) __bf_tramp_table[649] = dlsym(RTLD_NEXT, "umount");
    if (index == 649) return __bf_tramp_table[649];
    if (index == 650) { __bf_tramp_table[650] = (void*)(unsigned long long)__bf_tramp_table[650]; }
    if (index == 650) __bf_tramp_table[650] = dlsym(RTLD_NEXT, "swapon");
    if (index == 650) return __bf_tramp_table[650];
    if (index == 651) { __bf_tramp_table[651] = (void*)(unsigned long long)__bf_tramp_table[651]; }
    if (index == 651) __bf_tramp_table[651] = dlsym(RTLD_NEXT, "system");
    if (index == 651) return __bf_tramp_table[651];
    if (index == 652) { __bf_tramp_table[652] = (void*)(unsigned long long)__bf_tramp_table[652]; }
    if (index == 652) __bf_tramp_table[652] = dlsym(RTLD_NEXT, "fstatfs64");
    if (index == 652) return __bf_tramp_table[652];
    if (index == 653) { __bf_tramp_table[653] = (void*)(unsigned long long)__bf_tramp_table[653]; }
    if (index == 653) __bf_tramp_table[653] = dlsym(RTLD_NEXT, "sync");
    if (index == 653) return __bf_tramp_table[653];
    if (index == 654) { __bf_tramp_table[654] = (void*)(unsigned long long)__bf_tramp_table[654]; }
    if (index == 654) __bf_tramp_table[654] = dlsym(RTLD_NEXT, "posix_openpt");
    if (index == 654) return __bf_tramp_table[654];
    if (index == 655) { __bf_tramp_table[655] = (void*)(unsigned long long)__bf_tramp_table[655]; }
    if (index == 655) __bf_tramp_table[655] = dlsym(RTLD_NEXT, "grantpt");
    if (index == 655) return __bf_tramp_table[655];
    if (index == 656) { __bf_tramp_table[656] = (void*)(unsigned long long)__bf_tramp_table[656]; }
    if (index == 656) __bf_tramp_table[656] = dlsym(RTLD_NEXT, "unlockpt");
    if (index == 656) return __bf_tramp_table[656];
    if (index == 657) { __bf_tramp_table[657] = (void*)(unsigned long long)__bf_tramp_table[657]; }
    if (index == 657) __bf_tramp_table[657] = dlsym(RTLD_NEXT, "ptsname_r");
    if (index == 657) return __bf_tramp_table[657];
    if (index == 658) { __bf_tramp_table[658] = (void*)(unsigned long long)__bf_tramp_table[658]; }
    if (index == 658) __bf_tramp_table[658] = dlsym(RTLD_NEXT, "setsid");
    if (index == 658) return __bf_tramp_table[658];
    if (index == 659) { __bf_tramp_table[659] = (void*)(unsigned long long)__bf_tramp_table[659]; }
    if (index == 659) __bf_tramp_table[659] = dlsym(RTLD_NEXT, "android_link_namespaces");
    if (index == 659) return __bf_tramp_table[659];
    if (index == 660) { __bf_tramp_table[660] = (void*)(unsigned long long)__bf_tramp_table[660]; }
    if (index == 660) __bf_tramp_table[660] = dlsym(RTLD_NEXT, "android_create_namespace");
    if (index == 660) return __bf_tramp_table[660];
    if (index == 661) { __bf_tramp_table[661] = (void*)(unsigned long long)__bf_tramp_table[661]; }
    if (index == 661) __bf_tramp_table[661] = dlsym(RTLD_NEXT, "eventfd_read");
    if (index == 661) return __bf_tramp_table[661];
    if (index == 662) { __bf_tramp_table[662] = (void*)(unsigned long long)__bf_tramp_table[662]; }
    if (index == 662) __bf_tramp_table[662] = dlsym(RTLD_NEXT, "sem_clockwait");
    if (index == 662) return __bf_tramp_table[662];
    if (index == 663) { __bf_tramp_table[663] = (void*)(unsigned long long)__bf_tramp_table[663]; }
    if (index == 663) __bf_tramp_table[663] = dlsym(RTLD_NEXT, "asinh");
    if (index == 663) return __bf_tramp_table[663];
    if (index == 664) { __bf_tramp_table[664] = (void*)(unsigned long long)__bf_tramp_table[664]; }
    if (index == 664) __bf_tramp_table[664] = dlsym(RTLD_NEXT, "acosh");
    if (index == 664) return __bf_tramp_table[664];
    if (index == 665) { __bf_tramp_table[665] = (void*)(unsigned long long)__bf_tramp_table[665]; }
    if (index == 665) __bf_tramp_table[665] = dlsym(RTLD_NEXT, "atanh");
    if (index == 665) return __bf_tramp_table[665];
    if (index == 666) { __bf_tramp_table[666] = (void*)(unsigned long long)__bf_tramp_table[666]; }
    if (index == 666) __bf_tramp_table[666] = dlsym(RTLD_NEXT, "remainder");
    if (index == 666) return __bf_tramp_table[666];
    if (index == 667) { __bf_tramp_table[667] = (void*)(unsigned long long)__bf_tramp_table[667]; }
    if (index == 667) __bf_tramp_table[667] = dlsym(RTLD_NEXT, "getrandom");
    if (index == 667) return __bf_tramp_table[667];
    if (index == 668) { __bf_tramp_table[668] = (void*)(unsigned long long)__bf_tramp_table[668]; }
    if (index == 668) __bf_tramp_table[668] = dlsym(RTLD_NEXT, "erf");
    if (index == 668) return __bf_tramp_table[668];
    if (index == 669) { __bf_tramp_table[669] = (void*)(unsigned long long)__bf_tramp_table[669]; }
    if (index == 669) __bf_tramp_table[669] = dlsym(RTLD_NEXT, "asctime");
    if (index == 669) return __bf_tramp_table[669];
    if (index == 670) { __bf_tramp_table[670] = (void*)(unsigned long long)__bf_tramp_table[670]; }
    if (index == 670) __bf_tramp_table[670] = dlsym(RTLD_NEXT, "getprotobyname");
    if (index == 670) return __bf_tramp_table[670];
    if (index == 671) { __bf_tramp_table[671] = (void*)(unsigned long long)__bf_tramp_table[671]; }
    if (index == 671) __bf_tramp_table[671] = dlsym(RTLD_NEXT, "setns");
    if (index == 671) return __bf_tramp_table[671];
    if (index == 672) { __bf_tramp_table[672] = (void*)(unsigned long long)__bf_tramp_table[672]; }
    if (index == 672) __bf_tramp_table[672] = dlsym(RTLD_NEXT, "__gnu_basename");
    if (index == 672) return __bf_tramp_table[672];
    if (index == 673) { __bf_tramp_table[673] = (void*)(unsigned long long)__bf_tramp_table[673]; }
    if (index == 673) __bf_tramp_table[673] = dlsym(RTLD_NEXT, "fgetc");
    if (index == 673) return __bf_tramp_table[673];
    if (index == 674) { __bf_tramp_table[674] = (void*)(unsigned long long)__bf_tramp_table[674]; }
    if (index == 674) __bf_tramp_table[674] = dlsym(RTLD_NEXT, "glob");
    if (index == 674) return __bf_tramp_table[674];
    if (index == 675) { __bf_tramp_table[675] = (void*)(unsigned long long)__bf_tramp_table[675]; }
    if (index == 675) __bf_tramp_table[675] = dlsym(RTLD_NEXT, "globfree");
    if (index == 675) return __bf_tramp_table[675];
    if (index == 676) { __bf_tramp_table[676] = (void*)(unsigned long long)__bf_tramp_table[676]; }
    if (index == 676) __bf_tramp_table[676] = dlsym(RTLD_NEXT, "getgroups");
    if (index == 676) return __bf_tramp_table[676];
    if (index == 677) { __bf_tramp_table[677] = (void*)(unsigned long long)__bf_tramp_table[677]; }
    if (index == 677) __bf_tramp_table[677] = dlsym(RTLD_NEXT, "getopt_long_only");
    if (index == 677) return __bf_tramp_table[677];
    if (index == 678) { __bf_tramp_table[678] = (void*)(unsigned long long)__bf_tramp_table[678]; }
    if (index == 678) __bf_tramp_table[678] = dlsym(RTLD_NEXT, "inet_aton");
    if (index == 678) return __bf_tramp_table[678];
    if (index == 679) { __bf_tramp_table[679] = (void*)(unsigned long long)__bf_tramp_table[679]; }
    if (index == 679) __bf_tramp_table[679] = dlsym(RTLD_NEXT, "strcasestr");
    if (index == 679) return __bf_tramp_table[679];
    if (index == 680) { __bf_tramp_table[680] = (void*)(unsigned long long)__bf_tramp_table[680]; }
    if (index == 680) __bf_tramp_table[680] = dlsym(RTLD_NEXT, "inet_addr");
    if (index == 680) return __bf_tramp_table[680];
    if (index == 681) { __bf_tramp_table[681] = (void*)(unsigned long long)__bf_tramp_table[681]; }
    if (index == 681) __bf_tramp_table[681] = dlsym(RTLD_NEXT, "pthread_setschedprio");
    if (index == 681) return __bf_tramp_table[681];
    if (index == 682) { __bf_tramp_table[682] = (void*)(unsigned long long)__bf_tramp_table[682]; }
    if (index == 682) __bf_tramp_table[682] = dlsym(RTLD_NEXT, "pwrite64");
    if (index == 682) return __bf_tramp_table[682];
    if (index == 683) { __bf_tramp_table[683] = (void*)(unsigned long long)__bf_tramp_table[683]; }
    if (index == 683) __bf_tramp_table[683] = dlsym(RTLD_NEXT, "mmap64");
    if (index == 683) return __bf_tramp_table[683];
    if (index == 684) { __bf_tramp_table[684] = (void*)(unsigned long long)__bf_tramp_table[684]; }
    if (index == 684) __bf_tramp_table[684] = dlsym(RTLD_NEXT, "munlock");
    if (index == 684) return __bf_tramp_table[684];
    if (index == 685) { __bf_tramp_table[685] = (void*)(unsigned long long)__bf_tramp_table[685]; }
    if (index == 685) __bf_tramp_table[685] = dlsym(RTLD_NEXT, "malloc_backtrace");
    if (index == 685) return __bf_tramp_table[685];
    if (index == 686) { __bf_tramp_table[686] = (void*)(unsigned long long)__bf_tramp_table[686]; }
    if (index == 686) __bf_tramp_table[686] = dlsym(RTLD_NEXT, "malloc_disable");
    if (index == 686) return __bf_tramp_table[686];
    if (index == 687) { __bf_tramp_table[687] = (void*)(unsigned long long)__bf_tramp_table[687]; }
    if (index == 687) __bf_tramp_table[687] = dlsym(RTLD_NEXT, "malloc_enable");
    if (index == 687) return __bf_tramp_table[687];
    if (index == 688) { __bf_tramp_table[688] = (void*)(unsigned long long)__bf_tramp_table[688]; }
    if (index == 688) __bf_tramp_table[688] = dlsym(RTLD_NEXT, "malloc_iterate");
    if (index == 688) return __bf_tramp_table[688];
    if (index == 689) { __bf_tramp_table[689] = (void*)(unsigned long long)__bf_tramp_table[689]; }
    if (index == 689) __bf_tramp_table[689] = dlsym(RTLD_NEXT, "sethostname");
    if (index == 689) return __bf_tramp_table[689];
    if (index == 690) { __bf_tramp_table[690] = (void*)(unsigned long long)__bf_tramp_table[690]; }
    if (index == 690) __bf_tramp_table[690] = dlsym(RTLD_NEXT, "mknodat");
    if (index == 690) return __bf_tramp_table[690];
    if (index == 691) { __bf_tramp_table[691] = (void*)(unsigned long long)__bf_tramp_table[691]; }
    if (index == 691) __bf_tramp_table[691] = dlsym(RTLD_NEXT, "symlinkat");
    if (index == 691) return __bf_tramp_table[691];
    if (index == 692) { __bf_tramp_table[692] = (void*)(unsigned long long)__bf_tramp_table[692]; }
    if (index == 692) __bf_tramp_table[692] = dlsym(RTLD_NEXT, "fchdir");
    if (index == 692) return __bf_tramp_table[692];
    if (index == 693) { __bf_tramp_table[693] = (void*)(unsigned long long)__bf_tramp_table[693]; }
    if (index == 693) __bf_tramp_table[693] = dlsym(RTLD_NEXT, "initgroups");
    if (index == 693) return __bf_tramp_table[693];
    if (index == 694) { __bf_tramp_table[694] = (void*)(unsigned long long)__bf_tramp_table[694]; }
    if (index == 694) __bf_tramp_table[694] = dlsym(RTLD_NEXT, "sigtimedwait");
    if (index == 694) return __bf_tramp_table[694];
    if (index == 695) { __bf_tramp_table[695] = (void*)(unsigned long long)__bf_tramp_table[695]; }
    if (index == 695) __bf_tramp_table[695] = dlsym(RTLD_NEXT, "__libc_current_sigrtmax");
    if (index == 695) return __bf_tramp_table[695];
    if (index == 696) { __bf_tramp_table[696] = (void*)(unsigned long long)__bf_tramp_table[696]; }
    if (index == 696) __bf_tramp_table[696] = dlsym(RTLD_NEXT, "fexecve");
    if (index == 696) return __bf_tramp_table[696];
    if (index == 697) { __bf_tramp_table[697] = (void*)(unsigned long long)__bf_tramp_table[697]; }
    if (index == 697) __bf_tramp_table[697] = dlsym(RTLD_NEXT, "__libc_current_sigrtmin");
    if (index == 697) return __bf_tramp_table[697];
    if (index == 698) { __bf_tramp_table[698] = (void*)(unsigned long long)__bf_tramp_table[698]; }
    if (index == 698) __bf_tramp_table[698] = dlsym(RTLD_NEXT, "prlimit");
    if (index == 698) return __bf_tramp_table[698];
    if (index == 699) { __bf_tramp_table[699] = (void*)(unsigned long long)__bf_tramp_table[699]; }
    if (index == 699) __bf_tramp_table[699] = dlsym(RTLD_NEXT, "vsyslog");
    if (index == 699) return __bf_tramp_table[699];
    if (index == 700) { __bf_tramp_table[700] = (void*)(unsigned long long)__bf_tramp_table[700]; }
    if (index == 700) __bf_tramp_table[700] = dlsym(RTLD_NEXT, "vdprintf");
    if (index == 700) return __bf_tramp_table[700];
    if (index == 701) { __bf_tramp_table[701] = (void*)(unsigned long long)__bf_tramp_table[701]; }
    if (index == 701) __bf_tramp_table[701] = dlsym(RTLD_NEXT, "posix_madvise");
    if (index == 701) return __bf_tramp_table[701];
    if (index == 702) { __bf_tramp_table[702] = (void*)(unsigned long long)__bf_tramp_table[702]; }
    if (index == 702) __bf_tramp_table[702] = dlsym(RTLD_NEXT, "android_set_application_target_sdk_version");
    if (index == 702) return __bf_tramp_table[702];
    if (index == 703) { __bf_tramp_table[703] = (void*)(unsigned long long)__bf_tramp_table[703]; }
    if (index == 703) __bf_tramp_table[703] = dlsym(RTLD_NEXT, "res_mkquery");
    if (index == 703) return __bf_tramp_table[703];
    if (index == 704) { __bf_tramp_table[704] = (void*)(unsigned long long)__bf_tramp_table[704]; }
    if (index == 704) __bf_tramp_table[704] = dlsym(RTLD_NEXT, "__b64_ntop");
    if (index == 704) return __bf_tramp_table[704];
    if (index == 705) { __bf_tramp_table[705] = (void*)(unsigned long long)__bf_tramp_table[705]; }
    if (index == 705) __bf_tramp_table[705] = dlsym(RTLD_NEXT, "vfscanf");
    if (index == 705) return __bf_tramp_table[705];
    if (index == 706) { __bf_tramp_table[706] = (void*)(unsigned long long)__bf_tramp_table[706]; }
    if (index == 706) __bf_tramp_table[706] = dlsym(RTLD_NEXT, "clock_getcpuclockid");
    if (index == 706) return __bf_tramp_table[706];
    if (index == 707) { __bf_tramp_table[707] = (void*)(unsigned long long)__bf_tramp_table[707]; }
    if (index == 707) __bf_tramp_table[707] = dlsym(RTLD_NEXT, "clock_getres");
    if (index == 707) return __bf_tramp_table[707];
    if (index == 708) { __bf_tramp_table[708] = (void*)(unsigned long long)__bf_tramp_table[708]; }
    if (index == 708) __bf_tramp_table[708] = dlsym(RTLD_NEXT, "clock_settime");
    if (index == 708) return __bf_tramp_table[708];
    if (index == 709) { __bf_tramp_table[709] = (void*)(unsigned long long)__bf_tramp_table[709]; }
    if (index == 709) __bf_tramp_table[709] = dlsym(RTLD_NEXT, "getgrgid_r");
    if (index == 709) return __bf_tramp_table[709];
    if (index == 710) { __bf_tramp_table[710] = (void*)(unsigned long long)__bf_tramp_table[710]; }
    if (index == 710) __bf_tramp_table[710] = dlsym(RTLD_NEXT, "sigwait");
    if (index == 710) return __bf_tramp_table[710];
    if (index == 711) { __bf_tramp_table[711] = (void*)(unsigned long long)__bf_tramp_table[711]; }
    if (index == 711) __bf_tramp_table[711] = dlsym(RTLD_NEXT, "sigsuspend");
    if (index == 711) return __bf_tramp_table[711];
    if (index == 712) { __bf_tramp_table[712] = (void*)(unsigned long long)__bf_tramp_table[712]; }
    if (index == 712) __bf_tramp_table[712] = dlsym(RTLD_NEXT, "getgrouplist");
    if (index == 712) return __bf_tramp_table[712];
    if (index == 713) { __bf_tramp_table[713] = (void*)(unsigned long long)__bf_tramp_table[713]; }
    if (index == 713) __bf_tramp_table[713] = dlsym(RTLD_NEXT, "daemon");
    if (index == 713) return __bf_tramp_table[713];
    if (index == 714) { __bf_tramp_table[714] = (void*)(unsigned long long)__bf_tramp_table[714]; }
    if (index == 714) __bf_tramp_table[714] = dlsym(RTLD_NEXT, "setfsgid");
    if (index == 714) return __bf_tramp_table[714];
    if (index == 715) { __bf_tramp_table[715] = (void*)(unsigned long long)__bf_tramp_table[715]; }
    if (index == 715) __bf_tramp_table[715] = dlsym(RTLD_NEXT, "setfsuid");
    if (index == 715) return __bf_tramp_table[715];
    if (index == 716) { __bf_tramp_table[716] = (void*)(unsigned long long)__bf_tramp_table[716]; }
    if (index == 716) __bf_tramp_table[716] = dlsym(RTLD_NEXT, "process_vm_writev");
    if (index == 716) return __bf_tramp_table[716];
    if (index == 717) { __bf_tramp_table[717] = (void*)(unsigned long long)__bf_tramp_table[717]; }
    if (index == 717) __bf_tramp_table[717] = dlsym(RTLD_NEXT, "munlockall");
    if (index == 717) return __bf_tramp_table[717];
    if (index == 718) { __bf_tramp_table[718] = (void*)(unsigned long long)__bf_tramp_table[718]; }
    if (index == 718) __bf_tramp_table[718] = dlsym(RTLD_NEXT, "mlockall");
    if (index == 718) return __bf_tramp_table[718];
    if (index == 719) { __bf_tramp_table[719] = (void*)(unsigned long long)__bf_tramp_table[719]; }
    if (index == 719) __bf_tramp_table[719] = dlsym(RTLD_NEXT, "umask");
    if (index == 719) return __bf_tramp_table[719];
    if (index == 720) { __bf_tramp_table[720] = (void*)(unsigned long long)__bf_tramp_table[720]; }
    if (index == 720) __bf_tramp_table[720] = dlsym(RTLD_NEXT, "recv");
    if (index == 720) return __bf_tramp_table[720];
    if (index == 721) { __bf_tramp_table[721] = (void*)(unsigned long long)__bf_tramp_table[721]; }
    if (index == 721) __bf_tramp_table[721] = dlsym(RTLD_NEXT, "send");
    if (index == 721) return __bf_tramp_table[721];
    if (index == 722) { __bf_tramp_table[722] = (void*)(unsigned long long)__bf_tramp_table[722]; }
    if (index == 722) __bf_tramp_table[722] = dlsym(RTLD_NEXT, "cfsetspeed");
    if (index == 722) return __bf_tramp_table[722];
    if (index == 723) { __bf_tramp_table[723] = (void*)(unsigned long long)__bf_tramp_table[723]; }
    if (index == 723) __bf_tramp_table[723] = dlsym(RTLD_NEXT, "cfgetispeed");
    if (index == 723) return __bf_tramp_table[723];
    if (index == 724) { __bf_tramp_table[724] = (void*)(unsigned long long)__bf_tramp_table[724]; }
    if (index == 724) __bf_tramp_table[724] = dlsym(RTLD_NEXT, "cfgetospeed");
    if (index == 724) return __bf_tramp_table[724];
    if (index == 725) { __bf_tramp_table[725] = (void*)(unsigned long long)__bf_tramp_table[725]; }
    if (index == 725) __bf_tramp_table[725] = dlsym(RTLD_NEXT, "cfsetispeed");
    if (index == 725) return __bf_tramp_table[725];
    if (index == 726) { __bf_tramp_table[726] = (void*)(unsigned long long)__bf_tramp_table[726]; }
    if (index == 726) __bf_tramp_table[726] = dlsym(RTLD_NEXT, "cfsetospeed");
    if (index == 726) return __bf_tramp_table[726];
    if (index == 727) { __bf_tramp_table[727] = (void*)(unsigned long long)__bf_tramp_table[727]; }
    if (index == 727) __bf_tramp_table[727] = dlsym(RTLD_NEXT, "cfmakeraw");
    if (index == 727) return __bf_tramp_table[727];
    if (index == 728) { __bf_tramp_table[728] = (void*)(unsigned long long)__bf_tramp_table[728]; }
    if (index == 728) __bf_tramp_table[728] = dlsym(RTLD_NEXT, "getnetbyname");
    if (index == 728) return __bf_tramp_table[728];
    if (index == 729) { __bf_tramp_table[729] = (void*)(unsigned long long)__bf_tramp_table[729]; }
    if (index == 729) __bf_tramp_table[729] = dlsym(RTLD_NEXT, "iswdigit");
    if (index == 729) return __bf_tramp_table[729];
    if (index == 730) { __bf_tramp_table[730] = (void*)(unsigned long long)__bf_tramp_table[730]; }
    if (index == 730) __bf_tramp_table[730] = dlsym(RTLD_NEXT, "wait4");
    if (index == 730) return __bf_tramp_table[730];
    if (index == 731) { __bf_tramp_table[731] = (void*)(unsigned long long)__bf_tramp_table[731]; }
    if (index == 731) __bf_tramp_table[731] = dlsym(RTLD_NEXT, "getpgid");
    if (index == 731) return __bf_tramp_table[731];
    if (index == 732) { __bf_tramp_table[732] = (void*)(unsigned long long)__bf_tramp_table[732]; }
    if (index == 732) __bf_tramp_table[732] = dlsym(RTLD_NEXT, "lchown");
    if (index == 732) return __bf_tramp_table[732];
    if (index == 733) { __bf_tramp_table[733] = (void*)(unsigned long long)__bf_tramp_table[733]; }
    if (index == 733) __bf_tramp_table[733] = dlsym(RTLD_NEXT, "nanf");
    if (index == 733) return __bf_tramp_table[733];
    if (index == 734) { __bf_tramp_table[734] = (void*)(unsigned long long)__bf_tramp_table[734]; }
    if (index == 734) __bf_tramp_table[734] = dlsym(RTLD_NEXT, "strptime");
    if (index == 734) return __bf_tramp_table[734];
    if (index == 735) { __bf_tramp_table[735] = (void*)(unsigned long long)__bf_tramp_table[735]; }
    if (index == 735) __bf_tramp_table[735] = dlsym(RTLD_NEXT, "sem_timedwait");
    if (index == 735) return __bf_tramp_table[735];
    if (index == 736) { __bf_tramp_table[736] = (void*)(unsigned long long)__bf_tramp_table[736]; }
    if (index == 736) __bf_tramp_table[736] = dlsym(RTLD_NEXT, "frexpl");
    if (index == 736) return __bf_tramp_table[736];
    if (index == 737) { __bf_tramp_table[737] = (void*)(unsigned long long)__bf_tramp_table[737]; }
    if (index == 737) __bf_tramp_table[737] = dlsym(RTLD_NEXT, "ldexpl");
    if (index == 737) return __bf_tramp_table[737];
    if (index == 738) { __bf_tramp_table[738] = (void*)(unsigned long long)__bf_tramp_table[738]; }
    if (index == 738) __bf_tramp_table[738] = dlsym(RTLD_NEXT, "__recvfrom_chk");
    if (index == 738) return __bf_tramp_table[738];
    if (index == 739) { __bf_tramp_table[739] = (void*)(unsigned long long)__bf_tramp_table[739]; }
    if (index == 739) __bf_tramp_table[739] = dlsym(RTLD_NEXT, "readdir64");
    if (index == 739) return __bf_tramp_table[739];
    if (index == 740) { __bf_tramp_table[740] = (void*)(unsigned long long)__bf_tramp_table[740]; }
    if (index == 740) __bf_tramp_table[740] = dlsym(RTLD_NEXT, "readlinkat");
    if (index == 740) return __bf_tramp_table[740];
    if (index == 741) { __bf_tramp_table[741] = (void*)(unsigned long long)__bf_tramp_table[741]; }
    if (index == 741) __bf_tramp_table[741] = dlsym(RTLD_NEXT, "flistxattr");
    if (index == 741) return __bf_tramp_table[741];
    if (index == 742) { __bf_tramp_table[742] = (void*)(unsigned long long)__bf_tramp_table[742]; }
    if (index == 742) __bf_tramp_table[742] = dlsym(RTLD_NEXT, "llistxattr");
    if (index == 742) return __bf_tramp_table[742];
    if (index == 743) { __bf_tramp_table[743] = (void*)(unsigned long long)__bf_tramp_table[743]; }
    if (index == 743) __bf_tramp_table[743] = dlsym(RTLD_NEXT, "lremovexattr");
    if (index == 743) return __bf_tramp_table[743];
    if (index == 744) { __bf_tramp_table[744] = (void*)(unsigned long long)__bf_tramp_table[744]; }
    if (index == 744) __bf_tramp_table[744] = dlsym(RTLD_NEXT, "linkat");
    if (index == 744) return __bf_tramp_table[744];
    if (index == 745) { __bf_tramp_table[745] = (void*)(unsigned long long)__bf_tramp_table[745]; }
    if (index == 745) __bf_tramp_table[745] = dlsym(RTLD_NEXT, "openat64");
    if (index == 745) return __bf_tramp_table[745];
    if (index == 746) { __bf_tramp_table[746] = (void*)(unsigned long long)__bf_tramp_table[746]; }
    if (index == 746) __bf_tramp_table[746] = dlsym(RTLD_NEXT, "fstatat64");
    if (index == 746) return __bf_tramp_table[746];
    if (index == 747) { __bf_tramp_table[747] = (void*)(unsigned long long)__bf_tramp_table[747]; }
    if (index == 747) __bf_tramp_table[747] = dlsym(RTLD_NEXT, "fchownat");
    if (index == 747) return __bf_tramp_table[747];
    if (index == 748) { __bf_tramp_table[748] = (void*)(unsigned long long)__bf_tramp_table[748]; }
    if (index == 748) __bf_tramp_table[748] = dlsym(RTLD_NEXT, "posix_fadvise64");
    if (index == 748) return __bf_tramp_table[748];
    if (index == 749) { __bf_tramp_table[749] = (void*)(unsigned long long)__bf_tramp_table[749]; }
    if (index == 749) __bf_tramp_table[749] = dlsym(RTLD_NEXT, "fstatvfs64");
    if (index == 749) return __bf_tramp_table[749];
    if (index == 750) { __bf_tramp_table[750] = (void*)(unsigned long long)__bf_tramp_table[750]; }
    if (index == 750) __bf_tramp_table[750] = dlsym(RTLD_NEXT, "renameat");
    if (index == 750) return __bf_tramp_table[750];
    if (index == 751) { __bf_tramp_table[751] = (void*)(unsigned long long)__bf_tramp_table[751]; }
    if (index == 751) __bf_tramp_table[751] = dlsym(RTLD_NEXT, "fallocate64");
    if (index == 751) return __bf_tramp_table[751];
    if (index == 752) { __bf_tramp_table[752] = (void*)(unsigned long long)__bf_tramp_table[752]; }
    if (index == 752) __bf_tramp_table[752] = dlsym(RTLD_NEXT, "lgetxattr");
    if (index == 752) return __bf_tramp_table[752];
    if (index == 753) { __bf_tramp_table[753] = (void*)(unsigned long long)__bf_tramp_table[753]; }
    if (index == 753) __bf_tramp_table[753] = dlsym(RTLD_NEXT, "listxattr");
    if (index == 753) return __bf_tramp_table[753];
    if (index == 754) { __bf_tramp_table[754] = (void*)(unsigned long long)__bf_tramp_table[754]; }
    if (index == 754) __bf_tramp_table[754] = dlsym(RTLD_NEXT, "lsetxattr");
    if (index == 754) return __bf_tramp_table[754];
    if (index == 755) { __bf_tramp_table[755] = (void*)(unsigned long long)__bf_tramp_table[755]; }
    if (index == 755) __bf_tramp_table[755] = dlsym(RTLD_NEXT, "pause");
    if (index == 755) return __bf_tramp_table[755];
    if (index == 756) { __bf_tramp_table[756] = (void*)(unsigned long long)__bf_tramp_table[756]; }
    if (index == 756) __bf_tramp_table[756] = dlsym(RTLD_NEXT, "__stpcpy_chk");
    if (index == 756) return __bf_tramp_table[756];
    if (index == 757) { __bf_tramp_table[757] = (void*)(unsigned long long)__bf_tramp_table[757]; }
    if (index == 757) __bf_tramp_table[757] = dlsym(RTLD_NEXT, "__fsetlocking");
    if (index == 757) return __bf_tramp_table[757];
    if (index == 758) { __bf_tramp_table[758] = (void*)(unsigned long long)__bf_tramp_table[758]; }
    if (index == 758) __bf_tramp_table[758] = dlsym(RTLD_NEXT, "__mempcpy_chk");
    if (index == 758) return __bf_tramp_table[758];
    if (index == 759) { __bf_tramp_table[759] = (void*)(unsigned long long)__bf_tramp_table[759]; }
    if (index == 759) __bf_tramp_table[759] = dlsym(RTLD_NEXT, "reallocarray");
    if (index == 759) return __bf_tramp_table[759];
    if (index == 760) { __bf_tramp_table[760] = (void*)(unsigned long long)__bf_tramp_table[760]; }
    if (index == 760) __bf_tramp_table[760] = dlsym(RTLD_NEXT, "__system_properties_init");
    if (index == 760) return __bf_tramp_table[760];
    if (index == 761) { __bf_tramp_table[761] = (void*)(unsigned long long)__bf_tramp_table[761]; }
    if (index == 761) __bf_tramp_table[761] = dlsym(RTLD_NEXT, "fts_open");
    if (index == 761) return __bf_tramp_table[761];
    if (index == 762) { __bf_tramp_table[762] = (void*)(unsigned long long)__bf_tramp_table[762]; }
    if (index == 762) __bf_tramp_table[762] = dlsym(RTLD_NEXT, "fts_read");
    if (index == 762) return __bf_tramp_table[762];
    if (index == 763) { __bf_tramp_table[763] = (void*)(unsigned long long)__bf_tramp_table[763]; }
    if (index == 763) __bf_tramp_table[763] = dlsym(RTLD_NEXT, "fts_set");
    if (index == 763) return __bf_tramp_table[763];
    if (index == 764) { __bf_tramp_table[764] = (void*)(unsigned long long)__bf_tramp_table[764]; }
    if (index == 764) __bf_tramp_table[764] = dlsym(RTLD_NEXT, "fts_close");
    if (index == 764) return __bf_tramp_table[764];
    if (index == 765) { __bf_tramp_table[765] = (void*)(unsigned long long)__bf_tramp_table[765]; }
    if (index == 765) __bf_tramp_table[765] = dlsym(RTLD_NEXT, "stpcpy");
    if (index == 765) return __bf_tramp_table[765];
    if (index == 766) { __bf_tramp_table[766] = (void*)(unsigned long long)__bf_tramp_table[766]; }
    if (index == 766) __bf_tramp_table[766] = dlsym(RTLD_NEXT, "popen");
    if (index == 766) return __bf_tramp_table[766];
    if (index == 767) { __bf_tramp_table[767] = (void*)(unsigned long long)__bf_tramp_table[767]; }
    if (index == 767) __bf_tramp_table[767] = dlsym(RTLD_NEXT, "pclose");
    if (index == 767) return __bf_tramp_table[767];
    if (index == 768) { __bf_tramp_table[768] = (void*)(unsigned long long)__bf_tramp_table[768]; }
    if (index == 768) __bf_tramp_table[768] = dlsym(RTLD_NEXT, "pwritev");
    if (index == 768) return __bf_tramp_table[768];
    if (index == 769) { __bf_tramp_table[769] = (void*)(unsigned long long)__bf_tramp_table[769]; }
    if (index == 769) __bf_tramp_table[769] = dlsym(RTLD_NEXT, "fpathconf");
    if (index == 769) return __bf_tramp_table[769];
    if (index == 770) { __bf_tramp_table[770] = (void*)(unsigned long long)__bf_tramp_table[770]; }
    if (index == 770) __bf_tramp_table[770] = dlsym(RTLD_NEXT, "memmem");
    if (index == 770) return __bf_tramp_table[770];
    if (index == 771) { __bf_tramp_table[771] = (void*)(unsigned long long)__bf_tramp_table[771]; }
    if (index == 771) __bf_tramp_table[771] = dlsym(RTLD_NEXT, "strchrnul");
    if (index == 771) return __bf_tramp_table[771];
    if (index == 772) { __bf_tramp_table[772] = (void*)(unsigned long long)__bf_tramp_table[772]; }
    if (index == 772) __bf_tramp_table[772] = dlsym(RTLD_NEXT, "open64");
    if (index == 772) return __bf_tramp_table[772];
    if (index == 773) { __bf_tramp_table[773] = (void*)(unsigned long long)__bf_tramp_table[773]; }
    if (index == 773) __bf_tramp_table[773] = dlsym(RTLD_NEXT, "mkfifo");
    if (index == 773) return __bf_tramp_table[773];
    if (index == 774) { __bf_tramp_table[774] = (void*)(unsigned long long)__bf_tramp_table[774]; }
    if (index == 774) __bf_tramp_table[774] = dlsym(RTLD_NEXT, "preadv");
    if (index == 774) return __bf_tramp_table[774];
    if (index == 775) { __bf_tramp_table[775] = (void*)(unsigned long long)__bf_tramp_table[775]; }
    if (index == 775) __bf_tramp_table[775] = dlsym(RTLD_NEXT, "splice");
    if (index == 775) return __bf_tramp_table[775];
    if (index == 776) { __bf_tramp_table[776] = (void*)(unsigned long long)__bf_tramp_table[776]; }
    if (index == 776) __bf_tramp_table[776] = dlsym(RTLD_NEXT, "copy_file_range");
    if (index == 776) return __bf_tramp_table[776];
    if (index == 777) { __bf_tramp_table[777] = (void*)(unsigned long long)__bf_tramp_table[777]; }
    if (index == 777) __bf_tramp_table[777] = dlsym(RTLD_NEXT, "fseeko64");
    if (index == 777) return __bf_tramp_table[777];
    if (index == 778) { __bf_tramp_table[778] = (void*)(unsigned long long)__bf_tramp_table[778]; }
    if (index == 778) __bf_tramp_table[778] = dlsym(RTLD_NEXT, "ftello64");
    if (index == 778) return __bf_tramp_table[778];
    if (index == 779) { __bf_tramp_table[779] = (void*)(unsigned long long)__bf_tramp_table[779]; }
    if (index == 779) __bf_tramp_table[779] = dlsym(RTLD_NEXT, "sem_trywait");
    if (index == 779) return __bf_tramp_table[779];
    if (index == 780) { __bf_tramp_table[780] = (void*)(unsigned long long)__bf_tramp_table[780]; }
    if (index == 780) __bf_tramp_table[780] = dlsym(RTLD_NEXT, "pthread_mutex_timedlock");
    if (index == 780) return __bf_tramp_table[780];
    if (index == 781) { __bf_tramp_table[781] = (void*)(unsigned long long)__bf_tramp_table[781]; }
    if (index == 781) __bf_tramp_table[781] = dlsym(RTLD_NEXT, "strerrorname_np");
    if (index == 781) return __bf_tramp_table[781];
    if (index == 782) { __bf_tramp_table[782] = (void*)(unsigned long long)__bf_tramp_table[782]; }
    if (index == 782) __bf_tramp_table[782] = dlsym(RTLD_NEXT, "clone3");
    if (index == 782) return __bf_tramp_table[782];
    if (index == 783) { __bf_tramp_table[783] = (void*)(unsigned long long)__bf_tramp_table[783]; }
    if (index == 783) __bf_tramp_table[783] = dlsym(RTLD_NEXT, "setregid");
    if (index == 783) return __bf_tramp_table[783];
    if (index == 784) { __bf_tramp_table[784] = (void*)(unsigned long long)__bf_tramp_table[784]; }
    if (index == 784) __bf_tramp_table[784] = dlsym(RTLD_NEXT, "setreuid");
    if (index == 784) return __bf_tramp_table[784];
    return NULL;
}


/* ===== pthread_mutex_t ABI compatibility fix =====
 *
 * GSI libraries compiled for Bionic may have pthread_mutex_t structs
 * with different field layouts or initialization than glibc expects.
 *
 * glibc 2.43 ARM64 __pthread_mutex_lock disassembly shows the fast path
 * check: tst w1, #0x7c (test __kind bits 2..6). If ANY of these bits are
 * set, glibc branches to __pthread_mutex_lock_full which may trigger
 * __pthread_tpp_change_priority().
 *
 * glibc ARM64 struct __pthread_mutex_s layout:
 *   offset 0:  int __lock        (4 bytes)  — low bits = futex state
 *   offset 4:  unsigned int __count    (4 bytes)  — recursion count
 *   offset 8:  int __owner       (4 bytes)  — owning thread ID
 *   offset 12: unsigned int __nusers  (4 bytes)  — waiters count
 *   offset 16: int __kind        (4 bytes)  — mutex type in bits 0-1
 *   offset 20: int __spins       (4 bytes)
 *   offset 24: __pthread_list_t __list    (16 bytes, two pointers)
 *
 * glibc checks __kind bits 2..6 (mask 0x7c) to decide fast vs full path.
 * Glibc's PTHREAD_MUTEX_NORMAL (type=0) with no flags set passes this check.
 * Any non-zero in bits 2..6 means the mutex has protocol flags or extended
 * type bits set, triggering lock_full which may call TPP.
 *
 * The wrapper intercepts all mutex lock functions to sanitize __kind
 * before forwarding to the real glibc implementation.
 */
#define MUTEX_KIND_FLAG_MASK 0x7c  /* The bits that trigger lock_full */

/* Forward declarations */
typedef int (*pthread_mutex_lock_fn_t)(pthread_mutex_t *mutex);
typedef int (*pthread_mutex_trylock_fn_t)(pthread_mutex_t *mutex);
typedef int (*pthread_mutex_timedlock_fn_t)(pthread_mutex_t *mutex, const struct timespec *abstime);
typedef int (*pthread_mutex_init_fn_t)(pthread_mutex_t *mutex, const pthread_mutexattr_t *attr);

static pthread_mutex_lock_fn_t real_pthread_mutex_lock = NULL;
static pthread_mutex_trylock_fn_t real_pthread_mutex_trylock = NULL;
static pthread_mutex_timedlock_fn_t real_pthread_mutex_timedlock = NULL;

/* Sanitize __kind field to clear bits 2..6 that would trigger lock_full */
static inline void sanitize_mutex(pthread_mutex_t *mutex) {
    int *kind_ptr = (int *)((char *)mutex + 16);
    int kind = *kind_ptr;
    if (kind & MUTEX_KIND_FLAG_MASK) {
        /* Zero only the type-flag bits, keep the mutex type (bits 0-1) */
        *kind_ptr = kind & ~MUTEX_KIND_FLAG_MASK;
    }
}

static int bf_pthread_mutex_lock_wrapper(pthread_mutex_t *mutex) {
    if (mutex) sanitize_mutex(mutex);
    return real_pthread_mutex_lock(mutex);
}

static int bf_pthread_mutex_trylock_wrapper(pthread_mutex_t *mutex) {
    if (mutex) sanitize_mutex(mutex);
    return real_pthread_mutex_trylock(mutex);
}

static int bf_pthread_mutex_timedlock_wrapper(pthread_mutex_t *mutex, const struct timespec *abstime) {
    if (mutex) sanitize_mutex(mutex);
    return real_pthread_mutex_timedlock(mutex, abstime);
}

/* ===== Init function (called from JNI shim) ===== */
__attribute__((visibility("default")))
void __bf_init_data(void) {
    void *self = dlopen(NULL, RTLD_LAZY);
    if (!self) return;
    if (!__bf_data_stderr)
        *(void **)(__bf_data_stderr) = dlsym(self, "stderr");
    if (!__bf_data___sF)
        *(void **)(__bf_data___sF) = dlsym(self, "_IO_2_1_stderr_");
    if (!__bf_data_optarg)
        *(void **)(__bf_data_optarg) = dlsym(self, "optarg");
    if (!__bf_data_optind)
        *(void **)(__bf_data_optind) = dlsym(self, "optind");
    if (!__bf_data_tzname)
        *(void **)(__bf_data_tzname) = dlsym(self, "tzname");
    if (!__bf_data_daylight)
        *(void **)(__bf_data_daylight) = dlsym(self, "daylight");
    if (!__bf_data_timezone)
        *(void **)(__bf_data_timezone) = dlsym(self, "timezone");
    if (!__bf_data_environ)
        *(void **)(__bf_data_environ) = dlsym(self, "environ");
    if (!__bf_data_in6addr_any)
        *(void **)(__bf_data_in6addr_any) = dlsym(self, "in6addr_any");
    if (!__bf_data_stdin)
        *(void **)(__bf_data_stdin) = dlsym(self, "stdin");
    if (!__bf_data_stdout)
        *(void **)(__bf_data_stdout) = dlsym(self, "stdout");
    if (!__bf_data_in6addr_loopback)
        *(void **)(__bf_data_in6addr_loopback) = dlsym(self, "in6addr_loopback");
    if (!__bf_data___stack_chk_guard)
        *(void **)(__bf_data___stack_chk_guard) = dlsym(self, "__stack_chk_guard");
    if (!__bf_data_signgam)
        *(void **)(__bf_data_signgam) = dlsym(self, "signgam");
    dlclose(self);

    /* Install mutex wrappers to fix ABI mismatches between Bionic and glibc.
     *
     * GSI libraries compiled for Bionic may have pthread_mutex_t __kind
     * fields with type-flag bits (bits 2..6) set that glibc interprets as
     * protocol flags (PTHREAD_PRIO_PROTECT, etc.). When glibc sees these
     * bits, it takes the lock_full path which may call __pthread_tpp_change_priority()
     * and assertion-fail on non-RT threads.
     *
     * Index 38 = pthread_mutex_lock
     * Index 124 = pthread_mutex_trylock
     * Index 780 = pthread_mutex_timedlock
     */
    if (!real_pthread_mutex_lock) {
        real_pthread_mutex_lock = (pthread_mutex_lock_fn_t)__bf_c_resolve(38);
        __bf_tramp_table[38] = (void *)bf_pthread_mutex_lock_wrapper;
    }
    if (!real_pthread_mutex_trylock) {
        real_pthread_mutex_trylock = (pthread_mutex_trylock_fn_t)__bf_c_resolve(124);
        __bf_tramp_table[124] = (void *)bf_pthread_mutex_trylock_wrapper;
    }
    if (!real_pthread_mutex_timedlock) {
        real_pthread_mutex_timedlock = (pthread_mutex_timedlock_fn_t)__bf_c_resolve(780);
        __bf_tramp_table[780] = (void *)bf_pthread_mutex_timedlock_wrapper;
    }
}