//! Authored geometry, shaded and rasterized entirely on the CPU.
use crate::rendering::{
    framebuffer::Framebuffer, load_objects::Mesh, render::Renderer, uniforms::Uniforms,
    vertex::Vertex,
};
use raylib::prelude::*;
use std::f32::consts::{PI, TAU};
#[path = "destroyer_model.rs"]
mod destroyer_model;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleKind {
    Car,
    Saucer,
    Destroyer,
}

pub struct Vehicle {
    pub kind: VehicleKind,
    pub mesh: Mesh,
    glass: Option<Mesh>,
    pub radius: f32,
    pub muzzle: Option<Vector3>,
    center: Vector3,
    colors: Vec<Vector3>,
    glass_colors: Vec<Vector3>,
    glass_alphas: Vec<f32>,
    bay: Option<destroyer_model::WeaponBay>,
    pub weapon_charge: f32,
    pub weapon_open: f32,
    pub bomb_visible: bool,
}

pub fn bounds(mesh: &Mesh) -> (Vector3, f32) {
    let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = -min;
    for v in &mesh.vertices {
        min.x = min.x.min(v.position.x);
        min.y = min.y.min(v.position.y);
        min.z = min.z.min(v.position.z);
        max.x = max.x.max(v.position.x);
        max.y = max.y.max(v.position.y);
        max.z = max.z.max(v.position.z);
    }
    ((min + max) * 0.5, (max - min).length() * 0.5)
}

impl Vehicle {
    pub fn car() -> Self {
        let mesh = crate::rendering::load_objects::load_obj("assets/red_car.obj")
            .expect("No se pudo cargar assets/red_car.obj");
        let (center, radius) = bounds(&mesh);
        Self {
            kind: VehicleKind::Car,
            mesh,
            glass: None,
            radius,
            muzzle: None,
            center,
            colors: Vec::new(),
            glass_colors: Vec::new(),
            glass_alphas: Vec::new(),
            bay: None,
            weapon_charge: 0.0,
            weapon_open: 0.0,
            bomb_visible: true,
        }
    }

    pub fn saucer(reference_radius: f32) -> Self {
        let (mut mesh, mut glass) = saucer_geometry();
        // Match the car's dimensions, not its polygon count, for comparable framing.
        let (_, radius) = bounds(&mesh);
        let scale = reference_radius / radius;
        for v in mesh.vertices.iter_mut().chain(glass.vertices.iter_mut()) {
            v.position *= scale;
        }
        let (center, radius) = bounds(&mesh);
        let colors = vec![Vector3::zero(); mesh.vertices.len()];
        let glass_colors = vec![Vector3::zero(); glass.vertices.len()];
        let glass_alphas = vec![0.0; glass.vertices.len()];
        Self {
            kind: VehicleKind::Saucer,
            mesh,
            glass: Some(glass),
            radius,
            muzzle: Some(Vector3::new(1.65, 0.70, -0.90) * scale),
            center,
            colors,
            glass_colors,
            glass_alphas,
            bay: None,
            weapon_charge: 0.0,
            weapon_open: 0.0,
            bomb_visible: true,
        }
    }

    pub fn destroyer(reference_radius: f32) -> Self {
        let mut mesh = destroyer_model::geometry();
        let (_, radius) = bounds(&mesh);
        let scale = reference_radius * 1.35 / radius;
        for v in &mut mesh.vertices {
            v.position *= scale;
        }
        let (center, radius) = bounds(&mesh);
        let colors = vec![Vector3::zero(); mesh.vertices.len()];
        Self {
            kind: VehicleKind::Destroyer,
            mesh,
            glass: None,
            radius,
            muzzle: Some(Vector3::new(0.0, -1.06, 1.0) * scale),
            center,
            colors,
            glass_colors: vec![],
            glass_alphas: vec![],
            bay: Some(destroyer_model::WeaponBay::new(scale)),
            weapon_charge: 0.0,
            weapon_open: 0.0,
            bomb_visible: true,
        }
    }

