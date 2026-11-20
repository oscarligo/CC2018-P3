use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::projection::project_vertex;
use crate::rendering::uniforms::Uniforms;
use raylib::prelude::Vector3;
use std::f32::consts::TAU;

const STAR_COUNT: usize = 600;
const NEAR_STAR_DISTANCE: f32 = 28_000.0;
const MID_STAR_DISTANCE: f32 = 55_000.0;
const FAR_STAR_DISTANCE: f32 = 100_000.0;

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
        for index in 0..STAR_COUNT {
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
            let distance = match index % 20 {
                0..=2 => NEAR_STAR_DISTANCE,
                3..=8 => MID_STAR_DISTANCE,
                _ => FAR_STAR_DISTANCE,
            };
            stars.push(Star {
                position: direction * distance,
                color,
                radius: i32::from(
                    brightness > 0.94 || (distance == NEAR_STAR_DISTANCE && brightness > 0.78),
                ),
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
    fn catalog_is_deterministic_and_distributed_across_three_depths() {
        let first = Starfield::new();
        let second = Starfield::new();
        assert_eq!(first.stars.len(), STAR_COUNT);
        let mut layers = [0; 3];
        for (a, b) in first.stars.iter().zip(second.stars.iter()) {
            assert_eq!(a.position, b.position);
            let distance = a.position.length();
            let layer = [NEAR_STAR_DISTANCE, MID_STAR_DISTANCE, FAR_STAR_DISTANCE]
                .iter()
                .position(|expected| (distance - expected).abs() < 0.1)
                .expect("cada estrella debe pertenecer a una capa");
            layers[layer] += 1;
        }
        assert_eq!(layers, [90, 180, 330]);
    }

    #[test]
    fn camera_translation_moves_near_stars_more_than_far_stars() {
        use crate::rendering::projection::project_vertex;

        let screen_x = |distance| {
            project_vertex(
                Vector3::new(0.0, 0.0, distance),
                Vector3::new(1_000.0, 0.0, 0.0),
                Vector3::new(1_000.0, 0.0, 1.0),
                Vector3::new(0.0, 1.0, 0.0),
                800.0,
                600.0,
            )
            .unwrap()
            .0
            .x
        };
        let center = 400.0;
        let near_shift = (screen_x(NEAR_STAR_DISTANCE) - center).abs();
        let far_shift = (screen_x(FAR_STAR_DISTANCE) - center).abs();
        assert!(near_shift > far_shift * 3.0);
    }
}
