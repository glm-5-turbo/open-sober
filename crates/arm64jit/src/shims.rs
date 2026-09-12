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
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
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

/// The real Android `struct android_poll_source` layout, as the app-glue main
/// loop (guest 0x102bcd5d0) derefs it:
///   +0x00  int32 id
///   +0x08  struct android_app* app
///   +0x10  void (*process)(struct android_app*, struct android_poll_source*)
/// (disasm: `ldr x8,[x1,#16]; ldr x0,[x19,#24]; blr x8` — it loads
/// `source->process` at +16 and calls `process(app, source)`). The loop passes
/// `app` from its own android_app struct field +24, so the two call args are
/// (app_ptr, source_ptr). Referenced by the regression test that asserts the
/// emitted source's `process` sits at +0x10.
#[allow(dead_code)] // documented ABI constant; exercised by a #[cfg(test)]
const POLL_SOURCE_PROCESS_OFF: usize = 0x10;

/// Process-wide registry of `ALooper_addFd` poll-source registrations, keyed by
/// the fd the guest registered. The real glue call is
/// `ALooper_addFd(looper, msgread, LOOPER_ID_MAIN, ALOOPER_EVENT_INPUT,
/// callback, &app->cmd_source)` with `data` = the `android_poll_source*` whose
/// `process` the glue main loop `blr`s. We record `data` so `pollOnce` can hand
/// that exact source back through outData, matching the real contract (a guest
/// that derefs outData as `android_poll_source*` gets a real one, not a raw int).
fn poll_source_registry() -> &'static Mutex<HashMap<i32, u64>> {
    static R: OnceLock<Mutex<HashMap<i32, u64>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

/// ALooper_pollOnce(timeoutMillis, outFd*, outEvents*, outData*).
///
/// Real android_native_app_glue recall: after `ALooper_addFd` registered a
/// poll source (cmd_source with its `process` fn), the main loop calls
/// `ALooper_pollOnce(app->looper, -1, NULL, &events, (struct android_poll_source**)&outData)`
/// and, for a non-negative result, does `if (source != NULL)
/// (source->process)(app, source)` — i.e. **outData receives a pointer to an
/// `android_poll_source` whose `+0x10` is the guest's own process fn**, and the
/// guest calls through that function pointer itself. Writing the raw APP_CMD int
/// into outData is therefore only a *fallback*: a guest glue loop that follows
/// the real contract would deref the int as a poll_source (crash). So:
///   * when an `ALooper_addFd` poll-source registration exists for the queued
///     command's `cmd` ident, we return the *registered* `data` pointer (the
///     guest's own `android_poll_source`), so its `bl process` is real;
///   * otherwise (no addFd happened; host-driven lifecycle without a glue loop)
///     we keep the historic raw-int-in-outData fallback so the existing
///     GameActivity host-feed path still works.
/// Empty queue -> ALOOPER_POLL_TIMEOUT (never blocks on an fd we don't signal).
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
    // outFd/outEvents: the command pipe's read end (a small fd-like token) +
    // ALOOPER_EVENT_INPUT (readable), same as the historical host-feed shim.
    if outfd != 0 {
        unsafe { std::ptr::write(outfd as *mut i32, 0x23 /* synthetic readable fd */) };
    }
    if outevents != 0 {
        unsafe { std::ptr::write(outevents as *mut i32, 1 /* ALOOPER_EVENT_INPUT */) };
    }
    // outData: the real glue-contract poll_source, if the guest registered one
    // via ALooper_addFd under this fd; otherwise fall back to the raw APP_CMD.
    let registered_source = poll_source_registry()
        .lock()
        .unwrap()
        .get(&0x23_i32) // the command-pipe synthetic fd the glue registers
        .copied()
        .filter(|p| *p != 0);
    let emit = registered_source.unwrap_or(cmd as u64);
    if outdata != 0 {
        unsafe { std::ptr::write(outdata as *mut u64, emit) };
    }
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!(
            "[alooper] ALooper_pollOnce -> cmd={cmd} outData=0x{emit:x} ({} source)",
            if registered_source.is_some() { "registered poll" } else { "raw app_cmd" }
        );
    }
    cmd as u64
}

// ---- ANativeWindow ----
/// The real desktop X11 Window XID the runtime maps this Android `ANativeWindow`
/// to (GRAPHICS_RECOMMENDATION §5.3). Set by the host window layer (elfjit under
/// JIT_DRIVE_LIFECYCLE, or any caller wiring a real window) via
/// [`set_anativewindow_xid`]. Zero = no real window wired yet — in that case
/// [`anativewindow_fromsurface`] falls back to a stable non-null sentinel so the
/// window layer stays *coherent* (a non-null window with a sane size) even on a
/// headless/plain run.
static ANATIVE_WINDOW_XID: AtomicU64 = AtomicU64::new(0);

