// Minimal JNI shim to load libroblox.so and call JNI_OnLoad.
// This bypasses the need for a full Android Java runtime.
// Compile: gcc -shared -fPIC -o libjni_shim.so jni_shim.c -ldl

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

// Minimal JNI types
typedef int jint;
typedef void* jobject;
typedef void* jclass;
typedef void* jstring;
typedef void* jmethodID;
typedef void* jfieldID;
typedef void* jvalue;
typedef unsigned char jboolean;
typedef short jchar;

#define JNI_TRUE 1
#define JNI_FALSE 0
#define JNI_OK 0
#define JNI_ERR (-1)
#define JNI_VERSION_1_6 0x00010006
#define JNI_COMMIT 1
#define JNI_ABORT 2

// JNI function table (minimal - just what Roblox needs)
typedef struct JavaVM_ JavaVM;

typedef struct JNINativeInterface_ {
    void* reserved0[4];
    jint (*GetVersion)(JavaVM* vm);
    jclass (*FindClass)(JavaVM* env, const char* name);
    // ... many more functions
    void* reserved[200]; // placeholder for the rest
} JNINativeInterface;

typedef struct JNIEnv_ {
    const struct JNINativeInterface_* functions;
    void* reserved[4];
} JNIEnv;

typedef struct JavaVMOption_ {
    char* optionString;
    void* extraInfo;
} JavaVMOption;

typedef struct JavaVMInitArgs_ {
    jint version;
    jint nOptions;
    JavaVMOption* options;
    jboolean ignoreUnrecognized;
} JavaVMInitArgs;

typedef struct JavaVM_ {
    const struct JNIInvokeInterface_* functions;
    void* reserved[4];
} JavaVM;

typedef struct JNIInvokeInterface_ {
    void* reserved0[3];
    jint (*DestroyJavaVM)(JavaVM*);
    jint (*AttachCurrentThread)(JavaVM*, JNIEnv**, void*);
    jint (*DetachCurrentThread)(JavaVM*);
    jint (*GetEnv)(JavaVM*, void**, jint);
    jint (*AttachCurrentThreadAsDaemon)(JavaVM*, JNIEnv**, void*);
} JNIInvokeInterface;

// Thread-local JNIEnv
static __thread JNIEnv tls_env;
static JavaVM g_vm;
static JNIInvokeInterface g_vm_functions;

// Stub FindClass - returns NULL (game will need to handle)
static jclass stub_FindClass(JNIEnv* env, const char* name) {
    fprintf(stderr, "[jni_shim] FindClass: %s (returning NULL)\n", name);
    return NULL;
}

// Stub GetVersion
static jint stub_GetVersion(JNIEnv* env) {
    return JNI_VERSION_1_6;
}

// Stub RegisterNatives
static jint stub_RegisterNatives(JNIEnv* env, jclass clazz, const void* methods, jint nMethods) {
    fprintf(stderr, "[jni_shim] RegisterNatives: %d methods\n", nMethods);
    return JNI_OK;
}

// Initialize the JNI function table with stubs
static JNINativeInterface g_env_functions = {0};

__attribute__((visibility("default")))
__attribute__((constructor))
static void init_jni_functions() {
    // Set up the minimal function table
    g_env_functions.reserved0[0] = NULL; // reserved
    g_env_functions.reserved0[1] = NULL;
    g_env_functions.reserved0[2] = NULL;
    g_env_functions.reserved0[3] = NULL;
    g_env_functions.GetVersion = (void*)stub_GetVersion;
    g_env_functions.FindClass = (void*)stub_FindClass;
    // ... rest remain NULL (stubs)

    // Set up VM
    g_vm_functions.DestroyJavaVM = NULL;
    g_vm_functions.AttachCurrentThread = NULL;
    g_vm_functions.DetachCurrentThread = NULL;
    g_vm_functions.GetEnv = NULL;
    g_vm_functions.AttachCurrentThreadAsDaemon = NULL;
    g_vm.functions = &g_vm_functions;
}

// JNI_OnLoad - called when libroblox.so is loaded
// This is the entry point the game expects
jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    fprintf(stderr, "[jni_shim] JNI_OnLoad called\n");
    // Return the JNI version we support
    return JNI_VERSION_1_6;
}

// Total: 392 function symbols

