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
    const void** functions;  // flat function pointer array
    void* reserved[4];
};

struct JNIEnv_ {
    const void** functions;  // points to jni_table
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

    // Slot 4: GetVersion
    jni_table[4] = stub_GetVersion;
    // Slot 5: DefineClass
    jni_table[5] = stub_FindClass;
    // Slot 6: FindClass
    jni_table[6] = stub_FindClass;
    // Slot 7: FromReflectedMethod
    jni_table[7] = stub_GetMethodID;
    // Slot 8: FromReflectedField
    jni_table[8] = stub_GetFieldID;
    // Slot 9: ToReflectedMethod
    jni_table[9] = stub_voidp;
    // Slot 10: GetSuperclass
    jni_table[10] = stub_FindClass;
    // Slot 11: IsAssignableFrom
    jni_table[11] = (void *)stub_GetVersion;
    // Slot 12: ToReflectedField
    jni_table[12] = stub_voidp;
    // Slot 13: Throw
    jni_table[13] = stub_Throw;
    // Slot 14: ThrowNew
    jni_table[14] = stub_ThrowNew;
    // Slot 15: ExceptionOccurred
    jni_table[15] = stub_ExceptionOccurred;
    // Slot 16: ExceptionDescribe
    jni_table[16] = stub_ExceptionDescribe;
    // Slot 17: ExceptionClear
    jni_table[17] = stub_ExceptionClear;
    // Slot 18: FatalError
    jni_table[18] = (void *)abort;
    // Slot 19: PushLocalFrame
    jni_table[19] = (void *)stub_GetVersion;
    // Slot 20: PopLocalFrame
    jni_table[20] = stub_voidp;
    // Slot 21: NewGlobalRef
    jni_table[21] = stub_NewGlobalRef;
    // Slot 22: DeleteGlobalRef
    jni_table[22] = stub_DeleteGlobalRef;
    // Slot 23: DeleteLocalRef
    jni_table[23] = stub_DeleteLocalRef;
    // Slot 24: IsSameObject
    jni_table[24] = (void *)stub_GetVersion;
    // Slot 25: NewLocalRef
    jni_table[25] = stub_NewGlobalRef;
    // Slot 26: EnsureLocalCapacity
    jni_table[26] = (void *)stub_GetVersion;
    // Slot 27: AllocObject
    jni_table[27] = stub_NewObject;
    // Slot 28: NewObject
    jni_table[28] = stub_NewObject;
    // Slot 29: NewObjectV
    jni_table[29] = stub_NewObject;
    // Slot 30: NewObjectA
    jni_table[30] = stub_NewObject;
    // Slot 31: GetObjectClass
    jni_table[31] = stub_FindClass;
    // Slot 32: IsInstanceOf
    jni_table[32] = (void *)stub_GetVersion;
    // Slot 33: GetMethodID
    jni_table[33] = stub_GetMethodID;
    // Slot 34: GetStringUTFChars
    jni_table[34] = stub_GetStringUTFChars;
    // Slot 35: ReleaseStringUTFChars
    jni_table[35] = stub_ReleaseStringUTFChars;
    // Slot 36: NewStringUTF
    jni_table[36] = stub_NewStringUTF;
    // Slot 37: GetArrayLength
    jni_table[37] = stub_GetArrayLength;
    // Slot 38: NewObjectArray
    jni_table[38] = stub_NewObjectArray;
    // Slot 39: GetObjectArrayElement
    jni_table[39] = stub_GetObjectArrayElement;
    // Slot 40: SetObjectArrayElement
    jni_table[40] = stub_SetObjectArrayElement;

