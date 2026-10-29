use raylib::prelude::*;

const COLOR_CHANNEL_MAX: f32 = 255.0;
const OPAQUE_ALPHA: u8 = 255;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub color_buffer: Image,
    depth_buffer: Vec<f32>,
    background_color: Color,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, background_color: Color) -> Self {
        let color_buffer = Image::gen_image_color(width as i32, height as i32, background_color);
        Framebuffer {
            width,
            height,
            color_buffer,
            depth_buffer: vec![f32::INFINITY; (width * height) as usize],
            background_color,
        }
    }

    pub fn clear(&mut self) {
        self.color_buffer =
            Image::gen_image_color(self.width as i32, self.height as i32, self.background_color);
        self.depth_buffer.fill(f32::INFINITY);
    }

    pub fn point(&mut self, x: i32, y: i32, depth: f32, color: Vector3) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = y as usize * self.width as usize + x as usize;
            if depth >= self.depth_buffer[index] {
                return;
            }
            self.depth_buffer[index] = depth;
            let color = Color::new(
                (color.x.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                (color.y.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                (color.z.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                OPAQUE_ALPHA,
            );
            self.color_buffer.draw_pixel(x, y, color);
        }
    }

    pub fn swap_buffers(&mut self, window: &mut RaylibHandle, raylib_thread: &RaylibThread) {
        if let Ok(texture) = window.load_texture_from_image(raylib_thread, &self.color_buffer) {
            let mut renderer = window.begin_drawing(raylib_thread);

            renderer.draw_texture(&texture, 0, 0, Color::WHITE);
        }
    }
}