static const char *bf_table_syms[] = {
    "__cxa_finalize",
    "__cxa_atexit",
    "__register_atfork",
    "strlen",
    "memcmp",
    "pthread_mutex_init",
    "pthread_mutex_destroy",
    "pthread_once",
    "__memset_chk",
    "__memcpy_chk",
    "__strlen_chk",
    "memchr",
    "dladdr",
    "dlerror",
    "dlopen",
    "dlsym",
    "strncmp",
    "strcmp",
    "getauxval",
    "__errno",
    "close",
    "__open_2",
    "__read_chk",
    "read",
    "clock_gettime",
    "syscall",
    "sched_getcpu",
    "sysconf",
    "mmap",
    "mprotect",
    "munmap",
    "pthread_attr_init",
    "pthread_attr_setstacksize",
    "pthread_create",
    "pthread_attr_destroy",
    "pthread_join",
    "pthread_self",
    "memset",
    "pthread_setspecific",
    "__strncpy_chk",
    "sched_get_priority_max",
    "sched_setscheduler",
    "pthread_mutex_lock",
    "pthread_cond_wait",
    "pthread_mutex_unlock",
    "pthread_cond_signal",
    "pthread_cond_init",
    "gettid",
    "__vsprintf_chk",
    "atan2f",
    "vsnprintf",
    "fprintf",
    "rand",
    "strtoll",
    "time",
    "pthread_cond_broadcast",
    "pthread_cond_destroy",
    "stat",
    "opendir",
    "readdir",
    "closedir",
    "posix_fallocate",
    "open",
    "pthread_equal",
    "atoll",
    "srand",
    "strtod",
    "localtime",
    "fseek",
    "ftell",
    "fclose",
    "atan",
    "mkdir",
    "__fread_chk",
    "fread",
    "asinf",
    "dlclose",
    "access",
    "getenv",
    "strncpy",
    "feof",
    "fopen",
    "fseeko",
    "ftello",
    "fwrite",
    "fflush",
    "gmtime",
    "mktime",
    "pipe",
    "write",
    "pthread_key_create",
    "abort",
    "__assert",
    "strrchr",
    "__stack_chk_fail",
    "atoi",
    "fcntl",
    "memcpy",
    "strcpy",
    "strerror",
    "pthread_attr_setdetachstate",
    "pthread_cond_timedwait",
    "memmove",
    "strtol",
    "getpid",
    "gettimeofday",
    "localtime_r",
    "fputs",
    "strerror_r",
    "snprintf",
    "prctl",
    "sigaltstack",
    "getpagesize",
    "pthread_getspecific",
    "fork",
    "waitpid",
    "execv",
    "_exit",
    "execve",
    "getopt_long",
    "getppid",
    "geteuid",
    "epoll_create1",
    "eventfd",
    "epoll_ctl",
    "getsockopt",
    "setsockopt",
    "epoll_wait",
    "getuid",
    "pthread_mutex_trylock",
    "ldexp",
    "strchr",
    "getaddrinfo",
    "socket",
    "freeaddrinfo",
    "connect",
    "poll",
    "sched_getscheduler",
    "sched_getparam",
    "getpriority",
    "uname",
    "tzset",
    "strnlen",
    "writev",
    "sscanf",
    "strtoul",
    "strtoull",
    "lseek",
    "ftruncate",
    "fstat",
    "lstat",
    "rename",
    "unlink",
    "rmdir",
    "nanosleep",
    "sigemptyset",
    "sigaction",
    "raise",
    "fscanf",
    "pread64",
    "ptrace",
    "socketpair",
    "sendmsg",
    "recvmsg",
    "__cmsg_nxthdr",
    "readlink",
    "printf",
    "wmemchr",
    "localeconv",
    "__vsnprintf_chk",
    "__memmove_chk",
    "sched_yield",
    "modf",
    "strcasecmp",
    "getnameinfo",
    "strftime",
    "__strcpy_chk",
    "frexpf",
    "ldexpf",
    "tanf",
    "atanf",
    "erff",
    "acosf",
    "strstr",
    "erfcf",
    "modff",
    "coshf",
    "sinhf",
    "tanhf",
    "atan2",
    "cbrtf",
    "__strchr_chk",
    "clock",
    "fileno",
    "remainderf",
    "nan",
    "qsort",
    "nextafterf",
    "acos",
    "asin",
    "ilogb",
    "__FD_SET_chk",
    "select",
    "__FD_ISSET_chk",
    "sendto",
    "recvfrom",
    "__strcat_chk",
    "setpriority",
    "pthread_mutexattr_init",
    "pthread_mutexattr_settype",
    "pthread_mutexattr_destroy",
    "sem_init",
    "sem_destroy",
    "sem_wait",
    "sem_post",
    "inet_ntop",
    "inet_pton",
    "strncasecmp",
    "pthread_attr_setschedparam",
    "bind",
    "getsockname",
    "gethostname",
    "__sendto_chk",
    "puts",
    "gai_strerror",
    "__write_chk",
    "__poll_chk",
    "vprintf",
    "usleep",
    "pthread_kill",
    "pthread_detach",
    "exit",
    "ferror",
    "clearerr",
    "wcslen",
    "wmemcmp",
    "exp",
    "pow",
    "fmod",
    "log",
    "log2",
    "log10",
    "round",
    "frexp",
    "sin",
    "sinh",
    "cos",
    "cosh",
    "tan",
    "tanh",
    "atol",
    "atof",
    "strspn",
    "strtof",
    "ioctl",
    "getpeername",
    "listen",
    "accept",
    "epoll_create",
    "__FD_CLR_chk",
    "expm1",
    "if_indextoname",
    "sigaddset",
    "pthread_sigmask",
    "fgets",
    "setjmp",
    "longjmp",
    "pthread_condattr_init",
    "pthread_condattr_setclock",
    "pthread_condattr_destroy",
    "sched_get_priority_min",
    "pthread_setschedparam",
    "getgid",
    "getegid",
    "random",
    "sigfillset",
    "fdopen",
    "timerfd_create",
    "timerfd_settime",
    "fputc",
    "bsearch",
    "vfprintf",
    "pthread_exit",
    "finitef",
    "cbrt",
    "remquof",
    "strcspn",
    "gmtime_r",
    "difftime",
    "strpbrk",
    "shutdown",
    "memrchr",
    "accept4",
    "if_nametoindex",
    "setvbuf",
    "realpath",
    "recvmmsg",
    "getcwd",
    "pread",
    "pwrite",
    "fchmod",
    "fchown",
    "mremap",
    "fsync",
    "utimes",
    "msync",
    "statvfs",
    "mallinfo",
    "__readlink_chk",
    "__gnu_strerror_r",
    "pthread_getschedparam",
    "sinf",
    "sincosf",
    "exp2",
    "sincos",
    "fmal",
    "exp2f",
    "log10f",
    "logf",
    "powf",
    "fmodf",
    "log2f",
    "expf",
    "powl",
    "cosf",
    "pthread_key_delete",
    "sysinfo",
    "madvise",
    "pthread_setname_np",
    "pthread_getattr_np",
    "pthread_attr_getstack",
    "mlock",
    "dl_iterate_phdr",
    "isspace",
    "gethostbyname",
    "strcat",
    "sendmmsg",
    "tolower",
    "pthread_rwlock_destroy",
    "pthread_rwlock_init",
    "pthread_rwlock_rdlock",
    "pthread_rwlock_unlock",
    "pthread_rwlock_wrlock",
    "signal",
    "tcgetattr",
    "tcsetattr",
    "utime",
    "vasprintf",
    "openlog",
    "syslog",
    "closelog",
    "ungetc",
    "getc",
    "ungetwc",
    "getwc",
    "fputwc",
    "newlocale",
    "uselocale",
    "vsscanf",
    "strftime_l",
    "mbsrtowcs",
    "freelocale",
    "strcoll_l",
    "strxfrm_l",
    "wcscoll_l",
    "wcsxfrm_l",
    "iswlower_l",
    "iswspace_l",
    "iswprint_l",
    "iswblank_l",
    "iswcntrl_l",
    "iswupper_l",
    "iswalpha_l",
    "iswdigit_l",
    "iswpunct_l",
    "iswxdigit_l",
    "towupper_l",
    "towlower_l",
    "btowc",
    "wctob",
    "wcsnrtombs",
    "wcrtomb",
    "mbsnrtowcs",
    "mbrtowc",
    "mbtowc",
    "__ctype_get_mb_cur_max",
    "mbrlen",
    "strtoll_l",
    "strtoull_l",
    "strtold_l",
    "__cxa_thread_atexit_impl",
    "strncat",
};

