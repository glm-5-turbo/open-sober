// Minimal JNI shim to load libroblox.so and call JNI_OnLoad.
// This bypasses the need for a full Android Java runtime.

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <stdarg.h>
#include <sys/mman.h>
#include <signal.h>
#include <ucontext.h>
#include <errno.h>
#include <link.h>

// Minimal JNI types
typedef int jint;
typedef unsigned char jboolean;
typedef unsigned short jchar;
typedef short jshort;
typedef long long jlong;
typedef float jfloat;
typedef double jdouble;
typedef void* jobject;
typedef void* jclass;
typedef void* jstring;
typedef void* jmethodID;
typedef void* jfieldID;
typedef void* jthrowable;
typedef void* jarray;
typedef void* jobjectArray;
typedef union { jboolean z; int b; jchar c; jshort s; jint i; jlong j; jfloat f; jdouble d; jobject l; } jvalue;

#define JNI_OK 0
#define JNI_VERSION_1_6 0x00010006

struct JNIEnv_;
typedef struct JNIEnv_ JNIEnv;
struct JavaVM_;
typedef struct JavaVM_ JavaVM;
typedef struct { const char* name; const char* signature; void* fnPtr; } JNINativeMethod;

#define STUB_LOG(fmt, ...) do { fprintf(stderr, "[jni] " fmt "\n", ##__VA_ARGS__); fflush(stderr); } while(0)

/* Global canary value */
static uintptr_t g_canary = 0x0A0B0C0D0E0F1011ULL;
static uintptr_t g_libroblox_base = 0;

// ============== Tracking stub helpers ==============

// Track classes, methods, and fields to return unique pointers per name
// so that Roblox can distinguish between different classes/methods
#define MAX_TRACKED 512
static char *tracked_names[MAX_TRACKED];
static void *tracked_ptrs[MAX_TRACKED];
static int tracked_count = 0;

static void *track_ptr(const char *name) {
    if (!name) name = "";
    for (int i = 0; i < tracked_count; i++)
        if (strcmp(tracked_names[i], name) == 0)
            return tracked_ptrs[i];
    if (tracked_count >= MAX_TRACKED) return (void*)(uintptr_t)(0x1000 + tracked_count);
    char *copy = strdup(name);
    // Use the string address itself as the unique pointer
    if (copy) { tracked_names[tracked_count] = copy; tracked_ptrs[tracked_count] = copy; }
    else { tracked_names[tracked_count] = (char*)name; tracked_ptrs[tracked_count] = (void*)(uintptr_t)(0x2000 + tracked_count); }
    return tracked_ptrs[tracked_count++];
}

// ============== Stub functions ==============

static void* stub_voidp(void) { static char buf[64]; return buf; }
static jint stub_GetVersion(JNIEnv* e) { STUB_LOG("GetVersion"); return JNI_VERSION_1_6; }
static jclass stub_FindClass(JNIEnv* e, const char* n) {
    static int count = 0; count++;
    if (count <= 20 || (count % 100) == 0)
        STUB_LOG("FindClass[%d]: %s", count, n?n:"NULL");
    return (jclass)track_ptr(n);
}
static jmethodID stub_GetMethodID(JNIEnv* e, jclass c, const char* n, const char* s) {
    static int count = 0; count++;
    if (count <= 20 || (count % 100) == 0)
        STUB_LOG("GetMethodID[%d]: %s %s", count, n?n:"NULL", s?s:"NULL");
    return (jmethodID)track_ptr(n);
}
static jmethodID stub_GetStaticMethodID(JNIEnv* e, jclass c, const char* n, const char* s) {
    static int count = 0; count++;
    if (count <= 20 || (count % 100) == 0)
        STUB_LOG("GetStaticMethodID[%d]: %s %s", count, n?n:"NULL", s?s:"NULL");
    return (jmethodID)track_ptr(n);
}
static jfieldID stub_GetFieldID(JNIEnv* e, jclass c, const char* n, const char* s) { return (jfieldID)track_ptr(n); }
static jfieldID stub_GetStaticFieldID(JNIEnv* e, jclass c, const char* n, const char* s) { return (jfieldID)(uintptr_t)0x4001; }
static jstring stub_NewStringUTF(JNIEnv* e, const char* u) { return (jstring)track_ptr(u); }
static const char* stub_GetStringUTFChars(JNIEnv* e, jstring s, jboolean* c) { if(c)*c=0; return s ? (const char*)s : ""; }
static void stub_ReleaseStringUTFChars(JNIEnv* e, jstring s, const char* u) {}
static jint stub_RegisterNatives(JNIEnv* e, jclass c, const JNINativeMethod* m, jint n) {
    STUB_LOG("RegisterNatives: %d methods", n);
    for (jint i = 0; i < n && i < 5; i++)
        STUB_LOG("  native[%d]: name=%s sig=%s fn=%p", i, m[i].name?:"?", m[i].signature?:"?", m[i].fnPtr);
    return JNI_OK;
}
static jobject stub_NewGlobalRef(JNIEnv* e, jobject o) { return o; }
static void stub_DeleteGlobalRef(JNIEnv* e, jobject o) {}
static void stub_DeleteLocalRef(JNIEnv* e, jobject o) {}
static jint stub_ThrowNew(JNIEnv* e, jclass c, const char* m) { STUB_LOG("ThrowNew: %s", m?m:"NULL"); return 0; }
static jint stub_GetJavaVM(JNIEnv* e, JavaVM** v) { *v = (JavaVM*)0x1000; return 0; }
static jobject stub_NewObject(JNIEnv* e, jclass c, jmethodID m, ...) { return stub_voidp(); }
#define CALL_STUB(prefix, ret_type, ret_val) \
    static ret_type stub_##prefix##Method(JNIEnv* e, jobject o, jmethodID m, ...) { return ret_val; } \
    static ret_type stub_##prefix##MethodV(JNIEnv* e, jobject o, jmethodID m, va_list a) { return ret_val; } \
    static ret_type stub_##prefix##MethodA(JNIEnv* e, jobject o, jmethodID m, const jvalue* a) { return ret_val; }
