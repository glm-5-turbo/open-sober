//! Resolve guest `.so` imports (libc/libm/bionic names) to host x86-64 call
//! slots in the JIT's host-call bridge.
//!
//! When the JIT dispatcher re-enters at a `HOST_THUNK_BASE + slot*8` address,
//! it invokes the registered `HostCall` with the guest x0..x7 args (SysV GPR
//! convention) and stores its return into guest x0. This module assigns each
//! import *name* to a stable slot and points that slot at the real host
//! function found via `dlsym`. A loader then patches the guest's
//! `R_AARCH64_JUMP_SLOT` GOT entry (or answer) to the slot's guest address.
//!
//! ABI note: this resolves functions whose args/return travel in GPRs
//! (integers/pointers) — strlen, memcpy, memcmp, strcmp, abs, etc. Float
//! args/results (sinf/powf/...) use XMM registers and need a separate
//! float-ABI path, added later.

use crate::jit::{host_call_addr, register_host_call, register_float_call, HostCall, HostFloatCall};
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

/// Allocates thunk slots and remembers name -> slot addr.
struct Resolver {
    /// import name -> HOST_THUNK guest address of its call slot
    slots: HashMap<CString, u64>,
    next: usize,
}

impl Resolver {
    fn new() -> Self {
        // Start at a high slot index to avoid colliding with any consumer that
        // uses the low slots directly (e.g. the jit `host_call_bridge`
        // validation test registers slot 1 by hand).
        Resolver {
            slots: HashMap::new(),
            next: 1000,
        }
    }
}

fn resolver() -> &'static Mutex<Resolver> {
    static R: OnceLock<Mutex<Resolver>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(Resolver::new()))
}

/// Give an import name a host call slot. If the host symbol is found via
/// `dlsym`, register it and return the thunk's *guest address*; if the name
/// can't be resolved on the host, return `None` (caller must decide how to
/// handle a missing import).
pub fn resolve(name: &[u8]) -> Option<u64> {
    let mut r = resolver().lock().unwrap();
    let key = CString::new(name).ok()?;
    if let Some(addr) = r.slots.get(&key) {
        return Some(*addr);
    }
    // dlsym the host symbol. We are resolving against the process-global
    // symbol space (libc/libm/any shared lib already loaded), which covers
    // the aarch64 libc/libm imports whose names collide with host names.
    let sym = key.as_ptr();
    let ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, sym) };
    if ptr.is_null() {
        return None; // not present on the host
    }
    if r.next >= crate::jit::HOST_THUNK_MAX {
        return None;
    }
    let slot = r.next;
    r.next += 1;
    // Cast: `HostCall` takes 8 u64 args -> u64, which matches the SysV GPR
    // ABI for integer/pointer-returning C functions. SAFETY: we only resolve
    // GPR-ABI functions; the caller agrees not to route XMM-ABI funcs here.
    let hostf: HostCall = unsafe { std::mem::transmute(ptr) };
    register_host_call(slot, hostf);
    let addr = host_call_addr(slot);
    r.slots.insert(key, addr);
    Some(addr)
}

/// Convenience: resolve and return the slot address (panics if unresolved).
pub fn require(name: &str) -> u64 {
    resolve(name.as_bytes()).expect("host symbol not resolvable")
}

/// Resolve a **double-precision** float-ABI import to a float thunk guest addr.
/// The guest (Roblox) passes doubles in v0-v7; our float bridge reads those
/// lanes as f64 and calls the host double function through xmm0-xmm7. Only
/// double-precision names are safe here (single-precision `*f` need f32 lane
/// handling and are intentionally excluded).
pub fn resolve_float(name: &[u8]) -> Option<u64> {
    let key = CString::new(name).ok()?;
    let sym = key.as_ptr();
    let ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, sym) };
    if ptr.is_null() {
        return None;
    }
    // Host f64 -> f64 via double (xmm0..) ABI = `HostFloatCall`.
    let hostf: HostFloatCall = unsafe { std::mem::transmute(ptr) };
    Some(crate::jit::register_float_call(hostf))
}

/// Double-precision libm names whose f64 ABI matches our float bridge.
pub const DOUBLE_FLOAT_NAMES: &[&str] = &[
    "atan2", "atan", "asin", "acos", "sin", "cos", "tan", "exp", "log", "log10", "log2",
    "pow", "sqrt", "floor", "ceil", "fabs", "fmod", "hypot", "copysign", "trunc", "round",
    "exp2", "log1p", "expm1", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
];

