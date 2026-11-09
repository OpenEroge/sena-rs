use crate::event::{InputEvent, MouseButton};

/// PAL logical key identifiers. These are high-level keys used by game scripts.
/// Bit positions follow PAL_d.dll's virtual-key table at 0x103CB130.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PalKey {
    Return,
    Escape,
    Space,
    Ctrl,
    Up,
    Down,
    Left,
    Right,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    MouseLeft,
    MouseRight,
    MouseMiddle,
}

/// PAL mouse button identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PalMouseButton {
    Left,
    Right,
    Middle,
}

const KEY_RETURN: u32 = 1 << 2;
const KEY_ESCAPE: u32 = 1 << 0;
const KEY_SPACE: u32 = 1 << 1;
const KEY_CTRL: u32 = 1 << 3;
const KEY_UP: u32 = 1 << 7;
const KEY_DOWN: u32 = 1 << 8;
const KEY_LEFT: u32 = 1 << 9;
const KEY_RIGHT: u32 = 1 << 10;
const KEY_F1: u32 = 1 << 11;
const KEY_F2: u32 = 1 << 12;
const KEY_F3: u32 = 1 << 13;
const KEY_F4: u32 = 1 << 14;
const KEY_F5: u32 = 1 << 15;
const KEY_F6: u32 = 1 << 16;
const KEY_F7: u32 = 1 << 17;
const KEY_F8: u32 = 1 << 18;
const KEY_F9: u32 = 1 << 19;
const KEY_F10: u32 = 1 << 20;
const KEY_F11: u32 = 1 << 21;
const KEY_F12: u32 = 1 << 22;

const MOUSE_LEFT: u8 = 1 << 0;
const MOUSE_RIGHT: u8 = 1 << 1;
const MOUSE_MIDDLE: u8 = 1 << 2;

fn pal_key_bit(key: PalKey) -> u32 {
    match key {
        PalKey::Return => KEY_RETURN,
        PalKey::Escape => KEY_ESCAPE,
        PalKey::Space => KEY_SPACE,
        PalKey::Ctrl => KEY_CTRL,
        PalKey::Up => KEY_UP,
        PalKey::Down => KEY_DOWN,
        PalKey::Left => KEY_LEFT,
        PalKey::Right => KEY_RIGHT,
        PalKey::F1 => KEY_F1,
        PalKey::F2 => KEY_F2,
        PalKey::F3 => KEY_F3,
        PalKey::F4 => KEY_F4,
        PalKey::F5 => KEY_F5,
        PalKey::F6 => KEY_F6,
        PalKey::F7 => KEY_F7,
        PalKey::F8 => KEY_F8,
        PalKey::F9 => KEY_F9,
        PalKey::F10 => KEY_F10,
        PalKey::F11 => KEY_F11,
        PalKey::F12 => KEY_F12,
        // Mouse keys are tracked separately; these overlap with mouse_on mask
        PalKey::MouseLeft | PalKey::MouseRight | PalKey::MouseMiddle => 0,
    }
}

fn pal_mouse_bit(button: PalMouseButton) -> u8 {
    match button {
        PalMouseButton::Left => MOUSE_LEFT,
        PalMouseButton::Right => MOUSE_RIGHT,
        PalMouseButton::Middle => MOUSE_MIDDLE,
    }
}

// Map winit logical key name strings to PAL key bits.
fn key_name_to_bit(key_name: &str) -> u32 {
    match key_name {
        "Return" | "Enter" | "NumpadEnter" => KEY_RETURN,
        "Escape" => KEY_ESCAPE,
        "Space" | " " => KEY_SPACE,
        "Ctrl" | "Control" | "ControlLeft" | "ControlRight" | "LeftCtrl" | "RightCtrl" => KEY_CTRL,
        "Shift" | "ShiftLeft" | "ShiftRight" => 1 << 4,
        "Alt" | "AltLeft" | "AltRight" => 1 << 5,
        "Tab" => 1 << 6,
        "y" | "Y" | "KeyY" => 1 << 26,
        "n" | "N" | "KeyN" => 1 << 27,
        "z" | "Z" | "KeyZ" => 1 << 28,
        "x" | "X" | "KeyX" => 1 << 29,
        "c" | "C" | "KeyC" => 1 << 30,
        "ArrowUp" => KEY_UP,
        "ArrowDown" => KEY_DOWN,
        "ArrowLeft" => KEY_LEFT,
        "ArrowRight" => KEY_RIGHT,
        "F1" => KEY_F1,
        "F2" => KEY_F2,
        "F3" => KEY_F3,
        "F4" => KEY_F4,
        "F5" => KEY_F5,
        "F6" => KEY_F6,
        "F7" => KEY_F7,
        "F8" => KEY_F8,
        "F9" => KEY_F9,
        "F10" => KEY_F10,
        "F11" => KEY_F11,
        "F12" => KEY_F12,
        _ => 0,
    }
}