CALL_STUB(CallObject, jobject, stub_voidp())
CALL_STUB(CallBoolean, jboolean, 1)
CALL_STUB(CallByte, jint, 0)
CALL_STUB(CallChar, jint, 0)
CALL_STUB(CallShort, jint, 0)
CALL_STUB(CallInt, jint, 0)
CALL_STUB(CallLong, jlong, 0)
CALL_STUB(CallFloat, jfloat, 0)
CALL_STUB(CallDouble, jdouble, 0)
static void stub_CallVoidMethod(JNIEnv* e, jobject o, jmethodID m, ...) {}
static void stub_CallVoidMethodV(JNIEnv* e, jobject o, jmethodID m, va_list a) {}
static void stub_CallVoidMethodA(JNIEnv* e, jobject o, jmethodID m, const jvalue* a) {}
#define CALL_STATIC_STUB(prefix, ret_type, ret_val) \
    static ret_type stub_##prefix##Method(JNIEnv* e, jclass c, jmethodID m, ...) { return ret_val; } \
    static ret_type stub_##prefix##MethodV(JNIEnv* e, jclass c, jmethodID m, va_list a) { return ret_val; } \
    static ret_type stub_##prefix##MethodA(JNIEnv* e, jclass c, jmethodID m, const jvalue* a) { return ret_val; }
CALL_STATIC_STUB(CallStaticObject, jobject, stub_voidp())
CALL_STATIC_STUB(CallStaticBoolean, jboolean, 1)
CALL_STATIC_STUB(CallStaticByte, jint, 0)
CALL_STATIC_STUB(CallStaticChar, jint, 0)
CALL_STATIC_STUB(CallStaticShort, jint, 0)
CALL_STATIC_STUB(CallStaticInt, jint, 0)
CALL_STATIC_STUB(CallStaticLong, jlong, 0)
CALL_STATIC_STUB(CallStaticFloat, jfloat, 0)
CALL_STATIC_STUB(CallStaticDouble, jdouble, 0)
static void stub_CallStaticVoidMethod(JNIEnv* e, jclass c, jmethodID m, ...) {}
static void stub_CallStaticVoidMethodV(JNIEnv* e, jclass c, jmethodID m, va_list a) {}
static void stub_CallStaticVoidMethodA(JNIEnv* e, jclass c, jmethodID m, const jvalue* a) {}
static jobject stub_GetObjectField(JNIEnv* e, jobject o, jfieldID f) { return stub_voidp(); }
static void stub_SetObjectField(JNIEnv* e, jobject o, jfieldID f, jobject v) {}
static jint stub_GetIntField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static void stub_SetIntField(JNIEnv* e, jobject o, jfieldID f, jint v) {}
static jlong stub_GetLongField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static void stub_SetLongField(JNIEnv* e, jobject o, jfieldID f, jlong v) {}
static jboolean stub_GetBooleanField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static jfloat stub_GetFloatField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static jobject stub_GetStaticObjectField(JNIEnv* e, jclass c, jfieldID f) { return stub_voidp(); }
static jint stub_GetStaticIntField(JNIEnv* e, jclass c, jfieldID f) { return 0; }
static void stub_SetStaticIntField(JNIEnv* e, jclass c, jfieldID f, jint v) {}
static jlong stub_GetStaticLongField(JNIEnv* e, jclass c, jfieldID f) { return 0; }
static jint stub_GetArrayLength(JNIEnv* e, jarray a) { return 0; }
static jobject stub_GetObjectArrayElement(JNIEnv* e, jobjectArray a, jint i) { return stub_voidp(); }
static void stub_SetObjectArrayElement(JNIEnv* e, jobjectArray a, jint i, jobject v) {}
static jobjectArray stub_NewObjectArray(JNIEnv* e, jint l, jclass c, jobject i) { return (jobjectArray)stub_voidp(); }
static jthrowable stub_ExceptionOccurred(JNIEnv* e) { return NULL; }
static void stub_ExceptionDescribe(JNIEnv* e) {}
static void stub_ExceptionClear(JNIEnv* e) {}
static jint stub_Throw(JNIEnv* e, jthrowable o) { return 0; }

// ============== JNI function table ==============

#define JNI_SLOTS 256
static void* jni_table[JNI_SLOTS];

struct JavaVM_ { const void** functions; void* reserved[4]; };
struct JNIEnv_ { const void** functions; void* reserved[4]; };

#define VM_SLOTS 16
static void* vm_table[VM_SLOTS];
static JNIEnv g_env;
static JavaVM g_vm;

