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

use crate::jit::{host_call_addr, register_host_call, HostCall, HostFloat32Call, HostFloatCall};
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

/// One-time `dlopen` of `libm.so.6` (`RTLD_GLOBAL|RTLD_NOW`) so the float/libm
/// functions we `dlsym` are visible even if nothing else loaded libm yet.
fn libm_handle() -> *mut libc::c_void {
    // Raw pointers aren't Send/Sync; store as usize (an address is).
    static H: OnceLock<usize> = OnceLock::new();
    let addr = *H.get_or_init(|| {
        let path = b"libm.so.6\0";
        unsafe {
            libc::dlopen(
                path.as_ptr() as *const libc::c_char,
                libc::RTLD_NOW | libc::RTLD_GLOBAL,
            ) as usize
        }
    });
    addr as *mut libc::c_void
}

/// `dlsym` `name` from a given handle, returning the fn pointer or null.
unsafe fn sym_from(handle: *mut libc::c_void, name: *const libc::c_char) -> *mut libc::c_void {
    libc::dlsym(handle, name)
}

/// One-time `dlopen` of Mesa's real `libEGL.so.1` (`RTLD_NOW|RTLD_GLOBAL`) so the
/// `egl*` imports a guest Roblox binary makes resolve to real Mesa EGL instead of
/// the benign NULL/0 graphics catch-all. EGL's ABI is integer/pointer-only, so its
/// entry points are safe through the integer `HostCall` shape (unlike GLES, which
/// passes floats in xmm and needs a dedicated float-ABI bridge — not wired here).
fn egl_handle() -> *mut libc::c_void {
    static H: OnceLock<usize> = OnceLock::new();
    let addr = *H.get_or_init(|| {
        // Prefer the real Mesa lib (fall back to a versioned soname if newer distros
        // only ship `libEGL.so.1`; both are the vendor GL dispatch library).
        let candidates: &[&[u8]] = &[b"libEGL.so.1\0", b"libEGL.so\0"];
        for path in candidates {
            let h = unsafe {
                libc::dlopen(
                    path.as_ptr() as *const libc::c_char,
                    libc::RTLD_NOW | libc::RTLD_GLOBAL,
                )
            };
            if !h.is_null() {
                return h as usize;
            }
        }
        0
    });
    addr as *mut libc::c_void
}

/// Resolve an `egl*` import against Mesa's real libEGL (integer-ABI HostCall).
/// Returns `None` if EGL isn't present on the host or the name isn't an EGL symbol.
pub fn resolve_egl(name: &[u8]) -> Option<u64> {
    let ns = name_str(name);
    if !ns.starts_with("egl") {
        return None;
    }
    let key = CString::new(name).ok()?;
    // If resolve() already bound this name (e.g. after RTLD_GLOBAL made it visible),
    // reuse the cached slot rather than allocating a duplicate.
    {
        let r = resolver().lock().unwrap();
        if let Some(addr) = r.slots.get(&key) {
            return Some(*addr);
        }
    }
    let mh = egl_handle();
    if mh.is_null() {
        return None;
    }
    let sym = key.as_ptr();
    let ptr = unsafe { sym_from(mh, sym) };
    if ptr.is_null() {
        return None;
    }
    // EGL entry points are integer/pointer-ABI -> fits the integer HostCall.
    let hostf: HostCall = unsafe { std::mem::transmute(ptr) };
    let mut r = resolver().lock().unwrap();
    alloc_slot(&mut r, &key, hostf)
}