fn key_name_to_vk(name: &str) -> Option<u32> {
    let bit = key_name_to_bit(name);
    const VK: [u32; 31] = [
        0x1b, 0x20, 0x0d, 0x11, 0x10, 0x12, 0x09, 0x26, 0x28, 0x25, 0x27, 0x70, 0x71, 0x72, 0x73,
        0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 1, 2, 4, 0x59, 0x4e, 0x5a, 0x58, 0x43,
    ];
    if bit != 0 {
        return Some(VK[bit.trailing_zeros() as usize]);
    }
    let key = name
        .strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(name);
    if key.len() == 1 {
        let b = key.as_bytes()[0].to_ascii_uppercase();
        if b.is_ascii_alphanumeric() {
            return Some(b as u32);
        }
    }
    None
}

fn winit_mouse_to_pal(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => MOUSE_LEFT,
        MouseButton::Right => MOUSE_RIGHT,
        MouseButton::Middle => MOUSE_MIDDLE,
        _ => 0,
    }
}

/// PAL-style stateful input. Three-state (on/push/pull) for keyboard and mouse.
///
/// PAL's input system keeps per-frame push/on/pull masks computed in sub_10246500.
/// Winit events feed into this state; the frame boundary is marked by begin_frame().
///
/// Mouse coordinates are PAL logical coordinates. Winit feeds physical window
/// coordinates, then the engine maps them through the current render scale.
#[derive(Clone, Debug, Default)]
pub struct PalInputState {
    // Keyboard three-state (bitmask over PalKey variants)
    key_on: u32,
    physical_key_on: u32,
    stable_key_on: u32,
    cancel_keys: u32,
    cancel_extended: u32,
    physical_virtual: [u64; 4],
    key_push: u32,
    key_pull: u32,
    repeat_output: u32,
    repeat_source: u32,
    repeat_started_ms: u32,

    // Mouse button three-state
    mouse_on: u8,
    physical_mouse_on: u8,
    stable_mouse_on: u8,
    mouse_push: u8,
    mouse_pull: u8,

    // Mouse position in PAL logical coordinates.
    mouse_x: i32,
    mouse_y: i32,

    // Per-frame movement delta; cleared by begin_frame.
    mouse_move_x: i32,
    mouse_move_y: i32,

    // Running cursor position in PAL logical coordinates for delta computation.
    cursor_prev_x: i32,
    cursor_prev_y: i32,

    // Last raw physical cursor position from winit.
    cursor_raw_x: f64,
    cursor_raw_y: f64,

    // Current physical-to-PAL coordinate transform.
    window_width: u32,
    window_height: u32,
    logical_width: u32,
    logical_height: u32,

    // Last native wheel direction; cleared by begin_frame.
    wheel_delta: f32,

    // Tracks whether cursor position has ever been set.
    cursor_initialized: bool,
}

