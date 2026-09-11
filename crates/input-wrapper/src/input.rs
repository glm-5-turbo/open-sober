//! Android input event model + desktop→Android translation (GRAPHICS_RECOMMENDATION §6).
//!
//! Roblox-on-Android consumes `AInputEvent` / `AMotionEvent` / `AKeyEvent` from the
//! Android framework. The translation understands that model exactly:
//!   * multi-touch `AMotionEvent` (ACTION_DOWN/UP/MOVE + pointer index/id + x/y),
//!   * Android key codes (AKEYCODE_*) for text/chat entry.
//! The desktop sources (mouse buttons, mouse motion, scroll, keyboard) are mapped into
//! those Android primitives. Y-flip is NOT applied here: Roblox expects the same
//! coordinate handedness as Android, and the host window provides pixels from the top,
//! so the host window's Y maps directly (games use the surface-relative origin that EGL
//! reports). We keep top-left origin and let callers flip if their backend differs.

/// Android MotionEvent action codes (frameworks/native/include/input/InputEventLabels / MotionEvent.java).
pub mod action {
    pub const ACTION_DOWN: i32 = 0;
    pub const ACTION_UP: i32 = 1;
    pub const ACTION_MOVE: i32 = 2;
    pub const ACTION_CANCEL: i32 = 3;
    pub const ACTION_OUTSIDE: i32 = 4;
    pub const ACTION_POINTER_DOWN: i32 = 5;
    pub const ACTION_POINTER_UP: i32 = 6;
    /// Mask/offset for packing the pointer index into the action field.
    pub const ACTION_POINTER_INDEX_MASK: i32 = 0xFF00;
    pub const ACTION_POINTER_INDEX_SHIFT: i32 = 8;
}

/// X11 button numbers -> Android mouse/touch semantics (Button1 = left = primary touch).
/// `u8` matches X11's event `detail` field.
pub mod button {
    pub const PRIMARY: u8 = 1;
    pub const SECONDARY: u8 = 3; // right -> secondary pointer (two-finger)
    pub const WHEEL_UP: u8 = 4;
    pub const WHEEL_DOWN: u8 = 5;
}

/// Android key codes (android.view.KeyEvent / linux keycode-mapping header).
#[allow(non_upper_case_globals)]
pub mod keycode {
    pub const AKEYCODE_UNKNOWN: i32 = 0;
    pub const AKEYCODE_ENTER: i32 = 66;
    pub const AKEYCODE_BACK: i32 = 4;
    pub const AKEYCODE_DEL: i32 = 67;
    pub const AKEYCODE_SPACE: i32 = 62;
    pub const AKEYCODE_TAB: i32 = 61;
    pub const AKEYCODE_ESCAPE: i32 = 111;
    pub const AKEYCODE_DPAD_UP: i32 = 19;
    pub const AKEYCODE_DPAD_DOWN: i32 = 20;
    pub const AKEYCODE_DPAD_LEFT: i32 = 21;
    pub const AKEYCODE_DPAD_RIGHT: i32 = 22;
    pub const AKEYCODE_A: i32 = 29;
    pub const AKEYCODE_Z: i32 = 54;
    pub const AKEYCODE_0: i32 = 7;
    pub const AKEYCODE_9: i32 = 16;
}

/// A synthetic multi-touch motion event destined for the guest AInputQueue.
#[derive(Debug, Clone)]
pub struct MotionEvent {
    pub action: i32,
    pub pointer_index: u32,
    pub pointer_id: u32,
    pub x: f32,
    pub y: f32,
}

/// A synthetic key event destined for the guest AInputQueue.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub action: bool, // true = ACTION_DOWN, false = ACTION_UP
    pub keycode: i32,
    pub meta_state: u32,
}

/// Tracks the single active primary pointer so repeat motion deltas only emit
/// ACTION_MOVE while a button is held (Roblox expects down->move*->up).
#[derive(Debug, Default)]
pub struct PointerTracker {
    pub active: bool,
    pub x: f32,
    pub y: f32,
}