    // CallMethod slots 41-100
    jni_table[41] = stub_CallObjectMethod; jni_table[42] = stub_CallObjectMethodV; jni_table[43] = stub_CallObjectMethodA;
    jni_table[44] = stub_CallBooleanMethod; jni_table[45] = stub_CallBooleanMethodV; jni_table[46] = stub_CallBooleanMethodA;
    jni_table[47] = stub_CallByteMethod; jni_table[48] = stub_CallByteMethodV; jni_table[49] = stub_CallByteMethodA;
    jni_table[50] = stub_CallCharMethod; jni_table[51] = stub_CallCharMethodV; jni_table[52] = stub_CallCharMethodA;
    jni_table[53] = stub_CallShortMethod; jni_table[54] = stub_CallShortMethodV; jni_table[55] = stub_CallShortMethodA;
    jni_table[56] = stub_CallIntMethod; jni_table[57] = stub_CallIntMethodV; jni_table[58] = stub_CallIntMethodA;
    jni_table[59] = stub_CallLongMethod; jni_table[60] = stub_CallLongMethodV; jni_table[61] = stub_CallLongMethodA;
    jni_table[62] = stub_CallFloatMethod; jni_table[63] = stub_CallFloatMethodV; jni_table[64] = stub_CallFloatMethodA;
    jni_table[65] = stub_CallDoubleMethod; jni_table[66] = stub_CallDoubleMethodV; jni_table[67] = stub_CallDoubleMethodA;
    jni_table[68] = stub_CallVoidMethod; jni_table[69] = stub_CallVoidMethodV; jni_table[70] = stub_CallVoidMethodA;

    jni_table[71] = stub_CallStaticObjectMethod; jni_table[72] = stub_CallStaticObjectMethodV; jni_table[73] = stub_CallStaticObjectMethodA;
    jni_table[74] = stub_CallStaticBooleanMethod; jni_table[75] = stub_CallStaticBooleanMethodV; jni_table[76] = stub_CallStaticBooleanMethodA;
    jni_table[77] = stub_CallStaticByteMethod; jni_table[78] = stub_CallStaticByteMethodV; jni_table[79] = stub_CallStaticByteMethodA;
    jni_table[80] = stub_CallStaticCharMethod; jni_table[81] = stub_CallStaticCharMethodV; jni_table[82] = stub_CallStaticCharMethodA;
    jni_table[83] = stub_CallStaticShortMethod; jni_table[84] = stub_CallStaticShortMethodV; jni_table[85] = stub_CallStaticShortMethodA;
    jni_table[86] = stub_CallStaticIntMethod; jni_table[87] = stub_CallStaticIntMethodV; jni_table[88] = stub_CallStaticIntMethodA;
    jni_table[89] = stub_CallStaticLongMethod; jni_table[90] = stub_CallStaticLongMethodV; jni_table[91] = stub_CallStaticLongMethodA;
    jni_table[92] = stub_CallStaticFloatMethod; jni_table[93] = stub_CallStaticFloatMethodV; jni_table[94] = stub_CallStaticFloatMethodA;
    jni_table[95] = stub_CallStaticDoubleMethod; jni_table[96] = stub_CallStaticDoubleMethodV; jni_table[97] = stub_CallStaticDoubleMethodA;

    jni_table[98] = stub_CallVoidMethod; jni_table[99] = stub_CallVoidMethodV; jni_table[100] = stub_CallVoidMethodA;

    // Field access slots 101-149
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
    jni_table[121] = stub_GetStaticObjectField; jni_table[122] = stub_GetStaticObjectField;
    jni_table[128] = stub_GetStaticIntField; jni_table[129] = stub_SetStaticIntField;
    jni_table[130] = stub_GetStaticLongField; jni_table[131] = stub_SetLongField;
    jni_table[132] = stub_GetStaticIntField; jni_table[133] = stub_SetStaticIntField;
    jni_table[134] = stub_GetStaticIntField; jni_table[135] = stub_SetStaticIntField;
    jni_table[136] = stub_GetStaticIntField; jni_table[137] = stub_SetStaticIntField;
    jni_table[138] = stub_GetStaticIntField; jni_table[139] = stub_SetStaticIntField;
    jni_table[140] = stub_GetFloatField; jni_table[141] = stub_GetFloatField;
    jni_table[142] = stub_GetFloatField; jni_table[143] = stub_GetFloatField;

