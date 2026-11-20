use super::vehicle::{Vehicle, VehicleKind};
use crate::{
    rendering::{framebuffer::Framebuffer, render::Renderer},
    scene::{Starfield, world::TravelTarget},
};
use raylib::prelude::*;

const MINT: Color = Color::new(115, 195, 255, 255);
const MUTED: Color = Color::new(158, 179, 194, 255);
const PANEL: Color = Color::new(13, 23, 33, 245);
const HANGAR_BUTTON: Rectangle = Rectangle {
    x: 592.,
    y: 16.,
    width: 192.,
    height: 30.,
};

pub fn hangar_requested(window: &RaylibHandle) -> bool {
    window.is_key_pressed(KeyboardKey::KEY_F1)
        || (window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && inside(
                window.get_mouse_position(),
                HANGAR_BUTTON.x as i32,
                HANGAR_BUTTON.y as i32,
                HANGAR_BUTTON.width as i32,
                HANGAR_BUTTON.height as i32,
            ))
}

pub fn draw_hangar_return(d: &mut RaylibDrawHandle) {
    let hovered = inside(d.get_mouse_position(), 592, 16, 192, 30);
    d.draw_rectangle_rec(
        HANGAR_BUTTON,
        if hovered {
            Color::new(25, 58, 108, 245)
        } else {
            PANEL
        },
    );
    d.draw_rectangle_lines(592, 16, 192, 30, if hovered { MINT } else { MUTED });
    text(d, "F1: Cambiar nave", 610, 24, 16, MINT);
}

fn text(d: &mut RaylibDrawHandle, label: &str, x: i32, y: i32, size: i32, color: Color) {
    d.draw_text(label, x, y, size, color);
}
fn inside(p: Vector2, x: i32, y: i32, w: i32, h: i32) -> bool {
    p.x >= x as f32 && p.x < (x + w) as f32 && p.y >= y as f32 && p.y < (y + h) as f32
}
fn button(d: &mut RaylibDrawHandle, x: i32, y: i32, w: i32, label: &str, selected: bool) {
    let mut font_size = 20;
    while font_size > 13 && d.measure_text(label, font_size) > w - 36 {
        font_size -= 1;
    }
    d.draw_rectangle(
        x,
        y,
        w,
        52,
        if selected {
            Color::new(25, 58, 108, 245)
        } else {
            PANEL
        },
    );
    d.draw_rectangle_lines(x, y, w, 52, if selected { MINT } else { MUTED });
    text(
        d,
        label,
        x + 18,
        y + 17,
        font_size,
        if selected { MINT } else { Color::RAYWHITE },
    );
}

pub fn choose_vehicle(
    window: &mut RaylibHandle,
    thread: &RaylibThread,
    fb: &mut Framebuffer,
    renderer: &mut Renderer,
    vehicles: &mut [Vehicle; 3],
    initial_selection: usize,
) -> Option<usize> {
    choose_vehicle_with_input(
        window,
        thread,
        fb,
        renderer,
        vehicles,
        initial_selection,
        |_, _| {},
    )
}

fn choose_vehicle_with_input(
    window: &mut RaylibHandle,
    thread: &RaylibThread,
    fb: &mut Framebuffer,
    renderer: &mut Renderer,
    vehicles: &mut [Vehicle; 3],
    initial_selection: usize,
    mut before_input: impl FnMut(&mut RaylibHandle, usize),
) -> Option<usize> {
    let stars = Starfield::new();
    let mut selected = initial_selection.min(vehicles.len() - 1);
    let mut elapsed = 0.0;
    let mut frame = 0;
    while !window.window_should_close() {
        before_input(window, frame);
        crate::display::update(window);
        // Draw once before accepting input from a previous flight or dialog.
        if frame > 0 {
            if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                return None;
            }
            if window.is_key_pressed(KeyboardKey::KEY_LEFT) {
                selected = (selected + 2) % 3;
            }
            if window.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                selected = (selected + 1) % 3;
            }
            for (i, key) in [
                KeyboardKey::KEY_ONE,
                KeyboardKey::KEY_TWO,
                KeyboardKey::KEY_THREE,
            ]
            .into_iter()
            .enumerate()
            {
                if window.is_key_pressed(key) {
                    selected = i;
                }
            }
            let p = window.get_mouse_position();
            if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                for i in 0..3 {
                    if inside(p, 28 + i as i32 * 252, 430, 240, 52) {
                        selected = i;
                    }
                }
                if inside(p, 260, 530, 280, 52) {
                    return Some(selected);
                }
            }
            if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
                return Some(selected);
            }
        }
        frame += 1;
        elapsed += window.get_frame_time().min(0.1);
        fb.clear();
        renderer.begin_frame();
        // A gentle turntable always presents the pilot's face towards the viewer.
        let u = vehicles[selected].preview_uniforms((elapsed * 0.38).sin() * 0.65);
        stars.draw(fb, &u);
        vehicles[selected].draw(renderer, fb, &u, elapsed);
        fb.present(window, thread, None, |d| draw_hangar(d, selected));
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    None
}