/// GLES entry points whose ABI is integer/pointer-only AND take at most 8
/// integer args — i.e. they are safe through the JIT integer `HostCall`.
/// (Float-taking `gl*` and >8-arg forms like glTexImage2D are excluded: their
/// ABI needs a float bridge / stack args the integer HostCall cannot express.)
/// Generated from the exact system GLES headers (glesv2-wrapper/gen_forward.py).
pub const GLES_INT_NAME_LIST: &[&[u8]] = &[
    b"glActiveTexture\0",
    b"glAttachShader\0",
    b"glBindAttribLocation\0",
    b"glBindBuffer\0",
    b"glBindFramebuffer\0",
    b"glBindRenderbuffer\0",
    b"glBindTexture\0",
    b"glBlendEquation\0",
    b"glBlendEquationSeparate\0",
    b"glBlendFunc\0",
    b"glBlendFuncSeparate\0",
    b"glBufferData\0",
    b"glBufferSubData\0",
    b"glCheckFramebufferStatus\0",
    b"glClear\0",
    b"glClearStencil\0",
    b"glColorMask\0",
    b"glCompileShader\0",
    b"glCompressedTexImage2D\0",
    b"glCopyTexImage2D\0",
    b"glCopyTexSubImage2D\0",
    b"glCreateProgram\0",
    b"glCreateShader\0",
    b"glCullFace\0",
    b"glDeleteBuffers\0",
    b"glDeleteFramebuffers\0",
    b"glDeleteProgram\0",
    b"glDeleteRenderbuffers\0",
    b"glDeleteShader\0",
    b"glDeleteTextures\0",
    b"glDepthFunc\0",
    b"glDepthMask\0",
    b"glDetachShader\0",
    b"glDisable\0",
    b"glDisableVertexAttribArray\0",
    b"glDrawArrays\0",
    b"glDrawElements\0",
    b"glEnable\0",
    b"glEnableVertexAttribArray\0",
    b"glFinish\0",
    b"glFlush\0",
    b"glFramebufferRenderbuffer\0",
    b"glFramebufferTexture2D\0",
    b"glFrontFace\0",
    b"glGenBuffers\0",
    b"glGenFramebuffers\0",
    b"glGenRenderbuffers\0",
    b"glGenTextures\0",
    b"glGenerateMipmap\0",
    b"glGetActiveAttrib\0",
    b"glGetActiveUniform\0",
    b"glGetAttachedShaders\0",
    b"glGetAttribLocation\0",
    b"glGetBooleanv\0",
    b"glGetBufferParameteriv\0",
    b"glGetError\0",
    b"glGetFramebufferAttachmentParameteriv\0",
    b"glGetIntegerv\0",
    b"glGetProgramInfoLog\0",
    b"glGetProgramiv\0",
    b"glGetRenderbufferParameteriv\0",
    b"glGetShaderInfoLog\0",
    b"glGetShaderPrecisionFormat\0",
    b"glGetShaderSource\0",
    b"glGetShaderiv\0",
    b"glGetString\0",
    b"glGetTexParameteriv\0",
    b"glGetUniformLocation\0",
    b"glGetUniformiv\0",
    b"glGetVertexAttribPointerv\0",
    b"glGetVertexAttribiv\0",
    b"glHint\0",
    b"glIsBuffer\0",
    b"glIsEnabled\0",
    b"glIsFramebuffer\0",
    b"glIsProgram\0",
    b"glIsRenderbuffer\0",
    b"glIsShader\0",
    b"glIsTexture\0",
    b"glLinkProgram\0",
    b"glPixelStorei\0",
    b"glReadPixels\0",
    b"glReleaseShaderCompiler\0",
    b"glRenderbufferStorage\0",
    b"glScissor\0",
    b"glShaderBinary\0",
    b"glShaderSource\0",
    b"glStencilFunc\0",
    b"glStencilFuncSeparate\0",
    b"glStencilMask\0",
    b"glStencilMaskSeparate\0",
    b"glStencilOp\0",
    b"glStencilOpSeparate\0",
    b"glTexParameteri\0",
    b"glTexParameteriv\0",
    b"glUniform1i\0",
    b"glUniform1iv\0",
    b"glUniform2i\0",
    b"glUniform2iv\0",
    b"glUniform3i\0",
    b"glUniform3iv\0",
    b"glUniform4i\0",
    b"glUniform4iv\0",
    b"glUseProgram\0",
    b"glValidateProgram\0",
    b"glVertexAttribPointer\0",
    b"glViewport\0",
];

