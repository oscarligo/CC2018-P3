use raylib::prelude::*;

const COLOR_CHANNEL_MAX: f32 = 255.0;
const OPAQUE_ALPHA: u8 = 255;
const CHANNELS_PER_PIXEL: usize = 4;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
    depth_buffer: Vec<f32>,
    background_color: Color,
    texture: Option<Texture2D>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, background_color: Color) -> Self {
        let mut framebuffer = Framebuffer {
            width,
            height,
            pixels: vec![0; (width * height) as usize * CHANNELS_PER_PIXEL],
            depth_buffer: vec![f32::INFINITY; (width * height) as usize],
            background_color,
            texture: None,
        };
        framebuffer.clear();
        framebuffer
    }

    pub fn clear(&mut self) {
        for pixel in self.pixels.chunks_exact_mut(CHANNELS_PER_PIXEL) {
            pixel.copy_from_slice(&[
                self.background_color.r,
                self.background_color.g,
                self.background_color.b,
                self.background_color.a,
            ]);
        }
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
            let pixel = &mut self.pixels[index * CHANNELS_PER_PIXEL..][..CHANNELS_PER_PIXEL];
            pixel.copy_from_slice(&[color.r, color.g, color.b, color.a]);
        }
    }

    pub fn is_closer(&self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return false;
        }
        depth < self.depth_buffer[y as usize * self.width as usize + x as usize]
    }

    pub fn swap_buffers(&mut self, window: &mut RaylibHandle, raylib_thread: &RaylibThread) {
        if self.texture.is_none() {
            let image = Image::gen_image_color(
                self.width as i32,
                self.height as i32,
                self.background_color,
            );
            self.texture = window.load_texture_from_image(raylib_thread, &image).ok();
        }
        let Some(texture) = &mut self.texture else {
            return;
        };
        if texture.update_texture(&self.pixels).is_err() {
            return;
        }

        let mut renderer = window.begin_drawing(raylib_thread);
        renderer.draw_texture(texture, 0, 0, Color::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_reuses_pixel_storage() {
        let mut framebuffer = Framebuffer::new(4, 3, Color::new(1, 2, 3, 255));
        let pixels = framebuffer.pixels.as_ptr();
        framebuffer.point(1, 1, 1.0, Vector3::new(1.0, 0.0, 0.0));

        framebuffer.clear();

        assert_eq!(pixels, framebuffer.pixels.as_ptr());
        assert!(framebuffer
            .pixels
            .chunks_exact(CHANNELS_PER_PIXEL)
            .all(|pixel| pixel == [1, 2, 3, 255]));
    }
}
