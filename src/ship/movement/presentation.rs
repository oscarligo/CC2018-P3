use super::*;

const MAX_SHIP_PITCH_TILT: f32 = 16.0_f32.to_radians();
const MAX_SHIP_YAW_TILT: f32 = 14.0_f32.to_radians();
const AUTO_BANK_MAX_ANGLE: f32 = 32.0_f32.to_radians();
const MANUAL_ROLL_MAX_ANGLE: f32 = 55.0_f32.to_radians();
const SHIP_TILT_SMOOTHING: f32 = 6.5;

impl ShipMovement {
    pub fn draw_trail(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        self.effects.draw_trail(
            framebuffer,
            uniforms,
            self.camera.effective_distance(),
            self.camera.reference_distance(),
        );
    }

    pub fn draw_exhaust(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        self.effects
            .draw_exhaust(framebuffer, uniforms, self.camera.reference_distance());
        if let Some(alien) = &self.alien {
            alien.draw(framebuffer, uniforms, self.target);
        }
        self.impact.draw(framebuffer, uniforms);
    }

    pub fn draw_navigation(&self, d: &mut RaylibDrawHandle) {
        let y = if self.alien.is_some() { 120 } else { 52 };
        let label = if let Some(local) = &self.local {
            let altitude =
                ((self.target - local.body.position).length() - local.body.radius).max(0.0);
            let (state, control) = if local.cruising {
                ("SOBREVUELO", "Shift: parar")
            } else {
                ("PAUSA", "Espacio: seguir")
            };
            format!(
                "{state}: {} / ALT {:.0} / VEL {:.0} / {control} / L: salir",
                local.body.name, altitude, self.speed
            )
        } else if self.local_notice > 0.0 {
            "Acercate a un planeta para activar el vuelo local".to_owned()
        } else {
            "L: vuelo local cerca de un planeta".to_owned()
        };
        let width = d.measure_text(&label, 13) + 24;
        d.draw_rectangle(16, y, width, 27, Color::new(7, 18, 38, 205));
        d.draw_text(&label, 28, y + 7, 13, Color::new(139, 209, 255, 255));
    }

    pub(super) fn update_ship(&mut self, input: &InputState, frame_time: f32) {
        let smooth = 1.0 - (-SHIP_TILT_SMOOTHING * frame_time).exp();
        let target_pitch = (self.pitch_rate * MAX_SHIP_PITCH_TILT)
            .clamp(-MAX_SHIP_PITCH_TILT, MAX_SHIP_PITCH_TILT);
        let target_yaw =
            (self.yaw_rate * MAX_SHIP_YAW_TILT).clamp(-MAX_SHIP_YAW_TILT, MAX_SHIP_YAW_TILT);
        let auto_bank = -self.yaw_rate * AUTO_BANK_MAX_ANGLE;
        let manual_roll = axis_value(input.bank_left, input.bank_right) * MANUAL_ROLL_MAX_ANGLE;
        let target_roll = (auto_bank + manual_roll).clamp(-1.25, 1.25);
        self.ship_pitch += (target_pitch - self.ship_pitch) * smooth;
        self.ship_yaw += (target_yaw - self.ship_yaw) * smooth;
        self.ship_roll += (target_roll - self.ship_roll) * smooth;
    }

    pub(super) fn apply_model(&self, uniforms: &mut Uniforms) {
        let ship_direction = self.orientation.to_matrix();
        let local_attitude = Matrix::rotate_y(self.ship_yaw)
            * Matrix::rotate_x(self.ship_pitch)
            * Matrix::rotate_z(self.ship_roll);
        let special = self.alien.as_ref().map_or(Matrix::identity(), |a| {
            let r = a.visual_rotation();
            let s = a.visual_scale();
            Matrix::scale(s.x, s.y, s.z)
                * Matrix::rotate_y(r.y)
                * Matrix::rotate_x(r.x)
                * Matrix::rotate_z(r.z)
        });
        uniforms.model_matrix = Matrix::translate(
            -self.model_center.x,
            -self.model_center.y,
            -self.model_center.z,
        ) * special
            * local_attitude
            * ship_direction
            * Matrix::translate(self.target.x, self.target.y, self.target.z);
    }
}
