// SPDX-License-Identifier: MIT
//
// JNI Bridge — A JNI compatibility shim for loading libros.so directly.
//
// Export JNI_OnLoad and provide a fully-populated JNI function table that
// libroblox.so can call through during initialization.

#![allow(non_camel_case_types, dead_code, unused_variables)]

mod ffi_table;
mod jvm_state;
mod types;

use ffi_table::*;
use jvm_state::*;
use types::*;

use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Simple logging macro (uses tracing when available, fallback to stderr)
// ---------------------------------------------------------------------------

macro_rules! jni_trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::debug!($($arg)*);
        #[cfg(not(feature = "tracing"))]
        {
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "[jni-bridge] {}", format_args!($($arg)*));
        }
    };
}

// ---------------------------------------------------------------------------
// Global static tables — these MUST live for the lifetime of the process.
// ---------------------------------------------------------------------------

/// Static JNI function table (JNINativeInterface).  Every slot is filled
/// so libroblox.so doesn't crash on any JNI call.
static mut JNI_ENV_FUNCTIONS: JNINativeFunctions = JNINativeFunctions {
    // Indices 0..3: reserved (null)
    reserved0: std::ptr::null_mut(),
    reserved1: std::ptr::null_mut(),
    reserved2: std::ptr::null_mut(),
    reserved3: std::ptr::null_mut(),

    GetVersion: Some(jni_GetVersion),
    DefineClass: Some(jni_DefineClass),
    FindClass: Some(jni_FindClass),

    FromReflectedMethod: Some(jni_FromReflectedMethod),
    FromReflectedField: Some(jni_FromReflectedField),
    ToReflectedMethod: Some(jni_ToReflectedMethod),

    GetSuperclass: Some(jni_GetSuperclass),
    IsAssignableFrom: Some(jni_IsAssignableFrom),
    ToReflectedField: Some(jni_ToReflectedField),

    Throw: Some(jni_Throw),
    ThrowNew: Some(jni_ThrowNew),
    ExceptionOccurred: Some(jni_ExceptionOccurred),
    ExceptionDescribe: Some(jni_ExceptionDescribe),
    ExceptionClear: Some(jni_ExceptionClear),
    FatalError: Some(jni_FatalError),

    PushLocalFrame: Some(jni_PushLocalFrame),
    PopLocalFrame: Some(jni_PopLocalFrame),

    NewGlobalRef: Some(jni_NewGlobalRef),
    DeleteGlobalRef: Some(jni_DeleteGlobalRef),
    DeleteLocalRef: Some(jni_DeleteLocalRef),
    IsSameObject: Some(jni_IsSameObject),
    NewLocalRef: Some(jni_NewLocalRef),
    EnsureLocalCapacity: Some(jni_EnsureLocalCapacity),

    AllocObject: Some(jni_AllocObject),
    NewObject: Some(jni_NewObject),
    NewObjectV: Some(jni_NewObjectV),
    NewObjectA: Some(jni_NewObjectA),
    GetObjectClass: Some(jni_GetObjectClass),
    IsInstanceOf: Some(jni_IsInstanceOf),
    GetMethodID: Some(jni_GetMethodID),

    // Call<type>Method
    CallObjectMethod: Some(jni_CallObjectMethod),
    CallObjectMethodV: Some(jni_CallObjectMethodV),
    CallObjectMethodA: Some(jni_CallObjectMethodA),
    CallBooleanMethod: Some(jni_CallBooleanMethod),
    CallBooleanMethodV: Some(jni_CallBooleanMethodV),
    CallBooleanMethodA: Some(jni_CallBooleanMethodA),
    CallByteMethod: Some(jni_CallByteMethod),
    CallByteMethodV: Some(jni_CallByteMethodV),
    CallByteMethodA: Some(jni_CallByteMethodA),
    CallCharMethod: Some(jni_CallCharMethod),
    CallCharMethodV: Some(jni_CallCharMethodV),
    CallCharMethodA: Some(jni_CallCharMethodA),
    CallShortMethod: Some(jni_CallShortMethod),
    CallShortMethodV: Some(jni_CallShortMethodV),
    CallShortMethodA: Some(jni_CallShortMethodA),
    CallIntMethod: Some(jni_CallIntMethod),
    CallIntMethodV: Some(jni_CallIntMethodV),
    CallIntMethodA: Some(jni_CallIntMethodA),
    CallLongMethod: Some(jni_CallLongMethod),
    CallLongMethodV: Some(jni_CallLongMethodV),
    CallLongMethodA: Some(jni_CallLongMethodA),
    CallFloatMethod: Some(jni_CallFloatMethod),
    CallFloatMethodV: Some(jni_CallFloatMethodV),
    CallFloatMethodA: Some(jni_CallFloatMethodA),
    CallDoubleMethod: Some(jni_CallDoubleMethod),
    CallDoubleMethodV: Some(jni_CallDoubleMethodV),
    CallDoubleMethodA: Some(jni_CallDoubleMethodA),
    CallVoidMethod: Some(jni_CallVoidMethod),
    CallVoidMethodV: Some(jni_CallVoidMethodV),
    CallVoidMethodA: Some(jni_CallVoidMethodA),

    // CallNonvirtual<type>Method
    CallNonvirtualObjectMethod: Some(jni_CallNonvirtualObjectMethod),
    CallNonvirtualObjectMethodV: Some(jni_CallNonvirtualObjectMethodV),
    CallNonvirtualObjectMethodA: Some(jni_CallNonvirtualObjectMethodA),
    CallNonvirtualBooleanMethod: Some(jni_CallNonvirtualBooleanMethod),
    CallNonvirtualBooleanMethodV: Some(jni_CallNonvirtualBooleanMethodV),
    CallNonvirtualBooleanMethodA: Some(jni_CallNonvirtualBooleanMethodA),
    CallNonvirtualByteMethod: Some(jni_CallNonvirtualByteMethod),
    CallNonvirtualByteMethodV: Some(jni_CallNonvirtualByteMethodV),
    CallNonvirtualByteMethodA: Some(jni_CallNonvirtualByteMethodA),
    CallNonvirtualCharMethod: Some(jni_CallNonvirtualCharMethod),
    CallNonvirtualCharMethodV: Some(jni_CallNonvirtualCharMethodV),
    CallNonvirtualCharMethodA: Some(jni_CallNonvirtualCharMethodA),
    CallNonvirtualShortMethod: Some(jni_CallNonvirtualShortMethod),
    CallNonvirtualShortMethodV: Some(jni_CallNonvirtualShortMethodV),
    CallNonvirtualShortMethodA: Some(jni_CallNonvirtualShortMethodA),
    CallNonvirtualIntMethod: Some(jni_CallNonvirtualIntMethod),
    CallNonvirtualIntMethodV: Some(jni_CallNonvirtualIntMethodV),
    CallNonvirtualIntMethodA: Some(jni_CallNonvirtualIntMethodA),
    CallNonvirtualLongMethod: Some(jni_CallNonvirtualLongMethod),
    CallNonvirtualLongMethodV: Some(jni_CallNonvirtualLongMethodV),
    CallNonvirtualLongMethodA: Some(jni_CallNonvirtualLongMethodA),
    CallNonvirtualFloatMethod: Some(jni_CallNonvirtualFloatMethod),
    CallNonvirtualFloatMethodV: Some(jni_CallNonvirtualFloatMethodV),
    CallNonvirtualFloatMethodA: Some(jni_CallNonvirtualFloatMethodA),
    CallNonvirtualDoubleMethod: Some(jni_CallNonvirtualDoubleMethod),
    CallNonvirtualDoubleMethodV: Some(jni_CallNonvirtualDoubleMethodV),
    CallNonvirtualDoubleMethodA: Some(jni_CallNonvirtualDoubleMethodA),
    CallNonvirtualVoidMethod: Some(jni_CallNonvirtualVoidMethod),
    CallNonvirtualVoidMethodV: Some(jni_CallNonvirtualVoidMethodV),
    CallNonvirtualVoidMethodA: Some(jni_CallNonvirtualVoidMethodA),

    GetFieldID: Some(jni_GetFieldID),
    GetObjectField: Some(jni_GetObjectField),
    GetBooleanField: Some(jni_GetBooleanField),
    GetByteField: Some(jni_GetByteField),
    GetCharField: Some(jni_GetCharField),
    GetShortField: Some(jni_GetShortField),
    GetIntField: Some(jni_GetIntField),
    GetLongField: Some(jni_GetLongField),
    GetFloatField: Some(jni_GetFloatField),
    GetDoubleField: Some(jni_GetDoubleField),

    SetObjectField: Some(jni_SetObjectField),
    SetBooleanField: Some(jni_SetBooleanField),
    SetByteField: Some(jni_SetByteField),
    SetCharField: Some(jni_SetCharField),
    SetShortField: Some(jni_SetShortField),
    SetIntField: Some(jni_SetIntField),
    SetLongField: Some(jni_SetLongField),
    SetFloatField: Some(jni_SetFloatField),
    SetDoubleField: Some(jni_SetDoubleField),

    GetStaticMethodID: Some(jni_GetStaticMethodID),

    CallStaticObjectMethod: Some(jni_CallStaticObjectMethod),
    CallStaticObjectMethodV: Some(jni_CallStaticObjectMethodV),
    CallStaticObjectMethodA: Some(jni_CallStaticObjectMethodA),
    CallStaticBooleanMethod: Some(jni_CallStaticBooleanMethod),
    CallStaticBooleanMethodV: Some(jni_CallStaticBooleanMethodV),
    CallStaticBooleanMethodA: Some(jni_CallStaticBooleanMethodA),
    CallStaticByteMethod: Some(jni_CallStaticByteMethod),
    CallStaticByteMethodV: Some(jni_CallStaticByteMethodV),
    CallStaticByteMethodA: Some(jni_CallStaticByteMethodA),
    CallStaticCharMethod: Some(jni_CallStaticCharMethod),
    CallStaticCharMethodV: Some(jni_CallStaticCharMethodV),
    CallStaticCharMethodA: Some(jni_CallStaticCharMethodA),
    CallStaticShortMethod: Some(jni_CallStaticShortMethod),
    CallStaticShortMethodV: Some(jni_CallStaticShortMethodV),
    CallStaticShortMethodA: Some(jni_CallStaticShortMethodA),
    CallStaticIntMethod: Some(jni_CallStaticIntMethod),
    CallStaticIntMethodV: Some(jni_CallStaticIntMethodV),
    CallStaticIntMethodA: Some(jni_CallStaticIntMethodA),
    CallStaticLongMethod: Some(jni_CallStaticLongMethod),
    CallStaticLongMethodV: Some(jni_CallStaticLongMethodV),
    CallStaticLongMethodA: Some(jni_CallStaticLongMethodA),
    CallStaticFloatMethod: Some(jni_CallStaticFloatMethod),
    CallStaticFloatMethodV: Some(jni_CallStaticFloatMethodV),
    CallStaticFloatMethodA: Some(jni_CallStaticFloatMethodA),
    CallStaticDoubleMethod: Some(jni_CallStaticDoubleMethod),
    CallStaticDoubleMethodV: Some(jni_CallStaticDoubleMethodV),
    CallStaticDoubleMethodA: Some(jni_CallStaticDoubleMethodA),
    CallStaticVoidMethod: Some(jni_CallStaticVoidMethod),
    CallStaticVoidMethodV: Some(jni_CallStaticVoidMethodV),
    CallStaticVoidMethodA: Some(jni_CallStaticVoidMethodA),

    GetStaticFieldID: Some(jni_GetStaticFieldID),
    GetStaticObjectField: Some(jni_GetStaticObjectField),
    GetStaticBooleanField: Some(jni_GetStaticBooleanField),
    GetStaticByteField: Some(jni_GetStaticByteField),
    GetStaticCharField: Some(jni_GetStaticCharField),
    GetStaticShortField: Some(jni_GetStaticShortField),
    GetStaticIntField: Some(jni_GetStaticIntField),
    GetStaticLongField: Some(jni_GetStaticLongField),
    GetStaticFloatField: Some(jni_GetStaticFloatField),
    GetStaticDoubleField: Some(jni_GetStaticDoubleField),

    SetStaticObjectField: Some(jni_SetStaticObjectField),
    SetStaticBooleanField: Some(jni_SetStaticBooleanField),
    SetStaticByteField: Some(jni_SetStaticByteField),
    SetStaticCharField: Some(jni_SetStaticCharField),
    SetStaticShortField: Some(jni_SetStaticShortField),
    SetStaticIntField: Some(jni_SetStaticIntField),
    SetStaticLongField: Some(jni_SetStaticLongField),
    SetStaticFloatField: Some(jni_SetStaticFloatField),
    SetStaticDoubleField: Some(jni_SetStaticDoubleField),

    NewString: Some(jni_NewString),
    GetStringLength: Some(jni_GetStringLength),
    GetStringChars: Some(jni_GetStringChars),
    ReleaseStringChars: Some(jni_ReleaseStringChars),
    NewStringUTF: Some(jni_NewStringUTF),
    GetStringUTFLength: Some(jni_GetStringUTFLength),
    GetStringUTFChars: Some(jni_GetStringUTFChars),
    ReleaseStringUTFChars: Some(jni_ReleaseStringUTFChars),

    GetArrayLength: Some(jni_GetArrayLength),
    NewObjectArray: Some(jni_NewObjectArray),
    GetObjectArrayElement: Some(jni_GetObjectArrayElement),
    SetObjectArrayElement: Some(jni_SetObjectArrayElement),

    NewBooleanArray: Some(jni_NewBooleanArray),
    NewByteArray: Some(jni_NewByteArray),
    NewCharArray: Some(jni_NewCharArray),
    NewShortArray: Some(jni_NewShortArray),
    NewIntArray: Some(jni_NewIntArray),
    NewLongArray: Some(jni_NewLongArray),
    NewFloatArray: Some(jni_NewFloatArray),
    NewDoubleArray: Some(jni_NewDoubleArray),

    GetBooleanArrayElements: Some(jni_GetBooleanArrayElements),
    GetByteArrayElements: Some(jni_GetByteArrayElements),
    GetCharArrayElements: Some(jni_GetCharArrayElements),
    GetShortArrayElements: Some(jni_GetShortArrayElements),
    GetIntArrayElements: Some(jni_GetIntArrayElements),
    GetLongArrayElements: Some(jni_GetLongArrayElements),
    GetFloatArrayElements: Some(jni_GetFloatArrayElements),
    GetDoubleArrayElements: Some(jni_GetDoubleArrayElements),

    ReleaseBooleanArrayElements: Some(jni_ReleaseBooleanArrayElements),
    ReleaseByteArrayElements: Some(jni_ReleaseByteArrayElements),
    ReleaseCharArrayElements: Some(jni_ReleaseCharArrayElements),
    ReleaseShortArrayElements: Some(jni_ReleaseShortArrayElements),
    ReleaseIntArrayElements: Some(jni_ReleaseIntArrayElements),
    ReleaseLongArrayElements: Some(jni_ReleaseLongArrayElements),
    ReleaseFloatArrayElements: Some(jni_ReleaseFloatArrayElements),
    ReleaseDoubleArrayElements: Some(jni_ReleaseDoubleArrayElements),

    GetBooleanArrayRegion: Some(jni_GetBooleanArrayRegion),
    GetByteArrayRegion: Some(jni_GetByteArrayRegion),
    GetCharArrayRegion: Some(jni_GetCharArrayRegion),
    GetShortArrayRegion: Some(jni_GetShortArrayRegion),
    GetIntArrayRegion: Some(jni_GetIntArrayRegion),
    GetLongArrayRegion: Some(jni_GetLongArrayRegion),
    GetFloatArrayRegion: Some(jni_GetFloatArrayRegion),
    GetDoubleArrayRegion: Some(jni_GetDoubleArrayRegion),

    SetBooleanArrayRegion: Some(jni_SetBooleanArrayRegion),
    SetByteArrayRegion: Some(jni_SetByteArrayRegion),
    SetCharArrayRegion: Some(jni_SetCharArrayRegion),
    SetShortArrayRegion: Some(jni_SetShortArrayRegion),
    SetIntArrayRegion: Some(jni_SetIntArrayRegion),
    SetLongArrayRegion: Some(jni_SetLongArrayRegion),
    SetFloatArrayRegion: Some(jni_SetFloatArrayRegion),
    SetDoubleArrayRegion: Some(jni_SetDoubleArrayRegion),

    RegisterNatives: Some(jni_RegisterNatives),
    UnregisterNatives: Some(jni_UnregisterNatives),
    MonitorEnter: Some(jni_MonitorEnter),
    MonitorExit: Some(jni_MonitorExit),
    GetJavaVM: Some(jni_GetJavaVM),

    GetStringRegion: Some(jni_GetStringRegion),
    GetStringUTFRegion: Some(jni_GetStringUTFRegion),

    GetPrimitiveArrayCritical: Some(jni_GetPrimitiveArrayCritical),
    ReleasePrimitiveArrayCritical: Some(jni_ReleasePrimitiveArrayCritical),

    GetStringCritical: Some(jni_GetStringCritical),
    ReleaseStringCritical: Some(jni_ReleaseStringCritical),

    NewWeakGlobalRef: Some(jni_NewWeakGlobalRef),
    DeleteWeakGlobalRef: Some(jni_DeleteWeakGlobalRef),

    ExceptionCheck: Some(jni_ExceptionCheck),

    NewDirectByteBuffer: Some(jni_NewDirectByteBuffer),
    GetDirectBufferAddress: Some(jni_GetDirectBufferAddress),
    GetDirectBufferCapacity: Some(jni_GetDirectBufferCapacity),

    GetObjectRefType: Some(jni_GetObjectRefType),
};

