use crate::framebuffer::Framebuffer;
use crate::load_objects::Mesh;
use crate::performance::FrameMetrics;
use crate::projection::{camera_depth, project_vertex};
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

    pub fn record_object(&mut self, culled: bool) {
        self.metrics.objects += 1;
        if culled {
            self.metrics.culled_objects += 1;
        }
    }

    pub fn record_surface_update(&mut self, elapsed: std::time::Duration) {
        self.metrics.surface_updates += 1;
        self.metrics.surface_update += elapsed;
    }

    pub fn record_lod_switch(&mut self) {
        self.metrics.lod_switches += 1;
    }

    pub fn render(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms, mesh: &Mesh) {
        self.render_internal(framebuffer, uniforms, mesh, None);
    }

    pub fn render_colored(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: &[raylib::prelude::Vector3],
    ) {
        assert_eq!(colors.len(), mesh.vertices.len());
        self.render_internal(framebuffer, uniforms, mesh, Some(colors));
    }

    pub fn render_colored_transparent(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: &[raylib::prelude::Vector3],
        alpha: f32,
    ) {
        assert_eq!(colors.len(), mesh.vertices.len());
        self.render_internal_with_alpha(framebuffer, uniforms, mesh, Some(colors), alpha);
    }

    fn render_internal(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: Option<&[raylib::prelude::Vector3]>,
    ) {
        self.render_internal_with_alpha(framebuffer, uniforms, mesh, colors, 1.0);
    }

    fn render_internal_with_alpha(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: Option<&[raylib::prelude::Vector3]>,
        alpha: f32,
    ) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;

        // 1. Vertex Processing
        let stage_start = Instant::now();
        self.transformed_vertices.clear();
        self.transformed_vertices
            .extend(mesh.vertices.iter().enumerate().map(|(index, vertex)| {
                let mut transformed = vertex_shader(vertex, uniforms);
                if let Some(colors) = colors {
                    transformed.color = colors[index];
                }
                transformed.camera_depth = camera_depth(transformed.transformed_position, uniforms)
                    .unwrap_or(f32::INFINITY);
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
            uniforms,
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
                framebuffer.blend_point(x, y, fragment.depth, color, alpha, alpha >= 1.0);
            },
        );
        self.metrics.rasterization += stage_start.elapsed();
        self.metrics.triangles += mesh.indices.len() / 3;
        self.metrics.culled_triangles += stats.culled_triangles;
        self.metrics.near_clipped_triangles += stats.near_clipped_triangles;
        self.metrics.near_rejected_triangles += stats.near_rejected_triangles;
        self.metrics.covered_fragments += stats.covered_fragments;
        self.metrics.depth_rejected += depth_rejected;
        self.metrics.shaded_fragments += shaded_fragments;
    }
}
