use raylib::prelude::*;
use crate::framebuffer::Framebuffer;

// flood fill algorithm

pub fn flood_fill(framebuffer: &mut Framebuffer, point: Vector2, fill_color: Color) {
    let x = point.x as u32;
    let y = point.y as u32;

    framebuffer.set_current_color(fill_color);

    // Get the color of the pixel at the starting point
    let target_color = framebuffer.get_pixel(x, y);

    if target_color == fill_color {
        return;
    }

    // Use a stack to keep track of pixels to fill
    let mut stack: Vec<(u32, u32)> = vec![(x, y)];   

    while let Some((x, y)) = stack.pop() {
        if x >= framebuffer.width || y >= framebuffer.height {
            continue;
        }

        let current_color = framebuffer.get_pixel(x, y);

        if current_color != target_color {
            continue;
        }

        framebuffer.set_pixel(x, y);

        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }
}