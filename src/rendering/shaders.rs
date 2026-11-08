use crate::rendering::fragment::Fragment;
use crate::rendering::load_objects::Material;
use crate::rendering::uniforms::Uniforms;
use crate::rendering::vertex::Vertex;
use raylib::prelude::*;

// Shader constants
const TRANSPARENCY_CUTOFF: u8 = 16;
const COLOR_CHANNEL_MAX: f32 = 255.0;
const AMBIENT_LIGHT: f32 = 0.06;

fn multiply_matrix_vector4(matrix: &Matrix, vector: &Vector4) -> Vector4 {
    Vector4::new(
        matrix.m0 * vector.x + matrix.m4 * vector.y + matrix.m8 * vector.z + matrix.m12 * vector.w,
        matrix.m1 * vector.x + matrix.m5 * vector.y + matrix.m9 * vector.z + matrix.m13 * vector.w,
        matrix.m2 * vector.x + matrix.m6 * vector.y + matrix.m10 * vector.z + matrix.m14 * vector.w,
        matrix.m3 * vector.x + matrix.m7 * vector.y + matrix.m11 * vector.z + matrix.m15 * vector.w,
    )
}

fn normalized_or_zero(vector: Vector3) -> Vector3 {
    let length = vector.length();
    if length > f32::EPSILON {
        vector * (1.0 / length)
    } else {
        Vector3::zero()
    }
}

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let position = Vector4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let transformed = multiply_matrix_vector4(&uniforms.model_matrix, &position);
    let mut result = *vertex;
    result.transformed_position = Vector3::new(transformed.x, transformed.y, transformed.z);
    result
}

pub fn lit_vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let mut result = vertex_shader(vertex, uniforms);
    let Some(light_position) = uniforms.light_position else {
        return result;
    };
    let normal = multiply_matrix_vector4(
        &uniforms.model_matrix,
        &Vector4::new(vertex.normal.x, vertex.normal.y, vertex.normal.z, 0.0),
    );
    let normal = normalized_or_zero(Vector3::new(normal.x, normal.y, normal.z));
    let light_direction = normalized_or_zero(light_position - result.transformed_position);
    let diffuse = normal.dot(light_direction).max(0.0).powf(0.65);
    let intensity = AMBIENT_LIGHT + diffuse * (1.0 - AMBIENT_LIGHT);
    result.color = vertex.color * intensity;
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
    if pixel[3] < TRANSPARENCY_CUTOFF {
        return None;
    }
    let texture_color = Vector3::new(
        pixel[0] as f32 / COLOR_CHANNEL_MAX,
        pixel[1] as f32 / COLOR_CHANNEL_MAX,
        pixel[2] as f32 / COLOR_CHANNEL_MAX,
    );
    Some(Vector3::new(
        texture_color.x * fragment.color.x,
        texture_color.y * fragment.color.y,
        texture_color.z * fragment.color.z,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniforms(light_position: Vector3) -> Uniforms {
        Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: Some(light_position),
        }
    }

    #[test]
    fn lit_vertices_use_the_world_light_direction() {
        let mut front = Vertex::new(Vector3::zero());
        front.normal = Vector3::new(0.0, 0.0, 1.0);
        let mut back = front;
        back.normal = -front.normal;
        let uniforms = uniforms(Vector3::new(0.0, 0.0, 10.0));

        let front = lit_vertex_shader(&front, &uniforms);
        let back = lit_vertex_shader(&back, &uniforms);

        assert!(front.color.x > back.color.x * 10.0);
        assert!((back.color.x - AMBIENT_LIGHT).abs() < 0.001);
    }
}