    // String ops
    jni_table[147] = stub_GetStringUTFChars; jni_table[148] = stub_GetStringUTFChars;
    jni_table[149] = stub_ReleaseStringUTFChars;
    jni_table[150] = stub_NewStringUTF; jni_table[151] = stub_GetArrayLength;
    jni_table[152] = stub_GetStringUTFChars; jni_table[153] = stub_GetStringUTFChars;
    jni_table[154] = stub_GetStringUTFChars; jni_table[155] = stub_ReleaseStringUTFChars;
    jni_table[156] = stub_NewGlobalRef; jni_table[157] = stub_DeleteGlobalRef;
    jni_table[158] = stub_ExceptionOccurred;
    jni_table[159] = stub_NewObject; jni_table[160] = stub_voidp;
    jni_table[161] = stub_GetArrayLength; jni_table[162] = stub_voidp;

    // Natives
    jni_table[193] = stub_RegisterNatives; jni_table[194] = stub_RegisterNatives;
    jni_table[195] = stub_GetArrayLength; jni_table[196] = stub_voidp;
    jni_table[197] = stub_GetJavaVM;
    jni_table[198] = stub_GetStringUTFChars; jni_table[199] = stub_ReleaseStringUTFChars;

    // Fill 200-255
    for (int i = 200; i < JNI_SLOTS; i++) jni_table[i] = (void *)stub_GetVersion;

    // JavaVM table
    vm_table[3] = stub_DestroyJavaVM; vm_table[4] = stub_GetEnv_Attach;
    vm_table[5] = stub_DetachCurrentThread; vm_table[6] = stub_GetEnv_jint;
    vm_table[7] = stub_GetEnv_Attach;
    g_vm.functions = (const void**)vm_table;

    // Fill remaining NULLs
    for (int i = 0; i < JNI_SLOTS; i++) if (jni_table[i] == NULL) jni_table[i] = (void *)stub_GetVersion;
}

/* ============== QEMU exec page copy fix ==============
 *
 * QEMU 10.2.1 user-mode has a bug where it crashes (host-level SIGSEGV) when
 * JIT-compiling certain code pages of libroblox.so. The crash only happens on
 * pages near offset 0x1f64000 (JNI_OnLoad's page). Other pages work fine.
 *
 * The fix: copy JNI_OnLoad to a fresh mmap'd PROT_EXEC page, fix up all
 * adrp (PC-relative page address) instructions to point to the real targets,
 * and call the copy instead of the original.
 *
 * Additionally, when JNI_OnLoad calls sub-functions via bl, those may also
 * hit the same QEMU bug. We handle this with a SIGSEGV handler that works at
 * the HOST level: it's installed in main() and wraps the JNI_OnLoad call.
 * (Actually, QEMU's crash is internal, not a deliverable signal, so we can't
 * handle it from within the guest. We need to copy ALL potentially-bad pages.)
 *
 * Simple approach: copy JNI_OnLoad + all functions it's likely to call
 * (first ~64KB starting from the JNI_OnLoad page).
 */

/* Decode adrp immediate (21-bit signed) from instruction */
static int32_t adrp_get_imm(uint32_t instr) {
    int32_t imm = (((instr >> 5) & 0x7ffff) << 2) | ((instr >> 29) & 3);
    if (imm & 0x100000) imm -= 0x200000;
    return imm;
}

/* Encode adrp with new immediate */
static uint32_t adrp_set_imm(uint32_t orig, int32_t imm) {
    uint32_t rd = orig & 0x1f;
    return 0x90000000 | ((imm & 3) << 29) | (((imm >> 2) & 0x7ffff) << 5) | rd;
}

/* Fix adrp instructions in a copied code block.
 * For each adrp instruction, recalculate its immediate so it points to the
 * same target as the original would have, but from the new copy address.
 *
 * Parameters:
 *   copy     - start of the copy (in guest address space)
 *   copy_len - size of the copy in bytes
 *   orig_base - base address of libroblox.so
 *   copy_base - where the code was copied TO
 *   file_off  - file offset of the start of the copied region (relative to base)
 */
