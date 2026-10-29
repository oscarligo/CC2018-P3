use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::load_objects::{project_vertex, Mesh};
use crate::rasterizer::{rasterize_indexed, TriangleMode};
use crate::shaders::{fragment_shader, vertex_shader};
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;

pub fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    mesh: &Mesh,
    mode: TriangleMode,
    transformed_vertices: &mut Vec<Vertex>,
    fragments: &mut Vec<Fragment>,
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;

    // 1. Vertex Processing
    transformed_vertices.clear();
    transformed_vertices.extend(mesh.vertices.iter().map(|vertex| {
        let mut transformed = vertex_shader(vertex, uniforms);
        transformed.screen_position = project_vertex(
            transformed.transformed_position,
            uniforms.camera_pos,
            90.0,
            width,
            height,
        );
        transformed
    }));

    // 2. Primitive assembly and rasterization
    rasterize_indexed(transformed_vertices, &mesh.indices, mode, fragments);

    // 3. Fragment Processing
    for fragment in fragments.iter() {
        let color = fragment_shader(fragment);

        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            color,
        );
    }
}
