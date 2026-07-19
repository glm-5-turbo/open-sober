// Minimal JNI shim to load libroblox.so and call JNI_OnLoad.
// This bypasses the need for a full Android Java runtime.
// Compile: gcc -o jni_shim jni_shim.c -ldl

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

// We use a simple function pointer array indexed by JNI slot number.
// This avoids complex struct typedef dependencies.
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

// JavaVM function table — flat array indexed by slot
// JNI spec slot layout:
//   0-2: reserved
//   3: DestroyJavaVM
//   4: AttachCurrentThread
//   5: DetachCurrentThread
//   6: GetEnv
//   7: AttachCurrentThreadAsDaemon
#define VM_SLOTS 16
static void* vm_table[VM_SLOTS];


// Global instances (declared BEFORE stubs that reference them)
static JNIEnv g_env;
static JavaVM g_vm;

// AttachCurrentThread / GetEnv implementations
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
    // Zero out the table
    memset(jni_table, 0, sizeof(jni_table));

    // ===== JNI 1.6 standard function table =====
    // Slots are per the JNI specification for JNIEnv

    // Slots 0-3: Reserved (NULL)
    // Slot 4: GetVersion
    jni_table[4] = stub_GetVersion;
    // Slot 5: DefineClass
    jni_table[5] = stub_FindClass;  // safe enough
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
    jni_table[11] = (void *)stub_GetVersion; // returns JNI_TRUE/0 equivalent
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
    jni_table[18] = (void *)abort;  // actually abort on fatal
    // Slot 19: PushLocalFrame
    jni_table[19] = (void *)stub_GetVersion; // returns JNI_OK
    // Slot 20: PopLocalFrame
    jni_table[20] = stub_voidp;
    // Slot 21: NewGlobalRef
    jni_table[21] = stub_NewGlobalRef;
    // Slot 22: DeleteGlobalRef
    jni_table[22] = stub_DeleteGlobalRef;
    // Slot 23: DeleteLocalRef
    jni_table[23] = stub_DeleteLocalRef;
    // Slot 24: IsSameObject
    jni_table[24] = (void *)stub_GetVersion; // JNI_TRUE
    // Slot 25: NewLocalRef
    jni_table[25] = stub_NewGlobalRef;
    // Slot 26: EnsureLocalCapacity
    jni_table[26] = (void *)stub_GetVersion; // JNI_OK
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
    jni_table[32] = (void *)stub_GetVersion; // JNI_TRUE
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
    // Slots 41-43: CallObjectMethod(V/A)
    jni_table[41] = stub_CallObjectMethod;
    jni_table[42] = stub_CallObjectMethodV;
    jni_table[43] = stub_CallObjectMethodA;
    // Slots 44-46: CallBooleanMethod(V/A)
    jni_table[44] = stub_CallBooleanMethod;
    jni_table[45] = stub_CallBooleanMethodV;
    jni_table[46] = stub_CallBooleanMethodA;
    // Slots 47-49: CallByteMethod(V/A)
    jni_table[47] = stub_CallByteMethod;
    jni_table[48] = stub_CallByteMethodV;
    jni_table[49] = stub_CallByteMethodA;
    // Slots 50-52: CallCharMethod(V/A)
    jni_table[50] = stub_CallCharMethod;
    jni_table[51] = stub_CallCharMethodV;
    jni_table[52] = stub_CallCharMethodA;
    // Slots 53-55: CallShortMethod(V/A)
    jni_table[53] = stub_CallShortMethod;
    jni_table[54] = stub_CallShortMethodV;
    jni_table[55] = stub_CallShortMethodA;
    // Slots 56-58: CallIntMethod(V/A)
    jni_table[56] = stub_CallIntMethod;
    jni_table[57] = stub_CallIntMethodV;
    jni_table[58] = stub_CallIntMethodA;
    // Slots 59-61: CallLongMethod(V/A)
    jni_table[59] = stub_CallLongMethod;
    jni_table[60] = stub_CallLongMethodV;
    jni_table[61] = stub_CallLongMethodA;
    // Slots 62-64: CallFloatMethod(V/A)
    jni_table[62] = stub_CallFloatMethod;
    jni_table[63] = stub_CallFloatMethodV;
    jni_table[64] = stub_CallFloatMethodA;
    // Slots 65-67: CallDoubleMethod(V/A)
    jni_table[65] = stub_CallDoubleMethod;
    jni_table[66] = stub_CallDoubleMethodV;
    jni_table[67] = stub_CallDoubleMethodA;
    // Slots 68-70: CallVoidMethod(V/A)
    jni_table[68] = stub_CallVoidMethod;
    jni_table[69] = stub_CallVoidMethodV;
    jni_table[70] = stub_CallVoidMethodA;
    // Slots 71-73: CallStaticObjectMethod(V/A)
    jni_table[71] = stub_CallStaticObjectMethod;
    jni_table[72] = stub_CallStaticObjectMethodV;
    jni_table[73] = stub_CallStaticObjectMethodA;
    // Slots 74-76: CallStaticBooleanMethod(V/A)
    jni_table[74] = stub_CallStaticBooleanMethod;
    jni_table[75] = stub_CallStaticBooleanMethodV;
    jni_table[76] = stub_CallStaticBooleanMethodA;
    // Slots 77-79: CallStaticByteMethod(V/A)
    jni_table[77] = stub_CallStaticByteMethod;
    jni_table[78] = stub_CallStaticByteMethodV;
    jni_table[79] = stub_CallStaticByteMethodA;
    // Slots 80-82: CallStaticCharMethod(V/A)
    jni_table[80] = stub_CallStaticCharMethod;
    jni_table[81] = stub_CallStaticCharMethodV;
    jni_table[82] = stub_CallStaticCharMethodA;
    // Slots 83-85: CallStaticShortMethod(V/A)
    jni_table[83] = stub_CallStaticShortMethod;
    jni_table[84] = stub_CallStaticShortMethodV;
    jni_table[85] = stub_CallStaticShortMethodA;
    // Slots 86-88: CallStaticIntMethod(V/A)
    jni_table[86] = stub_CallStaticIntMethod;
    jni_table[87] = stub_CallStaticIntMethodV;
    jni_table[88] = stub_CallStaticIntMethodA;
    // Slots 89-91: CallStaticLongMethod(V/A)
    jni_table[89] = stub_CallStaticLongMethod;
    jni_table[90] = stub_CallStaticLongMethodV;
    jni_table[91] = stub_CallStaticLongMethodA;
    // Slots 92-94: CallStaticFloatMethod(V/A)
    jni_table[92] = stub_CallStaticFloatMethod;
    jni_table[93] = stub_CallStaticFloatMethodV;
    jni_table[94] = stub_CallStaticFloatMethodA;
    // Slots 95-97: CallStaticDoubleMethod(V/A)
    jni_table[95] = stub_CallStaticDoubleMethod;
    jni_table[96] = stub_CallStaticDoubleMethodV;
    jni_table[97] = stub_CallStaticDoubleMethodA;
    // Slot 98-100: CallNonvirtualVoidMethod(V/A) — safe alias to CallVoidMethod
    jni_table[98] = stub_CallVoidMethod;
    jni_table[99] = stub_CallVoidMethodV;
    jni_table[100] = stub_CallVoidMethodA;
    // Slots 101-149: Field access
    // Slot 101: GetFieldID
    jni_table[101] = stub_GetFieldID;
    // Slots 102-103: Get/SetObjectField
    jni_table[102] = stub_GetObjectField;
    jni_table[103] = stub_SetObjectField;
    // Slots 104-105: Get/SetBooleanField
    jni_table[104] = stub_GetBooleanField;
    // Slot 105: SetBooleanField — missing, use int variant
    jni_table[105] = stub_SetIntField;
    // Slots 106-107: Get/SetByteField
    jni_table[106] = stub_GetBooleanField;
    jni_table[107] = stub_SetIntField;
    // Slots 108-109: Get/SetCharField
    jni_table[108] = stub_GetIntField;
    jni_table[109] = stub_SetIntField;
    // Slots 110-111: Get/SetShortField
    jni_table[110] = stub_GetIntField;
    jni_table[111] = stub_SetIntField;
    // Slots 112-113: Get/SetIntField
    jni_table[112] = stub_GetIntField;
    jni_table[113] = stub_SetIntField;
    // Slots 114-115: Get/SetLongField
    jni_table[114] = stub_GetLongField;
    jni_table[115] = stub_SetLongField;
    // Slots 116-117: Get/SetFloatField
    jni_table[116] = stub_GetFloatField;
    jni_table[117] = stub_GetFloatField;
    // Slots 118-119: Get/SetDoubleField
    jni_table[118] = stub_GetFloatField;
    jni_table[119] = stub_GetFloatField;
    // Slot 120: GetStaticFieldID
    jni_table[120] = stub_GetStaticFieldID;
    // Slots 121-122: Get/SetStaticObjectField
    jni_table[121] = stub_GetStaticObjectField;
    // Slot 122: SetStaticObjectField — alias
    jni_table[122] = stub_GetStaticObjectField;
    // Slots 123-127: Static primitive field get/set
    // Slot 128-129: Get/SetStaticIntField
    jni_table[128] = stub_GetStaticIntField;
    jni_table[129] = stub_SetStaticIntField;
    // Slots 130-131: Get/SetStaticLongField
    jni_table[130] = stub_GetStaticLongField;
    // Slot 131: SetStaticLongField
    jni_table[131] = stub_SetLongField;
    // Slots 132-141: More static field variants — fill with generic stubs
    jni_table[132] = stub_GetStaticIntField; // GetStaticBooleanField
    jni_table[133] = stub_SetStaticIntField; // SetStaticBooleanField
    jni_table[134] = stub_GetStaticIntField; // GetStaticByteField
    jni_table[135] = stub_SetStaticIntField; // SetStaticByteField
    jni_table[136] = stub_GetStaticIntField; // GetStaticCharField
    jni_table[137] = stub_SetStaticIntField; // SetStaticCharField
    jni_table[138] = stub_GetStaticIntField; // GetStaticShortField
    jni_table[139] = stub_SetStaticIntField; // SetStaticShortField
    jni_table[140] = stub_GetFloatField;     // GetStaticFloatField
    jni_table[141] = stub_GetFloatField;     // SetStaticFloatField

    // Slots 142-143: Get/SetStaticDoubleField
    jni_table[142] = stub_GetFloatField;     // GetStaticDoubleField
    jni_table[143] = stub_GetFloatField;     // SetStaticDoubleField

    // Slots 144-146: CallStaticVoidMethod(V/A) — filled above at 98-100

    // Slots 147-148: GetStringLength, GetStringChars, etc.
    jni_table[147] = stub_GetStringUTFChars;  // GetStringLength (returns const jchar*)
    jni_table[148] = stub_GetStringUTFChars;  // GetStringChars
    jni_table[149] = stub_ReleaseStringUTFChars; // ReleaseStringChars

    // Slots 150-156: NewString, GetStringRegion, etc.
    jni_table[150] = stub_NewStringUTF;       // NewString
    jni_table[151] = stub_GetArrayLength;     // GetStringLength (returns jsize)
    jni_table[152] = stub_GetStringUTFChars;  // GetStringUTFRegion
    jni_table[153] = stub_GetStringUTFChars;  // GetStringRegion

    // Slots 154-156: String critical
    jni_table[154] = stub_GetStringUTFChars;  // GetStringCritical
    jni_table[155] = stub_ReleaseStringUTFChars; // ReleaseStringCritical

    // Slots 156-159: NewWeakGlobalRef, DeleteWeakGlobalRef
    jni_table[156] = stub_NewGlobalRef;       // NewWeakGlobalRef
    jni_table[157] = stub_DeleteGlobalRef;    // DeleteWeakGlobalRef

    // Slot 158: ExceptionCheck
    jni_table[158] = stub_ExceptionOccurred;  // returns jthrowable

    // Slots 159-170: NewDirectByteBuffer, GetDirectBufferAddress, etc.
    jni_table[159] = stub_NewObject;          // NewDirectByteBuffer
    jni_table[160] = stub_voidp;             // GetDirectBufferAddress
    jni_table[161] = stub_GetArrayLength;    // GetDirectBufferCapacity

    // Slots 162-191: GetObjectRefType, etc.
    jni_table[162] = stub_voidp;             // GetObjectRefType

    // Slots 192-199: reflection
    // Slot 193: RegisterNatives
    jni_table[193] = stub_RegisterNatives;
    // Slot 194: UnregisterNatives
    jni_table[194] = stub_RegisterNatives;   // JNI_OK
    // Slot 195: GetStringUTFRegion — use array length
    jni_table[195] = stub_GetArrayLength;
    // Slot 196: GetPrimitiveArrayCritical
    jni_table[196] = stub_voidp;
    // Slot 197: GetJavaVM (JNI spec slot 197)
    jni_table[197] = stub_GetJavaVM;
    // Slot 198: GetStringCritical
    jni_table[198] = stub_GetStringUTFChars;
    // Slot 199: ReleaseStringCritical
    jni_table[199] = stub_ReleaseStringUTFChars;

    // Slots 200-255: newer JNI functions (Java 8+)
    for (int i = 200; i < JNI_SLOTS; i++) {
        jni_table[i] = (void *)stub_GetVersion; // JNI_OK for functions returning jint
    }

    // JavaVM function table — flat array indexed by JNI spec slot
    vm_table[3] = stub_DestroyJavaVM;      // DestroyJavaVM (slot 3)
    vm_table[4] = stub_GetEnv_Attach;       // AttachCurrentThread (slot 4)
    vm_table[5] = stub_DetachCurrentThread; // DetachCurrentThread (slot 5)
    vm_table[6] = stub_GetEnv_jint;         // GetEnv (slot 6)
    vm_table[7] = stub_GetEnv_Attach;       // AttachCurrentThreadAsDaemon (slot 7)
    g_vm.functions = (const void**)vm_table;

    // Fill any remaining NULL slots with a safe default
    // This prevents SIGSEGV if JNI_OnLoad calls a slot we didn't explicitly set
    for (int i = 0; i < JNI_SLOTS; i++) {
        if (jni_table[i] == NULL) {
            jni_table[i] = (void *)stub_GetVersion;
        }
    }
}