impl PalInputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_coordinate_space(
        &mut self,
        window_width: u32,
        window_height: u32,
        logical_width: u32,
        logical_height: u32,
    ) {
        self.window_width = window_width.max(1);
        self.window_height = window_height.max(1);
        self.logical_width = logical_width.max(1);
        self.logical_height = logical_height.max(1);
        if self.cursor_initialized {
            let (x, y) = self.map_physical_to_logical(self.cursor_raw_x, self.cursor_raw_y);
            self.mouse_x = x;
            self.mouse_y = y;
            self.cursor_prev_x = x;
            self.cursor_prev_y = y;
        }
    }

    /// Clear per-frame transient state: push, pull, move delta, wheel.
    /// Must be called once per frame before new input events are processed.
    pub fn begin_frame(&mut self) {
        self.key_on = self.physical_key_on;
        self.stable_key_on = self.physical_key_on;
        self.mouse_on = self.physical_mouse_on;
        self.stable_mouse_on = self.physical_mouse_on;
        self.key_push = 0;
        self.key_pull = 0;
        self.mouse_push = 0;
        self.mouse_pull = 0;
        self.mouse_move_x = 0;
        self.mouse_move_y = 0;
        self.wheel_delta = 0.0;
        self.repeat_output = 0;
    }

    /// Sample PAL's repeat level once after host events and before script input.
    /// A new push replaces the entire remembered mask. After 500 ms, any
    /// surviving held bit keeps that whole mask repeating on every sample.
    pub fn sample_repeat(&mut self, now_ms: u32) {
        let push = self.key_push | ((self.mouse_push as u32) << 23);
        let held = self.stable_key_on | ((self.stable_mouse_on as u32) << 23);
        self.repeat_output = 0;
        if push != 0 {
            self.repeat_source = push;
            self.repeat_started_ms = now_ms;
        } else if self.repeat_source & held != 0 {
            if now_ms.wrapping_sub(self.repeat_started_ms) > 500 {
                self.repeat_output = self.repeat_source & !self.cancel_keys;
            }
        } else {
            self.repeat_source = 0;
        }
    }

    /// PalInputKeyRepeatClear preserves physical input and the timer value.
    pub fn clear_repeat(&mut self) {
        self.repeat_output = 0;
        self.repeat_source = 0;
    }

    /// Clear PAL's accumulated input masks. Mirrors PalInputClear_0's block reset.
    pub fn clear(&mut self) {
        self.key_on = 0;
        self.stable_key_on = 0;
        self.key_push = 0;
        self.key_pull = 0;
        self.mouse_on = 0;
        self.stable_mouse_on = 0;
        self.mouse_push = 0;
        self.mouse_pull = 0;
        self.repeat_output = 0;
    }

    /// Persistent native PalInputKeyCancel masks. The first mask also includes
    /// mouse buttons (bits 23–25); the second masks key-ex events, including wheel.
    pub fn set_cancel_masks(&mut self, keys: u32, extended: u32) {
        self.cancel_keys = keys;
        self.cancel_extended = extended;
        self.key_on &= !keys;
        self.stable_key_on &= !keys;
        self.key_push &= !keys;
        self.key_pull &= !keys;
        self.repeat_output &= !keys;
        let mouse = !((keys >> 23) as u8);
        self.mouse_on &= mouse;
        self.stable_mouse_on &= mouse;
        self.mouse_push &= mouse;
        self.mouse_pull &= mouse;
        if extended & 0x10 != 0 {
            self.wheel_delta = 0.0;
        }
    }

    pub fn async_virtual_key(&self, vk: u32) -> bool {
        vk < 256 && self.physical_virtual[vk as usize / 64] & (1 << (vk % 64)) != 0
    }

    fn set_physical_virtual(&mut self, vk: u32, pressed: bool) {
        if vk >= 256 {
            return;
        }
        let value = &mut self.physical_virtual[vk as usize / 64];
        if pressed {
            *value |= 1 << (vk % 64);
        } else {
            *value &= !(1 << (vk % 64));
        }
    }

    /// OR raw PAL key bits into the held-key mask.
    pub fn set_key_on_bits(&mut self, bits: u32) {
        self.key_on |= bits;
        self.stable_key_on |= bits;
    }

    /// OR raw PAL key bits into the per-frame push mask.
    pub fn set_key_push_bits(&mut self, bits: u32) {
        self.key_push |= bits;
    }

    /// OR raw PAL key bits into the per-frame pull mask.
    pub fn set_key_pull_bits(&mut self, bits: u32) {
        self.key_pull |= bits;
    }

    /// Process a keyboard event.
    pub fn handle_keyboard_event(&mut self, key_name: &str, pressed: bool) {
        if let Some(vk) = key_name_to_vk(key_name) {
            self.set_physical_virtual(vk, pressed);
        }
        let bit = key_name_to_bit(key_name);
        if bit == 0 {
            return;
        }
        if pressed {
            if self.physical_key_on & bit == 0 {
                self.key_push |= bit;
            }
            self.physical_key_on |= bit;
            self.key_on |= bit;
        } else {
            if self.physical_key_on & bit != 0 {
                self.key_pull |= bit;
            }
            self.physical_key_on &= !bit;
            self.stable_key_on &= !bit;
            self.key_on &= !bit;
        }
    }

    /// Process a mouse button event.
    pub fn handle_mouse_button_event(&mut self, button: MouseButton, pressed: bool) {
        let bit = winit_mouse_to_pal(button);
        let vk = match button {
            MouseButton::Left => 1,
            MouseButton::Right => 2,
            MouseButton::Middle => 4,
            _ => 0,
        };
        if vk != 0 {
            self.set_physical_virtual(vk, pressed);
        }
        if bit == 0 {
            return;
        }
        if pressed {
            if self.physical_mouse_on & bit == 0 {
                self.mouse_push |= bit;
            }
            self.physical_mouse_on |= bit;
            self.mouse_on |= bit;
        } else {
            if self.physical_mouse_on & bit != 0 {
                self.mouse_pull |= bit;
            }
            self.physical_mouse_on &= !bit;
            self.stable_mouse_on &= !bit;
            self.mouse_on &= !bit;
        }
    }

    /// Process a cursor-moved event.
    pub fn handle_cursor_moved(&mut self, x: f64, y: f64) {
        self.cursor_raw_x = x;
        self.cursor_raw_y = y;
        let (new_x, new_y) = self.map_physical_to_logical(x, y);
        if self.cursor_initialized {
            self.mouse_move_x = if new_x == -1 {
                0
            } else {
                self.mouse_move_x.wrapping_add(new_x.wrapping_sub(self.cursor_prev_x))
            };
            self.mouse_move_y = if new_y == -1 {
                0
            } else {
                self.mouse_move_y.wrapping_add(new_y.wrapping_sub(self.cursor_prev_y))
            };
        }
        self.cursor_prev_x = new_x;
        self.cursor_prev_y = new_y;
        self.mouse_x = new_x;
        self.mouse_y = new_y;
        self.cursor_initialized = true;
    }

    fn map_physical_to_logical(&self, x: f64, y: f64) -> (i32, i32) {
        fn axis(value: f64, origin: f32, span: f32, logical: u32) -> i32 {
            if logical == 0 {
                return value as i32;
            }
            // Native truncates toward zero and keeps signed off-screen values.
            let mapped = ((value as f32 - origin) * (logical as f32 / span.max(1.0))) as i32;
            let extent = logical.min(i32::MAX as u32) as i32;
            if mapped >= extent || mapped < -extent { -1 } else { mapped }
        }
        // The renderer letterboxes the PAL screen; map through the same rect.
        let viewport = crate::scene::letterbox_rect(
            [self.window_width, self.window_height],
            [self.logical_width, self.logical_height],
        );
        (
            axis(x, viewport.x, viewport.w, self.logical_width),
            axis(y, viewport.y, viewport.h, self.logical_height),
        )
    }

    /// Process a mouse wheel event.
    pub fn handle_mouse_wheel(&mut self, _delta_x: f32, delta_y: f32) {
        self.wheel_delta = if delta_y < 0.0 { 1.0 } else { -1.0 };
    }

    /// True if the key was pressed this frame (was not held last frame).
    pub fn key_push(&self, key: PalKey) -> bool {
        self.raw_key_push() & pal_key_bit(key) != 0
    }

    /// True if the key is currently held.
    pub fn key_on(&self, key: PalKey) -> bool {
        (self.key_on & !self.cancel_keys) & pal_key_bit(key) != 0
    }

    /// True if the key was released this frame.
    pub fn key_pull(&self, key: PalKey) -> bool {
        self.raw_key_pull() & pal_key_bit(key) != 0
    }

    /// True if the mouse button was pressed this frame.
    pub fn mouse_push(&self, button: PalMouseButton) -> bool {
        self.raw_mouse_push() & pal_mouse_bit(button) != 0
    }

    /// True if the mouse button is currently held.
    pub fn mouse_on(&self, button: PalMouseButton) -> bool {
        self.raw_mouse_on() & pal_mouse_bit(button) != 0
    }

    /// True if the mouse button was released this frame.
    pub fn mouse_pull(&self, button: PalMouseButton) -> bool {
        self.raw_mouse_pull() & pal_mouse_bit(button) != 0
    }

    /// Current mouse position in window coordinates.
    pub fn mouse_position(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    /// Per-frame mouse movement delta.
    pub fn mouse_delta(&self) -> (i32, i32) {
        (self.mouse_move_x, self.mouse_move_y)
    }

    /// Last wheel direction: down is +1, up (including a zero host delta) is -1.
    pub fn wheel_delta(&self) -> f32 {
        if self.cancel_extended & 0x10 != 0 {
            0.0
        } else {
            self.wheel_delta
        }
    }

    /// True if any key, mouse button, or positive mouse wheel input was pushed this frame.
    pub fn any_push(&self) -> bool {
        self.raw_key_push() != 0 || self.wheel_delta() > 0.0
    }

    /// True if a push this frame counts as a "click" that completes PAL click
    /// waits and the ADV typewriter reveal.
    ///
    /// Mirrors the push half of koikake.exe GetSkipState (0x437800): Space or
    /// Return push (PalInputGetKeyPush & 0x6), left mouse push (& 0x800000),
    /// or positive wheel delta (PalInputGetWheel > 0) set bit 3. Other keys
    /// (Escape, arrows, F-keys, right/middle mouse) never complete click
    /// waits; they reach scripts only through system-button slots.
    pub fn click_push(&self) -> bool {
        (self.raw_key_on() & (1 << 5) == 0
            && self.raw_key_push() & (KEY_SPACE | KEY_RETURN | (1 << 23)) != 0)
            || self.wheel_delta() > 0.0
    }

    /// True while the PAL fast-forward modifier is held.
    ///
    /// Native ADV skip is driven by PAL's keyboard state, not by a synthetic
    /// VM-side flag.  The headless Ctrl diagnostics feed the same keyboard
    /// path as winit, and wait/text code consumes this held state where the
    /// original engine treats Ctrl as fast-forward.
    pub fn fast_forward_held(&self) -> bool {
        self.raw_key_on() & KEY_CTRL != 0
    }

    /// Return a frame-local copy with mouse push edges removed.
    ///
    /// Game.exe button reactions consume the mouse click before PAL wait-click
    /// tasks see it. The held mouse state is preserved so hover/pressed visuals
    /// continue to render correctly during the same frame.
    pub fn without_mouse_push(&self) -> Self {
        let mut copy = self.clone();
        copy.mouse_push = 0;
        copy
    }

    /// Remove mouse press/release edges consumed by the button layer while
    /// preserving held state for slider drags and visual feedback.
    pub fn without_mouse_edges(&self) -> Self {
        let mut copy = self.without_mouse_push();
        copy.mouse_pull = 0;
        copy
    }

    /// Return a frame-local copy with all push edges removed.
    ///
    /// Used when an input edge has been consumed by the ADV typewriter reveal:
    /// the held state remains available for visuals, but the same edge must not
    /// also complete the following PAL wait-click task.
    pub fn without_push_edges(&self) -> Self {
        let mut copy = self.clone();
        copy.key_push = 0;
        copy.mouse_push = 0;
        copy.wheel_delta = 0.0;
        copy
    }

    /// Raw keyboard push bitmask (for direct PAL API access).
    pub fn raw_key_push(&self) -> u32 {
        (self.key_push | ((self.mouse_push as u32) << 23)) & !self.cancel_keys
    }

    /// Raw keyboard on bitmask.
    pub fn raw_key_on(&self) -> u32 {
        (self.stable_key_on | ((self.stable_mouse_on as u32) << 23)) & !self.cancel_keys
    }

    /// Native PalInputGetKeyOnEx repeat level, distinct from stable held keys.
    pub fn raw_key_on_ex(&self) -> u32 {
        self.repeat_output & !self.cancel_keys
    }

    /// Raw keyboard pull bitmask.
    pub fn raw_key_pull(&self) -> u32 {
        (self.key_pull | ((self.mouse_pull as u32) << 23)) & !self.cancel_keys
    }

    pub fn raw_mouse_on(&self) -> u8 {
        self.mouse_on & !((self.cancel_keys >> 23) as u8)
    }

    pub fn raw_mouse_push(&self) -> u8 {
        self.mouse_push & !((self.cancel_keys >> 23) as u8)
    }

    pub fn raw_mouse_pull(&self) -> u8 {
        self.mouse_pull & !((self.cancel_keys >> 23) as u8)
    }

    /// Feed a PalEvent::Input directly. Convenience wrapper for engine.handle_event.
    pub fn handle_input_event(&mut self, event: &crate::event::InputEvent) {
        match event {
            InputEvent::Keyboard { key_name, pressed } => {
                self.handle_keyboard_event(key_name, *pressed);
            }
            InputEvent::MouseInput { button, pressed } => {
                self.handle_mouse_button_event(*button, *pressed);
            }
            InputEvent::CursorMoved { x, y } => {
                self.handle_cursor_moved(*x, *y);
            }
            InputEvent::MouseWheel { delta_x, delta_y } => {
                self.handle_mouse_wheel(*delta_x, *delta_y);
            }
        }
    }
}

