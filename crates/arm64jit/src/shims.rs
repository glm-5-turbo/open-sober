//! Host-side bionic/Android runtime shims for the JIT `--no-qemu` path.
//!
//! These cover the handful of `libroblox.so` imports that don't name a real
//! host libc/libm symbol (they are bionic/Android symbols), so `resolve()` (the
//! plain `dlsym` path) cannot materialize them. We register each as a host
//! call thunk so translated Roblox can `blr` to them in-process. The full
//! Android/JNI/bionic runtime (AAssetManager/ALooper/AConfiguration/JNI vm/...)
//! is the larger port; the helpers here are the small, self-contained, safe
//! subset worth materializing immediately.

use crate::jit::HostCall;

unsafe extern "C" {
    fn strlen(s: *const std::ffi::c_char) -> usize;
    fn strncpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn __errno_location() -> *mut std::ffi::c_int;
}

// ---- bionic `__errno` (returns `int*`, the *address* of the host errno) ----
extern "C" fn bionic_errno(
    _a0: u64,
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    unsafe { __errno_location() as u64 }
}

// ---- `__strlen_chk`: fortified strlen - just compute the real length ----
extern "C" fn bionic_strlen_chk(
    s: u64,
    _slen: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    if s == 0 {
        return 0;
    }
    unsafe { strlen(s as *const std::ffi::c_char) as u64 }
}

// ---- `__strncpy_chk2`: bounded strncpy - dst/src/len; ignore dest bound ----
extern "C" fn bionic_strncpy_chk2(
    dst: u64,
    src: u64,
    n: u64,
    _dstlen: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    if dst == 0 || src == 0 {
        return dst;
    }
    unsafe { strncpy(dst as *mut u8, src as *const u8, n as usize) as u64 }
}

// ---- `__android_log_print`: print `[tag] msg` to stderr, return 1 ----
// The guest `fmt` is the already-literally-formatted bionic log string for the
// most common Roblox call (`__android_log_print(p, tag, "<string>")`). We print
// the literal tag + message text.
extern "C" fn bionic_android_log(
    _prio: u64,
    tag: u64,
    fmt: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    let t = if tag != 0 {
        unsafe { std::ffi::CStr::from_ptr(tag as *const std::ffi::c_char) }
            .to_string_lossy()
            .into_owned()
    } else {
        String::new()
    };
    let m = if fmt != 0 {
        unsafe { std::ffi::CStr::from_ptr(fmt as *const std::ffi::c_char) }
            .to_string_lossy()
            .into_owned()
    } else {
        String::new()
    };
    eprintln!("[roblox:{t}] {m}");
    1
}

/// Register all host-side bionic shims; returns the number registered.
pub fn register_shims() -> usize {
    let shims: &[(&[u8], HostCall)] = &[
        (b"__errno\0", bionic_errno),
        (b"__strlen_chk\0", bionic_strlen_chk),
        (b"__strncpy_chk2\0", bionic_strncpy_chk2),
        (b"__android_log_print\0", bionic_android_log),
    ];
    for (name, f) in shims {
        crate::resolver::register_named(name, *f);
    }
    shims.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bionic_errno_returns_valid_pointer() {
        assert!(crate::shims::bionic_errno(0, 0, 0, 0, 0, 0, 0, 0) != 0);
    }

    #[test]
    fn strlen_chk_measures_length() {
        let c = std::ffi::CString::new("hello").unwrap();
        assert_eq!(bionic_strlen_chk(c.as_ptr() as u64, 10, 0, 0, 0, 0, 0, 0), 5);
    }
}