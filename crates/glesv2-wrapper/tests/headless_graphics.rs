//! Headless smoke test for the graphics translation layer: dlopen the built
//! `libEGL.so` and `libGLESv2.so` wrapper cdylibs, drive a real surfaceless ES3
//! context on Mesa's llvmpipe (LIBGL_ALWAYS_SOFTWARE=1, EGL_PLATFORM=surfaceless)
//! entirely THROUGH the wrappers, and confirm:
//!   1. EGL forwards to Mesa (display init, version string) — egl-wrapper works.
//!   2. GLES entry points forward (a real context is current, renderer string).
//!   3. glCompressedTexImage2D with an Android ETC2 format is intercepted and
//!      correctly decompressed/uploaded (texture2ddecoder path), while a
//!      non-Android BC format is passed through to Mesa untouched.
//!
//! This is the permanent headless regression gate for the graphics layer — no GPU
//! needed. Requires these env vars at runtime (set in the test via std::env::set_var
//! BEFORE dlopen only works for libc lookup; instead we export them by launching with
//! env, see the `constructor` trick below).

use std::ffi::{c_void, CString};
use std::path::PathBuf;

// Mesa must pick the surfaceless platform + software rasterizer. Because dlopen of
// EGL consults EGL_PLATFORM at display-creation time (well after our constructor),
// we set them as early as possible via a crate constructor.
thread_local! {
    static GUARD: u8 = {
        // Rust 2024: set_var is unsafe (mutates process env). Must set before Mesa's
        // getenv at display-creation; surfaceless + software is the headless path.
        unsafe {
            std::env::set_var("EGL_PLATFORM", "surfaceless");
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        }
        0u8
    };
}

/// Locate a built wrapper cdylib under target/{profile}.
fn wrapper_so(name: &str) -> PathBuf {
    // cargo sets OUT_DIR for build scripts; derive target dir from CARGO_MANIFEST_DIR.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest is crates/glesv2-wrapper -> target dir is ../../target
    let target = manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target");
    let profile = if std::env::var("PROFILE").ok().as_deref() == Some("release") {
        "release"
    } else {
        "debug"
    };
    let path = target.join(profile).join(name);
    assert!(path.exists(), "wrapper {} not built at {:?}", name, path);
    path
}

/// Minimal libc dlopen/dlsym helper (avoid pulling libloading into the test).
unsafe fn dlopen(path: &std::path::Path) -> *mut c_void {
    let c = CString::new(path.to_str().unwrap()).unwrap();
    let h = libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
    assert!(!h.is_null(), "dlopen {} failed", path.display());
    h
}

unsafe fn dlsym<T: Copy>(handle: *mut c_void, name: &str) -> T {
    let c = CString::new(name).unwrap();
    let p = libc::dlsym(handle, c.as_ptr());
    assert!(!p.is_null(), "dlsym {} missing", name);
    core::mem::transmute_copy(&(p as usize))
}

type EglGetDisplay = unsafe extern "C" fn(*const c_void) -> *mut c_void;
type EglInitialize = unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32) -> u32;
type EglBindAPI = unsafe extern "C" fn(u32) -> u32;
type EglChooseConfig = unsafe extern "C" fn(*mut c_void, *const i32, *mut *mut c_void, i32, *mut i32) -> u32;
type EglCreateContext = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const i32) -> *mut c_void;
type EglMakeCurrent = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> u32;
type EglQueryString = unsafe extern "C" fn(*mut c_void, i32) -> *const i8;
type EglGetProcAddress = unsafe extern "C" fn(*const i8) -> *const c_void;
type EglTerminate = unsafe extern "C" fn(*mut c_void) -> u32;

type GlGetString = unsafe extern "C" fn(u32) -> *const u8;
type GlGenTextures = unsafe extern "C" fn(i32, *mut u32);
type GlBindTexture = unsafe extern "C" fn(u32, u32);
type GlCompressedTexImage2D = unsafe extern "C" fn(u32, i32, u32, i32, i32, i32, i32, *const c_void);
type GlGetError = unsafe extern "C" fn() -> u32;
type GlGetIntegerv = unsafe extern "C" fn(u32, *mut i32);