#[cfg(test)]
mod native_input_tests {
    use super::*;

    #[test]
    fn native_key_table_and_mouse_bits_match_pal() {
        let mut input = PalInputState::new();
        for key in [
            "Escape",
            "Space",
            "Return",
            "Ctrl",
            "Shift",
            "Alt",
            "Tab",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "F1",
            "F12",
            "Y",
            "C",
        ] {
            input.handle_keyboard_event(key, true);
        }
        input.handle_mouse_button_event(MouseButton::Left, true);
        input.handle_mouse_button_event(MouseButton::Right, true);
        input.handle_mouse_button_event(MouseButton::Middle, true);
        let expected = 0xfff | (1 << 22) | (7 << 23) | (1 << 26) | (1 << 30);
        assert_eq!(input.raw_key_push(), expected);
        assert_eq!(
            input.raw_key_on(),
            0,
            "native on excludes this sample's new presses"
        );
        input.begin_frame();
        assert_eq!(input.raw_key_on(), expected);
        input.handle_keyboard_event("Return", false);
        input.handle_mouse_button_event(MouseButton::Left, false);
        assert_eq!(input.raw_key_pull(), 4 | (1 << 23));
    }

    #[test]
    fn cursor_maps_through_the_letterboxed_pal_screen() {
        let mut input = PalInputState::new();
        // 2x HiDPI, same aspect: device pixels halve.
        input.set_coordinate_space(2560, 1440, 1280, 720);
        input.handle_cursor_moved(1281.0, 1439.0);
        assert_eq!(input.mouse_position(), (640, 719));
        // 3024x1964 has 131 device rows of bar above the screen.
        input.set_coordinate_space(3024, 1964, 1280, 720);
        input.handle_cursor_moved(0.0, 131.0);
        assert_eq!(input.mouse_position(), (0, 0));
        input.handle_cursor_moved(3023.0, 131.0 + 1700.0);
        assert_eq!(input.mouse_position(), (1279, 719));
        // The bars themselves are outside the PAL screen.
        input.handle_cursor_moved(100.0, 1900.0);
        assert_eq!(input.mouse_position().1, -1);
    }

