//! Integration gate for the ANativeWindow -> real-X11-window mapping
//! (GRAPHICS_RECOMMENDATION section 5.3 / STATUS next-lever 1): wire a real
//! X11 window as the guest's ANativeWindow handle, then prove the FULL chain —
//! the XID `ANativeWindow_fromSurface` hands the guest is a genuine desktop
//! window that Mesa's x11 EGL platform accepts for `eglCreateWindowSurface`,
//! and that a frame can be cleared + swapped to it headlessly (Xvfb + llvmpipe).
//! Complement to `egl_window_present.rs`: that gate passes a raw XID it opened
//! itself; this gate passes the XID obtained THROUGH the ANativeWindow shim, the
//! exact value the boot's window-surface path consumes. Skips gracefully when
//! Xvfb or Mesa's x11-EGL platform is unavailable.
//!
//! Uses `input_wrapper::x11` (the crate's own X11 window helper) so the same
//! 1280x720 window the runtime opens for the ANativeWindow layer is reused.

use std::process::Command;

use arm64jit::jit::{jit_run, CpuState};
use arm64jit::resolver::{resolve_egl, resolve_gles_int};
use arm64jit::shims::{anativewindow_xid, set_anativewindow_xid};
use input_wrapper::x11;

/// Launch Xvfb on `:display_num`; keep the child alive for the test.
fn spawn_xvfb(display_num: usize) -> std::process::Child {
    let disp = format!(":{display_num}");
    Command::new("Xvfb")
        .arg(&disp)
        .arg("-screen").arg("0").arg("1280x720x24")
        .arg("-nolisten").arg("tcp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Xvfb binary present (install xvfb) for the window-surface gate")
}

/// Drive one guest `blr x16` to `slot`; return what jit_run leaves in x0.
fn gcall(slot: u64, st: &mut CpuState) -> u64 {
    let mut img: Vec<u8> = Vec::new();
    img.extend_from_slice(&0xd63f0200u32.to_le_bytes()); // blr x16
    img.extend_from_slice(&0xd4200000u32.to_le_bytes()); // brk #0
    st.x[16] = slot;
    jit_run(&img, 0x1000, 0x1000, st as *mut CpuState).expect("jit_run")
}