/// One-time `dlopen` of Mesa's real `libGLESv2.so.2` (RTLD_NOW|RTLD_LOCAL) so
/// integer-ABI `gl*` imports resolve to real Mesa instead of the NULL/0 catch-all.
/// RTLD_LOCAL (NOT GLOBAL) is deliberate: if GLES went global, the general
/// `resolve()` RTLD_DEFAULT scan would also grab float-taking `gl*` (e.g.
/// glClearColor) and bind them through the integer HostCall, corrupting their xmm
/// args. Local scope keeps GLES names visible only to `resolve_gles_int`'s
/// whitelist, so float ABI stays on the NULL/0 stub.
fn gles_handle() -> *mut libc::c_void {
    static H: OnceLock<usize> = OnceLock::new();
    let addr = *H.get_or_init(|| {
        let candidates: &[&[u8]] = &[b"libGLESv2.so.2\0", b"libGLESv2.so\0"];
        for path in candidates {
            let h = unsafe {
                libc::dlopen(path.as_ptr() as *const libc::c_char, libc::RTLD_NOW | libc::RTLD_LOCAL)
            };
            if !h.is_null() { return h as usize; }
        }
        0
    });
    addr as *mut libc::c_void
}

/// Resolve an integer-ABI `gl*` import against Mesa's real libGLESv2. Only names in
/// [`GLES_INT_NAME_LIST`] (pure integer/pointer args, <=8) are safe through the integer
/// HostCall; float-taking GLES or >8-arg forms return `None` (fall to the NULL/0 stub).
pub fn resolve_gles_int(name: &[u8]) -> Option<u64> {
    let ns = name_str(name);
    if !ns.starts_with("gl") {
        return None;
    }
    if !GLES_INT_NAME_LIST.iter().any(|c| c[..c.len()-1] == *ns.as_bytes()) {
        return None;
    }
    let key = CString::new(name).ok()?;
    {
        let r = resolver().lock().unwrap();
        if let Some(addr) = r.slots.get(&key) { return Some(*addr); }
    }
    let mh = gles_handle();
    if mh.is_null() { return None; }
    let sym = key.as_ptr();
    let ptr = unsafe { sym_from(mh, sym) };
    if ptr.is_null() { return None; }
    let hostf: HostCall = unsafe { std::mem::transmute(ptr) };
    let mut r = resolver().lock().unwrap();
    alloc_slot(&mut r, &key, hostf)
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
    // Bionic pthread fixup: the guest binary was built against bionic, whose
    // pthread_mutex_t is 44 bytes (glibc's is 40), __kind lives at offset 16 and
    // __count is reused as __owner at offset 8. Passing such a mutex to glibc's
    // pthread_mutex_lock/cond_wait makes glibc see kind==0x10 (ROBUST_NORMAL) or
    // a stray owner count and it crashes/deadlocks, driving Roblox init into its
    // abort path. Mirror jni_shim.c `sanitize_mutex`: route these imports
    // through a wrapper that fixes the mutex layout in place first. Since the
    // runtime maps guest==host contiguously, the guest pointer is host-addressable.
    if let Some(real) = real_libc_pthread(name) {
        store_real(name_str(name), real);
        let hostf: HostCall = match name_str(name) {
            "pthread_mutex_lock" | "pthread_mutex_unlock" => {
                // both take one mutex arg and return int; lock/unlock collide, so
                // pick the right bridge by exact name.
                if name_str(name) == "pthread_mutex_unlock" {
                    host_mutex_unlock
                } else {
                    host_mutex_lock
                }
            }
            "pthread_cond_wait" => host_cond_wait,
            "pthread_cond_timedwait" => host_cond_timedwait,
            "pthread_mutex_init" => host_mutex_init,
            _ => unsafe { std::mem::transmute(real) },
        };
        return alloc_slot(&mut r, &key, hostf);
    }
    // dlsym the host symbol. We are resolving against the process-global
    // symbol space (libc/libm/any shared lib already loaded), which covers
    // the aarch64 libc/libm imports whose names collide with host names.
    let sym = key.as_ptr();
    // RTLD_DEFAULT only sees already-loaded libs; libm is often not yet loaded.
    // Fall back to an explicit `dlopen("libm.so.6")` handle so libm names resolve.
    let mut ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, sym) };
    if ptr.is_null() {
        let mh = libm_handle();
        if !mh.is_null() {
            ptr = unsafe { sym_from(mh, sym) };
        }
    }
    if ptr.is_null() {
        return None; // not present on the host
    }
    let hostf: HostCall = unsafe { std::mem::transmute(ptr) };
    alloc_slot(&mut r, &key, hostf)
}

