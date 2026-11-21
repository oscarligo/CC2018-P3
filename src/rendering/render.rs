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

type FragmentShader<'a> = &'a dyn Fn(&Fragment) -> Option<Vector3>;
const FOG_START: f32 = 2_500.0;
const FOG_END: f32 = 14_000.0;
const FOG_MAX_STRENGTH: f32 = 0.82;
const HIGHLIGHT_FOG_PROTECTION: f32 = 0.78;

fn apply_depth_fog(color: Vector3, depth: f32, fog_color: Vector3) -> Vector3 {
    let t = ((depth - FOG_START) / (FOG_END - FOG_START)).clamp(0.0, 1.0);
    let luminance = color.x * 0.2126 + color.y * 0.7152 + color.z * 0.0722;
    let highlight = ((luminance - 0.45) / 0.45).clamp(0.0, 1.0);
    let highlight = highlight * highlight * (3.0 - 2.0 * highlight);
    let strength =
        t * t * (3.0 - 2.0 * t) * FOG_MAX_STRENGTH * (1.0 - highlight * HIGHLIGHT_FOG_PROTECTION);
    color * (1.0 - strength) + fog_color * strength
}

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
        let shade = |fragment: &Fragment| {
            Some(shader(
                fragment.surface.expect("surface attributes requested"),
            ))
        };
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

    pub fn render_colored_shaded(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        mesh: &Mesh,
        colors: &[Vector3],
        shader: impl Fn(&Fragment) -> Option<Vector3>,
    ) {
        assert_eq!(colors.len(), mesh.vertices.len());
        self.render_internal(
            framebuffer,
            uniforms,
            mesh,
            Some(colors),
            None,
            RenderMode::Opaque,
            Some(&shader),
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
        surface_shader: Option<FragmentShader<'_>>,
    ) {
        let transparent = mode == RenderMode::Transparent;
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        let fog_color = framebuffer.background_rgb();

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
                    |shader| shader(&fragment),
                );
                let Some(color) = color else {
                    return;
                };
                let color = apply_depth_fog(color, fragment.depth, fog_color);
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
    fn depth_fog_preserves_near_colors_and_blends_distant_fragments() {
        let color = Vector3::new(0.35, 0.2, 0.1);
        let fog = Vector3::new(0.02, 0.04, 0.08);
        assert_eq!(apply_depth_fog(color, FOG_START, fog), color);

        let middle = apply_depth_fog(color, (FOG_START + FOG_END) * 0.5, fog);
        let far = apply_depth_fog(color, FOG_END, fog);
        assert!(middle.x < color.x && middle.x > far.x);
        assert!(middle.z > color.z.min(fog.z));
        assert_eq!(
            far,
            color * (1.0 - FOG_MAX_STRENGTH) + fog * FOG_MAX_STRENGTH
        );
    }

    #[test]
    fn bright_surfaces_remain_visible_through_distant_fog() {
        let fog = Vector3::new(0.02, 0.04, 0.08);
        let dark = apply_depth_fog(Vector3::new(0.3, 0.3, 0.3), FOG_END, fog);
        let bright = apply_depth_fog(Vector3::one(), FOG_END, fog);

        assert!(bright.x > 0.75);
        assert!(bright.x - fog.x > (dark.x - fog.x) * 8.0);
    }

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
