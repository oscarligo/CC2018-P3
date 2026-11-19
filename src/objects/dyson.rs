//! Two independent, counter-rotating mirror ribbons. Geometry and metal shading run on CPU.
use crate::{
    rendering::{
        framebuffer::Framebuffer, load_objects::Mesh, render::Renderer, uniforms::Uniforms,
        vertex::Vertex,
    },
    scene::world::{World, WorldObject, WorldTransform, WorldUpdate},
};
use raylib::prelude::{Matrix, Vector3};
use std::f32::consts::TAU;

const SEGMENTS: usize = 96;
const CROSS_SECTIONS: usize = 12;
const RADIAL_RATIO: f32 = 0.047;
const WIDTH_RATIO: f32 = 0.105;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Finish {
    Mirror,
    Armor,
    Edge,
    Panel,
}

pub struct DysonRing {
    transform: WorldTransform,
    mesh: Mesh,
    finishes: Vec<Finish>,
    colors: Vec<Vector3>,
    radius: f32,
    thickness: f32,
    half_width: f32,
    index: usize,
    time: f32,
}

/// Leave a conservative gap to the nearest planet's whole orbital envelope.
fn dimensions(sun_radius: f32, planet_clearance: f32) -> Option<[f32; 2]> {
    let outer = (sun_radius * 1.46).min(planet_clearance - sun_radius * 0.10 - 5.0);
    let inner = (sun_radius * 1.29).min(outer - sun_radius * 0.12);
    (inner > sun_radius * 1.20).then_some([inner, outer])
}

pub fn register(world: &mut World) {
    let Some((sun, sun_radius)) = world.primary_light_bounds() else {
        return;
    };
    let clearance = world
        .travel_targets()
        .iter()
        .map(|t| (t.position - sun).length() - t.radius)
        .fold(f32::INFINITY, f32::min);
    let Some(radii) = dimensions(sun_radius, clearance) else {
        return;
    };
    for (index, radius) in radii.into_iter().enumerate() {
        world.add(DysonRing::new(
            sun,
            radius,
            sun_radius * RADIAL_RATIO,
            sun_radius * WIDTH_RATIO,
            index,
        ));
    }
}

pub(crate) fn interior_view(sun: Vector3, sun_radius: f32, time: f32) -> Uniforms {
    let radius = sun_radius * 1.46;
    let mut ring = DysonRing::new(
        sun,
        radius,
        sun_radius * RADIAL_RATIO,
        sun_radius * WIDTH_RATIO,
        1,
    );
    ring.time = time;
    let rotation = ring.orientation();
    let radial = rotate(&rotation, Vector3::new(1.0, 0.0, 0.0));
    let up = rotate(&rotation, Vector3::new(0.0, 1.0, 0.0));
    Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: sun + radial * (radius - sun_radius * 0.27) + up * sun_radius * 0.07,
        camera_target: sun + radial * radius,
        camera_up: up,
        light_position: Some(sun),
    }
}

impl DysonRing {
    fn new(center: Vector3, radius: f32, thickness: f32, half_width: f32, index: usize) -> Self {
        let (mesh, finishes) = geometry(radius, thickness, half_width);
        let colors = vec![Vector3::zero(); mesh.vertices.len()];
        Self {
            transform: WorldTransform::new(center, Vector3::new(1., 1., 1.)),
            mesh,
            finishes,
            colors,
            radius,
            thickness,
            half_width,
            index,
            time: 0.0,
        }
    }
    fn orientation(&self) -> Matrix {
        let (sign, tilt, phase) = if self.index == 0 {
            (1.0, 0.74, 0.28)
        } else {
            (-1.0, 1.24, -0.62)
        };
        // A rigid circular hoop, with both axes passing exactly through the sun.
        // Cells complete a revolution in 2.6 seconds; the gimbal turns in 8 seconds.
        Matrix::rotate_y(sign * self.time * 2.4)
            * Matrix::rotate_x(tilt)
            * Matrix::rotate_z(phase)
            * Matrix::rotate_y(sign * self.time * 0.78)
    }
    fn extent(&self) -> f32 {
        ((self.radius + self.thickness).powi(2) + self.half_width.powi(2)).sqrt() + 0.7
    }
}