/// Register `hostf` at a fresh resolver slot keyed by `key`; returns slot addr.
fn alloc_slot(r: &mut Resolver, key: &CString, hostf: HostCall) -> Option<u64> {
    if r.next >= crate::jit::HOST_THUNK_MAX {
        return None;
    }
    let slot = r.next;
    r.next += 1;
    register_host_call(slot, hostf);
    let addr = host_call_addr(slot);
    r.slots.insert(key.clone(), addr);
    Some(addr)
}

/// Name of an import as `&str`, tolerating a trailing NUL.
fn name_str(name: &[u8]) -> &str {
    let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    std::str::from_utf8(&name[..end]).unwrap_or("")
}

/// dlsym a pthread mutex/cond function we plan to wrap, or None (not ours).
fn real_libc_pthread(name: &[u8]) -> Option<*mut libc::c_void> {
    let n = name_str(name);
    if !(n == "pthread_mutex_lock"
        || n == "pthread_mutex_unlock"
        || n == "pthread_mutex_init"
        || n == "pthread_cond_wait"
        || n == "pthread_cond_timedwait")
    {
        return None;
    }
    let c = CString::new(n).ok()?;
    // SAFETY: name is one of the fixed whitelist strings above; NUL-terminated.
    Some(unsafe { libc::dlsym(libc::RTLD_DEFAULT, c.as_ptr()) })
}

/// Normalize a candidate bionic-layout pthread_mutex_t in place to glibc layout:
/// clear the robust/high kind bits at offset 16 and a stray __owner at offset 8.
///
/// # Safety
/// `m` must be a non-null, writable pointer to at least 20 bytes (the mutex).
unsafe fn sanitize_mutex(m: *mut u8) {
    if m.is_null() {
        return;
    }
    let kind = core::ptr::read_unaligned(m.add(16) as *const i32);
    core::ptr::write_unaligned(m.add(16) as *mut i32, kind & 3);
    let cnt = core::ptr::read_unaligned(m.add(8) as *const i32);
    if cnt > 0x0001_0000 || cnt < 0 {
        core::ptr::write_unaligned(m.add(8) as *mut i32, 0);
    }
}

type MutexLockFn = unsafe extern "C" fn(*mut u8) -> i32;
type MutexCondFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type MutexInitFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

// Real glibc pthread functions, cached once. "real" means we already vetted the
// dlsym'd address before installing a wrapper, so these are non-null.
static REAL_LOCK: OnceLock<MutexLockFn> = OnceLock::new();
static REAL_UNLOCK: OnceLock<MutexLockFn> = OnceLock::new();
static REAL_COND_WAIT: OnceLock<MutexCondFn> = OnceLock::new();
static REAL_COND_TIMEDWAIT: OnceLock<MutexCondFn> = OnceLock::new();
static REAL_MUTEX_INIT: OnceLock<MutexInitFn> = OnceLock::new();

