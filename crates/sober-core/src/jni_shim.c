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

#define STUB_LOG(fmt, ...) fprintf(stderr, "[jni] " fmt "\n", ##__VA_ARGS__)

/* Global canary value — persists in BSS, libroblox.so reads this via GOT */
static uintptr_t g_canary = 0x0A0B0C0D0E0F1011ULL;

// ============== All stub functions ==============

static void* stub_voidp(void) { static char buf[64]; return buf; }

static jint   stub_GetVersion(JNIEnv* e) { return JNI_VERSION_1_6; }
static jclass stub_FindClass(JNIEnv* e, const char* n) { STUB_LOG("FindClass: %s", n?n:"NULL"); return (jclass)stub_voidp(); }
static jmethodID stub_GetMethodID(JNIEnv* e, jclass c, const char* n, const char* s) { STUB_LOG("GetMethodID: %s %s", n?n:"NULL", s?s:"NULL"); return (jmethodID)(uintptr_t)0x1001; }
static jmethodID stub_GetStaticMethodID(JNIEnv* e, jclass c, const char* n, const char* s) { STUB_LOG("GetStaticMethodID: %s %s", n?n:"NULL", s?s:"NULL"); return (jmethodID)(uintptr_t)0x2001; }
static jfieldID stub_GetFieldID(JNIEnv* e, jclass c, const char* n, const char* s) { return (jfieldID)(uintptr_t)0x3001; }
static jfieldID stub_GetStaticFieldID(JNIEnv* e, jclass c, const char* n, const char* s) { return (jfieldID)(uintptr_t)0x4001; }
static jstring stub_NewStringUTF(JNIEnv* e, const char* u) { return (jstring)stub_voidp(); }
static const char* stub_GetStringUTFChars(JNIEnv* e, jstring s, jboolean* c) { if(c)*c=1; return ""; }
static void stub_ReleaseStringUTFChars(JNIEnv* e, jstring s, const char* u) {}
static jint stub_RegisterNatives(JNIEnv* e, jclass c, const JNINativeMethod* m, jint n) { STUB_LOG("RegisterNatives: %d methods", n); return JNI_OK; }
static jobject stub_NewGlobalRef(JNIEnv* e, jobject o) { return o; }
static void stub_DeleteGlobalRef(JNIEnv* e, jobject o) {}
static void stub_DeleteLocalRef(JNIEnv* e, jobject o) {}
static jint stub_ThrowNew(JNIEnv* e, jclass c, const char* m) { STUB_LOG("ThrowNew: %s", m?m:"NULL"); return 0; }
static jint stub_GetJavaVM(JNIEnv* e, JavaVM** v) { *v = (JavaVM*)0x1000; return 0; }
static jobject stub_NewObject(JNIEnv* e, jclass c, jmethodID m, ...) { return stub_voidp(); }

// CallMethod stubs
#define CALL_METHOD_STUB(prefix, ret_type, ret_val) \
    static ret_type stub_##prefix##Method(JNIEnv* e, jobject o, jmethodID m, ...) { return ret_val; } \
    static ret_type stub_##prefix##MethodV(JNIEnv* e, jobject o, jmethodID m, va_list a) { return ret_val; } \
    static ret_type stub_##prefix##MethodA(JNIEnv* e, jobject o, jmethodID m, const jvalue* a) { return ret_val; }

CALL_METHOD_STUB(CallObject, jobject, stub_voidp())
CALL_METHOD_STUB(CallBoolean, jboolean, 1)
CALL_METHOD_STUB(CallByte, jint, 0)
CALL_METHOD_STUB(CallChar, jint, 0)
CALL_METHOD_STUB(CallShort, jint, 0)
CALL_METHOD_STUB(CallInt, jint, 0)
CALL_METHOD_STUB(CallLong, jlong, 0)
CALL_METHOD_STUB(CallFloat, jfloat, 0)
CALL_METHOD_STUB(CallDouble, jdouble, 0)
static void stub_CallVoidMethod(JNIEnv* e, jobject o, jmethodID m, ...) {}
static void stub_CallVoidMethodV(JNIEnv* e, jobject o, jmethodID m, va_list a) {}
static void stub_CallVoidMethodA(JNIEnv* e, jobject o, jmethodID m, const jvalue* a) {}

