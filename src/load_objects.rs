use crate::vertex::Vertex;
use raylib::prelude::{Vector2, Vector3};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn load_obj(path: &str) -> Result<Vec<Vertex>, Box<dyn std::error::Error>> {
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

                        indices.push((v_idx - 1) as u32);
                    }
                }
                _ => {}
            }
        }

        line_buffer.clear();
    }

    indices
        .into_iter()
        .map(|index| {
            vertices
                .get(index as usize)
                .copied()
                .map(Vertex::new)
                .ok_or_else(|| format!("Vertex index {} is out of bounds", index + 1).into())
        })
        .collect()
}

pub fn project_vertex(
    v: Vector3,
    camera_pos: Vector3,
    fov_scale: f32,
    width: f32,
    height: f32,
) -> Option<Vector2> {
    let z = v.z - camera_pos.z;

    if z <= 0.1 {
        return None;
    }

    let x = (v.x - camera_pos.x) / z * fov_scale + (width / 2.0);
    let y = -(v.y - camera_pos.y) / z * fov_scale + (height / 2.0);

    Some(Vector2::new(x, y))
}