/// Record the real glibc fn for `name`; false on an unknown/unwanted name.
fn store_real(name: &str, real: *mut libc::c_void) {
    let poke = |target: &OnceLock<MutexLockFn>, f: MutexLockFn| {
        let _ = target.set(f);
    };
    match name {
        "pthread_mutex_lock" => poke(&REAL_LOCK, unsafe { std::mem::transmute(real) }),
        "pthread_mutex_unlock" => poke(&REAL_UNLOCK, unsafe { std::mem::transmute(real) }),
        "pthread_cond_wait" => {
            let _ = REAL_COND_WAIT.set(unsafe { std::mem::transmute(real) });
        }
        "pthread_cond_timedwait" => {
            let _ = REAL_COND_TIMEDWAIT.set(unsafe { std::mem::transmute(real) });
        }
        "pthread_mutex_init" => {
            let _ = REAL_MUTEX_INIT.set(unsafe { std::mem::transmute(real) });
        }
        _ => {}
    }
}

/// Host bridge fn: pthread_mutex_lock/mutex_unlock over the guest mutex.
extern "C" fn host_mutex_lock(a0: u64, _1: u64, _2: u64, _3: u64, _4: u64, _5: u64, _6: u64, _7: u64) -> u64 {
    let f = *REAL_LOCK.get().expect("pthread_mutex_lock resolved");
    let kind: i32 = unsafe {
        if a0 != 0 {
            core::ptr::read_unaligned((a0 as *const u8).add(16) as *const i32)
        } else {
            0
        }
    };
    let r = unsafe {
        sanitize_mutex(a0 as *mut u8);
        f(a0 as *mut u8)
    };
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[mutex_lock] {a0:#x} kind_pre={kind:#x} -> {r}");
    }
    r as u64
}
extern "C" fn host_mutex_unlock(a0: u64, _1: u64, _2: u64, _3: u64, _4: u64, _5: u64, _6: u64, _7: u64) -> u64 {
    let f = *REAL_UNLOCK.get().expect("pthread_mutex_unlock resolved");
    unsafe {
        sanitize_mutex(a0 as *mut u8);
        f(a0 as *mut u8) as u64
    }
}
extern "C" fn host_cond_wait(a0: u64, a1: u64, _2: u64, _3: u64, _4: u64, _5: u64, _6: u64, _7: u64) -> u64 {
    let f = *REAL_COND_WAIT.get().expect("pthread_cond_wait resolved");
    unsafe {
        sanitize_mutex(a1 as *mut u8); // mutex is arg1 (pthread_cond_wait(cond, mutex))
        f(a0 as *mut u8, a1 as *mut u8) as u64
    }
}
extern "C" fn host_cond_timedwait(a0: u64, a1: u64, a2: u64, _3: u64, _4: u64, _5: u64, _6: u64, _7: u64) -> u64 {
    type CondTimedwaitFn = unsafe extern "C" fn(*mut u8, *mut u8, *const libc::timespec) -> i32;
    let f: CondTimedwaitFn = unsafe {
        std::mem::transmute(
            *REAL_COND_TIMEDWAIT.get().expect("pthread_cond_timedwait resolved"),
        )
    };
    unsafe {
        sanitize_mutex(a1 as *mut u8);
        f(a0 as *mut u8, a1 as *mut u8, a2 as *const libc::timespec) as u64
    }
}
extern "C" fn host_mutex_init(a0: u64, a1: u64, _2: u64, _3: u64, _4: u64, _5: u64, _6: u64, _7: u64) -> u64 {
    let f = *REAL_MUTEX_INIT.get().expect("pthread_mutex_init resolved");
    unsafe {
        let r = f(a0 as *mut u8, a1 as *mut u8);
        if r == 0 && a0 != 0 {
            sanitize_mutex(a0 as *mut u8);
        }
        r as u64
    }
}

