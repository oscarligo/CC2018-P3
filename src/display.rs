use raylib::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static CRT_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn crt_enabled() -> bool {
    CRT_ENABLED.load(Ordering::Relaxed)
}

pub const CONTENT_WIDTH: i32 = 800;
pub const CONTENT_HEIGHT: i32 = 600;

pub fn viewport(width: i32, height: i32, content_width: u32, content_height: u32) -> Camera2D {
    let zoom = (width.max(1) as f32 / content_width as f32)
        .min(height.max(1) as f32 / content_height as f32);
    Camera2D {
        offset: Vector2::new(
            (width as f32 - content_width as f32 * zoom) * 0.5,
            (height as f32 - content_height as f32 * zoom) * 0.5,
        ),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom,
    }
}

pub fn update(window: &mut RaylibHandle) {
    if window.is_key_pressed(KeyboardKey::KEY_F10) {
        CRT_ENABLED.fetch_xor(true, Ordering::Relaxed);
    }
    if window.is_key_pressed(KeyboardKey::KEY_F11) {
        window.toggle_borderless_windowed();
    }
    let camera = viewport(
        window.get_screen_width(),
        window.get_screen_height(),
        CONTENT_WIDTH as u32,
        CONTENT_HEIGHT as u32,
    );
    window.set_mouse_offset(Vector2::new(-camera.offset.x, -camera.offset.y));
    window.set_mouse_scale(1.0 / camera.zoom, 1.0 / camera.zoom);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_preserves_aspect_ratio_and_mouse_coordinates() {
        for (width, height) in [(800, 600), (1920, 1080), (1280, 1024), (600, 900)] {
            let camera = viewport(width, height, 800, 600);
            assert!(camera.offset.x >= 0.0 && camera.offset.y >= 0.0);
            assert!(camera.offset.x + 800.0 * camera.zoom <= width as f32 + 0.01);
            assert!(camera.offset.y + 600.0 * camera.zoom <= height as f32 + 0.01);
            let logical = Vector2::new(680.0, 30.0);
            let screen = logical * camera.zoom + camera.offset;
            let restored = (screen - camera.offset) * (1.0 / camera.zoom);
            assert!((restored - logical).length() < 0.001);
        }
    }
}