/// Static JavaVM function table (JNIInvokeInterface).
static mut JAVA_VM_FUNCTIONS: JNIInvokeFunctions = JNIInvokeFunctions {
    reserved0: std::ptr::null_mut(),
    reserved1: std::ptr::null_mut(),
    reserved2: std::ptr::null_mut(),
    DestroyJavaVM: Some(jvm_DestroyJavaVM),
    AttachCurrentThread: Some(jvm_AttachCurrentThread),
    DetachCurrentThread: Some(jvm_DetachCurrentThread),
    GetEnv: Some(jvm_GetEnv),
    AttachCurrentThreadAsDaemon: Some(jvm_AttachCurrentThreadAsDaemon),
};

// ---------------------------------------------------------------------------
// The single JNIEnv and JavaVM wrappers
// ---------------------------------------------------------------------------

/// The single JNIEnvWrapper that we hand out.
static mut JNI_ENV_WRAPPER: JNIEnvWrapper = JNIEnvWrapper {
    functions: std::ptr::null(), // filled at init
};

/// The single JavaVMWrapper that we hand out.
static mut JAVA_VM_WRAPPER: JavaVMWrapper = JavaVMWrapper {
    functions: std::ptr::null(), // filled at init
};

/// Initializes the global wrappers.  Called once from JNI_OnLoad.
unsafe fn init_global_tables() {
    JNI_ENV_WRAPPER.functions = &JNI_ENV_FUNCTIONS;
    JAVA_VM_WRAPPER.functions = &JAVA_VM_FUNCTIONS;

    let mut state = GLOBAL_JVM.lock().unwrap();
    state.java_vm = Some(&mut JAVA_VM_WRAPPER as *mut JavaVMWrapper);
    state.jni_env = Some(&mut JNI_ENV_WRAPPER as *mut JNIEnvWrapper);
}

