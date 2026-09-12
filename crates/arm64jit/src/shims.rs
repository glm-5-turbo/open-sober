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
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

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

// ---- Android AssetManager/AAsset shims (QEMU-consistent: NULL/0) ----
extern "C" fn aassetmanager_fromjava(
    _env: u64, _instance: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0 // static manager handle (NULL: no assets mounted)
}
extern "C" fn aassetmanager_open(
    _mgr: u64, _filename: u64, _mode: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0 // AAsset* NULL => open fails
}
extern "C" fn aasset_close(
    _asset: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
extern "C" fn aasset_getbuffer(
    _asset: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
extern "C" fn aasset_getlength(
    _asset: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

// ---- AConfiguration: report a tablet-ish screen so Roblox picks a UI size ----
extern "C" fn aconfig_get_screenwidthdp(
    _cfg: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    1080
}
extern "C" fn aconfig_get_screenheightdp(
    _cfg: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    1776
}
extern "C" fn aconfig_get_screensize(
    _cfg: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0xA // ACONFIGURATION_SCREENSIZE_ANY-compatible large
}
extern "C" fn aconfig_get_navhidden(
    _cfg: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    2 // ACONFIGURATION_KEYSHIDDEN_YES = 2
}

// ---- ALooper + Android app-command dispatch ----
// GameActivity's post-barrier main loop drives lifecycle by polling the native
// app-command queue (`ALooper_pollOnce`). The old shim returned 0 immediately
// (ALOOPER_POLL_TIMEOUT) with no event channel, so even a boot that crossed the
// recursive-mutex rendezvous would busy-spin the loop rather than dispatch
// APP_CMD_START/RESUME/INIT_WINDOW. This implements a real, host-feedable
// app-command FIFO: `post_app_command` (host side, e.g. elfjit under
// JIT_DRIVE_LIFECYCLE) pushes lifecycle events, and ALooper_pollOnce drains them
// into the guest's outFd/outEvents/outData slots, returning the ident like the
// real android_native_app_glue. APP_CMD_* values are the standard Android ones
// (native_activity.h): START=1, RESUME=2, PAUSE=3, STOP=4, WINDOW_RESIZED=5,
// CONFIGURATION_CHANGED=6, WINDOW_REDRAW_NEEDED=7, GAINED_FOCUS=8,
// LOST_FOCUS=9, INIT_WINDOW=11.
pub const APP_CMD_START: i32 = 1;
pub const APP_CMD_RESUME: i32 = 2;
pub const APP_CMD_INIT_WINDOW: i32 = 11;
pub const ALOOPER_POLL_CALLBACK: i32 = -2; // a callback was invoked
pub const ALOOPER_POLL_TIMEOUT: i32 = -3; // nothing ready before timeout

/// Process-wide FIFO of pending `APP_CMD_*` values (host -> guest).
fn app_cmd_queue() -> &'static Mutex<std::collections::VecDeque<i32>> {
    static Q: OnceLock<Mutex<std::collections::VecDeque<i32>>> = OnceLock::new();
    Q.get_or_init(|| Mutex::new(std::collections::VecDeque::new()))
}

/// Host side: queue an Android app command for the next `ALooper_pollOnce`.
pub fn post_app_command(cmd: i32) {
    app_cmd_queue().lock().unwrap().push_back(cmd);
}

/// ALooper_pollOnce(timeoutMillis, outFd*, outEvents*, outData*).
///
/// Drains one pending app command into the guest's out slots and returns the
/// command's synthetic looper ident (a small non-negative fd-like value), so the
/// guest's `while ((ident = ALooper_pollOnce(...)) >= 0)` loop dispatches it.
/// When the queue is empty it returns ALOOPER_POLL_TIMEOUT immediately (never
/// blocks/hangs the guest on an fd we never signal). Under JIT_DRIVE_LIFECYCLE a
/// host feed drives the queue concurrently.
extern "C" fn alooper_pollonce(
    _timeout: u64, outfd: u64, outevents: u64, outdata: u64, _a4: u64, _a5: u64, _a6: u64,
    _a7: u64,
) -> u64 {
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[alooper] ALooper_pollOnce (queue={})", app_cmd_queue().lock().unwrap().len());
    }
    let Some(cmd) = app_cmd_queue().lock().unwrap().pop_front() else {
        // Nothing to dispatch: report a benign timeout so the game loop checks
        // destroyRequested/lifecycle and re-polls rather than erroring out.
        return ALOOPER_POLL_TIMEOUT as u32 as u64;
    };
    // Android app-glue convention: outFd = the command pipe's read end (a small
    // fd-like token), outEvents = ALOOPER_EVENT_INPUT (1) meaning readable, and
    // outData = a pointer-sized app-command token the guest decodes. We encode
    // the APP_CMD_* value in outData so a guest that reads it (android_app_read_cmd)
    // sees the lifecycle event even though we don't host a real pipe.
    if outfd != 0 {
        unsafe { std::ptr::write(outfd as *mut i32, 0x23 /* synthetic readable fd */) };
    }
    if outevents != 0 {
        unsafe { std::ptr::write(outevents as *mut i32, 1 /* ALOOPER_EVENT_INPUT */) };
    }
    if outdata != 0 {
        unsafe { std::ptr::write(outdata as *mut u64, cmd as u64) };
    }
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[alooper] ALooper_pollOnce -> app_cmd={cmd} (APP_CMD dispatch)");
    }
    cmd as u64
}

