use super::*;

impl Doomsday {
    pub fn draw_hud(&self, d: &mut RaylibDrawHandle, aim: Option<TravelTarget>, u: &Uniforms) {
        let ink = Color::new(12, 15, 22, 245);
        let muted = Color::new(194, 198, 207, 255);
        let red = Color::new(255, 119, 86, 255);
        let white = Color::new(243, 244, 239, 255);
        let target = if self.busy() { self.target } else { aim };
        if let Some(t) = target
            && self.phase != Phase::Aftermath
            && let Some((p, _)) = project_vertex(
                t.position,
                u.camera_pos,
                u.camera_target,
                u.camera_up,
                800.,
                600.,
            )
        {
            let r = projected_sphere_radius(t.position, t.radius, u, 600).clamp(24., 170.) + 12.;
            if p.x + r > 0. && p.x - r < 800. && p.y + r > 0. && p.y - r < 600. {
                let x = (p.x - r) as i32;
                let y = (p.y - r) as i32;
                let side = (r * 2.) as i32;
                for (cx, cy, sx, sy) in [
                    (x, y, 1, 1),
                    (x + side, y, -1, 1),
                    (x, y + side, 1, -1),
                    (x + side, y + side, -1, -1),
                ] {
                    d.draw_line(cx, cy, cx + sx * 13, cy, red);
                    d.draw_line(cx, cy, cx, cy + sy * 13, red);
                }
            }
        }
        // Crosshair remains discoverable before a target can be locked.
        if self.phase == Phase::Idle {
            let nose = crate::model_point(&u.model_matrix, Vector3::new(0., 0., 1_000_000.));
            let p = project_vertex(nose, u.camera_pos, u.camera_target, u.camera_up, 800., 600.)
                .map_or(Vector2::new(400., 300.), |(p, _)| p);
            let (x, y) = (p.x as i32, p.y as i32);
            for (a, b, c, e) in [
                (x - 10, y, x - 4, y),
                (x + 4, y, x + 10, y),
                (x, y - 10, x, y - 4),
                (x, y + 4, x, y + 10),
            ] {
                d.draw_line(a, b, c, e, white);
            }
        }
        if self.phase == Phase::Confirm {
            d.draw_rectangle(0, 0, 800, 600, Color::new(3, 5, 10, 155));
            d.draw_rectangle(88, 158, 624, 290, ink);
            d.draw_rectangle_lines(88, 158, 624, 290, red);
            d.draw_text("¿Estás seguro de que quieres", 120, 186, 26, white);
            d.draw_text("exterminar un planeta completo?", 120, 220, 26, white);
            d.draw_text(
                &format!("Objetivo: {}", self.target.unwrap().name),
                120,
                272,
                20,
                red,
            );
            d.draw_text("La carga está lista. Tú decides.", 120, 304, 17, muted);
            for (x, label, selected) in [
                (120, "Sí, lanzar", self.launch_selected),
                (408, "Ay noooo", !self.launch_selected),
            ] {
                d.draw_rectangle(
                    x,
                    344,
                    272,
                    52,
                    if selected {
                        Color::new(72, 35, 32, 255)
                    } else {
                        Color::new(28, 32, 41, 255)
                    },
                );
                d.draw_rectangle_lines(x, 344, 272, 52, if selected { red } else { muted });
                d.draw_text(label, x + 22, 360, 22, white);
            }
            d.draw_text(
                "Flechas: elegir     Enter: confirmar     Esc: cancelar",
                120,
                418,
                15,
                muted,
            );
            return;
        }
        if self.phase == Phase::Charging {
            d.draw_rectangle(16, 414, 768, 170, ink);
            d.draw_rectangle_lines(16, 414, 768, 170, red);
            d.draw_text("Carga del reactor", 40, 433, 25, white);
            d.draw_text(
                &format!(
                    "Estabilización: {:02} s",
                    (MIN_CHARGE_SECONDS - self.charge_time).max(0.).ceil() as i32
                ),
                345,
                440,
                15,
                muted,
            );
            d.draw_text(
                &format!("{:03}%", (self.charge * 100.).floor() as i32),
                637,
                430,
                34,
                red,
            );
            d.draw_text(
                &format!("Objetivo: {}", self.target.unwrap().name),
                40,
                470,
                17,
                muted,
            );
            d.draw_text(
                "Al 100 %: pregunta final con Sí, lanzar / Ay noooo",
                40,
                489,
                13,
                muted,
            );
            d.draw_rectangle(
                40,
                507,
                54,
                49,
                if self.tap_flash > 0. {
                    Color::new(121, 49, 30, 255)
                } else {
                    Color::new(35, 39, 47, 255)
                },
            );
            d.draw_rectangle_lines(40, 507, 54, 49, red);
            d.draw_text("J", 58, 517, 29, white);
            d.draw_rectangle(114, 509, 630, 22, Color::new(38, 42, 49, 255));
            d.draw_rectangle(114, 509, (630. * self.charge) as i32, 22, red);
            for i in 1..20 {
                d.draw_line(114 + i * 630 / 20, 509, 114 + i * 630 / 20, 531, ink);
            }
            d.draw_text(
                &format!("{} pulsaciones/s  /  ¡Más rápido!", self.taps.len()),
                114,
                541,
                17,
                white,
            );
            d.draw_text(
                "Pulsa y suelta. Mantener J no carga.       B / Esc: cancelar",
                40,
                567,
                13,
                muted,
            );
            return;
        }
        let (title, subtitle) = match self.phase {
            Phase::Idle => (
                aim.map_or("Apunta a un planeta cercano".to_owned(), |t| {
                    format!("Objetivo: {}", t.name)
                }),
                "B: cargar bomba   /   3.14: viajar   /   F8: restaurar planetas",
            ),
            Phase::Flight => (
                "Bomba liberada".to_owned(),
                "Trayectoria fijada. El lanzamiento fue autorizado.",
            ),
            Phase::Rupture => (
                "Impacto confirmado".to_owned(),
                "Ruptura del núcleo planetario",
            ),
            Phase::Aftermath => (
                format!("{}: señal perdida", self.target.unwrap().name),
                "El sistema recordará tu decisión. F8 restaura al terminar.",
            ),
            Phase::Aborting => (
                "Lanzamiento cancelado".to_owned(),
                "Decisión razonable, capitán. Descargando el reactor...",
            ),
            _ => unreachable!(),
        };
        d.draw_rectangle(16, 500, 768, 84, ink);
        d.draw_text(&title, 36, 516, 22, if self.busy() { red } else { white });
        d.draw_text(subtitle, 36, 551, 16, muted);
        if let Some((label, _)) = self.notice {
            d.draw_rectangle(16, 449, 768, 40, ink);
            d.draw_text(label, 30, 461, 16, white);
        }
        if self.phase == Phase::Aftermath && self.timer < 0.35 {
            d.draw_rectangle(
                0,
                0,
                800,
                600,
                Color::new(255, 201, 140, ((1. - self.timer / 0.35) * 95.) as u8),
            );
        }
    }
}