impl WorldObject for DysonRing {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }
    fn bounding_radius(&self) -> Option<f32> {
        Some(self.extent())
    }
    fn update(&mut self, update: &WorldUpdate) {
        self.time = (self.time + update.frame_time).rem_euclid(10_000.0);
    }
    fn draw(&mut self, renderer: &mut Renderer, fb: &mut Framebuffer, u: &Uniforms) {
        let rotation = self.orientation();
        let center = self.transform.position;
        let model = rotation * Matrix::translate(center.x, center.y, center.z);
        for (i, v) in self.mesh.vertices.iter().enumerate() {
            let p = rotate(&rotation, v.position);
            let n = rotate(&rotation, v.normal).normalize();
            let view = (u.camera_pos - center - p).normalize();
            let light = (-p).normalize();
            let diffuse = n.dot(light).max(0.0);
            let half = (light + view).normalize();
            let glint = n.dot(half).max(0.0).powf(48.0);
            let grazing = (1.0 - n.dot(view).abs()).powf(3.0);
            let panel = v.color.x;
            self.colors[i] = match self.finishes[i] {
                Finish::Mirror => {
                    // Analytic reflection of the broad solar disc and a cool environment.
                    let reflection = n * (2.0 * n.dot(view)) - view;
                    let solar = reflection.dot(light).max(0.0).powf(5.0);
                    let silver = Vector3::new(0.76, 0.84, 0.93) * (0.78 + 0.30 * diffuse) * panel;
                    silver
                        + Vector3::new(0.95, 0.91, 0.79) * (solar * 0.55 + glint * 0.85)
                        + Vector3::new(0.25, 0.40, 0.52) * grazing * 0.20
                }
                Finish::Armor => {
                    Vector3::new(0.018, 0.025, 0.037) * (0.80 + 0.20 * (n.y * 0.5 + 0.5))
                }
                Finish::Edge => Vector3::new(0.12, 0.15, 0.19) * (0.62 + panel * 0.18),
                Finish::Panel => {
                    Vector3::new(0.075, 0.095, 0.13) * (0.65 + 0.35 * (n.y * 0.5 + 0.5)) * panel
                }
            };
        }
        renderer.render_colored(
            fb,
            &Uniforms {
                model_matrix: model,
                ..*u
            },
            &self.mesh,
            &self.colors,
        );
    }
}

fn rotate(m: &Matrix, v: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * v.x + m.m4 * v.y + m.m8 * v.z,
        m.m1 * v.x + m.m5 * v.y + m.m9 * v.z,
        m.m2 * v.x + m.m6 * v.y + m.m10 * v.z,
    )
}