static jint stub_GetEnv_Attach(JavaVM* vm, void** penv, void* args) {
    (void)args; if (!g_env.functions) g_env.functions = (const void**)jni_table; *penv = &g_env; return 0;
}
static jint stub_GetEnv_jint(JavaVM* vm, void** penv, jint version) {
    (void)version; if (!g_env.functions) g_env.functions = (const void**)jni_table; *penv = &g_env; return 0;
}
static jint stub_DestroyJavaVM(JavaVM* vm) { return 0; }
static jint stub_DetachCurrentThread(JavaVM* vm) { return 0; }

__attribute__((constructor))
static void init_jni_functions() {
    memset(jni_table, 0, sizeof(jni_table));
    jni_table[4] = stub_GetVersion;
    for (int i = 5; i <= 40; i++) jni_table[i] = stub_GetVersion;
    jni_table[5] = stub_FindClass; jni_table[6] = stub_FindClass;
    jni_table[7] = stub_GetMethodID; jni_table[8] = stub_GetFieldID;
    jni_table[9] = stub_voidp; jni_table[10] = stub_FindClass;
    jni_table[13] = stub_Throw; jni_table[14] = stub_ThrowNew;
    jni_table[15] = stub_ExceptionOccurred; jni_table[16] = stub_ExceptionDescribe;
    jni_table[17] = stub_ExceptionClear; jni_table[18] = (void*)abort;
    jni_table[21] = stub_NewGlobalRef; jni_table[22] = stub_DeleteGlobalRef;
    jni_table[23] = stub_DeleteLocalRef; jni_table[25] = stub_NewGlobalRef;
    jni_table[27] = jni_table[28] = jni_table[29] = jni_table[30] = stub_NewObject;
    jni_table[33] = stub_GetMethodID; jni_table[36] = stub_NewStringUTF;
    jni_table[37] = stub_GetArrayLength; jni_table[38] = stub_NewObjectArray;
    jni_table[39] = stub_GetObjectArrayElement; jni_table[40] = stub_SetObjectArrayElement;
    for (int i = 41; i <= 100; i++) jni_table[i] = (void*)stub_CallIntMethod;
    jni_table[101] = stub_GetFieldID;
    jni_table[102] = stub_GetObjectField; jni_table[103] = stub_SetObjectField;
    jni_table[193] = stub_RegisterNatives; jni_table[197] = stub_GetJavaVM;
    for (int i = 200; i < JNI_SLOTS; i++) jni_table[i] = (void *)stub_GetVersion;
    vm_table[3] = stub_DestroyJavaVM; vm_table[4] = stub_GetEnv_Attach;
    vm_table[5] = stub_DetachCurrentThread; vm_table[6] = stub_GetEnv_jint;
    vm_table[7] = stub_GetEnv_Attach;
    g_vm.functions = (const void**)vm_table;
    for (int i = 0; i < JNI_SLOTS; i++) if (jni_table[i] == NULL) jni_table[i] = (void *)stub_GetVersion;
}

// ============== Mutex sanitization wrappers ==============
// Bionic's pthread_mutex_t is 44 bytes, glibc's is 40 bytes.
// Key differences:
//   Bionic offset  8: __owner (pid_t or 0)
//   glibc offset   8: __count (recursion count)
//   Bionic offset 12: __rc (maybe 0)
//   glibc offset  12: __owner
//   Bionic offset 16: __kind (can be 0x10 = ROBUST_NORMAL)
//   glibc offset   16: __kind (only 0,1,2 valid)
//
// When libroblox calls glibc's pthread_mutex_lock on a Bionic-initialized mutex:
// - glibc sees __kind=0x10 (unknown type) and either crashes or deadlocks
// - glibc sees __count (offset 8) != 0 and thinks mutex is recursively held
// Solution: intercept mutex_init to match glibc layout, and sanitize in lock.

// Real function pointers saved so our wrappers can delegate
typedef int (*mutex_fn)(void*);
typedef int (*mutex_init_fn)(void*, void*);
typedef int (*cond_wait_fn)(void*, void*);
typedef int (*cond_timedwait_fn)(void*, void*, const void*);

static mutex_fn g_real_lock = NULL;
static mutex_fn g_real_unlock = NULL;
static mutex_init_fn g_real_mutex_init = NULL;
static cond_wait_fn g_real_cond_wait = NULL;
static cond_timedwait_fn g_real_cond_timedwait = NULL;

#define BIONIC_PTHREAD_MUTEX_ROBUST_NORMAL  0x10
#define BIONIC_PTHREAD_MUTEX_ROBUST_RECURSIVE 0x11

// Sanitize a mutex in-place to make it glibc-compatible
__attribute__((always_inline))
static inline void sanitize_mutex(void *mutex) {
    if (!mutex) return;
    int kind = *(volatile int*)((char*)mutex + 16);
    if ((kind & ~3) != 0) {
        // Bionic initialized with ROBUST_NORMAL or other bad type
        *(volatile int*)((char*)mutex + 16) = kind & 3;
    }
    // Clear __count (glibc offset 8) — Bionic's __owner leaks into it
    // Only clear if it looks like a Bionic owner value (not a recursion count)
    int cnt = *(volatile int*)((char*)mutex + 8);
    if (cnt > 0x10000 || cnt < 0) {
        *(volatile int*)((char*)mutex + 8) = 0;
    }
}

