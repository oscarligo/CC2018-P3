use raylib::prelude::*;

const COLOR_CHANNEL_MAX: f32 = 255.0;
const OPAQUE_ALPHA: u8 = 255;
const CHANNELS_PER_PIXEL: usize = 4;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
    display_pixels: Vec<u8>,
    depth_buffer: Vec<f32>,
    background_color: Color,
    texture: Option<Texture2D>,
}

impl Framebuffer {
    pub(crate) fn save_png(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), image::ImageError> {
        image::save_buffer(
            path,
            &self.pixels,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
    }
    pub fn new(width: u32, height: u32, background_color: Color) -> Self {
        let mut framebuffer = Framebuffer {
            width,
            height,
            pixels: vec![0; (width * height) as usize * CHANNELS_PER_PIXEL],
            display_pixels: vec![0; (width * height) as usize * CHANNELS_PER_PIXEL],
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

    fn prepare_display_pixels(&mut self) {
        const PIXEL_SIZE: usize = 2;
        self.display_pixels.copy_from_slice(&self.pixels);
        let width = self.width as usize;
        let height = self.height as usize;
        for top in (0..height).step_by(PIXEL_SIZE) {
            for left in (0..width).step_by(PIXEL_SIZE) {
                let mut sum = [0_u16; 4];
                let mut count = 0_u16;
                for y in top..(top + PIXEL_SIZE).min(height) {
                    for x in left..(left + PIXEL_SIZE).min(width) {
                        let start = (y * width + x) * CHANNELS_PER_PIXEL;
                        for channel in 0..4 {
                            sum[channel] += self.pixels[start + channel] as u16;
                        }
                        count += 1;
                    }
                }
                let color = sum.map(|value| (value / count) as u8);
                for y in top..(top + PIXEL_SIZE).min(height) {
                    for x in left..(left + PIXEL_SIZE).min(width) {
                        let start = (y * width + x) * CHANNELS_PER_PIXEL;
                        self.display_pixels[start..start + CHANNELS_PER_PIXEL]
                            .copy_from_slice(&color);
                    }
                }
            }
        }
    }

    pub fn point(&mut self, x: i32, y: i32, depth: f32, color: Vector3) {
        self.blend_point(x, y, depth, color, 1.0, true);
    }

    /// Emissive sprites and laser bloom add light without covering the scene with black quads.
    pub fn add_light(&mut self, x: i32, y: i32, depth: f32, color: Vector3, strength: f32) {
        if strength <= 0.0 || !self.is_closer(x, y, depth) {
            return;
        }
        let index = (y as usize * self.width as usize + x as usize) * CHANNELS_PER_PIXEL;
        let pixel = &mut self.pixels[index..index + CHANNELS_PER_PIXEL];
        for (channel, value) in [color.x, color.y, color.z].into_iter().enumerate() {
            pixel[channel] =
                (pixel[channel] as f32 + value.max(0.0) * strength * 255.0).min(255.0) as u8;
        }
    }

    pub fn blend_point(
        &mut self,
        x: i32,
        y: i32,
        depth: f32,
        color: Vector3,
        alpha: f32,
        write_depth: bool,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = y as usize * self.width as usize + x as usize;
            if depth >= self.depth_buffer[index] {
                return;
            }
            if write_depth {
                self.depth_buffer[index] = depth;
            }
            let pixel = &mut self.pixels[index * CHANNELS_PER_PIXEL..][..CHANNELS_PER_PIXEL];
            if alpha >= 1.0 {
                pixel.copy_from_slice(&[
                    (color.x.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                    (color.y.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                    (color.z.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX) as u8,
                    OPAQUE_ALPHA,
                ]);
                return;
            }
            let inverse_alpha = 1.0 - alpha;
            pixel[0] = (color.x.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX * alpha
                + pixel[0] as f32 * inverse_alpha) as u8;
            pixel[1] = (color.y.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX * alpha
                + pixel[1] as f32 * inverse_alpha) as u8;
            pixel[2] = (color.z.clamp(0.0, 1.0) * COLOR_CHANNEL_MAX * alpha
                + pixel[2] as f32 * inverse_alpha) as u8;
            pixel[3] = OPAQUE_ALPHA;
        }
    }

    pub fn is_closer(&self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return false;
        }
        depth < self.depth_buffer[y as usize * self.width as usize + x as usize]
    }

    pub fn swap_buffers(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        overlay: Option<&str>,
    ) {
        self.present(window, raylib_thread, overlay, |_| {});
    }

    pub fn present(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        overlay: Option<&str>,
        draw_ui: impl FnOnce(&mut RaylibDrawHandle),
    ) {
        self.present_with_overlay_bottom(window, raylib_thread, overlay, 14, draw_ui);
    }

    pub fn present_with_overlay_bottom(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        overlay: Option<&str>,
        bottom_margin: i32,
        draw_ui: impl FnOnce(&mut RaylibDrawHandle),
    ) {
        let crt_enabled = crate::display::crt_enabled();
        if crt_enabled {
            self.prepare_display_pixels();
        }
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
        let display = if crt_enabled {
            &self.display_pixels
        } else {
            &self.pixels
        };
        if texture.update_texture(display).is_err() {
            return;
        }

        const OVERLAY_FONT_SIZE: i32 = 18;
        let overlay_lines = overlay.map(|text| {
            wrap_overlay(text, self.width as i32 - 64, |s| {
                window.measure_text(s, OVERLAY_FONT_SIZE)
            })
        });
        let overlay_width = overlay_lines.as_ref().map(|lines| {
            lines
                .iter()
                .map(|s| window.measure_text(s, OVERLAY_FONT_SIZE))
                .max()
                .unwrap_or(0)
        });
        let camera = crate::display::viewport(
            window.get_screen_width(),
            window.get_screen_height(),
            self.width,
            self.height,
        );
        let mut drawing = window.begin_drawing(raylib_thread);
        drawing.clear_background(Color::BLACK);
        let mut renderer = drawing.begin_mode2D(camera);
        renderer.draw_texture(texture, 0, 0, Color::WHITE);
        if let Some((lines, width)) = overlay_lines.zip(overlay_width) {
            const PADDING: i32 = 14;
            let x = (self.width as i32 - width) / 2;
            let height = lines.len() as i32 * 24 + PADDING * 2;
            let y = self.height as i32 - height - bottom_margin;
            renderer.draw_rectangle(
                x - PADDING,
                y,
                width + PADDING * 2,
                height,
                Color::new(7, 18, 36, 235),
            );
            renderer.draw_rectangle_lines(
                x - PADDING,
                y,
                width + PADDING * 2,
                height,
                Color::new(85, 142, 199, 220),
            );
            for (i, line) in lines.iter().enumerate() {
                let color = if i == 0 && lines.len() > 1 {
                    Color::new(135, 205, 255, 255)
                } else {
                    Color::RAYWHITE
                };
                renderer.draw_text(
                    line,
                    x,
                    y + PADDING + i as i32 * 24,
                    OVERLAY_FONT_SIZE,
                    color,
                );
            }
        }
        draw_ui(&mut renderer);
        if crt_enabled {
            draw_crt_overlay(&mut renderer, self.width as i32, self.height as i32);
        }
    }
}

fn draw_crt_overlay(d: &mut impl RaylibDraw, width: i32, height: i32) {
    // Sparse phosphor stripes preserve color while scanlines hide hard polygon edges.
    for x in (0..width).step_by(8) {
        d.draw_rectangle(x, 0, 2, height, Color::new(255, 45, 35, 5));
        d.draw_rectangle(x + 4, 0, 2, height, Color::new(35, 80, 255, 5));
    }
    for y in (2..height).step_by(4) {
        d.draw_rectangle(0, y, width, 1, Color::new(0, 0, 0, 38));
    }

    // Layered edge falloff approximates the curved, dark glass of a CRT.
    for band in 0..18 {
        let alpha = (3 + (18 - band) * 2) as u8;
        d.draw_rectangle(band, band, width - band * 2, 1, Color::new(0, 0, 0, alpha));
        d.draw_rectangle(
            band,
            height - 1 - band,
            width - band * 2,
            1,
            Color::new(0, 0, 0, alpha),
        );
        d.draw_rectangle(band, band, 1, height - band * 2, Color::new(0, 0, 0, alpha));
        d.draw_rectangle(
            width - 1 - band,
            band,
            1,
            height - band * 2,
            Color::new(0, 0, 0, alpha),
        );
    }

    // Stepped corners make the rectangular framebuffer read as rounded glass.
    for (inset, length) in [(0, 18), (1, 13), (2, 9), (3, 6), (4, 3)] {
        let color = Color::new(0, 0, 0, 225);
        d.draw_rectangle(inset, inset, length, 1, color);
        d.draw_rectangle(width - inset - length, inset, length, 1, color);
        d.draw_rectangle(inset, height - inset - 1, length, 1, color);
        d.draw_rectangle(width - inset - length, height - inset - 1, length, 1, color);
    }
}

fn wrap_overlay(text: &str, max_width: i32, measure: impl Fn(&str) -> i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate) > max_width {
                lines.push(line);
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_layout_preserves_the_title_and_wraps_without_losing_words() {
        let lines = wrap_overlay(
            "Mundo Amatista\nRelieve violeta entre rocas y nubes",
            16,
            |s| s.chars().count() as i32,
        );
        assert_eq!(lines[0], "Mundo Amatista");
        assert!(lines.iter().all(|s| s.chars().count() <= 16));
        assert_eq!(lines[1..].join(" "), "Relieve violeta entre rocas y nubes");
    }

    #[test]
    fn emitted_light_adds_color_respects_depth_and_does_not_occlude() {
        let mut fb = Framebuffer::new(1, 1, Color::BLACK);
        fb.point(0, 0, 2.0, Vector3::new(0.2, 0.2, 0.2));
        fb.add_light(0, 0, 1.0, Vector3::zero(), 1.0);
        assert_eq!(&fb.pixels[..4], &[51, 51, 51, 255]);
        fb.add_light(0, 0, 1.0, Vector3::new(0., 1., 0.), 0.8);
        assert_eq!(&fb.pixels[..4], &[51, 255, 51, 255]);
        fb.add_light(0, 0, 3.0, Vector3::new(1., 0., 0.), 1.0);
        assert_eq!(&fb.pixels[..4], &[51, 255, 51, 255]);
        fb.point(0, 0, 1.5, Vector3::new(0., 0., 1.));
        assert_eq!(&fb.pixels[..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn clear_reuses_pixel_storage() {
        let mut framebuffer = Framebuffer::new(4, 3, Color::new(1, 2, 3, 255));
        let pixels = framebuffer.pixels.as_ptr();
        framebuffer.point(1, 1, 1.0, Vector3::new(1.0, 0.0, 0.0));

        framebuffer.clear();

        assert_eq!(pixels, framebuffer.pixels.as_ptr());
        assert!(
            framebuffer
                .pixels
                .chunks_exact(CHANNELS_PER_PIXEL)
                .all(|pixel| pixel == [1, 2, 3, 255])
        );
    }

    #[test]
    fn display_groups_the_scene_into_two_by_two_pixels() {
        let mut framebuffer = Framebuffer::new(3, 2, Color::BLACK);
        framebuffer.pixels[..12].copy_from_slice(&[0, 0, 0, 255, 100, 0, 0, 255, 20, 40, 60, 255]);
        framebuffer.pixels[12..]
            .copy_from_slice(&[0, 100, 0, 255, 100, 100, 0, 255, 60, 80, 100, 255]);

        framebuffer.prepare_display_pixels();

        for pixel in [0, 1, 3, 4] {
            let start = pixel * 4;
            assert_eq!(
                &framebuffer.display_pixels[start..start + 4],
                &[50, 50, 0, 255]
            );
        }
        assert_eq!(&framebuffer.display_pixels[8..12], &[40, 60, 80, 255]);
        assert_eq!(&framebuffer.display_pixels[20..24], &[40, 60, 80, 255]);
    }

    #[test]
    fn transparent_points_blend_without_occluding_later_geometry() {
        let mut framebuffer = Framebuffer::new(1, 1, Color::BLACK);
        framebuffer.blend_point(0, 0, 1.0, Vector3::new(1.0, 0.0, 0.0), 0.5, false);
        assert_eq!(&framebuffer.pixels[..4], &[127, 0, 0, 255]);

        framebuffer.point(0, 0, 2.0, Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(&framebuffer.pixels[..4], &[0, 255, 0, 255]);
    }
}
