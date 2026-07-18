// SPDX-License-Identifier: MIT
//
// JNI type definitions matching the Android jni.h C layout exactly.
// These are the primitive types, opaque handles, and struct layouts that
// libroblox.so expects.

#![allow(non_camel_case_types, dead_code)]

use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

pub type jboolean = u8;
pub type jbyte = i8;
pub type jchar = u16;
pub type jshort = i16;
pub type jint = i32;
pub type jlong = i64;
pub type jfloat = f32;
pub type jdouble = f64;
pub type jsize = jint;

// ---------------------------------------------------------------------------
// Object reference types (opaque pointers, matching C jni.h)
// ---------------------------------------------------------------------------

pub type jobject = *mut c_void;
pub type jclass = jobject;
pub type jstring = jobject;
pub type jarray = jobject;
pub type jobjectArray = jarray;
pub type jbooleanArray = jarray;
pub type jbyteArray = jarray;
pub type jcharArray = jarray;
pub type jshortArray = jarray;
pub type jintArray = jarray;
pub type jlongArray = jarray;
pub type jfloatArray = jarray;
pub type jdoubleArray = jarray;
pub type jthrowable = jobject;
pub type jweak = jobject;

// Opaque field and method IDs.
pub enum _jfieldID {}
pub type jfieldID = *mut _jfieldID;

pub enum _jmethodID {}
pub type jmethodID = *mut _jmethodID;

// ---------------------------------------------------------------------------
// jvalue union
// ---------------------------------------------------------------------------

#[repr(C)]
pub union jvalue {
    pub z: jboolean,
    pub b: jbyte,
    pub c: jchar,
    pub s: jshort,
    pub i: jint,
    pub j: jlong,
    pub f: jfloat,
    pub d: jdouble,
    pub l: jobject,
}

// ---------------------------------------------------------------------------
// JNINativeMethod struct (for RegisterNatives)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct JNINativeMethod {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub fnPtr: *mut c_void,
}

// ---------------------------------------------------------------------------
// jobjectRefType enum
// ---------------------------------------------------------------------------

#[repr(C)]
pub enum jobjectRefType {
    JNIInvalidRefType = 0,
    JNILocalRefType = 1,
    JNIGlobalRefType = 2,
    JNIWeakGlobalRefType = 3,
}

// ---------------------------------------------------------------------------
// JavaVMAttachArgs
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct JavaVMAttachArgs {
    pub version: jint,
    pub name: *const c_char,
    pub group: jobject,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const JNI_FALSE: jboolean = 0;
pub const JNI_TRUE: jboolean = 1;

pub const JNI_VERSION_1_1: jint = 0x00010001;
pub const JNI_VERSION_1_2: jint = 0x00010002;
pub const JNI_VERSION_1_4: jint = 0x00010004;
pub const JNI_VERSION_1_6: jint = 0x00010006;

pub const JNI_OK: jint = 0;
pub const JNI_ERR: jint = -1;
pub const JNI_EDETACHED: jint = -2;
pub const JNI_EVERSION: jint = -3;

pub const JNI_COMMIT: jint = 1;
pub const JNI_ABORT: jint = 2;