// ---- ANativeWindow ----
extern "C" fn anativewindow_fromsurface(
    _env: u64, _surf: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
extern "C" fn anativewindow_release(
    _win: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
// ANativeWindow_getWidth/getHeight: report a tablet-ish framebuffer so Roblox's
// ANativeWindow query on its (non-null stub) surface returns a sane size instead
// of 0 (which some engines treat as a headless/error path).
extern "C" fn anativewindow_getwidth(
    _win: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    1280
}
extern "C" fn anativewindow_getheight(
    _win: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    720
}
// ALooper_prepare / ALooper_forThread: return a stable non-null looper handle so
// GameActivity's `ALooper_forThread()` in its main loop gets a real object it can
// pass to ALooper_pollOnce (rather than NULL, which would take a crash path).
// acquire/release are no-ops; addFd/removeFd return 0 (nothing was registered)
// because our pollOnce doesn't watch real fds — lifecycle comes through the
// app-command queue instead.
extern "C" fn alooper_prepare(
    _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    crate::jit::HOST_THUNK_BASE | 0x2001
}
extern "C" fn alooper_forthread(
    _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    crate::jit::HOST_THUNK_BASE | 0x2001
}
// ALOOPER_POLL_CALLBACK convention: ALooper_addFd returns 1 on success (registered
// for callbacks). We register nothing but still indicate success; pollOnce uses
// the queue, so the fd is never actually polled.
extern "C" fn alooper_addfd(
    _looper: u64, _fd: u64, _iden: u64, _events: u64, _cb: u64, _data: u64,
    _a6: u64, _a7: u64,
) -> u64 {
    1
}
extern "C" fn alooper_removefd(
    _looper: u64, _fd: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
extern "C" fn alooper_acquire(
    _looper: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}
extern "C" fn alooper_release(
    _looper: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

// ---- Java_...IAPPurchaseManager.nativeFinishPayments...: benign JNI stub ----
extern "C" fn java_iap_purchase(
    _env: u64, _this: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

// ---- __cxa_guard_acquire/release/abort (Itanium C++ static-init guards) ----
// Roblox's FMOD/engine static init uses __cxa_guard_*; the minimal `--jni` env
// never provided these, so the guest dispatched its static-init into `pc=0x68c7
// 518` (a `.bss` guard) and stopped. Implement the single-threaded semantics the
// JIT needs (this is the same class as `patch_stack_canary` — guest C++ runtime
// glue we must supply). Guard variable is the standard byte-at-*g:
//   acquire: if *g==0  -> *g=1, return 1 (caller runs init); else return 0 (done).
//   release: *g=2 (init complete, no waiting needed single-threaded).
//   abort:   *g=0 (init crashed -> reset so it retries).
extern "C" fn cxa_guard_acquire(
    g: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    if g == 0 {
        return 0;
    }
    let byte = unsafe { &mut *(g as *mut u8) };
    if *byte == 0 {
        *byte = 1; // "initialization in progress"
        1 // caller must run the once-body
    } else {
        0 // already initialized (or in progress on another thread)
    }
}

extern "C" fn cxa_guard_release(
    g: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    if g != 0 {
        unsafe { *(g as *mut u8) = 2 } // complete
    }
    0
}

extern "C" fn cxa_guard_abort(
    g: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    if g != 0 {
        unsafe { *(g as *mut u8) = 0 } // reset
    }
    0 // void
}

// ---- __cxa_atexit: register a destructor call at exit. No-op (0 = success):
// we don't model at-exit ordering, and a dropped registered call is harmless
// for boot (the process teardown path is RTLD/loader-owned, not guest-owned).
extern "C" fn cxa_atexit(
    _fn: u64, _arg: u64, _dso: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

/// __cxa_thread_atexit_impl(func, arg, dso): register a thread-local
/// destructor. NO-OP (return 0 = success) WITHOUT storing anything, so glibc
/// never invokes the guest functor natively on thread exit.
///
/// Why: `__cxa_thread_atexit_impl` is an Android/glibc CRT hook that registers
/// a `__thread`/thread_local destructor. If we leave it resolved to the real
/// glibc function, glibc stores the *guest* AArch64 function pointer and, when
/// a guest thread (e.g. the worker spawned during JNI_OnLoad) exits, calls it
/// natively as x86 — jumping into guest `.text` (SIGSEGV executing ARM64 as
/// x86, the post-boot worker/teardown crash). Swallowing the registration is
/// safe: TLS destructors are insignificant to headless boot, and skipping them
/// keeps every call into guest code routed through `jit_run`.
extern "C" fn cxa_thread_atexit_impl(
    _fn: u64, _arg: u64, _dso: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

/// pthread_once(once_control, init_routine).
///
/// A guest `bl pthread_once@plt` passes a *guest* `init_routine` address. The
/// real glibc pthread_once would `call` it natively — executing guest ARM64
/// bytes as x86 (SIGILL on the first `paciasp`). Interpose it host-side: run
/// the guest routine once (per once_control, via `jit_run`) and mark done.
/// Single-threaded-conservative once semantics are sufficient for boot (a
/// second call sees "done" and skips).
fn once_guard_done() -> &'static Mutex<HashSet<u64>> {
    static DONE: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    DONE.get_or_init(|| Mutex::new(HashSet::new()))
}
extern "C" fn bionic_pthread_once(
    once: u64, routine: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    if routine == 0 || once == 0 {
        return 0;
    }
    // Nested reentry on the SAME guard would re-run the body; detect and skip.
    if once_guard_done().lock().unwrap().contains(&once) {
        return 0;
    }
    // Mark "in progress" before running so a reentrant pthread_once on this
    // guard (rare, but possible via a guest callback) does not recurse forever.
    once_guard_done().lock().unwrap().insert(once);
    let tp = crate::jit::current_guest_tp();
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[shim] pthread_once(once={once:#x}) runs guest routine {routine:#x}");
    }
    // The standard pthread_once init_routine takes no arguments.
    if let Err(e) = crate::jit::run_guest_callback(routine, [0; 8], tp) {
        eprintln!("[shim] pthread_once routine {routine:#x} failed: {e}");
    }
    0
}

/// pthread_create(thread*, attr, start_routine, arg).
///
/// Real glibc pthread_create calls the guest start_routine natively (SIGILL).
/// Interpose: spawn a fresh host thread running the guest start routine through
/// `jit_run` (per-thread guest stack + TLS), and write its guest tid as the
/// pthread_t. Returns 0 (success).
extern "C" fn bionic_pthread_create(
    thread: u64, _attr: u64, start_routine: u64, arg: u64,
    _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    let tid = crate::jit::spawn_pthread(start_routine, arg);
    if tid < 0 {
        return tid as u64; // error (EINVAL)
    }
    if thread != 0 {
        unsafe { std::ptr::write_unaligned(thread as *mut u64, tid as u64) };
    }
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[shim] pthread_create(start_routine={start_routine:#x}, arg={arg:#x}) -> tid={tid}");
    }
    0
}

/// pthread_join(t, retval): return 0 immediately. The guest worker threads we
/// spawn via `spawn_pthread` run to completion independently; a join is
/// fire-and-forget for boot (the retval pointer is left untouched).
extern "C" fn bionic_pthread_join(
    _t: u64, _retval: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

/// pthread_key_create(key*, destructor): create a real glibc key but DROP the
/// destructor (pass NULL to glibc).
///
/// Why: the worker thread (spawned during JNI_OnLoad) calls
/// `pthread_key_create(&key, dtor)` during mempool/per-thread-TLS init. Left
/// to real glibc, it stores the *guest* AArch64 destructor and, when the
/// thread exits, glibc runs it natively as x86 — jumping into guest `.text`
/// (SIGILL on the first `paciasp`, backtrace frames in `__pthread_keys`, the
/// post-crossed-boot worker/teardown crash). The book's own
/// `pthread_getspecific/setspecific` on the returned key keep working against
/// real glibc's per-thread storage (we return the real key); we only skip the
/// destructor call, which is insignificant to headless boot (same rationale as
/// `__cxa_thread_atexit_impl`).
extern "C" fn pthread_key_create(
    key: u64, _dtor: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    unsafe { libc::pthread_key_create(key as *mut libc::pthread_key_t, None) as u64 }
}

/// Register all guest C++ runtime shims (__cxa_guard_*, __cxa_atexit).
pub fn register_cxx_shims() -> usize {
    let shims: &[(&[u8], HostCall)] = &[
        (b"__cxa_guard_acquire\0", cxa_guard_acquire),
        (b"__cxa_guard_release\0", cxa_guard_release),
        (b"__cxa_guard_abort\0", cxa_guard_abort),
        (b"__cxa_atexit\0", cxa_atexit),
        (b"__cxa_thread_atexit_impl\0", cxa_thread_atexit_impl),
        (b"pthread_once\0", bionic_pthread_once),
        (b"pthread_create\0", bionic_pthread_create),
        (b"pthread_join\0", bionic_pthread_join),
        (b"pthread_key_create\0", pthread_key_create),
    ];
    for (name, f) in shims {
        crate::resolver::register_named(name, *f);
    }
    shims.len()
}

/// Register all host-side bionic shims; returns the number registered.
pub fn register_shims() -> usize {
    let shims: &[(&[u8], HostCall)] = &[
        (b"__errno\0", bionic_errno),
        (b"__strlen_chk\0", bionic_strlen_chk),
        (b"__strncpy_chk2\0", bionic_strncpy_chk2),
        (b"__android_log_print\0", bionic_android_log),
        // Android asset manager
        (b"AAssetManager_fromJava\0", aassetmanager_fromjava),
        (b"AAssetManager_open\0", aassetmanager_open),
        (b"AAsset_close\0", aasset_close),
        (b"AAsset_getBuffer\0", aasset_getbuffer),
        (b"AAsset_getLength\0", aasset_getlength),
        // Android configuration
        (b"AConfiguration_getScreenWidthDp\0", aconfig_get_screenwidthdp),
        (b"AConfiguration_getScreenHeightDp\0", aconfig_get_screenheightdp),
        (b"AConfiguration_getScreenSize\0", aconfig_get_screensize),
        (b"AConfiguration_getNavHidden\0", aconfig_get_navhidden),
        // Android looper (app-command dispatch + lifecycle handle)
        (b"ALooper_pollOnce\0", alooper_pollonce),
        (b"ALooper_prepare\0", alooper_prepare),
        (b"ALooper_forThread\0", alooper_forthread),
        (b"ALooper_addFd\0", alooper_addfd),
        (b"ALooper_removeFd\0", alooper_removefd),
        (b"ALooper_acquire\0", alooper_acquire),
        (b"ALooper_release\0", alooper_release),
        // Android native window
        (b"ANativeWindow_fromSurface\0", anativewindow_fromsurface),
        (b"ANativeWindow_release\0", anativewindow_release),
        (b"ANativeWindow_getWidth\0", anativewindow_getwidth),
        (b"ANativeWindow_getHeight\0", anativewindow_getheight),
        // Roblox JNI purchase gateway
        (
            b"Java_com_roblox_client_purchase_IAPPurchaseManager_nativeFinishPaymentsProtocolPurchaseWithReturn\0",
            java_iap_purchase,
        ),
    ];
    for (name, f) in shims {
        crate::resolver::register_named(name, *f);
    }
    shims.len()
}

/// Generic opaque-handle stub: return a stable non-null sentinel so handle-returning
/// routines (eglGetDisplay/eglCreate*, AMediaCodec creators, AConfiguration_new,
/// ALooper_prepare, AAsset*_open) "succeed" instead of failing boot with NULL.
extern "C" fn stub_handle(
    _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    crate::jit::HOST_THUNK_BASE | 0x1000 // a stable, non-null sentinel address
}
/// Integer/void/enum/what-returns-int stub: return 0.
extern "C" fn stub_zero(
    _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    0
}

/// Heuristic: is a name likely to RETURN a handle (vs an int/void/enum)? Used to
/// pick `stub_handle` vs `stub_zero` for an otherwise-unresolvable import.
fn is_handle_name(n: &[u8]) -> bool {
    let s = String::from_utf8_lossy(n);
    s.starts_with("eglGetDisplay")
        || s.starts_with("eglCreate")
        || s.starts_with("eglGetCurrentContext")
        || s.starts_with("slCreate")
        || s.starts_with("slGetInterface")
        || s.contains("_open")
        || s.contains("_fromJava")
        || s.contains("_new")
        || s.contains("_prepare")
        || s.contains("_forThread")
        || s.contains("_getBuffer")
        || s.contains("_getOutput")
        || s.contains("_getInput")
        || s.contains("_getOutputFormat")
}

/// Register a benign fallback stub for an otherwise-unresolved import name.
/// Callers (resolveimports / boot glue) use this for graphics/audio/media/bionic
/// names that have no host symbol and no hand-written shim: it binds the GOT so
/// execution can progress (QEMU's jni_stubs.h does the same, returning NULL/0).
pub fn register_fallback(name: &[u8]) -> u64 {
    let hc: HostCall = if is_handle_name(name) { stub_handle } else { stub_zero };
    crate::resolver::register_named(name, hc)
}

/// Register a catch-all fallback for **every** name in a slice (the full set of
/// unresolved imports). Returns how many were bound.
#[allow(non_snake_case)]
pub fn register_graphics_stubs(names: &[&[u8]]) -> usize {
    for n in names {
        register_fallback(n);
    }
    names.len()
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

    /// Itanium __cxa_guard_* semantics: acquire->release marks a guard as
    /// initialized so a later acquire returns 0 (already done); abort resets it.
    #[test]
    fn cxa_guard_acquire_release_abort_semantics() {
        // A guard variable in writable memory.
        let mut g = 0u8;
        let gp = &mut g as *mut u8 as u64;
        // First acquire: not initialized -> return 1 (run init) and mark in-progress.
        assert_eq!(cxa_guard_acquire(gp, 0, 0, 0, 0, 0, 0, 0), 1);
        assert_eq!(g, 1, "in-progress marker");
        // release completes initialization.
        cxa_guard_release(gp, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(g, 2, "done marker");
        // Second acquire: already initialized -> 0.
        assert_eq!(cxa_guard_acquire(gp, 0, 0, 0, 0, 0, 0, 0), 0);
        assert_eq!(g, 2, "acquire does not disturb a completed guard");
        // abort resets so the next acquire runs init again.
        cxa_guard_abort(gp, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(g, 0);
        assert_eq!(cxa_guard_acquire(gp, 0, 0, 0, 0, 0, 0, 0), 1);
        // NULL guard is safely ignored (returns 0, no fault).
        assert_eq!(cxa_guard_acquire(0, 0, 0, 0, 0, 0, 0, 0), 0);
        assert_eq!(cxa_atexit(0, 0, 0, 0, 0, 0, 0, 0), 0);
    }

    /// The __cxa_guard_* shims must be resolvable by name through the boot
    /// path (register_cxx_shims -> register_named -> resolver::resolve), not
    /// just callable directly — otherwise an import naming one never binds and
    /// falls to the NULL/0 graphics catch-all.
    #[test]
    fn register_cxx_shims_are_resolvable_by_name() {
        let n = register_cxx_shims();
        assert!(n >= 4, "guard_acquire/release/abort + atexit registered");
        for name in [
            "__cxa_guard_acquire",
            "__cxa_guard_release",
            "__cxa_guard_abort",
            "__cxa_atexit",
            "__cxa_thread_atexit_impl",
            "pthread_key_create",
        ] {
            let addr = crate::resolver::resolve(name.as_bytes())
                .unwrap_or_else(|| panic!("{name:?} not resolvable by name"));
            assert!(
                addr >= crate::jit::HOST_THUNK_BASE,
                "{name:?} bound to a real host thunk"
            );
        }
    }

    /// The ALooper app-command dispatch: post_app_command feeds the guest a
    /// lifecycle event that ALooper_pollOnce drains into its outFd/outEvents/
    /// outData slots and returns as the looper ident — the mechanism that lets a
    /// GameActivity main loop dispatch APP_CMD_* instead of busy-spinning on a
    /// never-signalled fd. Regression: empty queue returns ALOOPER_POLL_TIMEOUT;
    /// a posted command is returned exactly once and written to outData.
    #[test]
    fn alooper_pollonce_dispatches_host_fed_app_commands() {
        // Empty queue -> timeout (-3), nothing written.
        let mut fd = 0i32;
        let mut ev = 0i32;
        let mut data = 0u64;
        let r = alooper_pollonce(
            0, &mut fd as *mut i32 as u64, &mut ev as *mut i32 as u64,
            &mut data as *mut u64 as u64, 0, 0, 0, 0,
        );
        assert_eq!(r, ALOOPER_POLL_TIMEOUT as u32 as u64);

        // Post START, then RESUME; each poll drains one exactly-once.
        post_app_command(APP_CMD_START);
        post_app_command(APP_CMD_RESUME);
        let r1 = alooper_pollonce(
            0, &mut fd as *mut i32 as u64, &mut ev as *mut i32 as u64,
            &mut data as *mut u64 as u64, 0, 0, 0, 0,
        );
        assert_eq!(r1, APP_CMD_START as u64, "ident == posted command");
        assert_eq!(data, APP_CMD_START as u64, "outData carries the app command");
        assert_eq!(ev, 1, "ALOOPER_EVENT_INPUT");
        assert_eq!(fd, 0x23, "synthetic readable fd");

        let r2 = alooper_pollonce(
            0, &mut fd as *mut i32 as u64, &mut ev as *mut i32 as u64,
            &mut data as *mut u64 as u64, 0, 0, 0, 0,
        );
        assert_eq!(r2, APP_CMD_RESUME as u64);
        assert_eq!(data, APP_CMD_RESUME as u64);

        // Queue now drained -> timeout again.
        let r3 = alooper_pollonce(
            0, &mut fd as *mut i32 as u64, &mut ev as *mut i32 as u64,
            &mut data as *mut u64 as u64, 0, 0, 0, 0,
        );
        assert_eq!(r3, ALOOPER_POLL_TIMEOUT as u32 as u64);
    }
}