/* One-shot SIGSEGV handler: skip past faulting instruction in QEMU
 * (handles RELRO page access faults), then restore default handler. */
static struct sigaction jni_old_sa;
static void jni_segv_handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t* u = (ucontext_t*)ctx;
    fprintf(stderr, "[jni_shim] SIGSEGV at PC=0x%lx, fault=%p, skipping\n",
            u->uc_mcontext.pc, info->si_addr);
    u->uc_mcontext.pc += 4;
    sigaction(SIGSEGV, &jni_old_sa, NULL);
}

int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    // Install one-shot SIGSEGV handler before accessing loaded library
    // QEMU user-mode may enforce RELRO read-only GOT pages, causing SIGSEGV
    // when reading GOT entries. The handler skips the fault and returns.
    struct sigaction sa_jni;
    memset(&sa_jni, 0, sizeof(sa_jni));
    sa_jni.sa_sigaction = jni_segv_handler;
    sa_jni.sa_flags = SA_SIGINFO;
    sigemptyset(&sa_jni.sa_mask);
    sigaction(SIGSEGV, &sa_jni, &jni_old_sa);

    fprintf(stderr, "[jni_shim] Loading bionic shim...\n");
    void *bionic_shim = dlopen("libbionic_shim.so", RTLD_LAZY | RTLD_GLOBAL);
    if (!bionic_shim)
        fprintf(stderr, "[jni_shim] WARNING: libbionic_shim.so not found: %s\n", dlerror());

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);
    void* handle = dlopen(lib_path, RTLD_LAZY | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "[jni_shim] Failed: %s\n", dlerror());
        return 1;
    }
    fprintf(stderr, "[jni_shim] Loaded successfully\n");

    // Set up a one-shot SIGSEGV handler to handle RELRO page access issues.
    // After dlopen with RTLD_NOW, some load instructions from RELRO-protected
    // GOT pages may trigger SIGSEGV in QEMU user-mode. The handler skips past
    // the faulting instruction (returns a NULL/garbage value which is fine
    // since we patch the GOT entry immediately after).
    struct sigaction sa, old_sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = SIG_IGN;  // skip SIGSEGV — let the load return whatever is in x8
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, &old_sa);

    // Fix __stack_chk_guard in libroblox.so's data section.
    // The GOT entry pointing to __stack_chk_guard is at base+0x6473438
    // (inside .got.plt). After RTLD_NOW dlopen, the GOT is in a RELRO
    // read-only region. We must mprotect it writable before patching.
    Dl_info dl_info;
    if (dladdr((void*)dlsym(handle, "JNI_OnLoad"), &dl_info)) {
        uintptr_t base = (uintptr_t)dl_info.dli_fbase;
        uintptr_t guard_ptr_addr = base + 0x6473438;
        uintptr_t* guard_ptr = (uintptr_t*)guard_ptr_addr;

        // Make the entire GOT region writable (covers .got, .got.plt, .relro_padding)
        // libroblox.so data segments:
        //   LOAD-2: vaddr 0x5fbd2c0, size 0x4b7268, RW (contains .got, .got.plt)
        //   .got.plt spans 0x6473440 to 0x6474528
        //   The RELRO mprotect covers start of LOAD-2's RELRO to .got.plt end
        // We need full RW for the entire range.
        uintptr_t got_start = base + 0x5fbd000;  // page-aligned start of data segment
        uintptr_t got_end   = base + 0x6475000;  // page aligned end of .got.plt region
        mprotect((void*)got_start, got_end - got_start, PROT_READ | PROT_WRITE);

        fprintf(stderr, "[jni_shim] libroblox base=%p, guard_ptr at %p = %p\n",
                (void*)base, (void*)guard_ptr, (void*)*guard_ptr);
        if (*guard_ptr == 0) {
            // GOT was read-only (RELRO). Now writable after mprotect.
            *guard_ptr = (uintptr_t)&g_canary;
            fprintf(stderr, "[jni_shim] stack_chk_guard patched to -> 0x%lx\n", g_canary);
        }

        // Also set the global __stack_chk_guard for thread-safe access
        uintptr_t* libc_guard = (uintptr_t*)dlsym(RTLD_NEXT, "__stack_chk_guard");
        if (libc_guard && *libc_guard == 0) {
            *libc_guard = g_canary;
            fprintf(stderr, "[jni_shim] libc __stack_chk_guard set at %p\n", (void*)libc_guard);
        }
    }

    // Set up JNI env — functions pointer is jni_table
    g_env.functions = (const void**)jni_table;

    // Set up JavaVM with correct interface
    // g_vm.functions already set in constructor

    // Call JNI_OnLoad
    typedef int (*jni_onload_t)(JavaVM*, void*);
    jni_onload_t jni_onload = (jni_onload_t)dlsym(handle, "JNI_OnLoad");
    if (!jni_onload)
        jni_onload = (jni_onload_t)dlvsym(handle, "JNI_OnLoad", "LIBROBLOX");
    // Try to set __stack_chk_guard to a non-zero value.
    // libroblox.so accesses this via the thread pointer (tpidr_el0).
    // If it's 0, the stack protector crashes on any function entry.
    void* libc_guard = dlsym(RTLD_DEFAULT, "__stack_chk_guard");
    if (libc_guard) {
        uintptr_t* g = (uintptr_t*)libc_guard;
        if (*g == 0) {
            *g = 0x0A0B0C0D0E0F1011ULL;
            fprintf(stderr, "[jni_shim] set __stack_chk_guard at %p = 0x%lx\n", g, *g);
        }
    } else {
        fprintf(stderr, "[jni_shim] WARNING: __stack_chk_guard not found\n");
    }

    if (jni_onload) {
        fprintf(stderr, "[jni_shim] JNI_OnLoad at %p, calling...\n", (void*)jni_onload);
        jint ver = jni_onload(&g_vm, NULL);
        fprintf(stderr, "[jni_shim] JNI_OnLoad -> 0x%x\n", ver);
    } else {
        fprintf(stderr, "[jni_shim] JNI_OnLoad not found\n");
    }

    fprintf(stderr, "[jni_shim] Entering sleep loop\n");
    while (1) sleep(1);
    return 0;
}