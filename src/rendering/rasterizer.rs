use crate::rendering::fragment::Fragment;
use crate::rendering::projection::{NEAR_CLIP_DISTANCE, project_at_depth};
use crate::rendering::triangle::{is_back_facing, triangle_scanline};
use crate::rendering::uniforms::Uniforms;
use crate::rendering::vertex::Vertex;

pub struct RasterizationStats {
    pub culled_triangles: usize,
    pub covered_fragments: usize,
    pub near_clipped_triangles: usize,
    pub near_rejected_triangles: usize,
}

pub fn rasterize_indexed(
    vertices: &[Vertex],
    indices: &[u32],
    width: u32,
    height: u32,
    uniforms: &Uniforms,
    interpolate_alpha: bool,
    interpolate_surface: bool,
    mut emit: impl FnMut(Fragment),
) -> RasterizationStats {
    let mut culled_triangles = 0;
    let mut covered_fragments = 0;
    let mut near_clipped_triangles = 0;
    let mut near_rejected_triangles = 0;

    for indices in indices.chunks_exact(3) {
        let input = [
            vertices[indices[0] as usize],
            vertices[indices[1] as usize],
            vertices[indices[2] as usize],
        ];
        let inside_count = input
            .iter()
            .filter(|vertex| vertex.camera_depth >= NEAR_CLIP_DISTANCE)
            .count();
        if inside_count == 0 {
            near_rejected_triangles += 1;
            continue;
        }
        if inside_count < input.len() {
            near_clipped_triangles += 1;
        }

        let (polygon, polygon_len) = clip_near_plane(input);
        for index in 1..polygon_len - 1 {
            let mut triangle = [polygon[0], polygon[index], polygon[index + 1]];
            for vertex in &mut triangle {
                if vertex.screen_position.is_none() {
                    let Some(position) = project_at_depth(
                        vertex.transformed_position,
                        uniforms.camera_pos,
                        uniforms.camera_target,
                        uniforms.camera_up,
                        width as f32,
                        height as f32,
                        vertex.camera_depth,
                    ) else {
                        continue;
                    };
                    vertex.screen_position = Some(position);
                }
            }
            let [a, b, c] = &triangle;
            if is_back_facing(a, b, c) {
                culled_triangles += 1;
                continue;
            }
            covered_fragments += triangle_scanline(
                a,
                b,
                c,
                width,
                height,
                interpolate_alpha,
                interpolate_surface,
                &mut emit,
            );
        }
    }

    RasterizationStats {
        culled_triangles,
        covered_fragments,
        near_clipped_triangles,
        near_rejected_triangles,
    }
}

fn clip_near_plane(input: [Vertex; 3]) -> ([Vertex; 4], usize) {
    let mut output = [input[0]; 4];
    let mut output_len = 0;
    let mut previous = input[2];
    let mut previous_inside = previous.camera_depth >= NEAR_CLIP_DISTANCE;

    for current in input {
        let current_inside = current.camera_depth >= NEAR_CLIP_DISTANCE;
        if current_inside != previous_inside {
            output[output_len] = near_intersection(previous, current);
            output_len += 1;
        }
        if current_inside {
            output[output_len] = current;
            output_len += 1;
        }
        previous = current;
        previous_inside = current_inside;
    }

    (output, output_len)
}

fn near_intersection(start: Vertex, end: Vertex) -> Vertex {
    let amount =
        (NEAR_CLIP_DISTANCE - start.camera_depth) / (end.camera_depth - start.camera_depth);
    let mut vertex = start;
    vertex.position = start.position + (end.position - start.position) * amount;
    vertex.normal = start.normal + (end.normal - start.normal) * amount;
    vertex.transformed_position = start.transformed_position
        + (end.transformed_position - start.transformed_position) * amount;
    vertex.color = start.color + (end.color - start.color) * amount;
    vertex.alpha = start.alpha + (end.alpha - start.alpha) * amount;
    vertex.tex_coord = match (start.tex_coord, end.tex_coord) {
        (Some(start), Some(end)) => Some(start + (end - start) * amount),
        _ => None,
    };
    vertex.camera_depth = NEAR_CLIP_DISTANCE;
    vertex.screen_position = None;
    vertex
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(depth: f32) -> Vertex {
        let mut vertex = Vertex::new(raylib::prelude::Vector3::new(0.0, 0.0, depth));
        vertex.transformed_position = vertex.position;
        vertex.camera_depth = depth;
        vertex
    }

    #[test]
    fn close_triangles_and_near_plane_intersections_still_render() {
        use raylib::prelude::{Matrix, Vector3};
        let camera = Vector3::new(0.0, 0.0, 6000.0);
        let uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: camera,
            camera_target: camera + Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        for front_depth in [0.02, 0.15] {
            let vertices = [
                (-0.08, -0.08, 0.2),
                (0.08, -0.08, 0.2),
                (0.0, 0.08, front_depth),
            ]
            .map(|(x, y, z)| {
                let mut v = vertex(z);
                v.transformed_position = camera + Vector3::new(x, y, z);
                v
            });
            let mut fragments = 0;
            let stats = rasterize_indexed(
                &vertices,
                &[0, 1, 2, 2, 1, 0],
                80,
                60,
                &uniforms,
                false,
                false,
                |f| {
                    assert!(f.depth.is_finite());
                    fragments += 1;
                },
            );
            assert_eq!(stats.near_rejected_triangles, 0);
            assert!(fragments > 0);
        }
    }

    #[test]
    fn near_clipping_produces_triangle_or_quad() {
        assert_eq!(
            clip_near_plane([vertex(2.0), vertex(0.0), vertex(0.0)]).1,
            3
        );
        assert_eq!(
            clip_near_plane([vertex(2.0), vertex(2.0), vertex(0.0)]).1,
            4
        );
        assert_eq!(
            clip_near_plane([vertex(0.0), vertex(0.0), vertex(0.0)]).1,
            0
        );
    }
}
