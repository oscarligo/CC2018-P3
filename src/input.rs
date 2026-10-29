use raylib::prelude::*;

pub struct InputState {
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub bank_left: bool,
    pub bank_right: bool,
    pub orbit_delta: Vector2,
    pub zoom_delta: f32,
}

impl InputState {
    pub fn read(window: &RaylibHandle) -> Self {
        let orbit_delta = if window.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            window.get_mouse_delta()
        } else {
            Vector2::zero()
        };

        Self {
            pitch_up: window.is_key_down(KeyboardKey::KEY_S),
            pitch_down: window.is_key_down(KeyboardKey::KEY_W),
            turn_left: window.is_key_down(KeyboardKey::KEY_D),
            turn_right: window.is_key_down(KeyboardKey::KEY_A),
            bank_left: window.is_key_down(KeyboardKey::KEY_J),
            bank_right: window.is_key_down(KeyboardKey::KEY_L),
            orbit_delta,
            zoom_delta: window.get_mouse_wheel_move(),
        }
    }
}
