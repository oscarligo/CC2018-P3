use crate::vertex::Vertex;
use image::RgbaImage;
use raylib::prelude::{Vector2, Vector3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

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

    let mut positions: Vec<Vector3> = Vec::with_capacity(1024);
    let mut tex_coords: Vec<Vector2> = Vec::with_capacity(1024);
    let mut vertices: Vec<Vertex> = Vec::with_capacity(2048);
    let mut indices: Vec<u32> = Vec::with_capacity(2048);
    let mut materials = vec![Material {
        diffuse_texture: None,
    }];
    let mut material_indices = HashMap::new();
    let mut current_material = 0;
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
                    positions.push(Vector3::new(x, y, z));
                }
                "vt" => {
                    let u: f32 = parts.next().ok_or("Missing texture u")?.parse()?;
                    let v: f32 = parts.next().ok_or("Missing texture v")?.parse()?;
                    tex_coords.push(Vector2::new(u, v));
                }
                "mtllib" => {
                    let filename = parts.collect::<Vec<_>>().join(" ");
                    let mtl_path = obj_path.parent().unwrap_or(Path::new("")).join(filename);
                    let loaded = load_mtl(&mtl_path)?;
                    for (name, material) in loaded {
                        material_indices.insert(name, materials.len());
                        materials.push(material);
                    }
                }
                "usemtl" => {
                    let name = parts.collect::<Vec<_>>().join(" ");
                    current_material = material_indices.get(&name).copied().unwrap_or(0);
                }
                "f" => {
                    let corners = parts
                        .map(|token| parse_corner(token, &positions, &tex_coords, current_material))
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
    material_index: usize,
) -> Result<Vertex, Box<dyn std::error::Error>> {
    let mut fields = token.split('/');
    let position_index = parse_obj_index(fields.next().ok_or("Missing vertex index")?)?;
    let tex_coord_index = fields
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
    vertex.material_index = material_index;
    Ok(vertex)
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

pub fn project_vertex(
    v: Vector3,
    camera_pos: Vector3,
    camera_target: Vector3,
    fov_scale: f32,
    width: f32,
    height: f32,
) -> Option<(Vector2, f32)> {
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

    Some((Vector2::new(x, y), z))
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
    use super::load_obj;

    #[test]
    fn loads_ship_materials_and_texture_coordinates() {
        let mesh = load_obj("assets/ship.obj").unwrap();

        assert_eq!(mesh.materials.len(), 3);
        assert!(mesh.materials[1].diffuse_texture.is_some());
        assert!(mesh.materials[2].diffuse_texture.is_some());
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tex_coord.is_some()));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.material_index == 1));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.material_index == 2));
        assert_eq!(mesh.indices.len() % 3, 0);
    }

    #[test]
    fn loads_red_car_and_dog_textures() {
        let mesh = load_obj("assets/red_car.obj").unwrap();

        assert_eq!(mesh.materials.len(), 3);
        assert!(mesh.materials[1].diffuse_texture.is_some());
        assert!(mesh.materials[2].diffuse_texture.is_some());
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.material_index == 1));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.material_index == 2));
    }
}