    pub fn preview_uniforms(&self, angle: f32) -> Uniforms {
        if self.kind == VehicleKind::Destroyer {
            return Uniforms {
                model_matrix: Matrix::translate(-self.center.x, -self.center.y, -self.center.z)
                    * Matrix::rotate_y(angle),
                camera_pos: Vector3::new(self.radius * 1.60, self.radius * 1.1, self.radius * 1.9),
                camera_target: Vector3::new(0.0, self.radius * 0.02, 0.0),
                camera_up: Vector3::new(0.0, 1.0, 0.0),
                light_position: Some(Vector3::new(
                    self.radius * 3.0,
                    self.radius * 6.0,
                    self.radius * 2.0,
                )),
            };
        }
        Uniforms {
            model_matrix: Matrix::translate(-self.center.x, -self.center.y, -self.center.z)
                * Matrix::rotate_y(angle)
                * Matrix::translate(0.0, self.radius * 0.20, 0.0),
            camera_pos: Vector3::new(0.0, self.radius * 0.80, -self.radius * 1.90),
            camera_target: Vector3::zero(),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: Some(Vector3::new(
                -self.radius * 3.0,
                self.radius * 5.0,
                -self.radius * 4.0,
            )),
        }
    }

    pub fn draw(&mut self, renderer: &mut Renderer, fb: &mut Framebuffer, u: &Uniforms, time: f32) {
        if self.kind == VehicleKind::Car {
            renderer.render_lit(fb, u, &self.mesh);
            return;
        }
        let eye = u.camera_pos;
        let light = Vector3::new(-0.45, 0.8, -0.6).normalize();
        for (i, v) in self.mesh.vertices.iter().enumerate() {
            let n = direction(&u.model_matrix, v.normal).normalize();
            let p = point(&u.model_matrix, v.position);
            let view = (eye - p).normalize();
            let diffuse = n.dot(light).max(0.0);
            let half = (view + light).normalize();
            let spec = n.dot(half).max(0.0).powf(55.0);
            let rim = (1.0 - n.dot(view).abs()).powf(3.0);
            let emissive = v.alpha > 1.5;
            self.colors[i] = if emissive {
                v.color
                    * if self.kind == VehicleKind::Destroyer && v.color.x > v.color.z * 1.5 {
                        0.15 + self.weapon_charge
                            * (0.85 + 0.20 * (time * (5.0 + 15.0 * self.weapon_charge)).sin())
                    } else {
                        0.88 + 0.12 * (time * 2.0).sin()
                    }
            } else {
                v.color * (0.36 + diffuse * 0.72)
                    + Vector3::new(0.65, 0.83, 0.95) * (spec * 0.85 + rim * 0.12)
            };
        }
        renderer.render_colored(fb, u, &self.mesh, &self.colors);
        if let Some(bay) = &mut self.bay {
            bay.draw(
                renderer,
                fb,
                u,
                self.weapon_open,
                self.weapon_charge,
                self.bomb_visible,
                time,
            );
        }
        if let Some(glass) = &self.glass {
            for (i, v) in glass.vertices.iter().enumerate() {
                let n = direction(&u.model_matrix, v.normal).normalize();
                let view = (eye - point(&u.model_matrix, v.position)).normalize();
                let fresnel = (1.0 - n.dot(view).abs()).powf(3.0);
                let highlight = n.dot((view + light).normalize()).max(0.0).powf(90.0);
                self.glass_alphas[i] = 0.035 + fresnel * 0.27 + highlight * 0.22;
                self.glass_colors[i] =
                    Vector3::new(0.16, 0.65, 0.58) + Vector3::new(0.65, 0.32, 0.38) * highlight;
            }
            renderer.render_colored_transparent(
                fb,
                u,
                glass,
                &self.glass_colors,
                &self.glass_alphas,
            );
        }
    }
}

fn direction(m: &Matrix, v: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * v.x + m.m4 * v.y + m.m8 * v.z,
        m.m1 * v.x + m.m5 * v.y + m.m9 * v.z,
        m.m2 * v.x + m.m6 * v.y + m.m10 * v.z,
    )
}
fn point(m: &Matrix, v: Vector3) -> Vector3 {
    direction(m, v) + Vector3::new(m.m12, m.m13, m.m14)
}

