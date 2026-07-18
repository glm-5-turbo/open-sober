// SPDX-License-Identifier: MIT
//
// JNI function table structures.
//
// These mirror the C struct JNINativeInterface (for JNIEnv) and
// JNIInvokeInterface (for JavaVM) exactly — every function pointer in
// the correct slot.  libroblox.so will call through these tables.

#![allow(non_camel_case_types, dead_code)]

use crate::types::*;
use std::os::raw::c_char;
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Forward declaration — our opaque env/vm wrappers
// ---------------------------------------------------------------------------

/// Opaque JNIEnv wrapper.  The first field MUST be a pointer to
/// JNINativeFunctions, because C code casts it as `const struct
/// JNINativeInterface**` and dereferences it to get the function table.
#[repr(C)]
pub struct JNIEnvWrapper {
    pub functions: *const JNINativeFunctions,
}

/// Opaque JavaVM wrapper.  Same trick — first field is a pointer to
/// JNIInvokeFunctions.
#[repr(C)]
pub struct JavaVMWrapper {
    pub functions: *const JNIInvokeFunctions,
}

pub type JNIEnvPtr = *mut JNIEnvWrapper;
pub type JavaVMPtr = *mut JavaVMWrapper;

// ---------------------------------------------------------------------------
// JNIEnv function table (struct JNINativeInterface)
//
// 233 slots total (indices 0..232), matching the JNI 1.6 spec.
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct JNINativeFunctions {
    // Reserved / COM compatibility (indices 0..3)
    pub reserved0: *mut c_void,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,
    pub reserved3: *mut c_void,

    // Version (index 4)
    pub GetVersion: Option<unsafe extern "C" fn(env: JNIEnvPtr) -> jint>,

    // Class operations (indices 5..6)
    pub DefineClass: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            name: *const c_char,
            loader: jobject,
            buf: *const jbyte,
            bufLen: jsize,
        ) -> jclass,
    >,
    pub FindClass:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, name: *const c_char) -> jclass>,

    // Reflection (indices 7..9)
    pub FromReflectedMethod:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, method: jobject) -> jmethodID>,
    pub FromReflectedField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, field: jobject) -> jfieldID>,
    pub ToReflectedMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            cls: jclass,
            methodID: jmethodID,
            isStatic: jboolean,
        ) -> jobject,
    >,

    // Class operations (indices 10..11)
    pub GetSuperclass:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass) -> jclass>,
    pub IsAssignableFrom:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz1: jclass, clazz2: jclass) -> jboolean>,

    // Reflection (index 12)
    pub ToReflectedField: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            cls: jclass,
            fieldID: jfieldID,
            isStatic: jboolean,
        ) -> jobject,
    >,

    // Exceptions (indices 13..18)
    pub Throw:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jthrowable) -> jint>,
    pub ThrowNew: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, message: *const c_char) -> jint,
    >,
    pub ExceptionOccurred:
        Option<unsafe extern "C" fn(env: JNIEnvPtr) -> jthrowable>,
    pub ExceptionDescribe: Option<unsafe extern "C" fn(env: JNIEnvPtr)>,
    pub ExceptionClear: Option<unsafe extern "C" fn(env: JNIEnvPtr)>,
    pub FatalError:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, msg: *const c_char)>,

    // Local reference frames (indices 19..20)
    pub PushLocalFrame:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, capacity: jint) -> jint>,
    pub PopLocalFrame:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, result: jobject) -> jobject>,

    // Global / local references (indices 21..26)
    pub NewGlobalRef:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jobject>,
    pub DeleteGlobalRef: Option<unsafe extern "C" fn(env: JNIEnvPtr, globalRef: jobject)>,
    pub DeleteLocalRef: Option<unsafe extern "C" fn(env: JNIEnvPtr, localRef: jobject)>,
    pub IsSameObject: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, ref1: jobject, ref2: jobject) -> jboolean,
    >,
    pub NewLocalRef:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, ref_: jobject) -> jobject>,
    pub EnsureLocalCapacity:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, capacity: jint) -> jint>,

    // Object operations (indices 27..32)
    pub AllocObject:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass) -> jobject>,
    pub NewObject: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, methodID: jmethodID, ...) -> jobject,
    >,
    pub NewObjectV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jobject,
    >,
    pub NewObjectA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jobject,
    >,
    pub GetObjectClass:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jclass>,
    pub IsInstanceOf:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, clazz: jclass) -> jboolean>,

    // GetMethodID (index 33)
    pub GetMethodID: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            name: *const c_char,
            sig: *const c_char,
        ) -> jmethodID,
    >,

    // ── Call<type>Method variants (indices 34..63) ──────────────────
    // CallObjectMethod (34..36)
    pub CallObjectMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jobject,
    >,
    pub CallObjectMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jobject,
    >,
    pub CallObjectMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jobject,
    >,

    // CallBooleanMethod (37..39)
    pub CallBooleanMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jboolean,
    >,
    pub CallBooleanMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jboolean,
    >,
    pub CallBooleanMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jboolean,
    >,

    // CallByteMethod (40..42)
    pub CallByteMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jbyte,
    >,
    pub CallByteMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jbyte,
    >,
    pub CallByteMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jbyte,
    >,

    // CallCharMethod (43..45)
    pub CallCharMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jchar,
    >,
    pub CallCharMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jchar,
    >,
    pub CallCharMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jchar,
    >,

    // CallShortMethod (46..48)
    pub CallShortMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jshort,
    >,
    pub CallShortMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jshort,
    >,
    pub CallShortMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jshort,
    >,

    // CallIntMethod (49..51)
    pub CallIntMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jint,
    >,
    pub CallIntMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jint,
    >,
    pub CallIntMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jint,
    >,

    // CallLongMethod (52..54)
    pub CallLongMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jlong,
    >,
    pub CallLongMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jlong,
    >,
    pub CallLongMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jlong,
    >,

    // CallFloatMethod (55..57)
    pub CallFloatMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jfloat,
    >,
    pub CallFloatMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jfloat,
    >,
    pub CallFloatMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jfloat,
    >,

    // CallDoubleMethod (58..60)
    pub CallDoubleMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...) -> jdouble,
    >,
    pub CallDoubleMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jdouble,
    >,
    pub CallDoubleMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jdouble,
    >,

    // CallVoidMethod (61..63)
    pub CallVoidMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, methodID: jmethodID, ...),
    >,
    pub CallVoidMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ),
    >,
    pub CallVoidMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            methodID: jmethodID,
            args: *const jvalue,
        ),
    >,

    // ── CallNonvirtual<type>Method variants (indices 64..93) ────────
    pub CallNonvirtualObjectMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jobject,
    >,
    pub CallNonvirtualObjectMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jobject,
    >,
    pub CallNonvirtualObjectMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jobject,
    >,

    pub CallNonvirtualBooleanMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jboolean,
    >,
    pub CallNonvirtualBooleanMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jboolean,
    >,
    pub CallNonvirtualBooleanMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jboolean,
    >,

    pub CallNonvirtualByteMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jbyte,
    >,
    pub CallNonvirtualByteMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jbyte,
    >,
    pub CallNonvirtualByteMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jbyte,
    >,

    pub CallNonvirtualCharMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jchar,
    >,
    pub CallNonvirtualCharMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jchar,
    >,
    pub CallNonvirtualCharMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jchar,
    >,

    pub CallNonvirtualShortMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jshort,
    >,
    pub CallNonvirtualShortMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jshort,
    >,
    pub CallNonvirtualShortMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jshort,
    >,

    pub CallNonvirtualIntMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jint,
    >,
    pub CallNonvirtualIntMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jint,
    >,
    pub CallNonvirtualIntMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jint,
    >,

    pub CallNonvirtualLongMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jlong,
    >,
    pub CallNonvirtualLongMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jlong,
    >,
    pub CallNonvirtualLongMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jlong,
    >,

    pub CallNonvirtualFloatMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jfloat,
    >,
    pub CallNonvirtualFloatMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jfloat,
    >,
    pub CallNonvirtualFloatMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jfloat,
    >,

    pub CallNonvirtualDoubleMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jdouble,
    >,
    pub CallNonvirtualDoubleMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jdouble,
    >,
    pub CallNonvirtualDoubleMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jdouble,
    >,

    pub CallNonvirtualVoidMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ),
    >,
    pub CallNonvirtualVoidMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ),
    >,
    pub CallNonvirtualVoidMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            obj: jobject,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ),
    >,

    // ── Field access (index 94) ─────────────────────────────────────
    pub GetFieldID: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            name: *const c_char,
            sig: *const c_char,
        ) -> jfieldID,
    >,

    // Get<type>Field (indices 95..103)
    pub GetObjectField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jobject>,
    pub GetBooleanField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jboolean>,
    pub GetByteField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jbyte>,
    pub GetCharField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jchar>,
    pub GetShortField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jshort>,
    pub GetIntField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jint>,
    pub GetLongField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jlong>,
    pub GetFloatField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jfloat>,
    pub GetDoubleField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID) -> jdouble>,

    // Set<type>Field (indices 104..112)
    pub SetObjectField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jobject),
    >,
    pub SetBooleanField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jboolean),
    >,
    pub SetByteField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jbyte),
    >,
    pub SetCharField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jchar),
    >,
    pub SetShortField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jshort),
    >,
    pub SetIntField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jint),
    >,
    pub SetLongField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jlong),
    >,
    pub SetFloatField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jfloat),
    >,
    pub SetDoubleField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject, fieldID: jfieldID, value: jdouble),
    >,

    // ── GetStaticMethodID (index 113) ────────────────────────────────
    pub GetStaticMethodID: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            name: *const c_char,
            sig: *const c_char,
        ) -> jmethodID,
    >,

    // CallStatic<type>Method (indices 114..143) — object/boolean/byte/char/short/int/long/float/double/void × 3
    pub CallStaticObjectMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jobject,
    >,
    pub CallStaticObjectMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jobject,
    >,
    pub CallStaticObjectMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jobject,
    >,

    pub CallStaticBooleanMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jboolean,
    >,
    pub CallStaticBooleanMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jboolean,
    >,
    pub CallStaticBooleanMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jboolean,
    >,

    pub CallStaticByteMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jbyte,
    >,
    pub CallStaticByteMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jbyte,
    >,
    pub CallStaticByteMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jbyte,
    >,

    pub CallStaticCharMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jchar,
    >,
    pub CallStaticCharMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jchar,
    >,
    pub CallStaticCharMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jchar,
    >,

    pub CallStaticShortMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jshort,
    >,
    pub CallStaticShortMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jshort,
    >,
    pub CallStaticShortMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jshort,
    >,

    pub CallStaticIntMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jint,
    >,
    pub CallStaticIntMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jint,
    >,
    pub CallStaticIntMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jint,
    >,

    pub CallStaticLongMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jlong,
    >,
    pub CallStaticLongMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jlong,
    >,
    pub CallStaticLongMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jlong,
    >,

    pub CallStaticFloatMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jfloat,
    >,
    pub CallStaticFloatMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jfloat,
    >,
    pub CallStaticFloatMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jfloat,
    >,

    pub CallStaticDoubleMethod: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            ...
        ) -> jdouble,
    >,
    pub CallStaticDoubleMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ) -> jdouble,
    >,
    pub CallStaticDoubleMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ) -> jdouble,
    >,

    pub CallStaticVoidMethod: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, methodID: jmethodID, ...),
    >,
    pub CallStaticVoidMethodV: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *mut std::ffi::VaList,
        ),
    >,
    pub CallStaticVoidMethodA: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methodID: jmethodID,
            args: *const jvalue,
        ),
    >,

    // ── GetStaticFieldID (index 144) ─────────────────────────────────
    pub GetStaticFieldID: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            name: *const c_char,
            sig: *const c_char,
        ) -> jfieldID,
    >,

    // GetStatic<type>Field (indices 145..153)
    pub GetStaticObjectField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jobject>,
    pub GetStaticBooleanField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jboolean>,
    pub GetStaticByteField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jbyte>,
    pub GetStaticCharField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jchar>,
    pub GetStaticShortField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jshort>,
    pub GetStaticIntField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jint>,
    pub GetStaticLongField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jlong>,
    pub GetStaticFloatField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jfloat>,
    pub GetStaticDoubleField:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID) -> jdouble>,

    // SetStatic<type>Field (indices 154..162)
    pub SetStaticObjectField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jobject),
    >,
    pub SetStaticBooleanField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jboolean),
    >,
    pub SetStaticByteField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jbyte),
    >,
    pub SetStaticCharField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jchar),
    >,
    pub SetStaticShortField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jshort),
    >,
    pub SetStaticIntField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jint),
    >,
    pub SetStaticLongField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jlong),
    >,
    pub SetStaticFloatField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jfloat),
    >,
    pub SetStaticDoubleField: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass, fieldID: jfieldID, value: jdouble),
    >,

    // ── String operations (indices 163..170) ─────────────────────────
    pub NewString: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, unicodeChars: *const jchar, len: jsize) -> jstring,
    >,
    pub GetStringLength:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, string: jstring) -> jsize>,
    pub GetStringChars: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, string: jstring, isCopy: *mut jboolean) -> *const jchar,
    >,
    pub ReleaseStringChars:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, string: jstring, chars: *const jchar)>,
    pub NewStringUTF:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, bytes: *const c_char) -> jstring>,
    pub GetStringUTFLength:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, string: jstring) -> jsize>,
    pub GetStringUTFChars: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, string: jstring, isCopy: *mut jboolean) -> *const c_char,
    >,
    pub ReleaseStringUTFChars:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, string: jstring, utf: *const c_char)>,

    // ── Array operations (indices 171..174) ──────────────────────────
    pub GetArrayLength:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, array: jarray) -> jsize>,
    pub NewObjectArray: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            length: jsize,
            elementClass: jclass,
            initialElement: jobject,
        ) -> jobjectArray,
    >,
    pub GetObjectArrayElement:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, array: jobjectArray, index: jsize) -> jobject>,
    pub SetObjectArrayElement: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jobjectArray, index: jsize, value: jobject),
    >,

    // New<Primitive>Array (indices 175..182)
    pub NewBooleanArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jbooleanArray>,
    pub NewByteArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jbyteArray>,
    pub NewCharArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jcharArray>,
    pub NewShortArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jshortArray>,
    pub NewIntArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jintArray>,
    pub NewLongArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jlongArray>,
    pub NewFloatArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jfloatArray>,
    pub NewDoubleArray:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, len: jsize) -> jdoubleArray>,

    // Get<Primitive>ArrayElements (indices 183..190)
    pub GetBooleanArrayElements: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jbooleanArray,
            isCopy: *mut jboolean,
        ) -> *mut jboolean,
    >,
    pub GetByteArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jbyteArray, isCopy: *mut jboolean) -> *mut jbyte,
    >,
    pub GetCharArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jcharArray, isCopy: *mut jboolean) -> *mut jchar,
    >,
    pub GetShortArrayElements: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jshortArray,
            isCopy: *mut jboolean,
        ) -> *mut jshort,
    >,
    pub GetIntArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jintArray, isCopy: *mut jboolean) -> *mut jint,
    >,
    pub GetLongArrayElements: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jlongArray,
            isCopy: *mut jboolean,
        ) -> *mut jlong,
    >,
    pub GetFloatArrayElements: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jfloatArray,
            isCopy: *mut jboolean,
        ) -> *mut jfloat,
    >,
    pub GetDoubleArrayElements: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jdoubleArray,
            isCopy: *mut jboolean,
        ) -> *mut jdouble,
    >,

    // Release<Primitive>ArrayElements (indices 191..198)
    pub ReleaseBooleanArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jbooleanArray, elems: *mut jboolean, mode: jint),
    >,
    pub ReleaseByteArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jbyteArray, elems: *mut jbyte, mode: jint),
    >,
    pub ReleaseCharArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jcharArray, elems: *mut jchar, mode: jint),
    >,
    pub ReleaseShortArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jshortArray, elems: *mut jshort, mode: jint),
    >,
    pub ReleaseIntArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jintArray, elems: *mut jint, mode: jint),
    >,
    pub ReleaseLongArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jlongArray, elems: *mut jlong, mode: jint),
    >,
    pub ReleaseFloatArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jfloatArray, elems: *mut jfloat, mode: jint),
    >,
    pub ReleaseDoubleArrayElements: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jdoubleArray, elems: *mut jdouble, mode: jint),
    >,

    // Get<Primitive>ArrayRegion (indices 199..206)
    pub GetBooleanArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jbooleanArray,
            start: jsize,
            len: jsize,
            buf: *mut jboolean,
        ),
    >,
    pub GetByteArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jbyteArray,
            start: jsize,
            len: jsize,
            buf: *mut jbyte,
        ),
    >,
    pub GetCharArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jcharArray,
            start: jsize,
            len: jsize,
            buf: *mut jchar,
        ),
    >,
    pub GetShortArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jshortArray,
            start: jsize,
            len: jsize,
            buf: *mut jshort,
        ),
    >,
    pub GetIntArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jintArray,
            start: jsize,
            len: jsize,
            buf: *mut jint,
        ),
    >,
    pub GetLongArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jlongArray,
            start: jsize,
            len: jsize,
            buf: *mut jlong,
        ),
    >,
    pub GetFloatArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jfloatArray,
            start: jsize,
            len: jsize,
            buf: *mut jfloat,
        ),
    >,
    pub GetDoubleArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jdoubleArray,
            start: jsize,
            len: jsize,
            buf: *mut jdouble,
        ),
    >,

    // Set<Primitive>ArrayRegion (indices 207..214)
    pub SetBooleanArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jbooleanArray,
            start: jsize,
            len: jsize,
            buf: *const jboolean,
        ),
    >,
    pub SetByteArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jbyteArray,
            start: jsize,
            len: jsize,
            buf: *const jbyte,
        ),
    >,
    pub SetCharArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jcharArray,
            start: jsize,
            len: jsize,
            buf: *const jchar,
        ),
    >,
    pub SetShortArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jshortArray,
            start: jsize,
            len: jsize,
            buf: *const jshort,
        ),
    >,
    pub SetIntArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jintArray,
            start: jsize,
            len: jsize,
            buf: *const jint,
        ),
    >,
    pub SetLongArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jlongArray,
            start: jsize,
            len: jsize,
            buf: *const jlong,
        ),
    >,
    pub SetFloatArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jfloatArray,
            start: jsize,
            len: jsize,
            buf: *const jfloat,
        ),
    >,
    pub SetDoubleArrayRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jdoubleArray,
            start: jsize,
            len: jsize,
            buf: *const jdouble,
        ),
    >,

    // ── RegisterNatives / UnregisterNatives (indices 215..216) ──────
    pub RegisterNatives: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            clazz: jclass,
            methods: *const JNINativeMethod,
            nMethods: jint,
        ) -> jint,
    >,
    pub UnregisterNatives:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, clazz: jclass) -> jint>,

    // ── Monitor operations (indices 217..218) ───────────────────────
    pub MonitorEnter:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jint>,
    pub MonitorExit:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jint>,

    // ── GetJavaVM (index 219) ────────────────────────────────────────
    pub GetJavaVM: Option<unsafe extern "C" fn(env: JNIEnvPtr, vm: *mut JavaVMPtr) -> jint>,

    // ── String region operations (indices 220..221) ──────────────────
    pub GetStringRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            str_: jstring,
            start: jsize,
            len: jsize,
            buf: *mut jchar,
        ),
    >,
    pub GetStringUTFRegion: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            str_: jstring,
            start: jsize,
            len: jsize,
            buf: *mut c_char,
        ),
    >,

    // ── Primitive array critical (indices 222..223) ──────────────────
    pub GetPrimitiveArrayCritical: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            array: jarray,
            isCopy: *mut jboolean,
        ) -> *mut c_void,
    >,
    pub ReleasePrimitiveArrayCritical: Option<
        unsafe extern "C" fn(env: JNIEnvPtr, array: jarray, carray: *mut c_void, mode: jint),
    >,

    // ── String critical (indices 224..225) ───────────────────────────
    pub GetStringCritical: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            string: jstring,
            isCopy: *mut jboolean,
        ) -> *const jchar,
    >,
    pub ReleaseStringCritical:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, string: jstring, carray: *const jchar)>,

    // ── Weak global refs (indices 226..227) ─────────────────────────
    pub NewWeakGlobalRef:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jweak>,
    pub DeleteWeakGlobalRef:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jweak)>,

    // ── ExceptionCheck (index 228) ──────────────────────────────────
    pub ExceptionCheck:
        Option<unsafe extern "C" fn(env: JNIEnvPtr) -> jboolean>,

    // ── NIO direct buffer (indices 229..231) ────────────────────────
    pub NewDirectByteBuffer: Option<
        unsafe extern "C" fn(
            env: JNIEnvPtr,
            address: *mut c_void,
            capacity: jlong,
        ) -> jobject,
    >,
    pub GetDirectBufferAddress:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, buf: jobject) -> *mut c_void>,
    pub GetDirectBufferCapacity:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, buf: jobject) -> jlong>,

    // ── GetObjectRefType (index 232) ────────────────────────────────
    pub GetObjectRefType:
        Option<unsafe extern "C" fn(env: JNIEnvPtr, obj: jobject) -> jobjectRefType>,
}

// ---------------------------------------------------------------------------
// JavaVM function table (struct JNIInvokeInterface)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct JNIInvokeFunctions {
    pub reserved0: *mut c_void,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,

    pub DestroyJavaVM: Option<unsafe extern "C" fn(vm: JavaVMPtr) -> jint>,
    pub AttachCurrentThread:
        Option<unsafe extern "C" fn(vm: JavaVMPtr, penv: *mut JNIEnvPtr, args: *mut c_void) -> jint>,
    pub DetachCurrentThread: Option<unsafe extern "C" fn(vm: JavaVMPtr) -> jint>,
    pub GetEnv:
        Option<unsafe extern "C" fn(vm: JavaVMPtr, penv: *mut *mut c_void, version: jint) -> jint>,
    pub AttachCurrentThreadAsDaemon:
        Option<unsafe extern "C" fn(vm: JavaVMPtr, penv: *mut JNIEnvPtr, args: *mut c_void) -> jint>,
}