#define BF_TABLE_COUNT (sizeof(bf_table_syms)/sizeof(bf_table_syms[0]))

static void fill_bionic_table(void **table) {
    for (size_t i = 0; i < BF_TABLE_COUNT; i++) {
        if (!table[i]) {
            table[i] = dlsym(RTLD_DEFAULT, bf_table_syms[i]);
        }
    }
}
// Main function - loads libroblox.so and calls JNI_OnLoad
int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    // Load the bionic shim (provides @LIBC versioned symbols)
    // We fill its dispatch table from THIS binary (not from inside the shim)
    // to avoid PLT circularity: our calls to dlsym resolve to glibc directly.
    fprintf(stderr, "[jni_shim] Loading bionic shim...\n");
    void *bionic_shim = dlopen("libbionic_shim.so", RTLD_LAZY | RTLD_GLOBAL);
    if (!bionic_shim) {
        fprintf(stderr, "[jni_shim] WARNING: libbionic_shim.so not found: %s\n", dlerror());
    } else {
        // Get the dispatch table address and fill it from here
        void **table = dlsym(bionic_shim, "__bf_tramp_table");
        if (table) {
            fprintf(stderr, "[jni_shim] Filling bionic dispatch table (%zu entries)...\n",
                    sizeof(bf_table_syms)/sizeof(bf_table_syms[0]));
            for (size_t i = 0; i < sizeof(bf_table_syms)/sizeof(bf_table_syms[0]); i++) {
                if (!table[i]) {
                    table[i] = dlsym(RTLD_DEFAULT, bf_table_syms[i]);
                }
            }
        } else {
            fprintf(stderr, "[jni_shim] WARNING: __bf_tramp_table not found in bionic shim\n");
        }
    }

    // Initialize bionic data objects directly from this binary.
    // We do the dlsym calls HERE (not from the shim) because calling dlsym
    // from inside the shim goes through its own trampolines, which can
    // cause infinite recursion. From this binary, dlsym goes to glibc directly.
    fprintf(stderr, "[jni_shim] Initializing bionic data objects...\n");
    {
        // Data symbols to initialize (name, address of data slot in shim)
        struct { const char *name; const char *glibc_sym; } data_syms[] = {
            {"__bf_data_stderr",     "stderr"},
            {"__bf_data___sF",       "_IO_2_1_stderr_"},
            {"__bf_data_optarg",     "optarg"},
            {"__bf_data_optind",     "optind"},
            {"__bf_data_tzname",     "tzname"},
            {"__bf_data_daylight",   "daylight"},
            {"__bf_data_timezone",   "timezone"},
            {"__bf_data_environ",    "environ"},
            {"__bf_data_in6addr_any","in6addr_any"},
            {"__bf_data_stdin",      "stdin"},
            {"__bf_data_stdout",     "stdout"},
            {"__bf_data_in6addr_loopback", "in6addr_loopback"},
        };
        void *rtld_default_handle = dlopen(NULL, RTLD_LAZY | RTLD_NOLOAD);
        for (size_t i = 0; i < sizeof(data_syms)/sizeof(data_syms[0]); i++) {
            void **slot = dlsym(bionic_shim, data_syms[i].name);
            if (slot && !*slot) {
                *slot = dlsym(rtld_default_handle ? rtld_default_handle : RTLD_DEFAULT,
                              data_syms[i].glibc_sym);
            }
        }
        if (rtld_default_handle) dlclose(rtld_default_handle);

        // Initialize __stack_chk_guard with a non-zero canary
        void **canary_slot = dlsym(bionic_shim, "__bf_data___stack_chk_guard");
        if (canary_slot && !*canary_slot) {
            unsigned long long c = 0xdeadbeefcafebabeULL;
            // Try to get a random canary from glibc
            void *glibc_canary = dlsym(RTLD_DEFAULT, "__stack_chk_guard");
            if (glibc_canary) c = *(unsigned long long*)glibc_canary;
            if (!c) c = 0xdeadbeefcafebabeULL;
            *canary_slot = (void*)c;
        }
    }

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);

    void* handle = dlopen(lib_path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "[jni_shim] Failed to load %s: %s\n", lib_path, dlerror());
        return 1;
    }

    fprintf(stderr, "[jni_shim] Loaded %s successfully\n", lib_path);

    // Call JNI_OnLoad
    typedef jint (*jni_onload_t)(JavaVM*, void*);
    jni_onload_t jni_onload = (jni_onload_t)dlsym(handle, "JNI_OnLoad");
    if (jni_onload) {
        jint version = jni_onload(&g_vm, NULL);
        fprintf(stderr, "[jni_shim] JNI_OnLoad returned version 0x%x\n", version);
    } else {
        fprintf(stderr, "[jni_shim] No JNI_OnLoad symbol found in %s\n", lib_path);
    }

    // Keep running until signal
    fprintf(stderr, "[jni_shim] Entering main loop...\n");

    // Call the game's main if it exists
    typedef int (*main_func_t)(int, char**);
    main_func_t game_main = (main_func_t)dlsym(handle, "main");
    if (game_main) {
        fprintf(stderr, "[jni_shim] Calling game main()\n");
        return game_main(argc, argv);
    }

    // Otherwise just wait
    while (1) sleep(1);

    dlclose(handle);
    return 0;
}