#define CALL_STATIC_METHOD_STUB(prefix, ret_type, ret_val) \
    static ret_type stub_##prefix##Method(JNIEnv* e, jclass c, jmethodID m, ...) { return ret_val; } \
    static ret_type stub_##prefix##MethodV(JNIEnv* e, jclass c, jmethodID m, va_list a) { return ret_val; } \
    static ret_type stub_##prefix##MethodA(JNIEnv* e, jclass c, jmethodID m, const jvalue* a) { return ret_val; }

CALL_STATIC_METHOD_STUB(CallStaticObject, jobject, stub_voidp())
CALL_STATIC_METHOD_STUB(CallStaticBoolean, jboolean, 1)
CALL_STATIC_METHOD_STUB(CallStaticByte, jint, 0)
CALL_STATIC_METHOD_STUB(CallStaticChar, jint, 0)
CALL_STATIC_METHOD_STUB(CallStaticShort, jint, 0)
CALL_STATIC_METHOD_STUB(CallStaticInt, jint, 0)
CALL_STATIC_METHOD_STUB(CallStaticLong, jlong, 0)
CALL_STATIC_METHOD_STUB(CallStaticFloat, jfloat, 0)
CALL_STATIC_METHOD_STUB(CallStaticDouble, jdouble, 0)
static void stub_CallStaticVoidMethod(JNIEnv* e, jclass c, jmethodID m, ...) {}
static void stub_CallStaticVoidMethodV(JNIEnv* e, jclass c, jmethodID m, va_list a) {}
static void stub_CallStaticVoidMethodA(JNIEnv* e, jclass c, jmethodID m, const jvalue* a) {}

// Get/SetField stubs
static jobject stub_GetObjectField(JNIEnv* e, jobject o, jfieldID f) { return stub_voidp(); }
static void stub_SetObjectField(JNIEnv* e, jobject o, jfieldID f, jobject v) {}
static jint stub_GetIntField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static void stub_SetIntField(JNIEnv* e, jobject o, jfieldID f, jint v) {}
static jlong stub_GetLongField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static void stub_SetLongField(JNIEnv* e, jobject o, jfieldID f, jlong v) {}
static jboolean stub_GetBooleanField(JNIEnv* e, jobject o, jfieldID f) { return 0; }
static jfloat stub_GetFloatField(JNIEnv* e, jobject o, jfieldID f) { return 0; }

// Static field stubs
static jobject stub_GetStaticObjectField(JNIEnv* e, jclass c, jfieldID f) { return stub_voidp(); }
static jint stub_GetStaticIntField(JNIEnv* e, jclass c, jfieldID f) { return 0; }
static void stub_SetStaticIntField(JNIEnv* e, jclass c, jfieldID f, jint v) {}
static jlong stub_GetStaticLongField(JNIEnv* e, jclass c, jfieldID f) { return 0; }

// Array stubs
static jint stub_GetArrayLength(JNIEnv* e, jarray a) { return 0; }
static jobject stub_GetObjectArrayElement(JNIEnv* e, jobjectArray a, jint i) { return stub_voidp(); }
static void stub_SetObjectArrayElement(JNIEnv* e, jobjectArray a, jint i, jobject v) {}
static jobjectArray stub_NewObjectArray(JNIEnv* e, jint l, jclass c, jobject i) { return (jobjectArray)stub_voidp(); }

// Exception stubs
static jthrowable stub_ExceptionOccurred(JNIEnv* e) { return NULL; }
static void stub_ExceptionDescribe(JNIEnv* e) {}
static void stub_ExceptionClear(JNIEnv* e) {}
static jint stub_Throw(JNIEnv* e, jthrowable o) { return 0; }

