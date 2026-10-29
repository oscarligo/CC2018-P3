use crate::fragment::Fragment;
use crate::load_objects::Material;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use raylib::prelude::*;

fn multiply_matrix_vector4(matrix: &Matrix, vector: &Vector4) -> Vector4 {
    Vector4::new(
        matrix.m0 * vector.x + matrix.m4 * vector.y + matrix.m8 * vector.z + matrix.m12 * vector.w,
        matrix.m1 * vector.x + matrix.m5 * vector.y + matrix.m9 * vector.z + matrix.m13 * vector.w,
        matrix.m2 * vector.x + matrix.m6 * vector.y + matrix.m10 * vector.z + matrix.m14 * vector.w,
        matrix.m3 * vector.x + matrix.m7 * vector.y + matrix.m11 * vector.z + matrix.m15 * vector.w,
    )
}

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let position = Vector4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let transformed = multiply_matrix_vector4(&uniforms.model_matrix, &position);

    let mut result = *vertex;
    result.transformed_position = Vector3::new(transformed.x, transformed.y, transformed.z);
    result
}

pub fn fragment_shader(fragment: &Fragment, materials: &[Material]) -> Option<Vector3> {
    let Some(texture) = materials
        .get(fragment.material_index)
        .and_then(|material| material.diffuse_texture.as_ref())
    else {
        return Some(fragment.color);
    };
    let Some(uv) = fragment.tex_coord else {
        return Some(fragment.color);
    };

    let x = (uv.x.clamp(0.0, 1.0) * (texture.width() - 1) as f32) as u32;
    let y = ((1.0 - uv.y).clamp(0.0, 1.0) * (texture.height() - 1) as f32) as u32;
    let pixel = texture.get_pixel(x, y).0;
    if pixel[3] < 16 {
        return None;
    }
    Some(Vector3::new(
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
    ))
}
