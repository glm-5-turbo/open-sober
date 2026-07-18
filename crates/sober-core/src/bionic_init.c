/* Auto-generated Bionic shim C init. DO NOT EDIT.
 * 392 function pointers, 13 data pointers.
 */

#define _GNU_SOURCE
#include <stddef.h>
#include <stdint.h>
#include <dlfcn.h>

/* Defined in bionic_shim.S */
extern void *__bf_tramp_table[392];

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

/* ===== Bionic-only function stubs (no glibc equivalent) ===== */

/* __system_property_get */
__asm__(".symver __system_property_get,__system_property_get@@LIBC");
int __system_property_get(const char *key, char *value) {
    (void)key; if (value) value[0] = '\0'; return 0;
}

/* android_set_abort_message */
__asm__(".symver android_set_abort_message,android_set_abort_message@@LIBC");
void android_set_abort_message(const char *msg) { (void)msg; }

/* arc4random_buf — via syscalls to avoid libc dependency */
__asm__(".symver arc4random_buf,arc4random_buf@@LIBC");
void arc4random_buf(void *buf, size_t n) {
    long fd;
    register long x0 asm("x0") = -100;
    register const char *x1 asm("x1") = "/dev/urandom";
    register long x2 asm("x2") = 0;
    register long x8 asm("x8") = 56;
    asm volatile("svc #0" : "=r"(x0) : "r"(x0), "r"(x1), "r"(x2), "r"(x8));
    fd = x0;
    if (fd >= 0) {
        register long r0 asm("x0") = fd;
        register void *r1 asm("x1") = buf;
        register size_t r2 asm("x2") = n;
        register long r8 asm("x8") = 63;
        asm volatile("svc #0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r8));
        register long c0 asm("x0") = fd;
        register long r8c asm("x8") = 57;
        asm volatile("svc #0" : : "r"(c0), "r"(r8c));
    }
}

/* __assert2 */
__asm__(".symver __assert2,__assert2@@LIBC");
void __assert2(const char *file, int line, const char *func, const char *failed) {
    (void)file; (void)line; (void)func; (void)failed;
    register long r0 asm("x0") = 2;
    const char *msg = "Assertion failed\n";
    register const char *r1 asm("x1") = msg;
    register long r2 asm("x2") = 16;
    register long r8 asm("x8") = 64;
    asm volatile("svc #0" : : "r"(r0), "r"(r1), "r"(r2), "r"(r8));
    asm volatile("udf #0");
}

/* __strncpy_chk2 — Bionic fortified strncpy */
__asm__(".symver __strncpy_chk2,__strncpy_chk2@@LIBC");
char *__strncpy_chk2(char *dst, const char *src, size_t n, size_t dest_len) {
    (void)dest_len;
    char *d = dst; const char *s = src; size_t i;
    for (i = 0; i < n && *s; i++) *d++ = *s++;
    for (; i < n; i++) *d++ = '\0';
    return dst;
}

/* ===== Constructor: fill dispatch table and init data ===== */
__attribute__((constructor))
static void init_bionic_shim(void) {
    void *self = dlopen(NULL, RTLD_LAZY);
    if (!self) return;

    __bf_tramp_table[0] = dlsym(RTLD_NEXT, "__cxa_finalize");
    if (!__bf_tramp_table[0])
        __bf_tramp_table[0] = dlsym(self, "__cxa_finalize");
    __bf_tramp_table[1] = dlsym(RTLD_NEXT, "__cxa_atexit");
    if (!__bf_tramp_table[1])
        __bf_tramp_table[1] = dlsym(self, "__cxa_atexit");
    __bf_tramp_table[2] = dlsym(RTLD_NEXT, "__register_atfork");
    if (!__bf_tramp_table[2])
        __bf_tramp_table[2] = dlsym(self, "__register_atfork");
    __bf_tramp_table[3] = dlsym(RTLD_NEXT, "strlen");
    if (!__bf_tramp_table[3])
        __bf_tramp_table[3] = dlsym(self, "strlen");
    __bf_tramp_table[4] = dlsym(RTLD_NEXT, "memcmp");
    if (!__bf_tramp_table[4])
        __bf_tramp_table[4] = dlsym(self, "memcmp");
    __bf_tramp_table[5] = dlsym(RTLD_NEXT, "pthread_mutex_init");
    if (!__bf_tramp_table[5])
        __bf_tramp_table[5] = dlsym(self, "pthread_mutex_init");
    __bf_tramp_table[6] = dlsym(RTLD_NEXT, "pthread_mutex_destroy");
    if (!__bf_tramp_table[6])
        __bf_tramp_table[6] = dlsym(self, "pthread_mutex_destroy");
    __bf_tramp_table[7] = dlsym(RTLD_NEXT, "pthread_once");
    if (!__bf_tramp_table[7])
        __bf_tramp_table[7] = dlsym(self, "pthread_once");
    __bf_tramp_table[8] = dlsym(RTLD_NEXT, "__memset_chk");
    if (!__bf_tramp_table[8])
        __bf_tramp_table[8] = dlsym(self, "__memset_chk");
    __bf_tramp_table[9] = dlsym(RTLD_NEXT, "__memcpy_chk");
    if (!__bf_tramp_table[9])
        __bf_tramp_table[9] = dlsym(self, "__memcpy_chk");
    __bf_tramp_table[10] = dlsym(RTLD_NEXT, "__strlen_chk");
    if (!__bf_tramp_table[10])
        __bf_tramp_table[10] = dlsym(self, "__strlen_chk");
    __bf_tramp_table[11] = dlsym(RTLD_NEXT, "memchr");
    if (!__bf_tramp_table[11])
        __bf_tramp_table[11] = dlsym(self, "memchr");
    __bf_tramp_table[12] = dlsym(RTLD_NEXT, "dladdr");
    if (!__bf_tramp_table[12])
        __bf_tramp_table[12] = dlsym(self, "dladdr");
    __bf_tramp_table[13] = dlsym(RTLD_NEXT, "dlerror");
    if (!__bf_tramp_table[13])
        __bf_tramp_table[13] = dlsym(self, "dlerror");
    __bf_tramp_table[14] = dlsym(RTLD_NEXT, "dlopen");
    if (!__bf_tramp_table[14])
        __bf_tramp_table[14] = dlsym(self, "dlopen");
    __bf_tramp_table[15] = dlsym(RTLD_NEXT, "dlsym");
    if (!__bf_tramp_table[15])
        __bf_tramp_table[15] = dlsym(self, "dlsym");
    __bf_tramp_table[16] = dlsym(RTLD_NEXT, "strncmp");
    if (!__bf_tramp_table[16])
        __bf_tramp_table[16] = dlsym(self, "strncmp");
    __bf_tramp_table[17] = dlsym(RTLD_NEXT, "strcmp");
    if (!__bf_tramp_table[17])
        __bf_tramp_table[17] = dlsym(self, "strcmp");
    __bf_tramp_table[18] = dlsym(RTLD_NEXT, "getauxval");
    if (!__bf_tramp_table[18])
        __bf_tramp_table[18] = dlsym(self, "getauxval");
    __bf_tramp_table[19] = dlsym(RTLD_NEXT, "__errno_location");
    if (!__bf_tramp_table[19])
        __bf_tramp_table[19] = dlsym(self, "__errno_location");
    __bf_tramp_table[20] = dlsym(RTLD_NEXT, "close");
    if (!__bf_tramp_table[20])
        __bf_tramp_table[20] = dlsym(self, "close");
    __bf_tramp_table[21] = dlsym(RTLD_NEXT, "__open_2");
    if (!__bf_tramp_table[21])
        __bf_tramp_table[21] = dlsym(self, "__open_2");
    __bf_tramp_table[22] = dlsym(RTLD_NEXT, "__read_chk");
    if (!__bf_tramp_table[22])
        __bf_tramp_table[22] = dlsym(self, "__read_chk");
    __bf_tramp_table[23] = dlsym(RTLD_NEXT, "read");
    if (!__bf_tramp_table[23])
        __bf_tramp_table[23] = dlsym(self, "read");
    __bf_tramp_table[24] = dlsym(RTLD_NEXT, "clock_gettime");
    if (!__bf_tramp_table[24])
        __bf_tramp_table[24] = dlsym(self, "clock_gettime");
    __bf_tramp_table[25] = dlsym(RTLD_NEXT, "syscall");
    if (!__bf_tramp_table[25])
        __bf_tramp_table[25] = dlsym(self, "syscall");
    __bf_tramp_table[26] = dlsym(RTLD_NEXT, "sched_getcpu");
    if (!__bf_tramp_table[26])
        __bf_tramp_table[26] = dlsym(self, "sched_getcpu");
    __bf_tramp_table[27] = dlsym(RTLD_NEXT, "sysconf");
    if (!__bf_tramp_table[27])
        __bf_tramp_table[27] = dlsym(self, "sysconf");
    __bf_tramp_table[28] = dlsym(RTLD_NEXT, "mmap");
    if (!__bf_tramp_table[28])
        __bf_tramp_table[28] = dlsym(self, "mmap");
    __bf_tramp_table[29] = dlsym(RTLD_NEXT, "mprotect");
    if (!__bf_tramp_table[29])
        __bf_tramp_table[29] = dlsym(self, "mprotect");
    __bf_tramp_table[30] = dlsym(RTLD_NEXT, "munmap");
    if (!__bf_tramp_table[30])
        __bf_tramp_table[30] = dlsym(self, "munmap");
    __bf_tramp_table[31] = dlsym(RTLD_NEXT, "pthread_attr_init");
    if (!__bf_tramp_table[31])
        __bf_tramp_table[31] = dlsym(self, "pthread_attr_init");
    __bf_tramp_table[32] = dlsym(RTLD_NEXT, "pthread_attr_setstacksize");
    if (!__bf_tramp_table[32])
        __bf_tramp_table[32] = dlsym(self, "pthread_attr_setstacksize");
    __bf_tramp_table[33] = dlsym(RTLD_NEXT, "pthread_create");
    if (!__bf_tramp_table[33])
        __bf_tramp_table[33] = dlsym(self, "pthread_create");
    __bf_tramp_table[34] = dlsym(RTLD_NEXT, "pthread_attr_destroy");
    if (!__bf_tramp_table[34])
        __bf_tramp_table[34] = dlsym(self, "pthread_attr_destroy");
    __bf_tramp_table[35] = dlsym(RTLD_NEXT, "pthread_join");
    if (!__bf_tramp_table[35])
        __bf_tramp_table[35] = dlsym(self, "pthread_join");
    __bf_tramp_table[36] = dlsym(RTLD_NEXT, "pthread_self");
    if (!__bf_tramp_table[36])
        __bf_tramp_table[36] = dlsym(self, "pthread_self");
    __bf_tramp_table[37] = dlsym(RTLD_NEXT, "memset");
    if (!__bf_tramp_table[37])
        __bf_tramp_table[37] = dlsym(self, "memset");
    __bf_tramp_table[38] = dlsym(RTLD_NEXT, "pthread_setspecific");
    if (!__bf_tramp_table[38])
        __bf_tramp_table[38] = dlsym(self, "pthread_setspecific");
    __bf_tramp_table[39] = dlsym(RTLD_NEXT, "__strncpy_chk");
    if (!__bf_tramp_table[39])
        __bf_tramp_table[39] = dlsym(self, "__strncpy_chk");
    __bf_tramp_table[40] = dlsym(RTLD_NEXT, "sched_get_priority_max");
    if (!__bf_tramp_table[40])
        __bf_tramp_table[40] = dlsym(self, "sched_get_priority_max");
    __bf_tramp_table[41] = dlsym(RTLD_NEXT, "sched_setscheduler");
    if (!__bf_tramp_table[41])
        __bf_tramp_table[41] = dlsym(self, "sched_setscheduler");
    __bf_tramp_table[42] = dlsym(RTLD_NEXT, "pthread_mutex_lock");
    if (!__bf_tramp_table[42])
        __bf_tramp_table[42] = dlsym(self, "pthread_mutex_lock");
    __bf_tramp_table[43] = dlsym(RTLD_NEXT, "pthread_cond_wait");
    if (!__bf_tramp_table[43])
        __bf_tramp_table[43] = dlsym(self, "pthread_cond_wait");
    __bf_tramp_table[44] = dlsym(RTLD_NEXT, "pthread_mutex_unlock");
    if (!__bf_tramp_table[44])
        __bf_tramp_table[44] = dlsym(self, "pthread_mutex_unlock");
    __bf_tramp_table[45] = dlsym(RTLD_NEXT, "pthread_cond_signal");
    if (!__bf_tramp_table[45])
        __bf_tramp_table[45] = dlsym(self, "pthread_cond_signal");
    __bf_tramp_table[46] = dlsym(RTLD_NEXT, "pthread_cond_init");
    if (!__bf_tramp_table[46])
        __bf_tramp_table[46] = dlsym(self, "pthread_cond_init");
    __bf_tramp_table[47] = dlsym(RTLD_NEXT, "gettid");
    if (!__bf_tramp_table[47])
        __bf_tramp_table[47] = dlsym(self, "gettid");
    __bf_tramp_table[48] = dlsym(RTLD_NEXT, "__vsprintf_chk");
    if (!__bf_tramp_table[48])
        __bf_tramp_table[48] = dlsym(self, "__vsprintf_chk");
    __bf_tramp_table[49] = dlsym(RTLD_NEXT, "atan2f");
    if (!__bf_tramp_table[49])
        __bf_tramp_table[49] = dlsym(self, "atan2f");
    __bf_tramp_table[50] = dlsym(RTLD_NEXT, "vsnprintf");
    if (!__bf_tramp_table[50])
        __bf_tramp_table[50] = dlsym(self, "vsnprintf");
    __bf_tramp_table[51] = dlsym(RTLD_NEXT, "fprintf");
    if (!__bf_tramp_table[51])
        __bf_tramp_table[51] = dlsym(self, "fprintf");
    __bf_tramp_table[52] = dlsym(RTLD_NEXT, "rand");
    if (!__bf_tramp_table[52])
        __bf_tramp_table[52] = dlsym(self, "rand");
    __bf_tramp_table[53] = dlsym(RTLD_NEXT, "strtoll");
    if (!__bf_tramp_table[53])
        __bf_tramp_table[53] = dlsym(self, "strtoll");
    __bf_tramp_table[54] = dlsym(RTLD_NEXT, "time");
    if (!__bf_tramp_table[54])
        __bf_tramp_table[54] = dlsym(self, "time");
    __bf_tramp_table[55] = dlsym(RTLD_NEXT, "pthread_cond_broadcast");
    if (!__bf_tramp_table[55])
        __bf_tramp_table[55] = dlsym(self, "pthread_cond_broadcast");
    __bf_tramp_table[56] = dlsym(RTLD_NEXT, "pthread_cond_destroy");
    if (!__bf_tramp_table[56])
        __bf_tramp_table[56] = dlsym(self, "pthread_cond_destroy");
    __bf_tramp_table[57] = dlsym(RTLD_NEXT, "stat");
    if (!__bf_tramp_table[57])
        __bf_tramp_table[57] = dlsym(self, "stat");
    __bf_tramp_table[58] = dlsym(RTLD_NEXT, "opendir");
    if (!__bf_tramp_table[58])
        __bf_tramp_table[58] = dlsym(self, "opendir");
    __bf_tramp_table[59] = dlsym(RTLD_NEXT, "readdir");
    if (!__bf_tramp_table[59])
        __bf_tramp_table[59] = dlsym(self, "readdir");
    __bf_tramp_table[60] = dlsym(RTLD_NEXT, "closedir");
    if (!__bf_tramp_table[60])
        __bf_tramp_table[60] = dlsym(self, "closedir");
    __bf_tramp_table[61] = dlsym(RTLD_NEXT, "posix_fallocate");
    if (!__bf_tramp_table[61])
        __bf_tramp_table[61] = dlsym(self, "posix_fallocate");
    __bf_tramp_table[62] = dlsym(RTLD_NEXT, "open");
    if (!__bf_tramp_table[62])
        __bf_tramp_table[62] = dlsym(self, "open");
    __bf_tramp_table[63] = dlsym(RTLD_NEXT, "pthread_equal");
    if (!__bf_tramp_table[63])
        __bf_tramp_table[63] = dlsym(self, "pthread_equal");
    __bf_tramp_table[64] = dlsym(RTLD_NEXT, "atoll");
    if (!__bf_tramp_table[64])
        __bf_tramp_table[64] = dlsym(self, "atoll");
    __bf_tramp_table[65] = dlsym(RTLD_NEXT, "srand");
    if (!__bf_tramp_table[65])
        __bf_tramp_table[65] = dlsym(self, "srand");
    __bf_tramp_table[66] = dlsym(RTLD_NEXT, "strtod");
    if (!__bf_tramp_table[66])
        __bf_tramp_table[66] = dlsym(self, "strtod");
    __bf_tramp_table[67] = dlsym(RTLD_NEXT, "localtime");
    if (!__bf_tramp_table[67])
        __bf_tramp_table[67] = dlsym(self, "localtime");
    __bf_tramp_table[68] = dlsym(RTLD_NEXT, "fseek");
    if (!__bf_tramp_table[68])
        __bf_tramp_table[68] = dlsym(self, "fseek");
    __bf_tramp_table[69] = dlsym(RTLD_NEXT, "ftell");
    if (!__bf_tramp_table[69])
        __bf_tramp_table[69] = dlsym(self, "ftell");
    __bf_tramp_table[70] = dlsym(RTLD_NEXT, "fclose");
    if (!__bf_tramp_table[70])
        __bf_tramp_table[70] = dlsym(self, "fclose");
    __bf_tramp_table[71] = dlsym(RTLD_NEXT, "atan");
    if (!__bf_tramp_table[71])
        __bf_tramp_table[71] = dlsym(self, "atan");
    __bf_tramp_table[72] = dlsym(RTLD_NEXT, "mkdir");
    if (!__bf_tramp_table[72])
        __bf_tramp_table[72] = dlsym(self, "mkdir");
    __bf_tramp_table[73] = dlsym(RTLD_NEXT, "__fread_chk");
    if (!__bf_tramp_table[73])
        __bf_tramp_table[73] = dlsym(self, "__fread_chk");
    __bf_tramp_table[74] = dlsym(RTLD_NEXT, "fread");
    if (!__bf_tramp_table[74])
        __bf_tramp_table[74] = dlsym(self, "fread");
    __bf_tramp_table[75] = dlsym(RTLD_NEXT, "asinf");
    if (!__bf_tramp_table[75])
        __bf_tramp_table[75] = dlsym(self, "asinf");
    __bf_tramp_table[76] = dlsym(RTLD_NEXT, "dlclose");
    if (!__bf_tramp_table[76])
        __bf_tramp_table[76] = dlsym(self, "dlclose");
    __bf_tramp_table[77] = dlsym(RTLD_NEXT, "access");
    if (!__bf_tramp_table[77])
        __bf_tramp_table[77] = dlsym(self, "access");
    __bf_tramp_table[78] = dlsym(RTLD_NEXT, "getenv");
    if (!__bf_tramp_table[78])
        __bf_tramp_table[78] = dlsym(self, "getenv");
    __bf_tramp_table[79] = dlsym(RTLD_NEXT, "strncpy");
    if (!__bf_tramp_table[79])
        __bf_tramp_table[79] = dlsym(self, "strncpy");
    __bf_tramp_table[80] = dlsym(RTLD_NEXT, "feof");
    if (!__bf_tramp_table[80])
        __bf_tramp_table[80] = dlsym(self, "feof");
    __bf_tramp_table[81] = dlsym(RTLD_NEXT, "fopen");
    if (!__bf_tramp_table[81])
        __bf_tramp_table[81] = dlsym(self, "fopen");
    __bf_tramp_table[82] = dlsym(RTLD_NEXT, "fseeko");
    if (!__bf_tramp_table[82])
        __bf_tramp_table[82] = dlsym(self, "fseeko");
    __bf_tramp_table[83] = dlsym(RTLD_NEXT, "ftello");
    if (!__bf_tramp_table[83])
        __bf_tramp_table[83] = dlsym(self, "ftello");
    __bf_tramp_table[84] = dlsym(RTLD_NEXT, "fwrite");
    if (!__bf_tramp_table[84])
        __bf_tramp_table[84] = dlsym(self, "fwrite");
    __bf_tramp_table[85] = dlsym(RTLD_NEXT, "fflush");
    if (!__bf_tramp_table[85])
        __bf_tramp_table[85] = dlsym(self, "fflush");
    __bf_tramp_table[86] = dlsym(RTLD_NEXT, "gmtime");
    if (!__bf_tramp_table[86])
        __bf_tramp_table[86] = dlsym(self, "gmtime");
    __bf_tramp_table[87] = dlsym(RTLD_NEXT, "mktime");
    if (!__bf_tramp_table[87])
        __bf_tramp_table[87] = dlsym(self, "mktime");
    __bf_tramp_table[88] = dlsym(RTLD_NEXT, "pipe");
    if (!__bf_tramp_table[88])
        __bf_tramp_table[88] = dlsym(self, "pipe");
    __bf_tramp_table[89] = dlsym(RTLD_NEXT, "write");
    if (!__bf_tramp_table[89])
        __bf_tramp_table[89] = dlsym(self, "write");
    __bf_tramp_table[90] = dlsym(RTLD_NEXT, "pthread_key_create");
    if (!__bf_tramp_table[90])
        __bf_tramp_table[90] = dlsym(self, "pthread_key_create");
    __bf_tramp_table[91] = dlsym(RTLD_NEXT, "abort");
    if (!__bf_tramp_table[91])
        __bf_tramp_table[91] = dlsym(self, "abort");
    __bf_tramp_table[92] = dlsym(RTLD_NEXT, "__assert");
    if (!__bf_tramp_table[92])
        __bf_tramp_table[92] = dlsym(self, "__assert");
    __bf_tramp_table[93] = dlsym(RTLD_NEXT, "strrchr");
    if (!__bf_tramp_table[93])
        __bf_tramp_table[93] = dlsym(self, "strrchr");
    __bf_tramp_table[94] = dlsym(RTLD_NEXT, "__stack_chk_fail");
    if (!__bf_tramp_table[94])
        __bf_tramp_table[94] = dlsym(self, "__stack_chk_fail");
    __bf_tramp_table[95] = dlsym(RTLD_NEXT, "atoi");
    if (!__bf_tramp_table[95])
        __bf_tramp_table[95] = dlsym(self, "atoi");
    __bf_tramp_table[96] = dlsym(RTLD_NEXT, "fcntl");
    if (!__bf_tramp_table[96])
        __bf_tramp_table[96] = dlsym(self, "fcntl");
    __bf_tramp_table[97] = dlsym(RTLD_NEXT, "memcpy");
    if (!__bf_tramp_table[97])
        __bf_tramp_table[97] = dlsym(self, "memcpy");
    __bf_tramp_table[98] = dlsym(RTLD_NEXT, "strcpy");
    if (!__bf_tramp_table[98])
        __bf_tramp_table[98] = dlsym(self, "strcpy");
    __bf_tramp_table[99] = dlsym(RTLD_NEXT, "strerror");
    if (!__bf_tramp_table[99])
        __bf_tramp_table[99] = dlsym(self, "strerror");
    __bf_tramp_table[100] = dlsym(RTLD_NEXT, "pthread_attr_setdetachstate");
    if (!__bf_tramp_table[100])
        __bf_tramp_table[100] = dlsym(self, "pthread_attr_setdetachstate");
    __bf_tramp_table[101] = dlsym(RTLD_NEXT, "pthread_cond_timedwait");
    if (!__bf_tramp_table[101])
        __bf_tramp_table[101] = dlsym(self, "pthread_cond_timedwait");
    __bf_tramp_table[102] = dlsym(RTLD_NEXT, "memmove");
    if (!__bf_tramp_table[102])
        __bf_tramp_table[102] = dlsym(self, "memmove");
    __bf_tramp_table[103] = dlsym(RTLD_NEXT, "strtol");
    if (!__bf_tramp_table[103])
        __bf_tramp_table[103] = dlsym(self, "strtol");
    __bf_tramp_table[104] = dlsym(RTLD_NEXT, "getpid");
    if (!__bf_tramp_table[104])
        __bf_tramp_table[104] = dlsym(self, "getpid");
    __bf_tramp_table[105] = dlsym(RTLD_NEXT, "gettimeofday");
    if (!__bf_tramp_table[105])
        __bf_tramp_table[105] = dlsym(self, "gettimeofday");
    __bf_tramp_table[106] = dlsym(RTLD_NEXT, "localtime_r");
    if (!__bf_tramp_table[106])
        __bf_tramp_table[106] = dlsym(self, "localtime_r");
    __bf_tramp_table[107] = dlsym(RTLD_NEXT, "fputs");
    if (!__bf_tramp_table[107])
        __bf_tramp_table[107] = dlsym(self, "fputs");
    __bf_tramp_table[108] = dlsym(RTLD_NEXT, "strerror_r");
    if (!__bf_tramp_table[108])
        __bf_tramp_table[108] = dlsym(self, "strerror_r");
    __bf_tramp_table[109] = dlsym(RTLD_NEXT, "snprintf");
    if (!__bf_tramp_table[109])
        __bf_tramp_table[109] = dlsym(self, "snprintf");
    __bf_tramp_table[110] = dlsym(RTLD_NEXT, "prctl");
    if (!__bf_tramp_table[110])
        __bf_tramp_table[110] = dlsym(self, "prctl");
    __bf_tramp_table[111] = dlsym(RTLD_NEXT, "sigaltstack");
    if (!__bf_tramp_table[111])
        __bf_tramp_table[111] = dlsym(self, "sigaltstack");
    __bf_tramp_table[112] = dlsym(RTLD_NEXT, "getpagesize");
    if (!__bf_tramp_table[112])
        __bf_tramp_table[112] = dlsym(self, "getpagesize");
    __bf_tramp_table[113] = dlsym(RTLD_NEXT, "pthread_getspecific");
    if (!__bf_tramp_table[113])
        __bf_tramp_table[113] = dlsym(self, "pthread_getspecific");
    __bf_tramp_table[114] = dlsym(RTLD_NEXT, "fork");
    if (!__bf_tramp_table[114])
        __bf_tramp_table[114] = dlsym(self, "fork");
    __bf_tramp_table[115] = dlsym(RTLD_NEXT, "waitpid");
    if (!__bf_tramp_table[115])
        __bf_tramp_table[115] = dlsym(self, "waitpid");
    __bf_tramp_table[116] = dlsym(RTLD_NEXT, "execv");
    if (!__bf_tramp_table[116])
        __bf_tramp_table[116] = dlsym(self, "execv");
    __bf_tramp_table[117] = dlsym(RTLD_NEXT, "_exit");
    if (!__bf_tramp_table[117])
        __bf_tramp_table[117] = dlsym(self, "_exit");
    __bf_tramp_table[118] = dlsym(RTLD_NEXT, "execve");
    if (!__bf_tramp_table[118])
        __bf_tramp_table[118] = dlsym(self, "execve");
    __bf_tramp_table[119] = dlsym(RTLD_NEXT, "getopt_long");
    if (!__bf_tramp_table[119])
        __bf_tramp_table[119] = dlsym(self, "getopt_long");
    __bf_tramp_table[120] = dlsym(RTLD_NEXT, "getppid");
    if (!__bf_tramp_table[120])
        __bf_tramp_table[120] = dlsym(self, "getppid");
    __bf_tramp_table[121] = dlsym(RTLD_NEXT, "geteuid");
    if (!__bf_tramp_table[121])
        __bf_tramp_table[121] = dlsym(self, "geteuid");
    __bf_tramp_table[122] = dlsym(RTLD_NEXT, "epoll_create1");
    if (!__bf_tramp_table[122])
        __bf_tramp_table[122] = dlsym(self, "epoll_create1");
    __bf_tramp_table[123] = dlsym(RTLD_NEXT, "eventfd");
    if (!__bf_tramp_table[123])
        __bf_tramp_table[123] = dlsym(self, "eventfd");
    __bf_tramp_table[124] = dlsym(RTLD_NEXT, "epoll_ctl");
    if (!__bf_tramp_table[124])
        __bf_tramp_table[124] = dlsym(self, "epoll_ctl");
    __bf_tramp_table[125] = dlsym(RTLD_NEXT, "getsockopt");
    if (!__bf_tramp_table[125])
        __bf_tramp_table[125] = dlsym(self, "getsockopt");
    __bf_tramp_table[126] = dlsym(RTLD_NEXT, "setsockopt");
    if (!__bf_tramp_table[126])
        __bf_tramp_table[126] = dlsym(self, "setsockopt");
    __bf_tramp_table[127] = dlsym(RTLD_NEXT, "epoll_wait");
    if (!__bf_tramp_table[127])
        __bf_tramp_table[127] = dlsym(self, "epoll_wait");
    __bf_tramp_table[128] = dlsym(RTLD_NEXT, "getuid");
    if (!__bf_tramp_table[128])
        __bf_tramp_table[128] = dlsym(self, "getuid");
    __bf_tramp_table[129] = dlsym(RTLD_NEXT, "pthread_mutex_trylock");
    if (!__bf_tramp_table[129])
        __bf_tramp_table[129] = dlsym(self, "pthread_mutex_trylock");
    __bf_tramp_table[130] = dlsym(RTLD_NEXT, "ldexp");
    if (!__bf_tramp_table[130])
        __bf_tramp_table[130] = dlsym(self, "ldexp");
    __bf_tramp_table[131] = dlsym(RTLD_NEXT, "strchr");
    if (!__bf_tramp_table[131])
        __bf_tramp_table[131] = dlsym(self, "strchr");
    __bf_tramp_table[132] = dlsym(RTLD_NEXT, "getaddrinfo");
    if (!__bf_tramp_table[132])
        __bf_tramp_table[132] = dlsym(self, "getaddrinfo");
    __bf_tramp_table[133] = dlsym(RTLD_NEXT, "socket");
    if (!__bf_tramp_table[133])
        __bf_tramp_table[133] = dlsym(self, "socket");
    __bf_tramp_table[134] = dlsym(RTLD_NEXT, "freeaddrinfo");
    if (!__bf_tramp_table[134])
        __bf_tramp_table[134] = dlsym(self, "freeaddrinfo");
    __bf_tramp_table[135] = dlsym(RTLD_NEXT, "connect");
    if (!__bf_tramp_table[135])
        __bf_tramp_table[135] = dlsym(self, "connect");
    __bf_tramp_table[136] = dlsym(RTLD_NEXT, "poll");
    if (!__bf_tramp_table[136])
        __bf_tramp_table[136] = dlsym(self, "poll");
    __bf_tramp_table[137] = dlsym(RTLD_NEXT, "sched_getscheduler");
    if (!__bf_tramp_table[137])
        __bf_tramp_table[137] = dlsym(self, "sched_getscheduler");
    __bf_tramp_table[138] = dlsym(RTLD_NEXT, "sched_getparam");
    if (!__bf_tramp_table[138])
        __bf_tramp_table[138] = dlsym(self, "sched_getparam");
    __bf_tramp_table[139] = dlsym(RTLD_NEXT, "getpriority");
    if (!__bf_tramp_table[139])
        __bf_tramp_table[139] = dlsym(self, "getpriority");
    __bf_tramp_table[140] = dlsym(RTLD_NEXT, "uname");
    if (!__bf_tramp_table[140])
        __bf_tramp_table[140] = dlsym(self, "uname");
    __bf_tramp_table[141] = dlsym(RTLD_NEXT, "tzset");
    if (!__bf_tramp_table[141])
        __bf_tramp_table[141] = dlsym(self, "tzset");
    __bf_tramp_table[142] = dlsym(RTLD_NEXT, "strnlen");
    if (!__bf_tramp_table[142])
        __bf_tramp_table[142] = dlsym(self, "strnlen");
    __bf_tramp_table[143] = dlsym(RTLD_NEXT, "writev");
    if (!__bf_tramp_table[143])
        __bf_tramp_table[143] = dlsym(self, "writev");
    __bf_tramp_table[144] = dlsym(RTLD_NEXT, "sscanf");
    if (!__bf_tramp_table[144])
        __bf_tramp_table[144] = dlsym(self, "sscanf");
    __bf_tramp_table[145] = dlsym(RTLD_NEXT, "strtoul");
    if (!__bf_tramp_table[145])
        __bf_tramp_table[145] = dlsym(self, "strtoul");
    __bf_tramp_table[146] = dlsym(RTLD_NEXT, "strtoull");
    if (!__bf_tramp_table[146])
        __bf_tramp_table[146] = dlsym(self, "strtoull");
    __bf_tramp_table[147] = dlsym(RTLD_NEXT, "lseek");
    if (!__bf_tramp_table[147])
        __bf_tramp_table[147] = dlsym(self, "lseek");
    __bf_tramp_table[148] = dlsym(RTLD_NEXT, "ftruncate");
    if (!__bf_tramp_table[148])
        __bf_tramp_table[148] = dlsym(self, "ftruncate");
    __bf_tramp_table[149] = dlsym(RTLD_NEXT, "fstat");
    if (!__bf_tramp_table[149])
        __bf_tramp_table[149] = dlsym(self, "fstat");
    __bf_tramp_table[150] = dlsym(RTLD_NEXT, "lstat");
    if (!__bf_tramp_table[150])
        __bf_tramp_table[150] = dlsym(self, "lstat");
    __bf_tramp_table[151] = dlsym(RTLD_NEXT, "rename");
    if (!__bf_tramp_table[151])
        __bf_tramp_table[151] = dlsym(self, "rename");
    __bf_tramp_table[152] = dlsym(RTLD_NEXT, "unlink");
    if (!__bf_tramp_table[152])
        __bf_tramp_table[152] = dlsym(self, "unlink");
    __bf_tramp_table[153] = dlsym(RTLD_NEXT, "rmdir");
    if (!__bf_tramp_table[153])
        __bf_tramp_table[153] = dlsym(self, "rmdir");
    __bf_tramp_table[154] = dlsym(RTLD_NEXT, "nanosleep");
    if (!__bf_tramp_table[154])
        __bf_tramp_table[154] = dlsym(self, "nanosleep");
    __bf_tramp_table[155] = dlsym(RTLD_NEXT, "sigemptyset");
    if (!__bf_tramp_table[155])
        __bf_tramp_table[155] = dlsym(self, "sigemptyset");
    __bf_tramp_table[156] = dlsym(RTLD_NEXT, "sigaction");
    if (!__bf_tramp_table[156])
        __bf_tramp_table[156] = dlsym(self, "sigaction");
    __bf_tramp_table[157] = dlsym(RTLD_NEXT, "raise");
    if (!__bf_tramp_table[157])
        __bf_tramp_table[157] = dlsym(self, "raise");
    __bf_tramp_table[158] = dlsym(RTLD_NEXT, "fscanf");
    if (!__bf_tramp_table[158])
        __bf_tramp_table[158] = dlsym(self, "fscanf");
    __bf_tramp_table[159] = dlsym(RTLD_NEXT, "pread64");
    if (!__bf_tramp_table[159])
        __bf_tramp_table[159] = dlsym(self, "pread64");
    __bf_tramp_table[160] = dlsym(RTLD_NEXT, "ptrace");
    if (!__bf_tramp_table[160])
        __bf_tramp_table[160] = dlsym(self, "ptrace");
    __bf_tramp_table[161] = dlsym(RTLD_NEXT, "socketpair");
    if (!__bf_tramp_table[161])
        __bf_tramp_table[161] = dlsym(self, "socketpair");
    __bf_tramp_table[162] = dlsym(RTLD_NEXT, "sendmsg");
    if (!__bf_tramp_table[162])
        __bf_tramp_table[162] = dlsym(self, "sendmsg");
    __bf_tramp_table[163] = dlsym(RTLD_NEXT, "recvmsg");
    if (!__bf_tramp_table[163])
        __bf_tramp_table[163] = dlsym(self, "recvmsg");
    __bf_tramp_table[164] = dlsym(RTLD_NEXT, "__cmsg_nxthdr");
    if (!__bf_tramp_table[164])
        __bf_tramp_table[164] = dlsym(self, "__cmsg_nxthdr");
    __bf_tramp_table[165] = dlsym(RTLD_NEXT, "readlink");
    if (!__bf_tramp_table[165])
        __bf_tramp_table[165] = dlsym(self, "readlink");
    __bf_tramp_table[166] = dlsym(RTLD_NEXT, "printf");
    if (!__bf_tramp_table[166])
        __bf_tramp_table[166] = dlsym(self, "printf");
    __bf_tramp_table[167] = dlsym(RTLD_NEXT, "wmemchr");
    if (!__bf_tramp_table[167])
        __bf_tramp_table[167] = dlsym(self, "wmemchr");
    __bf_tramp_table[168] = dlsym(RTLD_NEXT, "localeconv");
    if (!__bf_tramp_table[168])
        __bf_tramp_table[168] = dlsym(self, "localeconv");
    __bf_tramp_table[169] = dlsym(RTLD_NEXT, "__vsnprintf_chk");
    if (!__bf_tramp_table[169])
        __bf_tramp_table[169] = dlsym(self, "__vsnprintf_chk");
    __bf_tramp_table[170] = dlsym(RTLD_NEXT, "__memmove_chk");
    if (!__bf_tramp_table[170])
        __bf_tramp_table[170] = dlsym(self, "__memmove_chk");
    __bf_tramp_table[171] = dlsym(RTLD_NEXT, "sched_yield");
    if (!__bf_tramp_table[171])
        __bf_tramp_table[171] = dlsym(self, "sched_yield");
    __bf_tramp_table[172] = dlsym(RTLD_NEXT, "modf");
    if (!__bf_tramp_table[172])
        __bf_tramp_table[172] = dlsym(self, "modf");
    __bf_tramp_table[173] = dlsym(RTLD_NEXT, "strcasecmp");
    if (!__bf_tramp_table[173])
        __bf_tramp_table[173] = dlsym(self, "strcasecmp");
    __bf_tramp_table[174] = dlsym(RTLD_NEXT, "getnameinfo");
    if (!__bf_tramp_table[174])
        __bf_tramp_table[174] = dlsym(self, "getnameinfo");
    __bf_tramp_table[175] = dlsym(RTLD_NEXT, "strftime");
    if (!__bf_tramp_table[175])
        __bf_tramp_table[175] = dlsym(self, "strftime");
    __bf_tramp_table[176] = dlsym(RTLD_NEXT, "__strcpy_chk");
    if (!__bf_tramp_table[176])
        __bf_tramp_table[176] = dlsym(self, "__strcpy_chk");
    __bf_tramp_table[177] = dlsym(RTLD_NEXT, "frexpf");
    if (!__bf_tramp_table[177])
        __bf_tramp_table[177] = dlsym(self, "frexpf");
    __bf_tramp_table[178] = dlsym(RTLD_NEXT, "ldexpf");
    if (!__bf_tramp_table[178])
        __bf_tramp_table[178] = dlsym(self, "ldexpf");
    __bf_tramp_table[179] = dlsym(RTLD_NEXT, "tanf");
    if (!__bf_tramp_table[179])
        __bf_tramp_table[179] = dlsym(self, "tanf");
    __bf_tramp_table[180] = dlsym(RTLD_NEXT, "atanf");
    if (!__bf_tramp_table[180])
        __bf_tramp_table[180] = dlsym(self, "atanf");
    __bf_tramp_table[181] = dlsym(RTLD_NEXT, "erff");
    if (!__bf_tramp_table[181])
        __bf_tramp_table[181] = dlsym(self, "erff");
    __bf_tramp_table[182] = dlsym(RTLD_NEXT, "acosf");
    if (!__bf_tramp_table[182])
        __bf_tramp_table[182] = dlsym(self, "acosf");
    __bf_tramp_table[183] = dlsym(RTLD_NEXT, "strstr");
    if (!__bf_tramp_table[183])
        __bf_tramp_table[183] = dlsym(self, "strstr");
    __bf_tramp_table[184] = dlsym(RTLD_NEXT, "erfcf");
    if (!__bf_tramp_table[184])
        __bf_tramp_table[184] = dlsym(self, "erfcf");
    __bf_tramp_table[185] = dlsym(RTLD_NEXT, "modff");
    if (!__bf_tramp_table[185])
        __bf_tramp_table[185] = dlsym(self, "modff");
    __bf_tramp_table[186] = dlsym(RTLD_NEXT, "coshf");
    if (!__bf_tramp_table[186])
        __bf_tramp_table[186] = dlsym(self, "coshf");
    __bf_tramp_table[187] = dlsym(RTLD_NEXT, "sinhf");
    if (!__bf_tramp_table[187])
        __bf_tramp_table[187] = dlsym(self, "sinhf");
    __bf_tramp_table[188] = dlsym(RTLD_NEXT, "tanhf");
    if (!__bf_tramp_table[188])
        __bf_tramp_table[188] = dlsym(self, "tanhf");
    __bf_tramp_table[189] = dlsym(RTLD_NEXT, "atan2");
    if (!__bf_tramp_table[189])
        __bf_tramp_table[189] = dlsym(self, "atan2");
    __bf_tramp_table[190] = dlsym(RTLD_NEXT, "cbrtf");
    if (!__bf_tramp_table[190])
        __bf_tramp_table[190] = dlsym(self, "cbrtf");
    __bf_tramp_table[191] = dlsym(RTLD_NEXT, "__strchr_chk");
    if (!__bf_tramp_table[191])
        __bf_tramp_table[191] = dlsym(self, "__strchr_chk");
    __bf_tramp_table[192] = dlsym(RTLD_NEXT, "clock");
    if (!__bf_tramp_table[192])
        __bf_tramp_table[192] = dlsym(self, "clock");
    __bf_tramp_table[193] = dlsym(RTLD_NEXT, "fileno");
    if (!__bf_tramp_table[193])
        __bf_tramp_table[193] = dlsym(self, "fileno");
    __bf_tramp_table[194] = dlsym(RTLD_NEXT, "remainderf");
    if (!__bf_tramp_table[194])
        __bf_tramp_table[194] = dlsym(self, "remainderf");
    __bf_tramp_table[195] = dlsym(RTLD_NEXT, "nan");
    if (!__bf_tramp_table[195])
        __bf_tramp_table[195] = dlsym(self, "nan");
    __bf_tramp_table[196] = dlsym(RTLD_NEXT, "qsort");
    if (!__bf_tramp_table[196])
        __bf_tramp_table[196] = dlsym(self, "qsort");
    __bf_tramp_table[197] = dlsym(RTLD_NEXT, "nextafterf");
    if (!__bf_tramp_table[197])
        __bf_tramp_table[197] = dlsym(self, "nextafterf");
    __bf_tramp_table[198] = dlsym(RTLD_NEXT, "acos");
    if (!__bf_tramp_table[198])
        __bf_tramp_table[198] = dlsym(self, "acos");
    __bf_tramp_table[199] = dlsym(RTLD_NEXT, "asin");
    if (!__bf_tramp_table[199])
        __bf_tramp_table[199] = dlsym(self, "asin");
    __bf_tramp_table[200] = dlsym(RTLD_NEXT, "ilogb");
    if (!__bf_tramp_table[200])
        __bf_tramp_table[200] = dlsym(self, "ilogb");
    __bf_tramp_table[201] = dlsym(RTLD_NEXT, "__FD_SET_chk");
    if (!__bf_tramp_table[201])
        __bf_tramp_table[201] = dlsym(self, "__FD_SET_chk");
    __bf_tramp_table[202] = dlsym(RTLD_NEXT, "select");
    if (!__bf_tramp_table[202])
        __bf_tramp_table[202] = dlsym(self, "select");
    __bf_tramp_table[203] = dlsym(RTLD_NEXT, "__FD_ISSET_chk");
    if (!__bf_tramp_table[203])
        __bf_tramp_table[203] = dlsym(self, "__FD_ISSET_chk");
    __bf_tramp_table[204] = dlsym(RTLD_NEXT, "sendto");
    if (!__bf_tramp_table[204])
        __bf_tramp_table[204] = dlsym(self, "sendto");
    __bf_tramp_table[205] = dlsym(RTLD_NEXT, "recvfrom");
    if (!__bf_tramp_table[205])
        __bf_tramp_table[205] = dlsym(self, "recvfrom");
    __bf_tramp_table[206] = dlsym(RTLD_NEXT, "__strcat_chk");
    if (!__bf_tramp_table[206])
        __bf_tramp_table[206] = dlsym(self, "__strcat_chk");
    __bf_tramp_table[207] = dlsym(RTLD_NEXT, "setpriority");
    if (!__bf_tramp_table[207])
        __bf_tramp_table[207] = dlsym(self, "setpriority");
    __bf_tramp_table[208] = dlsym(RTLD_NEXT, "pthread_mutexattr_init");
    if (!__bf_tramp_table[208])
        __bf_tramp_table[208] = dlsym(self, "pthread_mutexattr_init");
    __bf_tramp_table[209] = dlsym(RTLD_NEXT, "pthread_mutexattr_settype");
    if (!__bf_tramp_table[209])
        __bf_tramp_table[209] = dlsym(self, "pthread_mutexattr_settype");
    __bf_tramp_table[210] = dlsym(RTLD_NEXT, "pthread_mutexattr_destroy");
    if (!__bf_tramp_table[210])
        __bf_tramp_table[210] = dlsym(self, "pthread_mutexattr_destroy");
    __bf_tramp_table[211] = dlsym(RTLD_NEXT, "sem_init");
    if (!__bf_tramp_table[211])
        __bf_tramp_table[211] = dlsym(self, "sem_init");
    __bf_tramp_table[212] = dlsym(RTLD_NEXT, "sem_destroy");
    if (!__bf_tramp_table[212])
        __bf_tramp_table[212] = dlsym(self, "sem_destroy");
    __bf_tramp_table[213] = dlsym(RTLD_NEXT, "sem_wait");
    if (!__bf_tramp_table[213])
        __bf_tramp_table[213] = dlsym(self, "sem_wait");
    __bf_tramp_table[214] = dlsym(RTLD_NEXT, "sem_post");
    if (!__bf_tramp_table[214])
        __bf_tramp_table[214] = dlsym(self, "sem_post");
    __bf_tramp_table[215] = dlsym(RTLD_NEXT, "inet_ntop");
    if (!__bf_tramp_table[215])
        __bf_tramp_table[215] = dlsym(self, "inet_ntop");
    __bf_tramp_table[216] = dlsym(RTLD_NEXT, "inet_pton");
    if (!__bf_tramp_table[216])
        __bf_tramp_table[216] = dlsym(self, "inet_pton");
    __bf_tramp_table[217] = dlsym(RTLD_NEXT, "strncasecmp");
    if (!__bf_tramp_table[217])
        __bf_tramp_table[217] = dlsym(self, "strncasecmp");
    __bf_tramp_table[218] = dlsym(RTLD_NEXT, "pthread_attr_setschedparam");
    if (!__bf_tramp_table[218])
        __bf_tramp_table[218] = dlsym(self, "pthread_attr_setschedparam");
    __bf_tramp_table[219] = dlsym(RTLD_NEXT, "bind");
    if (!__bf_tramp_table[219])
        __bf_tramp_table[219] = dlsym(self, "bind");
    __bf_tramp_table[220] = dlsym(RTLD_NEXT, "getsockname");
    if (!__bf_tramp_table[220])
        __bf_tramp_table[220] = dlsym(self, "getsockname");
    __bf_tramp_table[221] = dlsym(RTLD_NEXT, "gethostname");
    if (!__bf_tramp_table[221])
        __bf_tramp_table[221] = dlsym(self, "gethostname");
    __bf_tramp_table[222] = dlsym(RTLD_NEXT, "__sendto_chk");
    if (!__bf_tramp_table[222])
        __bf_tramp_table[222] = dlsym(self, "__sendto_chk");
    __bf_tramp_table[223] = dlsym(RTLD_NEXT, "puts");
    if (!__bf_tramp_table[223])
        __bf_tramp_table[223] = dlsym(self, "puts");
    __bf_tramp_table[224] = dlsym(RTLD_NEXT, "gai_strerror");
    if (!__bf_tramp_table[224])
        __bf_tramp_table[224] = dlsym(self, "gai_strerror");
    __bf_tramp_table[225] = dlsym(RTLD_NEXT, "__write_chk");
    if (!__bf_tramp_table[225])
        __bf_tramp_table[225] = dlsym(self, "__write_chk");
    __bf_tramp_table[226] = dlsym(RTLD_NEXT, "__poll_chk");
    if (!__bf_tramp_table[226])
        __bf_tramp_table[226] = dlsym(self, "__poll_chk");
    __bf_tramp_table[227] = dlsym(RTLD_NEXT, "vprintf");
    if (!__bf_tramp_table[227])
        __bf_tramp_table[227] = dlsym(self, "vprintf");
    __bf_tramp_table[228] = dlsym(RTLD_NEXT, "usleep");
    if (!__bf_tramp_table[228])
        __bf_tramp_table[228] = dlsym(self, "usleep");
    __bf_tramp_table[229] = dlsym(RTLD_NEXT, "pthread_kill");
    if (!__bf_tramp_table[229])
        __bf_tramp_table[229] = dlsym(self, "pthread_kill");
    __bf_tramp_table[230] = dlsym(RTLD_NEXT, "pthread_detach");
    if (!__bf_tramp_table[230])
        __bf_tramp_table[230] = dlsym(self, "pthread_detach");
    __bf_tramp_table[231] = dlsym(RTLD_NEXT, "exit");
    if (!__bf_tramp_table[231])
        __bf_tramp_table[231] = dlsym(self, "exit");
    __bf_tramp_table[232] = dlsym(RTLD_NEXT, "ferror");
    if (!__bf_tramp_table[232])
        __bf_tramp_table[232] = dlsym(self, "ferror");
    __bf_tramp_table[233] = dlsym(RTLD_NEXT, "clearerr");
    if (!__bf_tramp_table[233])
        __bf_tramp_table[233] = dlsym(self, "clearerr");
    __bf_tramp_table[234] = dlsym(RTLD_NEXT, "wcslen");
    if (!__bf_tramp_table[234])
        __bf_tramp_table[234] = dlsym(self, "wcslen");
    __bf_tramp_table[235] = dlsym(RTLD_NEXT, "wmemcmp");
    if (!__bf_tramp_table[235])
        __bf_tramp_table[235] = dlsym(self, "wmemcmp");
    __bf_tramp_table[236] = dlsym(RTLD_NEXT, "exp");
    if (!__bf_tramp_table[236])
        __bf_tramp_table[236] = dlsym(self, "exp");
    __bf_tramp_table[237] = dlsym(RTLD_NEXT, "pow");
    if (!__bf_tramp_table[237])
        __bf_tramp_table[237] = dlsym(self, "pow");
    __bf_tramp_table[238] = dlsym(RTLD_NEXT, "fmod");
    if (!__bf_tramp_table[238])
        __bf_tramp_table[238] = dlsym(self, "fmod");
    __bf_tramp_table[239] = dlsym(RTLD_NEXT, "log");
    if (!__bf_tramp_table[239])
        __bf_tramp_table[239] = dlsym(self, "log");
    __bf_tramp_table[240] = dlsym(RTLD_NEXT, "log2");
    if (!__bf_tramp_table[240])
        __bf_tramp_table[240] = dlsym(self, "log2");
    __bf_tramp_table[241] = dlsym(RTLD_NEXT, "log10");
    if (!__bf_tramp_table[241])
        __bf_tramp_table[241] = dlsym(self, "log10");
    __bf_tramp_table[242] = dlsym(RTLD_NEXT, "round");
    if (!__bf_tramp_table[242])
        __bf_tramp_table[242] = dlsym(self, "round");
    __bf_tramp_table[243] = dlsym(RTLD_NEXT, "frexp");
    if (!__bf_tramp_table[243])
        __bf_tramp_table[243] = dlsym(self, "frexp");
    __bf_tramp_table[244] = dlsym(RTLD_NEXT, "sin");
    if (!__bf_tramp_table[244])
        __bf_tramp_table[244] = dlsym(self, "sin");
    __bf_tramp_table[245] = dlsym(RTLD_NEXT, "sinh");
    if (!__bf_tramp_table[245])
        __bf_tramp_table[245] = dlsym(self, "sinh");
    __bf_tramp_table[246] = dlsym(RTLD_NEXT, "cos");
    if (!__bf_tramp_table[246])
        __bf_tramp_table[246] = dlsym(self, "cos");
    __bf_tramp_table[247] = dlsym(RTLD_NEXT, "cosh");
    if (!__bf_tramp_table[247])
        __bf_tramp_table[247] = dlsym(self, "cosh");
    __bf_tramp_table[248] = dlsym(RTLD_NEXT, "tan");
    if (!__bf_tramp_table[248])
        __bf_tramp_table[248] = dlsym(self, "tan");
    __bf_tramp_table[249] = dlsym(RTLD_NEXT, "tanh");
    if (!__bf_tramp_table[249])
        __bf_tramp_table[249] = dlsym(self, "tanh");
    __bf_tramp_table[250] = dlsym(RTLD_NEXT, "atol");
    if (!__bf_tramp_table[250])
        __bf_tramp_table[250] = dlsym(self, "atol");
    __bf_tramp_table[251] = dlsym(RTLD_NEXT, "atof");
    if (!__bf_tramp_table[251])
        __bf_tramp_table[251] = dlsym(self, "atof");
    __bf_tramp_table[252] = dlsym(RTLD_NEXT, "strspn");
    if (!__bf_tramp_table[252])
        __bf_tramp_table[252] = dlsym(self, "strspn");
    __bf_tramp_table[253] = dlsym(RTLD_NEXT, "strtof");
    if (!__bf_tramp_table[253])
        __bf_tramp_table[253] = dlsym(self, "strtof");
    __bf_tramp_table[254] = dlsym(RTLD_NEXT, "ioctl");
    if (!__bf_tramp_table[254])
        __bf_tramp_table[254] = dlsym(self, "ioctl");
    __bf_tramp_table[255] = dlsym(RTLD_NEXT, "getpeername");
    if (!__bf_tramp_table[255])
        __bf_tramp_table[255] = dlsym(self, "getpeername");
    __bf_tramp_table[256] = dlsym(RTLD_NEXT, "listen");
    if (!__bf_tramp_table[256])
        __bf_tramp_table[256] = dlsym(self, "listen");
    __bf_tramp_table[257] = dlsym(RTLD_NEXT, "accept");
    if (!__bf_tramp_table[257])
        __bf_tramp_table[257] = dlsym(self, "accept");
    __bf_tramp_table[258] = dlsym(RTLD_NEXT, "epoll_create");
    if (!__bf_tramp_table[258])
        __bf_tramp_table[258] = dlsym(self, "epoll_create");
    __bf_tramp_table[259] = dlsym(RTLD_NEXT, "__FD_CLR_chk");
    if (!__bf_tramp_table[259])
        __bf_tramp_table[259] = dlsym(self, "__FD_CLR_chk");
    __bf_tramp_table[260] = dlsym(RTLD_NEXT, "expm1");
    if (!__bf_tramp_table[260])
        __bf_tramp_table[260] = dlsym(self, "expm1");
    __bf_tramp_table[261] = dlsym(RTLD_NEXT, "if_indextoname");
    if (!__bf_tramp_table[261])
        __bf_tramp_table[261] = dlsym(self, "if_indextoname");
    __bf_tramp_table[262] = dlsym(RTLD_NEXT, "sigaddset");
    if (!__bf_tramp_table[262])
        __bf_tramp_table[262] = dlsym(self, "sigaddset");
    __bf_tramp_table[263] = dlsym(RTLD_NEXT, "pthread_sigmask");
    if (!__bf_tramp_table[263])
        __bf_tramp_table[263] = dlsym(self, "pthread_sigmask");
    __bf_tramp_table[264] = dlsym(RTLD_NEXT, "fgets");
    if (!__bf_tramp_table[264])
        __bf_tramp_table[264] = dlsym(self, "fgets");
    __bf_tramp_table[265] = dlsym(RTLD_NEXT, "setjmp");
    if (!__bf_tramp_table[265])
        __bf_tramp_table[265] = dlsym(self, "setjmp");
    __bf_tramp_table[266] = dlsym(RTLD_NEXT, "longjmp");
    if (!__bf_tramp_table[266])
        __bf_tramp_table[266] = dlsym(self, "longjmp");
    __bf_tramp_table[267] = dlsym(RTLD_NEXT, "pthread_condattr_init");
    if (!__bf_tramp_table[267])
        __bf_tramp_table[267] = dlsym(self, "pthread_condattr_init");
    __bf_tramp_table[268] = dlsym(RTLD_NEXT, "pthread_condattr_setclock");
    if (!__bf_tramp_table[268])
        __bf_tramp_table[268] = dlsym(self, "pthread_condattr_setclock");
    __bf_tramp_table[269] = dlsym(RTLD_NEXT, "pthread_condattr_destroy");
    if (!__bf_tramp_table[269])
        __bf_tramp_table[269] = dlsym(self, "pthread_condattr_destroy");
    __bf_tramp_table[270] = dlsym(RTLD_NEXT, "sched_get_priority_min");
    if (!__bf_tramp_table[270])
        __bf_tramp_table[270] = dlsym(self, "sched_get_priority_min");
    __bf_tramp_table[271] = dlsym(RTLD_NEXT, "pthread_setschedparam");
    if (!__bf_tramp_table[271])
        __bf_tramp_table[271] = dlsym(self, "pthread_setschedparam");
    __bf_tramp_table[272] = dlsym(RTLD_NEXT, "getgid");
    if (!__bf_tramp_table[272])
        __bf_tramp_table[272] = dlsym(self, "getgid");
    __bf_tramp_table[273] = dlsym(RTLD_NEXT, "getegid");
    if (!__bf_tramp_table[273])
        __bf_tramp_table[273] = dlsym(self, "getegid");
    __bf_tramp_table[274] = dlsym(RTLD_NEXT, "random");
    if (!__bf_tramp_table[274])
        __bf_tramp_table[274] = dlsym(self, "random");
    __bf_tramp_table[275] = dlsym(RTLD_NEXT, "sigfillset");
    if (!__bf_tramp_table[275])
        __bf_tramp_table[275] = dlsym(self, "sigfillset");
    __bf_tramp_table[276] = dlsym(RTLD_NEXT, "fdopen");
    if (!__bf_tramp_table[276])
        __bf_tramp_table[276] = dlsym(self, "fdopen");
    __bf_tramp_table[277] = dlsym(RTLD_NEXT, "timerfd_create");
    if (!__bf_tramp_table[277])
        __bf_tramp_table[277] = dlsym(self, "timerfd_create");
    __bf_tramp_table[278] = dlsym(RTLD_NEXT, "timerfd_settime");
    if (!__bf_tramp_table[278])
        __bf_tramp_table[278] = dlsym(self, "timerfd_settime");
    __bf_tramp_table[279] = dlsym(RTLD_NEXT, "fputc");
    if (!__bf_tramp_table[279])
        __bf_tramp_table[279] = dlsym(self, "fputc");
    __bf_tramp_table[280] = dlsym(RTLD_NEXT, "bsearch");
    if (!__bf_tramp_table[280])
        __bf_tramp_table[280] = dlsym(self, "bsearch");
    __bf_tramp_table[281] = dlsym(RTLD_NEXT, "vfprintf");
    if (!__bf_tramp_table[281])
        __bf_tramp_table[281] = dlsym(self, "vfprintf");
    __bf_tramp_table[282] = dlsym(RTLD_NEXT, "pthread_exit");
    if (!__bf_tramp_table[282])
        __bf_tramp_table[282] = dlsym(self, "pthread_exit");
    __bf_tramp_table[283] = dlsym(RTLD_NEXT, "finitef");
    if (!__bf_tramp_table[283])
        __bf_tramp_table[283] = dlsym(self, "finitef");
    __bf_tramp_table[284] = dlsym(RTLD_NEXT, "cbrt");
    if (!__bf_tramp_table[284])
        __bf_tramp_table[284] = dlsym(self, "cbrt");
    __bf_tramp_table[285] = dlsym(RTLD_NEXT, "remquof");
    if (!__bf_tramp_table[285])
        __bf_tramp_table[285] = dlsym(self, "remquof");
    __bf_tramp_table[286] = dlsym(RTLD_NEXT, "strcspn");
    if (!__bf_tramp_table[286])
        __bf_tramp_table[286] = dlsym(self, "strcspn");
    __bf_tramp_table[287] = dlsym(RTLD_NEXT, "gmtime_r");
    if (!__bf_tramp_table[287])
        __bf_tramp_table[287] = dlsym(self, "gmtime_r");
    __bf_tramp_table[288] = dlsym(RTLD_NEXT, "difftime");
    if (!__bf_tramp_table[288])
        __bf_tramp_table[288] = dlsym(self, "difftime");
    __bf_tramp_table[289] = dlsym(RTLD_NEXT, "strpbrk");
    if (!__bf_tramp_table[289])
        __bf_tramp_table[289] = dlsym(self, "strpbrk");
    __bf_tramp_table[290] = dlsym(RTLD_NEXT, "shutdown");
    if (!__bf_tramp_table[290])
        __bf_tramp_table[290] = dlsym(self, "shutdown");
    __bf_tramp_table[291] = dlsym(RTLD_NEXT, "memrchr");
    if (!__bf_tramp_table[291])
        __bf_tramp_table[291] = dlsym(self, "memrchr");
    __bf_tramp_table[292] = dlsym(RTLD_NEXT, "accept4");
    if (!__bf_tramp_table[292])
        __bf_tramp_table[292] = dlsym(self, "accept4");
    __bf_tramp_table[293] = dlsym(RTLD_NEXT, "if_nametoindex");
    if (!__bf_tramp_table[293])
        __bf_tramp_table[293] = dlsym(self, "if_nametoindex");
    __bf_tramp_table[294] = dlsym(RTLD_NEXT, "setvbuf");
    if (!__bf_tramp_table[294])
        __bf_tramp_table[294] = dlsym(self, "setvbuf");
    __bf_tramp_table[295] = dlsym(RTLD_NEXT, "realpath");
    if (!__bf_tramp_table[295])
        __bf_tramp_table[295] = dlsym(self, "realpath");
    __bf_tramp_table[296] = dlsym(RTLD_NEXT, "recvmmsg");
    if (!__bf_tramp_table[296])
        __bf_tramp_table[296] = dlsym(self, "recvmmsg");
    __bf_tramp_table[297] = dlsym(RTLD_NEXT, "getcwd");
    if (!__bf_tramp_table[297])
        __bf_tramp_table[297] = dlsym(self, "getcwd");
    __bf_tramp_table[298] = dlsym(RTLD_NEXT, "pread");
    if (!__bf_tramp_table[298])
        __bf_tramp_table[298] = dlsym(self, "pread");
    __bf_tramp_table[299] = dlsym(RTLD_NEXT, "pwrite");
    if (!__bf_tramp_table[299])
        __bf_tramp_table[299] = dlsym(self, "pwrite");
    __bf_tramp_table[300] = dlsym(RTLD_NEXT, "fchmod");
    if (!__bf_tramp_table[300])
        __bf_tramp_table[300] = dlsym(self, "fchmod");
    __bf_tramp_table[301] = dlsym(RTLD_NEXT, "fchown");
    if (!__bf_tramp_table[301])
        __bf_tramp_table[301] = dlsym(self, "fchown");
    __bf_tramp_table[302] = dlsym(RTLD_NEXT, "mremap");
    if (!__bf_tramp_table[302])
        __bf_tramp_table[302] = dlsym(self, "mremap");
    __bf_tramp_table[303] = dlsym(RTLD_NEXT, "fsync");
    if (!__bf_tramp_table[303])
        __bf_tramp_table[303] = dlsym(self, "fsync");
    __bf_tramp_table[304] = dlsym(RTLD_NEXT, "utimes");
    if (!__bf_tramp_table[304])
        __bf_tramp_table[304] = dlsym(self, "utimes");
    __bf_tramp_table[305] = dlsym(RTLD_NEXT, "msync");
    if (!__bf_tramp_table[305])
        __bf_tramp_table[305] = dlsym(self, "msync");
    __bf_tramp_table[306] = dlsym(RTLD_NEXT, "statvfs");
    if (!__bf_tramp_table[306])
        __bf_tramp_table[306] = dlsym(self, "statvfs");
    __bf_tramp_table[307] = dlsym(RTLD_NEXT, "mallinfo");
    if (!__bf_tramp_table[307])
        __bf_tramp_table[307] = dlsym(self, "mallinfo");
    __bf_tramp_table[308] = dlsym(RTLD_NEXT, "__readlink_chk");
    if (!__bf_tramp_table[308])
        __bf_tramp_table[308] = dlsym(self, "__readlink_chk");
    __bf_tramp_table[309] = dlsym(RTLD_NEXT, "__gnu_strerror_r");
    if (!__bf_tramp_table[309])
        __bf_tramp_table[309] = dlsym(self, "__gnu_strerror_r");
    __bf_tramp_table[310] = dlsym(RTLD_NEXT, "pthread_getschedparam");
    if (!__bf_tramp_table[310])
        __bf_tramp_table[310] = dlsym(self, "pthread_getschedparam");
    __bf_tramp_table[311] = dlsym(RTLD_NEXT, "sinf");
    if (!__bf_tramp_table[311])
        __bf_tramp_table[311] = dlsym(self, "sinf");
    __bf_tramp_table[312] = dlsym(RTLD_NEXT, "sincosf");
    if (!__bf_tramp_table[312])
        __bf_tramp_table[312] = dlsym(self, "sincosf");
    __bf_tramp_table[313] = dlsym(RTLD_NEXT, "exp2");
    if (!__bf_tramp_table[313])
        __bf_tramp_table[313] = dlsym(self, "exp2");
    __bf_tramp_table[314] = dlsym(RTLD_NEXT, "sincos");
    if (!__bf_tramp_table[314])
        __bf_tramp_table[314] = dlsym(self, "sincos");
    __bf_tramp_table[315] = dlsym(RTLD_NEXT, "fmal");
    if (!__bf_tramp_table[315])
        __bf_tramp_table[315] = dlsym(self, "fmal");
    __bf_tramp_table[316] = dlsym(RTLD_NEXT, "exp2f");
    if (!__bf_tramp_table[316])
        __bf_tramp_table[316] = dlsym(self, "exp2f");
    __bf_tramp_table[317] = dlsym(RTLD_NEXT, "log10f");
    if (!__bf_tramp_table[317])
        __bf_tramp_table[317] = dlsym(self, "log10f");
    __bf_tramp_table[318] = dlsym(RTLD_NEXT, "logf");
    if (!__bf_tramp_table[318])
        __bf_tramp_table[318] = dlsym(self, "logf");
    __bf_tramp_table[319] = dlsym(RTLD_NEXT, "powf");
    if (!__bf_tramp_table[319])
        __bf_tramp_table[319] = dlsym(self, "powf");
    __bf_tramp_table[320] = dlsym(RTLD_NEXT, "fmodf");
    if (!__bf_tramp_table[320])
        __bf_tramp_table[320] = dlsym(self, "fmodf");
    __bf_tramp_table[321] = dlsym(RTLD_NEXT, "log2f");
    if (!__bf_tramp_table[321])
        __bf_tramp_table[321] = dlsym(self, "log2f");
    __bf_tramp_table[322] = dlsym(RTLD_NEXT, "expf");
    if (!__bf_tramp_table[322])
        __bf_tramp_table[322] = dlsym(self, "expf");
    __bf_tramp_table[323] = dlsym(RTLD_NEXT, "powl");
    if (!__bf_tramp_table[323])
        __bf_tramp_table[323] = dlsym(self, "powl");
    __bf_tramp_table[324] = dlsym(RTLD_NEXT, "cosf");
    if (!__bf_tramp_table[324])
        __bf_tramp_table[324] = dlsym(self, "cosf");
    __bf_tramp_table[325] = dlsym(RTLD_NEXT, "pthread_key_delete");
    if (!__bf_tramp_table[325])
        __bf_tramp_table[325] = dlsym(self, "pthread_key_delete");
    __bf_tramp_table[326] = dlsym(RTLD_NEXT, "sysinfo");
    if (!__bf_tramp_table[326])
        __bf_tramp_table[326] = dlsym(self, "sysinfo");
    __bf_tramp_table[327] = dlsym(RTLD_NEXT, "madvise");
    if (!__bf_tramp_table[327])
        __bf_tramp_table[327] = dlsym(self, "madvise");
    __bf_tramp_table[328] = dlsym(RTLD_NEXT, "pthread_setname_np");
    if (!__bf_tramp_table[328])
        __bf_tramp_table[328] = dlsym(self, "pthread_setname_np");
    __bf_tramp_table[329] = dlsym(RTLD_NEXT, "pthread_getattr_np");
    if (!__bf_tramp_table[329])
        __bf_tramp_table[329] = dlsym(self, "pthread_getattr_np");
    __bf_tramp_table[330] = dlsym(RTLD_NEXT, "pthread_attr_getstack");
    if (!__bf_tramp_table[330])
        __bf_tramp_table[330] = dlsym(self, "pthread_attr_getstack");
    __bf_tramp_table[331] = dlsym(RTLD_NEXT, "mlock");
    if (!__bf_tramp_table[331])
        __bf_tramp_table[331] = dlsym(self, "mlock");
    __bf_tramp_table[332] = dlsym(RTLD_NEXT, "dl_iterate_phdr");
    if (!__bf_tramp_table[332])
        __bf_tramp_table[332] = dlsym(self, "dl_iterate_phdr");
    __bf_tramp_table[333] = dlsym(RTLD_NEXT, "isspace");
    if (!__bf_tramp_table[333])
        __bf_tramp_table[333] = dlsym(self, "isspace");
    __bf_tramp_table[334] = dlsym(RTLD_NEXT, "gethostbyname");
    if (!__bf_tramp_table[334])
        __bf_tramp_table[334] = dlsym(self, "gethostbyname");
    __bf_tramp_table[335] = dlsym(RTLD_NEXT, "strcat");
    if (!__bf_tramp_table[335])
        __bf_tramp_table[335] = dlsym(self, "strcat");
    __bf_tramp_table[336] = dlsym(RTLD_NEXT, "sendmmsg");
    if (!__bf_tramp_table[336])
        __bf_tramp_table[336] = dlsym(self, "sendmmsg");
    __bf_tramp_table[337] = dlsym(RTLD_NEXT, "tolower");
    if (!__bf_tramp_table[337])
        __bf_tramp_table[337] = dlsym(self, "tolower");
    __bf_tramp_table[338] = dlsym(RTLD_NEXT, "pthread_rwlock_destroy");
    if (!__bf_tramp_table[338])
        __bf_tramp_table[338] = dlsym(self, "pthread_rwlock_destroy");
    __bf_tramp_table[339] = dlsym(RTLD_NEXT, "pthread_rwlock_init");
    if (!__bf_tramp_table[339])
        __bf_tramp_table[339] = dlsym(self, "pthread_rwlock_init");
    __bf_tramp_table[340] = dlsym(RTLD_NEXT, "pthread_rwlock_rdlock");
    if (!__bf_tramp_table[340])
        __bf_tramp_table[340] = dlsym(self, "pthread_rwlock_rdlock");
    __bf_tramp_table[341] = dlsym(RTLD_NEXT, "pthread_rwlock_unlock");
    if (!__bf_tramp_table[341])
        __bf_tramp_table[341] = dlsym(self, "pthread_rwlock_unlock");
    __bf_tramp_table[342] = dlsym(RTLD_NEXT, "pthread_rwlock_wrlock");
    if (!__bf_tramp_table[342])
        __bf_tramp_table[342] = dlsym(self, "pthread_rwlock_wrlock");
    __bf_tramp_table[343] = dlsym(RTLD_NEXT, "signal");
    if (!__bf_tramp_table[343])
        __bf_tramp_table[343] = dlsym(self, "signal");
    __bf_tramp_table[344] = dlsym(RTLD_NEXT, "tcgetattr");
    if (!__bf_tramp_table[344])
        __bf_tramp_table[344] = dlsym(self, "tcgetattr");
    __bf_tramp_table[345] = dlsym(RTLD_NEXT, "tcsetattr");
    if (!__bf_tramp_table[345])
        __bf_tramp_table[345] = dlsym(self, "tcsetattr");
    __bf_tramp_table[346] = dlsym(RTLD_NEXT, "utime");
    if (!__bf_tramp_table[346])
        __bf_tramp_table[346] = dlsym(self, "utime");
    __bf_tramp_table[347] = dlsym(RTLD_NEXT, "vasprintf");
    if (!__bf_tramp_table[347])
        __bf_tramp_table[347] = dlsym(self, "vasprintf");
    __bf_tramp_table[348] = dlsym(RTLD_NEXT, "openlog");
    if (!__bf_tramp_table[348])
        __bf_tramp_table[348] = dlsym(self, "openlog");
    __bf_tramp_table[349] = dlsym(RTLD_NEXT, "syslog");
    if (!__bf_tramp_table[349])
        __bf_tramp_table[349] = dlsym(self, "syslog");
    __bf_tramp_table[350] = dlsym(RTLD_NEXT, "closelog");
    if (!__bf_tramp_table[350])
        __bf_tramp_table[350] = dlsym(self, "closelog");
    __bf_tramp_table[351] = dlsym(RTLD_NEXT, "ungetc");
    if (!__bf_tramp_table[351])
        __bf_tramp_table[351] = dlsym(self, "ungetc");
    __bf_tramp_table[352] = dlsym(RTLD_NEXT, "getc");
    if (!__bf_tramp_table[352])
        __bf_tramp_table[352] = dlsym(self, "getc");
    __bf_tramp_table[353] = dlsym(RTLD_NEXT, "ungetwc");
    if (!__bf_tramp_table[353])
        __bf_tramp_table[353] = dlsym(self, "ungetwc");
    __bf_tramp_table[354] = dlsym(RTLD_NEXT, "getwc");
    if (!__bf_tramp_table[354])
        __bf_tramp_table[354] = dlsym(self, "getwc");
    __bf_tramp_table[355] = dlsym(RTLD_NEXT, "fputwc");
    if (!__bf_tramp_table[355])
        __bf_tramp_table[355] = dlsym(self, "fputwc");
    __bf_tramp_table[356] = dlsym(RTLD_NEXT, "newlocale");
    if (!__bf_tramp_table[356])
        __bf_tramp_table[356] = dlsym(self, "newlocale");
    __bf_tramp_table[357] = dlsym(RTLD_NEXT, "uselocale");
    if (!__bf_tramp_table[357])
        __bf_tramp_table[357] = dlsym(self, "uselocale");
    __bf_tramp_table[358] = dlsym(RTLD_NEXT, "vsscanf");
    if (!__bf_tramp_table[358])
        __bf_tramp_table[358] = dlsym(self, "vsscanf");
    __bf_tramp_table[359] = dlsym(RTLD_NEXT, "strftime_l");
    if (!__bf_tramp_table[359])
        __bf_tramp_table[359] = dlsym(self, "strftime_l");
    __bf_tramp_table[360] = dlsym(RTLD_NEXT, "mbsrtowcs");
    if (!__bf_tramp_table[360])
        __bf_tramp_table[360] = dlsym(self, "mbsrtowcs");
    __bf_tramp_table[361] = dlsym(RTLD_NEXT, "freelocale");
    if (!__bf_tramp_table[361])
        __bf_tramp_table[361] = dlsym(self, "freelocale");
    __bf_tramp_table[362] = dlsym(RTLD_NEXT, "strcoll_l");
    if (!__bf_tramp_table[362])
        __bf_tramp_table[362] = dlsym(self, "strcoll_l");
    __bf_tramp_table[363] = dlsym(RTLD_NEXT, "strxfrm_l");
    if (!__bf_tramp_table[363])
        __bf_tramp_table[363] = dlsym(self, "strxfrm_l");
    __bf_tramp_table[364] = dlsym(RTLD_NEXT, "wcscoll_l");
    if (!__bf_tramp_table[364])
        __bf_tramp_table[364] = dlsym(self, "wcscoll_l");
    __bf_tramp_table[365] = dlsym(RTLD_NEXT, "wcsxfrm_l");
    if (!__bf_tramp_table[365])
        __bf_tramp_table[365] = dlsym(self, "wcsxfrm_l");
    __bf_tramp_table[366] = dlsym(RTLD_NEXT, "iswlower_l");
    if (!__bf_tramp_table[366])
        __bf_tramp_table[366] = dlsym(self, "iswlower_l");
    __bf_tramp_table[367] = dlsym(RTLD_NEXT, "iswspace_l");
    if (!__bf_tramp_table[367])
        __bf_tramp_table[367] = dlsym(self, "iswspace_l");
    __bf_tramp_table[368] = dlsym(RTLD_NEXT, "iswprint_l");
    if (!__bf_tramp_table[368])
        __bf_tramp_table[368] = dlsym(self, "iswprint_l");
    __bf_tramp_table[369] = dlsym(RTLD_NEXT, "iswblank_l");
    if (!__bf_tramp_table[369])
        __bf_tramp_table[369] = dlsym(self, "iswblank_l");
    __bf_tramp_table[370] = dlsym(RTLD_NEXT, "iswcntrl_l");
    if (!__bf_tramp_table[370])
        __bf_tramp_table[370] = dlsym(self, "iswcntrl_l");
    __bf_tramp_table[371] = dlsym(RTLD_NEXT, "iswupper_l");
    if (!__bf_tramp_table[371])
        __bf_tramp_table[371] = dlsym(self, "iswupper_l");
    __bf_tramp_table[372] = dlsym(RTLD_NEXT, "iswalpha_l");
    if (!__bf_tramp_table[372])
        __bf_tramp_table[372] = dlsym(self, "iswalpha_l");
    __bf_tramp_table[373] = dlsym(RTLD_NEXT, "iswdigit_l");
    if (!__bf_tramp_table[373])
        __bf_tramp_table[373] = dlsym(self, "iswdigit_l");
    __bf_tramp_table[374] = dlsym(RTLD_NEXT, "iswpunct_l");
    if (!__bf_tramp_table[374])
        __bf_tramp_table[374] = dlsym(self, "iswpunct_l");
    __bf_tramp_table[375] = dlsym(RTLD_NEXT, "iswxdigit_l");
    if (!__bf_tramp_table[375])
        __bf_tramp_table[375] = dlsym(self, "iswxdigit_l");
    __bf_tramp_table[376] = dlsym(RTLD_NEXT, "towupper_l");
    if (!__bf_tramp_table[376])
        __bf_tramp_table[376] = dlsym(self, "towupper_l");
    __bf_tramp_table[377] = dlsym(RTLD_NEXT, "towlower_l");
    if (!__bf_tramp_table[377])
        __bf_tramp_table[377] = dlsym(self, "towlower_l");
    __bf_tramp_table[378] = dlsym(RTLD_NEXT, "btowc");
    if (!__bf_tramp_table[378])
        __bf_tramp_table[378] = dlsym(self, "btowc");
    __bf_tramp_table[379] = dlsym(RTLD_NEXT, "wctob");
    if (!__bf_tramp_table[379])
        __bf_tramp_table[379] = dlsym(self, "wctob");
    __bf_tramp_table[380] = dlsym(RTLD_NEXT, "wcsnrtombs");
    if (!__bf_tramp_table[380])
        __bf_tramp_table[380] = dlsym(self, "wcsnrtombs");
    __bf_tramp_table[381] = dlsym(RTLD_NEXT, "wcrtomb");
    if (!__bf_tramp_table[381])
        __bf_tramp_table[381] = dlsym(self, "wcrtomb");
    __bf_tramp_table[382] = dlsym(RTLD_NEXT, "mbsnrtowcs");
    if (!__bf_tramp_table[382])
        __bf_tramp_table[382] = dlsym(self, "mbsnrtowcs");
    __bf_tramp_table[383] = dlsym(RTLD_NEXT, "mbrtowc");
    if (!__bf_tramp_table[383])
        __bf_tramp_table[383] = dlsym(self, "mbrtowc");
    __bf_tramp_table[384] = dlsym(RTLD_NEXT, "mbtowc");
    if (!__bf_tramp_table[384])
        __bf_tramp_table[384] = dlsym(self, "mbtowc");
    __bf_tramp_table[385] = dlsym(RTLD_NEXT, "__ctype_get_mb_cur_max");
    if (!__bf_tramp_table[385])
        __bf_tramp_table[385] = dlsym(self, "__ctype_get_mb_cur_max");
    __bf_tramp_table[386] = dlsym(RTLD_NEXT, "mbrlen");
    if (!__bf_tramp_table[386])
        __bf_tramp_table[386] = dlsym(self, "mbrlen");
    __bf_tramp_table[387] = dlsym(RTLD_NEXT, "strtoll_l");
    if (!__bf_tramp_table[387])
        __bf_tramp_table[387] = dlsym(self, "strtoll_l");
    __bf_tramp_table[388] = dlsym(RTLD_NEXT, "strtoull_l");
    if (!__bf_tramp_table[388])
        __bf_tramp_table[388] = dlsym(self, "strtoull_l");
    __bf_tramp_table[389] = dlsym(RTLD_NEXT, "strtold_l");
    if (!__bf_tramp_table[389])
        __bf_tramp_table[389] = dlsym(self, "strtold_l");
    __bf_tramp_table[390] = dlsym(RTLD_NEXT, "__cxa_thread_atexit_impl");
    if (!__bf_tramp_table[390])
        __bf_tramp_table[390] = dlsym(self, "__cxa_thread_atexit_impl");
    __bf_tramp_table[391] = dlsym(RTLD_NEXT, "strncat");
    if (!__bf_tramp_table[391])
        __bf_tramp_table[391] = dlsym(self, "strncat");

    dlclose(self);

    *(void **)(__bf_data_stderr) = dlsym(RTLD_NEXT, "stderr");
    *(void **)(__bf_data___sF) = dlsym(RTLD_NEXT, "_IO_2_1_stderr_");
    *(void **)(__bf_data_optarg) = dlsym(RTLD_NEXT, "optarg");
    *(void **)(__bf_data_optind) = dlsym(RTLD_NEXT, "optind");
    *(void **)(__bf_data_tzname) = dlsym(RTLD_NEXT, "tzname");
    *(void **)(__bf_data_daylight) = dlsym(RTLD_NEXT, "daylight");
    *(void **)(__bf_data_timezone) = dlsym(RTLD_NEXT, "timezone");
    *(void **)(__bf_data_environ) = dlsym(RTLD_NEXT, "environ");
    *(void **)(__bf_data_in6addr_any) = dlsym(RTLD_NEXT, "in6addr_any");
    *(void **)(__bf_data_stdin) = dlsym(RTLD_NEXT, "stdin");
    *(void **)(__bf_data_stdout) = dlsym(RTLD_NEXT, "stdout");
    *(void **)(__bf_data_in6addr_loopback) = dlsym(RTLD_NEXT, "in6addr_loopback");
    *(void **)(__bf_data___stack_chk_guard) = dlsym(RTLD_NEXT, "__stack_chk_guard");
}