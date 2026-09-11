// dlopen+dlsym resolver shared by the EGL/GLES wrapper cdylibs.
// dlopen the real Mesa .so once (lazily), cache each resolved symbol address,
// and provide a typed fn-pointer getter so the generated forwarders stay one-liners.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug)]
pub struct ResolverError(pub String);

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ResolverError {}

/// Send+Sync wrapper for a raw library handle (dlopen'd pointers are not
/// Send/Sync by themselves; we own the handle and never deref across threads).
#[derive(Debug, Clone, Copy)]
struct Handle(*mut std::ffi::c_void);
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

static LIB: OnceLock<Handle> = OnceLock::new();
static SYMS: OnceLock<Mutex<HashMap<&'static str, usize>>> = OnceLock::new();

/// Resolve `symbol` in `libname`, dlopen-ing the library on first use.
/// Returns a raw address (usize) or an error if the symbol is missing.
pub fn addr(libname: &str, symbol: &'static str) -> Result<usize, ResolverError> {
    if let Some(&a) = SYMS.get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .get(symbol)
    {
        return Ok(a);
    }
    let handle = *LIB.get_or_init(|| {
        let cname = std::ffi::CString::new(libname).unwrap();
        let h = unsafe { libc::dlopen(cname.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        Handle(h)
    });
    let Handle(hptr) = handle;
    if hptr.is_null() {
        let err = cstr();
        return Err(ResolverError(format!(
            "dlopen {} failed: {}",
            libname, err
        )));
    }
    let cname = std::ffi::CString::new(symbol).unwrap();
    let p = unsafe { libc::dlsym(hptr, cname.as_ptr()) };
    if p.is_null() {
        return Err(ResolverError(format!(
            "dlsym {} not found in {}",
            symbol, libname
        )));
    }
    let a = p as usize;
    SYMS.get().unwrap().lock().unwrap().insert(symbol, a);
    Ok(a)
}

fn cstr() -> String {
    unsafe {
        let p = libc::dlerror();
        if p.is_null() {
            "unknown".into()
        } else {
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

/// Get a typed function pointer for `symbol`, aborting loudly if the Mesa ABI
/// table is out of date (a missing entry would otherwise dispatch through null).
pub fn sym<F: Copy>(libname: &str, symbol: &'static str) -> F {
    match addr(libname, symbol) {
        Ok(a) => unsafe { core::mem::transmute_copy(&a) },
        Err(e) => {
            eprintln!("egl/gles wrapper: {e}");
            unsafe { libc::abort() }
        }
    }
}