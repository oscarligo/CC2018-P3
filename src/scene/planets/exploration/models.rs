//! Original low-poly flora, fauna and archive stations, generated in the CPU pipeline.
use super::ModelKind;
use crate::rendering::{load_objects::Mesh, vertex::Vertex};
use raylib::prelude::Vector3;
use std::{
    f32::consts::{PI, TAU},
    sync::OnceLock,
};
pub(super) struct Model {
    pub solid: Mesh,
    pub glow: Mesh,
}
fn empty() -> Mesh {
    Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        materials: Vec::new(),
    }
}
fn tri(m: &mut Mesh, a: Vector3, mut b: Vector3, mut c: Vector3, out: Vector3, color: Vector3) {
    if (b - a).cross(c - a).dot(out) < 0. {
        std::mem::swap(&mut b, &mut c);
    }
    let n = (b - a).cross(c - a);
    if n.length_sqr() < 1e-12 {
        return;
    }
    for p in [a, b, c] {
        let mut v = Vertex::new(p);
        v.normal = n.normalize();
        v.color = color;
        m.indices.push(m.vertices.len() as u32);
        m.vertices.push(v);
    }
}
fn quad(m: &mut Mesh, p: [Vector3; 4], out: Vector3, c: Vector3) {
    tri(m, p[0], p[1], p[2], out, c);
    tri(m, p[0], p[2], p[3], out, c);
}
fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}
fn ellipsoid(m: &mut Mesh, center: Vector3, r: Vector3, color: Vector3, full: bool) {
    let (rows, cols) = if full { (4, 8) } else { (2, 6) };
    let point = |i: usize, j: usize| {
        let lat = PI * i as f32 / rows as f32;
        let lon = TAU * j as f32 / cols as f32;
        center
            + v(
                r.x * lat.sin() * lon.cos(),
                r.y * lat.cos(),
                r.z * lat.sin() * lon.sin(),
            )
    };
    for i in 0..rows {
        for j in 0..cols {
            let a = point(i, j);
            let b = point(i + 1, j);
            let c = point(i + 1, j + 1);
            let d = point(i, j + 1);
            let out = (a + b + c + d) * 0.25 - center;
            quad(m, [a, b, c, d], out, color * (0.92 + 0.08 * (j % 2) as f32));
        }
    }
}
fn cone(m: &mut Mesh, center: Vector3, radius: f32, height: f32, color: Vector3, full: bool) {
    let sides = if full { 10 } else { 6 };
    for i in 0..sides {
        let a = i as f32 * TAU / sides as f32;
        let b = (i + 1) as f32 * TAU / sides as f32;
        let p = center + v(a.cos() * radius, 0., a.sin() * radius);
        let q = center + v(b.cos() * radius, 0., b.sin() * radius);
        tri(
            m,
            p,
            q,
            center + v(0., height, 0.),
            v(
                ((a + b) * 0.5).cos(),
                radius / height,
                ((a + b) * 0.5).sin(),
            ),
            color,
        );
        tri(m, center, q, p, v(0., -1., 0.), color);
    }
}
fn box_mesh(m: &mut Mesh, c: Vector3, r: Vector3, color: Vector3) {
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let face = |a: f32, b: f32| match axis {
                0 => c + v(sign * r.x, a * r.y, b * r.z),
                1 => c + v(a * r.x, sign * r.y, b * r.z),
                _ => c + v(a * r.x, b * r.y, sign * r.z),
            };
            let out = match axis {
                0 => v(sign, 0., 0.),
                1 => v(0., sign, 0.),
                _ => v(0., 0., sign),
            };
            quad(
                m,
                [face(-1., -1.), face(1., -1.), face(1., 1.), face(-1., 1.)],
                out,
                color,
            );
        }
    }
}
fn eyes(model: &mut Model, x: f32, y: f32, z: f32, size: f32, full: bool) {
    for sign in [-1., 1.] {
        ellipsoid(
            &mut model.solid,
            v(sign * x, y, z),
            v(size, size * 1.12, size * 0.62),
            v(0.025, 0.04, 0.065),
            full,
        );
        ellipsoid(
            &mut model.glow,
            v(sign * x - size * 0.18, y + size * 0.30, z + size * 0.55),
            v(size * 0.22, size * 0.28, size * 0.12),
            v(0.78, 1., 0.87),
            false,
        );
    }
}
fn build(kind: ModelKind, full: bool) -> Model {
    let mut m = Model {
        solid: empty(),
        glow: empty(),
    };
    match kind {
        ModelKind::Canopy => {
            cone(
                &mut m.solid,
                v(0., 0., 0.),
                0.12,
                1.9,
                v(0.23, 0.35, 0.12),
                full,
            );
            for (height, radius) in [(1.10, 0.65), (1.65, 0.88), (2.12, 0.60)] {
                ellipsoid(
                    &mut m.solid,
                    v(0., height, 0.),
                    v(radius, 0.20, radius * 0.83),
                    v(0.17, 0.77, 0.22),
                    full,
                );
                cone(
                    &mut m.glow,
                    v(0., height + 0.10, 0.),
                    0.17,
                    0.32,
                    v(0.45, 0.98, 0.65),
                    full,
                );
            }
            for i in 0..5 {
                let a = i as f32 * TAU / 5.;
                let p = v(a.cos() * 0.63, 0.06, a.sin() * 0.63);
                tri(
                    &mut m.solid,
                    v(-0.09, 0.1, 0.),
                    p,
                    v(0.09, 0.1, 0.),
                    v(0., 1., 0.),
                    v(0.29, 0.58, 0.12),
                );
            }
        }
        ModelKind::Fern => {
            cone(
                &mut m.solid,
                v(0., 0., 0.),
                0.09,
                1.2,
                v(0.10, 0.48, 0.23),
                full,
            );
            let leaves = if full { 9 } else { 5 };
            for i in 0..leaves {
                let a = i as f32 * TAU / leaves as f32;
                let dir = v(a.cos(), 0., a.sin());
                let side = v(-a.sin(), 0., a.cos()) * 0.22;
                let root = v(0., 0.16, 0.);
                let middle = dir * 0.58 + v(0., 0.63, 0.);
                let tip = dir * 0.96 + v(0., 0.42, 0.);
                let col = if i % 2 == 0 {
                    v(0.10, 0.80, 0.49)
                } else {
                    v(0.42, 0.90, 0.15)
                };
                quad(
                    &mut m.solid,
                    [root, middle + side, tip, middle - side],
                    v(0., 1., 0.),
                    col,
                );
                quad(
                    &mut m.solid,
                    [root, middle - side, tip, middle + side],
                    v(0., -1., 0.),
                    col * 0.6,
                );
            }
            ellipsoid(
                &mut m.glow,
                v(0., 1.12, 0.),
                v(0.22, 0.29, 0.22),
                v(1., 0.37, 0.69),
                full,
            );
            cone(
                &mut m.glow,
                v(0., 1.20, 0.),
                0.16,
                0.38,
                v(1., 0.82, 0.45),
                full,
            );
        }
        ModelKind::Hopper => {
            ellipsoid(
                &mut m.solid,
                v(0., 0.58, 0.),
                v(0.62, 0.43, 0.77),
                v(0.42, 0.89, 0.14),
                full,
            );
            ellipsoid(
                &mut m.solid,
                v(0., 0.88, 0.47),
                v(0.51, 0.35, 0.42),
                v(0.62, 0.98, 0.28),
                full,
            );
            for sign in [-1., 1.] {
                ellipsoid(
                    &mut m.solid,
                    v(sign * 0.52, 0.23, -0.31),
                    v(0.25, 0.24, 0.39),
                    v(0.22, 0.63, 0.23),
                    full,
                );
                ellipsoid(
                    &mut m.solid,
                    v(sign * 0.39, 0.09, 0.54),
                    v(0.20, 0.09, 0.34),
                    v(0.10, 0.46, 0.29),
                    full,
                );
                cone(
                    &mut m.solid,
                    v(sign * 0.30, 1.03, 0.43),
                    0.055,
                    0.52,
                    v(0.29, 0.65, 0.19),
                    full,
                );
                ellipsoid(
                    &mut m.glow,
                    v(sign * 0.30, 1.57, 0.43),
                    v(0.09, 0.10, 0.09),
                    v(0.97, 0.62, 0.14),
                    false,
                );
            }
            eyes(&mut m, 0.25, 1.02, 0.80, 0.18, full);
            for i in 0..3 {
                ellipsoid(
                    &mut m.glow,
                    v(0., 0.99 - i as f32 * 0.07, -0.15 - i as f32 * 0.22),
                    v(0.12, 0.045, 0.09),
                    v(0.18, 0.91, 0.91),
                    false,
                );
            }
        }
        ModelKind::Skimmer => {
            ellipsoid(
                &mut m.solid,
                v(0., 0.55, 0.),
                v(0.32, 0.28, 0.78),
                v(0.68, 0.30, 0.92),
                full,
            );
            for sign in [-1., 1.] {
                let root = v(sign * 0.20, 0.62, 0.);
                let tip = v(sign * 1.42, 0.72, -0.30);
                let front = v(sign * 0.60, 0.62, 0.68);
                let rear = v(sign * 0.50, 0.32, -0.65);
                tri(
                    &mut m.solid,
                    root,
                    front,
                    tip,
                    v(0., 1., 0.),
                    v(0.36, 0.75, 0.93),
                );
                tri(
                    &mut m.solid,
                    root,
                    tip,
                    rear,
                    v(0., 1., 0.),
                    v(0.65, 0.33, 0.93),
                );
                tri(
                    &mut m.glow,
                    front,
                    rear,
                    tip,
                    v(0., -1., 0.),
                    v(0.35, 0.80, 0.66),
                );
                ellipsoid(
                    &mut m.glow,
                    tip,
                    v(0.11, 0.06, 0.11),
                    v(0.92, 0.66, 1.),
                    false,
                );
            }
            eyes(&mut m, 0.17, 0.70, 0.69, 0.13, full);
            cone(
                &mut m.solid,
                v(0., 0.72, -0.55),
                0.20,
                0.45,
                v(0.84, 0.42, 0.92),
                full,
            );
        }
        ModelKind::Station => {
            ellipsoid(
                &mut m.solid,
                v(0., 0., 0.),
                v(0.46, 0.40, 0.91),
                v(0.56, 0.61, 0.67),
                full,
            );
            box_mesh(
                &mut m.solid,
                v(0., -0.36, -0.10),
                v(0.35, 0.10, 0.55),
                v(0.10, 0.14, 0.22),
            );
            for sign in [-1., 1.] {
                box_mesh(
                    &mut m.solid,
                    v(sign * 0.94, 0., -0.16),
                    v(0.70, 0.055, 0.07),
                    v(0.58, 0.59, 0.61),
                );
                for row in [-1., 1.] {
                    let panel = v(sign * 1.30, 0., row * 0.54);
                    box_mesh(
                        &mut m.solid,
                        panel,
                        v(0.70, 0.045, 0.40),
                        v(0.08, 0.13, 0.23),
                    );
                    box_mesh(
                        &mut m.glow,
                        panel + v(0., 0.048, 0.),
                        v(0.63, 0.008, 0.34),
                        v(0.12, 0.42, 0.70),
                    );
                    for stripe in 0..4 {
                        box_mesh(
                            &mut m.solid,
                            panel + v(-0.48 + stripe as f32 * 0.32, 0.06, 0.),
                            v(0.014, 0.009, 0.35),
                            v(0.65, 0.71, 0.76),
                        );
                    }
                }
            }
            ellipsoid(
                &mut m.glow,
                v(0., 0., 0.83),
                v(0.20, 0.20, 0.13),
                v(0.31, 0.93, 1.),
                full,
            );
            cone(
                &mut m.solid,
                v(0., 0.30, 0.30),
                0.18,
                0.40,
                v(0.84, 0.66, 0.40),
                full,
            );
            for i in 0..8 {
                let a = i as f32 * TAU / 8.;
                box_mesh(
                    &mut m.glow,
                    v(a.cos() * 0.47, a.sin() * 0.42, -0.10),
                    v(0.048, 0.046, 0.21),
                    v(1., 0.57, 0.75),
                );
            }
        }
    }
    weld(&mut m.solid);
    weld(&mut m.glow);
    m
}
// Adjacent triangles on a facet can share vertices. Keep sharp normals and color boundaries.
fn weld(mesh: &mut Mesh) {
    use std::collections::HashMap;
    let mut unique = HashMap::new();
    let mut vertices = Vec::new();
    for index in &mut mesh.indices {
        let vertex = mesh.vertices[*index as usize];
        let p = vertex.position;
        let n = vertex.normal;
        let c = vertex.color;
        let key =
            [p.x, p.y, p.z, n.x, n.y, n.z, c.x, c.y, c.z].map(|x| (x * 10000.).round() as i32);
        *index = *unique.entry(key).or_insert_with(|| {
            let i = vertices.len() as u32;
            vertices.push(vertex);
            i
        });
    }
    mesh.vertices = vertices;
}
pub(super) fn get(kind: ModelKind, full: bool) -> &'static Model {
    static MODELS: OnceLock<Vec<Model>> = OnceLock::new();
    let models = MODELS.get_or_init(|| {
        [
            ModelKind::Canopy,
            ModelKind::Fern,
            ModelKind::Hopper,
            ModelKind::Skimmer,
            ModelKind::Station,
        ]
        .into_iter()
        .flat_map(|kind| [build(kind, false), build(kind, true)])
        .collect()
    });
    &models[kind as usize * 2 + usize::from(full)]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_models_have_valid_geometry_and_reduced_lods() {
        for kind in [
            ModelKind::Canopy,
            ModelKind::Fern,
            ModelKind::Hopper,
            ModelKind::Skimmer,
            ModelKind::Station,
        ] {
            let full = get(kind, true);
            let low = get(kind, false);
            assert!(full.solid.indices.len() > low.solid.indices.len());
            for mesh in [&full.solid, &full.glow, &low.solid, &low.glow] {
                assert!(!mesh.indices.is_empty());
                assert!(mesh.vertices.iter().all(|v|v.position.length()<4. && (v.normal.length()-1.).abs()<0.001));
            }
        }
    }
}
