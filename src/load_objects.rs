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
    fov_scale: f32,
    width: f32,
    height: f32,
) -> Option<Vector2> {
    let z = v.z - camera_pos.z;

    if z <= 0.1 {
        return None;
    }

    let aspect_ratio = width / height;
    let x = (v.x - camera_pos.x) / z * fov_scale / aspect_ratio + (width / 2.0);
    let y = -(v.y - camera_pos.y) / z * fov_scale + (height / 2.0);

    Some(Vector2::new(x, y))
}