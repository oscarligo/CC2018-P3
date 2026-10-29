use crate::vertex::Vertex;
use raylib::prelude::{Vector2, Vector3};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn load_obj(path: &str) -> Result<Mesh, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut vertices: Vec<Vector3> = Vec::with_capacity(1024);
    let mut indices: Vec<u32> = Vec::with_capacity(2048);
    let mut line_buffer = String::with_capacity(128);

    while reader.read_line(&mut line_buffer)? > 0 {
        let trimmed = line_buffer.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            line_buffer.clear();
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        if let Some(prefix) = parts.next() {
            match prefix {
                "v" => {
                    let x: f32 = parts.next().ok_or("Missing x")?.parse()?;
                    let y: f32 = parts.next().ok_or("Missing y")?.parse()?;
                    let z: f32 = parts.next().ok_or("Missing z")?.parse()?;
                    vertices.push(Vector3::new(x, y, z));
                }
                "f" => {
                    for _ in 0..3 {
                        let token = parts.next().ok_or("Expected 3 vertices per face")?;
                        let v_str = token.split('/').next().ok_or("Invalid face format")?;
                        let v_idx: usize = v_str.parse()?;
                        let index = v_idx.checked_sub(1).ok_or("OBJ indices start at 1")?;
                        indices.push(u32::try_from(index)?);
                    }
                }
                _ => {}
            }
        }

        line_buffer.clear();
    }

    if let Some(index) = indices
        .iter()
        .find(|index| **index as usize >= vertices.len())
    {
        return Err(format!("Vertex index {} is out of bounds", index + 1).into());
    }

    Ok(Mesh {
        vertices: vertices.into_iter().map(Vertex::new).collect(),
        indices,
    })
}

pub fn project_vertex(
    v: Vector3,
    camera_pos: Vector3,
    camera_target: Vector3,
    fov_scale: f32,
    width: f32,
    height: f32,
) -> Option<Vector2> {
    let forward = normalize(camera_target - camera_pos)?;
    let right = normalize(cross(Vector3::new(0.0, 1.0, 0.0), forward))?;
    let up = cross(forward, right);
    let relative = v - camera_pos;
    let camera_x = dot(relative, right);
    let camera_y = dot(relative, up);
    let z = dot(relative, forward);

    if z <= 0.1 {
        return None;
    }

    let aspect_ratio = width / height;
    let x = camera_x / z * fov_scale / aspect_ratio + (width / 2.0);
    let y = -camera_y / z * fov_scale + (height / 2.0);

    Some(Vector2::new(x, y))
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