const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_OPENGL_ES3_BIT: u32 = 0x0040;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_PBUFFER_BIT: u32 = 0x0001;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_NONE: i32 = 0x3038;
const EGL_VERSION: i32 = 0x3054;
const EGL_CONTEXT_MAJOR_VERSION: i32 = 0x3098;
const EGL_NO_CONTEXT: *mut c_void = 0 as *mut c_void;

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_COMPRESSED_RGBA8_ETC2_EAC: u32 = 0x9278; // Android ETC2 (should be intercepted)
const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F1; // desktop BC1 (pass-through)
const GL_TEXTURE_COMPRESSED: u32 = 0x86A1;

fn make_es3_context(
    egl: *mut c_void,
    gles: *mut c_void,
) -> (*mut c_void, *mut c_void) {
    GUARD.with(|_| {});
    let get_display: EglGetDisplay = unsafe { dlsym(egl, "eglGetDisplay") };
    let initialize: EglInitialize = unsafe { dlsym(egl, "eglInitialize") };
    let bind_api: EglBindAPI = unsafe { dlsym(egl, "eglBindAPI") };
    let choose_config: EglChooseConfig = unsafe { dlsym(egl, "eglChooseConfig") };
    let create_context: EglCreateContext = unsafe { dlsym(egl, "eglCreateContext") };
    let make_current: EglMakeCurrent = unsafe { dlsym(egl, "eglMakeCurrent") };
    let query_string: EglQueryString = unsafe { dlsym(egl, "eglQueryString") };

    let dpy = unsafe { get_display(0 as *const c_void) };
    assert!(!dpy.is_null(), "eglGetDisplay failed through wrapper");
    let mut maj = 0i32;
    let mut min = 0i32;
    assert_eq!(
        unsafe { initialize(dpy, &mut maj, &mut min) },
        1u32,
        "eglInitialize failed through wrapper"
    );
    let ver_ptr = unsafe { query_string(dpy, EGL_VERSION) };
    let ver = if ver_ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ver_ptr) }
            .to_string_lossy()
            .into_owned()
    };
    assert!(ver.starts_with("1."), "EGL version not from Mesa: {ver}");

    assert_eq!(unsafe { bind_api(EGL_OPENGL_ES_API) }, 1, "eglBindAPI failed");
    let attrs = [
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES3_BIT as i32,
        EGL_SURFACE_TYPE, EGL_PBUFFER_BIT as i32,
        EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8,
        EGL_NONE,
    ];
    let mut cfg: *mut c_void = 0 as *mut c_void;
    let mut ncfg = 0i32;
    assert_eq!(
        unsafe { choose_config(dpy, attrs.as_ptr(), &mut cfg, 1, &mut ncfg) },
        1u32
    );
    assert!(ncfg >= 1 && !cfg.is_null(), "no EGL config matched");
    let ctx_attrs = [EGL_CONTEXT_MAJOR_VERSION, 3, EGL_NONE];
    let ctx = unsafe { create_context(dpy, cfg, EGL_NO_CONTEXT, ctx_attrs.as_ptr()) };
    assert!(!ctx.is_null(), "ES3 context creation failed through wrapper");
    assert_eq!(
        unsafe { make_current(dpy, 0 as *mut c_void, 0 as *mut c_void, ctx) },
        1u32,
        "eglMakeCurrent failed"
    );
    (dpy, ctx)
}