// =========================================================================
// JNIEnv function implementations
// =========================================================================

// --- Version (index 4) ---

unsafe extern "C" fn jni_GetVersion(env: JNIEnvPtr) -> jint {
    JNI_VERSION_1_6
}

// --- Class operations (indices 5, 6, 10, 11) ---

unsafe extern "C" fn jni_DefineClass(
    env: JNIEnvPtr,
    name: *const c_char,
    loader: jobject,
    buf: *const jbyte,
    bufLen: jsize,
) -> jclass {
    let name_str = cstr_to_str(name);
    // Register the class and return an opaque handle.
    let mut state = GLOBAL_JVM.lock().unwrap();
    let cls = state.find_or_create_class(name_str);
    jni_trace!("DefineClass({}) = {:p}", name_str, cls);
    cls
}

unsafe extern "C" fn jni_FindClass(env: JNIEnvPtr, name: *const c_char) -> jclass {
    let name_str = cstr_to_str(name);
    let mut state = GLOBAL_JVM.lock().unwrap();
    let cls = state.find_or_create_class(name_str);
    jni_trace!("FindClass({}) = {:p}", name_str, cls);
    cls
}

unsafe extern "C" fn jni_GetSuperclass(env: JNIEnvPtr, clazz: jclass) -> jclass {
    // libroblox.so probably doesn't care about the true hierarchy.
    // Return the class itself as a fake "Object" superclass.
    let mut state = GLOBAL_JVM.lock().unwrap();
    let cls = state.find_or_create_class("java/lang/Object");
    jni_trace!("GetSuperclass({:p}) = {:p}", clazz, cls);
    cls
}

unsafe extern "C" fn jni_IsAssignableFrom(
    env: JNIEnvPtr,
    clazz1: jclass,
    clazz2: jclass,
) -> jboolean {
    JNI_TRUE
}

// --- Reflection (indices 7..9, 12) ---

