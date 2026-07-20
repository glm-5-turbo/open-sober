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
            static const char * const tramp_names[] = {
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
        "strlen",
        "memchr",
        "strncmp",
        "strcmp",
        "getauxval",
        "__errno_location",
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
        "strchr",
        "clock",
        "fileno",
        "remainderf",
        "nan",
        "qsort",
        "nextafterf",
        "acos",
        "asin",
        "ilogb",
        "FD_SET",
        "select",
        "FD_ISSET",
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
        "FD_CLR",
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
        "__cfi_slowpath",
        "android_fdsan_close_with_tag",
        "android_fdsan_create_owner_tag",
        "android_fdsan_exchange_owner_tag",
        "free",
        "malloc",
        "dup",
        "android_mallopt",
        "__system_property_find",
        "__system_property_read_callback",
        "getprogname",
        "sleep",
        "android_dlopen_ext",
        "nrand48",
        "remove",
        "utimensat",
        "futimens",
        "getpwuid",
        "getgrgid",
        "aligned_alloc",
        "_Unwind_RaiseException",
        "_Unwind_Resume",
        "_Unwind_SetGR",
        "_Unwind_SetIP",
        "_Unwind_GetLanguageSpecificData",
        "_Unwind_GetIP",
        "_Unwind_GetRegionStart",
        "realloc",
        "posix_memalign",
        "pthread_cond_clockwait",
        "asinhf",
        "acoshf",
        "atanhf",
        "calloc",
        "memalign",
        "unsetenv",
        "setenv",
        "_Unwind_DeleteException",
        "setbuf",
        "strtof_l",
        "strtod_l",
        "strdup",
        "fabs",
        "ceil",
        "floor",
        "sqrt",
        "sigprocmask",
        "__pread_chk",
        "statfs",
        "symlink",
        "strsep",
        "setrlimit",
        "dup2",
        "isatty",
        "getrusage",
        "alarm",
        "kill",
        "wait",
        "strsignal",
        "getrlimit",
        "__register_frame",
        "__deregister_frame",
        "__strlcpy_chk",
        "arc4random",
        "__strrchr_chk",
        "__strncat_chk",
        "tgammaf",
        "expm1f",
        "fdimf",
        "hypotf",
        "ilogbf",
        "lgammaf",
        "lgammaf_r",
        "log1pf",
        "logbf",
        "strndup",
        "strlcpy",
        "setpgid",
        "__strlcat_chk",
        "clock_nanosleep",
        "_Unwind_GetCFA",
        "_Unwind_FindEnclosingFunction",
        "_Unwind_Backtrace",
        "asprintf",
        "__system_property_area_serial",
        "__system_property_serial",
        "__system_property_read",
        "fnmatch",
        "timerfd_gettime",
        "__openat_2",
        "android_getaddrinfofornet",
        "__system_property_set",
        "setprogname",
        "fallocate",
        "lstat64",
        "mkstemp",
        "chmod",
        "statfs64",
        "android_dlwarning",
        "mallopt",
        "malloc_info",
        "fdatasync",
        "android_get_application_target_sdk_version",
        "getpwnam",
        "getgrnam",
        "__sched_cpucount",
        "sched_getaffinity",
        "get_nprocs_conf",
        "setuid",
        "setgid",
        "tgkill",
        "getline",
        "strtok_r",
        "pidfd_open",
        "tcflush",
        "tcsendbreak",
        "inotify_init1",
        "inotify_add_watch",
        "inotify_rm_watch",
        "android_fdsan_get_error_level",
        "android_reset_stack_guards",
        "android_fdsan_set_error_level",
        "dup3",
        "capget",
        "setresgid",
        "setresuid",
        "android_set_16kb_appcompat_mode",
        "capset",
        "mount",
        "unshare",
        "__system_properties_zygote_reload",
        "setgroups",
        "setmntent",
        "getmntent",
        "endmntent",
        "umount2",
        "statx",
        "memset_explicit",
        "lseek64",
        "dirfd",
        "malloc_usable_size",
        "fstatfs",
        "_Unwind_GetTextRelBase",
        "_Unwind_GetDataRelBase",
        "_Unwind_GetIPInfo",
        "settimeofday",
        "sigqueue",
        "eventfd_write",
        "process_madvise",
        "inotify_init",
        "flock",
        "execvp",
        "putchar",
        "dprintf",
        "memfd_create",
        "android_get_exported_namespace",
        "readv",
        "mkdtemp",
        "pthread_gettid_np",
        "isnanf",
        "sched_setaffinity",
        "pthread_getname_np",
        "nftw",
        "__pwrite_chk",
        "basename",
        "__system_property_wait",
        "inet_ntoa",
        "pipe2",
        "hypot",
        "faccessat",
        "fmemopen",
        "dirname",
        "memccpy",
        "tsearch",
        "tfind",
        "tdelete",
        "tdestroy",
        "android_get_device_api_level",
        "chown",
        "perror",
        "clone",
        "getpwnam_r",
        "getgrnam_r",
        "signalfd",
        "sigdelset",
        "sigismember",
        "mkdirat",
        "openat",
        "waitid",
        "strtold",
        "wcstol",
        "wcstoul",
        "wcstoll",
        "wcstoull",
        "wcstof",
        "wcstod",
        "wcstold",
        "swprintf",
        "setlocale",
        "link",
        "pathconf",
        "chdir",
        "fchmodat",
        "truncate",
        "sendfile",
        "fdopendir",
        "unlinkat",
        "fgetxattr",
        "getxattr",
        "fremovexattr",
        "fsetxattr",
        "removexattr",
        "setxattr",
        "chroot",
        "__pwrite64_chk",
        "fstat64",
        "creat",
        "stat64",
        "iswspace",
        "wcschr",
        "timegm",
        "personality",
        "ppoll",
        "logb",
        "scalbn",
        "scalbnf",
        "logbl",
        "scalbnl",
        "fmaxl",
        "putc",
        "rewinddir",
        "lockf",
        "__umask_chk",
        "getservbyname",
        "regcomp",
        "regerror",
        "regexec",
        "regfree",
        "__fwrite_chk",
        "getifaddrs",
        "freeifaddrs",
        "getpwuid_r",
        "android_fdsan_get_owner_tag",
        "__system_property_foreach",
        "strtoimax",
        "fstatat",
        "readdir_r",
        "mknod",
        "open_memstream",
        "vfork",
        "getprotobynumber",
        "__fgets_chk",
        "strtok",
        "ctime",
        "scandir",
        "alphasort",
        "rewind",
        "hasmntopt",
        "__pread64_chk",
        "posix_fadvise",
        "ftruncate64",
        "process_vm_readv",
        "umount",
        "swapon",
        "system",
        "fstatfs64",
        "sync",
        "posix_openpt",
        "grantpt",
        "unlockpt",
        "ptsname_r",
        "setsid",
        "android_link_namespaces",
        "android_create_namespace",
        "eventfd_read",
        "sem_clockwait",
        "asinh",
        "acosh",
        "atanh",
        "remainder",
        "getrandom",
        "erf",
        "asctime",
        "getprotobyname",
        "setns",
        "__gnu_basename",
        "fgetc",
        "glob",
        "globfree",
        "getgroups",
        "getopt_long_only",
        "inet_aton",
        "strcasestr",
        "inet_addr",
        "pthread_setschedprio",
        "pwrite64",
        "mmap64",
        "munlock",
        "malloc_backtrace",
        "malloc_disable",
        "malloc_enable",
        "malloc_iterate",
        "sethostname",
        "mknodat",
        "symlinkat",
        "fchdir",
        "initgroups",
        "sigtimedwait",
        "__libc_current_sigrtmax",
        "fexecve",
        "__libc_current_sigrtmin",
        "prlimit",
        "vsyslog",
        "vdprintf",
        "posix_madvise",
        "android_set_application_target_sdk_version",
        "res_mkquery",
        "__b64_ntop",
        "vfscanf",
        "clock_getcpuclockid",
        "clock_getres",
        "clock_settime",
        "getgrgid_r",
        "sigwait",
        "sigsuspend",
        "getgrouplist",
        "daemon",
        "setfsgid",
        "setfsuid",
        "process_vm_writev",
        "munlockall",
        "mlockall",
        "umask",
        "recv",
        "send",
        "cfsetspeed",
        "cfgetispeed",
        "cfgetospeed",
        "cfsetispeed",
        "cfsetospeed",
        "cfmakeraw",
        "getnetbyname",
        "iswdigit",
        "wait4",
        "getpgid",
        "lchown",
        "nanf",
        "strptime",
        "sem_timedwait",
        "frexpl",
        "ldexpl",
        "__recvfrom_chk",
        "readdir64",
        "readlinkat",
        "flistxattr",
        "llistxattr",
        "lremovexattr",
        "linkat",
        "openat64",
        "fstatat64",
        "fchownat",
        "posix_fadvise64",
        "fstatvfs64",
        "renameat",
        "fallocate64",
        "lgetxattr",
        "listxattr",
        "lsetxattr",
        "pause",
        "__stpcpy_chk",
        "__fsetlocking",
        "__mempcpy_chk",
        "reallocarray",
        "__system_properties_init",
        "fts_open",
        "fts_read",
        "fts_set",
        "fts_close",
        "stpcpy",
        "popen",
        "pclose",
        "pwritev",
        "fpathconf",
        "memmem",
        "strchrnul",
        "open64",
        "mkfifo",
        "preadv",
        "splice",
        "copy_file_range",
        "fseeko64",
        "ftello64",
        "sem_trywait",
        "pthread_mutex_timedlock",
        "strerrorname_np",
        "clone3",
        "setregid",
        "setreuid",
    };
            int resolved = 0, failed = 0;
            for (int i = 0; i < 785; i++) {
                void *fn = dlsym(RTLD_DEFAULT, tramp_names[i]);
                if (fn) { tramp[i] = fn; resolved++; }
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

            // Disable _dl_mcount profiling by zeroing rtld_global's dl_profile field.
            // QEMU user-mode loads the guest ld-linux, which has profiling enabled
            // by default (field at rtld_global+0xCA0 = 0x40). Setting it to 0
            // makes _dl_mcount return early without the expensive strcmp loop.
            // Use mprotect to force QEMU JIT cache invalidation on the data page.
            {
                Dl_info mcount_info;
                void *mcount = dlsym(RTLD_DEFAULT, "_dl_mcount");
                if (mcount && dladdr(mcount, &mcount_info)) {
                    uintptr_t m_start = (uintptr_t)mcount;
                    // Get the adrp target: _dl_mcount code:
                    //   d000015a  adrp x26, 40000   (x26 = _rtld_global)
                    //   b94ca343  ldr  w3, [x26, #3232]  (w3 = dl_profile)
                    // rtld_global + 0xCA0 is the dl_profile field
                    uint32_t *code = (uint32_t*)m_start;
                    if ((code[0] & 0x9f00001f) == 0x90000000) {
                        // Decode adrp: immhi = bits 23:5 of instruction, immlo = bits 30:29
                        uint64_t adrp_hi = (code[0] >> 5) & 0x7ffff;
                        uint64_t adrp_lo = (code[0] >> 29) & 0x3;
                        int64_t page_off = (adrp_hi << 14) | (adrp_lo << 12);
                        uint64_t pc_page = m_start & ~0xfffULL;
                        uint64_t rtld_global_addr = pc_page + page_off;
                        // ldr w3, [x26, #3232] at code[1]
                        uint32_t ldr_val = code[1];
                        uint32_t ldr_off = (ldr_val >> 5) & 0xfff;  // scaled by 4
                        if ((ldr_val & 0xffc00000) == 0xb9400000 && ldr_off == 0xCA0/4) {
                            uintptr_t profile_field = rtld_global_addr + 0xCA0;
                            uintptr_t pf_page = profile_field & ~0xfffULL;
                            mprotect((void*)pf_page, 0x10000, PROT_READ|PROT_WRITE);
                            *(volatile uint32_t*)profile_field = 0;
                            mprotect((void*)pf_page, 0x10000, PROT_READ);
                            fprintf(stderr, "[jni_shim] zeroed dl_profile at 0x%lx\n",
                                    (unsigned long)profile_field);
                        }
                    }
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