impl PointerTracker {
    /// Feed a mouse button press/release; yields an Android touch event.
    pub fn on_button(&mut self, _button: u8, pressed: bool, x: f32, y: f32) -> MotionEvent {
        const POINTER_INDEX: u32 = 0;
        let action = if pressed {
            action::ACTION_DOWN
        } else {
            action::ACTION_UP
        };
        self.active = pressed;
        self.x = x;
        self.y = y;
        MotionEvent {
            action,
            pointer_index: POINTER_INDEX,
            pointer_id: POINTER_INDEX, // primary pointer id 0
            x,
            y,
        }
    }

    /// Feed mouse motion: only emit ACTION_MOVE while down.
    pub fn on_motion(&mut self, x: f32, y: f32) -> Option<MotionEvent> {
        if !self.active {
            return None;
        }
        self.x = x;
        self.y = y;
        Some(MotionEvent {
            action: action::ACTION_MOVE,
            pointer_index: 0,
            pointer_id: 0,
            x,
            y,
        })
    }
}

/// Map an X11 keysym (after shift handling) to an Android key code.
/// This is a small, game-relevant subset (text/chat + DPAD + system keys).
pub fn x11_keysym_to_keycode(keysym: u32) -> i32 {
    match keysym {
        0xFF0D /*Return*/ => keycode::AKEYCODE_ENTER,
        0xFF08 /*BackSpace*/ => keycode::AKEYCODE_DEL,
        0xFF09 /*Tab*/ => keycode::AKEYCODE_TAB,
        0xFF1B /*Escape*/ => keycode::AKEYCODE_ESCAPE,
        0xFF52 /*Up*/ => keycode::AKEYCODE_DPAD_UP,
        0xFF54 /*Down*/ => keycode::AKEYCODE_DPAD_DOWN,
        0xFF51 /*Left*/ => keycode::AKEYCODE_DPAD_LEFT,
        0xFF53 /*Right*/ => keycode::AKEYCODE_DPAD_RIGHT,
        0x0020 /*space*/ => keycode::AKEYCODE_SPACE,
        0x0061..=0x007A => keycode::AKEYCODE_A + (keysym - 0x0061) as i32,
        0x0030..=0x0039 => keycode::AKEYCODE_0 + (keysym - 0x0030) as i32,
        _ => keycode::AKEYCODE_UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_press_release_turns_into_down_up() {
        let mut tr = PointerTracker::default();
        let down = tr.on_button(button::PRIMARY, true, 100.0, 50.0);
        assert_eq!(down.action, action::ACTION_DOWN);
        assert_eq!(down.x, 100.0);
        assert_eq!(down.y, 50.0);
        let up = tr.on_button(button::PRIMARY, false, 120.0, 60.0);
        assert_eq!(up.action, action::ACTION_UP);
        assert!(!tr.active);
    }

    #[test]
    fn motion_only_emits_move_while_down() {
        let mut tr = PointerTracker::default();
        assert!(tr.on_motion(10.0, 10.0).is_none(), "no move before down");
        tr.on_button(button::PRIMARY, true, 10.0, 10.0);
        let m = tr.on_motion(30.0, 25.0).expect("move while down");
        assert_eq!(m.action, action::ACTION_MOVE);
        assert_eq!(m.x, 30.0);
        assert_eq!(m.y, 25.0);
        tr.on_button(button::PRIMARY, false, 30.0, 25.0);
        assert!(tr.on_motion(40.0, 40.0).is_none(), "no move after up");
    }

    #[test]
    fn keysyms_map_to_android_keycodes() {
        assert_eq!(x11_keysym_to_keycode(0xFF0D), keycode::AKEYCODE_ENTER);
        assert_eq!(x11_keysym_to_keycode(0x0061), keycode::AKEYCODE_A); // 'a'
        assert_eq!(x11_keysym_to_keycode(0x007A), keycode::AKEYCODE_Z); // 'z'
        assert_eq!(x11_keysym_to_keycode(0x0030), keycode::AKEYCODE_0); // '0'
        assert_eq!(x11_keysym_to_keycode(0x0039), keycode::AKEYCODE_9);
        assert_eq!(x11_keysym_to_keycode(0xFF52), keycode::AKEYCODE_DPAD_UP);
        assert_eq!(x11_keysym_to_keycode(0x9999), keycode::AKEYCODE_UNKNOWN);
    }
}