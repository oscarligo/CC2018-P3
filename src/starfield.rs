use crate::framebuffer::Framebuffer;
use crate::projection::project_vertex;
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;
use std::f32::consts::TAU;

const STAR_COUNT: usize = 600;
// shortcut: el mundo jugable debe permanecer muy dentro de esta esfera, recentrarla si supera esa escala.
const STAR_DISTANCE: f32 = 100_000.0;

pub struct Starfield {
    stars: Vec<Star>,
}

struct Star {
    position: Vector3,
    color: Vector3,
    radius: i32,
}

impl Starfield {
    pub fn new() -> Self {
        let mut random = Random::new(0x51a7_f13d);
        let mut stars = Vec::with_capacity(STAR_COUNT);
        for _ in 0..STAR_COUNT {
            let z = random.next_f32() * 2.0 - 1.0;
            let theta = random.next_f32() * TAU;
            let radial = (1.0 - z * z).sqrt();
            let direction = Vector3::new(radial * theta.cos(), z, radial * theta.sin());
            let brightness = 0.45 + random.next_f32() * 0.55;
            let temperature = random.next_f32();
            let color = if temperature < 0.20 {
                Vector3::new(0.68, 0.78, 1.0)
            } else if temperature > 0.82 {
                Vector3::new(1.0, 0.76, 0.52)
            } else {
                Vector3::new(0.94, 0.95, 1.0)
            } * brightness;
            stars.push(Star {
                position: direction * STAR_DISTANCE,
                color,
                radius: i32::from(brightness > 0.94),
            });
        }
        Self { stars }
    }

    pub fn draw(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        for star in &self.stars {
            let Some((position, depth)) = project_vertex(
                star.position,
                uniforms.camera_pos,
                uniforms.camera_target,
                uniforms.camera_up,
                framebuffer.width as f32,
                framebuffer.height as f32,
            ) else {
                continue;
            };
            let x = position.x as i32;
            let y = position.y as i32;
            for offset_y in -star.radius..=star.radius {
                for offset_x in -star.radius..=star.radius {
                    framebuffer.point(x + offset_x, y + offset_y, depth, star.color);
                }
            }
        }
    }
}

struct Random {
    state: u32,
}

impl Random {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.state >> 8) as f32 / 0x00ff_ffff as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_deterministic_and_on_the_distant_sphere() {
        let first = Starfield::new();
        let second = Starfield::new();
        assert_eq!(first.stars.len(), STAR_COUNT);
        for (a, b) in first.stars.iter().zip(second.stars.iter()) {
            assert_eq!(a.position, b.position);
            assert!((a.position.length() - STAR_DISTANCE).abs() < 0.1);
        }
    }
}
