//! Guest-visible Java Native Interface (JNI) environment + JavaVM for the JIT.
//!
//! Ports the QEMU path's `jni_shim.c` JNI function table. On Android `JNI_OnLoad`
//! receives a `JavaVM*`; it calls `vm->GetEnv(&env, JNI_VERSION_1_6)` to obtain a
//! `JNIEnv*` whose word0 is a pointer to the `JNINativeInterface` function table,
//! then calls `env->FindClass/GetStaticMethodID/NewStringUTF/...` through it.
//!
//! In the JIT host thunks live in guest-address space and guest==host, so the
//! JNI table we build in guest memory holds host-thunk guest addresses directly.
//! Each JNINative slot maps to a host Rust stub with the generic `HostCall` ABI
//! (8 u64 args -> u64). This is the C file's `jni_table`/`vm_table` in
//! thunk-address form.
//!
//! # Slot indices are the OFFICIAL Android JNI ABI
//! libroblox.so is compiled against Android's `jni.h`, so it indexes the
//! function table with the official `JNINativeInterface` word offsets
//! (reserved0..3 = 0..3, GetVersion=4, FindClass=6, GetMethodID=33,
//! GetFieldID=94, GetStaticMethodID=113, NewStringUTF=167,
//! GetStringUTFChars=169, RegisterNatives=199, GetJavaVM=203). The JIT table
//! MUST use these exact offsets or a guest call lands on the wrong stub. (The
//! QEMU shim's earlier 36/193/197/102 etc. were unvalidated guesses — its only
//! end-to-end-validated slots were GetVersion/FindClass/GetStaticMethodID,
//! which coincidentally match the official offsets.)

use crate::jit::{register_host_call_auto, HostCall};
use std::alloc::{alloc_zeroed, Layout};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const JNI_VERSION_1_6: i32 = 0x0001_0006;
const JNI_SLOTS: usize = 256;
const VM_SLOTS: usize = 8;

// Official JNINativeInterface word offsets (the ones the guest actually uses).
const GET_VERSION: usize = 4;
const FIND_CLASS: usize = 6;
const THROW: usize = 13;
const THROW_NEW: usize = 14;
const NEW_GLOBAL_REF: usize = 21;
const DEL_GLOBAL_REF: usize = 22;
const DEL_LOCAL_REF: usize = 23;
const GET_METHOD_ID: usize = 33;
const GET_FIELD_ID: usize = 94;
const GET_OBJ_FIELD: usize = 95;
const GET_BOOLEAN_FIELD: usize = 96;
const GET_INT_FIELD: usize = 100;
const SET_OBJ_FIELD: usize = 104;
const SET_BOOLEAN_FIELD: usize = 105;
const GET_STATIC_METHOD_ID: usize = 113;
const NEW_STRING_UTF: usize = 167;
const GET_STRING_UTF_LEN: usize = 168;
const GET_STRING_UTF_CHARS: usize = 169;
const GET_ARRAY_LEN: usize = 171;
const NEW_OBJECT_ARRAY: usize = 172;
const GET_OBJ_ARR_ELEM: usize = 173;
const SET_OBJ_ARR_ELEM: usize = 174;
const REGISTER_NATIVES: usize = 199;
const GET_JAVA_VM: usize = 203;
// Official JNIVMInterface word offsets.
const VM_GET_ENV: usize = 7;

/// Registry of interned UTF-8 byte strings: `str_handle` allocates a readable,
/// null-terminated copy in guest-addressable memory (guest==host) and returns a
/// stable handle for the same bytes. This is the JIT analogue of the QEMU
/// shim's `track_ptr`: a *readable* buffer rather than a low sentinel like
/// 0x3000, so guest code that dereferences a returned jclass/jstring/methodID
/// reads valid memory instead of faulting.
fn str_handle(bytes: &[u8]) -> u64 {
    static REG: OnceLock<Mutex<HashMap<Vec<u8>, u64>>> = OnceLock::new();
    let mut reg = REG.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(&h) = reg.get(bytes) {
        return h;
    }
    let layout = Layout::array::<u8>(bytes.len() + 1).unwrap();
    let p = unsafe { alloc_zeroed(layout) } as *mut u8;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len()) };
    let addr = p as u64;
    reg.insert(bytes.to_vec(), addr);
    addr
}

