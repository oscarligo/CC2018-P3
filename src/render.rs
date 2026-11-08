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
        let mut depth_rejected = 0;
        let mut shaded_fragments = 0;
        let stats = rasterize_indexed(
            &self.transformed_vertices,
            &mesh.indices,
            framebuffer.width,
            framebuffer.height,
            |fragment| {
                let x = fragment.position.x as i32;
                let y = fragment.position.y as i32;
                if !framebuffer.is_closer(x, y, fragment.depth) {
                    depth_rejected += 1;
                    return;
                }
                let Some(color) = fragment_shader(&fragment, &mesh.materials) else {
                    return;
                };
                shaded_fragments += 1;
                framebuffer.point(x, y, fragment.depth, color);
            },
        );
        self.metrics.rasterization += stage_start.elapsed();
        self.metrics.triangles += mesh.indices.len() / 3;
        self.metrics.culled_triangles += stats.culled_triangles;
        self.metrics.covered_fragments += stats.covered_fragments;
        self.metrics.depth_rejected += depth_rejected;
        self.metrics.shaded_fragments += shaded_fragments;
    }
}
