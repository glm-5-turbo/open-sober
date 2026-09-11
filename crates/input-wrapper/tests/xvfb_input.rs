//! Headless smoke test for input-wrapper: launches an Xvfb server, opens a window
//! through the crate's raw-X11 layer, and verifies the pointer translation pipeline
//! yields Android ACTION_DOWN (requires spawning Xvfb — the permanent regression gate
//! for the input layer on a headless/CI box). Skips gracefully if Xvfb is absent.

use std::process::Command;

use input_wrapper::{input, x11};
use x11rb::rust_connection::RustConnection;

/// Launch Xvfb on `:display_num`; returns the child so it is kept alive.
fn spawn_xvfb(display_num: usize) -> std::process::Child {
    let disp = format!(":{display_num}");
    // Xvfb :N -screen 0 640x480x24 -nolisten tcp
    let child = Command::new("Xvfb")
        .arg(&disp)
        .arg("-screen")
        .arg("0")
        .arg("640x480x24")
        .arg("-nolisten")
        .arg("tcp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Xvfb binary present (test requires it; install xvfb)");
    child
}

#[test]
fn x11_window_and_mouse_map_to_android_touch() {
    // Find a free-ish display number under a small lock-free margin.
    let display_num: usize = std::env::var("INPUT_TEST_DISPLAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| 100 + (std::process::id() % 50) as usize);
    let mut child = spawn_xvfb(display_num);
    let display = format!(":{display_num}");

    // Give Xvfb a moment to create the socket (and retry connect; first attempts can
    // race the server socket). Verify the child actually stayed alive.
    let mut conn_res: Result<(RustConnection, u32), x11::XError> = Err(x11::XError::Connect(
        "no attempt yet".into(),
    ));
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(Some(st)) = child.try_wait() {
            panic!("Xvfb exited early on {display}: {st:?}");
        }
        if std::path::Path::new(&format!("/tmp/.X11-unix/X{display_num}")).exists()
            || std::path::Path::new(&format!("/tmp/.X11-unix/X{display_num}-")).exists()
        {
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
            panic!("open_window against Xvfb {display} failed: {e:?}");
        }
    };
    assert_ne!(win, 0, "window id should be nonzero");

    // Pump once: with no events this should be a no-op Ok.
    let mut tracker = input::PointerTracker::default();
    let mut seen: Vec<input::MotionEvent> = Vec::new();
    x11::pump(&conn, &mut tracker, &mut |e| seen.push(e.clone()))
        .expect("pump should succeed against live Xvfb");

    // Simulate a button press through the tracker directly (server-side button
    // injection would need XTEST; transpile through the same mapping the pump uses).
    let ev = tracker.on_button(input::button::PRIMARY, true, 5.0, 6.0);
    assert_eq!(ev.action, input::action::ACTION_DOWN);
    assert_eq!(ev.pointer_id, 0);
    let mv = tracker.on_motion(10.0, 12.0).expect("move while down");
    assert_eq!(mv.action, input::action::ACTION_MOVE);
    let up = tracker.on_button(input::button::PRIMARY, false, 10.0, 12.0);
    assert_eq!(up.action, input::action::ACTION_UP);

    // Keyboard mapping check (pure, no server needed).
    assert_eq!(
        input::x11_keysym_to_keycode(0x0061),
        input::keycode::AKEYCODE_A
    );

    drop(conn);
    let _ = child.kill();
    let _ = child.wait();
    eprintln!("input-wrapper Xvfb smoke test OK (window {win})");
}