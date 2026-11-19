use raylib::prelude::*;

// Propulsión estilo No Man's Sky
const THRUST_KEY: KeyboardKey = KeyboardKey::KEY_SPACE;
const BRAKE_KEY: KeyboardKey = KeyboardKey::KEY_LEFT_SHIFT;

// Alabeo (Roll) en Q/E
const ROLL_LEFT_KEY: KeyboardKey = KeyboardKey::KEY_Q;
const ROLL_RIGHT_KEY: KeyboardKey = KeyboardKey::KEY_E;

// Dirección por teclado
const PITCH_UP_KEY: KeyboardKey = KeyboardKey::KEY_S;
const PITCH_DOWN_KEY: KeyboardKey = KeyboardKey::KEY_W;
const TURN_LEFT_KEY: KeyboardKey = KeyboardKey::KEY_A;
const TURN_RIGHT_KEY: KeyboardKey = KeyboardKey::KEY_D;
const INTERACT_KEY: KeyboardKey = KeyboardKey::KEY_F;

const ORBIT_BUTTON: MouseButton = MouseButton::MOUSE_BUTTON_RIGHT;

#[derive(Default)]
pub struct InputState {
    pub spin_pressed: bool,
    pub spin_held: bool,
    pub firing: bool,
    pub mouse_position: Vector2,
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub bank_left: bool,
    pub bank_right: bool,
    pub thrust: bool,
    pub boost: bool,
    pub braking: bool,
    pub interact_pressed: bool,
    pub mouse_delta: Vector2,
    pub orbiting: bool,
    pub zoom_delta: f32,
}

impl InputState {
    pub fn read(window: &RaylibHandle) -> Self {
        let boost = window.is_key_down(THRUST_KEY);
        let thrust = window.is_key_down(THRUST_KEY) || boost;
        let braking = window.is_key_down(BRAKE_KEY);
        let orbiting = window.is_mouse_button_down(ORBIT_BUTTON);

        Self {
            spin_pressed: window.is_key_pressed(KeyboardKey::KEY_R),
            spin_held: window.is_key_down(KeyboardKey::KEY_R),
            firing: window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT),
            mouse_position: window.get_mouse_position(),
            pitch_up: window.is_key_down(PITCH_UP_KEY),
            pitch_down: window.is_key_down(PITCH_DOWN_KEY),
            turn_left: window.is_key_down(TURN_LEFT_KEY),
            turn_right: window.is_key_down(TURN_RIGHT_KEY),
            bank_left: window.is_key_down(ROLL_LEFT_KEY),
            bank_right: window.is_key_down(ROLL_RIGHT_KEY),
            thrust,
            boost,
            braking,
            interact_pressed: window.is_key_pressed(INTERACT_KEY),
            mouse_delta: window.get_mouse_delta(),
            orbiting,
            zoom_delta: window.get_mouse_wheel_move(),
        }
    }

    pub fn for_vehicle(window: &RaylibHandle, _saucer: bool) -> Self {
        Self::read(window)
    }
}
