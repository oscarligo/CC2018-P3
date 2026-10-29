use crate::fragment::Fragment;
use crate::vertex::Vertex;
use raylib::prelude::Vector2;

// bresenham's line algorithm
pub fn line(start: &Vertex, end: &Vertex) -> Vec<Fragment> {
    let (Some(start_position), Some(end_position)) = (start.screen_position, end.screen_position)
    else {
        return Vec::new();
    };

    let mut fragments = Vec::new();
    let mut x0 = start_position.x.round() as i32;
    let mut y0 = start_position.y.round() as i32;
    let x1 = end_position.x.round() as i32;
    let y1 = end_position.y.round() as i32;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = (y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = dx - dy;

    loop {
        fragments.push(Fragment {
            position: Vector2::new(x0 as f32, y0 as f32),
            color: start.color,
            depth: start.camera_depth,
            tex_coord: start.tex_coord, 
            material_index: start.material_index,
        });

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= -dy {
            err -= dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    fragments
}
