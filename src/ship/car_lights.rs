use super::{direction, point};
use crate::rendering::{
    fragment::Fragment,
    framebuffer::Framebuffer,
    load_objects::Material,
    projection::{project_vertex, projected_sphere_radius},
    shaders::fragment_shader,
    uniforms::Uniforms,
    vertex::Vertex,
};
use raylib::prelude::*;

const DASHBOARD_SOURCES: [Vector3; 2] = [
    Vector3 {
        x: -0.45,
        y: 1.30,
        z: 0.74,
    },
    Vector3 {
        x: 0.45,
        y: 1.30,
        z: 0.74,
    },
];
const DASHBOARD_COLOR: Vector3 = Vector3 {
    x: 0.12,
    y: 0.65,
    z: 0.75,
};

fn rear_intensity(braking: bool) -> f32 {
    if braking { 1.0 } else { 0.28 }
}

pub(super) fn shade(fragment: &Fragment, materials: &[Material], braking: bool) -> Option<Vector3> {
    let shaded = fragment_shader(fragment, materials)?;
    let (Some(uv), Some(surface)) = (fragment.tex_coord, fragment.surface) else {
        return Some(shaded);
    };
    let Some(texture) = materials
        .get(fragment.material_index)
        .and_then(|m| m.diffuse_texture.as_ref())
    else {
        return Some(shaded);
    };
    if texture.dimensions() != (128, 128) {
        return Some(shaded);
    }
    let x = (uv.x.clamp(0.0, 1.0) * 127.0) as u32;
    let y = ((1.0 - uv.y).clamp(0.0, 1.0) * 127.0) as u32;
    let pixel = texture.get_pixel(x, y).0;
    // Mask the actual lens texels, rather than making the painted panel emissive.
    let rear = surface.position.z < -3.38
        && surface.normal.z < -0.9
        && (106..=120).contains(&x)
        && (7..=10).contains(&y)
        && pixel[0] as f32 > pixel[1] as f32 * 1.8;
    let front = surface.position.z > 3.38
        && surface.normal.z > 0.9
        && (78..=88).contains(&x)
        && (5..=8).contains(&y)
        && pixel[0] > 110
        && pixel[1] > 90;
    if rear || front {
        let color = Vector3::new(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32) * (1.0 / 255.0);
        return Some(
            color
                * if rear {
                    0.75 + rear_intensity(braking) * 2.0
                } else {
                    2.2
                },
        );
    }
    Some(shaded)
}

pub(super) fn illuminate(vertex: &Vertex, color: &mut Vector3) {
    let p = vertex.position;
    // The instruments face the seats (-Z); their spill fades quickly away from the dash.
    if p.x.abs() < 0.95 && p.z.abs() < 1.65 && p.y > 1.05 {
        for source in DASHBOARD_SOURCES {
            let to_light = source - p;
            let distance = to_light.length().max(0.01);
            let facing = vertex.normal.dot(to_light * (1.0 / distance)).max(0.0);
            let toward_seats = ((source.z - p.z) * 4.0).clamp(0.0, 1.0);
            *color += DASHBOARD_COLOR
                * (facing * toward_seats * 0.65 / (1.0 + 5.0 * distance * distance));
        }
    }
    // The textured dashboard includes the blue instruments and steering wheel.
    if p.x.abs() < 1.12
        && p.z > 0.70
        && p.z < 0.90
        && p.y > 0.90
        && p.y < 1.45
        && vertex.normal.z < -0.95
    {
        *color = Vector3::new(color.x.max(1.25), color.y.max(1.65), color.z.max(1.85));
    }
}

fn glow(
    fb: &mut Framebuffer,
    u: &Uniforms,
    local: Vector3,
    radius: f32,
    color: Vector3,
    power: f32,
) {
    let position = point(&u.model_matrix, local);
    if let Some((screen, depth)) = project_vertex(
        position,
        u.camera_pos,
        u.camera_target,
        u.camera_up,
        fb.width as f32,
        fb.height as f32,
    ) {
        let radius = projected_sphere_radius(position, radius, u, fb.height).clamp(1.0, 60.0);
        crate::ship::alien::glow(fb, screen, depth, radius, color, power);
    }
}

