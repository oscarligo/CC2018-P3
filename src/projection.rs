use raylib::prelude::{Vector2, Vector3};

use crate::uniforms::Uniforms;

pub(crate) const NEAR_CLIP_DISTANCE: f32 = 1.0;
const VERTICAL_FOV: f32 = 60.0_f32.to_radians();

pub fn project_vertex(
    vertex: Vector3,
    camera_pos: Vector3,
    camera_target: Vector3,
    camera_up: Vector3,
    width: f32,
    height: f32,
) -> Option<(Vector2, f32)> {
    let forward = normalize(camera_target - camera_pos)?;
    let right = normalize(cross(camera_up, forward))?;
    let up = cross(forward, right);
    let relative = vertex - camera_pos;
    let camera_x = dot(relative, right);
    let camera_y = dot(relative, up);
    let depth = dot(relative, forward);

    if depth < NEAR_CLIP_DISTANCE {
        return None;
    }

    let focal_length = height * 0.5 / (VERTICAL_FOV * 0.5).tan();
    let x = camera_x / depth * focal_length + width * 0.5;
    let y = -camera_y / depth * focal_length + height * 0.5;

    Some((Vector2::new(x, y), depth))
}

pub fn camera_depth(vertex: Vector3, uniforms: &Uniforms) -> Option<f32> {
    let forward = normalize(uniforms.camera_target - uniforms.camera_pos)?;
    Some(dot(vertex - uniforms.camera_pos, forward))
}

pub fn sphere_is_visible(
    center: Vector3,
    radius: f32,
    uniforms: &Uniforms,
    width: u32,
    height: u32,
) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    let Some(forward) = normalize(uniforms.camera_target - uniforms.camera_pos) else {
        return false;
    };
    let Some(right) = normalize(cross(uniforms.camera_up, forward)) else {
        return false;
    };
    let up = cross(forward, right);
    let relative = center - uniforms.camera_pos;
    let depth = dot(relative, forward);
    if depth + radius <= NEAR_CLIP_DISTANCE {
        return false;
    }

    let vertical_slope = (VERTICAL_FOV * 0.5).tan();
    let horizontal_slope = vertical_slope * width as f32 / height as f32;
    let horizontal_margin = radius * (1.0 + horizontal_slope * horizontal_slope).sqrt();
    let vertical_margin = radius * (1.0 + vertical_slope * vertical_slope).sqrt();

    dot(relative, right).abs() <= depth * horizontal_slope + horizontal_margin
        && dot(relative, up).abs() <= depth * vertical_slope + vertical_margin
}

pub fn projected_sphere_radius(
    center: Vector3,
    radius: f32,
    uniforms: &Uniforms,
    viewport_height: u32,
) -> f32 {
    let Some(forward) = normalize(uniforms.camera_target - uniforms.camera_pos) else {
        return 0.0;
    };
    let depth = dot(center - uniforms.camera_pos, forward);
    if depth <= NEAR_CLIP_DISTANCE {
        return f32::INFINITY;
    }
    let focal_length = viewport_height as f32 * 0.5 / (VERTICAL_FOV * 0.5).tan();
    radius * focal_length / depth
}

fn dot(a: Vector3, b: Vector3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalize(vector: Vector3) -> Option<Vector3> {
    let length = dot(vector, vector).sqrt();
    (length > f32::EPSILON).then(|| vector * (1.0 / length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use raylib::prelude::Matrix;

    fn uniforms() -> Uniforms {
        Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        }
    }

    #[test]
    fn sphere_visibility_is_conservative_at_frustum_edges() {
        let uniforms = uniforms();
        assert!(sphere_is_visible(
            Vector3::new(0.0, 0.0, 10.0),
            1.0,
            &uniforms,
            800,
            600
        ));
        assert!(sphere_is_visible(
            Vector3::new(8.0, 0.0, 10.0),
            1.0,
            &uniforms,
            800,
            600
        ));
        assert!(!sphere_is_visible(
            Vector3::new(20.0, 0.0, 10.0),
            1.0,
            &uniforms,
            800,
            600
        ));
        assert!(!sphere_is_visible(
            Vector3::new(0.0, 0.0, -2.0),
            0.5,
            &uniforms,
            800,
            600
        ));
    }

    #[test]
    fn projected_radius_decreases_with_distance() {
        let uniforms = uniforms();
        let near = projected_sphere_radius(Vector3::new(0.0, 0.0, 10.0), 1.0, &uniforms, 600);
        let far = projected_sphere_radius(Vector3::new(0.0, 0.0, 20.0), 1.0, &uniforms, 600);
        assert!((near / far - 2.0).abs() < 0.001);
    }
}