static void fix_adrp_in_range(void *copy, size_t copy_len,
                               uintptr_t orig_base, uintptr_t copy_base,
                               uintptr_t file_off) {
    uint32_t *p = (uint32_t*)copy;
    for (size_t off = 0; off < copy_len; off += 4) {
        uint32_t instr = p[off/4];
        if ((instr >> 24) != 0x90) continue;  // not adrp

        int32_t old_imm = adrp_get_imm(instr);
        uintptr_t orig_pc = orig_base + file_off + off;
        uintptr_t orig_page = orig_pc & ~0xfffULL;
        uintptr_t target_abs = orig_page + (old_imm << 12);

        uintptr_t copy_pc = copy_base + off;
        uintptr_t copy_page = copy_pc & ~0xfffULL;

        int64_t diff = (int64_t)(target_abs - copy_page);
        int32_t new_imm = (int32_t)(diff >> 12);

        if (new_imm < -0x80000 || new_imm > 0x7ffff) {
            fprintf(stderr, "[jni_shim] WARNING: adrp imm out of range at +0x%zx\n", off);
            continue;
        }

        p[off/4] = adrp_set_imm(instr, new_imm);
    }
}

/* SIGSEGV handler for guest-level fix: copy faulting page and redirect */
static struct sigaction jni_old_sa;
static uintptr_t jni_base = 0;
static uintptr_t jni_text_start = 0;
static uintptr_t jni_text_end = 0;
static void *jni_fallback_region = NULL;
static uintptr_t jni_fallback_used = 0;

static void jni_segv_handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t *u = (ucontext_t*)ctx;
    uintptr_t fault_addr = (uintptr_t)info->si_addr;
    uintptr_t fault_page = fault_addr & ~0xfffULL;

    // Check if this is in our text segment
    if (fault_addr >= jni_text_start && fault_addr < jni_text_end) {
        // Allocate a page from the fallback region
        uintptr_t dest = ((uintptr_t)jni_fallback_region + jni_fallback_used + 0xfff) & ~0xfffULL;
        if (dest + 0x10000 < (uintptr_t)jni_fallback_region + 0x40000000) {
            // mmap a 64KB chunk at dest
            void *map = mmap((void*)dest, 0x10000, PROT_READ|PROT_WRITE|PROT_EXEC,
                           MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED, -1, 0);
            if (map != MAP_FAILED) {
                // Try to read from the faulting address (may or may not work)
                memcpy(map, (void*)fault_page, 0x10000);
                // Fix adrp
                fix_adrp_in_range(map, 0x10000, jni_base, dest, fault_page - jni_base);
                __builtin___clear_cache(map, (void*)(dest + 0x10000));

                // Redirect PC to the copy
                uintptr_t pc_delta = u->uc_mcontext.pc - fault_page;
                u->uc_mcontext.pc = dest + pc_delta;

                jni_fallback_used = dest + 0x10000 - (uintptr_t)jni_fallback_region;
                return;  // resume
            }
        }
    }
    // Can't handle — chain to old
    sigaction(SIGSEGV, &jni_old_sa, NULL);
}

