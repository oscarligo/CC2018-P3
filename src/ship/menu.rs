use super::vehicle::{Vehicle, VehicleKind};
use crate::{
    rendering::{framebuffer::Framebuffer, render::Renderer},
    scene::{Starfield, world::TravelTarget},
};
use raylib::prelude::*;

const MINT: Color = Color::new(115, 195, 255, 255);
const MUTED: Color = Color::new(158, 179, 194, 255);
const PANEL: Color = Color::new(13, 23, 33, 245);

fn text(d: &mut RaylibDrawHandle, label: &str, x: i32, y: i32, size: i32, color: Color) {
    d.draw_text(label, x, y, size, color);
}
fn inside(p: Vector2, x: i32, y: i32, w: i32, h: i32) -> bool {
    p.x >= x as f32 && p.x < (x + w) as f32 && p.y >= y as f32 && p.y < (y + h) as f32
}
fn button(d: &mut RaylibDrawHandle, x: i32, y: i32, w: i32, label: &str, selected: bool) {
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
        if label.len() > 21 { 16 } else { 20 },
        if selected { MINT } else { Color::RAYWHITE },
    );
}

pub fn choose_vehicle(
    window: &mut RaylibHandle,
    thread: &RaylibThread,
    fb: &mut Framebuffer,
    renderer: &mut Renderer,
    vehicles: &mut [Vehicle; 2],
) -> Option<usize> {
    let stars = Starfield::new();
    let mut selected = 0;
    let mut elapsed = 0.0;
    while !window.window_should_close() {
        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            return None;
        }
        if window.is_key_pressed(KeyboardKey::KEY_LEFT)
            || window.is_key_pressed(KeyboardKey::KEY_ONE)
        {
            selected = 0;
        }
        if window.is_key_pressed(KeyboardKey::KEY_RIGHT)
            || window.is_key_pressed(KeyboardKey::KEY_TWO)
        {
            selected = 1;
        }
        let p = window.get_mouse_position();
        if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if inside(p, 48, 430, 340, 52) {
                selected = 0;
            }
            if inside(p, 412, 430, 340, 52) {
                selected = 1;
            }
            if inside(p, 260, 530, 280, 52) {
                return Some(selected);
            }
        }
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
            return Some(selected);
        }
        elapsed += window.get_frame_time().min(0.1);
        fb.clear();
        renderer.begin_frame();
        // A gentle turntable always presents the pilot's face towards the viewer.
        let u = vehicles[selected].preview_uniforms((elapsed * 0.38).sin() * 0.65);
        stars.draw(fb, &u);
        vehicles[selected].draw(renderer, fb, &u, elapsed);
        fb.present(window, thread, None, |d| {
            d.draw_rectangle(0, 0, 800, 112, Color::new(7, 13, 22, 235));
            text(d, "GALAXY / HANGAR", 48, 28, 16, MINT);
            text(d, "Elige tu nave", 48, 56, 34, Color::RAYWHITE);
            text(d, &format!("0{} / 02", selected + 1), 660, 66, 18, MUTED);
            d.draw_rectangle(0, 415, 800, 185, Color::new(7, 13, 22, 235));
            button(d, 48, 430, 340, "1   CARRO ORIGINAL", selected == 0);
            button(d, 412, 430, 340, "2   OVNI / GATO ALIEN", selected == 1);
            let info = if selected == 0 {
                "Vuelo clasico / Velocidad 1x / fm: radio del carro"
            } else {
                "Espacio: acelerar / R: giro / mishmish: punteria / achu"
            };
            text(d, info, 48, 498, 16, MUTED);
            button(d, 260, 530, 280, "DESPEGAR  [ENTER]", true);
        });
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    None
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
        if kind == VehicleKind::Saucer && requested {
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
