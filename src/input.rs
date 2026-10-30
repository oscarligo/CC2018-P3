use raylib::prelude::*;

const PITCH_UP_KEY: KeyboardKey = KeyboardKey::KEY_W;
const PITCH_DOWN_KEY: KeyboardKey = KeyboardKey::KEY_S;
const TURN_LEFT_KEY: KeyboardKey = KeyboardKey::KEY_L;
const TURN_RIGHT_KEY: KeyboardKey = KeyboardKey::KEY_J;
const BANK_LEFT_KEY: KeyboardKey = KeyboardKey::KEY_A;
const BANK_RIGHT_KEY: KeyboardKey = KeyboardKey::KEY_D;
const THRUST_KEY: KeyboardKey = KeyboardKey::KEY_SPACE;
const BRAKE_LEFT_KEY: KeyboardKey = KeyboardKey::KEY_LEFT_SHIFT;
const BRAKE_RIGHT_KEY: KeyboardKey = KeyboardKey::KEY_RIGHT_SHIFT;
const ORBIT_BUTTON: MouseButton = MouseButton::MOUSE_BUTTON_LEFT;

pub struct InputState {
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub bank_left: bool,
    pub bank_right: bool,
    pub thrust: bool,
    pub braking: bool,
    pub orbit_delta: Vector2,
    pub zoom_delta: f32,
}

impl InputState {
    pub fn read(window: &RaylibHandle) -> Self {
        let orbit_delta = if window.is_mouse_button_down(ORBIT_BUTTON) {
            window.get_mouse_delta()
        } else {
            Vector2::zero()
        };

        let braking = window.is_key_down(BRAKE_LEFT_KEY) || window.is_key_down(BRAKE_RIGHT_KEY);
        Self {
            pitch_up: window.is_key_down(PITCH_UP_KEY),
            pitch_down: window.is_key_down(PITCH_DOWN_KEY) || braking,
            turn_left: window.is_key_down(TURN_LEFT_KEY),
            turn_right: window.is_key_down(TURN_RIGHT_KEY),
            bank_left: window.is_key_down(BANK_LEFT_KEY),
            bank_right: window.is_key_down(BANK_RIGHT_KEY),
            thrust: window.is_key_down(THRUST_KEY),
            braking,
            orbit_delta,
            zoom_delta: window.get_mouse_wheel_move(),
        }
    }
}
