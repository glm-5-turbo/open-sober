//! Headless window-presentation gate for the graphics translation layer
//! (GRAPHICS_RECOMMENDATION section 5): under an Xvfb X server, open a real X11
//! window, then drive a REAL EGL window-surface + swap through the JIT resolver
//! bridges entirely via guest `blr` — eglGetDisplay/Initialize/ChooseConfig ->
//! eglCreateWindowSurface -> CreateContext -> MakeCurrent, then glClearColor +
//! glClear ... eglSwapBuffers. Proves a translated guest can create a windowed
//! GLES surface and present frames to a desktop native window without a GPU
//! (Mesa llvmpipe software + the x11 EGL platform). Skips gracefully if Xvfb is
//! absent; the window surface path is where a guest's ANativeWindow is mapped to
//! an X11 Window XID before eglCreateWindowSurface.

use std::process::Command;

use arm64jit::jit::{jit_run, CpuState};
use arm64jit::resolver::{resolve_egl, resolve_gles_int};
use input_wrapper::x11;
use x11rb::rust_connection::RustConnection;

/// Launch Xvfb on `:display_num`; keep the child alive for the test.
fn spawn_xvfb(display_num: usize) -> std::process::Child {
    let disp = format!(":{display_num}");
    Command::new("Xvfb")
        .arg(&disp)
        .arg("-screen").arg("0").arg("640x480x24")
        .arg("-nolisten").arg("tcp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Xvfb binary present (install xvfb) for the window-present gate")
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
fn egl_window_surface_and_swap_reach_real_mesa_under_xvfb() {
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

    // Start Xvfb on a process-unique display, open a window through input-wrapper.
    let display_num = 200 + (std::process::id() % 50) as usize;
    let display = format!(":{display_num}");
    let mut child = spawn_xvfb(display_num);
    let mut conn_res: Result<(RustConnection, u32), x11::XError> =
        Err(x11::XError::Connect("no attempt yet".into()));
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(Some(st)) = child.try_wait() {
            let _ = child.wait();
            panic!("Xvfb exited early on {display}: {st:?}");
        }
        if std::path::Path::new(&format!("/tmp/.X11-unix/X{display_num}")).exists() {
            conn_res = x11::open_window(Some(&display));
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

    // Mesa's x11 EGL platform reads DISPLAY (and EGL_PLATFORM if set); set both
    // before the resolve_egl dlopen path first touches the display, and before
    // eglGetDisplay. This process is this binary alone (single test), so the
    // process-global env mutation is isolated and safe.
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
    const EGL_NO_SURFACE: u64 = 0;
    const EGL_COLOR_BUFFER_BIT: u64 = 0x4000;

    let mut st = CpuState::new();

    let dpy = gcall(egl_getdisplay, &mut st);
    assert_ne!(dpy, 0, "eglGetDisplay() under x11 must return a display");
    let mut ver = [0u32; 2];
    st.x[0] = dpy;
    st.x[1] = ver.as_mut_ptr() as u64;
    gcall(egl_initialize, &mut st);
    assert!(ver[0] >= 1, "eglInitialize >= 1");

    // Window-capable config (SURFACE_TYPE = WINDOW_BIT, ES2 renderable).
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

    // eglCreateWindowSurface(dpy, config, native_window = X11 Window XID, attribs).
    // This is the section-5 mapping: the guest's ANativeWindow value maps to the
    // desktop X11 Window XID we pass here.
    let mut win_attribs = [EGL_NONE, 0];
    st.x[0] = dpy;
    st.x[1] = config;
    st.x[2] = win as u64;
    st.x[3] = win_attribs.as_mut_ptr() as u64;
    let surface = gcall(egl_create_win_surface, &mut st);
    assert_ne!(surface, 0, "eglCreateWindowSurface on the X11 window succeeded");

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
    assert_eq!(made, 1, "eglMakeCurrent on the window surface");

    // Render a frame: clear to a known color, then present with eglSwapBuffers.
    let set_sf = |st: &mut CpuState, n: usize, f: f32| {
        st.v[2 * n] = (st.v[2 * n] & !0xffff_ffffu64) | (f.to_bits() as u64);
    };
    set_sf(&mut st, 0, 0.2);
    set_sf(&mut st, 1, 0.4);
    set_sf(&mut st, 2, 0.6);
    set_sf(&mut st, 3, 1.0);
    gcall(gl_clear_color, &mut st);
    st.x[0] = EGL_COLOR_BUFFER_BIT;
    gcall(gl_clear, &mut st);

    st.x[0] = dpy;
    st.x[1] = surface;
    let swapped = gcall(egl_swap_buffers, &mut st);
    assert_eq!(swapped, 1, "eglSwapBuffers presented the cleared frame (EGL_TRUE)");

    drop(conn);
    let _ = child.kill();
    let _ = child.wait();
    eprintln!("window-present gate OK: created a real EGL window surface on Xvfb and presented a frame through the JIT bridges");
}