/// Regist directly known common imports: name -> host function. Returns a map
/// of import name -> thunk guest address for the ones the host provides.
pub fn resolve_common() -> HashMap<String, u64> {
    let names: &[&str] = &[
        "strlen",
        "strcmp",
        "strncmp",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "abs",
        "labs",
        "llabs",
        "atoi",
        "atol",
        "strtol",
        "strtoul",
        "strtod",
        "strstr",
        "strchr",
        "strrchr",
        "strcspn",
        "strspn",
        "strlen",
        "strdup",
        "strndup",
        "memchr",
        "malloc",
        "free",
        "realloc",
        "calloc",
        "rand",
        "srand",
        "atoi",
        "isalpha",
        "isdigit",
        "isalnum",
        "isupper",
        "islower",
        "toupper",
        "tolower",
        "getenv",
        "setenv",
        "qsort",
        "bsearch",
        "fabs",
        "fabsf",
        "fmax",
        "fmin",
        "floor",
        "floorf",
        "ceil",
        "ceilf",
        "round",
        "roundf",
        "trunc",
        "truncf",
        "sqrt",
        "sqrtf",
        "log10",
        "log2",
        "logf",
        "exp",
        "expf",
        "pow",
        "powf",
        "sin",
        "sinf",
        "cos",
        "cosf",
        "tan",
        "tanf",
        "fmod",
        "fmodf",
        "memcpy",
        "qsort_r",
        "__errno_location",
        "__cxa_atexit",
        "__cxa_thread_atexit_impl",
        "__android_log_print",
        "__android_log_vprint",
        "dlopen",
        "dlclose",
        "dlsym",
        "dlerror",
        "getpid",
        "getuid",
        "getgid",
        "getppid",
        "clock_gettime",
        "nanosleep",
        "futex",
        "pthread_mutex_lock",
        "pthread_mutex_unlock",
        "pthread_mutex_init",
        "pthread_mutex_destroy",
        "pthread_cond_wait",
        "pthread_cond_broadcast",
        "pthread_cond_signal",
        "pthread_cond_destroy",
        "pthread_mutexattr_init",
        "pthread_mutexattr_destroy",
        "pthread_mutexattr_settype",
        "pthread_key_create",
        "pthread_getspecific",
        "pthread_setspecific",
        "pthread_once",
        "pthread_self",
        "pthread_cleanup_push",
        "pthread_cleanup_pop",
        "pthread_condattr_init",
        "pthread_condattr_setclock",
        "usleep",
        "sysconf",
    ];
    let mut out = HashMap::new();
    for n in names {
        if let Some(addr) = resolve(n.as_bytes()) {
            out.insert(n.to_string(), addr);
        }
    }
    out
}

/// Readable C string for debug/log.
pub fn cstr(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::{jit_run, CpuState};

    #[test]
    fn resolve_strlen_and_call_via_guest_blr() {
        // Resolve the *host* `strlen` into a thunk slot, then make guest code
        // `blr` to that slot and confirm the host strlen runs on a guest C
        // string and returns its length into x0.
        let slot = resolve(b"strlen").expect("host strlen resolvable");
        // Put a C string somewhere the guest and host both can read. Use host
        // memory directly (guest==host addressing), store the ptr in x0.
        let msg = b"hello-roblox\0";
        // Guest code: x0 = &msg (set from host), blr x16 (x16 = slot), brk.
        let mut img: Vec<u8> = Vec::new();
        img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
        img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0

        let msg_ptr = msg.as_ptr() as u64;
        let mut st = CpuState::new();
        st.x[0] = msg_ptr; // arg0 = the string
        st.x[16] = slot; // target = host strlen slot
        let r = jit_run(&img, 0x1000, 0x1000, &mut st as *mut CpuState).expect("jit_run");
        assert_eq!(
            r as usize,
            msg.len() - 1,
            "guest blr to host strlen(hello-roblox) == 12"
        );
    }

    #[test]
    fn resolve_common_has_strlib() {
        let m = resolve_common();
        assert!(m.contains_key("strlen"), "common imports include strlen");
        assert!(m.contains_key("memcpy"), "common imports include memcpy");
        assert!(m.contains_key("abs"), "common imports include abs");
    }
}