int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    fprintf(stderr, "[jni_shim] Loading bionic shim...\n");
    void *bionic_shim = dlopen("libbionic_shim.so", RTLD_LAZY | RTLD_GLOBAL);
    if (!bionic_shim)
        fprintf(stderr, "[jni_shim] WARNING: libbionic_shim.so not found: %s\n", dlerror());

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);
    void* handle = dlopen(lib_path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "[jni_shim] Failed: %s\n", dlerror());
        return 1;
    }
    fprintf(stderr, "[jni_shim] Loaded successfully\n");

    // Fix __stack_chk_guard GOT entry
    // The GOT canary pointer is at base+0x6473438 (inside .got.plt)
    // After RTLD_NOW dlopen, the GOT is read-only (RELRO), so mprotect first
    Dl_info dl_info;
    if (dladdr((void*)dlsym(handle, "JNI_OnLoad"), &dl_info)) {
        uintptr_t base = (uintptr_t)dl_info.dli_fbase;
        uintptr_t guard_ptr_addr = base + 0x6473438;
        uintptr_t* guard_ptr = (uintptr_t*)guard_ptr_addr;

        // Make entire RELRO region writable: covers .got, .got.plt, .relro_padding
        uintptr_t got_start = base + 0x5fbd000;
        uintptr_t got_end   = base + 0x6475000;
        mprotect((void*)got_start, got_end - got_start, PROT_READ | PROT_WRITE);

        fprintf(stderr, "[jni_shim] libroblox base=%p, guard_ptr at %p = %p\n",
                (void*)base, (void*)guard_ptr, (void*)*guard_ptr);
        if (*guard_ptr == 0) {
            *guard_ptr = (uintptr_t)&g_canary;
            fprintf(stderr, "[jni_shim] stack_chk_guard patched to -> 0x%lx\n", g_canary);
        }

        uintptr_t* libc_guard = (uintptr_t*)dlsym(RTLD_NEXT, "__stack_chk_guard");
        if (libc_guard && *libc_guard == 0) {
            *libc_guard = g_canary;
            fprintf(stderr, "[jni_shim] libc __stack_chk_guard set\n");
        }

        // Set up globals for SIGSEGV handler
        jni_base = base;
        jni_text_start = base;
        jni_text_end = base + 0x5fb92a0;

        // Allocate fallback region for page copies
        jni_fallback_region = mmap(NULL, 0x40000000, PROT_NONE,
                                   MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (jni_fallback_region != MAP_FAILED) {
            jni_fallback_used = 0;

            // Install SIGSEGV handler for page copies
            struct sigaction sa;
            memset(&sa, 0, sizeof(sa));
            sa.sa_sigaction = jni_segv_handler;
            sa.sa_flags = SA_SIGINFO;
            sigemptyset(&sa.sa_mask);
            sigaction(SIGSEGV, &sa, &jni_old_sa);
            fprintf(stderr, "[jni_shim] Fallback handler installed\n");
        }

        // ===== COPY JNI_OnLoad to an executable page =====
        // JNI_OnLoad is at file offset 0x1f64e58, size 0xaf4 bytes
        // We also copy the surrounding 64KB to catch sub-functions
        uintptr_t jni_file_off = 0x1f64000;  // page-aligned
        size_t copy_size = 0x20000;  // 128KB covers JNI_OnLoad + nearby functions

        void *jni_orig = (void*)(base + jni_file_off);
        void *jni_copy = mmap(NULL, copy_size, PROT_READ|PROT_WRITE|PROT_EXEC,
                              MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);

        memcpy(jni_copy, jni_orig, copy_size);
        fix_adrp_in_range(jni_copy, copy_size, base, (uintptr_t)jni_copy, jni_file_off);
        __builtin___clear_cache(jni_copy, (void*)((uintptr_t)jni_copy + copy_size));

        fprintf(stderr, "[jni_shim] JNI_OnLoad copied to %p (adrp fixed)\n", jni_copy);

        // Calculate offset of JNI_OnLoad within the copied chunk
        uintptr_t jni_copy_addr = (uintptr_t)jni_copy + (0x1f64e58 - jni_file_off);
        fprintf(stderr, "[jni_shim] JNI_OnLoad at %p, calling...\n", (void*)jni_copy_addr);
        fflush(stderr);

        // Set up JNI env and call JNI_OnLoad via the copy
        g_env.functions = (const void**)jni_table;

        typedef int (*jni_onload_t)(JavaVM*, void*);
        jni_onload_t jni_onload = (jni_onload_t)jni_copy_addr;

        jint ver = jni_onload(&g_vm, NULL);
        fprintf(stderr, "[jni_shim] JNI_OnLoad -> 0x%x\n", ver);
    }

    fprintf(stderr, "[jni_shim] Entering sleep loop\n");
    while (1) sleep(1);
    return 0;
}
