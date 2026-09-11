//! input-wrapper: Android input emulation for the Open Sober runtime
//! (GRAPHICS_RECOMMENDATION §6).
//!
//! Roblox-on-Android reads `AInputEvent` (motion/key). This crate models that input
//! surface (`input::MotionEvent`, `input::KeyEvent`, Android action/key codes) and
//! translates desktop pointer/keyboard input into it:
//!   * `input::PointerTracker` — mouse button/motion → ACTION_DOWN/MOVE/UP multi-touch.
//!   * `input::x11_keysym_to_keycode` — X11 keysym → Android key code (chat/DPAD/system).
//!   * `x11::open_window` / `x11::pump` — raw-X11 (pure-Rust x11rb) event source that
//!     pulls real server events and hands translated Android events to a dispatch
//!     callback (which the caller routes to the guest AInputQueue).
//!
//! The translation logic has no system dependency and is unit-tested; the X11 wiring
//! is smoke-tested headlessly against Xvfb.

pub mod input;
pub mod x11;