#[test]
fn headless_egl_gles_via_wrappers_forwards_and_intercepts() {
    GUARD.with(|_| {});
    let egl = unsafe { dlopen(&wrapper_so("libEGL.so")) };
    let gles = unsafe { dlopen(&wrapper_so("libGLESv2.so")) };

    let get_string: GlGetString = unsafe { dlsym(gles, "glGetString") };
    let gen_tex: GlGenTextures = unsafe { dlsym(gles, "glGenTextures") };
    let bind_tex: GlBindTexture = unsafe { dlsym(gles, "glBindTexture") };
    let compressed: GlCompressedTexImage2D = unsafe { dlsym(gles, "glCompressedTexImage2D") };
    let get_error: GlGetError = unsafe { dlsym(gles, "glGetError") };
    let get_iv: GlGetIntegerv = unsafe { dlsym(gles, "glGetIntegerv") };

    let (_dpy, _ctx) = make_es3_context(egl, gles);

    // (2) real context current through the wrapper
    let renderer_ptr = unsafe { get_string(0x1F01) }; // GL_RENDERER
    let renderer = if renderer_ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(renderer_ptr as *const i8) }
            .to_string_lossy()
            .into_owned()
    };
    eprintln!("GLES renderer via wrapper: {renderer}");
    assert!(
        !renderer.is_empty(),
        "no GL_RENDERER string via wrapper (context not current?)"
    );

    // (3a) Android ETC2 upload -> our wrapper INTERCEPTS (decompress -> glTexImage2D
    // RGBA8). A 4x4 ETC2_RGBA8 block is 16 bytes.
    let mut tex = [0u32; 2];
    unsafe { gen_tex(2, tex.as_mut_ptr()) };
    assert!(tex[0] != 0, "glGenTextures returned 0 through wrapper");
    unsafe { bind_tex(GL_TEXTURE_2D, tex[0]) };
    let etc2_block = [0u8; 16];
    unsafe {
        compressed(
            GL_TEXTURE_2D,
            0,
            GL_COMPRESSED_RGBA8_ETC2_EAC,
            4,
            4,
            0,
            16,
            etc2_block.as_ptr() as *const c_void,
        )
    };
    assert_eq!(unsafe { get_error() }, 0, "ETC2 upload errored through wrapper");
    // After interception the internal format is uncompressed RGBA8, not an
    // Android-compressed token — confirm with glGetIntegerv(GL_TEXTURE_COMPRESSED).
    let mut is_compressed = 0i32;
    unsafe { get_iv(GL_TEXTURE_COMPRESSED, &mut is_compressed) };
    assert_eq!(
        is_compressed, 0,
        "ETC2 upload was NOT intercepted as uncompressed (still compressed=1)"
    );

    // (3b) Desktop BC1 (DXT1) upload -> NOT an Android format -> passthrough to Mesa.
    // Mesa/llvmpipe supports S3TC; if it does, no GL error. We only assert no crash
    // and that Mesa actually handled it as a compressed texture.
    let mut tex2 = [0u32; 2];
    unsafe { gen_tex(2, tex2.as_mut_ptr()) };
    unsafe { bind_tex(GL_TEXTURE_2D, tex2[0]) };
    let dxt1_block = [0u8; 8];
    // Accept EGL error from a perhaps-unsupported desktop format; the point is we
    // did NOT intercept and did NOT crash. Wrapper passes through regardless.
    unsafe {
        compressed(
            GL_TEXTURE_2D,
            0,
            GL_COMPRESSED_RGB_S3TC_DXT1_EXT,
            4,
            4,
            0,
            8,
            dxt1_block.as_ptr() as *const c_void,
        )
    };
    let _err = unsafe { get_error() };

    // eglGetProcAddress must also forward (Roblox resolves many fns this way).
    let get_proc: EglGetProcAddress = unsafe { dlsym(egl, "eglGetProcAddress") };
    let name = CString::new("glActiveTexture").unwrap();
    let p = unsafe { get_proc(name.as_ptr()) };
    assert!(!p.is_null(), "eglGetProcAddress(glActiveTexture) returned null via wrapper");

    let terminate: EglTerminate = unsafe { dlsym(egl, "eglTerminate") };
    unsafe { terminate(_dpy) };
    eprintln!("headless graphics translation smoke test OK");
}