struct Sculpt {
    mesh: Mesh,
}
impl Sculpt {
    fn new() -> Self {
        Self {
            mesh: Mesh {
                vertices: Vec::new(),
                indices: Vec::new(),
                materials: Vec::new(),
            },
        }
    }
    fn triangle(
        &mut self,
        positions: [Vector3; 3],
        normals: [Vector3; 3],
        color: Vector3,
        glow: bool,
    ) {
        let start = self.mesh.vertices.len() as u32;
        let face = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
        let normal = normals[0] + normals[1] + normals[2];
        let order = if face.dot(normal) >= 0.0 {
            [0, 1, 2]
        } else {
            [0, 2, 1]
        };
        for i in 0..3 {
            let mut v = Vertex::new(positions[i]);
            v.normal = normals[i];
            v.color = color;
            v.alpha = if glow { 2.0 } else { 1.0 };
            self.mesh.vertices.push(v);
        }
        self.mesh.indices.extend(order.map(|i| start + i));
    }
    fn ellipsoid(&mut self, center: Vector3, size: Vector3, color: Vector3, glow: bool) {
        let small = size.x.max(size.y).max(size.z) < 0.1;
        self.cap(
            center,
            size,
            color,
            glow,
            PI,
            if small { 12 } else { 28 },
            if small { 8 } else { 16 },
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn cap(
        &mut self,
        center: Vector3,
        size: Vector3,
        color: Vector3,
        glow: bool,
        end: f32,
        slices: usize,
        rows: usize,
    ) {
        let at = |row: usize, col: usize| {
            let a = end * row as f32 / rows as f32;
            let b = TAU * col as f32 / slices as f32;
            let d = Vector3::new(a.sin() * b.cos(), a.cos(), a.sin() * b.sin());
            (
                center + Vector3::new(d.x * size.x, d.y * size.y, d.z * size.z),
                Vector3::new(d.x / size.x, d.y / size.y, d.z / size.z).normalize(),
            )
        };
        for row in 0..rows {
            for col in 0..slices {
                let (a, na) = at(row, col);
                let (b, nb) = at(row + 1, col);
                let (c, nc) = at(row + 1, col + 1);
                let (d, nd) = at(row, col + 1);
                self.triangle([a, b, c], [na, nb, nc], color, glow);
                self.triangle([a, c, d], [na, nc, nd], color, glow);
            }
        }
    }
    fn ring(&mut self, radius: f32, y: f32, tube: f32, color: Vector3, glow: bool) {
        for i in 0..96 {
            for j in 0..8 {
                let at = |i: usize, j: usize| {
                    let a = TAU * i as f32 / 96.0;
                    let b = TAU * j as f32 / 8.0;
                    let n = Vector3::new(a.cos() * b.cos(), b.sin(), a.sin() * b.cos());
                    (
                        Vector3::new(radius * a.cos(), y, radius * a.sin()) + n * tube,
                        n,
                    )
                };
                let (a, na) = at(i, j);
                let (b, nb) = at(i + 1, j);
                let (c, nc) = at(i + 1, j + 1);
                let (d, nd) = at(i, j + 1);
                self.triangle([a, b, c], [na, nb, nc], color, glow);
                self.triangle([a, c, d], [na, nc, nd], color, glow);
            }
        }
    }
    fn rod(&mut self, a: Vector3, b: Vector3, radius: f32, color: Vector3, glow: bool) {
        let axis = (b - a).normalize();
        let other = if axis.y.abs() < 0.9 {
            Vector3::new(0., 1., 0.)
        } else {
            Vector3::new(1., 0., 0.)
        };
        let right = axis.cross(other).normalize();
        let up = axis.cross(right);
        for i in 0..10 {
            let n =
                |j: usize| right * (TAU * j as f32 / 10.).cos() + up * (TAU * j as f32 / 10.).sin();
            let x = n(i);
            let y = n(i + 1);
            self.triangle(
                [a + x * radius, b + x * radius, b + y * radius],
                [x, x, y],
                color,
                glow,
            );
            self.triangle(
                [a + x * radius, b + y * radius, a + y * radius],
                [x, y, y],
                color,
                glow,
            );
        }
    }
}

fn saucer_geometry() -> (Mesh, Mesh) {
    let mut s = Sculpt::new();
    let metal = Vector3::new(0.12, 0.17, 0.24);
    let trim = Vector3::new(0.42, 0.53, 0.57);
    let black = Vector3::new(0.025, 0.04, 0.055);
    let green = Vector3::new(0.22, 1.0, 0.37);
    // Smooth machined lenticular hull. Independently authored concentric panels.
    s.ellipsoid(
        Vector3::new(0., -0.08, 0.),
        Vector3::new(2.48, 0.49, 2.48),
        metal,
        false,
    );
    s.ellipsoid(
        Vector3::new(0., 0.13, 0.),
        Vector3::new(1.48, 0.40, 1.48),
        black,
        false,
    );
    s.ring(2.43, -0.08, 0.052, trim, false);
    s.ring(2.44, -0.015, 0.018, green, true);
    s.ring(1.13, 0.43, 0.075, trim, false);
    s.ring(1.15, 0.48, 0.034, green, true);
    s.ring(1.76, 0.265, 0.014, trim, false);
    s.ring(0.66, -0.50, 0.075, green, true);
    for i in 0..16 {
        let a = TAU * i as f32 / 16.;
        let p = Vector3::new(a.cos() * 2.13, 0.21, a.sin() * 2.13);
        s.ellipsoid(p, Vector3::new(0.075, 0.045, 0.075), green, true);
        // Radial engraved seams and fasteners.
        s.rod(
            Vector3::new(a.cos() * 1.50, 0.335, a.sin() * 1.50),
            Vector3::new(a.cos() * 1.99, 0.226, a.sin() * 1.99),
            0.009,
            black,
            false,
        );
        s.ellipsoid(
            Vector3::new(a.cos() * 2.30, 0.102, a.sin() * 2.30),
            Vector3::new(0.025, 0.018, 0.025),
            trim,
            false,
        );
    }
    // Cockpit seat, console and pilot, facing the chase camera (-Z).
    s.ellipsoid(
        Vector3::new(0., 0.55, 0.20),
        Vector3::new(0.52, 0.17, 0.48),
        black,
        false,
    );
    let fur = Vector3::new(0.48, 0.78, 0.19);
    let pale = Vector3::new(0.65, 0.88, 0.32);
    s.ellipsoid(
        Vector3::new(0., 0.81, 0.06),
        Vector3::new(0.35, 0.43, 0.28),
        fur,
        false,
    );
    s.ellipsoid(
        Vector3::new(0., 1.32, -0.12),
        Vector3::new(0.55, 0.43, 0.37),
        fur,
        false,
    );
    for side in [-1.0, 1.0] {
        // Triangular feline ears, inset soft pink inner ears.
        let a = Vector3::new(side * 0.27, 1.52, -0.20);
        let b = Vector3::new(side * 0.56, 1.55, -0.13);
        let c = Vector3::new(side * 0.43, 1.99, -0.06);
        let back = Vector3::new(side * 0.43, 1.56, 0.17);
        for face in [[a, b, c], [b, back, c], [back, a, c]] {
            let mut n = (face[1] - face[0]).cross(face[2] - face[0]).normalize();
            let middle = (a + b + c + back) * 0.25;
            if n.dot((face[0] + face[1] + face[2]) * (1.0 / 3.0) - middle) < 0.0 {
                n = -n;
            }
            s.triangle(face, [n; 3], fur, false);
        }
        let inner = [
            Vector3::new(side * 0.32, 1.58, -0.205),
            Vector3::new(side * 0.50, 1.60, -0.14),
            Vector3::new(side * 0.43, 1.88, -0.085),
        ];
        s.triangle(
            inner,
            [Vector3::new(0., 0., -1.); 3],
            Vector3::new(0.68, 0.49, 0.40),
            false,
        );
        // Glossy alien eyes with twin catchlights.
        s.ellipsoid(
            Vector3::new(side * 0.235, 1.38, -0.438),
            Vector3::new(0.167, 0.105, 0.055),
            Vector3::new(0.008, 0.025, 0.016),
            false,
        );
        s.ellipsoid(
            Vector3::new(side * 0.21, 1.415, -0.485),
            Vector3::new(0.032, 0.024, 0.012),
            Vector3::new(0.85, 1., 0.88),
            true,
        );
        s.ellipsoid(
            Vector3::new(side * 0.27, 1.365, -0.49),
            Vector3::new(0.012, 0.010, 0.008),
            green,
            true,
        );
        s.ellipsoid(
            Vector3::new(side * 0.12, 1.18, -0.43),
            Vector3::new(0.145, 0.105, 0.08),
            pale,
            false,
        );
        s.ellipsoid(
            Vector3::new(side * 0.25, 0.56, -0.25),
            Vector3::new(0.13, 0.10, 0.19),
            pale,
            false,
        );
        // Antennae end within the glass dome.
        let root = Vector3::new(side * 0.19, 1.67, 0.01);
        let tip = Vector3::new(side * 0.25, 2.09, 0.02);
        s.rod(root, tip, 0.023, pale, false);
        s.ellipsoid(tip, Vector3::new(0.068, 0.075, 0.06), green, true);
        for j in 0..3 {
            let start = Vector3::new(side * 0.21, 1.16, -0.485);
            s.rod(
                start,
                Vector3::new(side * 0.61, 1.13 + j as f32 * 0.075, -0.43),
                0.006,
                pale,
                false,
            );
        }
    }
    s.ellipsoid(
        Vector3::new(0., 1.23, -0.514),
        Vector3::new(0.06, 0.038, 0.025),
        Vector3::new(0.23, 0.32, 0.15),
        false,
    );
    s.rod(
        Vector3::new(0., 1.20, -0.505),
        Vector3::new(0., 1.13, -0.49),
        0.008,
        black,
        false,
    );
    // Small flight console with luminous controls.
    s.ellipsoid(
        Vector3::new(0., 0.57, -0.69),
        Vector3::new(0.43, 0.095, 0.19),
        metal,
        false,
    );
    for i in -2..3 {
        s.ellipsoid(
            Vector3::new(i as f32 * 0.13, 0.66, -0.69),
            Vector3::new(0.025, 0.012, 0.025),
            green,
            true,
        );
    }
    // Visible energy emitter mounted on the near right side of the hull.
    s.rod(
        Vector3::new(1.65, 0.26, -0.90),
        Vector3::new(1.65, 0.58, -0.90),
        0.09,
        trim,
        false,
    );
    s.ellipsoid(
        Vector3::new(1.65, 0.58, -0.90),
        Vector3::new(0.18, 0.14, 0.18),
        black,
        false,
    );
    s.ellipsoid(
        Vector3::new(1.65, 0.70, -0.90),
        Vector3::new(0.12, 0.04, 0.12),
        green,
        true,
    );
    let mut glass = Sculpt::new();
    glass.cap(
        Vector3::new(0., 0.44, 0.),
        Vector3::new(1.10, 1.85, 1.10),
        Vector3::new(0.2, 0.7, 0.6),
        false,
        PI * 0.5,
        80,
        32,
    );
    (compact(s.mesh), compact(glass.mesh))
}

/// Share authored surface vertices to avoid transforming the same point six times per frame.
fn compact(mut mesh: Mesh) -> Mesh {
    use std::collections::HashMap;
    let mut unique = HashMap::new();
    let mut vertices = Vec::new();
    let mut remap = Vec::with_capacity(mesh.vertices.len());
    for v in mesh.vertices {
        let key = [
            v.position.x,
            v.position.y,
            v.position.z,
            v.normal.x,
            v.normal.y,
            v.normal.z,
            v.color.x,
            v.color.y,
            v.color.z,
            v.alpha,
        ]
        .map(f32::to_bits);
        let index = *unique.entry(key).or_insert_with(|| {
            let i = vertices.len() as u32;
            vertices.push(v);
            i
        });
        remap.push(index);
    }
    for i in &mut mesh.indices {
        *i = remap[*i as usize];
    }
    mesh.vertices = vertices;
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destroyer_geometry_is_bounded_finite_and_has_a_working_bay() {
        let v = Vehicle::destroyer(10.);
        assert!(v.mesh.materials.len() > 1);
        assert!(
            v.mesh
                .vertices
                .iter()
                .any(|p| p.alpha > 1.5 && p.color.z > 0.8)
        );
        assert!(
            v.mesh
                .vertices
                .iter()
                .any(|p| p.alpha > 1.5 && p.color.x > 0.8)
        );
        assert!(
            v.mesh
                .vertices
                .iter()
                .any(|p| p.alpha == 1. && p.color.x < 0.5)
        );
        assert!((v.radius - 13.5).abs() < 0.001);
        assert!(v.bay.is_some());
        assert!(v.muzzle.is_some());
        assert!(v.mesh.indices.len() / 3 < 20_000);
        assert!(
            v.mesh
                .indices
                .iter()
                .all(|&i| (i as usize) < v.mesh.vertices.len())
        );
        assert!(v.mesh.vertices.iter().all(|p| {
            [
                p.position.x,
                p.position.y,
                p.position.z,
                p.normal.x,
                p.normal.y,
                p.normal.z,
            ]
            .into_iter()
            .all(f32::is_finite)
        }));
    }
    #[test]
    fn saucer_has_finite_geometry_and_encloses_the_pilot() {
        let v = Vehicle::saucer(10.0);
        assert!((v.radius - 10.0).abs() < 0.001);
        for mesh in [&v.mesh, v.glass.as_ref().unwrap()] {
            assert!(mesh.indices.len() > 1000);
            assert!(
                mesh.indices
                    .iter()
                    .all(|&i| (i as usize) < mesh.vertices.len())
            );
            assert!(
                mesh.vertices
                    .iter()
                    .all(|v| v.position.x.is_finite() && v.normal.x.is_finite())
            );
        }
    }
}