fn geometry(radius: f32, thickness: f32, half_width: f32) -> (Mesh, Vec<Finish>) {
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        materials: Vec::new(),
    };
    let mut finishes = Vec::new();
    let at = |a: f32, b: f32, lift: f32| {
        let radial = Vector3::new(a.cos(), 0.0, a.sin());
        let normal = (radial * (b.cos() / thickness)
            + Vector3::new(0.0, b.sin() / half_width, 0.0))
        .normalize();
        (
            radial * (radius + thickness * b.cos())
                + Vector3::new(0.0, half_width * b.sin(), 0.0)
                + normal * lift,
            normal,
        )
    };
    for segment in 0..SEGMENTS {
        for cross in 0..CROSS_SECTIONS {
            let a = TAU * segment as f32 / SEGMENTS as f32;
            let b = TAU * cross as f32 / CROSS_SECTIONS as f32;
            let da = TAU / SEGMENTS as f32;
            let db = TAU / CROSS_SECTIONS as f32;
            // A complete opaque structural shell beneath matte outer panels and inner mirrors.
            for inset in [false, true] {
                let gap = if inset { 0.085 } else { 0.0 };
                let lift = if inset { 0.65 } else { 0.0 };
                let face = [
                    at(a + da * gap, b + db * gap, lift),
                    at(a + da * (1.0 - gap), b + db * gap, lift),
                    at(a + da * (1.0 - gap), b + db * (1.0 - gap), lift),
                    at(a + da * gap, b + db * (1.0 - gap), lift),
                ];
                let facing = (b + db * 0.5).cos();
                let finish = if !inset {
                    Finish::Armor
                } else if facing < -0.25 {
                    Finish::Mirror
                } else if facing.abs() < 0.26 {
                    Finish::Edge
                } else {
                    Finish::Panel
                };
                // Asymmetric dark panel blocks make actual rotation easy to track.
                let panel = if segment % 24 < 3 {
                    0.25
                } else {
                    0.86 + (segment % 5) as f32 * 0.035
                };
                let start = mesh.vertices.len() as u32;
                for (p, n) in face {
                    let mut v = Vertex::new(p);
                    v.normal = n;
                    v.color = Vector3::new(panel, panel, panel);
                    mesh.vertices.push(v);
                    finishes.push(finish);
                }
                let normal = face[0].1 + face[1].1 + face[2].1;
                let winding = (face[1].0 - face[0].0)
                    .cross(face[2].0 - face[0].0)
                    .dot(normal);
                if winding >= 0.0 {
                    mesh.indices
                        .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
                } else {
                    mesh.indices
                        .extend([start, start + 2, start + 1, start, start + 3, start + 2]);
                }
            }
        }
    }
    (mesh, finishes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ribbons_clear_the_sun_and_the_nearest_planet_throughout_rotation() {
        let mut world = World::new();
        crate::objects::register_all(&mut world);
        crate::scene::planets::register_all(&mut world);
        world.update(&WorldUpdate { frame_time: 0.0 });
        let (sun, radius) = world.primary_light_bounds().unwrap();
        let clearance = world
            .travel_targets()
            .iter()
            .map(|t| (t.position - sun).length() - t.radius)
            .fold(f32::INFINITY, f32::min);
        let radii = dimensions(radius, clearance).unwrap();
        assert!(radii[0] > radius * 1.18 + 10.0);
        for (index, r) in radii.into_iter().enumerate() {
            let mut ring =
                DysonRing::new(sun, r, radius * RADIAL_RATIO, radius * WIDTH_RATIO, index);
            assert!(ring.extent() + 15.0 < clearance);
            for time in [0., 5., 17., 39., 83.] {
                ring.time = time;
                for v in &ring.mesh.vertices {
                    let p = rotate(&ring.orientation(), v.position);
                    assert!((p.length() - v.position.length()).abs() < 0.01);
                    assert!(p.length() < clearance - 15.0);
                }
            }
            assert_eq!(ring.mesh.indices.len() / 3, SEGMENTS * CROSS_SECTIONS * 4);
            assert!(ring.finishes.contains(&Finish::Mirror));
            assert!(ring.finishes.contains(&Finish::Armor));
        }
    }
    #[test]
    fn rings_have_different_counter_rotating_motion() {
        let mut a = DysonRing::new(Vector3::zero(), 470., 3., 10., 0);
        let mut b = DysonRing::new(Vector3::zero(), 530., 3., 10., 1);
        let p = Vector3::new(470., 0., 0.);
        let a0 = rotate(&a.orientation(), p);
        let b0 = rotate(&b.orientation(), p);
        a.time = 0.02;
        b.time = 0.02;
        let da = rotate(&a.orientation(), p) - a0;
        let db = rotate(&b.orientation(), p) - b0;
        assert!(da.length() > 0.1 && db.length() > 0.1);
        assert!(da.z * db.z < 0.0);
    }
}
