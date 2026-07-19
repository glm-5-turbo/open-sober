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

/* Global canary value */
static uintptr_t g_canary = 0x0A0B0C0D0E0F1011ULL;

// ============== Stub functions ==============

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

typedef int (*mutex_lock_fn)(void*);
typedef int (*mutex_timedlock_fn)(void*, const void*);

static mutex_lock_fn g_real_lock = NULL;
static mutex_lock_fn g_real_trylock = NULL;
static mutex_timedlock_fn g_real_timedlock = NULL;

static int sanitize_and_lock(void *mutex) {
    if (mutex) {
        *(volatile int*)((char*)mutex + 8) = 0;    // clear __owner
        int k = *(volatile int*)((char*)mutex + 16); // sanitize __kind
        *(volatile int*)((char*)mutex + 16) = k & 3;
    }
    return g_real_lock(mutex);
}
static int sanitize_and_trylock(void *mutex) {
    if (mutex) {
        *(volatile int*)((char*)mutex + 8) = 0;
        int k = *(volatile int*)((char*)mutex + 16);
        *(volatile int*)((char*)mutex + 16) = k & 3;
    }
    return g_real_trylock(mutex);
}
static int sanitize_and_timedlock(void *mutex, const void *abstime) {
    if (mutex) {
        *(volatile int*)((char*)mutex + 8) = 0;
        int k = *(volatile int*)((char*)mutex + 16);
        *(volatile int*)((char*)mutex + 16) = k & 3;
    }
    return g_real_timedlock(mutex, abstime);
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
    if (mprotect((void*)fault_page, 0x1000, PROT_READ|PROT_WRITE) == 0) {
        u->uc_mcontext.pc = pc + 4;
        if (jni_segv_count <= 5)
            fprintf(stderr, "[jni_segv] #%d: mprotect 0x%lx advance\n", jni_segv_count, fault_page);
        return;
    }

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
                mprotect((void*)s, e - s, PROT_READ|PROT_WRITE); count++;
            }
        }
    }
    fclose(maps);
    if (count) fprintf(stderr, "[mprotect] %d RELRO pages -> RW\n", count);
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
            // Pre-resolve key functions to bypass lazy resolver
            struct { int idx; const char *name; } pre[] = {
                {0,"__cxa_finalize"},{1,"__cxa_atexit"},{3,"strlen"},{4,"memcmp"},
                {5,"pthread_mutex_init"},{6,"pthread_mutex_destroy"},{7,"pthread_once"},
                {8,"__memset_chk"},{9,"__memcpy_chk"},{10,"strlen"},{11,"memchr"},
                {12,"strncmp"},{13,"strcmp"},{14,"getauxval"},{15,"__errno_location"},
                {16,"close"},{40,"pthread_mutex_unlock"},
            };
            for (int i = 0; i < (int)(sizeof(pre)/sizeof(pre[0])); i++) {
                void *fn = dlsym(RTLD_DEFAULT, pre[i].name);
                if (fn) tramp[pre[i].idx] = fn;
            }

            // Resolve real mutex functions and install sanitize wrappers
            g_real_lock = dlsym(RTLD_DEFAULT, "pthread_mutex_lock");
            g_real_trylock = dlsym(RTLD_DEFAULT, "pthread_mutex_trylock");
            g_real_timedlock = dlsym(RTLD_DEFAULT, "pthread_mutex_timedlock");
            if (g_real_lock) {
                tramp[38] = sanitize_and_lock;
                // Also resolve index 0 (which is __cxa_finalize) — already done above
                fprintf(stderr, "[jni_shim] mutex wrapper installed (real=%p)\n", (void*)g_real_lock);
            }
            if (g_real_trylock) tramp[124] = sanitize_and_trylock;
            if (g_real_timedlock) tramp[780] = sanitize_and_timedlock;
        }
    }

    int (*jni_onload)(JavaVM*, void*) = (int (*)(JavaVM*, void*))dlsym(handle, "JNI_OnLoad");
    g_env.functions = (const void**)jni_table;

    // Copy JNI_OnLoad + 128KB to a fresh page to bypass QEMU JIT bug
    // (QEMU host-crashes when JIT-compiling pages near offset 0x1f64000)
    Dl_info onload_info;
    if (dladdr((void*)jni_onload, &onload_info)) {
        uintptr_t base = (uintptr_t)onload_info.dli_fbase;
        uintptr_t jni_page = 0x1f64000;
        size_t copy_sz = 0x20000;
        void *orig = (void*)(base + jni_page);
        void *copy = mmap(NULL, copy_sz, PROT_READ|PROT_WRITE|PROT_EXEC,
                          MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (copy != MAP_FAILED) {
            memcpy(copy, orig, copy_sz);
            // Fix adrp: recalculate immediates for the new code location
            // Target must remain the absolute address (base + original_target_offset)
            uint32_t *code = (uint32_t*)copy;
            for (size_t off = 0; off < copy_sz; off += 4) {
                uint32_t ins = code[off/4];
                if ((ins >> 24) != 0x90) continue;
                int32_t old_imm = (((ins >> 5) & 0x7ffff) << 2) | ((ins >> 29) & 3);
                if (old_imm & 0x100000) old_imm -= 0x200000;
                // target = original_PC_page + (old_imm << 12)
                uintptr_t tgt = (base + jni_page + off) & ~0xfffULL;
                tgt += (int64_t)old_imm << 12;
                // new_imm = (target - copy_PC_page) >> 12
                uintptr_t cpy_page = ((uintptr_t)copy + off) & ~0xfffULL;
                int64_t diff = (int64_t)(tgt - cpy_page);
                int32_t new_imm = (int32_t)(diff >> 12);
                if (new_imm >= -0x80000 && new_imm <= 0x7ffff) {
                    uint32_t rd = ins & 0x1f;
                    code[off/4] = 0x90000000 | ((new_imm & 3) << 29) |
                                  (((new_imm >> 2) & 0x7ffff) << 5) | rd;
                }
            }
            __builtin___clear_cache(copy, (void*)((uintptr_t)copy + copy_sz));

            // Patch out the canary check: at offset 0x1f64e88 (JNI_OnLoad+0x30),
            // change "ldr x8, [x24]" to "mov x8, xzr" (0xaa1f03e8)
            // This zeros x8 instead of loading the canary, making the check pass.
            uintptr_t ldr_off = 0x1f64e88 - jni_page;
            if (ldr_off < copy_sz) {
                code[ldr_off/4] = 0xaa1f03e8;  // mov x8, xzr
                fprintf(stderr, "[jni_shim] patched canary check at +0x%zx\n", ldr_off);
            }

            uintptr_t onload_off = 0x1f64e58 - jni_page;
            jni_onload = (int (*)(JavaVM*, void*))((uintptr_t)copy + onload_off);
            fprintf(stderr, "[jni_shim] JNI_OnLoad copied to %p\n", copy);
        }
    }

    fprintf(stderr, "[jni_shim] JNI_OnLoad at %p, calling...\n", (void*)jni_onload);
    fflush(stderr);

    jint ver = jni_onload(&g_vm, NULL);
    fprintf(stderr, "[jni_shim] JNI_OnLoad -> 0x%x\n", ver);

    fprintf(stderr, "[jni_shim] Entering sleep loop\n");
    while (1) sleep(1);
    return 0;
}