unsafe extern "C" fn jni_FromReflectedMethod(env: JNIEnvPtr, method: jobject) -> jmethodID {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_FromReflectedField(env: JNIEnvPtr, field: jobject) -> jfieldID {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_ToReflectedMethod(
    env: JNIEnvPtr,
    cls: jclass,
    methodID: jmethodID,
    isStatic: jboolean,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_ToReflectedField(
    env: JNIEnvPtr,
    cls: jclass,
    fieldID: jfieldID,
    isStatic: jboolean,
) -> jobject {
    std::ptr::null_mut()
}

// --- Exceptions (indices 13..18) ---

unsafe extern "C" fn jni_Throw(env: JNIEnvPtr, obj: jthrowable) -> jint {
    jni_trace!("Throw called");
    JNI_OK
}

unsafe extern "C" fn jni_ThrowNew(
    env: JNIEnvPtr,
    clazz: jclass,
    message: *const c_char,
) -> jint {
    let msg = cstr_to_str(message);
    jni_trace!("ThrowNew({})", msg);
    JNI_OK
}

unsafe extern "C" fn jni_ExceptionOccurred(env: JNIEnvPtr) -> jthrowable {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_ExceptionDescribe(env: JNIEnvPtr) {
    // no-op
}

unsafe extern "C" fn jni_ExceptionClear(env: JNIEnvPtr) {
    // no-op
}

unsafe extern "C" fn jni_FatalError(env: JNIEnvPtr, msg: *const c_char) {
    let msg_str = cstr_to_str(msg);
    jni_trace!("FatalError: {}", msg_str);
    // In a real JVM this would abort; for now we panic.
    panic!("JNI FatalError: {}", msg_str);
}

// --- Local reference frames (indices 19..20) ---

unsafe extern "C" fn jni_PushLocalFrame(env: JNIEnvPtr, capacity: jint) -> jint {
    JNI_OK
}

unsafe extern "C" fn jni_PopLocalFrame(env: JNIEnvPtr, result: jobject) -> jobject {
    result
}

// --- Global / local references (indices 21..26) ---

unsafe extern "C" fn jni_NewGlobalRef(env: JNIEnvPtr, obj: jobject) -> jobject {
    // Return the object as-is — this is a stub.
    // For a more complete implementation we would track refs.
    obj
}

unsafe extern "C" fn jni_DeleteGlobalRef(env: JNIEnvPtr, globalRef: jobject) {
    // no-op
}

unsafe extern "C" fn jni_DeleteLocalRef(env: JNIEnvPtr, localRef: jobject) {
    // no-op
}

unsafe extern "C" fn jni_IsSameObject(
    env: JNIEnvPtr,
    ref1: jobject,
    ref2: jobject,
) -> jboolean {
    if ref1 == ref2 { JNI_TRUE } else { JNI_FALSE }
}

unsafe extern "C" fn jni_NewLocalRef(env: JNIEnvPtr, ref_: jobject) -> jobject {
    ref_
}

unsafe extern "C" fn jni_EnsureLocalCapacity(env: JNIEnvPtr, capacity: jint) -> jint {
    JNI_OK
}

// --- Object operations (indices 27..32) ---

unsafe extern "C" fn jni_AllocObject(env: JNIEnvPtr, clazz: jclass) -> jobject {
    // Return a non-null opaque handle
    clazz
}

unsafe extern "C" fn jni_NewObject(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jobject {
    clazz
}

unsafe extern "C" fn jni_NewObjectV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jobject {
    clazz
}

unsafe extern "C" fn jni_NewObjectA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jobject {
    clazz
}

unsafe extern "C" fn jni_GetObjectClass(env: JNIEnvPtr, obj: jobject) -> jclass {
    // For stub purposes, just return the object as its own class.
    obj
}

unsafe extern "C" fn jni_IsInstanceOf(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
) -> jboolean {
    JNI_TRUE
}

// --- GetMethodID (index 33) ---

unsafe extern "C" fn jni_GetMethodID(
    env: JNIEnvPtr,
    clazz: jclass,
    name: *const c_char,
    sig: *const c_char,
) -> jmethodID {
    let name_str = cstr_to_str(name);
    let sig_str = cstr_to_str(sig);
    // We need the class name.  Since clazz is opaque, we'd need to track it.
    // For now, use a generic class key.
    let mut state = GLOBAL_JVM.lock().unwrap();
    let mid = state.find_or_create_method("", name_str, sig_str);
    jni_trace!("GetMethodID({}, {}) = {:p}", name_str, sig_str, mid);
    mid
}

// ── Call<type>Method (indices 34..63) ─────────────────────────────────────
// These all return zero / null and log the call.

unsafe extern "C" fn jni_CallObjectMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallObjectMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallObjectMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallBooleanMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallBooleanMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallBooleanMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallByteMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallByteMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallByteMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallCharMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallCharMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallCharMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallShortMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallShortMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallShortMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallIntMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jint {
    0
}

unsafe extern "C" fn jni_CallIntMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jint {
    0
}

unsafe extern "C" fn jni_CallIntMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jint {
    0
}

unsafe extern "C" fn jni_CallLongMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallLongMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallLongMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallFloatMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallFloatMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallFloatMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallDoubleMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallDoubleMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallDoubleMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallVoidMethod(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    ...
) {
}

unsafe extern "C" fn jni_CallVoidMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) {
}

unsafe extern "C" fn jni_CallVoidMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    methodID: jmethodID,
    args: *const jvalue,
) {
}

// ── CallNonvirtual<type>Method (indices 64..93) ───────────────────────────
// These all return zero/null.

unsafe extern "C" fn jni_CallNonvirtualObjectMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallNonvirtualObjectMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallNonvirtualObjectMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallNonvirtualBooleanMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallNonvirtualBooleanMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallNonvirtualBooleanMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallNonvirtualByteMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallNonvirtualByteMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallNonvirtualByteMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallNonvirtualCharMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallNonvirtualCharMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallNonvirtualCharMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallNonvirtualShortMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallNonvirtualShortMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallNonvirtualShortMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallNonvirtualIntMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jint {
    0
}

unsafe extern "C" fn jni_CallNonvirtualIntMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jint {
    0
}

unsafe extern "C" fn jni_CallNonvirtualIntMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jint {
    0
}

unsafe extern "C" fn jni_CallNonvirtualLongMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallNonvirtualLongMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallNonvirtualLongMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallNonvirtualFloatMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualFloatMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualFloatMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualDoubleMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualDoubleMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualDoubleMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallNonvirtualVoidMethod(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    ...
) {
}

unsafe extern "C" fn jni_CallNonvirtualVoidMethodV(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) {
}

unsafe extern "C" fn jni_CallNonvirtualVoidMethodA(
    env: JNIEnvPtr,
    obj: jobject,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) {
}

// ── GetFieldID (index 94) ─────────────────────────────────────────────────

unsafe extern "C" fn jni_GetFieldID(
    env: JNIEnvPtr,
    clazz: jclass,
    name: *const c_char,
    sig: *const c_char,
) -> jfieldID {
    let name_str = cstr_to_str(name);
    let sig_str = cstr_to_str(sig);
    let mut state = GLOBAL_JVM.lock().unwrap();
    let fid = state.find_or_create_field("", name_str, sig_str);
    jni_trace!("GetFieldID({}, {}) = {:p}", name_str, sig_str, fid);
    fid
}

// ── Get<type>Field (indices 95..103) ──────────────────────────────────────
// All return zero.

unsafe extern "C" fn jni_GetObjectField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jobject {
    std::ptr::null_mut()
}
unsafe extern "C" fn jni_GetBooleanField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jboolean {
    JNI_FALSE
}
unsafe extern "C" fn jni_GetByteField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jbyte {
    0
}
unsafe extern "C" fn jni_GetCharField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jchar {
    0
}
unsafe extern "C" fn jni_GetShortField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jshort {
    0
}
unsafe extern "C" fn jni_GetIntField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jint {
    0
}
unsafe extern "C" fn jni_GetLongField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jlong {
    0
}
unsafe extern "C" fn jni_GetFloatField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jfloat {
    0.0
}
unsafe extern "C" fn jni_GetDoubleField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
) -> jdouble {
    0.0
}

// ── Set<type>Field (indices 104..112) ─────────────────────────────────────

unsafe extern "C" fn jni_SetObjectField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jobject,
) {
}
unsafe extern "C" fn jni_SetBooleanField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jboolean,
) {
}
unsafe extern "C" fn jni_SetByteField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jbyte,
) {
}
unsafe extern "C" fn jni_SetCharField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jchar,
) {
}
unsafe extern "C" fn jni_SetShortField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jshort,
) {
}
unsafe extern "C" fn jni_SetIntField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jint,
) {
}
unsafe extern "C" fn jni_SetLongField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jlong,
) {
}
unsafe extern "C" fn jni_SetFloatField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jfloat,
) {
}
unsafe extern "C" fn jni_SetDoubleField(
    env: JNIEnvPtr,
    obj: jobject,
    fieldID: jfieldID,
    value: jdouble,
) {
}

