use crate::performance::FrameMetrics;
use crate::rendering::fragment::{Fragment, SurfacePoint};
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::Mesh;
use crate::rendering::projection::{camera_depth, project_vertex};
use crate::rendering::rasterizer::rasterize_indexed;
use crate::rendering::shaders::{fragment_shader, lit_vertex_shader, vertex_shader};
use crate::rendering::uniforms::Uniforms;
use crate::rendering::vertex::Vertex;
use raylib::prelude::Vector3;
use std::time::Instant;

#[derive(Clone, Copy, Eq, PartialEq)]
enum RenderMode {
    Opaque,
    Lit,
    LitFill,
    Transparent,
}

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
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            None,
            None,
            RenderMode::Opaque,
            None,
        );
    }

    pub fn render_lit(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms, mesh: &Mesh) {
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            None,
            None,
            RenderMode::Lit,
            None,
        );
    }

    /// Diffuse light plus a soft ambient fill for life visible in Ureski's permanent dusk.
    pub fn render_lit_fill(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
    ) {
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            None,
            None,
            RenderMode::LitFill,
            None,
        );
    }

    pub fn render_colored(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: &[raylib::prelude::Vector3],
    ) {
        assert_eq!(colors.len(), mesh.vertices.len());
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            Some(colors),
            None,
            RenderMode::Opaque,
            None,
        );
    }

    pub fn render_transparent(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
    ) {
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            None,
            None,
            RenderMode::Transparent,
            None,
        );
    }

    pub fn render_colored_transparent(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: &[raylib::prelude::Vector3],
        alphas: &[f32],
    ) {
        assert_eq!(colors.len(), mesh.vertices.len());
        assert_eq!(alphas.len(), mesh.vertices.len());
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            Some(colors),
            Some(alphas),
            RenderMode::Transparent,
            None,
        );
    }

    /// Evalúa un shader CPU por fragmento visible, después del z-buffer.
    pub fn render_surface(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        shader: impl Fn(SurfacePoint) -> Vector3,
    ) {
        let shade =
            |fragment: &Fragment| shader(fragment.surface.expect("surface attributes requested"));
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            None,
            None,
            RenderMode::Opaque,
            Some(&shade),
        );
    }

    fn render_internal(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: Option<&[raylib::prelude::Vector3]>,
        alphas: Option<&[f32]>,
        mode: RenderMode,
        surface_shader: Option<&dyn Fn(&Fragment) -> Vector3>,
    ) {
        let transparent = mode == RenderMode::Transparent;
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;

        // 1. Vertex Processing
        let stage_start = Instant::now();
        self.transformed_vertices.clear();
        self.transformed_vertices
            .extend(mesh.vertices.iter().enumerate().map(|(index, vertex)| {
                let mut transformed = if matches!(mode, RenderMode::Lit | RenderMode::LitFill) {
                    lit_vertex_shader(vertex, uniforms)
                } else {
                    vertex_shader(vertex, uniforms)
                };
                if mode == RenderMode::LitFill {
                    transformed.color = transformed.color * 0.62 + vertex.color * 0.38;
                }
                if let Some(colors) = colors {
                    transformed.color = colors[index];
                }
                if let Some(alphas) = alphas {
                    transformed.alpha = alphas[index];
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
            transparent,
            surface_shader.is_some(),
            |fragment| {
                let x = fragment.position.x as i32;
                let y = fragment.position.y as i32;
                if !framebuffer.is_closer(x, y, fragment.depth) {
                    depth_rejected += 1;
                    return;
                }
                let color = surface_shader.map_or_else(
                    || fragment_shader(&fragment, &mesh.materials),
                    |shader| Some(shader(&fragment)),
                );
                let Some(color) = color else {
                    return;
                };
                shaded_fragments += 1;
                let alpha = if transparent { fragment.alpha } else { 1.0 };
                framebuffer.blend_point(x, y, fragment.depth, color, alpha, !transparent);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::load_objects::Material;
    use raylib::prelude::{Color, Matrix};
    use std::cell::Cell;

    #[test]
    fn surface_shader_runs_per_visible_fragment_and_respects_depth() {
        let mesh = Mesh {
            vertices: [
                Vector3::new(-0.8, -0.8, 2.0),
                Vector3::new(0.0, 0.8, 4.0),
                Vector3::new(0.8, -0.8, 2.0),
            ]
            .into_iter()
            .map(Vertex::new)
            .collect(),
            indices: vec![0, 1, 2],
            materials: vec![Material {
                diffuse_texture: None,
            }],
        };
        let uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        let mut renderer = Renderer::default();
        let mut framebuffer = Framebuffer::new(32, 32, Color::BLACK);
        let calls = Cell::new(0);
        renderer.render_surface(&mut framebuffer, &uniforms, &mesh, |surface| {
            calls.set(calls.get() + 1);
            assert!(surface.position.z >= 2.0 && surface.position.z <= 4.0);
            Vector3::new(0.0, 1.0, 0.0)
        });
        assert!(calls.get() > mesh.vertices.len());
        assert_eq!(calls.get(), renderer.metrics().shaded_fragments);
        calls.set(0);
        renderer.render_surface(&mut framebuffer, &uniforms, &mesh, |_| {
            calls.set(calls.get() + 1);
            Vector3::zero()
        });
        assert_eq!(calls.get(), 0);
        assert!(renderer.metrics().depth_rejected > 0);
    }
}