pub(super) fn draw_hangar(d: &mut RaylibDrawHandle, selected: usize) {
    d.draw_rectangle(0, 0, 800, 112, Color::new(7, 13, 22, 235));
    text(d, "GALAXY / HANGAR", 48, 28, 16, MINT);
    text(d, "F11: Pantalla completa", 550, 28, 16, MUTED);
    text(d, "Elige tu nave", 48, 56, 34, Color::RAYWHITE);
    text(d, &format!("0{} / 03", selected + 1), 660, 66, 18, MUTED);
    d.draw_rectangle(0, 415, 800, 185, Color::new(7, 13, 22, 235));
    button(d, 28, 430, 240, "1  CARRO ORIGINAL", selected == 0);
    button(d, 280, 430, 240, "2  OVNI / GATO ALIEN", selected == 1);
    button(d, 532, 430, 240, "3  DESTRUCTOR ESTELAR", selected == 2);
    let info = if selected == 0 {
        "Vuelo clasico / Velocidad 1x / fm: radio del carro"
    } else if selected == 1 {
        "Espacio: acelerar / R: giro / mishmish: punteria / achu"
    } else {
        "Destructor Estelar / B: bomba / J: cargar / 3.14: viajar"
    };
    text(d, info, 48, 498, 16, MUTED);
    button(d, 260, 530, 280, "DESPEGAR  [ENTER]", true);
}

#[derive(Default)]
pub struct TravelMenu {
    pub open: bool,
    selected: usize,
}
impl TravelMenu {
    pub fn update(
        &mut self,
        window: &mut RaylibHandle,
        kind: VehicleKind,
        requested: bool,
        targets: &[TravelTarget],
    ) -> Option<usize> {
        if kind != VehicleKind::Car && requested {
            self.open = true;
        }
        if !self.open || targets.is_empty() {
            return None;
        }
        self.selected = self.selected.min(targets.len() - 1);
        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.open = false;
            return None;
        }
        let wheel = window.get_mouse_wheel_move();
        if window.is_key_pressed(KeyboardKey::KEY_DOWN) || wheel < 0.0 {
            self.selected = (self.selected + 1) % targets.len();
        }
        if window.is_key_pressed(KeyboardKey::KEY_UP) || wheel > 0.0 {
            self.selected = (self.selected + targets.len() - 1) % targets.len();
        }
        let page = self.selected / 8 * 8;
        if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let p = window.get_mouse_position();
            for row in 0..8 {
                if page + row < targets.len() && inside(p, 128, 148 + row as i32 * 38, 544, 34) {
                    self.selected = page + row;
                }
            }
            if inside(p, 260, 506, 280, 52) {
                self.open = false;
                return Some(self.selected);
            }
        }
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
            self.open = false;
            return Some(self.selected);
        }
        None
    }

    pub fn draw(
        &self,
        d: &mut RaylibDrawHandle,
        kind: VehicleKind,
        targets: &[TravelTarget],
        flash: f32,
    ) {
        if flash > 0.0 {
            d.draw_rectangle(
                0,
                0,
                800,
                600,
                Color::new(94, 175, 255, (flash.min(1.0) * 110.0) as u8),
            );
        }
        if !self.open {
            d.draw_rectangle(16, 16, 370, 30, Color::new(7, 13, 22, 190));
            text(
                d,
                if kind == VehicleKind::Saucer {
                    "OVNI  /  3x  /  Teletransporte: 3.14"
                } else if kind == VehicleKind::Destroyer {
                    "DESTRUCTOR ESTELAR  /  B: bomba"
                } else {
                    "CARRO  /  1x  /  fm: radio"
                },
                28,
                24,
                16,
                if kind == VehicleKind::Saucer {
                    MINT
                } else {
                    Color::RAYWHITE
                },
            );
            return;
        }
        d.draw_rectangle(0, 0, 800, 600, Color::new(1, 7, 14, 205));
        d.draw_rectangle(108, 48, 584, 528, PANEL);
        d.draw_rectangle_lines(108, 48, 584, 528, MINT);
        text(d, "3.14 / MOTOR DE SALTO", 132, 69, 16, MINT);
        text(d, "Elige un destino", 132, 96, 30, Color::RAYWHITE);
        let page = self.selected / 8 * 8;
        for (row, target) in targets.iter().skip(page).take(8).enumerate() {
            let y = 148 + row as i32 * 38;
            if page + row == self.selected {
                d.draw_rectangle(128, y, 544, 34, Color::new(25, 58, 108, 255));
            }
            text(
                d,
                &format!("{:02}   {}", page + row + 1, target.name),
                140,
                y + 8,
                18,
                if page + row == self.selected {
                    MINT
                } else {
                    Color::RAYWHITE
                },
            );
        }
        if let Some(t) = targets.get(self.selected) {
            text(
                d,
                &format!(
                    "Posicion orbital: {:.0}, {:.0}, {:.0}",
                    t.position.x, t.position.y, t.position.z
                ),
                132,
                463,
                16,
                MUTED,
            );
        }
        text(
            d,
            &format!(
                "Flechas / rueda  |  Pagina {}/{}  |  Esc: volver",
                self.selected / 8 + 1,
                targets.len().div_ceil(8)
            ),
            132,
            485,
            15,
            MUTED,
        );
        button(d, 260, 506, 280, "TELETRANSPORTAR [ENTER]", true);
    }
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