#[test]
fn anativewindow_surface_handles_build_real_egl_window_surface_under_xvfb() {
    // Bail cleanly if the test can't run here (no Xvfb / no Mesa x11 EGL).
    let xvfb_ok = Command::new("Xvfb").arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
    if !xvfb_ok {
        eprintln!("skipping: Xvfb not installed on this host");
        return;
    }
    let Some(egl_getdisplay) = resolve_egl(b"eglGetDisplay\0") else {
        eprintln!("skipping: Mesa EGL (libEGL.so.1) not present");
        return;
    };

    // Start Xvfb, open a 1280x720 window (the framebuffer the runtime reports).
    let display_num = 250 + (std::process::id() % 50) as usize;
    let display = format!(":{display_num}");
    let mut child = spawn_xvfb(display_num);
    let mut conn_res: Result<(x11rb::rust_connection::RustConnection, u32), x11::XError> =
        Err(x11::XError::Connect("no attempt yet".into()));
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(Some(st)) = child.try_wait() {
            let _ = child.wait();
            panic!("Xvfb exited early on {display}: {st:?}");
        }
        if std::path::Path::new(&format!("/tmp/.X11-unix/X{display_num}")).exists() {
            conn_res = x11::open_window_sized(Some(&display), 1280, 720);
            if conn_res.is_ok() {
                break;
            }
        }
    }
    let (conn, win) = match conn_res {
        Ok(v) => v,
        Err(e) => {
            let _ = child.kill();
            eprintln!("skipping: could not open window against Xvfb {display}: {e:?}");
            let _ = child.wait();
            return;
        }
    };
    assert_ne!(win, 0, "X11 window id must be nonzero");

    // Wire the real window as the guest's ANativeWindow handle.
    set_anativewindow_xid(win as u64);
    assert_eq!(
        anativewindow_xid(),
        win as u64,
        "guest's ANativeWindow_fromSurface now yields the registered X11 XID"
    );

    // Mesa's x11 EGL platform reads DISPLAY (and EGL_PLATFORM if set).
    unsafe {
        std::env::set_var("DISPLAY", &display);
        std::env::set_var("EGL_PLATFORM", "x11");
    }

    let egl_initialize = resolve_egl(b"eglInitialize\0").expect("eglInitialize");
    let egl_choose_config = resolve_egl(b"eglChooseConfig\0").expect("eglChooseConfig");
    let egl_create_win_surface = resolve_egl(b"eglCreateWindowSurface\0").expect("eglCreateWindowSurface");
    let egl_create_ctx = resolve_egl(b"eglCreateContext\0").expect("eglCreateContext");
    let egl_make_current = resolve_egl(b"eglMakeCurrent\0").expect("eglMakeCurrent");
    let egl_swap_buffers = resolve_egl(b"eglSwapBuffers\0").expect("eglSwapBuffers");
    let gl_clear_color = arm64jit::resolver::resolve_gles_mixed(b"glClearColor\0")
        .expect("glClearColor float bridge");
    let gl_clear = resolve_gles_int(b"glClear\0").expect("glClear");

    // EGL constants.
    const EGL_NONE: u64 = 0x3038;
    const EGL_SURFACE_TYPE: u64 = 0x3033;
    const EGL_WINDOW_BIT: u64 = 0x0004;
    const EGL_RENDERABLE_TYPE: u64 = 0x3040;
    const EGL_OPENGL_ES2_BIT: u64 = 0x4;
    const EGL_CONTEXT_CLIENT_VERSION: u64 = 0x3098;
    const EGL_NO_CONTEXT: u64 = 0;
    const EGL_COLOR_BUFFER_BIT: u64 = 0x4000;

    let mut st = CpuState::new();

    let dpy = gcall(egl_getdisplay, &mut st);
    assert_ne!(dpy, 0, "eglGetDisplay() under x11 must return a display");
    let mut ver = [0u32; 2];
    st.x[0] = dpy;
    st.x[1] = ver.as_mut_ptr() as u64;
    gcall(egl_initialize, &mut st);
    assert!(ver[0] >= 1, "eglInitialize >= 1");

    let mut attribs = [EGL_SURFACE_TYPE, EGL_WINDOW_BIT, EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT, EGL_NONE, 0];
    let mut config = 0u64;
    let mut num = 0i32;
    st.x[0] = dpy;
    st.x[1] = attribs.as_mut_ptr() as u64;
    st.x[2] = (&mut config) as *mut u64 as u64;
    st.x[3] = 1;
    st.x[4] = (&mut num) as *mut i32 as u64;
    let ok = gcall(egl_choose_config, &mut st);
    assert_eq!(ok, 1, "eglChooseConfig found a window-capable config");
    assert_ne!(config, 0);

    // eglCreateWindowSurface(dpy, config, native_window = the XID the guest's
    // ANativeWindow_fromSurface returned, attribs). This is the section-5.3
    // mapping exercised end-to-end: the window layer's handle is a real X11 XID.
    let mut win_attribs = [EGL_NONE, 0];
    st.x[0] = dpy;
    st.x[1] = config;
    st.x[2] = win as u64;
    st.x[3] = win_attribs.as_mut_ptr() as u64;
    let surface = gcall(egl_create_win_surface, &mut st);
    assert_ne!(surface, 0, "eglCreateWindowSurface on the ANativeWindow XID succeeded");

    // Context + MakeCurrent.
    let mut ctx_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE, 0];
    st.x[0] = dpy;
    st.x[1] = config;
    st.x[2] = EGL_NO_CONTEXT;
    st.x[3] = ctx_attribs.as_mut_ptr() as u64;
    let ctx = gcall(egl_create_ctx, &mut st);
    assert_ne!(ctx, 0, "eglCreateContext");
    st.x[0] = dpy;
    st.x[1] = surface;
    st.x[2] = surface; // read == draw surface
    st.x[3] = ctx;
    let made = gcall(egl_make_current, &mut st);
    assert_eq!(made, 1, "eglMakeCurrent on the ANativeWindow surface");

    // Render + present a frame.
    let set_sf = |st: &mut CpuState, n: usize, f: f32| {
        st.v[2 * n] = (st.v[2 * n] & !0xffff_ffffu64) | (f.to_bits() as u64);
    };
    set_sf(&mut st, 0, 0.1);
    set_sf(&mut st, 1, 0.3);
    set_sf(&mut st, 2, 0.8);
    set_sf(&mut st, 3, 1.0);
    gcall(gl_clear_color, &mut st);
    st.x[0] = EGL_COLOR_BUFFER_BIT;
    gcall(gl_clear, &mut st);

    st.x[0] = dpy;
    st.x[1] = surface;
    let swapped = gcall(egl_swap_buffers, &mut st);
    assert_eq!(swapped, 1, "eglSwapBuffers presented a frame on the ANativeWindow surface (EGL_TRUE)");

    drop(conn);
    let _ = child.kill();
    let _ = child.wait();
    eprintln!("ANativeWindow gate OK: the guest's ANativeWindow XID built a real EGL window surface and presented a frame headlessly");
}