// ── GetStaticMethodID (index 113) ─────────────────────────────────────────

unsafe extern "C" fn jni_GetStaticMethodID(
    env: JNIEnvPtr,
    clazz: jclass,
    name: *const c_char,
    sig: *const c_char,
) -> jmethodID {
    let name_str = cstr_to_str(name);
    let sig_str = cstr_to_str(sig);
    let mut state = GLOBAL_JVM.lock().unwrap();
    let mid = state.find_or_create_method("", name_str, sig_str);
    jni_trace!("GetStaticMethodID({}, {}) = {:p}", name_str, sig_str, mid);
    mid
}

// ── CallStatic<type>Method (indices 114..143) ─────────────────────────────
// All return zero.

unsafe extern "C" fn jni_CallStaticObjectMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jobject {
    std::ptr::null_mut()
}
unsafe extern "C" fn jni_CallStaticObjectMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jobject {
    std::ptr::null_mut()
}
unsafe extern "C" fn jni_CallStaticObjectMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_CallStaticBooleanMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jboolean {
    JNI_FALSE
}
unsafe extern "C" fn jni_CallStaticBooleanMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jboolean {
    JNI_FALSE
}
unsafe extern "C" fn jni_CallStaticBooleanMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jboolean {
    JNI_FALSE
}

unsafe extern "C" fn jni_CallStaticByteMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jbyte {
    0
}
unsafe extern "C" fn jni_CallStaticByteMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jbyte {
    0
}
unsafe extern "C" fn jni_CallStaticByteMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jbyte {
    0
}

unsafe extern "C" fn jni_CallStaticCharMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jchar {
    0
}
unsafe extern "C" fn jni_CallStaticCharMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jchar {
    0
}
unsafe extern "C" fn jni_CallStaticCharMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jchar {
    0
}

unsafe extern "C" fn jni_CallStaticShortMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jshort {
    0
}
unsafe extern "C" fn jni_CallStaticShortMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jshort {
    0
}
unsafe extern "C" fn jni_CallStaticShortMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jshort {
    0
}

unsafe extern "C" fn jni_CallStaticIntMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jint {
    0
}
unsafe extern "C" fn jni_CallStaticIntMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jint {
    0
}
unsafe extern "C" fn jni_CallStaticIntMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jint {
    0
}

unsafe extern "C" fn jni_CallStaticLongMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jlong {
    0
}
unsafe extern "C" fn jni_CallStaticLongMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jlong {
    0
}
unsafe extern "C" fn jni_CallStaticLongMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jlong {
    0
}

unsafe extern "C" fn jni_CallStaticFloatMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jfloat {
    0.0
}
unsafe extern "C" fn jni_CallStaticFloatMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jfloat {
    0.0
}
unsafe extern "C" fn jni_CallStaticFloatMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jfloat {
    0.0
}

unsafe extern "C" fn jni_CallStaticDoubleMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) -> jdouble {
    0.0
}
unsafe extern "C" fn jni_CallStaticDoubleMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) -> jdouble {
    0.0
}
unsafe extern "C" fn jni_CallStaticDoubleMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) -> jdouble {
    0.0
}

unsafe extern "C" fn jni_CallStaticVoidMethod(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    ...
) {
}
unsafe extern "C" fn jni_CallStaticVoidMethodV(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *mut std::ffi::VaList,
) {
}
unsafe extern "C" fn jni_CallStaticVoidMethodA(
    env: JNIEnvPtr,
    clazz: jclass,
    methodID: jmethodID,
    args: *const jvalue,
) {
}

// ── GetStaticFieldID (index 144) ──────────────────────────────────────────

unsafe extern "C" fn jni_GetStaticFieldID(
    env: JNIEnvPtr,
    clazz: jclass,
    name: *const c_char,
    sig: *const c_char,
) -> jfieldID {
    let name_str = cstr_to_str(name);
    let sig_str = cstr_to_str(sig);
    let mut state = GLOBAL_JVM.lock().unwrap();
    let fid = state.find_or_create_field("", name_str, sig_str);
    jni_trace!("GetStaticFieldID({}, {}) = {:p}", name_str, sig_str, fid);
    fid
}

// ── GetStatic<type>Field (indices 145..153) ───────────────────────────────

unsafe extern "C" fn jni_GetStaticObjectField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jobject {
    std::ptr::null_mut()
}
unsafe extern "C" fn jni_GetStaticBooleanField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jboolean {
    JNI_FALSE
}
unsafe extern "C" fn jni_GetStaticByteField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jbyte {
    0
}
unsafe extern "C" fn jni_GetStaticCharField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jchar {
    0
}
unsafe extern "C" fn jni_GetStaticShortField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jshort {
    0
}
unsafe extern "C" fn jni_GetStaticIntField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jint {
    0
}
unsafe extern "C" fn jni_GetStaticLongField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jlong {
    0
}
unsafe extern "C" fn jni_GetStaticFloatField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jfloat {
    0.0
}
unsafe extern "C" fn jni_GetStaticDoubleField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
) -> jdouble {
    0.0
}

// ── SetStatic<type>Field (indices 154..162) ───────────────────────────────