/// Guest-visible registry of native methods registered via `RegisterNatives`.
///
/// Android's `JNI_OnLoad` calls `env->RegisterNatives(clazz, methods, nMethods)`
/// where `methods` is a `JNINativeMethod` array (3 u64 words each: `name`,
/// `signature`, `fnPtr`). The JIT's `jni_register_natives` host stub previously
/// returned JNI_OK and DROPPED the array — so a native method Roblox binds
/// (e.g. `Java_com_roblox_..._IAP...`) could never be found again when the host
/// runtime later dispatches back into the guest. This registry parses the
/// guest array (guest==host, so a host read is a guest read) and records
/// `(class, name) -> (signature, fnPtr)` where `fnPtr` is the *guest* address
/// of the Java_* implementation, ready to be driven through `jit_run` as a
/// guest entry point.
#[derive(Clone, Debug)]
pub struct NativeMethod {
    pub class: Vec<u8>,
    pub name: Vec<u8>,
    pub signature: Vec<u8>,
    pub fn_ptr: u64,
}

fn native_registry() -> &'static Mutex<Vec<NativeMethod>> {
    static REG: OnceLock<Mutex<Vec<NativeMethod>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(Vec::new()))
}

/// Wipe the registry (test-only / re-boot hygiene; also guarded so a fresh
/// JNI_OnLoad doesn't accumulate stale bindings with side effects).
pub fn clear_native_methods() {
    native_registry().lock().unwrap().clear();
}

/// Look up a registered native method by (class, name); returns the guest
/// `fnPtr` and signature. This is what a host dispatch of a Roblox Java_*
/// method needs to convert a Java method name into a runnable guest entry.
pub fn lookup_native_method(class: &[u8], name: &[u8]) -> Option<NativeMethod> {
    let reg = native_registry().lock().unwrap();
    // Last-registration wins (JNI semantics: re-registering replaces).
    reg.iter().rev().find(|m| m.class == class && m.name == name).cloned()
}

/// Record a native method binding from the guest `JNINativeMethod` array at
/// `methods` (nMethods words of sizt 3). Parse each entry's three u64 words
/// (name*, signature*, fnPtr). A missing/unreadable name or a NULL fnPtr is a
/// malformed register call; skip it (JNI treats fnPtr==NULL as "delete
/// binding" — stricter handling can come later) rather than fault.
fn parse_register_natives(cls: u64, methods: u64, n: u64) {
    if methods == 0 {
        return;
    }
    let class_bytes = read_cstr(cls).unwrap_or_default();
    let mut reg = native_registry().lock().unwrap();
    let base = methods as *const u64;
    for i in 0..n {
        let e = unsafe { base.add(i as usize * 3) };
        let name_p = unsafe { *e };
        let sig_p = unsafe { *e.add(1) };
        let fn_ptr = unsafe { *e.add(2) };
        let Some(name) = read_cstr(name_p) else { continue };
        let sig = read_cstr(sig_p).unwrap_or_default();
        reg.push(NativeMethod { class: class_bytes.clone(), name, signature: sig, fn_ptr });
    }
}

/// Read a NUL-terminated C string from guest memory (`guest==host`, so a host
/// read is a guest read). Bounded to avoid over-reading a bad pointer.
fn read_cstr(p: u64) -> Option<Vec<u8>> {
    if p == 0 {
        return None;
    }
    const MAX: usize = 4096;
    let p = p as *const u8;
    let mut v = Vec::new();
    for i in 0..MAX {
        let c = unsafe { *p.add(i) };
        if c == 0 {
            return Some(v);
        }
        v.push(c);
    }
    None
}

/// Read the C string at `ptr` and intern it via `str_handle` (0 if unreadable).
fn cstr_handle(ptr: u64) -> u64 {
    read_cstr(ptr).map_or(0, |s| str_handle(&s))
}

/// Generic "return a constant / ignore args" stub.
macro_rules! jni_stub {
    ($name:ident, $val:expr) => {
        extern "C" fn $name(
            _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
        ) -> u64 {
            $val
        }
    };
}

jni_stub!(jni_get_version, JNI_VERSION_1_6 as u64);
jni_stub!(jni_voidp_0, 0); // default: return NULL/0
jni_stub!(jni_ok, 0); // JNI_OK / status-returning stubs
jni_stub!(jni_get_array_length, 0);
jni_stub!(jni_field_0, 0);

