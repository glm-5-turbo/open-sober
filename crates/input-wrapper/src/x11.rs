//! X11 event source for input-wrapper (§6): connect to an X server (headless via
//! Xvfb or a real desktop), create a window, translate server mouse/keyboard events
//! into Android touch/key events via [`input::PointerTracker`], and hand them to a
//! caller callback (which the runtime routes to the guest AInputQueue).
//!
//! This module is backend-agnostic on purpose — the translation itself lives in
//! `input.rs` and is fully unit-tested. The `pump` function here only proves the
//! X11 wiring compiles and can pull real events headlessly (Xvfb smoke test).

use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, EventMask, WindowClass};
use x11rb::rust_connection::RustConnection;

use crate::input::{self, PointerTracker, MotionEvent};

/// Errors surfaced from the X11 layer.
#[derive(Debug)]
pub enum XError {
    Connect(String),
    Create(String),
}

/// Open a connection to the X server given by `display` (e.g. ":99" for Xvfb) and
/// create a mapped 640x480 window. Returns the connection + window id, or an error.
pub fn open_window(display: Option<&str>) -> Result<(RustConnection, u32), XError> {
    open_window_sized(display, 640, 480)
}

/// Like [`open_window`] but with an explicit framebuffer `width`/`height`. The
/// runtime's ANativeWindow layer (GRAPHICS_RECOMMENDATION §5.3) hands the guest
/// a desktop window as its ANativeWindow handle, so the window is sized to the
/// framebuffer `ANativeWindow_getWidth/Height` report (1280x720) — a coherent
/// window whose EGL window surface is buildable by Mesa's x11 platform.
pub fn open_window_sized(
    display: Option<&str>,
    width: u16,
    height: u16,
) -> Result<(RustConnection, u32), XError> {
    let (conn, screen_num) = x11rb::connect(display)
        .map_err(|e| XError::Connect(format!("x11rb connect: {e}")))?;
    let screen = &conn.setup().roots[screen_num];
    let win = conn
        .generate_id()
        .map_err(|e| XError::Create(format!("generate_id: {e}")))?;
    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        width,
        height,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new()
            .background_pixel(screen.white_pixel)
            .event_mask(
                EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::POINTER_MOTION
                    | EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE,
            ),
    )
    .map_err(|e| XError::Create(format!("create_window: {e}")))?
    .check()
    .map_err(|e| XError::Create(format!("create_window check: {e}")))?;
    conn.map_window(win)
        .map_err(|e| XError::Create(format!("map: {e}")))?
        .check()
        .map_err(|e| XError::Create(format!("map check: {e}")))?;
    Ok((conn, win))
}

/// Drain pending server events once, translating mouse/keyboard through the tracker
/// and invoking `dispatch` for each synthesized Android event. Non-blocking.
pub fn pump(
    conn: &RustConnection,
    tracker: &mut PointerTracker,
    dispatch: &mut dyn FnMut(MotionEvent),
) -> Result<(), XError> {
    use x11rb::protocol::Event;
    loop {
        let Ok(Some(ev)) = conn.poll_for_event() else {
            break;
        };
        match ev {
            Event::ButtonPress(b) | Event::ButtonRelease(b) => {
                let pressed = matches!(ev, Event::ButtonPress(_));
                // Only the primary button drives the touch stream; wheel/context are
                // non-touch and deliberately ignored here (guests read those via keys).
                if b.detail == input::button::PRIMARY {
                    let e = tracker.on_button(b.detail, pressed, b.event_x as f32, b.event_y as f32);
                    dispatch(e);
                }
            }
            Event::MotionNotify(m) => {
                if let Some(e) = tracker.on_motion(m.event_x as f32, m.event_y as f32) {
                    dispatch(e);
                }
            }
            // Keyboard is handled by the caller (needs keysym decode); we expose the
            // keycode mapping in input.rs. Here we merely ignore key events.
            _ => {}
        }
    }
    Ok(())
}

/// Block for a short time (used in the smoke test to let a real server run).
pub fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}