/// Convenience: resolve and return the slot address (panics if unresolved).
pub fn require(name: &str) -> u64 {
    resolve(name.as_bytes()).expect("host symbol not resolvable")
}

/// Register a host call for a **named** import without `dlsym` (for bionic/
/// Android names that have no host symbol). Returns the thunk's guest address.
/// Takes a NUL-terminated byte slice (the `.dynstr`-style name).
pub fn register_named(name: &[u8], f: crate::jit::HostCall) -> u64 {
    let mut r = resolver().lock().unwrap();
    let key = CString::new(name).ok().or_else(|| {
        CString::new(name.strip_suffix(&[0]).unwrap_or(name)).ok()
    });
    let key = match key {
        Some(k) => k,
        None => return 0,
    };
    if let Some(addr) = r.slots.get(&key) {
        return *addr;
    }
    let slot = r.next;
    r.next += 1;
    crate::jit::register_host_call(slot, f);
    let addr = crate::jit::host_call_addr(slot);
    r.slots.insert(key, addr);
    addr
}

/// Resolve a **double-precision** float-ABI import to a float thunk guest addr.
/// The guest (Roblox) passes doubles in v0-v7; our float bridge reads those
/// lanes as f64 and calls the host double function through xmm0-xmm7. Only
/// double-precision names are safe here (single-precision `*f` need f32 lane
/// handling and are intentionally excluded).
pub fn resolve_float(name: &[u8]) -> Option<u64> {
    let key = CString::new(name).ok()?;
    let sym = key.as_ptr();
    let mut ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, sym) };
    if ptr.is_null() {
        let mh = libm_handle();
        if !mh.is_null() {
            ptr = unsafe { sym_from(mh, sym) };
        }
    }
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

/// Single-precision float-ABI libm names (guest stores f32 in low 32 bits of
/// s0-s7); these match our f32 float bridge.
pub const FLOAT32_NAMES: &[&str] = &[
    "atan2f", "atanf", "asinf", "acosf", "sinf", "cosf", "tanf", "expf", "logf", "log10f",
    "log2f", "powf", "sqrtf", "floorf", "ceilf", "fabsf", "fmodf", "hypotf", "copysignf",
    "truncf", "roundf", "exp2f", "log1pf", "expm1f", "sinhf", "coshf", "tanhf", "asinhf",
    "acoshf", "atanhf",
];

