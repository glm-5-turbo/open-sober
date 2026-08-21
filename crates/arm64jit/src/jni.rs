//! Guest-visible Java Native Interface (JNI) environment + JavaVM for the JIT.
//!
//! Ports the QEMU path's `jni_shim.c` JNI function table. On Android `JNI_OnLoad`
//! receives a `JavaVM*`; it calls `vm->GetEnv(&env, JNI_VERSION_1_6)` to obtain a
//! `JNIEnv*` whose `functions` is a table of function pointers, then calls
//! `env->FindClass/RegisterNatives/Call*Method/...` through that table.
//!
//! In the JIT host thunks live in guest-address space and guest==host, so the
//! JNI table we build in guest memory holds host-thunk guest addresses directly.
//! Each JNINative slot maps to a host Rust stub with the generic `HostCall` ABI
//! (8 u64 args -> u64). This is the C file's `jni_table`/`vm_table` in
//! thunk-address form.

use crate::jit::{register_host_call_auto, HostCall};

pub const JNI_VERSION_1_6: i32 = 0x0001_0006;
const JNI_SLOTS: usize = 256;
const VM_SLOTS: usize = 8;

macro_rules! jni_stub {
    ($name:ident, $val:expr) => {
        extern "C" fn $name(
            _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
        ) -> u64 {
            $val
        }
    };
}

jni_stub!(jni_get_version, JNI_VERSION_1_6 as u64); // slot 4 GetVersion -> 0x10006
jni_stub!(jni_voidp_0, 0); // default NULL/0 (FindClass/NewString/...)
jni_stub!(jni_int_0, 0); // int/status-returning
jni_stub!(jni_trackptr, 0x3000); // stable non-null field/method id sentinel
jni_stub!(jni_env_slot_0, 0); // (n/a) reserved

/// JVM `GetEnv`: `jint GetEnv(JavaVM*, void** penv, jint version)` writes the
/// current thread's JNIEnv into `*penv` and returns JNI_OK (0). The JNIEnv is
/// the singleton built by `build_jni`; args are a1 (u64) = &env.
extern "C" fn jni_vm_getenv(
    _vm: u64, penv: u64, _version: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    let (env, _vm) = build_jni();
    unsafe { *(penv as *mut u64) = env };
    0 // JNI_OK
}