pub(super) fn draw(fb: &mut Framebuffer, u: &Uniforms) {
    let dashboard_back = direction(&u.model_matrix, Vector3::new(0.0, 0.0, -1.0)).normalize();
    for source in DASHBOARD_SOURCES {
        let facing = dashboard_back
            .dot((u.camera_pos - point(&u.model_matrix, source)).normalize())
            .max(0.0);
        glow(fb, u, source, 0.16, DASHBOARD_COLOR, 0.10 * facing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_lens_pixels_emit_light_and_paint_keeps_its_shading() {
        use crate::rendering::fragment::SurfacePoint;
        let mesh = crate::rendering::load_objects::load_obj("assets/red_car.obj").unwrap();
        let material_index = mesh
            .materials
            .iter()
            .position(|m| {
                m.diffuse_texture
                    .as_ref()
                    .is_some_and(|t| t.dimensions() == (128, 128))
            })
            .unwrap();
        let fragment = |x: u32, y: u32, position, normal, intensity| Fragment {
            surface: Some(SurfacePoint { position, normal }),
            position: Vector2::zero(),
            color: Vector3::one() * intensity,
            alpha: 1.0,
            depth: 1.0,
            tex_coord: Some(Vector2::new(
                (x as f32 + 0.25) / 127.0,
                1.0 - (y as f32 + 0.25) / 127.0,
            )),
            material_index,
        };
        for side in [-1.0, 1.0] {
            for x in [108, 113, 118] {
                let mut lamp = fragment(
                    x,
                    8,
                    Vector3::new(side * 0.7, 1.05, -3.46),
                    Vector3::new(0.0, 0.0, -1.0),
                    0.0,
                );
                let unlit = shade(&lamp, &mesh.materials, false).unwrap();
                assert!(unlit.x > 0.5, "rear lens at {x} must emit");
                lamp.color = Vector3::one();
                assert_eq!(shade(&lamp, &mesh.materials, false).unwrap(), unlit);
                assert!(shade(&lamp, &mesh.materials, true).unwrap().x > unlit.x * 2.0);
            }
        }
        for (x, y, z, normal) in [
            (110, 4, -3.46, -1.0), // rear red paint
            (82, 2, 3.46, 1.0),    // front red paint
            (106, 6, -3.46, -1.0), // rear lens surround
        ] {
            let paint = fragment(
                x,
                y,
                Vector3::new(0.7, 1.1, z),
                Vector3::new(0.0, 0.0, normal),
                0.1,
            );
            assert_eq!(
                shade(&paint, &mesh.materials, true),
                fragment_shader(&paint, &mesh.materials)
            );
        }
    }

    #[test]
    fn dashboard_spill_lights_surfaces_facing_controls_and_fades_toward_seats() {
        let shade = |position, normal| {
            let mut vertex = Vertex::new(position);
            vertex.normal = normal;
            let mut color = Vector3::zero();
            illuminate(&vertex, &mut color);
            color
        };
        let facing = Vector3::new(0.0, 0.0, 1.0);
        let near = shade(Vector3::new(0.45, 1.30, 0.40), facing);
        let far = shade(Vector3::new(0.45, 1.30, -1.0), facing);
        assert!(near.y > far.y * 3.0);
        assert!(near.y > near.x * 3.0);
        assert_eq!(
            shade(Vector3::new(0.45, 1.30, 0.40), -facing),
            Vector3::zero()
        );
    }

    #[test]
    fn actual_dashboard_and_steering_wheel_texture_stay_lit_without_sunlight() {
        let mesh = crate::rendering::load_objects::load_obj("assets/red_car.obj").unwrap();
        let mut lit = 0;
        for vertex in &mesh.vertices {
            let p = vertex.position;
            if p.x.abs() < 1.12
                && p.z > 0.70
                && p.z < 0.90
                && p.y > 0.90
                && p.y < 1.45
                && vertex.normal.z < -0.95
            {
                let mut color = Vector3::zero();
                illuminate(vertex, &mut color);
                assert!(color.x >= 1.25 && color.y >= 1.65 && color.z >= 1.85);
                lit += 1;
            }
        }
        assert!(lit >= 6, "The dashboard faces must match the actual model");
    }
}