// ============== JNI function table (opaque, indexed by slot) ==============

#define JNI_SLOTS 256
static void* jni_table[JNI_SLOTS];

struct JavaVM_ {
    const void** functions;
    void* reserved[4];
};

struct JNIEnv_ {
    const void** functions;
    void* reserved[4];
};

#define VM_SLOTS 16
static void* vm_table[VM_SLOTS];

static JNIEnv g_env;
static JavaVM g_vm;

static jint stub_GetEnv_Attach(JavaVM* vm, void** penv, void* args) {
    (void)args;
    if (!g_env.functions) g_env.functions = (const void**)jni_table;
    *penv = &g_env;
    return 0;
}

static jint stub_GetEnv_jint(JavaVM* vm, void** penv, jint version) {
    (void)version;
    if (!g_env.functions) g_env.functions = (const void**)jni_table;
    *penv = &g_env;
    return 0;
}

static jint stub_DestroyJavaVM(JavaVM* vm) { return 0; }
static jint stub_DetachCurrentThread(JavaVM* vm) { return 0; }

__attribute__((constructor))
static void init_jni_functions() {
    memset(jni_table, 0, sizeof(jni_table));

    jni_table[4] = stub_GetVersion;
    jni_table[5] = stub_FindClass;
    jni_table[6] = stub_FindClass;
    jni_table[7] = stub_GetMethodID;
    jni_table[8] = stub_GetFieldID;
    jni_table[9] = stub_voidp;
    jni_table[10] = stub_FindClass;
    jni_table[11] = (void *)stub_GetVersion;
    jni_table[12] = stub_voidp;
    jni_table[13] = stub_Throw;
    jni_table[14] = stub_ThrowNew;
    jni_table[15] = stub_ExceptionOccurred;
    jni_table[16] = stub_ExceptionDescribe;
    jni_table[17] = stub_ExceptionClear;
    jni_table[18] = (void *)abort;
    jni_table[19] = (void *)stub_GetVersion;
    jni_table[20] = stub_voidp;
    jni_table[21] = stub_NewGlobalRef;
    jni_table[22] = stub_DeleteGlobalRef;
    jni_table[23] = stub_DeleteLocalRef;
    jni_table[24] = (void *)stub_GetVersion;
    jni_table[25] = stub_NewGlobalRef;
    jni_table[26] = (void *)stub_GetVersion;
    jni_table[27] = stub_NewObject;
    jni_table[28] = stub_NewObject;
    jni_table[29] = stub_NewObject;
    jni_table[30] = stub_NewObject;
    jni_table[31] = stub_FindClass;
    jni_table[32] = (void *)stub_GetVersion;
    jni_table[33] = stub_GetMethodID;
    jni_table[34] = stub_GetStringUTFChars;
    jni_table[35] = stub_ReleaseStringUTFChars;
    jni_table[36] = stub_NewStringUTF;
    jni_table[37] = stub_GetArrayLength;
    jni_table[38] = stub_NewObjectArray;
    jni_table[39] = stub_GetObjectArrayElement;
    jni_table[40] = stub_SetObjectArrayElement;

    for (int i = 41; i <= 100; i++) {
        void *stubs[] = {
            stub_CallObjectMethod, stub_CallObjectMethodV, stub_CallObjectMethodA,
            stub_CallBooleanMethod, stub_CallBooleanMethodV, stub_CallBooleanMethodA,
            stub_CallByteMethod, stub_CallByteMethodV, stub_CallByteMethodA,
            stub_CallCharMethod, stub_CallCharMethodV, stub_CallCharMethodA,
            stub_CallShortMethod, stub_CallShortMethodV, stub_CallShortMethodA,
            stub_CallIntMethod, stub_CallIntMethodV, stub_CallIntMethodA,
            stub_CallLongMethod, stub_CallLongMethodV, stub_CallLongMethodA,
            stub_CallFloatMethod, stub_CallFloatMethodV, stub_CallFloatMethodA,
            stub_CallDoubleMethod, stub_CallDoubleMethodV, stub_CallDoubleMethodA,
            stub_CallVoidMethod, stub_CallVoidMethodV, stub_CallVoidMethodA,
            stub_CallStaticObjectMethod, stub_CallStaticObjectMethodV, stub_CallStaticObjectMethodA,
            stub_CallStaticBooleanMethod, stub_CallStaticBooleanMethodV, stub_CallStaticBooleanMethodA,
            stub_CallStaticByteMethod, stub_CallStaticByteMethodV, stub_CallStaticByteMethodA,
            stub_CallStaticCharMethod, stub_CallStaticCharMethodV, stub_CallStaticCharMethodA,
            stub_CallStaticShortMethod, stub_CallStaticShortMethodV, stub_CallStaticShortMethodA,
            stub_CallStaticIntMethod, stub_CallStaticIntMethodV, stub_CallStaticIntMethodA,
            stub_CallStaticLongMethod, stub_CallStaticLongMethodV, stub_CallStaticLongMethodA,
            stub_CallStaticFloatMethod, stub_CallStaticFloatMethodV, stub_CallStaticFloatMethodA,
            stub_CallStaticDoubleMethod, stub_CallStaticDoubleMethodV, stub_CallStaticDoubleMethodA,
            stub_CallVoidMethod, stub_CallVoidMethodV, stub_CallVoidMethodA,
        };
        if ((unsigned)(i - 41) < sizeof(stubs)/sizeof(stubs[0]))
            jni_table[i] = stubs[i - 41];
    }

    jni_table[101] = stub_GetFieldID;
    jni_table[102] = stub_GetObjectField; jni_table[103] = stub_SetObjectField;
    jni_table[104] = stub_GetBooleanField; jni_table[105] = stub_SetIntField;
    jni_table[106] = stub_GetBooleanField; jni_table[107] = stub_SetIntField;
    jni_table[108] = stub_GetIntField; jni_table[109] = stub_SetIntField;
    jni_table[110] = stub_GetIntField; jni_table[111] = stub_SetIntField;
    jni_table[112] = stub_GetIntField; jni_table[113] = stub_SetIntField;
    jni_table[114] = stub_GetLongField; jni_table[115] = stub_SetLongField;
    jni_table[116] = stub_GetFloatField; jni_table[117] = stub_GetFloatField;
    jni_table[118] = stub_GetFloatField; jni_table[119] = stub_GetFloatField;
    jni_table[120] = stub_GetStaticFieldID;

    vm_table[3] = stub_DestroyJavaVM; vm_table[4] = stub_GetEnv_Attach;
    vm_table[5] = stub_DetachCurrentThread; vm_table[6] = stub_GetEnv_jint;
    vm_table[7] = stub_GetEnv_Attach;
    g_vm.functions = (const void**)vm_table;

    for (int i = 0; i < JNI_SLOTS; i++) if (jni_table[i] == NULL) jni_table[i] = (void *)stub_GetVersion;
}