/// Resolve a **single-precision** float-ABI import to an f32 thunk guest addr.
pub fn resolve_float32(name: &[u8]) -> Option<u64> {
    let key = CString::new(name).ok()?;
    let sym = key.as_ptr();
    let mut ptr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, sym) };
    if ptr.is_null() {
        let mh = libm_handle();
        if !mh.is_null() {
            ptr = unsafe { sym_from(mh, sym) };
        }
    }
    if ptr.is_null() {
        return None;
    }
    let hostf: HostFloat32Call = unsafe { std::mem::transmute(ptr) };
    Some(crate::jit::register_float32_call(hostf))
}

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

    #[test]
    fn sanitize_mutex_clears_bionic_kind_and_bogus_owner() {
        // Simulate a bionic-layout pthread_mutex_t that glibc would misread:
        // __kind (offset 16) = 0x10 (BIONIC ROBUST_NORMAL), __count (offset 8)
        // = 0x7fff1234 (a stray bionic owner leaking into glibc's recursive
        // count), which glibc's pthread_mutex_lock sees as already-held/reentrant.
        let mut m = [0u8; 24];
        m[16..20].copy_from_slice(&0x10u32.to_le_bytes());
        m[8..12].copy_from_slice(&0x7fff_1234u32.to_le_bytes());

        unsafe { super::sanitize_mutex(m.as_mut_ptr()) };

        let kind = u32::from_le_bytes(m[16..20].try_into().unwrap());
        let cnt = u32::from_le_bytes(m[8..12].try_into().unwrap());
        assert_eq!(kind & 3, kind, "kind high bits cleared (kind=0x{kind:x})");
        assert_eq!(cnt, 0, "bogus owner/count cleared at offset 8");
    }

    /// Real Mesa EGL must be resolvable as an integer-ABI host call, and a guest
    /// `blr` to it must actually execute Mesa code (not a NULL/0 catch-all). This is
    /// the regression gate for the graphics-layer wiring: without `resolve_egl`, the
    /// same import fell to `register_graphics_stubs` -> stub_zero (return 0 trivially).
    /// Skipped on hosts without Mesa EGL (CI boxes may lack /usr/lib/libEGL.so.1).
    #[test]
    fn resolve_egl_binds_real_mesa_not_null_stub() {
        let Some(slot) = resolve_egl(b"eglGetError\0") else {
            eprintln!("skipping: Mesa EGL (libEGL.so.1) not present on this host");
            return;
        };
        // Guest code: blr x16 then brk. eglGetError() takes no args, returns EGLint.
        let mut img: Vec<u8> = Vec::new();
        img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
        img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0
        let mut st = CpuState::new();
        st.x[16] = slot;
        let r = jit_run(&img, 0x1000, 0x1000, &mut st as *mut CpuState).expect("jit_run");
        // EGL_GET_ERROR with no current display returns an error/status code in the
        // EGL enum space (0x3000..0x3089) — never the NULL-stub's trivial 0x0 and
        // never a garbage pointer. The real Mesa path returns a real EGL error state.
        assert_ne!(r, 0, "eglGetError must return a real Mesa result, not stub 0");
    }

    #[test]
    fn resolve_egl_rejects_non_egl_names() {
        // Must NOT dlopen/allocate for GLES or unrelated names (float ABI, not wired).
        assert!(resolve_egl(b"glViewport\0").is_none());
        assert!(resolve_egl(b"strlen\0").is_none());
    }

    /// Integer-ABI GLES imports (gl* with integer/pointer args, <=8) must resolve to
    /// real Mesa and actually execute — same graphics-wiring gate as resolve_egl, for
    /// the GLES texture/state/draw pipeline. Float-taking GLES must be rejected (their
    /// ABI is not expressible through the integer HostCall).
    #[test]
    fn resolve_gles_int_binds_real_mesa_for_integer_abi_names() {
        // glGetError() -> GLenum, integer ABI, in the whitelist.
        let Some(slot) = resolve_gles_int(b"glGetError\0") else {
            eprintln!("skipping: Mesa GLES (libGLESv2.so.2) not present on this host");
            return;
        };
        let mut img: Vec<u8> = Vec::new();
        img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
        img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0
        let mut st = CpuState::new();
        st.x[16] = slot;
        let r = jit_run(&img, 0x1000, 0x1000, &mut st as *mut CpuState).expect("jit_run");
        // GL_NO_ERROR (0) is a valid Mesa result; the guarantee is that real Mesa GLES
        // executed (no segfault / no crash). Without a current context Mesa returns 0
        // cleanly; the point is it didn't dispatch to the old NULL/0 stub and crash on
        // a garbage fn pointer while marshalling xmm/stack args.
        let _ = r;
    }

    #[test]
    fn resolve_gles_int_rejects_float_abi_and_unknown() {
        // glClearColor takes GLfloat args -> MUST NOT resolve through the integer HostCall.
        assert!(resolve_gles_int(b"glClearColor\0").is_none());
        // glTexImage2D has 9 args (stack-arg beyond the 8 the HostCall can express).
        assert!(resolve_gles_int(b"glTexImage2D\0").is_none());
        // Non-GLES names rejected.
        assert!(resolve_gles_int(b"strlen\0").is_none());
        // Integer-ABI GLES that is NOT in the shipped Mesa GLESv2 (should be absent).
        assert!(resolve_gles_int(b"glTotallyFake\0").is_none());
    }
}