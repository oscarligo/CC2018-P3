use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::load_objects::Mesh;
use crate::performance::FrameMetrics;
use crate::projection::project_vertex;
use crate::rasterizer::rasterize_indexed;
use crate::shaders::{fragment_shader, vertex_shader};
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use std::time::Instant;

#[derive(Default)]
pub struct Renderer {
    transformed_vertices: Vec<Vertex>,
    fragments: Vec<Fragment>,
    metrics: FrameMetrics,
}

impl Renderer {
    pub fn begin_frame(&mut self) {
        self.metrics = FrameMetrics::default();
    }

    pub fn metrics(&self) -> FrameMetrics {
        self.metrics
    }

    pub fn render(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms, mesh: &Mesh) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;

        // 1. Vertex Processing
        let stage_start = Instant::now();
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
        self.metrics.vertex += stage_start.elapsed();

        // 2. Primitive assembly and rasterization
        let stage_start = Instant::now();
        rasterize_indexed(
            &self.transformed_vertices,
            &mesh.indices,
            framebuffer.width,
            framebuffer.height,
            &mut self.fragments,
        );
        self.metrics.rasterization += stage_start.elapsed();
        self.metrics.triangles += mesh.indices.len() / 3;
        self.metrics.fragment_count += self.fragments.len();

        // 3. Fragment Processing
        let stage_start = Instant::now();
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
        self.metrics.fragments += stage_start.elapsed();
    }
}