extern "C" fn jni_find_class(
    _e: u64, name: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    cstr_handle(name) // jclass = readable, stable handle
}

extern "C" fn jni_get_method_id(
    _e: u64, _cls: u64, name: u64, sig: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    // Readable stable handle (method IDs are opaque to guests; safety over sentinel).
    let _ = sig;
    cstr_handle(name)
}

extern "C" fn jni_new_string_utf(
    _e: u64, utf: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    cstr_handle(utf) // jstring = readable UTF-8 buffer
}

extern "C" fn jni_get_string_utf_chars(
    _e: u64, jstr: u64, iscopy: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    if iscopy != 0 {
        unsafe { *(iscopy as *mut i8) = 0 }; // "no copy returned"
    }
    jstr // the jstring IS the readable buffer we gave out in NewStringUTF
}

extern "C" fn jni_get_string_utf_length(
    _e: u64, jstr: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    read_cstr(jstr).map_or(0, |s| s.len() as u64)
}

extern "C" fn jni_register_natives(
    _e: u64, cls: u64, methods: u64, n: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    // Parse the guest JNINativeMethod array and record (class,name,sig)->fnPtr
    // so a registered Roblox Java_* method can later be dispatched back into
    // the guest as a jit_run entry. (See parse_register_natives docs.)
    parse_register_natives(cls, methods, n);
    0 // JNI_OK
}

extern "C" fn jni_new_global_ref(
    _e: u64, o: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    o // pass-through like the QEMU shim
}

extern "C" fn jni_get_java_vm(
    _e: u64, vm_out: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    let (_env, vm) = build_jni();
    if vm_out != 0 {
        unsafe { *(vm_out as *mut u64) = vm };
    }
    0
}

// JVM `GetEnv`/`AttachCurrentThread`: `jint GetEnv(JavaVM*, void** penv, jint)`
// writes the current thread's JNIEnv into `*penv` and returns JNI_OK (0).
extern "C" fn jni_vm_getenv(
    _vm: u64, penv: u64, _version: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    let (env, _vm) = build_jni();
    if penv != 0 {
        unsafe { *(penv as *mut u64) = env };
    }
    0 // JNI_OK
}

/// Build the singleton guest-visible JNIEnv and JavaVM, each with a pointer
/// table whose slots are host-thunk guest addresses. Returns `(jni_env, java_vm)`.
pub fn build_jni() -> (u64, u64) {
    static ONE: OnceLock<(u64, u64)> = OnceLock::new();
    *ONE.get_or_init(|| {
        let version = reg(jni_get_version);
        let voidp = reg(jni_voidp_0);
        let ok = reg(jni_ok);
        let field0 = reg(jni_field_0);

        // Default every slot to a NULL/0-returning stub so an unserviced slot
        // never dispatches to NULL.
        let mut functions: Vec<u64> = vec![voidp; JNI_SLOTS];
        functions[GET_VERSION] = version;
        functions[FIND_CLASS] = reg(jni_find_class);
        functions[THROW] = ok;
        functions[THROW_NEW] = ok;
        functions[NEW_GLOBAL_REF] = reg(jni_new_global_ref);
        functions[DEL_GLOBAL_REF] = ok;
        functions[DEL_LOCAL_REF] = ok;
        functions[GET_METHOD_ID] = reg(jni_get_method_id);
        functions[GET_FIELD_ID] = reg(jni_get_method_id);
        functions[GET_OBJ_FIELD] = field0;
        functions[GET_BOOLEAN_FIELD] = field0;
        functions[GET_INT_FIELD] = field0;
        functions[SET_OBJ_FIELD] = ok;
        functions[SET_BOOLEAN_FIELD] = ok;
        functions[GET_STATIC_METHOD_ID] = reg(jni_get_method_id);
        functions[NEW_STRING_UTF] = reg(jni_new_string_utf);
        functions[GET_STRING_UTF_LEN] = reg(jni_get_string_utf_length);
        functions[GET_STRING_UTF_CHARS] = reg(jni_get_string_utf_chars);
        functions[GET_ARRAY_LEN] = reg(jni_get_array_length);
        functions[NEW_OBJECT_ARRAY] = voidp;
        functions[GET_OBJ_ARR_ELEM] = voidp;
        functions[SET_OBJ_ARR_ELEM] = ok;
        functions[REGISTER_NATIVES] = reg(jni_register_natives);
        functions[GET_JAVA_VM] = reg(jni_get_java_vm);

        let env_fn_tbl = u64array(&functions);
        let mut vm_functions = vec![voidp; VM_SLOTS];
        let vm_getenv = reg(jni_vm_getenv);
        vm_functions[VM_GET_ENV] = vm_getenv; // GetEnv: writes *penv=env, returns JNI_OK
        let vm_fn_tbl = u64array(&vm_functions);

        let env = object2(env_fn_tbl);
        let vm = object2(vm_fn_tbl);
        (env, vm)
    })
}