// Wrapped mutex_init: call glibc's real init, then sanitize
__attribute__((noinline))
static int wrap_mutex_init(void *mutex, void *attr) {
    int ret = g_real_mutex_init(mutex, attr);
    if (ret == 0 && mutex) {
        // Ensure __kind is valid for glibc
        *(volatile int*)((char*)mutex + 16) &= 3;
        // Clear __count and __owner to start clean
        *(volatile int*)((char*)mutex + 8) = 0;
        *(volatile int*)((char*)mutex + 12) = 0;
    }
    return ret;
}

// Wrapped mutex_lock: sanitize before locking
__attribute__((noinline))
static int wrap_mutex_lock(void *mutex) {
    sanitize_mutex(mutex);
    return g_real_lock(mutex);
}

// Wrapped cond_wait: the condvar might also need sanitization
// On entry, the mutex is locked (by caller). pthread_cond_wait will
// atomically unlock it and sleep. On return, it re-locks.
// If mutex is corrupted, this chain breaks.
// We sanitize BEFORE cond_wait to ensure atomic unlock works.
__attribute__((noinline))
static int wrap_cond_wait(void *cond, void *mutex) {
    sanitize_mutex(mutex);
    return g_real_cond_wait(cond, mutex);
}

__attribute__((noinline))
static int wrap_cond_timedwait(void *cond, void *mutex, const void *abstime) {
    sanitize_mutex(mutex);
    return g_real_cond_timedwait(cond, mutex, abstime);
}

// ============== SIGSEGV handler ==============

static struct sigaction jni_old_sa;
static int jni_segv_count = 0;

static void jni_segv_handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t *u = (ucontext_t*)ctx;
    uintptr_t fault_addr = (uintptr_t)info->si_addr;
    uintptr_t fault_page = fault_addr & ~0xfffULL;
    uintptr_t pc = u->uc_mcontext.pc;
    jni_segv_count++;

    if (jni_segv_count > 500) {
        fprintf(stderr, "[jni_segv] #%d: too many faults\n", jni_segv_count);
        sigaction(SIGSEGV, &jni_old_sa, NULL); return;
    }

    // Bad address (0, -1, -4096) — genuine fault, abort
    if (fault_addr < 0x1000 || fault_addr == (uintptr_t)-1 || fault_page == ~0xfffULL) {
        fprintf(stderr, "[jni_segv] #%d: bad addr=0x%lx pc=0x%lx\n", jni_segv_count, fault_addr, pc);
        sigaction(SIGSEGV, &jni_old_sa, NULL); return;
    }

    // Self-write (pc == fault_addr): QEMU JIT produced bad translation.
    // Toggle page permissions to force JIT cache invalidation.
    if (pc == fault_addr) {
        mprotect((void*)fault_page, 0x1000, PROT_NONE);
        *(volatile int*)fault_page = 0;
        mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE|PROT_EXEC);
        if (jni_segv_count <= 5)
            fprintf(stderr, "[jni_segv] #%d: self-write pc=0x%lx toggle-ok\n", jni_segv_count, pc);
        return;
    }

    // Other fault: try mprotecting the page then advance
    // First try RW, if that fails try RWX (GOT section might need execute too)
    int mpret = mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE);
    if (mpret != 0)
        mpret = mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE|PROT_EXEC);
    if (mpret == 0) {
        u->uc_mcontext.pc = pc + 4;
        if (jni_segv_count <= 5)
            fprintf(stderr, "[jni_segv] #%d: mprotect 0x%lx advance\n", jni_segv_count, fault_page);
        return;
    }

    // If mprotect fails, try advancing PC by 4 without mprotect
    // (transient QEMU TLB issue — retry might work after the fault handler returns)
    // Only do this once to avoid infinite loops
    // Skip-retry was removed because it can skip store instructions
    // and cause stack smashing. Just chain to the old handler.

    // Can't handle — chain
    fprintf(stderr, "[jni_segv] #%d: unhandled pc=0x%lx fault=0x%lx\n", jni_segv_count, pc, fault_addr);
    sigaction(SIGSEGV, &jni_old_sa, NULL);
}

// ============== Pre-mprotect RELRO via /proc/self/maps ==============

static void pre_mprotect_relro(void) {
    FILE *maps = fopen("/proc/self/maps", "r");
    if (!maps) return;
    char line[512]; int count = 0;
    while (fgets(line, sizeof(line), maps)) {
        uintptr_t s, e; char perms[8]; char path[256] = "";
        int n = sscanf(line, "%lx-%lx %7s %*x %*x:%*x %*d %255s", &s, &e, perms, path);
        if (n >= 3 && strcmp(perms, "r--p") == 0) {
            if (path[0] && !strstr(path, "ld-linux") && !strstr(path, "libc.so") &&
                !strstr(path, "libm.so") && !strstr(path, "libdl.so") &&
                !strstr(path, "libpthread.so") && !strstr(path, "bionic_shim") &&
                !strstr(path, "[vdso]")) {
                // Do single mprotect to RW
                if (mprotect((void*)s, e - s, PROT_READ|PROT_WRITE) == 0) {
                    count++;
                }
            }
        }
    }
    fclose(maps);
    if (count) fprintf(stderr, "[mprotect] %d RELRO pages -> RW (with TLB flush)\n", count);
}

// ============== Main ==============

