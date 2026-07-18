// SPDX-License-Identifier: MIT
//
// JVM state — manages registered classes, methods, fields, and object
// references that libroblox.so creates during initialization.

#![allow(dead_code)]

use crate::ffi_table::{JNIEnvPtr, JavaVMPtr};
use crate::types::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Wrapper types that implement Send/Sync for raw pointers used in HashMaps
// ---------------------------------------------------------------------------

/// A raw pointer that is Send + Sync.  SAFETY: used only behind a Mutex.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendPtr(pub *mut std::os::raw::c_void);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendConstPtr(pub *const u8);
unsafe impl Send for SendConstPtr {}
unsafe impl Sync for SendConstPtr {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendJMethodID(pub jmethodID);
unsafe impl Send for SendJMethodID {}
unsafe impl Sync for SendJMethodID {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendJFieldID(pub jfieldID);
unsafe impl Send for SendJFieldID {}
unsafe impl Sync for SendJFieldID {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendJClass(pub jclass);
unsafe impl Send for SendJClass {}
unsafe impl Sync for SendJClass {}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendCVoidPtr(pub *mut std::os::raw::c_void);
unsafe impl Send for SendCVoidPtr {}
unsafe impl Sync for SendCVoidPtr {}

// ---------------------------------------------------------------------------
// Class key for methods/fields — Send + Sync wrapper
// ---------------------------------------------------------------------------

pub type MethodKey = (String, String, String);
pub type FieldKey = (String, String, String);

/// Global JVM state, accessed via the thread-safe singleton.
pub struct JvmState {
    /// Registered class names -> jclass (opaque pointers)
    pub classes: HashMap<String, SendJClass>,

    /// Method lookups: "(className, methodName, signature)" -> jmethodID
    pub methods: HashMap<MethodKey, SendJMethodID>,

    /// Field lookups: "(className, fieldName, signature)" -> jfieldID
    pub fields: HashMap<FieldKey, SendJFieldID>,

    /// Registered native methods via RegisterNatives
    pub native_methods: Vec<(SendJClass, String, String, SendCVoidPtr)>,

    /// Next opaque handle value for classes, methods, fields
    pub next_class_handle: usize,
    pub next_method_handle: usize,
    pub next_field_handle: usize,

    /// Strings we've created via NewStringUTF (keep alive so the pointer stays valid)
    pub utf_strings: HashMap<SendConstPtr, CString>,

    /// The single JavaVM instance
    pub java_vm: Option<JavaVMPtr>,

    /// The per-thread JNIEnv instance
    pub jni_env: Option<JNIEnvPtr>,

    /// Global refs: simple refcount.  jobject -> refcount.
    pub global_refs: HashMap<isize, i32>,
    pub next_global_ref_id: isize,
}

impl JvmState {
    pub fn new() -> Self {
        JvmState {
            classes: HashMap::new(),
            methods: HashMap::new(),
            fields: HashMap::new(),
            native_methods: Vec::new(),
            next_class_handle: 1,
            next_method_handle: 1,
            next_field_handle: 1,
            utf_strings: HashMap::new(),
            java_vm: None,
            jni_env: None,
            global_refs: HashMap::new(),
            next_global_ref_id: 1000,
        }
    }

    pub fn find_or_create_class(&mut self, name: &str) -> jclass {
        if let Some(&cls) = self.classes.get(name) {
            return cls.0;
        }
        let handle = self.next_class_handle;
        self.next_class_handle += 1;
        let ptr = handle as *mut std::os::raw::c_void;
        self.classes.insert(name.to_string(), SendJClass(ptr));
        tracing::debug!(class = %name, handle = handle, "JNI: created class");
        ptr
    }

    pub fn find_or_create_method(
        &mut self,
        class_name: &str,
        name: &str,
        sig: &str,
    ) -> jmethodID {
        let key = (class_name.to_string(), name.to_string(), sig.to_string());
        if let Some(&mid) = self.methods.get(&key) {
            return mid.0;
        }
        let handle = self.next_method_handle;
        self.next_method_handle += 1;
        let ptr = handle as *mut _jmethodID;
        self.methods.insert(key, SendJMethodID(ptr));
        tracing::debug!(method = %name, sig = %sig, handle = handle, "JNI: created method");
        ptr
    }

    pub fn find_or_create_field(
        &mut self,
        class_name: &str,
        name: &str,
        sig: &str,
    ) -> jfieldID {
        let key = (class_name.to_string(), name.to_string(), sig.to_string());
        if let Some(&fid) = self.fields.get(&key) {
            return fid.0;
        }
        let handle = self.next_field_handle;
        self.next_field_handle += 1;
        let ptr = handle as *mut _jfieldID;
        self.fields.insert(key, SendJFieldID(ptr));
        tracing::debug!(field = %name, sig = %sig, handle = handle, "JNI: created field");
        ptr
    }

    pub fn store_utf_string(&mut self, s: &str) -> *const c_char {
        let cstr = CString::new(s).unwrap();
        let ptr = cstr.as_ptr();
        self.utf_strings.insert(SendConstPtr(ptr as *const u8), cstr);
        ptr
    }

    pub fn register_native(&mut self, clazz: jclass, name: &str, sig: &str, fn_ptr: *mut std::os::raw::c_void) {
        self.native_methods.push((
            SendJClass(clazz),
            name.to_string(),
            sig.to_string(),
            SendCVoidPtr(fn_ptr),
        ));
        tracing::debug!(name = %name, sig = %sig, "JNI: registered native method");
    }
}

// Use std::sync::OnceLock instead of lazy_static to avoid the Send/Sync issues
use std::sync::OnceLock;

fn global_jvm_inner() -> &'static Mutex<Option<JvmState>> {
    static STATE: OnceLock<Mutex<Option<JvmState>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

/// Get a reference to the global JVM state, initializing it first.
pub fn with_jvm_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut JvmState) -> R,
{
    let lock = global_jvm_inner();
    let mut guard = lock.lock().unwrap();
    if guard.is_none() {
        *guard = Some(JvmState::new());
    }
    f(guard.as_mut().unwrap())
}

pub fn init_jvm_state() {
    with_jvm_state(|_| {});
}

// ---------------------------------------------------------------------------
// Helper to convert C strings to Rust
// ---------------------------------------------------------------------------

pub unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

/// Get a const char* that lives as long as the JvmState.
/// The caller MUST ensure GLOBAL_JVM is locked.
pub fn store_string(state: &mut JvmState, s: &str) -> *const c_char {
    state.store_utf_string(s)
}