use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::load_objects::Mesh;
use crate::projection::project_vertex;
use crate::rasterizer::rasterize_indexed;
use crate::shaders::{fragment_shader, vertex_shader};
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;

#[derive(Default)]
pub struct Renderer {
    transformed_vertices: Vec<Vertex>,
    fragments: Vec<Fragment>,
}

impl Renderer {
    pub fn render(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms, mesh: &Mesh) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;

        // 1. Vertex Processing
        self.transformed_vertices.clear();
        self.transformed_vertices
            .extend(mesh.vertices.iter().map(|vertex| {
                let mut transformed = vertex_shader(vertex, uniforms);
                if let Some((screen_position, depth)) = project_vertex(
                    transformed.transformed_position,
                    uniforms.camera_pos,
                    uniforms.camera_target,
                    uniforms.camera_up,
                    width,
                    height,
                ) {
                    transformed.screen_position = Some(screen_position);
                    transformed.camera_depth = depth;
                } else {
                    transformed.screen_position = None;
                }
                transformed
            }));

        // 2. Primitive assembly and rasterization
        rasterize_indexed(
            &self.transformed_vertices,
            &mesh.indices,
            framebuffer.width,
            framebuffer.height,
            &mut self.fragments,
        );

        // 3. Fragment Processing
        for fragment in &self.fragments {
            let Some(color) = fragment_shader(fragment, &mesh.materials) else {
                continue;
            };

            framebuffer.point(
                fragment.position.x as i32,
                fragment.position.y as i32,
                fragment.depth,
                color,
            );
        }
    }
}