int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    fprintf(stderr, "[jni_shim] Loading bionic shim...\n");
    void *bionic_shim = dlopen("libbionic_shim.so", RTLD_LAZY | RTLD_GLOBAL);
    if (!bionic_shim)
        fprintf(stderr, "[jni_shim] WARNING: libbionic_shim.so not found: %s\n", dlerror());

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);
    void* handle = dlopen(lib_path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) { fprintf(stderr, "[jni_shim] Failed: %s\n", dlerror()); return 1; }
    fprintf(stderr, "[jni_shim] Loaded successfully\n");

    // Fix __stack_chk_guard
    Dl_info dl_info;
    if (dladdr((void*)dlsym(handle, "JNI_OnLoad"), &dl_info)) {
        uintptr_t base = (uintptr_t)dl_info.dli_fbase;
        // Pre-mprotect the entire GOT section to avoid RELRO faults
        // .got: VA 0x646B7E8, size 0x7C58; .got.plt: VA 0x6473440, size 0x10E8
        // Combined range: 0x646B000 to 0x6475000 should cover both
        uintptr_t got_start = base + 0x646B000;
        uintptr_t got_end = base + 0x6475000;
        mprotect((void*)got_start, got_end - got_start, PROT_READ|PROT_WRITE);
        // Also mprotect the init_array section which may contain relocations
        mprotect((void*)(base + 0x6464000), 0x8000, PROT_READ|PROT_WRITE);

        // The canary GOT entry at base + 0x6473438 (adrp target 0x6473000 + ldr offset 0x438)
        uintptr_t guard_ptr_addr = base + 0x6473438;
        uintptr_t* guard_ptr = (uintptr_t*)guard_ptr_addr;
        uintptr_t canary_page = guard_ptr_addr & ~0xfffULL;
        int mp_ret = mprotect((void*)canary_page, 0x1000, PROT_READ|PROT_WRITE);
        fprintf(stderr, "[jni_shim] base=%p guard=%p val=%p canary=%p mprotect=%d errno=%d\n",
                (void*)base, (void*)guard_ptr, (void*)*guard_ptr, (void*)&g_canary,
                mp_ret, errno);
        if (*guard_ptr == 0) {
            *guard_ptr = (uintptr_t)&g_canary;
            fprintf(stderr, "[jni_shim] wrote canary -> %p\n", (void*)*guard_ptr);
        }
        uintptr_t* libc_guard = (uintptr_t*)dlsym(RTLD_NEXT, "__stack_chk_guard");
        if (libc_guard && *libc_guard == 0) *libc_guard = g_canary;
        g_libroblox_base = base;
    }

    // Install SIGSEGV handler
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = jni_segv_handler;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, &jni_old_sa);
    fprintf(stderr, "[jni_shim] SIGSEGV handler installed\n");

    // Pre-mprotect RELRO
    pre_mprotect_relro();

    // Install mutex sanitization wrappers into bionic_shim trampoline table
    // Bypass __bf_c_resolve (uses buggy dlsym(RTLD_NEXT) under QEMU)
    // Resolve via RTLD_DEFAULT and install our own sanitize wrappers
    if (bionic_shim) {
        void **tramp = (void **)dlsym(bionic_shim, "__bf_tramp_table");
        if (tramp) {
            // Pre-resolve ALL key functions to bypass lazy resolver
            // The lazy resolver uses dlsym(RTLD_NEXT) which can fail under QEMU user-mode
            struct { int idx; const char *name; } pre[] = {
                {0,"__cxa_finalize"},{1,"__cxa_atexit"},{2,"__register_atfork"},
                {3,"strlen"},{4,"memcmp"},
                {5,"pthread_mutex_init"},{6,"pthread_mutex_destroy"},{7,"pthread_once"},
                {8,"__memset_chk"},{9,"__memcpy_chk"},{10,"strlen"},{11,"memchr"},
                {12,"strncmp"},{13,"strcmp"},{14,"getauxval"},{15,"__errno_location"},
                {16,"close"},{17,"__open_2"},{18,"__read_chk"},{19,"read"},
                {20,"clock_gettime"},{21,"syscall"},{22,"sched_getcpu"},{23,"sysconf"},
                {24,"mmap"},{25,"mprotect"},{26,"munmap"},
                {27,"pthread_attr_init"},{28,"pthread_attr_setstacksize"},
                {29,"pthread_create"},{30,"pthread_attr_destroy"},{31,"pthread_join"},
                {32,"pthread_self"},{33,"memset"},{34,"pthread_setspecific"},
                {35,"__strncpy_chk"},{36,"sched_get_priority_max"},{37,"sched_setscheduler"},
                {38,"pthread_mutex_lock"},{39,"pthread_cond_wait"},{40,"pthread_mutex_unlock"},
                {41,"pthread_cond_signal"},{42,"pthread_cond_init"},{43,"gettid"},
                {44,"__vsprintf_chk"},{45,"atan2f"},{46,"vsnprintf"},{47,"fprintf"},
                {48,"rand"},{49,"strtoll"},{50,"time"},{51,"pthread_cond_broadcast"},
                {52,"pthread_cond_destroy"},{53,"stat"},{54,"opendir"},{55,"readdir"},
                {56,"closedir"},{57,"posix_fallocate"},{58,"open"},{59,"pthread_equal"},
                {60,"atoll"},{61,"srand"},{62,"strtod"},{63,"localtime"},{64,"fseek"},
                {65,"ftell"},{66,"fclose"},{67,"atan"},{68,"mkdir"},{69,"__fread_chk"},
                {70,"fread"},{71,"asinf"},{72,"access"},{73,"getenv"},{74,"strncpy"},
                {75,"feof"},{76,"fopen"},{77,"fseeko"},{78,"ftello"},{79,"fwrite"},
                {80,"fflush"},{81,"gmtime"},{82,"mktime"},{83,"pipe"},{84,"write"},
                {85,"pthread_key_create"},{86,"abort"},{87,"__assert"},{88,"strrchr"},
                {89,"__stack_chk_fail"},{90,"atoi"},{91,"fcntl"},{92,"memcpy"},
                {93,"strcpy"},{94,"strerror"},{95,"pthread_attr_setdetachstate"},
                {96,"pthread_cond_timedwait"},{97,"memmove"},{98,"strtol"},{99,"getpid"},
                {100,"gettimeofday"},{101,"localtime_r"},{102,"fputs"},{103,"strerror_r"},
                {104,"snprintf"},{105,"prctl"},{106,"sigaltstack"},{107,"getpagesize"},
                {108,"pthread_getspecific"},{109,"fork"},{110,"waitpid"},
                {111,"execv"},{112,"_exit"},{113,"execve"},{114,"getopt_long"},
                {115,"getppid"},{116,"geteuid"},{117,"epoll_create1"},{118,"eventfd"},
                {119,"epoll_ctl"},{120,"getsockopt"},{121,"setsockopt"},{122,"epoll_wait"},
                {123,"getuid"},{124,"pthread_mutex_trylock"},{125,"ldexp"},{126,"strchr"},
                {127,"getaddrinfo"},{128,"socket"},{129,"freeaddrinfo"},{130,"connect"},
                {131,"poll"},{132,"sched_getscheduler"},{133,"sched_getparam"},
                {134,"getpriority"},{135,"uname"},{136,"tzset"},{137,"strnlen"},
                {138,"writev"},{139,"sscanf"},{140,"strtoul"},{141,"strtoull"},
                {142,"lseek"},{143,"ftruncate"},{144,"fstat"},{145,"lstat"},
                {146,"rename"},{147,"unlink"},{148,"rmdir"},{149,"nanosleep"},
                {150,"sigemptyset"},{151,"sigaction"},{152,"raise"},{153,"fscanf"},
                {154,"pread64"},{155,"ptrace"},{156,"socketpair"},{157,"sendmsg"},
                {158,"recvmsg"},{159,"__cmsg_nxthdr"},{160,"readlink"},{161,"printf"},
                {162,"wmemchr"},{163,"localeconv"},{164,"__vsnprintf_chk"},
                {165,"__memmove_chk"},{166,"sched_yield"},{167,"modf"},{168,"strcasecmp"},
                {169,"getnameinfo"},{170,"strftime"},{171,"__strcpy_chk"},{172,"frexpf"},
                {173,"ldexpf"},{174,"tanf"},{175,"atanf"},{176,"erff"},{177,"acosf"},
                {178,"strstr"},{179,"erfcf"},{180,"modff"},{181,"coshf"},{182,"sinhf"},
                {183,"tanhf"},{184,"atan2"},{185,"cbrtf"},{186,"strchr"},{187,"clock"},
                {188,"fileno"},{189,"remainderf"},{190,"nan"},{191,"qsort"},
                {192,"nextafterf"},{193,"acos"},{194,"asin"},{195,"ilogb"},
                {196,"FD_SET"},{197,"select"},{198,"FD_ISSET"},{199,"sendto"},
                {200,"recvfrom"},{201,"__strcat_chk"},{202,"setpriority"},
                {203,"pthread_mutexattr_init"},{204,"pthread_mutexattr_settype"},
                {205,"pthread_mutexattr_destroy"},{206,"sem_init"},{207,"sem_destroy"},
                {208,"sem_wait"},{209,"sem_post"},{210,"inet_ntop"},{211,"inet_pton"},
                {212,"strncasecmp"},{213,"pthread_attr_setschedparam"},{214,"bind"},
                {215,"getsockname"},{216,"gethostname"},{217,"__sendto_chk"},{218,"puts"},
                {219,"gai_strerror"},{220,"__write_chk"},{221,"__poll_chk"},{222,"vprintf"},
                {223,"usleep"},{224,"pthread_kill"},{225,"pthread_detach"},{226,"exit"},
                {227,"ferror"},{228,"clearerr"},{229,"wcslen"},{230,"wmemcmp"},{231,"exp"},
                {232,"pow"},{233,"fmod"},{234,"log"},{235,"log2"},{236,"log10"},
                {237,"round"},{238,"frexp"},{239,"sin"},{240,"sinh"},{241,"cos"},
                {242,"cosh"},{243,"tan"},{244,"tanh"},{245,"atol"},{246,"atof"},
                {247,"strspn"},{248,"strtof"},{249,"ioctl"},{250,"getpeername"},
                {251,"listen"},{252,"accept"},{253,"epoll_create"},{254,"FD_CLR"},
                {255,"expm1"},{256,"if_indextoname"},{257,"sigaddset"},{258,"pthread_sigmask"},
                {259,"fgets"},{260,"setjmp"},{261,"longjmp"},{262,"pthread_condattr_init"},
                {263,"pthread_condattr_setclock"},{264,"pthread_condattr_destroy"},
                {265,"sched_get_priority_min"},{266,"pthread_setschedparam"},
                {267,"getgid"},{268,"getegid"},{269,"random"},{270,"sigfillset"},
                {271,"fdopen"},{272,"timerfd_create"},{273,"timerfd_settime"},
                {274,"fputc"},{275,"bsearch"},{276,"vfprintf"},{277,"pthread_exit"},
                {278,"finitef"},{279,"cbrt"},{280,"remquof"},{281,"strcspn"},
                {282,"gmtime_r"},{283,"difftime"},{284,"strpbrk"},{285,"shutdown"},
                {286,"memrchr"},{287,"accept4"},{288,"if_nametoindex"},{289,"setvbuf"},
                {290,"realpath"},{291,"recvmmsg"},{292,"getcwd"},{293,"pread"},
                {294,"pwrite"},{295,"fchmod"},{296,"fchown"},{297,"mremap"},
                {298,"fsync"},{299,"utimes"},{300,"msync"},{301,"statvfs"},
                {302,"mallinfo"},{303,"__readlink_chk"},{304,"__gnu_strerror_r"},
                {305,"pthread_getschedparam"},{306,"sinf"},{307,"sincosf"},{308,"exp2"},
                {309,"sincos"},{310,"fmal"},{311,"exp2f"},{312,"log10f"},{313,"logf"},
                {314,"powf"},{315,"fmodf"},{316,"log2f"},{317,"expf"},{318,"powl"},
                {319,"cosf"},{320,"pthread_key_delete"},{321,"sysinfo"},{322,"madvise"},
                {323,"pthread_setname_np"},{324,"pthread_getattr_np"},{325,"pthread_attr_getstack"},
                {326,"mlock"},{327,"dl_iterate_phdr"},{328,"isspace"},{329,"gethostbyname"},
                {330,"strcat"},{331,"sendmmsg"},{332,"tolower"},{333,"pthread_rwlock_destroy"},
                {334,"pthread_rwlock_init"},{335,"pthread_rwlock_rdlock"},{336,"pthread_rwlock_unlock"},
                {337,"pthread_rwlock_wrlock"},{338,"signal"},{339,"tcgetattr"},{340,"tcsetattr"},
                {341,"utime"},{342,"vasprintf"},{343,"openlog"},{344,"syslog"},{345,"closelog"},
                {346,"ungetc"},{347,"getc"},{348,"ungetwc"},{349,"getwc"},{350,"fputwc"},
                {351,"newlocale"},{352,"uselocale"},{353,"vsscanf"},{354,"strftime_l"},
                {355,"mbsrtowcs"},{356,"freelocale"},{357,"strcoll_l"},{358,"strxfrm_l"},
            };
            int resolved = 0, failed = 0;
            for (int i = 0; i < (int)(sizeof(pre)/sizeof(pre[0])); i++) {
                void *fn = dlsym(RTLD_DEFAULT, pre[i].name);
                if (fn) { tramp[pre[i].idx] = fn; resolved++; }
                else { failed++; }
            }
            fprintf(stderr, "[jni_shim] pre-resolved %d/%d trampolines (%d failed)\n",
                    resolved, resolved+failed, failed);

            // Fill ALL remaining trampoline entries (785 total, 8 bytes each = 6280)
            // with a safe no-op function to prevent __bf_c_resolve from calling
            // dlsym(RTLD_NEXT, ...) which triggers expensive _dl_mcount strcmp loops.
            // For libc functions that return int, returning 0 is generally safe.
            // For pointer-returning functions, return a valid mmap'd buffer.
            // Use RTLD_DEFAULT to find a valid function for any remaining name.
            {
                int tramp_count = 6280 / 8;  // 785 entries
                int unfilled = 0;
                for (int i = 0; i < tramp_count; i++) {
                    if (tramp[i] == NULL) unfilled++;
                }
                if (unfilled > 0) {
                    void *safe_fn = dlsym(RTLD_DEFAULT, "__errno_location");
                    if (!safe_fn) safe_fn = dlsym(RTLD_DEFAULT, "getpid");
                    for (int i = 0; i < tramp_count; i++) {
                        if (tramp[i] == NULL) tramp[i] = safe_fn;
                    }
                    fprintf(stderr, "[jni_shim] filled %d remaining trampolines with safe default\n", unfilled);
                }
            }

            // Disable ALL of _dl_mcount's code to stop the O(n) strcmp loop that
            // dominates execution under QEMU JIT. The strcmp at ld-linux .text
            // offset 0x1d758 is still active even after noping _dl_mcount's entry
            // (offset 0x15f24 in .text). We need to find and fill the ENTIRE
            // ld-linux .text region from _dl_mcount's start (0x16aa4 VA) past
            // the strcmp at 0x1e2d8 VA. That's 0x1e2d8 - 0x16aa4 = 0x7834 bytes,
            // so we nop ~32KB to be safe.
            // Instead of patching individual functions, use the .text section info:
            // ld-linux .text is from VA 0xb80 (offset in file) to 0x0b80+0x1f458.
            // The hot code is in the 0x16aa4-0x1e300 range. Nop this entire range.
            {
                Dl_info mcount_info;
                void *mcount = dlsym(RTLD_DEFAULT, "_dl_mcount");
                if (mcount && dladdr(mcount, &mcount_info)) {
                    uintptr_t m_start = (uintptr_t)mcount;
                    // Make pages from m_start to m_start+0x10000 RWX
                    uintptr_t m_page = m_start & ~0xfffULL;
                    mprotect((void*)m_page, 0x10000, PROT_READ|PROT_WRITE|PROT_EXEC);
                    // Fill with 'ret' (0xd65f03c0)
                    uint32_t *mcode = (uint32_t*)m_start;
                    for (int i = 0; i < 0x8000; i++) {  // 128KB of ret
                        mcode[i] = 0xd65f03c0;
                    }
                    __builtin___clear_cache((void*)m_start, (void*)(m_start + 0x20000));
                    fprintf(stderr, "[jni_shim] noped _dl_mcount region at %p (128KB)\n", (void*)m_start);
                } else {
                    fprintf(stderr, "[jni_shim] WARNING: _dl_mcount not found\n");
                }
            }

            // Install mutex sanitization wrappers into the trampoline table.
            // These fix Bionic→glibc pthread_mutex_t ABI mismatches.
            // WARNING: Previous attempt caused QEMU JIT crash on re-entry.
            // The wrappers are now __attribute__((noinline)) to prevent
            // TCG cross-TB linking, and we call the glibc functions via
            // saved function pointers (not through the trampoline table).
            g_real_lock = (mutex_fn)dlsym(RTLD_DEFAULT, "pthread_mutex_lock");
            g_real_unlock = (mutex_fn)dlsym(RTLD_DEFAULT, "pthread_mutex_unlock");
            g_real_mutex_init = (mutex_init_fn)dlsym(RTLD_DEFAULT, "pthread_mutex_init");
            g_real_cond_wait = (cond_wait_fn)dlsym(RTLD_DEFAULT, "pthread_cond_wait");
            g_real_cond_timedwait = (cond_timedwait_fn)dlsym(RTLD_DEFAULT, "pthread_cond_timedwait");
            // Revert to direct glibc calls — the wrapper approach causes QEMU
            // JIT crash (SIGSEGV) because the wrapper's blr to a host-side
            // function pointer triggers a TCG goto_tb issue.
            // For now, the pre-resolved trampoline entries already point to
            // glibc's functions directly, so mutex operations work at the
            // glibc level. If the Bionic/glibc ABI mismatch causes deadlocks,
            // we need to pre-sanitize mutex memory before JNI_OnLoad runs.
            if (g_real_lock && g_real_mutex_init) {
                fprintf(stderr, "[jni_shim] direct glibc mutex (lock=%p init=%p)\n",
                        (void*)g_real_lock, (void*)g_real_mutex_init);
            }

            // Override pthread_cond_wait (index 39) and pthread_cond_timedwait
            // (index 96) with a raw ARM shim that returns 0 immediately.
            // This prevents all condvar deadlocks (init guards, etc.) under QEMU
            // where no other thread exists to signal the condvar.
            // mov w0, #0 = 0x52800000, ret = 0xd65f03c0
            void *cond_shim = mmap(NULL, 4096, PROT_READ|PROT_WRITE|PROT_EXEC,
                                   MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
            if (cond_shim != MAP_FAILED) {
                ((uint32_t*)cond_shim)[0] = 0x52800000;  // mov w0, #0
                ((uint32_t*)cond_shim)[1] = 0xd65f03c0;  // ret
                __builtin___clear_cache(cond_shim, (void*)((uintptr_t)cond_shim + 8));
                tramp[39] = cond_shim;   // pthread_cond_wait
                tramp[96] = cond_shim;   // pthread_cond_timedwait (same effect)
                fprintf(stderr, "[jni_shim] condvar shim %p -> tramp[39,96]\n", cond_shim);
            } else {
                fprintf(stderr, "[jni_shim] WARNING: condvar shim mmap failed\n");
            }
        }
    }

    int (*jni_onload)(JavaVM*, void*) = (int (*)(JavaVM*, void*))dlsym(handle, "JNI_OnLoad");
    g_env.functions = (const void**)jni_table;

    // Direct call — no code copy needed with CF_NO_GOTO_TB QEMU patch.
    // The condvar shim at tramp[39,96] handles the init deadlock.
    // Do NOT set guard=1 — let the init function run naturally.
    // It will check guard==0, call 26c0c7c (mutex+condvar), the condvar
    // shim returns immediately, init completes, writes guard=1 and
    // stores JavaVM* at 0x6a26e48 — properly initializing BSS state.
    // Only pre-init the JavaVM* at 0x6a26e48 so other code paths that
    // check for a valid VM pointer don't crash.
    if (g_libroblox_base) {
        uintptr_t jvm_global = g_libroblox_base + 0x6a26e48;
        *(volatile uintptr_t*)jvm_global = (uintptr_t)&g_vm;
        fprintf(stderr, "[jni_shim] pre-init JavaVM at 0x%lx\n",
                (unsigned long)jvm_global);
    }

    fprintf(stderr, "[jni_shim] entering JNI_OnLoad...\n");
    fflush(stderr);
    jint ver = jni_onload(&g_vm, NULL);
    fprintf(stderr, "[jni_shim] JNI_OnLoad -> 0x%x\n", ver);
    fflush(stderr);

    fprintf(stderr, "[jni_shim] Entering sleep loop\n");
    while (1) sleep(1);
    return 0;
}