// ============== SIGSEGV handler for QEMU JIT workaround ==============

static struct sigaction jni_old_sa;
static int jni_segv_count = 0;

/* SIGSEGV handler that handles writes to read-only pages.
 * GSI libraries try to write to pages that glibc has made read-only (RELRO).
 * Under QEMU, JIT-cached translations of write instructions to these pages
 * fault repeatedly because QEMU doesn't invalidate the JIT cache on mprotect.
 *
 * Strategy:
 *   1. Self-referencing write (pc == fault_addr): instruction tries to write
 *      to its own code page. Skip past it (advance PC by 4).
 *   2. Other faults within mapped address space: mprotect to RWX and retry.
 *   3. NULL deref: chain to old handler. */
static void jni_segv_handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t *u = (ucontext_t*)ctx;
    uintptr_t fault_addr = (uintptr_t)info->si_addr;
    uintptr_t fault_page = fault_addr & ~0xfffULL;
    uintptr_t pc = u->uc_mcontext.pc;

    jni_segv_count++;

    // Limit to prevent infinite loops
    if (jni_segv_count > 500) {
        fprintf(stderr, "[jni_segv] #%d: too many faults, aborting\n", jni_segv_count);
        sigaction(SIGSEGV, &jni_old_sa, NULL);
        return;
    }

    // Invalid address (0 or -1 or similar) — abort
    if (fault_addr < 0x1000 || fault_addr == (uintptr_t)-1 || fault_addr == ~0xfffULL) {
        fprintf(stderr, "[jni_segv] #%d: bad addr=0x%lx at pc=0x%lx — aborting\n",
                jni_segv_count, fault_addr, pc);
        sigaction(SIGSEGV, &jni_old_sa, NULL);
        return;
    }

    // Self-referencing write: instruction is writing to its own page.
    // Toggle page permissions to force QEMU to flush its JIT cache
    // (PROT_NONE forces QEMU to drop cached TB for this page),
    // then restore to RWX and retry the SAME instruction.
    if (pc == fault_addr) {
        mprotect((void*)fault_page, 0x1000, PROT_NONE);
        // Write to the page to force QEMU to re-fault (triggers TB recreation)
        *(volatile int*)fault_page = 0;
        mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE|PROT_EXEC);
        if (jni_segv_count <= 5)
            fprintf(stderr, "[jni_segv] #%d: self-write at pc=0x%lx, toggle+retry\n",
                    jni_segv_count, pc);
        // Return to the SAME instruction — QEMU should re-translate with correct perms
        return;
    }

    // Try to make the faulting page writable, then advance 4 bytes
    // to work around QEMU JIT cache invalidation issues.
    if (mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE) == 0) {
        if (jni_segv_count <= 5)
            fprintf(stderr, "[jni_segv] #%d: mprotect(RW) 0x%lx OK, advancing pc 0x%lx->0x%lx\n",
                    jni_segv_count, fault_page, pc, pc + 4);
        u->uc_mcontext.pc = pc + 4;
        return;
    }

    // mprotect failed — page might be unmapped. Try mmap.
    fprintf(stderr, "[jni_segv] #%d: mprotect failed for 0x%lx (errno=%d), trying mmap\n",
            jni_segv_count, fault_page, errno);
    void *map = mmap((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE,
                     MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED, -1, 0);
    if (map != MAP_FAILED) {
        fprintf(stderr, "[jni_segv] #%d: mmap(FIXED) 0x%lx OK\n", jni_segv_count, fault_page);
        return;
    }

    fprintf(stderr, "[jni_segv] #%d: unhandled pc=0x%lx fault=0x%lx\n", jni_segv_count, pc, fault_addr);
    sigaction(SIGSEGV, &jni_old_sa, NULL);
}

