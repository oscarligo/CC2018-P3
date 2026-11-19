use crate::rendering::load_objects::{Material, Mesh};
use crate::rendering::vertex::Vertex;
use raylib::prelude::Vector3;

const AMBER: Vector3 = Vector3 {
    x: 1.0,
    y: 0.66,
    z: 0.34,
};
const BLUE: Vector3 = Vector3 {
    x: 0.44,
    y: 0.82,
    z: 1.0,
};

pub(super) struct Architecture {
    pub(super) shell: Mesh,
    pub(super) lights: Mesh,
}

impl Architecture {
    pub(super) fn empty() -> Self {
        let mesh = || Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            materials: vec![Material {
                diffuse_texture: None,
            }],
        };
        Self {
            shell: mesh(),
            lights: mesh(),
        }
    }
}

pub(super) fn tower() -> Architecture {
    let mut model = Architecture::empty();
    profile(
        &mut model.shell,
        Vector3::zero(),
        &[
            (-1.0, 0.48, 0.40),
            (0.12, 0.48, 0.40),
            (0.17, 0.38, 0.33),
            (0.54, 0.38, 0.33),
            (0.57, 0.41, 0.36),
            (0.60, 0.41, 0.36),
            (0.64, 0.27, 0.24),
            (0.85, 0.27, 0.24),
            (0.89, 0.17, 0.15),
            (1.0, 0.08, 0.08),
        ],
        Vector3::new(0.40, 0.46, 0.53),
    );
    for (height, width, depth, color) in [
        (0.23, 0.38, 0.33, AMBER * 0.68),
        (0.32, 0.38, 0.33, BLUE * 0.61),
        (0.42, 0.38, 0.33, AMBER * 0.88),
        (0.58, 0.41, 0.36, BLUE * 0.92),
        (0.70, 0.27, 0.24, AMBER * 0.70),
        (0.80, 0.27, 0.24, AMBER),
    ] {
        facade_lights(&mut model.lights, height, 0.011, width, depth, color);
    }
    profile(
        &mut model.shell,
        Vector3::zero(),
        &[
            (0.99, 0.065, 0.065),
            (1.04, 0.065, 0.065),
            (1.13, 0.022, 0.022),
        ],
        Vector3::new(0.68, 0.72, 0.76),
    );
    facade_lights(&mut model.lights, 1.025, 0.006, 0.065, 0.065, AMBER);
    model
}

pub(super) fn platform() -> Architecture {
    let mut model = Architecture::empty();
    let alloy = Vector3::new(0.33, 0.39, 0.46);
    for x in [-0.21, 0.21] {
        profile(
            &mut model.shell,
            Vector3::new(x, 0.0, 0.0),
            &[(-1.0, 0.10, 0.19), (0.64, 0.10, 0.19), (0.73, 0.15, 0.24)],
            alloy,
        );
    }
    profile(
        &mut model.shell,
        Vector3::zero(),
        &[
            (0.59, 0.32, 0.29),
            (0.74, 0.50, 0.46),
            (0.88, 0.50, 0.46),
            (0.94, 0.44, 0.40),
        ],
        Vector3::new(0.50, 0.55, 0.59),
    );
    facade_lights(&mut model.lights, 0.81, 0.030, 0.50, 0.46, BLUE * 0.88);
    // Una pista octogonal rodea las dos marcas de aproximación.
    for segment in 0..8 {
        let angle = |i: usize| (i as f32 + 0.5) * std::f32::consts::FRAC_PI_4;
        let point = |a: f32, radius: f32| Vector3::new(a.cos() * radius, 0.942, a.sin() * radius);
        let a = angle(segment) + 0.04;
        let b = angle(segment + 1) - 0.04;
        face(
            &mut model.lights,
            &[
                point(a, 0.33),
                point(b, 0.33),
                point(b, 0.35),
                point(a, 0.35),
            ],
            AMBER * 0.83,
        );
    }
    for z in [-0.07, 0.07] {
        roof_bar(&mut model.lights, 0.942, 0.12, z, 0.014, BLUE);
    }
    model
}

fn profile(mesh: &mut Mesh, origin: Vector3, levels: &[(f32, f32, f32)], color: Vector3) {
    let ring = |(height, x, z): (f32, f32, f32)| {
        let bevel = x.min(z) * 0.28;
        [
            (-x + bevel, -z),
            (x - bevel, -z),
            (x, -z + bevel),
            (x, z - bevel),
            (x - bevel, z),
            (-x + bevel, z),
            (-x, z - bevel),
            (-x, -z + bevel),
        ]
        .map(|(x, z)| origin + Vector3::new(x, height, z))
    };

    let mut lower = ring(levels[0]);
    face(mesh, &lower, color);
    for &level in &levels[1..] {
        let upper = ring(level);
        for side in 0..lower.len() {
            let next = (side + 1) % lower.len();
            face(
                mesh,
                &[lower[side], upper[side], upper[next], lower[next]],
                color,
            );
        }
        lower = upper;
    }
    lower.reverse();
    face(mesh, &lower, color);
}

fn facade_lights(mesh: &mut Mesh, y: f32, half_height: f32, x: f32, z: f32, color: Vector3) {
    let up = Vector3::new(0.0, 1.0, 0.0);
    for (normal, distance, span) in [
        (Vector3::new(1.0, 0.0, 0.0), x, z),
        (Vector3::new(-1.0, 0.0, 0.0), x, z),
        (Vector3::new(0.0, 0.0, 1.0), z, x),
        (Vector3::new(0.0, 0.0, -1.0), z, x),
    ] {
        let origin = normal * (distance + 0.003) + up * y;
        let tangent = up.cross(normal) * span;
        let height = up * half_height;
        for (left, right) in [(-0.65, -0.09), (0.09, 0.65)] {
            let a = origin + tangent * left;
            let b = origin + tangent * right;
            face(
                mesh,
                &[a - height, b - height, b + height, a + height],
                color,
            );
        }
    }
}

fn roof_bar(mesh: &mut Mesh, y: f32, half_width: f32, z: f32, half_depth: f32, color: Vector3) {
    face(
        mesh,
        &[
            Vector3::new(-half_width, y, z + half_depth),
            Vector3::new(half_width, y, z + half_depth),
            Vector3::new(half_width, y, z - half_depth),
            Vector3::new(-half_width, y, z - half_depth),
        ],
        color,
    );
}

fn face(mesh: &mut Mesh, points: &[Vector3], color: Vector3) {
    let normal = (points[1] - points[0])
        .cross(points[2] - points[0])
        .normalize();
    let start = mesh.vertices.len() as u32;
    mesh.vertices.extend(points.iter().map(|&point| {
        let mut vertex = Vertex::new(point);
        vertex.normal = normal;
        vertex.color = color;
        vertex
    }));
    for index in 1..points.len() as u32 - 1 {
        mesh.indices
            .extend([start, start + index, start + index + 1]);
    }
}
