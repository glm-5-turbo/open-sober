/* Auto-generated bionic shim C helpers. DO NOT EDIT.
 * 392 lazy-resolve entries, 13 data ptrs.
 */
#define _GNU_SOURCE
#include <stddef.h>
#include <dlfcn.h>

/* Dispatch table and data — defined in bionic_shim.S */
extern void *__bf_tramp_table[392];
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
        register long r8a asm("x8") = 63;
        asm volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r8a));
        register long c0 asm("x0") = fd;
        register long r8b asm("x8") = 57;
        asm volatile("svc #0" : : "r"(c0), "r"(r8b));
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
    if (index < 0 || index >= 392)
        return NULL;
    if (index == 0) { __bf_tramp_table[0] = (void*)(unsigned long long)__bf_tramp_table[0]; }
    if (index == 0) __bf_tramp_table[0] = dlsym(RTLD_DEFAULT, "__cxa_finalize");
    if (index == 0) return __bf_tramp_table[0];
    if (index == 1) { __bf_tramp_table[1] = (void*)(unsigned long long)__bf_tramp_table[1]; }
    if (index == 1) __bf_tramp_table[1] = dlsym(RTLD_DEFAULT, "__cxa_atexit");
    if (index == 1) return __bf_tramp_table[1];
    if (index == 2) { __bf_tramp_table[2] = (void*)(unsigned long long)__bf_tramp_table[2]; }
    if (index == 2) __bf_tramp_table[2] = dlsym(RTLD_DEFAULT, "__register_atfork");
    if (index == 2) return __bf_tramp_table[2];
    if (index == 3) { __bf_tramp_table[3] = (void*)(unsigned long long)__bf_tramp_table[3]; }
    if (index == 3) __bf_tramp_table[3] = dlsym(RTLD_DEFAULT, "strlen");
    if (index == 3) return __bf_tramp_table[3];
    if (index == 4) { __bf_tramp_table[4] = (void*)(unsigned long long)__bf_tramp_table[4]; }
    if (index == 4) __bf_tramp_table[4] = dlsym(RTLD_DEFAULT, "memcmp");
    if (index == 4) return __bf_tramp_table[4];
    if (index == 5) { __bf_tramp_table[5] = (void*)(unsigned long long)__bf_tramp_table[5]; }
    if (index == 5) __bf_tramp_table[5] = dlsym(RTLD_DEFAULT, "pthread_mutex_init");
    if (index == 5) return __bf_tramp_table[5];
    if (index == 6) { __bf_tramp_table[6] = (void*)(unsigned long long)__bf_tramp_table[6]; }
    if (index == 6) __bf_tramp_table[6] = dlsym(RTLD_DEFAULT, "pthread_mutex_destroy");
    if (index == 6) return __bf_tramp_table[6];
    if (index == 7) { __bf_tramp_table[7] = (void*)(unsigned long long)__bf_tramp_table[7]; }
    if (index == 7) __bf_tramp_table[7] = dlsym(RTLD_DEFAULT, "pthread_once");
    if (index == 7) return __bf_tramp_table[7];
    if (index == 8) { __bf_tramp_table[8] = (void*)(unsigned long long)__bf_tramp_table[8]; }
    if (index == 8) __bf_tramp_table[8] = dlsym(RTLD_DEFAULT, "__memset_chk");
    if (index == 8) return __bf_tramp_table[8];
    if (index == 9) { __bf_tramp_table[9] = (void*)(unsigned long long)__bf_tramp_table[9]; }
    if (index == 9) __bf_tramp_table[9] = dlsym(RTLD_DEFAULT, "__memcpy_chk");
    if (index == 9) return __bf_tramp_table[9];
    if (index == 10) { __bf_tramp_table[10] = (void*)(unsigned long long)__bf_tramp_table[10]; }
    if (index == 10) __bf_tramp_table[10] = dlsym(RTLD_DEFAULT, "strlen");
    if (index == 10) return __bf_tramp_table[10];
    if (index == 11) { __bf_tramp_table[11] = (void*)(unsigned long long)__bf_tramp_table[11]; }
    if (index == 11) __bf_tramp_table[11] = dlsym(RTLD_DEFAULT, "memchr");
    if (index == 11) return __bf_tramp_table[11];
    if (index == 12) { __bf_tramp_table[12] = (void*)(unsigned long long)__bf_tramp_table[12]; }
    if (index == 12) __bf_tramp_table[12] = dlsym(RTLD_DEFAULT, "dladdr");
    if (index == 12) return __bf_tramp_table[12];
    if (index == 13) { __bf_tramp_table[13] = (void*)(unsigned long long)__bf_tramp_table[13]; }
    if (index == 13) __bf_tramp_table[13] = dlsym(RTLD_DEFAULT, "dlerror");
    if (index == 13) return __bf_tramp_table[13];
    if (index == 14) { __bf_tramp_table[14] = (void*)(unsigned long long)__bf_tramp_table[14]; }
    if (index == 14) __bf_tramp_table[14] = dlsym(RTLD_DEFAULT, "dlopen");
    if (index == 14) return __bf_tramp_table[14];
    if (index == 15) { __bf_tramp_table[15] = (void*)(unsigned long long)__bf_tramp_table[15]; }
    if (index == 15) __bf_tramp_table[15] = dlsym(RTLD_DEFAULT, "dlsym");
    if (index == 15) return __bf_tramp_table[15];
    if (index == 16) { __bf_tramp_table[16] = (void*)(unsigned long long)__bf_tramp_table[16]; }
    if (index == 16) __bf_tramp_table[16] = dlsym(RTLD_DEFAULT, "strncmp");
    if (index == 16) return __bf_tramp_table[16];
    if (index == 17) { __bf_tramp_table[17] = (void*)(unsigned long long)__bf_tramp_table[17]; }
    if (index == 17) __bf_tramp_table[17] = dlsym(RTLD_DEFAULT, "strcmp");
    if (index == 17) return __bf_tramp_table[17];
    if (index == 18) { __bf_tramp_table[18] = (void*)(unsigned long long)__bf_tramp_table[18]; }
    if (index == 18) __bf_tramp_table[18] = dlsym(RTLD_DEFAULT, "getauxval");
    if (index == 18) return __bf_tramp_table[18];
    if (index == 19) { __bf_tramp_table[19] = (void*)(unsigned long long)__bf_tramp_table[19]; }
    if (index == 19) __bf_tramp_table[19] = dlsym(RTLD_DEFAULT, "__errno_location");
    if (index == 19) return __bf_tramp_table[19];
    if (index == 20) { __bf_tramp_table[20] = (void*)(unsigned long long)__bf_tramp_table[20]; }
    if (index == 20) __bf_tramp_table[20] = dlsym(RTLD_DEFAULT, "close");
    if (index == 20) return __bf_tramp_table[20];
    if (index == 21) { __bf_tramp_table[21] = (void*)(unsigned long long)__bf_tramp_table[21]; }
    if (index == 21) __bf_tramp_table[21] = dlsym(RTLD_DEFAULT, "__open_2");
    if (index == 21) return __bf_tramp_table[21];
    if (index == 22) { __bf_tramp_table[22] = (void*)(unsigned long long)__bf_tramp_table[22]; }
    if (index == 22) __bf_tramp_table[22] = dlsym(RTLD_DEFAULT, "__read_chk");
    if (index == 22) return __bf_tramp_table[22];
    if (index == 23) { __bf_tramp_table[23] = (void*)(unsigned long long)__bf_tramp_table[23]; }
    if (index == 23) __bf_tramp_table[23] = dlsym(RTLD_DEFAULT, "read");
    if (index == 23) return __bf_tramp_table[23];
    if (index == 24) { __bf_tramp_table[24] = (void*)(unsigned long long)__bf_tramp_table[24]; }
    if (index == 24) __bf_tramp_table[24] = dlsym(RTLD_DEFAULT, "clock_gettime");
    if (index == 24) return __bf_tramp_table[24];
    if (index == 25) { __bf_tramp_table[25] = (void*)(unsigned long long)__bf_tramp_table[25]; }
    if (index == 25) __bf_tramp_table[25] = dlsym(RTLD_DEFAULT, "syscall");
    if (index == 25) return __bf_tramp_table[25];
    if (index == 26) { __bf_tramp_table[26] = (void*)(unsigned long long)__bf_tramp_table[26]; }
    if (index == 26) __bf_tramp_table[26] = dlsym(RTLD_DEFAULT, "sched_getcpu");
    if (index == 26) return __bf_tramp_table[26];
    if (index == 27) { __bf_tramp_table[27] = (void*)(unsigned long long)__bf_tramp_table[27]; }
    if (index == 27) __bf_tramp_table[27] = dlsym(RTLD_DEFAULT, "sysconf");
    if (index == 27) return __bf_tramp_table[27];
    if (index == 28) { __bf_tramp_table[28] = (void*)(unsigned long long)__bf_tramp_table[28]; }
    if (index == 28) __bf_tramp_table[28] = dlsym(RTLD_DEFAULT, "mmap");
    if (index == 28) return __bf_tramp_table[28];
    if (index == 29) { __bf_tramp_table[29] = (void*)(unsigned long long)__bf_tramp_table[29]; }
    if (index == 29) __bf_tramp_table[29] = dlsym(RTLD_DEFAULT, "mprotect");
    if (index == 29) return __bf_tramp_table[29];
    if (index == 30) { __bf_tramp_table[30] = (void*)(unsigned long long)__bf_tramp_table[30]; }
    if (index == 30) __bf_tramp_table[30] = dlsym(RTLD_DEFAULT, "munmap");
    if (index == 30) return __bf_tramp_table[30];
    if (index == 31) { __bf_tramp_table[31] = (void*)(unsigned long long)__bf_tramp_table[31]; }
    if (index == 31) __bf_tramp_table[31] = dlsym(RTLD_DEFAULT, "pthread_attr_init");
    if (index == 31) return __bf_tramp_table[31];
    if (index == 32) { __bf_tramp_table[32] = (void*)(unsigned long long)__bf_tramp_table[32]; }
    if (index == 32) __bf_tramp_table[32] = dlsym(RTLD_DEFAULT, "pthread_attr_setstacksize");
    if (index == 32) return __bf_tramp_table[32];
    if (index == 33) { __bf_tramp_table[33] = (void*)(unsigned long long)__bf_tramp_table[33]; }
    if (index == 33) __bf_tramp_table[33] = dlsym(RTLD_DEFAULT, "pthread_create");
    if (index == 33) return __bf_tramp_table[33];
    if (index == 34) { __bf_tramp_table[34] = (void*)(unsigned long long)__bf_tramp_table[34]; }
    if (index == 34) __bf_tramp_table[34] = dlsym(RTLD_DEFAULT, "pthread_attr_destroy");
    if (index == 34) return __bf_tramp_table[34];
    if (index == 35) { __bf_tramp_table[35] = (void*)(unsigned long long)__bf_tramp_table[35]; }
    if (index == 35) __bf_tramp_table[35] = dlsym(RTLD_DEFAULT, "pthread_join");
    if (index == 35) return __bf_tramp_table[35];
    if (index == 36) { __bf_tramp_table[36] = (void*)(unsigned long long)__bf_tramp_table[36]; }
    if (index == 36) __bf_tramp_table[36] = dlsym(RTLD_DEFAULT, "pthread_self");
    if (index == 36) return __bf_tramp_table[36];
    if (index == 37) { __bf_tramp_table[37] = (void*)(unsigned long long)__bf_tramp_table[37]; }
    if (index == 37) __bf_tramp_table[37] = dlsym(RTLD_DEFAULT, "memset");
    if (index == 37) return __bf_tramp_table[37];
    if (index == 38) { __bf_tramp_table[38] = (void*)(unsigned long long)__bf_tramp_table[38]; }
    if (index == 38) __bf_tramp_table[38] = dlsym(RTLD_DEFAULT, "pthread_setspecific");
    if (index == 38) return __bf_tramp_table[38];
    if (index == 39) { __bf_tramp_table[39] = (void*)(unsigned long long)__bf_tramp_table[39]; }
    if (index == 39) __bf_tramp_table[39] = dlsym(RTLD_DEFAULT, "__strncpy_chk");
    if (index == 39) return __bf_tramp_table[39];
    if (index == 40) { __bf_tramp_table[40] = (void*)(unsigned long long)__bf_tramp_table[40]; }
    if (index == 40) __bf_tramp_table[40] = dlsym(RTLD_DEFAULT, "sched_get_priority_max");
    if (index == 40) return __bf_tramp_table[40];
    if (index == 41) { __bf_tramp_table[41] = (void*)(unsigned long long)__bf_tramp_table[41]; }
    if (index == 41) __bf_tramp_table[41] = dlsym(RTLD_DEFAULT, "sched_setscheduler");
    if (index == 41) return __bf_tramp_table[41];
    if (index == 42) { __bf_tramp_table[42] = (void*)(unsigned long long)__bf_tramp_table[42]; }
    if (index == 42) __bf_tramp_table[42] = dlsym(RTLD_DEFAULT, "pthread_mutex_lock");
    if (index == 42) return __bf_tramp_table[42];
    if (index == 43) { __bf_tramp_table[43] = (void*)(unsigned long long)__bf_tramp_table[43]; }
    if (index == 43) __bf_tramp_table[43] = dlsym(RTLD_DEFAULT, "pthread_cond_wait");
    if (index == 43) return __bf_tramp_table[43];
    if (index == 44) { __bf_tramp_table[44] = (void*)(unsigned long long)__bf_tramp_table[44]; }
    if (index == 44) __bf_tramp_table[44] = dlsym(RTLD_DEFAULT, "pthread_mutex_unlock");
    if (index == 44) return __bf_tramp_table[44];
    if (index == 45) { __bf_tramp_table[45] = (void*)(unsigned long long)__bf_tramp_table[45]; }
    if (index == 45) __bf_tramp_table[45] = dlsym(RTLD_DEFAULT, "pthread_cond_signal");
    if (index == 45) return __bf_tramp_table[45];
    if (index == 46) { __bf_tramp_table[46] = (void*)(unsigned long long)__bf_tramp_table[46]; }
    if (index == 46) __bf_tramp_table[46] = dlsym(RTLD_DEFAULT, "pthread_cond_init");
    if (index == 46) return __bf_tramp_table[46];
    if (index == 47) { __bf_tramp_table[47] = (void*)(unsigned long long)__bf_tramp_table[47]; }
    if (index == 47) __bf_tramp_table[47] = dlsym(RTLD_DEFAULT, "gettid");
    if (index == 47) return __bf_tramp_table[47];
    if (index == 48) { __bf_tramp_table[48] = (void*)(unsigned long long)__bf_tramp_table[48]; }
    if (index == 48) __bf_tramp_table[48] = dlsym(RTLD_DEFAULT, "__vsprintf_chk");
    if (index == 48) return __bf_tramp_table[48];
    if (index == 49) { __bf_tramp_table[49] = (void*)(unsigned long long)__bf_tramp_table[49]; }
    if (index == 49) __bf_tramp_table[49] = dlsym(RTLD_DEFAULT, "atan2f");
    if (index == 49) return __bf_tramp_table[49];
    if (index == 50) { __bf_tramp_table[50] = (void*)(unsigned long long)__bf_tramp_table[50]; }
    if (index == 50) __bf_tramp_table[50] = dlsym(RTLD_DEFAULT, "vsnprintf");
    if (index == 50) return __bf_tramp_table[50];
    if (index == 51) { __bf_tramp_table[51] = (void*)(unsigned long long)__bf_tramp_table[51]; }
    if (index == 51) __bf_tramp_table[51] = dlsym(RTLD_DEFAULT, "fprintf");
    if (index == 51) return __bf_tramp_table[51];
    if (index == 52) { __bf_tramp_table[52] = (void*)(unsigned long long)__bf_tramp_table[52]; }
    if (index == 52) __bf_tramp_table[52] = dlsym(RTLD_DEFAULT, "rand");
    if (index == 52) return __bf_tramp_table[52];
    if (index == 53) { __bf_tramp_table[53] = (void*)(unsigned long long)__bf_tramp_table[53]; }
    if (index == 53) __bf_tramp_table[53] = dlsym(RTLD_DEFAULT, "strtoll");
    if (index == 53) return __bf_tramp_table[53];
    if (index == 54) { __bf_tramp_table[54] = (void*)(unsigned long long)__bf_tramp_table[54]; }
    if (index == 54) __bf_tramp_table[54] = dlsym(RTLD_DEFAULT, "time");
    if (index == 54) return __bf_tramp_table[54];
    if (index == 55) { __bf_tramp_table[55] = (void*)(unsigned long long)__bf_tramp_table[55]; }
    if (index == 55) __bf_tramp_table[55] = dlsym(RTLD_DEFAULT, "pthread_cond_broadcast");
    if (index == 55) return __bf_tramp_table[55];
    if (index == 56) { __bf_tramp_table[56] = (void*)(unsigned long long)__bf_tramp_table[56]; }
    if (index == 56) __bf_tramp_table[56] = dlsym(RTLD_DEFAULT, "pthread_cond_destroy");
    if (index == 56) return __bf_tramp_table[56];
    if (index == 57) { __bf_tramp_table[57] = (void*)(unsigned long long)__bf_tramp_table[57]; }
    if (index == 57) __bf_tramp_table[57] = dlsym(RTLD_DEFAULT, "stat");
    if (index == 57) return __bf_tramp_table[57];
    if (index == 58) { __bf_tramp_table[58] = (void*)(unsigned long long)__bf_tramp_table[58]; }
    if (index == 58) __bf_tramp_table[58] = dlsym(RTLD_DEFAULT, "opendir");
    if (index == 58) return __bf_tramp_table[58];
    if (index == 59) { __bf_tramp_table[59] = (void*)(unsigned long long)__bf_tramp_table[59]; }
    if (index == 59) __bf_tramp_table[59] = dlsym(RTLD_DEFAULT, "readdir");
    if (index == 59) return __bf_tramp_table[59];
    if (index == 60) { __bf_tramp_table[60] = (void*)(unsigned long long)__bf_tramp_table[60]; }
    if (index == 60) __bf_tramp_table[60] = dlsym(RTLD_DEFAULT, "closedir");
    if (index == 60) return __bf_tramp_table[60];
    if (index == 61) { __bf_tramp_table[61] = (void*)(unsigned long long)__bf_tramp_table[61]; }
    if (index == 61) __bf_tramp_table[61] = dlsym(RTLD_DEFAULT, "posix_fallocate");
    if (index == 61) return __bf_tramp_table[61];
    if (index == 62) { __bf_tramp_table[62] = (void*)(unsigned long long)__bf_tramp_table[62]; }
    if (index == 62) __bf_tramp_table[62] = dlsym(RTLD_DEFAULT, "open");
    if (index == 62) return __bf_tramp_table[62];
    if (index == 63) { __bf_tramp_table[63] = (void*)(unsigned long long)__bf_tramp_table[63]; }
    if (index == 63) __bf_tramp_table[63] = dlsym(RTLD_DEFAULT, "pthread_equal");
    if (index == 63) return __bf_tramp_table[63];
    if (index == 64) { __bf_tramp_table[64] = (void*)(unsigned long long)__bf_tramp_table[64]; }
    if (index == 64) __bf_tramp_table[64] = dlsym(RTLD_DEFAULT, "atoll");
    if (index == 64) return __bf_tramp_table[64];
    if (index == 65) { __bf_tramp_table[65] = (void*)(unsigned long long)__bf_tramp_table[65]; }
    if (index == 65) __bf_tramp_table[65] = dlsym(RTLD_DEFAULT, "srand");
    if (index == 65) return __bf_tramp_table[65];
    if (index == 66) { __bf_tramp_table[66] = (void*)(unsigned long long)__bf_tramp_table[66]; }
    if (index == 66) __bf_tramp_table[66] = dlsym(RTLD_DEFAULT, "strtod");
    if (index == 66) return __bf_tramp_table[66];
    if (index == 67) { __bf_tramp_table[67] = (void*)(unsigned long long)__bf_tramp_table[67]; }
    if (index == 67) __bf_tramp_table[67] = dlsym(RTLD_DEFAULT, "localtime");
    if (index == 67) return __bf_tramp_table[67];
    if (index == 68) { __bf_tramp_table[68] = (void*)(unsigned long long)__bf_tramp_table[68]; }
    if (index == 68) __bf_tramp_table[68] = dlsym(RTLD_DEFAULT, "fseek");
    if (index == 68) return __bf_tramp_table[68];
    if (index == 69) { __bf_tramp_table[69] = (void*)(unsigned long long)__bf_tramp_table[69]; }
    if (index == 69) __bf_tramp_table[69] = dlsym(RTLD_DEFAULT, "ftell");
    if (index == 69) return __bf_tramp_table[69];
    if (index == 70) { __bf_tramp_table[70] = (void*)(unsigned long long)__bf_tramp_table[70]; }
    if (index == 70) __bf_tramp_table[70] = dlsym(RTLD_DEFAULT, "fclose");
    if (index == 70) return __bf_tramp_table[70];
    if (index == 71) { __bf_tramp_table[71] = (void*)(unsigned long long)__bf_tramp_table[71]; }
    if (index == 71) __bf_tramp_table[71] = dlsym(RTLD_DEFAULT, "atan");
    if (index == 71) return __bf_tramp_table[71];
    if (index == 72) { __bf_tramp_table[72] = (void*)(unsigned long long)__bf_tramp_table[72]; }
    if (index == 72) __bf_tramp_table[72] = dlsym(RTLD_DEFAULT, "mkdir");
    if (index == 72) return __bf_tramp_table[72];
    if (index == 73) { __bf_tramp_table[73] = (void*)(unsigned long long)__bf_tramp_table[73]; }
    if (index == 73) __bf_tramp_table[73] = dlsym(RTLD_DEFAULT, "__fread_chk");
    if (index == 73) return __bf_tramp_table[73];
    if (index == 74) { __bf_tramp_table[74] = (void*)(unsigned long long)__bf_tramp_table[74]; }
    if (index == 74) __bf_tramp_table[74] = dlsym(RTLD_DEFAULT, "fread");
    if (index == 74) return __bf_tramp_table[74];
    if (index == 75) { __bf_tramp_table[75] = (void*)(unsigned long long)__bf_tramp_table[75]; }
    if (index == 75) __bf_tramp_table[75] = dlsym(RTLD_DEFAULT, "asinf");
    if (index == 75) return __bf_tramp_table[75];
    if (index == 76) { __bf_tramp_table[76] = (void*)(unsigned long long)__bf_tramp_table[76]; }
    if (index == 76) __bf_tramp_table[76] = dlsym(RTLD_DEFAULT, "dlclose");
    if (index == 76) return __bf_tramp_table[76];
    if (index == 77) { __bf_tramp_table[77] = (void*)(unsigned long long)__bf_tramp_table[77]; }
    if (index == 77) __bf_tramp_table[77] = dlsym(RTLD_DEFAULT, "access");
    if (index == 77) return __bf_tramp_table[77];
    if (index == 78) { __bf_tramp_table[78] = (void*)(unsigned long long)__bf_tramp_table[78]; }
    if (index == 78) __bf_tramp_table[78] = dlsym(RTLD_DEFAULT, "getenv");
    if (index == 78) return __bf_tramp_table[78];
    if (index == 79) { __bf_tramp_table[79] = (void*)(unsigned long long)__bf_tramp_table[79]; }
    if (index == 79) __bf_tramp_table[79] = dlsym(RTLD_DEFAULT, "strncpy");
    if (index == 79) return __bf_tramp_table[79];
    if (index == 80) { __bf_tramp_table[80] = (void*)(unsigned long long)__bf_tramp_table[80]; }
    if (index == 80) __bf_tramp_table[80] = dlsym(RTLD_DEFAULT, "feof");
    if (index == 80) return __bf_tramp_table[80];
    if (index == 81) { __bf_tramp_table[81] = (void*)(unsigned long long)__bf_tramp_table[81]; }
    if (index == 81) __bf_tramp_table[81] = dlsym(RTLD_DEFAULT, "fopen");
    if (index == 81) return __bf_tramp_table[81];
    if (index == 82) { __bf_tramp_table[82] = (void*)(unsigned long long)__bf_tramp_table[82]; }
    if (index == 82) __bf_tramp_table[82] = dlsym(RTLD_DEFAULT, "fseeko");
    if (index == 82) return __bf_tramp_table[82];
    if (index == 83) { __bf_tramp_table[83] = (void*)(unsigned long long)__bf_tramp_table[83]; }
    if (index == 83) __bf_tramp_table[83] = dlsym(RTLD_DEFAULT, "ftello");
    if (index == 83) return __bf_tramp_table[83];
    if (index == 84) { __bf_tramp_table[84] = (void*)(unsigned long long)__bf_tramp_table[84]; }
    if (index == 84) __bf_tramp_table[84] = dlsym(RTLD_DEFAULT, "fwrite");
    if (index == 84) return __bf_tramp_table[84];
    if (index == 85) { __bf_tramp_table[85] = (void*)(unsigned long long)__bf_tramp_table[85]; }
    if (index == 85) __bf_tramp_table[85] = dlsym(RTLD_DEFAULT, "fflush");
    if (index == 85) return __bf_tramp_table[85];
    if (index == 86) { __bf_tramp_table[86] = (void*)(unsigned long long)__bf_tramp_table[86]; }
    if (index == 86) __bf_tramp_table[86] = dlsym(RTLD_DEFAULT, "gmtime");
    if (index == 86) return __bf_tramp_table[86];
    if (index == 87) { __bf_tramp_table[87] = (void*)(unsigned long long)__bf_tramp_table[87]; }
    if (index == 87) __bf_tramp_table[87] = dlsym(RTLD_DEFAULT, "mktime");
    if (index == 87) return __bf_tramp_table[87];
    if (index == 88) { __bf_tramp_table[88] = (void*)(unsigned long long)__bf_tramp_table[88]; }
    if (index == 88) __bf_tramp_table[88] = dlsym(RTLD_DEFAULT, "pipe");
    if (index == 88) return __bf_tramp_table[88];
    if (index == 89) { __bf_tramp_table[89] = (void*)(unsigned long long)__bf_tramp_table[89]; }
    if (index == 89) __bf_tramp_table[89] = dlsym(RTLD_DEFAULT, "write");
    if (index == 89) return __bf_tramp_table[89];
    if (index == 90) { __bf_tramp_table[90] = (void*)(unsigned long long)__bf_tramp_table[90]; }
    if (index == 90) __bf_tramp_table[90] = dlsym(RTLD_DEFAULT, "pthread_key_create");
    if (index == 90) return __bf_tramp_table[90];
    if (index == 91) { __bf_tramp_table[91] = (void*)(unsigned long long)__bf_tramp_table[91]; }
    if (index == 91) __bf_tramp_table[91] = dlsym(RTLD_DEFAULT, "abort");
    if (index == 91) return __bf_tramp_table[91];
    if (index == 92) { __bf_tramp_table[92] = (void*)(unsigned long long)__bf_tramp_table[92]; }
    if (index == 92) __bf_tramp_table[92] = dlsym(RTLD_DEFAULT, "__assert");
    if (index == 92) return __bf_tramp_table[92];
    if (index == 93) { __bf_tramp_table[93] = (void*)(unsigned long long)__bf_tramp_table[93]; }
    if (index == 93) __bf_tramp_table[93] = dlsym(RTLD_DEFAULT, "strrchr");
    if (index == 93) return __bf_tramp_table[93];
    if (index == 94) { __bf_tramp_table[94] = (void*)(unsigned long long)__bf_tramp_table[94]; }
    if (index == 94) __bf_tramp_table[94] = dlsym(RTLD_DEFAULT, "__stack_chk_fail");
    if (index == 94) return __bf_tramp_table[94];
    if (index == 95) { __bf_tramp_table[95] = (void*)(unsigned long long)__bf_tramp_table[95]; }
    if (index == 95) __bf_tramp_table[95] = dlsym(RTLD_DEFAULT, "atoi");
    if (index == 95) return __bf_tramp_table[95];
    if (index == 96) { __bf_tramp_table[96] = (void*)(unsigned long long)__bf_tramp_table[96]; }
    if (index == 96) __bf_tramp_table[96] = dlsym(RTLD_DEFAULT, "fcntl");
    if (index == 96) return __bf_tramp_table[96];
    if (index == 97) { __bf_tramp_table[97] = (void*)(unsigned long long)__bf_tramp_table[97]; }
    if (index == 97) __bf_tramp_table[97] = dlsym(RTLD_DEFAULT, "memcpy");
    if (index == 97) return __bf_tramp_table[97];
    if (index == 98) { __bf_tramp_table[98] = (void*)(unsigned long long)__bf_tramp_table[98]; }
    if (index == 98) __bf_tramp_table[98] = dlsym(RTLD_DEFAULT, "strcpy");
    if (index == 98) return __bf_tramp_table[98];
    if (index == 99) { __bf_tramp_table[99] = (void*)(unsigned long long)__bf_tramp_table[99]; }
    if (index == 99) __bf_tramp_table[99] = dlsym(RTLD_DEFAULT, "strerror");
    if (index == 99) return __bf_tramp_table[99];
    if (index == 100) { __bf_tramp_table[100] = (void*)(unsigned long long)__bf_tramp_table[100]; }
    if (index == 100) __bf_tramp_table[100] = dlsym(RTLD_DEFAULT, "pthread_attr_setdetachstate");
    if (index == 100) return __bf_tramp_table[100];
    if (index == 101) { __bf_tramp_table[101] = (void*)(unsigned long long)__bf_tramp_table[101]; }
    if (index == 101) __bf_tramp_table[101] = dlsym(RTLD_DEFAULT, "pthread_cond_timedwait");
    if (index == 101) return __bf_tramp_table[101];
    if (index == 102) { __bf_tramp_table[102] = (void*)(unsigned long long)__bf_tramp_table[102]; }
    if (index == 102) __bf_tramp_table[102] = dlsym(RTLD_DEFAULT, "memmove");
    if (index == 102) return __bf_tramp_table[102];
    if (index == 103) { __bf_tramp_table[103] = (void*)(unsigned long long)__bf_tramp_table[103]; }
    if (index == 103) __bf_tramp_table[103] = dlsym(RTLD_DEFAULT, "strtol");
    if (index == 103) return __bf_tramp_table[103];
    if (index == 104) { __bf_tramp_table[104] = (void*)(unsigned long long)__bf_tramp_table[104]; }
    if (index == 104) __bf_tramp_table[104] = dlsym(RTLD_DEFAULT, "getpid");
    if (index == 104) return __bf_tramp_table[104];
    if (index == 105) { __bf_tramp_table[105] = (void*)(unsigned long long)__bf_tramp_table[105]; }
    if (index == 105) __bf_tramp_table[105] = dlsym(RTLD_DEFAULT, "gettimeofday");
    if (index == 105) return __bf_tramp_table[105];
    if (index == 106) { __bf_tramp_table[106] = (void*)(unsigned long long)__bf_tramp_table[106]; }
    if (index == 106) __bf_tramp_table[106] = dlsym(RTLD_DEFAULT, "localtime_r");
    if (index == 106) return __bf_tramp_table[106];
    if (index == 107) { __bf_tramp_table[107] = (void*)(unsigned long long)__bf_tramp_table[107]; }
    if (index == 107) __bf_tramp_table[107] = dlsym(RTLD_DEFAULT, "fputs");
    if (index == 107) return __bf_tramp_table[107];
    if (index == 108) { __bf_tramp_table[108] = (void*)(unsigned long long)__bf_tramp_table[108]; }
    if (index == 108) __bf_tramp_table[108] = dlsym(RTLD_DEFAULT, "strerror_r");
    if (index == 108) return __bf_tramp_table[108];
    if (index == 109) { __bf_tramp_table[109] = (void*)(unsigned long long)__bf_tramp_table[109]; }
    if (index == 109) __bf_tramp_table[109] = dlsym(RTLD_DEFAULT, "snprintf");
    if (index == 109) return __bf_tramp_table[109];
    if (index == 110) { __bf_tramp_table[110] = (void*)(unsigned long long)__bf_tramp_table[110]; }
    if (index == 110) __bf_tramp_table[110] = dlsym(RTLD_DEFAULT, "prctl");
    if (index == 110) return __bf_tramp_table[110];
    if (index == 111) { __bf_tramp_table[111] = (void*)(unsigned long long)__bf_tramp_table[111]; }
    if (index == 111) __bf_tramp_table[111] = dlsym(RTLD_DEFAULT, "sigaltstack");
    if (index == 111) return __bf_tramp_table[111];
    if (index == 112) { __bf_tramp_table[112] = (void*)(unsigned long long)__bf_tramp_table[112]; }
    if (index == 112) __bf_tramp_table[112] = dlsym(RTLD_DEFAULT, "getpagesize");
    if (index == 112) return __bf_tramp_table[112];
    if (index == 113) { __bf_tramp_table[113] = (void*)(unsigned long long)__bf_tramp_table[113]; }
    if (index == 113) __bf_tramp_table[113] = dlsym(RTLD_DEFAULT, "pthread_getspecific");
    if (index == 113) return __bf_tramp_table[113];
    if (index == 114) { __bf_tramp_table[114] = (void*)(unsigned long long)__bf_tramp_table[114]; }
    if (index == 114) __bf_tramp_table[114] = dlsym(RTLD_DEFAULT, "fork");
    if (index == 114) return __bf_tramp_table[114];
    if (index == 115) { __bf_tramp_table[115] = (void*)(unsigned long long)__bf_tramp_table[115]; }
    if (index == 115) __bf_tramp_table[115] = dlsym(RTLD_DEFAULT, "waitpid");
    if (index == 115) return __bf_tramp_table[115];
    if (index == 116) { __bf_tramp_table[116] = (void*)(unsigned long long)__bf_tramp_table[116]; }
    if (index == 116) __bf_tramp_table[116] = dlsym(RTLD_DEFAULT, "execv");
    if (index == 116) return __bf_tramp_table[116];
    if (index == 117) { __bf_tramp_table[117] = (void*)(unsigned long long)__bf_tramp_table[117]; }
    if (index == 117) __bf_tramp_table[117] = dlsym(RTLD_DEFAULT, "_exit");
    if (index == 117) return __bf_tramp_table[117];
    if (index == 118) { __bf_tramp_table[118] = (void*)(unsigned long long)__bf_tramp_table[118]; }
    if (index == 118) __bf_tramp_table[118] = dlsym(RTLD_DEFAULT, "execve");
    if (index == 118) return __bf_tramp_table[118];
    if (index == 119) { __bf_tramp_table[119] = (void*)(unsigned long long)__bf_tramp_table[119]; }
    if (index == 119) __bf_tramp_table[119] = dlsym(RTLD_DEFAULT, "getopt_long");
    if (index == 119) return __bf_tramp_table[119];
    if (index == 120) { __bf_tramp_table[120] = (void*)(unsigned long long)__bf_tramp_table[120]; }
    if (index == 120) __bf_tramp_table[120] = dlsym(RTLD_DEFAULT, "getppid");
    if (index == 120) return __bf_tramp_table[120];
    if (index == 121) { __bf_tramp_table[121] = (void*)(unsigned long long)__bf_tramp_table[121]; }
    if (index == 121) __bf_tramp_table[121] = dlsym(RTLD_DEFAULT, "geteuid");
    if (index == 121) return __bf_tramp_table[121];
    if (index == 122) { __bf_tramp_table[122] = (void*)(unsigned long long)__bf_tramp_table[122]; }
    if (index == 122) __bf_tramp_table[122] = dlsym(RTLD_DEFAULT, "epoll_create1");
    if (index == 122) return __bf_tramp_table[122];
    if (index == 123) { __bf_tramp_table[123] = (void*)(unsigned long long)__bf_tramp_table[123]; }
    if (index == 123) __bf_tramp_table[123] = dlsym(RTLD_DEFAULT, "eventfd");
    if (index == 123) return __bf_tramp_table[123];
    if (index == 124) { __bf_tramp_table[124] = (void*)(unsigned long long)__bf_tramp_table[124]; }
    if (index == 124) __bf_tramp_table[124] = dlsym(RTLD_DEFAULT, "epoll_ctl");
    if (index == 124) return __bf_tramp_table[124];
    if (index == 125) { __bf_tramp_table[125] = (void*)(unsigned long long)__bf_tramp_table[125]; }
    if (index == 125) __bf_tramp_table[125] = dlsym(RTLD_DEFAULT, "getsockopt");
    if (index == 125) return __bf_tramp_table[125];
    if (index == 126) { __bf_tramp_table[126] = (void*)(unsigned long long)__bf_tramp_table[126]; }
    if (index == 126) __bf_tramp_table[126] = dlsym(RTLD_DEFAULT, "setsockopt");
    if (index == 126) return __bf_tramp_table[126];
    if (index == 127) { __bf_tramp_table[127] = (void*)(unsigned long long)__bf_tramp_table[127]; }
    if (index == 127) __bf_tramp_table[127] = dlsym(RTLD_DEFAULT, "epoll_wait");
    if (index == 127) return __bf_tramp_table[127];
    if (index == 128) { __bf_tramp_table[128] = (void*)(unsigned long long)__bf_tramp_table[128]; }
    if (index == 128) __bf_tramp_table[128] = dlsym(RTLD_DEFAULT, "getuid");
    if (index == 128) return __bf_tramp_table[128];
    if (index == 129) { __bf_tramp_table[129] = (void*)(unsigned long long)__bf_tramp_table[129]; }
    if (index == 129) __bf_tramp_table[129] = dlsym(RTLD_DEFAULT, "pthread_mutex_trylock");
    if (index == 129) return __bf_tramp_table[129];
    if (index == 130) { __bf_tramp_table[130] = (void*)(unsigned long long)__bf_tramp_table[130]; }
    if (index == 130) __bf_tramp_table[130] = dlsym(RTLD_DEFAULT, "ldexp");
    if (index == 130) return __bf_tramp_table[130];
    if (index == 131) { __bf_tramp_table[131] = (void*)(unsigned long long)__bf_tramp_table[131]; }
    if (index == 131) __bf_tramp_table[131] = dlsym(RTLD_DEFAULT, "strchr");
    if (index == 131) return __bf_tramp_table[131];
    if (index == 132) { __bf_tramp_table[132] = (void*)(unsigned long long)__bf_tramp_table[132]; }
    if (index == 132) __bf_tramp_table[132] = dlsym(RTLD_DEFAULT, "getaddrinfo");
    if (index == 132) return __bf_tramp_table[132];
    if (index == 133) { __bf_tramp_table[133] = (void*)(unsigned long long)__bf_tramp_table[133]; }
    if (index == 133) __bf_tramp_table[133] = dlsym(RTLD_DEFAULT, "socket");
    if (index == 133) return __bf_tramp_table[133];
    if (index == 134) { __bf_tramp_table[134] = (void*)(unsigned long long)__bf_tramp_table[134]; }
    if (index == 134) __bf_tramp_table[134] = dlsym(RTLD_DEFAULT, "freeaddrinfo");
    if (index == 134) return __bf_tramp_table[134];
    if (index == 135) { __bf_tramp_table[135] = (void*)(unsigned long long)__bf_tramp_table[135]; }
    if (index == 135) __bf_tramp_table[135] = dlsym(RTLD_DEFAULT, "connect");
    if (index == 135) return __bf_tramp_table[135];
    if (index == 136) { __bf_tramp_table[136] = (void*)(unsigned long long)__bf_tramp_table[136]; }
    if (index == 136) __bf_tramp_table[136] = dlsym(RTLD_DEFAULT, "poll");
    if (index == 136) return __bf_tramp_table[136];
    if (index == 137) { __bf_tramp_table[137] = (void*)(unsigned long long)__bf_tramp_table[137]; }
    if (index == 137) __bf_tramp_table[137] = dlsym(RTLD_DEFAULT, "sched_getscheduler");
    if (index == 137) return __bf_tramp_table[137];
    if (index == 138) { __bf_tramp_table[138] = (void*)(unsigned long long)__bf_tramp_table[138]; }
    if (index == 138) __bf_tramp_table[138] = dlsym(RTLD_DEFAULT, "sched_getparam");
    if (index == 138) return __bf_tramp_table[138];
    if (index == 139) { __bf_tramp_table[139] = (void*)(unsigned long long)__bf_tramp_table[139]; }
    if (index == 139) __bf_tramp_table[139] = dlsym(RTLD_DEFAULT, "getpriority");
    if (index == 139) return __bf_tramp_table[139];
    if (index == 140) { __bf_tramp_table[140] = (void*)(unsigned long long)__bf_tramp_table[140]; }
    if (index == 140) __bf_tramp_table[140] = dlsym(RTLD_DEFAULT, "uname");
    if (index == 140) return __bf_tramp_table[140];
    if (index == 141) { __bf_tramp_table[141] = (void*)(unsigned long long)__bf_tramp_table[141]; }
    if (index == 141) __bf_tramp_table[141] = dlsym(RTLD_DEFAULT, "tzset");
    if (index == 141) return __bf_tramp_table[141];
    if (index == 142) { __bf_tramp_table[142] = (void*)(unsigned long long)__bf_tramp_table[142]; }
    if (index == 142) __bf_tramp_table[142] = dlsym(RTLD_DEFAULT, "strnlen");
    if (index == 142) return __bf_tramp_table[142];
    if (index == 143) { __bf_tramp_table[143] = (void*)(unsigned long long)__bf_tramp_table[143]; }
    if (index == 143) __bf_tramp_table[143] = dlsym(RTLD_DEFAULT, "writev");
    if (index == 143) return __bf_tramp_table[143];
    if (index == 144) { __bf_tramp_table[144] = (void*)(unsigned long long)__bf_tramp_table[144]; }
    if (index == 144) __bf_tramp_table[144] = dlsym(RTLD_DEFAULT, "sscanf");
    if (index == 144) return __bf_tramp_table[144];
    if (index == 145) { __bf_tramp_table[145] = (void*)(unsigned long long)__bf_tramp_table[145]; }
    if (index == 145) __bf_tramp_table[145] = dlsym(RTLD_DEFAULT, "strtoul");
    if (index == 145) return __bf_tramp_table[145];
    if (index == 146) { __bf_tramp_table[146] = (void*)(unsigned long long)__bf_tramp_table[146]; }
    if (index == 146) __bf_tramp_table[146] = dlsym(RTLD_DEFAULT, "strtoull");
    if (index == 146) return __bf_tramp_table[146];
    if (index == 147) { __bf_tramp_table[147] = (void*)(unsigned long long)__bf_tramp_table[147]; }
    if (index == 147) __bf_tramp_table[147] = dlsym(RTLD_DEFAULT, "lseek");
    if (index == 147) return __bf_tramp_table[147];
    if (index == 148) { __bf_tramp_table[148] = (void*)(unsigned long long)__bf_tramp_table[148]; }
    if (index == 148) __bf_tramp_table[148] = dlsym(RTLD_DEFAULT, "ftruncate");
    if (index == 148) return __bf_tramp_table[148];
    if (index == 149) { __bf_tramp_table[149] = (void*)(unsigned long long)__bf_tramp_table[149]; }
    if (index == 149) __bf_tramp_table[149] = dlsym(RTLD_DEFAULT, "fstat");
    if (index == 149) return __bf_tramp_table[149];
    if (index == 150) { __bf_tramp_table[150] = (void*)(unsigned long long)__bf_tramp_table[150]; }
    if (index == 150) __bf_tramp_table[150] = dlsym(RTLD_DEFAULT, "lstat");
    if (index == 150) return __bf_tramp_table[150];
    if (index == 151) { __bf_tramp_table[151] = (void*)(unsigned long long)__bf_tramp_table[151]; }
    if (index == 151) __bf_tramp_table[151] = dlsym(RTLD_DEFAULT, "rename");
    if (index == 151) return __bf_tramp_table[151];
    if (index == 152) { __bf_tramp_table[152] = (void*)(unsigned long long)__bf_tramp_table[152]; }
    if (index == 152) __bf_tramp_table[152] = dlsym(RTLD_DEFAULT, "unlink");
    if (index == 152) return __bf_tramp_table[152];
    if (index == 153) { __bf_tramp_table[153] = (void*)(unsigned long long)__bf_tramp_table[153]; }
    if (index == 153) __bf_tramp_table[153] = dlsym(RTLD_DEFAULT, "rmdir");
    if (index == 153) return __bf_tramp_table[153];
    if (index == 154) { __bf_tramp_table[154] = (void*)(unsigned long long)__bf_tramp_table[154]; }
    if (index == 154) __bf_tramp_table[154] = dlsym(RTLD_DEFAULT, "nanosleep");
    if (index == 154) return __bf_tramp_table[154];
    if (index == 155) { __bf_tramp_table[155] = (void*)(unsigned long long)__bf_tramp_table[155]; }
    if (index == 155) __bf_tramp_table[155] = dlsym(RTLD_DEFAULT, "sigemptyset");
    if (index == 155) return __bf_tramp_table[155];
    if (index == 156) { __bf_tramp_table[156] = (void*)(unsigned long long)__bf_tramp_table[156]; }
    if (index == 156) __bf_tramp_table[156] = dlsym(RTLD_DEFAULT, "sigaction");
    if (index == 156) return __bf_tramp_table[156];
    if (index == 157) { __bf_tramp_table[157] = (void*)(unsigned long long)__bf_tramp_table[157]; }
    if (index == 157) __bf_tramp_table[157] = dlsym(RTLD_DEFAULT, "raise");
    if (index == 157) return __bf_tramp_table[157];
    if (index == 158) { __bf_tramp_table[158] = (void*)(unsigned long long)__bf_tramp_table[158]; }
    if (index == 158) __bf_tramp_table[158] = dlsym(RTLD_DEFAULT, "fscanf");
    if (index == 158) return __bf_tramp_table[158];
    if (index == 159) { __bf_tramp_table[159] = (void*)(unsigned long long)__bf_tramp_table[159]; }
    if (index == 159) __bf_tramp_table[159] = dlsym(RTLD_DEFAULT, "pread64");
    if (index == 159) return __bf_tramp_table[159];
    if (index == 160) { __bf_tramp_table[160] = (void*)(unsigned long long)__bf_tramp_table[160]; }
    if (index == 160) __bf_tramp_table[160] = dlsym(RTLD_DEFAULT, "ptrace");
    if (index == 160) return __bf_tramp_table[160];
    if (index == 161) { __bf_tramp_table[161] = (void*)(unsigned long long)__bf_tramp_table[161]; }
    if (index == 161) __bf_tramp_table[161] = dlsym(RTLD_DEFAULT, "socketpair");
    if (index == 161) return __bf_tramp_table[161];
    if (index == 162) { __bf_tramp_table[162] = (void*)(unsigned long long)__bf_tramp_table[162]; }
    if (index == 162) __bf_tramp_table[162] = dlsym(RTLD_DEFAULT, "sendmsg");
    if (index == 162) return __bf_tramp_table[162];
    if (index == 163) { __bf_tramp_table[163] = (void*)(unsigned long long)__bf_tramp_table[163]; }
    if (index == 163) __bf_tramp_table[163] = dlsym(RTLD_DEFAULT, "recvmsg");
    if (index == 163) return __bf_tramp_table[163];
    if (index == 164) { __bf_tramp_table[164] = (void*)(unsigned long long)__bf_tramp_table[164]; }
    if (index == 164) __bf_tramp_table[164] = dlsym(RTLD_DEFAULT, "__cmsg_nxthdr");
    if (index == 164) return __bf_tramp_table[164];
    if (index == 165) { __bf_tramp_table[165] = (void*)(unsigned long long)__bf_tramp_table[165]; }
    if (index == 165) __bf_tramp_table[165] = dlsym(RTLD_DEFAULT, "readlink");
    if (index == 165) return __bf_tramp_table[165];
    if (index == 166) { __bf_tramp_table[166] = (void*)(unsigned long long)__bf_tramp_table[166]; }
    if (index == 166) __bf_tramp_table[166] = dlsym(RTLD_DEFAULT, "printf");
    if (index == 166) return __bf_tramp_table[166];
    if (index == 167) { __bf_tramp_table[167] = (void*)(unsigned long long)__bf_tramp_table[167]; }
    if (index == 167) __bf_tramp_table[167] = dlsym(RTLD_DEFAULT, "wmemchr");
    if (index == 167) return __bf_tramp_table[167];
    if (index == 168) { __bf_tramp_table[168] = (void*)(unsigned long long)__bf_tramp_table[168]; }
    if (index == 168) __bf_tramp_table[168] = dlsym(RTLD_DEFAULT, "localeconv");
    if (index == 168) return __bf_tramp_table[168];
    if (index == 169) { __bf_tramp_table[169] = (void*)(unsigned long long)__bf_tramp_table[169]; }
    if (index == 169) __bf_tramp_table[169] = dlsym(RTLD_DEFAULT, "__vsnprintf_chk");
    if (index == 169) return __bf_tramp_table[169];
    if (index == 170) { __bf_tramp_table[170] = (void*)(unsigned long long)__bf_tramp_table[170]; }
    if (index == 170) __bf_tramp_table[170] = dlsym(RTLD_DEFAULT, "__memmove_chk");
    if (index == 170) return __bf_tramp_table[170];
    if (index == 171) { __bf_tramp_table[171] = (void*)(unsigned long long)__bf_tramp_table[171]; }
    if (index == 171) __bf_tramp_table[171] = dlsym(RTLD_DEFAULT, "sched_yield");
    if (index == 171) return __bf_tramp_table[171];
    if (index == 172) { __bf_tramp_table[172] = (void*)(unsigned long long)__bf_tramp_table[172]; }
    if (index == 172) __bf_tramp_table[172] = dlsym(RTLD_DEFAULT, "modf");
    if (index == 172) return __bf_tramp_table[172];
    if (index == 173) { __bf_tramp_table[173] = (void*)(unsigned long long)__bf_tramp_table[173]; }
    if (index == 173) __bf_tramp_table[173] = dlsym(RTLD_DEFAULT, "strcasecmp");
    if (index == 173) return __bf_tramp_table[173];
    if (index == 174) { __bf_tramp_table[174] = (void*)(unsigned long long)__bf_tramp_table[174]; }
    if (index == 174) __bf_tramp_table[174] = dlsym(RTLD_DEFAULT, "getnameinfo");
    if (index == 174) return __bf_tramp_table[174];
    if (index == 175) { __bf_tramp_table[175] = (void*)(unsigned long long)__bf_tramp_table[175]; }
    if (index == 175) __bf_tramp_table[175] = dlsym(RTLD_DEFAULT, "strftime");
    if (index == 175) return __bf_tramp_table[175];
    if (index == 176) { __bf_tramp_table[176] = (void*)(unsigned long long)__bf_tramp_table[176]; }
    if (index == 176) __bf_tramp_table[176] = dlsym(RTLD_DEFAULT, "__strcpy_chk");
    if (index == 176) return __bf_tramp_table[176];
    if (index == 177) { __bf_tramp_table[177] = (void*)(unsigned long long)__bf_tramp_table[177]; }
    if (index == 177) __bf_tramp_table[177] = dlsym(RTLD_DEFAULT, "frexpf");
    if (index == 177) return __bf_tramp_table[177];
    if (index == 178) { __bf_tramp_table[178] = (void*)(unsigned long long)__bf_tramp_table[178]; }
    if (index == 178) __bf_tramp_table[178] = dlsym(RTLD_DEFAULT, "ldexpf");
    if (index == 178) return __bf_tramp_table[178];
    if (index == 179) { __bf_tramp_table[179] = (void*)(unsigned long long)__bf_tramp_table[179]; }
    if (index == 179) __bf_tramp_table[179] = dlsym(RTLD_DEFAULT, "tanf");
    if (index == 179) return __bf_tramp_table[179];
    if (index == 180) { __bf_tramp_table[180] = (void*)(unsigned long long)__bf_tramp_table[180]; }
    if (index == 180) __bf_tramp_table[180] = dlsym(RTLD_DEFAULT, "atanf");
    if (index == 180) return __bf_tramp_table[180];
    if (index == 181) { __bf_tramp_table[181] = (void*)(unsigned long long)__bf_tramp_table[181]; }
    if (index == 181) __bf_tramp_table[181] = dlsym(RTLD_DEFAULT, "erff");
    if (index == 181) return __bf_tramp_table[181];
    if (index == 182) { __bf_tramp_table[182] = (void*)(unsigned long long)__bf_tramp_table[182]; }
    if (index == 182) __bf_tramp_table[182] = dlsym(RTLD_DEFAULT, "acosf");
    if (index == 182) return __bf_tramp_table[182];
    if (index == 183) { __bf_tramp_table[183] = (void*)(unsigned long long)__bf_tramp_table[183]; }
    if (index == 183) __bf_tramp_table[183] = dlsym(RTLD_DEFAULT, "strstr");
    if (index == 183) return __bf_tramp_table[183];
    if (index == 184) { __bf_tramp_table[184] = (void*)(unsigned long long)__bf_tramp_table[184]; }
    if (index == 184) __bf_tramp_table[184] = dlsym(RTLD_DEFAULT, "erfcf");
    if (index == 184) return __bf_tramp_table[184];
    if (index == 185) { __bf_tramp_table[185] = (void*)(unsigned long long)__bf_tramp_table[185]; }
    if (index == 185) __bf_tramp_table[185] = dlsym(RTLD_DEFAULT, "modff");
    if (index == 185) return __bf_tramp_table[185];
    if (index == 186) { __bf_tramp_table[186] = (void*)(unsigned long long)__bf_tramp_table[186]; }
    if (index == 186) __bf_tramp_table[186] = dlsym(RTLD_DEFAULT, "coshf");
    if (index == 186) return __bf_tramp_table[186];
    if (index == 187) { __bf_tramp_table[187] = (void*)(unsigned long long)__bf_tramp_table[187]; }
    if (index == 187) __bf_tramp_table[187] = dlsym(RTLD_DEFAULT, "sinhf");
    if (index == 187) return __bf_tramp_table[187];
    if (index == 188) { __bf_tramp_table[188] = (void*)(unsigned long long)__bf_tramp_table[188]; }
    if (index == 188) __bf_tramp_table[188] = dlsym(RTLD_DEFAULT, "tanhf");
    if (index == 188) return __bf_tramp_table[188];
    if (index == 189) { __bf_tramp_table[189] = (void*)(unsigned long long)__bf_tramp_table[189]; }
    if (index == 189) __bf_tramp_table[189] = dlsym(RTLD_DEFAULT, "atan2");
    if (index == 189) return __bf_tramp_table[189];
    if (index == 190) { __bf_tramp_table[190] = (void*)(unsigned long long)__bf_tramp_table[190]; }
    if (index == 190) __bf_tramp_table[190] = dlsym(RTLD_DEFAULT, "cbrtf");
    if (index == 190) return __bf_tramp_table[190];
    if (index == 191) { __bf_tramp_table[191] = (void*)(unsigned long long)__bf_tramp_table[191]; }
    if (index == 191) __bf_tramp_table[191] = dlsym(RTLD_DEFAULT, "strchr");
    if (index == 191) return __bf_tramp_table[191];
    if (index == 192) { __bf_tramp_table[192] = (void*)(unsigned long long)__bf_tramp_table[192]; }
    if (index == 192) __bf_tramp_table[192] = dlsym(RTLD_DEFAULT, "clock");
    if (index == 192) return __bf_tramp_table[192];
    if (index == 193) { __bf_tramp_table[193] = (void*)(unsigned long long)__bf_tramp_table[193]; }
    if (index == 193) __bf_tramp_table[193] = dlsym(RTLD_DEFAULT, "fileno");
    if (index == 193) return __bf_tramp_table[193];
    if (index == 194) { __bf_tramp_table[194] = (void*)(unsigned long long)__bf_tramp_table[194]; }
    if (index == 194) __bf_tramp_table[194] = dlsym(RTLD_DEFAULT, "remainderf");
    if (index == 194) return __bf_tramp_table[194];
    if (index == 195) { __bf_tramp_table[195] = (void*)(unsigned long long)__bf_tramp_table[195]; }
    if (index == 195) __bf_tramp_table[195] = dlsym(RTLD_DEFAULT, "nan");
    if (index == 195) return __bf_tramp_table[195];
    if (index == 196) { __bf_tramp_table[196] = (void*)(unsigned long long)__bf_tramp_table[196]; }
    if (index == 196) __bf_tramp_table[196] = dlsym(RTLD_DEFAULT, "qsort");
    if (index == 196) return __bf_tramp_table[196];
    if (index == 197) { __bf_tramp_table[197] = (void*)(unsigned long long)__bf_tramp_table[197]; }
    if (index == 197) __bf_tramp_table[197] = dlsym(RTLD_DEFAULT, "nextafterf");
    if (index == 197) return __bf_tramp_table[197];
    if (index == 198) { __bf_tramp_table[198] = (void*)(unsigned long long)__bf_tramp_table[198]; }
    if (index == 198) __bf_tramp_table[198] = dlsym(RTLD_DEFAULT, "acos");
    if (index == 198) return __bf_tramp_table[198];
    if (index == 199) { __bf_tramp_table[199] = (void*)(unsigned long long)__bf_tramp_table[199]; }
    if (index == 199) __bf_tramp_table[199] = dlsym(RTLD_DEFAULT, "asin");
    if (index == 199) return __bf_tramp_table[199];
    if (index == 200) { __bf_tramp_table[200] = (void*)(unsigned long long)__bf_tramp_table[200]; }
    if (index == 200) __bf_tramp_table[200] = dlsym(RTLD_DEFAULT, "ilogb");
    if (index == 200) return __bf_tramp_table[200];
    if (index == 201) { __bf_tramp_table[201] = (void*)(unsigned long long)__bf_tramp_table[201]; }
    if (index == 201) __bf_tramp_table[201] = dlsym(RTLD_DEFAULT, "FD_SET");
    if (index == 201) return __bf_tramp_table[201];
    if (index == 202) { __bf_tramp_table[202] = (void*)(unsigned long long)__bf_tramp_table[202]; }
    if (index == 202) __bf_tramp_table[202] = dlsym(RTLD_DEFAULT, "select");
    if (index == 202) return __bf_tramp_table[202];
    if (index == 203) { __bf_tramp_table[203] = (void*)(unsigned long long)__bf_tramp_table[203]; }
    if (index == 203) __bf_tramp_table[203] = dlsym(RTLD_DEFAULT, "FD_ISSET");
    if (index == 203) return __bf_tramp_table[203];
    if (index == 204) { __bf_tramp_table[204] = (void*)(unsigned long long)__bf_tramp_table[204]; }
    if (index == 204) __bf_tramp_table[204] = dlsym(RTLD_DEFAULT, "sendto");
    if (index == 204) return __bf_tramp_table[204];
    if (index == 205) { __bf_tramp_table[205] = (void*)(unsigned long long)__bf_tramp_table[205]; }
    if (index == 205) __bf_tramp_table[205] = dlsym(RTLD_DEFAULT, "recvfrom");
    if (index == 205) return __bf_tramp_table[205];
    if (index == 206) { __bf_tramp_table[206] = (void*)(unsigned long long)__bf_tramp_table[206]; }
    if (index == 206) __bf_tramp_table[206] = dlsym(RTLD_DEFAULT, "__strcat_chk");
    if (index == 206) return __bf_tramp_table[206];
    if (index == 207) { __bf_tramp_table[207] = (void*)(unsigned long long)__bf_tramp_table[207]; }
    if (index == 207) __bf_tramp_table[207] = dlsym(RTLD_DEFAULT, "setpriority");
    if (index == 207) return __bf_tramp_table[207];
    if (index == 208) { __bf_tramp_table[208] = (void*)(unsigned long long)__bf_tramp_table[208]; }
    if (index == 208) __bf_tramp_table[208] = dlsym(RTLD_DEFAULT, "pthread_mutexattr_init");
    if (index == 208) return __bf_tramp_table[208];
    if (index == 209) { __bf_tramp_table[209] = (void*)(unsigned long long)__bf_tramp_table[209]; }
    if (index == 209) __bf_tramp_table[209] = dlsym(RTLD_DEFAULT, "pthread_mutexattr_settype");
    if (index == 209) return __bf_tramp_table[209];
    if (index == 210) { __bf_tramp_table[210] = (void*)(unsigned long long)__bf_tramp_table[210]; }
    if (index == 210) __bf_tramp_table[210] = dlsym(RTLD_DEFAULT, "pthread_mutexattr_destroy");
    if (index == 210) return __bf_tramp_table[210];
    if (index == 211) { __bf_tramp_table[211] = (void*)(unsigned long long)__bf_tramp_table[211]; }
    if (index == 211) __bf_tramp_table[211] = dlsym(RTLD_DEFAULT, "sem_init");
    if (index == 211) return __bf_tramp_table[211];
    if (index == 212) { __bf_tramp_table[212] = (void*)(unsigned long long)__bf_tramp_table[212]; }
    if (index == 212) __bf_tramp_table[212] = dlsym(RTLD_DEFAULT, "sem_destroy");
    if (index == 212) return __bf_tramp_table[212];
    if (index == 213) { __bf_tramp_table[213] = (void*)(unsigned long long)__bf_tramp_table[213]; }
    if (index == 213) __bf_tramp_table[213] = dlsym(RTLD_DEFAULT, "sem_wait");
    if (index == 213) return __bf_tramp_table[213];
    if (index == 214) { __bf_tramp_table[214] = (void*)(unsigned long long)__bf_tramp_table[214]; }
    if (index == 214) __bf_tramp_table[214] = dlsym(RTLD_DEFAULT, "sem_post");
    if (index == 214) return __bf_tramp_table[214];
    if (index == 215) { __bf_tramp_table[215] = (void*)(unsigned long long)__bf_tramp_table[215]; }
    if (index == 215) __bf_tramp_table[215] = dlsym(RTLD_DEFAULT, "inet_ntop");
    if (index == 215) return __bf_tramp_table[215];
    if (index == 216) { __bf_tramp_table[216] = (void*)(unsigned long long)__bf_tramp_table[216]; }
    if (index == 216) __bf_tramp_table[216] = dlsym(RTLD_DEFAULT, "inet_pton");
    if (index == 216) return __bf_tramp_table[216];
    if (index == 217) { __bf_tramp_table[217] = (void*)(unsigned long long)__bf_tramp_table[217]; }
    if (index == 217) __bf_tramp_table[217] = dlsym(RTLD_DEFAULT, "strncasecmp");
    if (index == 217) return __bf_tramp_table[217];
    if (index == 218) { __bf_tramp_table[218] = (void*)(unsigned long long)__bf_tramp_table[218]; }
    if (index == 218) __bf_tramp_table[218] = dlsym(RTLD_DEFAULT, "pthread_attr_setschedparam");
    if (index == 218) return __bf_tramp_table[218];
    if (index == 219) { __bf_tramp_table[219] = (void*)(unsigned long long)__bf_tramp_table[219]; }
    if (index == 219) __bf_tramp_table[219] = dlsym(RTLD_DEFAULT, "bind");
    if (index == 219) return __bf_tramp_table[219];
    if (index == 220) { __bf_tramp_table[220] = (void*)(unsigned long long)__bf_tramp_table[220]; }
    if (index == 220) __bf_tramp_table[220] = dlsym(RTLD_DEFAULT, "getsockname");
    if (index == 220) return __bf_tramp_table[220];
    if (index == 221) { __bf_tramp_table[221] = (void*)(unsigned long long)__bf_tramp_table[221]; }
    if (index == 221) __bf_tramp_table[221] = dlsym(RTLD_DEFAULT, "gethostname");
    if (index == 221) return __bf_tramp_table[221];
    if (index == 222) { __bf_tramp_table[222] = (void*)(unsigned long long)__bf_tramp_table[222]; }
    if (index == 222) __bf_tramp_table[222] = dlsym(RTLD_DEFAULT, "__sendto_chk");
    if (index == 222) return __bf_tramp_table[222];
    if (index == 223) { __bf_tramp_table[223] = (void*)(unsigned long long)__bf_tramp_table[223]; }
    if (index == 223) __bf_tramp_table[223] = dlsym(RTLD_DEFAULT, "puts");
    if (index == 223) return __bf_tramp_table[223];
    if (index == 224) { __bf_tramp_table[224] = (void*)(unsigned long long)__bf_tramp_table[224]; }
    if (index == 224) __bf_tramp_table[224] = dlsym(RTLD_DEFAULT, "gai_strerror");
    if (index == 224) return __bf_tramp_table[224];
    if (index == 225) { __bf_tramp_table[225] = (void*)(unsigned long long)__bf_tramp_table[225]; }
    if (index == 225) __bf_tramp_table[225] = dlsym(RTLD_DEFAULT, "__write_chk");
    if (index == 225) return __bf_tramp_table[225];
    if (index == 226) { __bf_tramp_table[226] = (void*)(unsigned long long)__bf_tramp_table[226]; }
    if (index == 226) __bf_tramp_table[226] = dlsym(RTLD_DEFAULT, "__poll_chk");
    if (index == 226) return __bf_tramp_table[226];
    if (index == 227) { __bf_tramp_table[227] = (void*)(unsigned long long)__bf_tramp_table[227]; }
    if (index == 227) __bf_tramp_table[227] = dlsym(RTLD_DEFAULT, "vprintf");
    if (index == 227) return __bf_tramp_table[227];
    if (index == 228) { __bf_tramp_table[228] = (void*)(unsigned long long)__bf_tramp_table[228]; }
    if (index == 228) __bf_tramp_table[228] = dlsym(RTLD_DEFAULT, "usleep");
    if (index == 228) return __bf_tramp_table[228];
    if (index == 229) { __bf_tramp_table[229] = (void*)(unsigned long long)__bf_tramp_table[229]; }
    if (index == 229) __bf_tramp_table[229] = dlsym(RTLD_DEFAULT, "pthread_kill");
    if (index == 229) return __bf_tramp_table[229];
    if (index == 230) { __bf_tramp_table[230] = (void*)(unsigned long long)__bf_tramp_table[230]; }
    if (index == 230) __bf_tramp_table[230] = dlsym(RTLD_DEFAULT, "pthread_detach");
    if (index == 230) return __bf_tramp_table[230];
    if (index == 231) { __bf_tramp_table[231] = (void*)(unsigned long long)__bf_tramp_table[231]; }
    if (index == 231) __bf_tramp_table[231] = dlsym(RTLD_DEFAULT, "exit");
    if (index == 231) return __bf_tramp_table[231];
    if (index == 232) { __bf_tramp_table[232] = (void*)(unsigned long long)__bf_tramp_table[232]; }
    if (index == 232) __bf_tramp_table[232] = dlsym(RTLD_DEFAULT, "ferror");
    if (index == 232) return __bf_tramp_table[232];
    if (index == 233) { __bf_tramp_table[233] = (void*)(unsigned long long)__bf_tramp_table[233]; }
    if (index == 233) __bf_tramp_table[233] = dlsym(RTLD_DEFAULT, "clearerr");
    if (index == 233) return __bf_tramp_table[233];
    if (index == 234) { __bf_tramp_table[234] = (void*)(unsigned long long)__bf_tramp_table[234]; }
    if (index == 234) __bf_tramp_table[234] = dlsym(RTLD_DEFAULT, "wcslen");
    if (index == 234) return __bf_tramp_table[234];
    if (index == 235) { __bf_tramp_table[235] = (void*)(unsigned long long)__bf_tramp_table[235]; }
    if (index == 235) __bf_tramp_table[235] = dlsym(RTLD_DEFAULT, "wmemcmp");
    if (index == 235) return __bf_tramp_table[235];
    if (index == 236) { __bf_tramp_table[236] = (void*)(unsigned long long)__bf_tramp_table[236]; }
    if (index == 236) __bf_tramp_table[236] = dlsym(RTLD_DEFAULT, "exp");
    if (index == 236) return __bf_tramp_table[236];
    if (index == 237) { __bf_tramp_table[237] = (void*)(unsigned long long)__bf_tramp_table[237]; }
    if (index == 237) __bf_tramp_table[237] = dlsym(RTLD_DEFAULT, "pow");
    if (index == 237) return __bf_tramp_table[237];
    if (index == 238) { __bf_tramp_table[238] = (void*)(unsigned long long)__bf_tramp_table[238]; }
    if (index == 238) __bf_tramp_table[238] = dlsym(RTLD_DEFAULT, "fmod");
    if (index == 238) return __bf_tramp_table[238];
    if (index == 239) { __bf_tramp_table[239] = (void*)(unsigned long long)__bf_tramp_table[239]; }
    if (index == 239) __bf_tramp_table[239] = dlsym(RTLD_DEFAULT, "log");
    if (index == 239) return __bf_tramp_table[239];
    if (index == 240) { __bf_tramp_table[240] = (void*)(unsigned long long)__bf_tramp_table[240]; }
    if (index == 240) __bf_tramp_table[240] = dlsym(RTLD_DEFAULT, "log2");
    if (index == 240) return __bf_tramp_table[240];
    if (index == 241) { __bf_tramp_table[241] = (void*)(unsigned long long)__bf_tramp_table[241]; }
    if (index == 241) __bf_tramp_table[241] = dlsym(RTLD_DEFAULT, "log10");
    if (index == 241) return __bf_tramp_table[241];
    if (index == 242) { __bf_tramp_table[242] = (void*)(unsigned long long)__bf_tramp_table[242]; }
    if (index == 242) __bf_tramp_table[242] = dlsym(RTLD_DEFAULT, "round");
    if (index == 242) return __bf_tramp_table[242];
    if (index == 243) { __bf_tramp_table[243] = (void*)(unsigned long long)__bf_tramp_table[243]; }
    if (index == 243) __bf_tramp_table[243] = dlsym(RTLD_DEFAULT, "frexp");
    if (index == 243) return __bf_tramp_table[243];
    if (index == 244) { __bf_tramp_table[244] = (void*)(unsigned long long)__bf_tramp_table[244]; }
    if (index == 244) __bf_tramp_table[244] = dlsym(RTLD_DEFAULT, "sin");
    if (index == 244) return __bf_tramp_table[244];
    if (index == 245) { __bf_tramp_table[245] = (void*)(unsigned long long)__bf_tramp_table[245]; }
    if (index == 245) __bf_tramp_table[245] = dlsym(RTLD_DEFAULT, "sinh");
    if (index == 245) return __bf_tramp_table[245];
    if (index == 246) { __bf_tramp_table[246] = (void*)(unsigned long long)__bf_tramp_table[246]; }
    if (index == 246) __bf_tramp_table[246] = dlsym(RTLD_DEFAULT, "cos");
    if (index == 246) return __bf_tramp_table[246];
    if (index == 247) { __bf_tramp_table[247] = (void*)(unsigned long long)__bf_tramp_table[247]; }
    if (index == 247) __bf_tramp_table[247] = dlsym(RTLD_DEFAULT, "cosh");
    if (index == 247) return __bf_tramp_table[247];
    if (index == 248) { __bf_tramp_table[248] = (void*)(unsigned long long)__bf_tramp_table[248]; }
    if (index == 248) __bf_tramp_table[248] = dlsym(RTLD_DEFAULT, "tan");
    if (index == 248) return __bf_tramp_table[248];
    if (index == 249) { __bf_tramp_table[249] = (void*)(unsigned long long)__bf_tramp_table[249]; }
    if (index == 249) __bf_tramp_table[249] = dlsym(RTLD_DEFAULT, "tanh");
    if (index == 249) return __bf_tramp_table[249];
    if (index == 250) { __bf_tramp_table[250] = (void*)(unsigned long long)__bf_tramp_table[250]; }
    if (index == 250) __bf_tramp_table[250] = dlsym(RTLD_DEFAULT, "atol");
    if (index == 250) return __bf_tramp_table[250];
    if (index == 251) { __bf_tramp_table[251] = (void*)(unsigned long long)__bf_tramp_table[251]; }
    if (index == 251) __bf_tramp_table[251] = dlsym(RTLD_DEFAULT, "atof");
    if (index == 251) return __bf_tramp_table[251];
    if (index == 252) { __bf_tramp_table[252] = (void*)(unsigned long long)__bf_tramp_table[252]; }
    if (index == 252) __bf_tramp_table[252] = dlsym(RTLD_DEFAULT, "strspn");
    if (index == 252) return __bf_tramp_table[252];
    if (index == 253) { __bf_tramp_table[253] = (void*)(unsigned long long)__bf_tramp_table[253]; }
    if (index == 253) __bf_tramp_table[253] = dlsym(RTLD_DEFAULT, "strtof");
    if (index == 253) return __bf_tramp_table[253];
    if (index == 254) { __bf_tramp_table[254] = (void*)(unsigned long long)__bf_tramp_table[254]; }
    if (index == 254) __bf_tramp_table[254] = dlsym(RTLD_DEFAULT, "ioctl");
    if (index == 254) return __bf_tramp_table[254];
    if (index == 255) { __bf_tramp_table[255] = (void*)(unsigned long long)__bf_tramp_table[255]; }
    if (index == 255) __bf_tramp_table[255] = dlsym(RTLD_DEFAULT, "getpeername");
    if (index == 255) return __bf_tramp_table[255];
    if (index == 256) { __bf_tramp_table[256] = (void*)(unsigned long long)__bf_tramp_table[256]; }
    if (index == 256) __bf_tramp_table[256] = dlsym(RTLD_DEFAULT, "listen");
    if (index == 256) return __bf_tramp_table[256];
    if (index == 257) { __bf_tramp_table[257] = (void*)(unsigned long long)__bf_tramp_table[257]; }
    if (index == 257) __bf_tramp_table[257] = dlsym(RTLD_DEFAULT, "accept");
    if (index == 257) return __bf_tramp_table[257];
    if (index == 258) { __bf_tramp_table[258] = (void*)(unsigned long long)__bf_tramp_table[258]; }
    if (index == 258) __bf_tramp_table[258] = dlsym(RTLD_DEFAULT, "epoll_create");
    if (index == 258) return __bf_tramp_table[258];
    if (index == 259) { __bf_tramp_table[259] = (void*)(unsigned long long)__bf_tramp_table[259]; }
    if (index == 259) __bf_tramp_table[259] = dlsym(RTLD_DEFAULT, "FD_CLR");
    if (index == 259) return __bf_tramp_table[259];
    if (index == 260) { __bf_tramp_table[260] = (void*)(unsigned long long)__bf_tramp_table[260]; }
    if (index == 260) __bf_tramp_table[260] = dlsym(RTLD_DEFAULT, "expm1");
    if (index == 260) return __bf_tramp_table[260];
    if (index == 261) { __bf_tramp_table[261] = (void*)(unsigned long long)__bf_tramp_table[261]; }
    if (index == 261) __bf_tramp_table[261] = dlsym(RTLD_DEFAULT, "if_indextoname");
    if (index == 261) return __bf_tramp_table[261];
    if (index == 262) { __bf_tramp_table[262] = (void*)(unsigned long long)__bf_tramp_table[262]; }
    if (index == 262) __bf_tramp_table[262] = dlsym(RTLD_DEFAULT, "sigaddset");
    if (index == 262) return __bf_tramp_table[262];
    if (index == 263) { __bf_tramp_table[263] = (void*)(unsigned long long)__bf_tramp_table[263]; }
    if (index == 263) __bf_tramp_table[263] = dlsym(RTLD_DEFAULT, "pthread_sigmask");
    if (index == 263) return __bf_tramp_table[263];
    if (index == 264) { __bf_tramp_table[264] = (void*)(unsigned long long)__bf_tramp_table[264]; }
    if (index == 264) __bf_tramp_table[264] = dlsym(RTLD_DEFAULT, "fgets");
    if (index == 264) return __bf_tramp_table[264];
    if (index == 265) { __bf_tramp_table[265] = (void*)(unsigned long long)__bf_tramp_table[265]; }
    if (index == 265) __bf_tramp_table[265] = dlsym(RTLD_DEFAULT, "setjmp");
    if (index == 265) return __bf_tramp_table[265];
    if (index == 266) { __bf_tramp_table[266] = (void*)(unsigned long long)__bf_tramp_table[266]; }
    if (index == 266) __bf_tramp_table[266] = dlsym(RTLD_DEFAULT, "longjmp");
    if (index == 266) return __bf_tramp_table[266];
    if (index == 267) { __bf_tramp_table[267] = (void*)(unsigned long long)__bf_tramp_table[267]; }
    if (index == 267) __bf_tramp_table[267] = dlsym(RTLD_DEFAULT, "pthread_condattr_init");
    if (index == 267) return __bf_tramp_table[267];
    if (index == 268) { __bf_tramp_table[268] = (void*)(unsigned long long)__bf_tramp_table[268]; }
    if (index == 268) __bf_tramp_table[268] = dlsym(RTLD_DEFAULT, "pthread_condattr_setclock");
    if (index == 268) return __bf_tramp_table[268];
    if (index == 269) { __bf_tramp_table[269] = (void*)(unsigned long long)__bf_tramp_table[269]; }
    if (index == 269) __bf_tramp_table[269] = dlsym(RTLD_DEFAULT, "pthread_condattr_destroy");
    if (index == 269) return __bf_tramp_table[269];
    if (index == 270) { __bf_tramp_table[270] = (void*)(unsigned long long)__bf_tramp_table[270]; }
    if (index == 270) __bf_tramp_table[270] = dlsym(RTLD_DEFAULT, "sched_get_priority_min");
    if (index == 270) return __bf_tramp_table[270];
    if (index == 271) { __bf_tramp_table[271] = (void*)(unsigned long long)__bf_tramp_table[271]; }
    if (index == 271) __bf_tramp_table[271] = dlsym(RTLD_DEFAULT, "pthread_setschedparam");
    if (index == 271) return __bf_tramp_table[271];
    if (index == 272) { __bf_tramp_table[272] = (void*)(unsigned long long)__bf_tramp_table[272]; }
    if (index == 272) __bf_tramp_table[272] = dlsym(RTLD_DEFAULT, "getgid");
    if (index == 272) return __bf_tramp_table[272];
    if (index == 273) { __bf_tramp_table[273] = (void*)(unsigned long long)__bf_tramp_table[273]; }
    if (index == 273) __bf_tramp_table[273] = dlsym(RTLD_DEFAULT, "getegid");
    if (index == 273) return __bf_tramp_table[273];
    if (index == 274) { __bf_tramp_table[274] = (void*)(unsigned long long)__bf_tramp_table[274]; }
    if (index == 274) __bf_tramp_table[274] = dlsym(RTLD_DEFAULT, "random");
    if (index == 274) return __bf_tramp_table[274];
    if (index == 275) { __bf_tramp_table[275] = (void*)(unsigned long long)__bf_tramp_table[275]; }
    if (index == 275) __bf_tramp_table[275] = dlsym(RTLD_DEFAULT, "sigfillset");
    if (index == 275) return __bf_tramp_table[275];
    if (index == 276) { __bf_tramp_table[276] = (void*)(unsigned long long)__bf_tramp_table[276]; }
    if (index == 276) __bf_tramp_table[276] = dlsym(RTLD_DEFAULT, "fdopen");
    if (index == 276) return __bf_tramp_table[276];
    if (index == 277) { __bf_tramp_table[277] = (void*)(unsigned long long)__bf_tramp_table[277]; }
    if (index == 277) __bf_tramp_table[277] = dlsym(RTLD_DEFAULT, "timerfd_create");
    if (index == 277) return __bf_tramp_table[277];
    if (index == 278) { __bf_tramp_table[278] = (void*)(unsigned long long)__bf_tramp_table[278]; }
    if (index == 278) __bf_tramp_table[278] = dlsym(RTLD_DEFAULT, "timerfd_settime");
    if (index == 278) return __bf_tramp_table[278];
    if (index == 279) { __bf_tramp_table[279] = (void*)(unsigned long long)__bf_tramp_table[279]; }
    if (index == 279) __bf_tramp_table[279] = dlsym(RTLD_DEFAULT, "fputc");
    if (index == 279) return __bf_tramp_table[279];
    if (index == 280) { __bf_tramp_table[280] = (void*)(unsigned long long)__bf_tramp_table[280]; }
    if (index == 280) __bf_tramp_table[280] = dlsym(RTLD_DEFAULT, "bsearch");
    if (index == 280) return __bf_tramp_table[280];
    if (index == 281) { __bf_tramp_table[281] = (void*)(unsigned long long)__bf_tramp_table[281]; }
    if (index == 281) __bf_tramp_table[281] = dlsym(RTLD_DEFAULT, "vfprintf");
    if (index == 281) return __bf_tramp_table[281];
    if (index == 282) { __bf_tramp_table[282] = (void*)(unsigned long long)__bf_tramp_table[282]; }
    if (index == 282) __bf_tramp_table[282] = dlsym(RTLD_DEFAULT, "pthread_exit");
    if (index == 282) return __bf_tramp_table[282];
    if (index == 283) { __bf_tramp_table[283] = (void*)(unsigned long long)__bf_tramp_table[283]; }
    if (index == 283) __bf_tramp_table[283] = dlsym(RTLD_DEFAULT, "finitef");
    if (index == 283) return __bf_tramp_table[283];
    if (index == 284) { __bf_tramp_table[284] = (void*)(unsigned long long)__bf_tramp_table[284]; }
    if (index == 284) __bf_tramp_table[284] = dlsym(RTLD_DEFAULT, "cbrt");
    if (index == 284) return __bf_tramp_table[284];
    if (index == 285) { __bf_tramp_table[285] = (void*)(unsigned long long)__bf_tramp_table[285]; }
    if (index == 285) __bf_tramp_table[285] = dlsym(RTLD_DEFAULT, "remquof");
    if (index == 285) return __bf_tramp_table[285];
    if (index == 286) { __bf_tramp_table[286] = (void*)(unsigned long long)__bf_tramp_table[286]; }
    if (index == 286) __bf_tramp_table[286] = dlsym(RTLD_DEFAULT, "strcspn");
    if (index == 286) return __bf_tramp_table[286];
    if (index == 287) { __bf_tramp_table[287] = (void*)(unsigned long long)__bf_tramp_table[287]; }
    if (index == 287) __bf_tramp_table[287] = dlsym(RTLD_DEFAULT, "gmtime_r");
    if (index == 287) return __bf_tramp_table[287];
    if (index == 288) { __bf_tramp_table[288] = (void*)(unsigned long long)__bf_tramp_table[288]; }
    if (index == 288) __bf_tramp_table[288] = dlsym(RTLD_DEFAULT, "difftime");
    if (index == 288) return __bf_tramp_table[288];
    if (index == 289) { __bf_tramp_table[289] = (void*)(unsigned long long)__bf_tramp_table[289]; }
    if (index == 289) __bf_tramp_table[289] = dlsym(RTLD_DEFAULT, "strpbrk");
    if (index == 289) return __bf_tramp_table[289];
    if (index == 290) { __bf_tramp_table[290] = (void*)(unsigned long long)__bf_tramp_table[290]; }
    if (index == 290) __bf_tramp_table[290] = dlsym(RTLD_DEFAULT, "shutdown");
    if (index == 290) return __bf_tramp_table[290];
    if (index == 291) { __bf_tramp_table[291] = (void*)(unsigned long long)__bf_tramp_table[291]; }
    if (index == 291) __bf_tramp_table[291] = dlsym(RTLD_DEFAULT, "memrchr");
    if (index == 291) return __bf_tramp_table[291];
    if (index == 292) { __bf_tramp_table[292] = (void*)(unsigned long long)__bf_tramp_table[292]; }
    if (index == 292) __bf_tramp_table[292] = dlsym(RTLD_DEFAULT, "accept4");
    if (index == 292) return __bf_tramp_table[292];
    if (index == 293) { __bf_tramp_table[293] = (void*)(unsigned long long)__bf_tramp_table[293]; }
    if (index == 293) __bf_tramp_table[293] = dlsym(RTLD_DEFAULT, "if_nametoindex");
    if (index == 293) return __bf_tramp_table[293];
    if (index == 294) { __bf_tramp_table[294] = (void*)(unsigned long long)__bf_tramp_table[294]; }
    if (index == 294) __bf_tramp_table[294] = dlsym(RTLD_DEFAULT, "setvbuf");
    if (index == 294) return __bf_tramp_table[294];
    if (index == 295) { __bf_tramp_table[295] = (void*)(unsigned long long)__bf_tramp_table[295]; }
    if (index == 295) __bf_tramp_table[295] = dlsym(RTLD_DEFAULT, "realpath");
    if (index == 295) return __bf_tramp_table[295];
    if (index == 296) { __bf_tramp_table[296] = (void*)(unsigned long long)__bf_tramp_table[296]; }
    if (index == 296) __bf_tramp_table[296] = dlsym(RTLD_DEFAULT, "recvmmsg");
    if (index == 296) return __bf_tramp_table[296];
    if (index == 297) { __bf_tramp_table[297] = (void*)(unsigned long long)__bf_tramp_table[297]; }
    if (index == 297) __bf_tramp_table[297] = dlsym(RTLD_DEFAULT, "getcwd");
    if (index == 297) return __bf_tramp_table[297];
    if (index == 298) { __bf_tramp_table[298] = (void*)(unsigned long long)__bf_tramp_table[298]; }
    if (index == 298) __bf_tramp_table[298] = dlsym(RTLD_DEFAULT, "pread");
    if (index == 298) return __bf_tramp_table[298];
    if (index == 299) { __bf_tramp_table[299] = (void*)(unsigned long long)__bf_tramp_table[299]; }
    if (index == 299) __bf_tramp_table[299] = dlsym(RTLD_DEFAULT, "pwrite");
    if (index == 299) return __bf_tramp_table[299];
    if (index == 300) { __bf_tramp_table[300] = (void*)(unsigned long long)__bf_tramp_table[300]; }
    if (index == 300) __bf_tramp_table[300] = dlsym(RTLD_DEFAULT, "fchmod");
    if (index == 300) return __bf_tramp_table[300];
    if (index == 301) { __bf_tramp_table[301] = (void*)(unsigned long long)__bf_tramp_table[301]; }
    if (index == 301) __bf_tramp_table[301] = dlsym(RTLD_DEFAULT, "fchown");
    if (index == 301) return __bf_tramp_table[301];
    if (index == 302) { __bf_tramp_table[302] = (void*)(unsigned long long)__bf_tramp_table[302]; }
    if (index == 302) __bf_tramp_table[302] = dlsym(RTLD_DEFAULT, "mremap");
    if (index == 302) return __bf_tramp_table[302];
    if (index == 303) { __bf_tramp_table[303] = (void*)(unsigned long long)__bf_tramp_table[303]; }
    if (index == 303) __bf_tramp_table[303] = dlsym(RTLD_DEFAULT, "fsync");
    if (index == 303) return __bf_tramp_table[303];
    if (index == 304) { __bf_tramp_table[304] = (void*)(unsigned long long)__bf_tramp_table[304]; }
    if (index == 304) __bf_tramp_table[304] = dlsym(RTLD_DEFAULT, "utimes");
    if (index == 304) return __bf_tramp_table[304];
    if (index == 305) { __bf_tramp_table[305] = (void*)(unsigned long long)__bf_tramp_table[305]; }
    if (index == 305) __bf_tramp_table[305] = dlsym(RTLD_DEFAULT, "msync");
    if (index == 305) return __bf_tramp_table[305];
    if (index == 306) { __bf_tramp_table[306] = (void*)(unsigned long long)__bf_tramp_table[306]; }
    if (index == 306) __bf_tramp_table[306] = dlsym(RTLD_DEFAULT, "statvfs");
    if (index == 306) return __bf_tramp_table[306];
    if (index == 307) { __bf_tramp_table[307] = (void*)(unsigned long long)__bf_tramp_table[307]; }
    if (index == 307) __bf_tramp_table[307] = dlsym(RTLD_DEFAULT, "mallinfo");
    if (index == 307) return __bf_tramp_table[307];
    if (index == 308) { __bf_tramp_table[308] = (void*)(unsigned long long)__bf_tramp_table[308]; }
    if (index == 308) __bf_tramp_table[308] = dlsym(RTLD_DEFAULT, "__readlink_chk");
    if (index == 308) return __bf_tramp_table[308];
    if (index == 309) { __bf_tramp_table[309] = (void*)(unsigned long long)__bf_tramp_table[309]; }
    if (index == 309) __bf_tramp_table[309] = dlsym(RTLD_DEFAULT, "__gnu_strerror_r");
    if (index == 309) return __bf_tramp_table[309];
    if (index == 310) { __bf_tramp_table[310] = (void*)(unsigned long long)__bf_tramp_table[310]; }
    if (index == 310) __bf_tramp_table[310] = dlsym(RTLD_DEFAULT, "pthread_getschedparam");
    if (index == 310) return __bf_tramp_table[310];
    if (index == 311) { __bf_tramp_table[311] = (void*)(unsigned long long)__bf_tramp_table[311]; }
    if (index == 311) __bf_tramp_table[311] = dlsym(RTLD_DEFAULT, "sinf");
    if (index == 311) return __bf_tramp_table[311];
    if (index == 312) { __bf_tramp_table[312] = (void*)(unsigned long long)__bf_tramp_table[312]; }
    if (index == 312) __bf_tramp_table[312] = dlsym(RTLD_DEFAULT, "sincosf");
    if (index == 312) return __bf_tramp_table[312];
    if (index == 313) { __bf_tramp_table[313] = (void*)(unsigned long long)__bf_tramp_table[313]; }
    if (index == 313) __bf_tramp_table[313] = dlsym(RTLD_DEFAULT, "exp2");
    if (index == 313) return __bf_tramp_table[313];
    if (index == 314) { __bf_tramp_table[314] = (void*)(unsigned long long)__bf_tramp_table[314]; }
    if (index == 314) __bf_tramp_table[314] = dlsym(RTLD_DEFAULT, "sincos");
    if (index == 314) return __bf_tramp_table[314];
    if (index == 315) { __bf_tramp_table[315] = (void*)(unsigned long long)__bf_tramp_table[315]; }
    if (index == 315) __bf_tramp_table[315] = dlsym(RTLD_DEFAULT, "fmal");
    if (index == 315) return __bf_tramp_table[315];
    if (index == 316) { __bf_tramp_table[316] = (void*)(unsigned long long)__bf_tramp_table[316]; }
    if (index == 316) __bf_tramp_table[316] = dlsym(RTLD_DEFAULT, "exp2f");
    if (index == 316) return __bf_tramp_table[316];
    if (index == 317) { __bf_tramp_table[317] = (void*)(unsigned long long)__bf_tramp_table[317]; }
    if (index == 317) __bf_tramp_table[317] = dlsym(RTLD_DEFAULT, "log10f");
    if (index == 317) return __bf_tramp_table[317];
    if (index == 318) { __bf_tramp_table[318] = (void*)(unsigned long long)__bf_tramp_table[318]; }
    if (index == 318) __bf_tramp_table[318] = dlsym(RTLD_DEFAULT, "logf");
    if (index == 318) return __bf_tramp_table[318];
    if (index == 319) { __bf_tramp_table[319] = (void*)(unsigned long long)__bf_tramp_table[319]; }
    if (index == 319) __bf_tramp_table[319] = dlsym(RTLD_DEFAULT, "powf");
    if (index == 319) return __bf_tramp_table[319];
    if (index == 320) { __bf_tramp_table[320] = (void*)(unsigned long long)__bf_tramp_table[320]; }
    if (index == 320) __bf_tramp_table[320] = dlsym(RTLD_DEFAULT, "fmodf");
    if (index == 320) return __bf_tramp_table[320];
    if (index == 321) { __bf_tramp_table[321] = (void*)(unsigned long long)__bf_tramp_table[321]; }
    if (index == 321) __bf_tramp_table[321] = dlsym(RTLD_DEFAULT, "log2f");
    if (index == 321) return __bf_tramp_table[321];
    if (index == 322) { __bf_tramp_table[322] = (void*)(unsigned long long)__bf_tramp_table[322]; }
    if (index == 322) __bf_tramp_table[322] = dlsym(RTLD_DEFAULT, "expf");
    if (index == 322) return __bf_tramp_table[322];
    if (index == 323) { __bf_tramp_table[323] = (void*)(unsigned long long)__bf_tramp_table[323]; }
    if (index == 323) __bf_tramp_table[323] = dlsym(RTLD_DEFAULT, "powl");
    if (index == 323) return __bf_tramp_table[323];
    if (index == 324) { __bf_tramp_table[324] = (void*)(unsigned long long)__bf_tramp_table[324]; }
    if (index == 324) __bf_tramp_table[324] = dlsym(RTLD_DEFAULT, "cosf");
    if (index == 324) return __bf_tramp_table[324];
    if (index == 325) { __bf_tramp_table[325] = (void*)(unsigned long long)__bf_tramp_table[325]; }
    if (index == 325) __bf_tramp_table[325] = dlsym(RTLD_DEFAULT, "pthread_key_delete");
    if (index == 325) return __bf_tramp_table[325];
    if (index == 326) { __bf_tramp_table[326] = (void*)(unsigned long long)__bf_tramp_table[326]; }
    if (index == 326) __bf_tramp_table[326] = dlsym(RTLD_DEFAULT, "sysinfo");
    if (index == 326) return __bf_tramp_table[326];
    if (index == 327) { __bf_tramp_table[327] = (void*)(unsigned long long)__bf_tramp_table[327]; }
    if (index == 327) __bf_tramp_table[327] = dlsym(RTLD_DEFAULT, "madvise");
    if (index == 327) return __bf_tramp_table[327];
    if (index == 328) { __bf_tramp_table[328] = (void*)(unsigned long long)__bf_tramp_table[328]; }
    if (index == 328) __bf_tramp_table[328] = dlsym(RTLD_DEFAULT, "pthread_setname_np");
    if (index == 328) return __bf_tramp_table[328];
    if (index == 329) { __bf_tramp_table[329] = (void*)(unsigned long long)__bf_tramp_table[329]; }
    if (index == 329) __bf_tramp_table[329] = dlsym(RTLD_DEFAULT, "pthread_getattr_np");
    if (index == 329) return __bf_tramp_table[329];
    if (index == 330) { __bf_tramp_table[330] = (void*)(unsigned long long)__bf_tramp_table[330]; }
    if (index == 330) __bf_tramp_table[330] = dlsym(RTLD_DEFAULT, "pthread_attr_getstack");
    if (index == 330) return __bf_tramp_table[330];
    if (index == 331) { __bf_tramp_table[331] = (void*)(unsigned long long)__bf_tramp_table[331]; }
    if (index == 331) __bf_tramp_table[331] = dlsym(RTLD_DEFAULT, "mlock");
    if (index == 331) return __bf_tramp_table[331];
    if (index == 332) { __bf_tramp_table[332] = (void*)(unsigned long long)__bf_tramp_table[332]; }
    if (index == 332) __bf_tramp_table[332] = dlsym(RTLD_DEFAULT, "dl_iterate_phdr");
    if (index == 332) return __bf_tramp_table[332];
    if (index == 333) { __bf_tramp_table[333] = (void*)(unsigned long long)__bf_tramp_table[333]; }
    if (index == 333) __bf_tramp_table[333] = dlsym(RTLD_DEFAULT, "isspace");
    if (index == 333) return __bf_tramp_table[333];
    if (index == 334) { __bf_tramp_table[334] = (void*)(unsigned long long)__bf_tramp_table[334]; }
    if (index == 334) __bf_tramp_table[334] = dlsym(RTLD_DEFAULT, "gethostbyname");
    if (index == 334) return __bf_tramp_table[334];
    if (index == 335) { __bf_tramp_table[335] = (void*)(unsigned long long)__bf_tramp_table[335]; }
    if (index == 335) __bf_tramp_table[335] = dlsym(RTLD_DEFAULT, "strcat");
    if (index == 335) return __bf_tramp_table[335];
    if (index == 336) { __bf_tramp_table[336] = (void*)(unsigned long long)__bf_tramp_table[336]; }
    if (index == 336) __bf_tramp_table[336] = dlsym(RTLD_DEFAULT, "sendmmsg");
    if (index == 336) return __bf_tramp_table[336];
    if (index == 337) { __bf_tramp_table[337] = (void*)(unsigned long long)__bf_tramp_table[337]; }
    if (index == 337) __bf_tramp_table[337] = dlsym(RTLD_DEFAULT, "tolower");
    if (index == 337) return __bf_tramp_table[337];
    if (index == 338) { __bf_tramp_table[338] = (void*)(unsigned long long)__bf_tramp_table[338]; }
    if (index == 338) __bf_tramp_table[338] = dlsym(RTLD_DEFAULT, "pthread_rwlock_destroy");
    if (index == 338) return __bf_tramp_table[338];
    if (index == 339) { __bf_tramp_table[339] = (void*)(unsigned long long)__bf_tramp_table[339]; }
    if (index == 339) __bf_tramp_table[339] = dlsym(RTLD_DEFAULT, "pthread_rwlock_init");
    if (index == 339) return __bf_tramp_table[339];
    if (index == 340) { __bf_tramp_table[340] = (void*)(unsigned long long)__bf_tramp_table[340]; }
    if (index == 340) __bf_tramp_table[340] = dlsym(RTLD_DEFAULT, "pthread_rwlock_rdlock");
    if (index == 340) return __bf_tramp_table[340];
    if (index == 341) { __bf_tramp_table[341] = (void*)(unsigned long long)__bf_tramp_table[341]; }
    if (index == 341) __bf_tramp_table[341] = dlsym(RTLD_DEFAULT, "pthread_rwlock_unlock");
    if (index == 341) return __bf_tramp_table[341];
    if (index == 342) { __bf_tramp_table[342] = (void*)(unsigned long long)__bf_tramp_table[342]; }
    if (index == 342) __bf_tramp_table[342] = dlsym(RTLD_DEFAULT, "pthread_rwlock_wrlock");
    if (index == 342) return __bf_tramp_table[342];
    if (index == 343) { __bf_tramp_table[343] = (void*)(unsigned long long)__bf_tramp_table[343]; }
    if (index == 343) __bf_tramp_table[343] = dlsym(RTLD_DEFAULT, "signal");
    if (index == 343) return __bf_tramp_table[343];
    if (index == 344) { __bf_tramp_table[344] = (void*)(unsigned long long)__bf_tramp_table[344]; }
    if (index == 344) __bf_tramp_table[344] = dlsym(RTLD_DEFAULT, "tcgetattr");
    if (index == 344) return __bf_tramp_table[344];
    if (index == 345) { __bf_tramp_table[345] = (void*)(unsigned long long)__bf_tramp_table[345]; }
    if (index == 345) __bf_tramp_table[345] = dlsym(RTLD_DEFAULT, "tcsetattr");
    if (index == 345) return __bf_tramp_table[345];
    if (index == 346) { __bf_tramp_table[346] = (void*)(unsigned long long)__bf_tramp_table[346]; }
    if (index == 346) __bf_tramp_table[346] = dlsym(RTLD_DEFAULT, "utime");
    if (index == 346) return __bf_tramp_table[346];
    if (index == 347) { __bf_tramp_table[347] = (void*)(unsigned long long)__bf_tramp_table[347]; }
    if (index == 347) __bf_tramp_table[347] = dlsym(RTLD_DEFAULT, "vasprintf");
    if (index == 347) return __bf_tramp_table[347];
    if (index == 348) { __bf_tramp_table[348] = (void*)(unsigned long long)__bf_tramp_table[348]; }
    if (index == 348) __bf_tramp_table[348] = dlsym(RTLD_DEFAULT, "openlog");
    if (index == 348) return __bf_tramp_table[348];
    if (index == 349) { __bf_tramp_table[349] = (void*)(unsigned long long)__bf_tramp_table[349]; }
    if (index == 349) __bf_tramp_table[349] = dlsym(RTLD_DEFAULT, "syslog");
    if (index == 349) return __bf_tramp_table[349];
    if (index == 350) { __bf_tramp_table[350] = (void*)(unsigned long long)__bf_tramp_table[350]; }
    if (index == 350) __bf_tramp_table[350] = dlsym(RTLD_DEFAULT, "closelog");
    if (index == 350) return __bf_tramp_table[350];
    if (index == 351) { __bf_tramp_table[351] = (void*)(unsigned long long)__bf_tramp_table[351]; }
    if (index == 351) __bf_tramp_table[351] = dlsym(RTLD_DEFAULT, "ungetc");
    if (index == 351) return __bf_tramp_table[351];
    if (index == 352) { __bf_tramp_table[352] = (void*)(unsigned long long)__bf_tramp_table[352]; }
    if (index == 352) __bf_tramp_table[352] = dlsym(RTLD_DEFAULT, "getc");
    if (index == 352) return __bf_tramp_table[352];
    if (index == 353) { __bf_tramp_table[353] = (void*)(unsigned long long)__bf_tramp_table[353]; }
    if (index == 353) __bf_tramp_table[353] = dlsym(RTLD_DEFAULT, "ungetwc");
    if (index == 353) return __bf_tramp_table[353];
    if (index == 354) { __bf_tramp_table[354] = (void*)(unsigned long long)__bf_tramp_table[354]; }
    if (index == 354) __bf_tramp_table[354] = dlsym(RTLD_DEFAULT, "getwc");
    if (index == 354) return __bf_tramp_table[354];
    if (index == 355) { __bf_tramp_table[355] = (void*)(unsigned long long)__bf_tramp_table[355]; }
    if (index == 355) __bf_tramp_table[355] = dlsym(RTLD_DEFAULT, "fputwc");
    if (index == 355) return __bf_tramp_table[355];
    if (index == 356) { __bf_tramp_table[356] = (void*)(unsigned long long)__bf_tramp_table[356]; }
    if (index == 356) __bf_tramp_table[356] = dlsym(RTLD_DEFAULT, "newlocale");
    if (index == 356) return __bf_tramp_table[356];
    if (index == 357) { __bf_tramp_table[357] = (void*)(unsigned long long)__bf_tramp_table[357]; }
    if (index == 357) __bf_tramp_table[357] = dlsym(RTLD_DEFAULT, "uselocale");
    if (index == 357) return __bf_tramp_table[357];
    if (index == 358) { __bf_tramp_table[358] = (void*)(unsigned long long)__bf_tramp_table[358]; }
    if (index == 358) __bf_tramp_table[358] = dlsym(RTLD_DEFAULT, "vsscanf");
    if (index == 358) return __bf_tramp_table[358];
    if (index == 359) { __bf_tramp_table[359] = (void*)(unsigned long long)__bf_tramp_table[359]; }
    if (index == 359) __bf_tramp_table[359] = dlsym(RTLD_DEFAULT, "strftime_l");
    if (index == 359) return __bf_tramp_table[359];
    if (index == 360) { __bf_tramp_table[360] = (void*)(unsigned long long)__bf_tramp_table[360]; }
    if (index == 360) __bf_tramp_table[360] = dlsym(RTLD_DEFAULT, "mbsrtowcs");
    if (index == 360) return __bf_tramp_table[360];
    if (index == 361) { __bf_tramp_table[361] = (void*)(unsigned long long)__bf_tramp_table[361]; }
    if (index == 361) __bf_tramp_table[361] = dlsym(RTLD_DEFAULT, "freelocale");
    if (index == 361) return __bf_tramp_table[361];
    if (index == 362) { __bf_tramp_table[362] = (void*)(unsigned long long)__bf_tramp_table[362]; }
    if (index == 362) __bf_tramp_table[362] = dlsym(RTLD_DEFAULT, "strcoll_l");
    if (index == 362) return __bf_tramp_table[362];
    if (index == 363) { __bf_tramp_table[363] = (void*)(unsigned long long)__bf_tramp_table[363]; }
    if (index == 363) __bf_tramp_table[363] = dlsym(RTLD_DEFAULT, "strxfrm_l");
    if (index == 363) return __bf_tramp_table[363];
    if (index == 364) { __bf_tramp_table[364] = (void*)(unsigned long long)__bf_tramp_table[364]; }
    if (index == 364) __bf_tramp_table[364] = dlsym(RTLD_DEFAULT, "wcscoll_l");
    if (index == 364) return __bf_tramp_table[364];
    if (index == 365) { __bf_tramp_table[365] = (void*)(unsigned long long)__bf_tramp_table[365]; }
    if (index == 365) __bf_tramp_table[365] = dlsym(RTLD_DEFAULT, "wcsxfrm_l");
    if (index == 365) return __bf_tramp_table[365];
    if (index == 366) { __bf_tramp_table[366] = (void*)(unsigned long long)__bf_tramp_table[366]; }
    if (index == 366) __bf_tramp_table[366] = dlsym(RTLD_DEFAULT, "iswlower_l");
    if (index == 366) return __bf_tramp_table[366];
    if (index == 367) { __bf_tramp_table[367] = (void*)(unsigned long long)__bf_tramp_table[367]; }
    if (index == 367) __bf_tramp_table[367] = dlsym(RTLD_DEFAULT, "iswspace_l");
    if (index == 367) return __bf_tramp_table[367];
    if (index == 368) { __bf_tramp_table[368] = (void*)(unsigned long long)__bf_tramp_table[368]; }
    if (index == 368) __bf_tramp_table[368] = dlsym(RTLD_DEFAULT, "iswprint_l");
    if (index == 368) return __bf_tramp_table[368];
    if (index == 369) { __bf_tramp_table[369] = (void*)(unsigned long long)__bf_tramp_table[369]; }
    if (index == 369) __bf_tramp_table[369] = dlsym(RTLD_DEFAULT, "iswblank_l");
    if (index == 369) return __bf_tramp_table[369];
    if (index == 370) { __bf_tramp_table[370] = (void*)(unsigned long long)__bf_tramp_table[370]; }
    if (index == 370) __bf_tramp_table[370] = dlsym(RTLD_DEFAULT, "iswcntrl_l");
    if (index == 370) return __bf_tramp_table[370];
    if (index == 371) { __bf_tramp_table[371] = (void*)(unsigned long long)__bf_tramp_table[371]; }
    if (index == 371) __bf_tramp_table[371] = dlsym(RTLD_DEFAULT, "iswupper_l");
    if (index == 371) return __bf_tramp_table[371];
    if (index == 372) { __bf_tramp_table[372] = (void*)(unsigned long long)__bf_tramp_table[372]; }
    if (index == 372) __bf_tramp_table[372] = dlsym(RTLD_DEFAULT, "iswalpha_l");
    if (index == 372) return __bf_tramp_table[372];
    if (index == 373) { __bf_tramp_table[373] = (void*)(unsigned long long)__bf_tramp_table[373]; }
    if (index == 373) __bf_tramp_table[373] = dlsym(RTLD_DEFAULT, "iswdigit_l");
    if (index == 373) return __bf_tramp_table[373];
    if (index == 374) { __bf_tramp_table[374] = (void*)(unsigned long long)__bf_tramp_table[374]; }
    if (index == 374) __bf_tramp_table[374] = dlsym(RTLD_DEFAULT, "iswpunct_l");
    if (index == 374) return __bf_tramp_table[374];
    if (index == 375) { __bf_tramp_table[375] = (void*)(unsigned long long)__bf_tramp_table[375]; }
    if (index == 375) __bf_tramp_table[375] = dlsym(RTLD_DEFAULT, "iswxdigit_l");
    if (index == 375) return __bf_tramp_table[375];
    if (index == 376) { __bf_tramp_table[376] = (void*)(unsigned long long)__bf_tramp_table[376]; }
    if (index == 376) __bf_tramp_table[376] = dlsym(RTLD_DEFAULT, "towupper_l");
    if (index == 376) return __bf_tramp_table[376];
    if (index == 377) { __bf_tramp_table[377] = (void*)(unsigned long long)__bf_tramp_table[377]; }
    if (index == 377) __bf_tramp_table[377] = dlsym(RTLD_DEFAULT, "towlower_l");
    if (index == 377) return __bf_tramp_table[377];
    if (index == 378) { __bf_tramp_table[378] = (void*)(unsigned long long)__bf_tramp_table[378]; }
    if (index == 378) __bf_tramp_table[378] = dlsym(RTLD_DEFAULT, "btowc");
    if (index == 378) return __bf_tramp_table[378];
    if (index == 379) { __bf_tramp_table[379] = (void*)(unsigned long long)__bf_tramp_table[379]; }
    if (index == 379) __bf_tramp_table[379] = dlsym(RTLD_DEFAULT, "wctob");
    if (index == 379) return __bf_tramp_table[379];
    if (index == 380) { __bf_tramp_table[380] = (void*)(unsigned long long)__bf_tramp_table[380]; }
    if (index == 380) __bf_tramp_table[380] = dlsym(RTLD_DEFAULT, "wcsnrtombs");
    if (index == 380) return __bf_tramp_table[380];
    if (index == 381) { __bf_tramp_table[381] = (void*)(unsigned long long)__bf_tramp_table[381]; }
    if (index == 381) __bf_tramp_table[381] = dlsym(RTLD_DEFAULT, "wcrtomb");
    if (index == 381) return __bf_tramp_table[381];
    if (index == 382) { __bf_tramp_table[382] = (void*)(unsigned long long)__bf_tramp_table[382]; }
    if (index == 382) __bf_tramp_table[382] = dlsym(RTLD_DEFAULT, "mbsnrtowcs");
    if (index == 382) return __bf_tramp_table[382];
    if (index == 383) { __bf_tramp_table[383] = (void*)(unsigned long long)__bf_tramp_table[383]; }
    if (index == 383) __bf_tramp_table[383] = dlsym(RTLD_DEFAULT, "mbrtowc");
    if (index == 383) return __bf_tramp_table[383];
    if (index == 384) { __bf_tramp_table[384] = (void*)(unsigned long long)__bf_tramp_table[384]; }
    if (index == 384) __bf_tramp_table[384] = dlsym(RTLD_DEFAULT, "mbtowc");
    if (index == 384) return __bf_tramp_table[384];
    if (index == 385) { __bf_tramp_table[385] = (void*)(unsigned long long)__bf_tramp_table[385]; }
    if (index == 385) __bf_tramp_table[385] = dlsym(RTLD_DEFAULT, "__ctype_get_mb_cur_max");
    if (index == 385) return __bf_tramp_table[385];
    if (index == 386) { __bf_tramp_table[386] = (void*)(unsigned long long)__bf_tramp_table[386]; }
    if (index == 386) __bf_tramp_table[386] = dlsym(RTLD_DEFAULT, "mbrlen");
    if (index == 386) return __bf_tramp_table[386];
    if (index == 387) { __bf_tramp_table[387] = (void*)(unsigned long long)__bf_tramp_table[387]; }
    if (index == 387) __bf_tramp_table[387] = dlsym(RTLD_DEFAULT, "strtoll_l");
    if (index == 387) return __bf_tramp_table[387];
    if (index == 388) { __bf_tramp_table[388] = (void*)(unsigned long long)__bf_tramp_table[388]; }
    if (index == 388) __bf_tramp_table[388] = dlsym(RTLD_DEFAULT, "strtoull_l");
    if (index == 388) return __bf_tramp_table[388];
    if (index == 389) { __bf_tramp_table[389] = (void*)(unsigned long long)__bf_tramp_table[389]; }
    if (index == 389) __bf_tramp_table[389] = dlsym(RTLD_DEFAULT, "strtold_l");
    if (index == 389) return __bf_tramp_table[389];
    if (index == 390) { __bf_tramp_table[390] = (void*)(unsigned long long)__bf_tramp_table[390]; }
    if (index == 390) __bf_tramp_table[390] = dlsym(RTLD_DEFAULT, "__cxa_thread_atexit_impl");
    if (index == 390) return __bf_tramp_table[390];
    if (index == 391) { __bf_tramp_table[391] = (void*)(unsigned long long)__bf_tramp_table[391]; }
    if (index == 391) __bf_tramp_table[391] = dlsym(RTLD_DEFAULT, "strncat");
    if (index == 391) return __bf_tramp_table[391];
    return NULL;
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
    dlclose(self);
}