// ============== Pre-mprotect RELRO pages using /proc/self/maps ==============

/* Read /proc/self/maps and mprotect any read-only data pages
 * (r--p) belonging to GSI libraries back to read-write.
 * This prevents write faults during GSI library initialization
 * (BSS init, mutex init, etc.) under QEMU. */
static void pre_mprotect_relro(void) {
    FILE *maps = fopen("/proc/self/maps", "r");
    if (!maps) {
        fprintf(stderr, "[mprotect] WARNING: cannot open /proc/self/maps\n");
        return;
    }

    char line[512];
    int count = 0;
    while (fgets(line, sizeof(line), maps)) {
        uintptr_t start, end;
        char perms[8];
        char path[256] = "";
        int n = sscanf(line, "%lx-%lx %7s %*x %*x:%*x %*d %255s", &start, &end, perms, path);

        // Look for read-only private pages (r--p) from GSI libraries
        if (n >= 3 && strcmp(perms, "r--p") == 0) {
            // Only touch GSI libs (not glibc, not vdso, not stack)
            if (path[0] && !strstr(path, "ld-linux") &&
                !strstr(path, "libc.so") && !strstr(path, "libm.so") &&
                !strstr(path, "libdl.so") && !strstr(path, "libpthread.so") &&
                !strstr(path, "bionic_shim") && !strstr(path, "[vdso]")) {
                mprotect((void*)start, end - start, PROT_READ | PROT_WRITE);
                count++;
            }
        }
    }
    fclose(maps);
    if (count > 0)
        fprintf(stderr, "[mprotect] Re-writable %d RELRO pages\n", count);
}