    #[test]
    fn clear_preserves_physical_keys_wheel_and_mouse_coordinates() {
        let mut input = PalInputState::new();
        input.set_coordinate_space(1280, 720, 1280, 720);
        input.handle_cursor_moved(30.0, 40.0);
        input.handle_keyboard_event("Return", true);
        input.handle_mouse_button_event(MouseButton::Left, true);
        input.handle_mouse_wheel(0.0, 2.0);
        input.clear();
        assert_eq!(input.raw_key_push(), 0);
        assert_eq!(input.raw_key_on(), 0);
        assert_eq!(input.wheel_delta(), -1.0);
        assert_eq!(input.mouse_position(), (30, 40));
        assert!(input.async_virtual_key(0x0d));
        input.handle_keyboard_event("Return", true); // operating-system repeat
        input.handle_mouse_button_event(MouseButton::Left, true);
        assert_eq!(
            input.raw_key_push(),
            0,
            "clear cannot manufacture a new edge"
        );
        input.begin_frame();
        assert_eq!(input.raw_key_on(), 4 | (1 << 23));
        assert_eq!(input.raw_key_push(), 0);
    }

    #[test]
    fn cancellation_suppresses_keys_mouse_wheel_without_erasing_physical_state() {
        let mut input = PalInputState::new();
        input.handle_keyboard_event("Ctrl", true);
        input.handle_mouse_button_event(MouseButton::Left, true);
        input.handle_mouse_wheel(0.0, 1.0);
        input.set_cancel_masks(8 | (1 << 23), 0x10);
        assert!(!input.fast_forward_held());
        assert!(!input.click_push());
        assert_eq!(input.raw_key_push(), 0);
        assert!(input.async_virtual_key(0x11));
        assert!(input.async_virtual_key(1));
        input.set_cancel_masks(0, 0);
        assert_eq!(
            input.raw_key_push(),
            0,
            "unmask does not restore consumed edges"
        );
        input.begin_frame();
        assert_eq!(input.raw_key_on(), 8 | (1 << 23));
        assert_eq!(input.raw_key_push(), 0);
    }
}
