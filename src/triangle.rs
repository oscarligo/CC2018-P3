use crate::fragment::Fragment;
use crate::line::line;
use crate::vertex::Vertex;
use raylib::prelude::Vector2;

// Triangle constants
const EPSILON: f32 = 1e-5;
const PIXEL_CENTER_OFFSET: f32 = 0.5;
const SCANLINE_INTERSECTION_CAPACITY: usize = 2;

pub fn triangle(a: &Vertex, b: &Vertex, c: &Vertex) -> Vec<Fragment> {
    let mut fragments = line(a, b);
    fragments.extend(line(b, c));
    fragments.extend(line(c, a));
    fragments
}

fn positions(a: &Vertex, b: &Vertex, c: &Vertex) -> Option<[Vector2; 3]> {
    Some([a.screen_position?, b.screen_position?, c.screen_position?])
}

fn edge(a: Vector2, b: Vector2, point: Vector2) -> f32 {
    (point.x - a.x) * (b.y - a.y) - (point.y - a.y) * (b.x - a.x)
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

pub fn triangle_barycentric(a: &Vertex, b: &Vertex, c: &Vertex) -> Vec<Fragment> {
    let Some(points) = positions(a, b, c) else {
        return Vec::new();
    };
    let area = edge(points[1], points[2], points[0]);
    if area.abs() <= EPSILON {
        return Vec::new();
    }

    let min_x = points
        .iter()
        .map(|p| p.x)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_x = points
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    let min_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    let mut fragments = Vec::new();

    for y in min_y..max_y {
        for x in min_x..max_x {
            let center = Vector2::new(
                x as f32 + PIXEL_CENTER_OFFSET,
                y as f32 + PIXEL_CENTER_OFFSET,
            );
            let weights = barycentric(center, points, area);
            if weights.iter().all(|weight| *weight >= -EPSILON) {
                fragments.push(fragment_at(center, [a, b, c], weights));
            }
        }
    }

    fragments
}

pub fn triangle_scanline(a: &Vertex, b: &Vertex, c: &Vertex) -> Vec<Fragment> {
    let Some(points) = positions(a, b, c) else {
        return Vec::new();
    };
    let area = edge(points[1], points[2], points[0]);
    if area.abs() <= EPSILON {
        return Vec::new();
    }

    let min_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    let edges = [
        (points[0], points[1]),
        (points[1], points[2]),
        (points[2], points[0]),
    ];
    let mut fragments = Vec::new();

    for y in min_y..max_y {
        let sample_y = y as f32 + PIXEL_CENTER_OFFSET;
        let mut intersections = Vec::with_capacity(SCANLINE_INTERSECTION_CAPACITY);

        for (start, end) in edges {
            let crosses = (start.y <= sample_y && sample_y < end.y)
                || (end.y <= sample_y && sample_y < start.y);
            if crosses {
                let t = (sample_y - start.y) / (end.y - start.y);
                intersections.push(start.x + t * (end.x - start.x));
            }
        }

        if intersections.len() != 2 {
            continue;
        }
        intersections.sort_by(f32::total_cmp);
        let min_x = (intersections[0] - PIXEL_CENTER_OFFSET).ceil() as i32;
        let max_x = (intersections[1] - PIXEL_CENTER_OFFSET).floor() as i32;

        for x in min_x..=max_x {
            let center = Vector2::new(x as f32 + PIXEL_CENTER_OFFSET, sample_y);
            let weights = barycentric(center, points, area);
            if weights.iter().all(|weight| *weight >= -EPSILON) {
                fragments.push(fragment_at(center, [a, b, c], weights));
            }
        }
    }

    fragments
}