/// Register the real desktop X11 Window XID backing the guest's `ANativeWindow`.
/// Call once the host window is created (before the guest reaches the window/
/// EGL surface path); `anativewindow_fromsurface` then hands that XID to the
/// guest so `eglCreateWindowSurface(dpy, config, win, ...)` builds a surface on
/// a genuine window.
pub fn set_anativewindow_xid(xid: u64) {
    ANATIVE_WINDOW_XID.store(xid, AtomicOrdering::Relaxed);
}

/// The currently-registered real desktop X11 Window XID (0 = none wired).
pub fn anativewindow_xid() -> u64 {
    ANATIVE_WINDOW_XID.load(AtomicOrdering::Relaxed)
}
extern "C" fn anativewindow_fromsurface(
    _env: u64, _surf: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64, _a7: u64,
) -> u64 {
    let xid = ANATIVE_WINDOW_XID.load(AtomicOrdering::Relaxed);
    if std::env::var_os("JIT_TRACE").is_some() {
        eprintln!("[anativewindow] fromSurface(env=0x{_env:x} surf=0x{_surf:x}) -> xid=0x{xid:x}");
    }
    if xid != 0 {
        return xid; // the real X11 Window XID backing the window surface path
    }
    crate::jit::HOST_THUNK_BASE | 0x2000 // stable non-null sentinel ANativeWindow*
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
    _looper: u64, fd: u64, _iden: u64, _events: u64, _cb: u64, data: u64,
    _a6: u64, _a7: u64,
) -> u64 {
    // Record the guest's `data` argument — in real android_native_app_glue this
    // is `&app->cmd_source`, an `android_poll_source*` whose `process` fn the glue
    // loop `blr`s. Storing it lets `pollOnce` hand that exact source back through
    // outData (the real contract) instead of a raw APP_CMD int, so a guest glue
    // loop that follows the real `source->process(app, source)` dispatch works.
    if data != 0 {
        let key = if fd == 0 { 0x23_i32 } else { fd as i32 };
        poll_source_registry().lock().unwrap().insert(key, data);
        if std::env::var_os("JIT_TRACE").is_some() {
            eprintln!("[alooper] ALooper_addFd(fd={key} data=0x{data:x}) -> poll_source registered");
        }
    }
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

    /// Serialize the ALooper tests: they mutate the process-wide app-command
    /// queue + poll-source registry, and the test harness runs tests in
    /// parallel by default, so un-serialized they'd race each other's state.
    fn looper_test_lock() -> &'static Mutex<()> {
        static L: OnceLock<Mutex<()>> = OnceLock::new();
        L.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn bionic_errno_returns_valid_pointer() {
        assert!(crate::shims::bionic_errno(0, 0, 0, 0, 0, 0, 0, 0) != 0);
    }

    /// The native-window layer is coherent: ANativeWindow_fromSurface hands out a
    /// stable non-null handle that getWidth/getHeight answer (GRAPHICS_-
    /// RECOMMENDATION §5.3). A NULL window with a size was internally
    /// inconsistent and guaranteed eglCreateWindowSurface would never be reached.
    #[test]
    fn anativewindow_fromsurface_returns_stable_nonnull_handle_with_size() {
        // Default (no real window wired): stable non-null sentinel with a size.
        set_anativewindow_xid(0);
        let win = anativewindow_fromsurface(0, 0, 0, 0, 0, 0, 0, 0);
        assert_ne!(win, 0, "ANativeWindow_fromSurface must not return NULL");
        // Stable across calls (same sentinel each time) so a guest that holds the
        // handle and re-queries it sees a consistent address.
        assert_eq!(win, anativewindow_fromsurface(0, 0, 0, 0, 0, 0, 0, 0));
        let w = anativewindow_getwidth(win, 0, 0, 0, 0, 0, 0, 0);
        let h = anativewindow_getheight(win, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!((w, h), (1280, 720), "non-null window reports a sane framebuffer");
    }

    /// When a real desktop X11 window is wired (set_anativewindow_xid), the guest
    /// ANativeWindow_fromSurface returns exactly that XID — the window surface
    /// eglCreateWindowSurface builds on is a genuine window, not a sentinel
    /// (GRAPHICS_RECOMMENDATION §5.3). Resetting to 0 restores the sentinel
    /// fallback so a headless run stays coherent.
    #[test]
    fn anativewindow_fromsurface_returns_registered_real_x11_window() {
        set_anativewindow_xid(0x2c00000du64); // some real X11 Window XID
        let handle = anativewindow_fromsurface(0, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(handle, 0x2c00000du64, "guest receives the real X11 Window XID");
        // Unwired -> sentinel fallback is a different, still-non-null value.
        set_anativewindow_xid(0);
        let fallback = anativewindow_fromsurface(0, 0, 0, 0, 0, 0, 0, 0);
        assert_ne!(fallback, 0);
        assert_eq!(fallback, anativewindow_fromsurface(0, 0, 0, 0, 0, 0, 0, 0));
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
        let _g = looper_test_lock().lock().unwrap();
        poll_source_registry().lock().unwrap().clear();
        app_cmd_queue().lock().unwrap().clear();
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

    /// The app-glue main loop (guest 0x102bcd5d0) derefs ALooper_pollOnce's
    /// outData as `struct android_poll_source*` and `blr`s `source->process`
    /// (the fn at +0x10) with (app, source) as args — NOT as a raw APP_CMD int.
    /// So a guest that first calls `ALooper_addFd(..., &app->cmd_source)` must
    /// get its *registered* poll_source back through outData, and the shim must
    /// lay it out with `process` at +0x10 (the exact offset the loop's
    /// `ldr x8,[x1,#16]; blr x8` reads). Regression: the old shim wrote a raw
    /// int into outData, which a glue loop would deref as a poll_source and
    /// crash.
    #[test]
    fn alooper_pollonce_emits_registered_poll_source_not_raw_int() {
        let _g = looper_test_lock().lock().unwrap();
        // Reset registry state so prior tests' addFd entries don't leak in.
        poll_source_registry().lock().unwrap().clear();
        app_cmd_queue().lock().unwrap().clear();

        // Fabricate a guest android_poll_source at a readable host address with
        // a sentinel `process` fn pointer at +0x10 (POLL_SOURCE_PROCESS_OFF).
        let mut src = [0u64; 8]; // 64 bytes: id@0, app@8, process@16
        let process_fn = 0x102bcd6bc_u64; // any guest-ish fn ptr the loop would blr
        let src_ptr = src.as_mut_ptr() as u64;
        unsafe {
            std::ptr::write((src_ptr as *mut u64).add(POLL_SOURCE_PROCESS_OFF / 8), process_fn);
        }

        // Guest-side: ALooper_addFd(looper, fd=0, ident, events, cb, data=&src).
        let ret = alooper_addfd(0, 0, 1, 1, 0, src_ptr, 0, 0);
        assert_eq!(ret, 1, "addFd registers the command source");

        // Post a command and consume it via pollOnce; outData must carry the
        // REGISTERED poll_source pointer (0x10 = our process_fn), not the raw int.
        post_app_command(APP_CMD_START);
        let mut outfd = 0i32;
        let mut events = 0i32;
        let mut outdata = 0u64;
        let r = alooper_pollonce(
            0, &mut outfd as *mut i32 as u64, &mut events as *mut i32 as u64,
            &mut outdata as *mut u64 as u64, 0, 0, 0, 0,
        );
        assert_eq!(r, APP_CMD_START as u64, "ident still the posted command");
        assert_eq!(outdata, src_ptr, "outData is the registered android_poll_source*");
        // The emitted poll_source's +0x10 is the process fn the glue loop blr's.
        let emitted_process = unsafe {
            std::ptr::read((outdata as *const u64).add(POLL_SOURCE_PROCESS_OFF / 8))
        };
        assert_eq!(emitted_process, process_fn, "source->process sits at +0x10");
        poll_source_registry().lock().unwrap().clear();
    }

    /// Without an ALooper_addFd registration (pure host-driven lifecycle, no glue
    /// loop in play), pollOnce falls back to the historic raw-APP_CMD-in-outData
    /// so the GameActivity host-feed path stays intact.
    #[test]
    fn alooper_pollonce_falls_back_to_raw_cmd_without_registration() {
        let _g = looper_test_lock().lock().unwrap();
        poll_source_registry().lock().unwrap().clear();
        app_cmd_queue().lock().unwrap().clear();
        post_app_command(APP_CMD_RESUME);
        let mut outdata = 0u64;
        alooper_pollonce(0, 0, 0, &mut outdata as *mut u64 as u64, 0, 0, 0, 0);
        assert_eq!(outdata, APP_CMD_RESUME as u64, "no registration -> raw app_cmd fallback");
        poll_source_registry().lock().unwrap().clear();
    }
}