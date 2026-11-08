use crate::vertex::Vertex;
use image::RgbaImage;
use raylib::prelude::{Vector2, Vector3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const POSITION_CAPACITY: usize = 1024;
const TEXTURE_COORD_CAPACITY: usize = 1024;
const NORMAL_CAPACITY: usize = 1024;
const VERTEX_CAPACITY: usize = 2048;
const INDEX_CAPACITY: usize = 2048;
const LINE_BUFFER_CAPACITY: usize = 128;

pub struct Material {
    pub diffuse_texture: Option<RgbaImage>,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub materials: Vec<Material>,
}

pub fn load_obj(path: &str) -> Result<Mesh, Box<dyn std::error::Error>> {
    let obj_path = Path::new(path);
    let file = File::open(obj_path)?;
    let mut reader = BufReader::new(file);

    let mut positions: Vec<Vector3> = Vec::with_capacity(POSITION_CAPACITY);
    let mut tex_coords: Vec<Vector2> = Vec::with_capacity(TEXTURE_COORD_CAPACITY);
    let mut normals: Vec<Vector3> = Vec::with_capacity(NORMAL_CAPACITY);
    let mut vertices: Vec<Vertex> = Vec::with_capacity(VERTEX_CAPACITY);
    let mut indices: Vec<u32> = Vec::with_capacity(INDEX_CAPACITY);
    let mut materials = vec![Material {
        diffuse_texture: None,
    }];
    let mut material_indices = HashMap::new();
    let mut current_material = 0;
    let mut line_buffer = String::with_capacity(LINE_BUFFER_CAPACITY);

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
                    positions.push(Vector3::new(x, y, z));
                }
                "vt" => {
                    let u: f32 = parts.next().ok_or("Missing texture u")?.parse()?;
                    let v: f32 = parts.next().ok_or("Missing texture v")?.parse()?;
                    tex_coords.push(Vector2::new(u, v));
                }
                "vn" => {
                    let x: f32 = parts.next().ok_or("Missing normal x")?.parse()?;
                    let y: f32 = parts.next().ok_or("Missing normal y")?.parse()?;
                    let z: f32 = parts.next().ok_or("Missing normal z")?.parse()?;
                    normals.push(Vector3::new(x, y, z));
                }
                "mtllib" => {
                    let filename = parts.collect::<Vec<_>>().join(" ");
                    let mtl_path = obj_path.parent().unwrap_or(Path::new("")).join(filename);
                    if mtl_path.exists() {
                        for (name, material) in load_mtl(&mtl_path)? {
                            material_indices.insert(name, materials.len());
                            materials.push(material);
                        }
                    }
                }
                "usemtl" => {
                    let name = parts.collect::<Vec<_>>().join(" ");
                    current_material = material_indices.get(&name).copied().unwrap_or(0);
                }
                "f" => {
                    let corners = parts
                        .map(|token| {
                            parse_corner(token, &positions, &tex_coords, &normals, current_material)
                        })
                        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                    for triangle in 1..corners.len().saturating_sub(1) {
                        for corner in [corners[0], corners[triangle], corners[triangle + 1]] {
                            indices.push(u32::try_from(vertices.len())?);
                            vertices.push(corner);
                        }
                    }
                }
                _ => {}
            }
        }

        line_buffer.clear();
    }

    Ok(Mesh {
        vertices,
        indices,
        materials,
    })
}

fn parse_corner(
    token: &str,
    positions: &[Vector3],
    tex_coords: &[Vector2],
    normals: &[Vector3],
    material_index: usize,
) -> Result<Vertex, Box<dyn std::error::Error>> {
    let mut fields = token.split('/');
    let position_index = parse_obj_index(fields.next().ok_or("Missing vertex index")?)?;
    let tex_coord_index = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(parse_obj_index)
        .transpose()?;
    let normal_index = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(parse_obj_index)
        .transpose()?;
    let mut vertex = Vertex::new(
        *positions
            .get(position_index)
            .ok_or("Vertex index is out of bounds")?,
    );
    vertex.tex_coord = tex_coord_index
        .map(|index| {
            tex_coords
                .get(index)
                .copied()
                .ok_or("Texture index is out of bounds")
        })
        .transpose()?;
    vertex.normal = normal_index
        .map(|index| {
            normals
                .get(index)
                .copied()
                .ok_or("Normal index is out of bounds")
        })
        .transpose()?
        .unwrap_or_else(|| normalize(vertex.position));
    vertex.material_index = material_index;
    Ok(vertex)
}

fn normalize(vector: Vector3) -> Vector3 {
    let length = vector.length();
    if length > f32::EPSILON {
        vector * (1.0 / length)
    } else {
        Vector3::zero()
    }
}

fn parse_obj_index(value: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let index: usize = value.parse()?;
    Ok(index.checked_sub(1).ok_or("OBJ indices start at 1")?)
}

fn load_mtl(path: &Path) -> Result<Vec<(String, Material)>, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(path)?);
    let mut materials = Vec::new();
    let mut current_name: Option<String> = None;
    let mut texture_path: Option<String> = None;

    let finish_material =
        |materials: &mut Vec<(String, Material)>, name: Option<String>, texture: Option<String>| {
            if let Some(name) = name {
                let diffuse_texture = texture.and_then(|texture| {
                    let path = path.parent().unwrap_or(Path::new("")).join(texture);
                    image::open(path).ok().map(|image| image.to_rgba8())
                });
                materials.push((name, Material { diffuse_texture }));
            }
        };

    for line in reader.lines() {
        let line = line?;
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("newmtl") => {
                finish_material(&mut materials, current_name.take(), texture_path.take());
                current_name = Some(parts.collect::<Vec<_>>().join(" "));
            }
            Some("map_Kd") => texture_path = Some(parts.collect::<Vec<_>>().join(" ")),
            _ => {}
        }
    }
    finish_material(&mut materials, current_name, texture_path);
    Ok(materials)
}
