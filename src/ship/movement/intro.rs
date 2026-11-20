use super::*;

pub(crate) fn entry_fade(time: f32) -> u8 {
    (smooth(-(time + 3.0) / 3.0) * 255.0).round() as u8
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn camera_pose(time: f32, radius: f32) -> (Vector3, Vector3) {
    let driver = Vector3::new(-0.55, 1.25, -0.3);
    if time < 13.0 {
        let angle = smooth(time / 9.0) * 0.65;
        let close = driver + Vector3::new(-1.3 * angle.cos(), 0.05, 1.3 * angle.sin());
        let reveal = smooth((time - 9.0) / 4.0);
        let front = Vector3::new(-radius * 0.35, radius * 0.65, -radius * 1.2);
        let nose = Vector3::new(0.0, radius * 0.1, radius * 20.0);
        return (
            close + (front - close) * reveal,
            driver + (nose - driver) * reveal,
        );
    }
    // The driver shot reveals the nose, then a full orbit precedes the two fixed shots.
    let (yaw, distance, height) = if time < 20.0 {
        let t = (time - 13.0) / 7.0;
        (
            std::f32::consts::PI - std::f32::consts::TAU * t,
            radius * 2.3,
            radius * (0.6 + 0.4 * (t * std::f32::consts::PI).sin()),
        )
    } else if time < 23.0 {
        let t = (time - 20.0) / 3.0;
        (-1.8 + 3.2 * t, radius * 1.7, radius * 0.3)
    } else {
        let t = smooth((time - 23.0) / 4.0);
        (
            1.4 * (1.0 - t),
            radius * (2.0 + 0.6 * t),
            radius * (0.9 - 0.25 * t),
        )
    };
    (
        Vector3::new(yaw.sin() * distance, height, -yaw.cos() * distance),
        Vector3::zero(),
    )
}

impl ShipMovement {
    pub fn intro_frame(
        &mut self,
        time: f32,
        dt: f32,
        destination: Vector3,
        uniforms: &mut Uniforms,
    ) {
        // Start ten times farther away, already under thrust, and ease to the spawn point.
        let remaining = (1.0 - time / 27.0).clamp(0.0, 1.0);
        let distance = if time < 0.0 {
            self.radius * (1800.0 - time * 400.0 / 3.0)
        } else {
            self.radius * 1800.0 * remaining.powi(2)
        };
        self.target = destination - Vector3::new(0.0, 0.0, distance);
        self.speed = self.radius * 3600.0 / 27.0 * remaining;
        self.apply_model(uniforms);
        self.camera.snap(self.orientation);
        self.camera.update(
            &InputState::default(),
            0.0,
            self.target,
            self.orientation,
            false,
            uniforms,
        );
        let (position, target) = camera_pose(time, self.radius);
        let center = self.model_center;
        uniforms.camera_pos = crate::model_point(&uniforms.model_matrix, center + position);
        uniforms.camera_target = crate::model_point(&uniforms.model_matrix, center + target);
        let fixed_shot = if time < 0.0 {
            Some((2000.0, Vector3::new(-8.0, 2.0, 0.0)))
        } else if (20.0..21.5).contains(&time) {
            Some((
                1800.0 * (1.0_f32 - 20.75 / 27.0).powi(2),
                Vector3::new(7.0, 9.0, -2.0),
            ))
        } else if (21.5..23.0).contains(&time) {
            Some((
                1800.0 * (1.0_f32 - 22.25 / 27.0).powi(2),
                Vector3::new(-6.0, 1.0, -8.0),
            ))
        } else {
            None
        };
        if let Some((distance, offset)) = fixed_shot {
            // Fixed world positions let the ship cross the frame instead of following it.
            let crossing = destination - Vector3::new(0.0, 0.0, self.radius * distance);
            uniforms.camera_pos = crossing + offset * self.radius;
            uniforms.camera_target = crossing;
        }
        uniforms.camera_up = Vector3::new(0.0, 1.0, 0.0);
        self.effects.update(
            time < 27.0,
            time < 27.0,
            dt,
            self.world_velocity(),
            self.speed_ratio(),
            (uniforms.camera_pos - self.target).length(),
            self.camera.reference_distance(),
            uniforms,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Genera captura CPU del encuadre final del conductor"]
    fn preview_driver_reveal() {
        let mut car = crate::ship::vehicle::Vehicle::car();
        let mut movement = ShipMovement::new(&car.mesh, 5.0);
        let destination = movement.position();
        let mut u = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::zero(),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        let mut world = crate::scene::build_solar_system();
        movement.intro_frame(12.99, 0.0, destination, &mut u);
        u.light_position = world.primary_light_position();
        let mut fb = Framebuffer::new(800, 600, Color::BLACK);
        let mut renderer = crate::rendering::render::Renderer::default();
        crate::scene::Starfield::new().draw(&mut fb, &u);
        world.draw(&mut renderer, &mut fb, &u);
        car.draw(&mut renderer, &mut fb, &u, 12.99);
        std::fs::create_dir_all("target/previews").unwrap();
        fb.save_png("target/previews/intro-driver-reveal.png")
            .unwrap();
    }

    #[test]
    fn entry_transition_reveals_intro_over_three_seconds() {
        assert_eq!(entry_fade(-6.0), 255);
        assert_eq!(entry_fade(-4.5), 128);
        assert_eq!(entry_fade(-3.0), 0);
        assert_eq!(entry_fade(0.0), 0);
        assert_eq!(entry_fade(27.0), 0);
    }

    #[test]
    fn final_frame_matches_normal_camera_and_spawn_position() {
        let mesh = crate::rendering::load_objects::load_obj("assets/red_car.obj").unwrap();
        let mut movement = ShipMovement::new(&mesh, 5.0);
        let destination = movement.position();
        let mut uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::zero(),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        movement.intro_frame(-3.0, 1.0 / 60.0, destination, &mut uniforms);
        let fixed_camera = uniforms.camera_pos;
        let fixed_aim = uniforms.camera_target;
        let approach = movement.position();
        let screen_start = crate::rendering::projection::project_vertex(
            approach,
            uniforms.camera_pos,
            uniforms.camera_target,
            uniforms.camera_up,
            800.0,
            600.0,
        )
        .unwrap()
        .0;
        movement.intro_frame(-0.001, 1.0 / 60.0, destination, &mut uniforms);
        assert_eq!(uniforms.camera_pos, fixed_camera);
        assert_eq!(uniforms.camera_target, fixed_aim);
        assert!(approach.z < fixed_aim.z);
        assert!(movement.position().z > fixed_aim.z);
        let screen_end = crate::rendering::projection::project_vertex(
            movement.position(),
            uniforms.camera_pos,
            uniforms.camera_target,
            uniforms.camera_up,
            800.0,
            600.0,
        )
        .unwrap()
        .0;
        assert!(screen_end.x < screen_start.x);
        for (start, end) in [(20.0, 21.49), (21.5, 22.99)] {
            movement.intro_frame(start, 1.0 / 60.0, destination, &mut uniforms);
            let camera = uniforms.camera_pos;
            let aim = uniforms.camera_target;
            let position = movement.position();
            movement.intro_frame(end, 1.0 / 60.0, destination, &mut uniforms);
            assert_eq!(uniforms.camera_pos, camera);
            assert_eq!(uniforms.camera_target, aim);
            assert!(movement.position().z > position.z);
        }
        movement.intro_frame(0.0, 1.0 / 60.0, destination, &mut uniforms);
        assert!((movement.position() - destination).length() > movement.radius * 1700.0);
        assert!(movement.is_moving());
        let start = movement.position();
        movement.intro_frame(13.0, 1.0 / 60.0, destination, &mut uniforms);
        assert!(movement.position().z > start.z);
        assert!(movement.is_moving());
        movement.intro_frame(27.0, 1.0 / 60.0, destination, &mut uniforms);
        let final_camera = uniforms.camera_pos;
        let final_target = uniforms.camera_target;
        movement.camera.update(
            &InputState::default(),
            1.0 / 60.0,
            destination,
            movement.orientation,
            false,
            &mut uniforms,
        );
        assert!((uniforms.camera_pos - final_camera).length() < 0.0001);
        assert!((uniforms.camera_target - final_target).length() < 0.0001);
        assert_eq!(movement.position(), destination);
        assert_eq!(movement.speed(), 0.0);
    }

    #[test]
    fn camera_stays_near_driver_then_returns_smoothly_to_follow_view() {
        let radius = 4.0;
        let (start, _) = camera_pose(0.0, radius);
        let (close, _) = camera_pose(8.999, radius);
        assert!((close - start).length() < 2.0);
        assert!(close.z > start.z);
        assert!(close.x > start.x);
        for frame in 0..90 {
            let (position, target) = camera_pose(frame as f32 / 10.0, radius);
            let offset = position - target;
            assert!(offset.x < -1.0);
            assert!(offset.z >= 0.0 && offset.z < 0.8);
            assert!(offset.length() < 1.5);
        }
        let (front, aim) = camera_pose(12.999, radius);
        assert!(front.z < -radius);
        assert!(front.y > radius * 0.6);
        assert!(aim.z > radius * 19.0);
        assert!((aim - front).z > 0.0);
        let (orbit_front, _) = camera_pose(13.0, radius);
        let (orbit_back, _) = camera_pose(16.5, radius);
        assert!(orbit_front.z > radius && orbit_back.z < -radius);
        for boundary in [27.0] {
            let (before, _) = camera_pose(boundary - 0.001, radius);
            let (after, _) = camera_pose(boundary + 0.001, radius);
            assert!((after - before).length() < 0.01);
        }
        for cut in [13.0, 20.0, 23.0] {
            let (before, _) = camera_pose(cut - 0.001, radius);
            let (after, _) = camera_pose(cut, radius);
            assert!((after - before).length() > radius * 0.2);
        }
        let (position, target) = camera_pose(27.0, radius);
        assert_eq!(target, Vector3::zero());
        assert_eq!(position, Vector3::new(0.0, radius * 0.65, -radius * 2.6));
    }
}