// ============== Main ==============
int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    fprintf(stderr, "[jni_shim] Loading bionic shim...\n");
    void *bionic_shim = dlopen("libbionic_shim.so", RTLD_LAZY | RTLD_GLOBAL);
    if (!bionic_shim) {
        fprintf(stderr, "[jni_shim] WARNING: libbionic_shim.so not found: %s\n", dlerror());
    }

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);
    void* handle = dlopen(lib_path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "[jni_shim] Failed: %s\n", dlerror());
        return 1;
    }
    fprintf(stderr, "[jni_shim] Loaded successfully\n");

    // Fix __stack_chk_guard GOT entry
    Dl_info dl_info;
    if (dladdr((void*)dlsym(handle, "JNI_OnLoad"), &dl_info)) {
        uintptr_t base = (uintptr_t)dl_info.dli_fbase;
        uintptr_t guard_ptr_addr = base + 0x6473438;
        uintptr_t* guard_ptr = (uintptr_t*)guard_ptr_addr;

        uintptr_t got_start = base + 0x5fbd000;
        uintptr_t got_end   = base + 0x6475000;
        mprotect((void*)got_start, got_end - got_start, PROT_READ | PROT_WRITE);

        fprintf(stderr, "[jni_shim] libroblox base=%p, guard_ptr at %p = %p\n",
                (void*)base, (void*)guard_ptr, (void*)*guard_ptr);
        if (*guard_ptr == 0) {
            *guard_ptr = (uintptr_t)&g_canary;
        }

        uintptr_t* libc_guard = (uintptr_t*)dlsym(RTLD_NEXT, "__stack_chk_guard");
        if (libc_guard && *libc_guard == 0) {
            *libc_guard = g_canary;
        }
    } else {
        fprintf(stderr, "[jni_shim] WARNING: dladdr(JNI_OnLoad) failed\n");
    }

    // Install SIGSEGV handler
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = jni_segv_handler;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, &jni_old_sa);
    fprintf(stderr, "[jni_shim] SIGSEGV handler installed\n");

    // Pre-mprotect RELRO pages to prevent write faults
    pre_mprotect_relro();

    // Install mutex sanitization wrappers into bionic shim's trampoline table.
    // This calls __bf_c_resolve which uses dlsym(RTLD_NEXT) to find the real
    // functions, then installs sanitize_mutex wrappers over the trampoline
    // entries at indices 38 (pthread_mutex_lock), 124 (pthread_mutex_trylock),
    // and 780 (pthread_mutex_timedlock).
    if (bionic_shim) {
        void (*install_wrappers)(void) = dlsym(bionic_shim, "__bf_install_mutex_wrappers");
        if (install_wrappers) {
            install_wrappers();
            fprintf(stderr, "[jni_shim] mutex wrappers installed\n");
        }
    }

    // Call JNI_OnLoad
    typedef int (*jni_onload_t)(JavaVM*, void*);
    jni_onload_t jni_onload = (jni_onload_t)dlsym(handle, "JNI_OnLoad");
    g_env.functions = (const void**)jni_table;

    fprintf(stderr, "[jni_shim] JNI_OnLoad at %p, calling...\n", (void*)jni_onload);
    fflush(stderr);

    jint ver = jni_onload(&g_vm, NULL);
    fprintf(stderr, "[jni_shim] JNI_OnLoad -> 0x%x\n", ver);

    fprintf(stderr, "[jni_shim] Entering sleep loop\n");
    while (1) sleep(1);
    return 0;
}