use raylib::prelude::{Vector2, Vector3};

const NEAR_CLIP_DISTANCE: f32 = 1.0;
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

    if depth <= NEAR_CLIP_DISTANCE {
        return None;
    }

    let focal_length = height * 0.5 / (VERTICAL_FOV * 0.5).tan();
    let x = camera_x / depth * focal_length + width * 0.5;
    let y = -camera_y / depth * focal_length + height * 0.5;

    Some((Vector2::new(x, y), depth))
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