unsafe extern "C" fn jni_SetStaticObjectField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jobject,
) {
}
unsafe extern "C" fn jni_SetStaticBooleanField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jboolean,
) {
}
unsafe extern "C" fn jni_SetStaticByteField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jbyte,
) {
}
unsafe extern "C" fn jni_SetStaticCharField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jchar,
) {
}
unsafe extern "C" fn jni_SetStaticShortField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jshort,
) {
}
unsafe extern "C" fn jni_SetStaticIntField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jint,
) {
}
unsafe extern "C" fn jni_SetStaticLongField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jlong,
) {
}
unsafe extern "C" fn jni_SetStaticFloatField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jfloat,
) {
}
unsafe extern "C" fn jni_SetStaticDoubleField(
    env: JNIEnvPtr,
    clazz: jclass,
    fieldID: jfieldID,
    value: jdouble,
) {
}

// ── String operations (indices 163..170) ──────────────────────────────────

unsafe extern "C" fn jni_NewString(
    env: JNIEnvPtr,
    unicodeChars: *const jchar,
    len: jsize,
) -> jstring {
    // Return a dummy non-null string handle
    1isize as *mut std::os::raw::c_void
}

unsafe extern "C" fn jni_GetStringLength(env: JNIEnvPtr, string: jstring) -> jsize {
    // We don't store real string data, so return a plausible length
    0
}

unsafe extern "C" fn jni_GetStringChars(
    env: JNIEnvPtr,
    string: jstring,
    isCopy: *mut jboolean,
) -> *const jchar {
    std::ptr::null()
}

unsafe extern "C" fn jni_ReleaseStringChars(
    env: JNIEnvPtr,
    string: jstring,
    chars: *const jchar,
) {
}

unsafe extern "C" fn jni_NewStringUTF(
    env: JNIEnvPtr,
    bytes: *const c_char,
) -> jstring {
    let s = cstr_to_str(bytes);
    jni_trace!("NewStringUTF({})", s);

    // Store the CString so the pointer remains valid for GetStringUTFChars.
    // Return the CString's pointer cast to jobject so the caller can
    // retrieve the string later via GetStringUTFChars.
    let cstr = CString::new(s).unwrap();
    let ptr = cstr.as_ptr() as *mut std::os::raw::c_void;

    // We need to leak the CString so it lives forever.
    // In a real JVM, the string would be GC'd.
    std::mem::forget(cstr);

    ptr
}

unsafe extern "C" fn jni_GetStringUTFLength(env: JNIEnvPtr, string: jstring) -> jsize {
    // If the string was created by NewStringUTF, the jobject IS the C string pointer.
    if string.is_null() {
        return 0;
    }
    let cstr = std::ffi::CStr::from_ptr(string as *const c_char);
    match cstr.to_str() {
        Ok(s) => s.len() as jsize,
        Err(_) => 0,
    }
}

unsafe extern "C" fn jni_GetStringUTFChars(
    env: JNIEnvPtr,
    string: jstring,
    isCopy: *mut jboolean,
) -> *const c_char {
    if string.is_null() {
        return std::ptr::null();
    }
    // Return the string pointer directly (it was leaked by NewStringUTF)
    string as *const c_char
}

unsafe extern "C" fn jni_ReleaseStringUTFChars(
    env: JNIEnvPtr,
    string: jstring,
    utf: *const c_char,
) {
    // no-op — we leaked the memory in NewStringUTF
}

// ── Array operations (indices 171..174) ───────────────────────────────────

unsafe extern "C" fn jni_GetArrayLength(env: JNIEnvPtr, array: jarray) -> jsize {
    0
}

unsafe extern "C" fn jni_NewObjectArray(
    env: JNIEnvPtr,
    length: jsize,
    elementClass: jclass,
    initialElement: jobject,
) -> jobjectArray {
    1isize as *mut std::os::raw::c_void
}

