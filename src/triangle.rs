use crate::fragment::Fragment;
use crate::vertex::Vertex;
use raylib::prelude::Vector2;

// Triangle constants
const EPSILON: f32 = 1e-5;
const PIXEL_CENTER_OFFSET: f32 = 0.5;

fn positions(a: &Vertex, b: &Vertex, c: &Vertex) -> Option<[Vector2; 3]> {
    Some([a.screen_position?, b.screen_position?, c.screen_position?])
}

fn edge(a: Vector2, b: Vector2, point: Vector2) -> f32 {
    (point.x - a.x) * (b.y - a.y) - (point.y - a.y) * (b.x - a.x)
}

pub(crate) fn is_back_facing(a: &Vertex, b: &Vertex, c: &Vertex) -> bool {
    positions(a, b, c).is_some_and(|points| edge(points[1], points[2], points[0]) > EPSILON)
}

fn fragment_at(position: Vector2, vertices: [&Vertex; 3], weights: [f32; 3]) -> Fragment {
    let inverse_depth = weights[0] / vertices[0].camera_depth
        + weights[1] / vertices[1].camera_depth
        + weights[2] / vertices[2].camera_depth;
    let corrected = [
        weights[0] / vertices[0].camera_depth / inverse_depth,
        weights[1] / vertices[1].camera_depth / inverse_depth,
        weights[2] / vertices[2].camera_depth / inverse_depth,
    ];
    let color = vertices[0].color * corrected[0]
        + vertices[1].color * corrected[1]
        + vertices[2].color * corrected[2];
    let depth = 1.0 / inverse_depth;
    let tex_coord = match (
        vertices[0].tex_coord,
        vertices[1].tex_coord,
        vertices[2].tex_coord,
    ) {
        (Some(a), Some(b), Some(c)) => Some(a * corrected[0] + b * corrected[1] + c * corrected[2]),
        _ => None,
    };

    Fragment {
        position,
        color,
        depth,
        tex_coord,
        material_index: vertices[0].material_index,
    }
}

fn barycentric(point: Vector2, positions: [Vector2; 3], area: f32) -> [f32; 3] {
    [
        edge(positions[1], positions[2], point) / area,
        edge(positions[2], positions[0], point) / area,
        edge(positions[0], positions[1], point) / area,
    ]
}

pub fn triangle_scanline(
    a: &Vertex,
    b: &Vertex,
    c: &Vertex,
    width: u32,
    height: u32,
    emit: &mut impl FnMut(Fragment),
) -> usize {
    let Some(points) = positions(a, b, c) else {
        return 0;
    };
    let area = edge(points[1], points[2], points[0]);
    if area.abs() <= EPSILON {
        return 0;
    }

    if width == 0 || height == 0 {
        return 0;
    }

    let viewport_width = i32::try_from(width).unwrap_or(i32::MAX);
    let viewport_height = i32::try_from(height).unwrap_or(i32::MAX);
    let min_y = (points
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32)
        .clamp(0, viewport_height);
    let max_y = (points
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32)
        .clamp(0, viewport_height);
    let edges = [
        (points[0], points[1]),
        (points[1], points[2]),
        (points[2], points[0]),
    ];
    let mut fragment_count = 0;

    for y in min_y..max_y {
        let sample_y = y as f32 + PIXEL_CENTER_OFFSET;
        let mut intersections = [0.0; 2];
        let mut intersection_count = 0;

        for (start, end) in edges {
            let crosses = (start.y <= sample_y && sample_y < end.y)
                || (end.y <= sample_y && sample_y < start.y);
            if crosses {
                let t = (sample_y - start.y) / (end.y - start.y);
                if intersection_count < intersections.len() {
                    intersections[intersection_count] = start.x + t * (end.x - start.x);
                    intersection_count += 1;
                }
            }
        }

        if intersection_count != 2 {
            continue;
        }
        if intersections[0] > intersections[1] {
            intersections.swap(0, 1);
        }
        let min_x =
            ((intersections[0] - PIXEL_CENTER_OFFSET).ceil() as i32).clamp(0, viewport_width - 1);
        let max_x =
            ((intersections[1] - PIXEL_CENTER_OFFSET).floor() as i32).clamp(0, viewport_width - 1);

        if min_x > max_x {
            continue;
        }

        for x in min_x..=max_x {
            let center = Vector2::new(x as f32 + PIXEL_CENTER_OFFSET, sample_y);
            let weights = barycentric(center, points, area);
            if weights.iter().all(|weight| *weight >= -EPSILON) {
                emit(fragment_at(center, [a, b, c], weights));
                fragment_count += 1;
            }
        }
    }

    fragment_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32, y: f32) -> Vertex {
        let mut vertex = Vertex::new(Default::default());
        vertex.screen_position = Some(Vector2::new(x, y));
        vertex
    }

    #[test]
    fn culls_only_clockwise_screen_winding() {
        let a = vertex(0.0, 0.0);
        let b = vertex(0.0, 1.0);
        let c = vertex(1.0, 0.0);

        assert!(is_back_facing(&a, &b, &c));
        assert!(!is_back_facing(&a, &c, &b));
    }

    #[test]
    fn emits_fragments_without_an_intermediate_buffer() {
        let mut a = vertex(0.0, 0.0);
        let mut b = vertex(0.0, 2.0);
        let mut c = vertex(2.0, 0.0);
        a.camera_depth = 1.0;
        b.camera_depth = 1.0;
        c.camera_depth = 1.0;
        let mut emitted = 0;

        let count = triangle_scanline(&a, &c, &b, 3, 3, &mut |_| emitted += 1);

        assert_eq!(count, emitted);
        assert!(count > 0);
    }
}