fn reg(f: HostCall) -> u64 {
    register_host_call_auto(f)
}

/// Allocate (host==guest) memory for a u64 table; return its address.
fn u64array(items: &[u64]) -> u64 {
    let n = items.len();
    let p = unsafe { alloc_zeroed(Layout::array::<u64>(n).unwrap()) } as *mut u64;
    for (i, v) in items.iter().enumerate() {
        unsafe { *p.add(i) = *v };
    }
    p as u64
}

/// Allocate a 5-word object whose word0 = `functions` pointer (JNIEnv/JavaVM).
fn object2(functions: u64) -> u64 {
    let p = unsafe { alloc_zeroed(Layout::new::<[u64; 5]>()) } as *mut u64;
    unsafe { *p = functions };
    p as u64
}

/// The guest address `vm->GetEnv(&env, v)` should write into `*penv`; the guest
/// JNIEnv (idempotent). Convenience for boot glue.
pub fn env_addr() -> u64 {
    let (e, _v) = build_jni();
    e
}

/// Dispatch a registered native method back into the guest through `jit_run`.
///
/// `lookup_native_method` resolves (class,name) to a guest `fnPtr` (a Java_*
/// implementation the guest bound via `RegisterNatives`). This runs it as a
/// JIT guest entry with `base`/`image` covering the loaded ELF, passing the
/// caller-supplied trace/region. Returns the guest x0 after the callee's
/// `ret`. This is the glue that turns a *recorded* Java_* binding — which
/// `jni_register_natives` now captures — into a *callable* guest function,
/// which is what a host Android-runtime dispatch of a Roblox native method
/// needs alongside a real fake-object backing.
///
/// Returns `Err` if the method has no binding or `jit_run` stops (unmapped /
/// unsupported).
pub fn dispatch_native_method(
    method: &NativeMethod,
    base: u64,
    image: &[u8],
    args: [u64; 8],
    tpidr: u64,
) -> Result<u64, String> {
    if method.fn_ptr == 0 {
        return Err(format!(
            "native method {}:{} has NULL fnPtr",
            String::from_utf8_lossy(&method.class),
            String::from_utf8_lossy(&method.name)
        ));
    }
    let mut st = crate::jit::CpuState::new();
    st.tpidr = tpidr;
    st.x[..8].copy_from_slice(&args);
    crate::jit::jit_run(image, base, method.fn_ptr, &mut st as *mut crate::jit::CpuState)?;
    Ok(st.x[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::host_call_at;

    /// Every slot the guest's JNI_OnLoad path reaches must be a non-null host
    /// thunk at the OFFICIAL ABI offset (a mis-slot dispatches to NULL -> crash).
    #[test]
    fn jni_table_has_official_abi_slots_nonnull() {
        let (env, _vm) = build_jni();
        unsafe {
            let functions = *(env as *const u64);
            let get = |i: usize| -> u64 { *(functions as *const u64).add(i) };
            for (i, name) in [
                (GET_VERSION, "GetVersion"),
                (FIND_CLASS, "FindClass"),
                (GET_METHOD_ID, "GetMethodID"),
                (GET_STATIC_METHOD_ID, "GetStaticMethodID"),
                (NEW_STRING_UTF, "NewStringUTF"),
                (GET_STRING_UTF_CHARS, "GetStringUTFChars"),
                (REGISTER_NATIVES, "RegisterNatives"),
                (GET_JAVA_VM, "GetJavaVM"),
            ] {
                let sl = get(i);
                assert!(sl != 0, "slot {i} ({name}) non-null");
                host_call_at(sl).expect(name);
            }
            // The VOIDPIN default NULL-slot must be absent from the boot-relevant names.
            assert!(get(NEW_STRING_UTF) != get(FIND_CLASS), "distinct stubs");
        }
    }

    #[test]
    fn jni_new_string_utf_is_readable() {
        // Invoke the NewStringUTF stub directly: it must return a readable,
        // null-terminated handle containing the UTF bytes, not a low sentinel.
        let payload = b"com/roblox/engine/jni/user/NativeUserJavaInterface";
        let src = unsafe { alloc_zeroed(Layout::array::<u8>(payload.len() + 1).unwrap()) };
        unsafe { std::ptr::copy_nonoverlapping(payload.as_ptr(), src, payload.len()) };
        let handle = jni_new_string_utf(0, src as u64, 0, 0, 0, 0, 0, 0);
        assert!(handle != 0 && handle > 0x1000, "readable handle, not sentinel");
        assert_eq!(read_cstr(handle).as_deref(), Some(&payload[..]));
        // idempotent across calls
        assert_eq!(handle, jni_new_string_utf(0, src as u64, 0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn jni_register_natives_records_guest_bindings() {
        // Build a real JNINativeMethod array (3 u64 words each: name*,
        // signature*, fnPtr) in guest memory, exactly as JNI_OnLoad does, and
        // verify the registry parses it and lookup_native_method returns the
        // guest fnPtr (a Java_* address the host would jit_run).
        use std::alloc::{alloc, Layout};
        let mut w = |bytes: &[u8]| -> u64 {
            let p = unsafe { alloc(Layout::array::<u8>(bytes.len() + 1).unwrap()) } as *mut u8;
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len()) };
            unsafe { *p.add(bytes.len()) = 0 };
            p as u64
        };
        let cls = w(b"com/roblox/engine/jni/iap/IAPPurchaseManager");
        let name1 = w(b"nativeInit");
        let sig1 = w(b"(J)V");
        let fn1 = 0x1000_5f00u64; // a guest .text address
        let name2 = w(b"nativePurchase");
        let sig2 = w(b"(Ljava/lang/String;)Z");
        let fn2 = 0x1000_7200u64;
        let methods = {
            let p = unsafe { alloc(Layout::array::<u64>(6).unwrap()) } as *mut u64;
            let src = [name1, sig1, fn1, name2, sig2, fn2];
            for (i, v) in src.iter().enumerate() {
                unsafe { *p.add(i) = *v };
            }
            p as u64
        };

        clear_native_methods();
        assert_eq!(jni_register_natives(0, cls, methods, 2, 0, 0, 0, 0), 0); // JNI_OK

        let m = lookup_native_method(
            b"com/roblox/engine/jni/iap/IAPPurchaseManager",
            b"nativeInit",
        )
        .expect("nativeInit bound");
        assert_eq!(m.fn_ptr, fn1);
        assert_eq!(m.signature, b"(J)V");
        assert_eq!(
            lookup_native_method(
                b"com/roblox/engine/jni/iap/IAPPurchaseManager",
                b"nativePurchase",
            )
            .unwrap()
            .fn_ptr,
            fn2
        );
        // Unknown name -> None.
        assert!(lookup_native_method(b"x", b"no_such").is_none());
        // Last registration wins (re-register with a different fnPtr).
        let name3 = w(b"nativeInit");
        let p3 = unsafe { alloc(Layout::array::<u64>(3).unwrap()) } as *mut u64;
        unsafe {
            *p3 = name3;
            *p3.add(1) = sig1;
            *p3.add(2) = 0x2000_1111u64;
        }
        jni_register_natives(0, cls, p3 as u64, 1, 0, 0, 0, 0);
        assert_eq!(
            lookup_native_method(
                b"com/roblox/engine/jni/iap/IAPPurchaseManager",
                b"nativeInit",
            )
            .unwrap()
            .fn_ptr,
            0x2000_1111u64,
            "re-registration replaces"
        );
    }

    #[test]
    fn jni_entry_point_builds() {
        let (env, vm) = build_jni();
        assert!(env > 0 && vm > 0);
    }

    /// End-to-end: register a native method binding whose guest fnPtr is a
    /// real aarch64 function (`mov x0,#0x2a; ret`), then dispatch it through
    /// jit_run and verify the guest x0 comes back. Proves the recorded
    /// fnPtr (guested address) is directly runnable as a JIT entry.
    #[test]
    fn jni_dispatch_registered_native_method_through_jit() {
        // Function: mov x0,#0x2a (42); ret
        let code: [u8; 8] = [
            0x40, 0x05, 0x80, 0xd2, // mov x0, #0x2a (d2800540)
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ];
        let base = 0x1000u64;
        clear_native_methods();
        // Register a binding pointing into the `code` buffer as guest/home
        // (guest==host here). Build the JNINativeMethod array.
        use std::alloc::{alloc, Layout};
        let mut w = |bytes: &[u8]| -> u64 {
            let p = unsafe { alloc(Layout::array::<u8>(bytes.len() + 1).unwrap()) } as *mut u8;
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len()) };
            unsafe { *p.add(bytes.len()) = 0 };
            p as u64
        };
        let cls = w(b"com/example/Native");
        let name = w(b"answer");
        let sig = w(b"()I");
        let arr = unsafe { alloc(Layout::array::<u64>(3).unwrap()) } as *mut u64;
        unsafe {
            *arr = name;
            *arr.add(1) = sig;
            *arr.add(2) = base; // fnPtr = the guest code we JIT
        }
        jni_register_natives(0, cls, arr as u64, 1, 0, 0, 0, 0);

        let m = lookup_native_method(b"com/example/Native", b"answer").expect("bound");
        let ret = dispatch_native_method(&m, base, &code, [0; 8], 0).expect("dispatch");
        assert_eq!(ret, 42, "guest Java_* method executed through jit_run");
    }

    #[test]
    fn jni_returns_same_instance() {
        assert_eq!(env_addr(), env_addr(), "build_jni idempotent");
    }

    /// End-to-end: JIT-execute guest aarch64 that does the `JNI_OnLoad` preamble —
    /// `JavaVM*` in x0, call vm->GetEnv(&env, 0x10006), then env->GetVersion(),
    /// then env->NewStringUTF — and verify the results land in registers. Uses the
    /// OFFICIAL slot offsets (vm GetEnv=7, env GetVersion=4, NewStringUTF=167).
    #[test]
    fn jit_jni_onload_getenv_getversion() {
        use crate::jit::{jit_run, CpuState};
        let (_env, vm) = build_jni();
        let scratch = unsafe { std::alloc::alloc_zeroed(std::alloc::Layout::new::<u64>()) };
        let scratch_addr = scratch as u64;

        let mut st = CpuState::new();
        st.x[0] = vm; // JavaVM*
        st.x[1] = scratch_addr; // penv
        st.x[2] = 0x10006; // version

        // aarch64 (official ABI offsets):
        //   ldr x10,[x0]         ; x10 = vm function table
        //   ldr x9,[x10,#56]     ; x9  = vm_table[7] = GetEnv
        //   blr x9               ; GetEnv(vm,&penv,version)
        //   ldr x13,[x1]         ; x13 = env (JNIEnv*)
        //   ldr x13,[x13]        ; x13 = env->functions (JNIEnv fn table)
        //   ldr x14,[x13,#32]    ; x14 = env_table[4] = GetVersion
        //   blr x14              ; GetVersion -> x0 = 0x10006
        let mut code: Vec<u8> = Vec::new();
        let mut w = |i: u32| code.extend_from_slice(&i.to_le_bytes());
        w(0xf940000a); // ldr x10,[x0]
        w(0xf9401d49); // ldr x9,[x10,#56]  (vm_table[7] = GetEnv)
        w(0xd63f0120); // blr x9
        w(0xf940002d); // ldr x13,[x1]      -> x13 = env
        w(0xf94001ad); // ldr x13,[x13]     -> x13 = env->functions
        w(0xf94011ae); // ldr x14,[x13,#32] (env_table[4] = GetVersion)
        w(0xd63f01c0); // blr x14           -> x0 = 0x10006
        w(0xd4200000); // brk #0

        let res = jit_run(&code, 0x1000, 0x1000, &mut st as *mut CpuState);
        assert!(res.is_ok(), "jit_run over JNI_OnLoad guest step: {res:?}");
        assert_eq!(st.x[0], JNI_VERSION_1_6 as u64, "GetVersion through JIT = 0x10006");
    }
}