/// Build the singleton guest-visible JNIEnv and JavaVM, each with a pointer table
/// whose slots are host-thunk guest addresses. Returns `(jni_env_guest, java_vm_guest)`.
pub fn build_jni() -> (u64, u64) {
    use std::sync::OnceLock;
    static ONE: OnceLock<(u64, u64)> = OnceLock::new();
    *ONE.get_or_init(|| {
        // Register each distinct stub once, then point table slots at them.
        let get_version = reg(jni_get_version);
        let voidp = reg(jni_voidp_0);
        let int0 = reg(jni_int_0);
        let track = reg(jni_trackptr);

        let mut functions: Vec<u64> = vec![voidp; JNI_SLOTS];
        functions[4] = get_version; // GetVersion
        functions[5] = track; // FindClass
        functions[6] = track; // FindClass(from)
        functions[7] = track; // GetMethodID
        functions[8] = track; // GetFieldID
        functions[13] = int0; // Throw
        functions[14] = int0; // ThrowNew
        functions[21] = track; // NewGlobalRef
        functions[22] = int0; // DeleteGlobalRef
        functions[23] = int0; // DeleteLocalRef
        functions[24] = track; // PushLocalFrame
        functions[25] = int0; // PopLocalFrame
        functions[33] = track; // GetStaticMethodID(??) -> sentinel
        functions[36] = track; // NewStringUTF
        functions[37] = int0; // GetArrayLength
        functions[38] = track; // NewObjectArray
        functions[39] = track; // GetObjectArrayElement
        functions[40] = int0; // SetObjectArrayElement
        functions[101] = int0; // GetIntField
        functions[102] = track; // GetObjectField
        functions[103] = int0; // SetObjectField
        functions[104] = int0; // GetBooleanField
        functions[105] = int0; // SetBooleanField
        functions[113] = track; // GetStatic...ID sentinel
        functions[193] = int0; // RegisterNatives -> 0
        functions[197] = track; // GetJavaVM -> non-null vm
        // remaining slots already = voidp (NULL/0)

        // JNIEnv struct: { JNINativeInterface* functions; const void* reserved0..3; }
        // Same layout for JavaVM: { JNIVMInterface* functions; reserved0..3; }
        let env_fn_tbl = u64array(&functions);
        let mut vm_functions = vec![voidp; VM_SLOTS];
        let vm_getenv = reg(jni_vm_getenv); // GetEnv: writes *penv=env, returns JNI_OK
        vm_functions[3] = int0; // DestroyJavaVM
        vm_functions[4] = vm_getenv; // AttachCurrentThread->GetEnv (writes env)
        vm_functions[5] = int0; // DetachCurrentThread
        vm_functions[6] = vm_getenv; // GetEnv
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
    use std::alloc::{alloc_zeroed, Layout};
    let n = items.len();
    let p = unsafe { alloc_zeroed(Layout::array::<u64>(n).unwrap()) } as *mut u64;
    for (i, v) in items.iter().enumerate() {
        unsafe { *p.add(i) = *v };
    }
    p as u64
}

/// Allocate a 5-word object whose word0 = `functions` pointer (JNIEnv/JavaVM).
fn object2(functions: u64) -> u64 {
    use std::alloc::{alloc_zeroed, Layout};
    let p = unsafe { alloc_zeroed(Layout::new::<[u64; 5]>()) } as *mut u64;
    unsafe { *p = functions };
    p as u64
}

/// The guest address `vm->GetEnv(&env, v)` should write into `*penv`. Build and
/// return the guest JNIEnv (idempotent). Convenience for boot glue.
pub fn env_addr() -> u64 {
    let (e, _v) = build_jni();
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::host_call_at;

    #[test]
    fn jni_env_table_is_guest_callable() {
        let (env, vm) = build_jni();
        // JNIEnv word0 = functions (table addr); JavaVM word0 = its function table.
        unsafe {
            let functions = *(env as *const u64);
            let vm_functions = *(vm as *const u64);
            // Both must be non-null and lie in the high host-thunk guest space.
            assert!(functions > 0 && vm_functions > 0, "tables non-null");
            // Slot 4 (GetVersion) must be a host thunk address; the dispatcher
            // (host_call_at) recognizes it and calls the stub returning 0x10006.
            let get_version = *(functions as *const u64).add(4);
            let (hostf, _slot) = host_call_at(get_version).expect("GetVersion thunk registered");
            let ret = hostf(0, 0, 0, 0, 0, 0, 0, 0);
            assert_eq!(ret, JNI_VERSION_1_6 as u64, "GetVersion returns 0x10006");
        }
    }

    #[test]
    fn jni_returns_same_instance() {
        assert_eq!(env_addr(), env_addr(), "build_jni idempotent");
    }

    /// End-to-end: JIT-execute guest aarch64 that does the `JNI_OnLoad` preamble —
    /// `JavaVM*` in x0, call vm->GetEnv(&env, 0x10006), then env->GetVersion() —
    /// and verify the version lands in a register. Proves the guest indirect
    /// calls through both function tables dispatch into host thunks correctly.
    #[test]
    fn jit_jni_onload_getenv_getversion() {
        use crate::jit::{jit_run, CpuState};
        let (_env, vm) = build_jni();
        // scratch word to receive *penv (guest==host, so a host alloc is also a
        // dereferenceable guest address)
        let scratch = unsafe { std::alloc::alloc_zeroed(std::alloc::Layout::new::<u64>()) };
        let scratch_addr = scratch as u64;

        let mut st = CpuState::new();
        st.x[0] = vm; // JavaVM* in x0 (as Android would pass to JNI_OnLoad)
        st.x[1] = scratch_addr; // penv = &env-word
        st.x[2] = 0x10006; // version

        // aarch64:
        //   ldr x10,[x0]         ; x10 = vm function table
        //   ldr x9,[x10,#48]     ; x9  = vm_table[6] = GetEnv
        //   blr x9               ; GetEnv(vm,&penv,version) -> x0=JNI_OK; writes *penv=env
        //   ldr x13,[x1]         ; x13 = env (JNIEnv*)
        //   ldr x13,[x13]        ; x13 = env function table
        //   ldr x14,[x13,#32]    ; x14 = env_table[4] = GetVersion
        //   blr x14              ; GetVersion(env) -> x0
        //   brk #0               ; halt
        let mut code: Vec<u8> = Vec::new();
        let mut w = |i: u32| code.extend_from_slice(&i.to_le_bytes());
        w(0xf940000a); // ldr x10,[x0]
        w(0xf9401949); // ldr x9,[x10,#48]
        w(0xd63f0120); // blr x9
        w(0xf940002d); // ldr x13,[x1]
        w(0xf94001ad); // ldr x13,[x13]
        w(0xf94011ae); // ldr x14,[x13,#32]
        w(0xd63f01c0); // blr x14
        w(0xd4200000); // brk #0

        let res = jit_run(&code, 0x1000, 0x1000, &mut st as *mut CpuState);
        assert!(res.is_ok(), "jit_run over JNI_OnLoad guest step: {res:?}");
        // GetVersion returned through the host thunk into x0.
        assert_eq!(
            st.x[0], JNI_VERSION_1_6 as u64,
            "GetVersion through JIT = 0x10006"
        );
    }
}