unsafe extern "C" fn jni_GetObjectArrayElement(
    env: JNIEnvPtr,
    array: jobjectArray,
    index: jsize,
) -> jobject {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_SetObjectArrayElement(
    env: JNIEnvPtr,
    array: jobjectArray,
    index: jsize,
    value: jobject,
) {
}

// ── New<Primitive>Array (indices 175..182) ────────────────────────────────
// All return a dummy handle.

macro_rules! jni_new_primitive_array {
    ($name:ident, $ret:ty) => {
        unsafe extern "C" fn $name(env: JNIEnvPtr, len: jsize) -> $ret {
            1isize as *mut std::os::raw::c_void as $ret
        }
    };
}

jni_new_primitive_array!(jni_NewBooleanArray, jbooleanArray);
jni_new_primitive_array!(jni_NewByteArray, jbyteArray);
jni_new_primitive_array!(jni_NewCharArray, jcharArray);
jni_new_primitive_array!(jni_NewShortArray, jshortArray);
jni_new_primitive_array!(jni_NewIntArray, jintArray);
jni_new_primitive_array!(jni_NewLongArray, jlongArray);
jni_new_primitive_array!(jni_NewFloatArray, jfloatArray);
jni_new_primitive_array!(jni_NewDoubleArray, jdoubleArray);

// ── Get<Primitive>ArrayElements (indices 183..190) ────────────────────────
// All return null.

macro_rules! jni_get_primitive_array_elements {
    ($name:ident, $ret:ty) => {
        unsafe extern "C" fn $name(
            env: JNIEnvPtr,
            array: jarray,
            isCopy: *mut jboolean,
        ) -> *mut $ret {
            std::ptr::null_mut()
        }
    };
}

jni_get_primitive_array_elements!(jni_GetBooleanArrayElements, jboolean);
jni_get_primitive_array_elements!(jni_GetByteArrayElements, jbyte);
jni_get_primitive_array_elements!(jni_GetCharArrayElements, jchar);
jni_get_primitive_array_elements!(jni_GetShortArrayElements, jshort);
jni_get_primitive_array_elements!(jni_GetIntArrayElements, jint);
jni_get_primitive_array_elements!(jni_GetLongArrayElements, jlong);
jni_get_primitive_array_elements!(jni_GetFloatArrayElements, jfloat);
jni_get_primitive_array_elements!(jni_GetDoubleArrayElements, jdouble);

// ── Release<Primitive>ArrayElements (indices 191..198) ────────────────────

macro_rules! jni_release_primitive_array_elements {
    ($name:ident, $elem_ty:ty) => {
        unsafe extern "C" fn $name(
            env: JNIEnvPtr,
            array: jarray,
            elems: *mut $elem_ty,
            mode: jint,
        ) {
        }
    };
}

jni_release_primitive_array_elements!(jni_ReleaseBooleanArrayElements, jboolean);
jni_release_primitive_array_elements!(jni_ReleaseByteArrayElements, jbyte);
jni_release_primitive_array_elements!(jni_ReleaseCharArrayElements, jchar);
jni_release_primitive_array_elements!(jni_ReleaseShortArrayElements, jshort);
jni_release_primitive_array_elements!(jni_ReleaseIntArrayElements, jint);
jni_release_primitive_array_elements!(jni_ReleaseLongArrayElements, jlong);
jni_release_primitive_array_elements!(jni_ReleaseFloatArrayElements, jfloat);
jni_release_primitive_array_elements!(jni_ReleaseDoubleArrayElements, jdouble);

// ── Get<Primitive>ArrayRegion (indices 199..206) ──────────────────────────

macro_rules! jni_get_primitive_array_region {
    ($name:ident, $elem_ty:ty) => {
        unsafe extern "C" fn $name(
            env: JNIEnvPtr,
            array: jarray,
            start: jsize,
            len: jsize,
            buf: *mut $elem_ty,
        ) {
        }
    };
}

jni_get_primitive_array_region!(jni_GetBooleanArrayRegion, jboolean);
jni_get_primitive_array_region!(jni_GetByteArrayRegion, jbyte);
jni_get_primitive_array_region!(jni_GetCharArrayRegion, jchar);
jni_get_primitive_array_region!(jni_GetShortArrayRegion, jshort);
jni_get_primitive_array_region!(jni_GetIntArrayRegion, jint);
jni_get_primitive_array_region!(jni_GetLongArrayRegion, jlong);
jni_get_primitive_array_region!(jni_GetFloatArrayRegion, jfloat);
jni_get_primitive_array_region!(jni_GetDoubleArrayRegion, jdouble);

// ── Set<Primitive>ArrayRegion (indices 207..214) ──────────────────────────

macro_rules! jni_set_primitive_array_region {
    ($name:ident, $elem_ty:ty) => {
        unsafe extern "C" fn $name(
            env: JNIEnvPtr,
            array: jarray,
            start: jsize,
            len: jsize,
            buf: *const $elem_ty,
        ) {
        }
    };
}

jni_set_primitive_array_region!(jni_SetBooleanArrayRegion, jboolean);
jni_set_primitive_array_region!(jni_SetByteArrayRegion, jbyte);
jni_set_primitive_array_region!(jni_SetCharArrayRegion, jchar);
jni_set_primitive_array_region!(jni_SetShortArrayRegion, jshort);
jni_set_primitive_array_region!(jni_SetIntArrayRegion, jint);
jni_set_primitive_array_region!(jni_SetLongArrayRegion, jlong);
jni_set_primitive_array_region!(jni_SetFloatArrayRegion, jfloat);
jni_set_primitive_array_region!(jni_SetDoubleArrayRegion, jdouble);

// ── RegisterNatives / UnregisterNatives (indices 215..216) ────────────────

unsafe extern "C" fn jni_RegisterNatives(
    env: JNIEnvPtr,
    clazz: jclass,
    methods: *const JNINativeMethod,
    nMethods: jint,
) -> jint {
    let methods_slice = std::slice::from_raw_parts(methods, nMethods as usize);
    let mut state = GLOBAL_JVM.lock().unwrap();
    for m in methods_slice {
        let name = cstr_to_str(m.name);
        let sig = cstr_to_str(m.signature);
        state.register_native(clazz, name, sig, m.fnPtr);
    }
    JNI_OK
}

unsafe extern "C" fn jni_UnregisterNatives(
    env: JNIEnvPtr,
    clazz: jclass,
) -> jint {
    JNI_OK
}

// ── Monitor operations (indices 217..218) ─────────────────────────────────

unsafe extern "C" fn jni_MonitorEnter(env: JNIEnvPtr, obj: jobject) -> jint {
    JNI_OK
}

unsafe extern "C" fn jni_MonitorExit(env: JNIEnvPtr, obj: jobject) -> jint {
    JNI_OK
}

// ── GetJavaVM (index 219) ─────────────────────────────────────────────────

unsafe extern "C" fn jni_GetJavaVM(
    env: JNIEnvPtr,
    vm: *mut JavaVMPtr,
) -> jint {
    let state = GLOBAL_JVM.lock().unwrap();
    if let Some(vm_ptr) = state.java_vm {
        *vm = vm_ptr;
        JNI_OK
    } else {
        JNI_ERR
    }
}

// ── String region (indices 220..221) ──────────────────────────────────────

unsafe extern "C" fn jni_GetStringRegion(
    env: JNIEnvPtr,
    str_: jstring,
    start: jsize,
    len: jsize,
    buf: *mut jchar,
) {
}

unsafe extern "C" fn jni_GetStringUTFRegion(
    env: JNIEnvPtr,
    str_: jstring,
    start: jsize,
    len: jsize,
    buf: *mut c_char,
) {
}

// ── Primitive array critical (indices 222..223) ───────────────────────────

unsafe extern "C" fn jni_GetPrimitiveArrayCritical(
    env: JNIEnvPtr,
    array: jarray,
    isCopy: *mut jboolean,
) -> *mut c_void {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_ReleasePrimitiveArrayCritical(
    env: JNIEnvPtr,
    array: jarray,
    carray: *mut c_void,
    mode: jint,
) {
}

// ── String critical (indices 224..225) ────────────────────────────────────

unsafe extern "C" fn jni_GetStringCritical(
    env: JNIEnvPtr,
    string: jstring,
    isCopy: *mut jboolean,
) -> *const jchar {
    std::ptr::null()
}

unsafe extern "C" fn jni_ReleaseStringCritical(
    env: JNIEnvPtr,
    string: jstring,
    carray: *const jchar,
) {
}

// ── Weak global refs (indices 226..227) ───────────────────────────────────

unsafe extern "C" fn jni_NewWeakGlobalRef(env: JNIEnvPtr, obj: jobject) -> jweak {
    obj
}

unsafe extern "C" fn jni_DeleteWeakGlobalRef(env: JNIEnvPtr, obj: jweak) {
}

// ── ExceptionCheck (index 228) ────────────────────────────────────────────

unsafe extern "C" fn jni_ExceptionCheck(env: JNIEnvPtr) -> jboolean {
    JNI_FALSE
}

// ── NIO direct buffer (indices 229..231) ──────────────────────────────────

unsafe extern "C" fn jni_NewDirectByteBuffer(
    env: JNIEnvPtr,
    address: *mut c_void,
    capacity: jlong,
) -> jobject {
    1isize as *mut std::os::raw::c_void
}

unsafe extern "C" fn jni_GetDirectBufferAddress(
    env: JNIEnvPtr,
    buf: jobject,
) -> *mut c_void {
    std::ptr::null_mut()
}

unsafe extern "C" fn jni_GetDirectBufferCapacity(
    env: JNIEnvPtr,
    buf: jobject,
) -> jlong {
    0
}

// ── GetObjectRefType (index 232) ──────────────────────────────────────────

unsafe extern "C" fn jni_GetObjectRefType(
    env: JNIEnvPtr,
    obj: jobject,
) -> jobjectRefType {
    jobjectRefType::JNILocalRefType
}

// =========================================================================
// JavaVM function implementations
// =========================================================================

unsafe extern "C" fn jvm_DestroyJavaVM(vm: JavaVMPtr) -> jint {
    jni_trace!("DestroyJavaVM");
    JNI_OK
}

unsafe extern "C" fn jvm_AttachCurrentThread(
    vm: JavaVMPtr,
    penv: *mut JNIEnvPtr,
    args: *mut c_void,
) -> jint {
    let state = GLOBAL_JVM.lock().unwrap();
    if let Some(env_ptr) = state.jni_env {
        *penv = env_ptr;
        JNI_OK
    } else {
        JNI_ERR
    }
}

unsafe extern "C" fn jvm_DetachCurrentThread(vm: JavaVMPtr) -> jint {
    JNI_OK
}

unsafe extern "C" fn jvm_GetEnv(
    vm: JavaVMPtr,
    penv: *mut *mut c_void,
    version: jint,
) -> jint {
    let state = GLOBAL_JVM.lock().unwrap();
    if let Some(env_ptr) = state.jni_env {
        *penv = env_ptr as *mut c_void;
        JNI_OK
    } else {
        JNI_EDETACHED
    }
}

unsafe extern "C" fn jvm_AttachCurrentThreadAsDaemon(
    vm: JavaVMPtr,
    penv: *mut JNIEnvPtr,
    args: *mut c_void,
) -> jint {
    let state = GLOBAL_JVM.lock().unwrap();
    if let Some(env_ptr) = state.jni_env {
        *penv = env_ptr;
        JNI_OK
    } else {
        JNI_ERR
    }
}

// =========================================================================
// Exported entry points — called by the linker when libroblox.so loads
// =========================================================================

/// JNI_OnLoad is called by the Android JNI framework when the library is
/// first loaded via System.loadLibrary().  libroblox.so calls this to get
/// a JavaVM* and register its native methods.
///
/// This is the primary entry point for the JNI bridge.  We return
/// JNI_VERSION_1_6 to satisfy the protocol.
#[no_mangle]
pub unsafe extern "C" fn JNI_OnLoad(vm: JavaVMPtr, reserved: *mut c_void) -> jint {
    // Initialize our global table pointers
    init_global_tables();

    tracing::info!("jni-bridge: JNI_OnLoad called, returning JNI_VERSION_1_6");

    // Write our JavaVM wrapper into the vm pointer the caller passed.
    // In the real JNI, Android passes a real JavaVM*.  We store ours.
    if !vm.is_null() {
        // Copy our function table into the VM they provided.
        // (But we don't want to overwrite their memory; instead we set
        //  our own global pointers and let GetEnv use them.)
    }

    JNI_VERSION_1_6
}

/// JNI_OnUnload is the teardown counterpart.
#[no_mangle]
pub unsafe extern "C" fn JNI_OnUnload(vm: JavaVMPtr, reserved: *mut c_void) {
    tracing::info!("jni-bridge: JNI_OnUnload called");
}

/// JNI_GetDefaultJavaVMInitArgs — minimal implementation so that the linker
/// can find the symbol.
#[no_mangle]
pub unsafe extern "C" fn JNI_GetDefaultJavaVMInitArgs(args: *mut c_void) -> jint {
    JNI_ERR
}

/// JNI_CreateJavaVM — minimal implementation.
#[no_mangle]
pub unsafe extern "C" fn JNI_CreateJavaVM(
    vm: *mut JavaVMPtr,
    env: *mut JNIEnvPtr,
    args: *mut c_void,
) -> jint {
    init_global_tables();
    let state = GLOBAL_JVM.lock().unwrap();
    if let (Some(vm_ptr), Some(env_ptr)) = (state.java_vm, state.jni_env) {
        *vm = vm_ptr;
        *env = env_ptr;
        JNI_OK
    } else {
        JNI_ERR
    }
}

/// JNI_GetCreatedJavaVMs — returns our JVM.
#[no_mangle]
pub unsafe extern "C" fn JNI_GetCreatedJavaVMs(
    vmBuf: *mut JavaVMPtr,
    bufLen: jsize,
    nVMs: *mut jsize,
) -> jint {
    let state = GLOBAL_JVM.lock().unwrap();
    if bufLen >= 1 {
        if let Some(vm_ptr) = state.java_vm {
            *vmBuf = vm_ptr;
            *nVMs = 1;
        } else {
            *nVMs = 0;
        }
    }
    JNI_OK
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jni_env_table_is_populated() {
        unsafe {
            assert!(JNI_ENV_FUNCTIONS.GetVersion.is_some());
            assert!(JNI_ENV_FUNCTIONS.FindClass.is_some());
            assert!(JNI_ENV_FUNCTIONS.GetMethodID.is_some());
            assert!(JNI_ENV_FUNCTIONS.NewStringUTF.is_some());
            assert!(JNI_ENV_FUNCTIONS.GetStringUTFChars.is_some());
            assert!(JNI_ENV_FUNCTIONS.CallVoidMethodV.is_some());
            assert!(JNI_ENV_FUNCTIONS.RegisterNatives.is_some());
            assert!(JNI_ENV_FUNCTIONS.GetJavaVM.is_some());
        }
    }

    #[test]
    fn test_java_vm_table_is_populated() {
        unsafe {
            assert!(JAVA_VM_FUNCTIONS.GetEnv.is_some());
            assert!(JAVA_VM_FUNCTIONS.AttachCurrentThread.is_some());
            assert!(JAVA_VM_FUNCTIONS.DestroyJavaVM.is_some());
        }
    }

    #[test]
    fn test_jni_onload_returns_version() {
        unsafe {
            let result = JNI_OnLoad(std::ptr::null_mut(), std::ptr::null_mut());
            assert_eq!(result, JNI_VERSION_1_6);
        }
    }

    #[test]
    fn test_find_class_returns_non_null() {
        unsafe {
            let mut env_wrapper = JNIEnvWrapper {
                functions: &JNI_ENV_FUNCTIONS,
            };
            let env_ptr: JNIEnvPtr = &mut env_wrapper;
            let name = CString::new("com/roblox/client/RobloxActivity").unwrap();
            let cls = jni_FindClass(env_ptr, name.as_ptr());
            assert!(!cls.is_null());
        }
    }

    #[test]
    fn test_new_string_utf_roundtrip() {
        unsafe {
            let mut env_wrapper = JNIEnvWrapper {
                functions: &JNI_ENV_FUNCTIONS,
            };
            let env_ptr: JNIEnvPtr = &mut env_wrapper;

            let input = CString::new("hello").unwrap();
            let str_handle = jni_NewStringUTF(env_ptr, input.as_ptr());
            assert!(!str_handle.is_null());

            let output = jni_GetStringUTFChars(env_ptr, str_handle, std::ptr::null_mut());
            assert!(!output.is_null());

            let output_str = CStr::from_ptr(output).to_str().unwrap();
            assert_eq!(output_str, "hello");
        }
    }

    #[test]
    fn test_register_natives_ok() {
        unsafe {
            let mut env_wrapper = JNIEnvWrapper {
                functions: &JNI_ENV_FUNCTIONS,
            };
            let env_ptr: JNIEnvPtr = &mut env_wrapper;

            let name = CString::new("nativeInit").unwrap();
            let sig = CString::new("()V").unwrap();
            let method = JNINativeMethod {
                name: name.as_ptr(),
                signature: sig.as_ptr(),
                fnPtr: std::ptr::null_mut(),
            };

            let result = jni_RegisterNatives(
                env_ptr,
                1isize as *mut c_void,
                &method,
                1,
            );
            assert_eq!(result, JNI_OK);
        }
    }

    #[test]
    fn test_jvm_get_env_returns_env() {
        unsafe {
            init_global_tables();

            let mut vm = JavaVMWrapper {
                functions: &JAVA_VM_FUNCTIONS,
            };
            let vm_ptr: JavaVMPtr = &mut vm;

            let mut penv: *mut c_void = std::ptr::null_mut();
            let result = jvm_GetEnv(vm_ptr, &mut penv, JNI_VERSION_1_6);
            assert_eq!(result, JNI_OK);
            assert!(!penv.is_null());
        }
    }
}