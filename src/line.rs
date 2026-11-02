use crate::fragment::Fragment;
use crate::vertex::Vertex;
use raylib::prelude::Vector2;

// bresenham's line algorithm
pub fn line(start: &Vertex, end: &Vertex, width: u32, height: u32) -> Vec<Fragment> {
    let (Some(start_position), Some(end_position)) = (start.screen_position, end.screen_position)
    else {
        return Vec::new();
    };
    let Some((start_position, end_position, start_t, end_t)) =
        clip_to_viewport(start_position, end_position, width, height)
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
    let steps = dx.max(dy).max(1) as f32;
    let start_depth = start.camera_depth + (end.camera_depth - start.camera_depth) * start_t;
    let end_depth = start.camera_depth + (end.camera_depth - start.camera_depth) * end_t;
    let mut step = 0.0;

    loop {
        fragments.push(Fragment {
            position: Vector2::new(x0 as f32, y0 as f32),
            color: start.color,
            depth: start_depth + (end_depth - start_depth) * (step / steps),
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
        step += 1.0;
    }

    fragments
}

fn clip_to_viewport(
    start: Vector2,
    end: Vector2,
    width: u32,
    height: u32,
) -> Option<(Vector2, Vector2, f32, f32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let delta = end - start;
    let bounds = [
        (-delta.x, start.x),
        (delta.x, width as f32 - 1.0 - start.x),
        (-delta.y, start.y),
        (delta.y, height as f32 - 1.0 - start.y),
    ];
    let mut enter = 0.0_f32;
    let mut exit = 1.0_f32;

    for (direction, distance) in bounds {
        if direction.abs() <= f32::EPSILON {
            if distance < 0.0 {
                return None;
            }
            continue;
        }

        let amount = distance / direction;
        if direction < 0.0 {
            enter = enter.max(amount);
        } else {
            exit = exit.min(amount);
        }
        if enter > exit {
            return None;
        }
    }

    Some((start + delta * enter, start + delta * exit, enter, exit))
}
