use crate::rendering::{framebuffer::Framebuffer, projection::project_vertex, uniforms::Uniforms};
use raylib::prelude::*;
use std::f32::consts::TAU;

struct Spark {
    position: Vector3,
    velocity: Vector3,
    age: f32,
    life: f32,
    size: f32,
}
#[derive(Default)]
pub struct ImpactEffects {
    sparks: Vec<Spark>,
    pub shake: f32,
    pending: bool,
}
impl ImpactEffects {
    pub fn update(&mut self, dt: f32) {
        self.shake = (self.shake - dt * 2.8).max(0.0);
        for spark in &mut self.sparks {
            spark.age += dt;
            spark.position += spark.velocity * dt;
            spark.velocity *= (-2.0 * dt).exp();
        }
        self.sparks.retain(|s| s.age < s.life);
    }
    pub fn burst(&mut self, position: Vector3, normal: Vector3, radius: f32) {
        self.pending = true;
        self.shake = 1.0;
        for i in 0..80 {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 80.0;
            let angle = i as f32 * TAU * 0.618034;
            let r = (1.0 - y * y).sqrt();
            let mut direction = Vector3::new(r * angle.cos(), y, r * angle.sin());
            if direction.dot(normal) < 0.0 {
                direction -= normal * (2.0 * direction.dot(normal));
            }
            self.sparks.push(Spark {
                position,
                velocity: (direction + normal * 0.4) * radius * (10.0 + (i % 7) as f32 * 2.0),
                age: 0.0,
                life: 0.35 + (i % 9) as f32 * 0.08,
                size: radius * (0.12 + (i % 4) as f32 * 0.05),
            });
        }
        if self.sparks.len() > 320 {
            self.sparks.drain(..self.sparks.len() - 320);
        }
    }
    pub fn take_event(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
    pub fn clear(&mut self) {
        self.sparks.clear();
        self.shake = 0.0;
        self.pending = false;
    }
    pub fn draw(&self, fb: &mut Framebuffer, u: &Uniforms) {
        for spark in &self.sparks {
            if let Some((screen, depth)) = project_vertex(
                spark.position,
                u.camera_pos,
                u.camera_target,
                u.camera_up,
                fb.width as f32,
                fb.height as f32,
            ) {
                let t = spark.age / spark.life;
                let radius = (spark.size * (1.0 + t * 3.0) / depth * 520.0).clamp(2.0, 65.0);
                let color = Vector3::new(1.0, 0.20 + (1.0 - t) * 0.65, 0.03 + (1.0 - t) * 0.18);
                super::alien::glow(fb, screen, depth, radius * 2.0, color, (1.0 - t) * 0.6);
                super::alien::glow(
                    fb,
                    screen,
                    depth,
                    radius * 0.3,
                    Vector3::new(1.0, 0.95, 0.75),
                    (1.0 - t) * 1.3,
                );
            }
        }
    }
}
