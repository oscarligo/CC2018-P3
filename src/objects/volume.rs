//! A camera-aligned software shading cache. Geometry and depth still use the CPU rasterizer.
use raylib::prelude::Vector3;

#[derive(Default)]
pub(super) struct VolumeCache {
    colors: Vec<Vector3>,
    size: usize,
    camera: Vector3,
    forward: Vector3,
    right: Vector3,
    up: Vector3,
    tangent: f32,
}
impl VolumeCache {
    pub fn prepare(
        &mut self,
        camera: Vector3,
        light: Vector3,
        time: f32,
        size: usize,
        shade: fn(Vector3, Vector3, f32, Vector3) -> Vector3,
    ) {
        self.size = size;
        self.camera = camera;
        self.forward = -camera.normalize();
        self.right = Vector3::new(0., 1., 0.).cross(self.forward);
        if self.right.length_sqr() < 0.001 {
            self.right = Vector3::new(1., 0., 0.).cross(self.forward);
        }
        self.right = self.right.normalize();
        self.up = self.forward.cross(self.right).normalize();
        self.tangent = 1.0 / (camera.length_sqr() - 1.0).max(0.01).sqrt();
        self.colors.resize(size * size, Vector3::zero());
        for y in 0..size {
            for x in 0..size {
                let mut u = x as f32 / (size - 1) as f32 * 2. - 1.;
                let mut v = y as f32 / (size - 1) as f32 * 2. - 1.;
                // Extend the limb into the unused square corners for clean bilinear edges.
                let distance = u * u + v * v;
                if distance > 0.999 {
                    let scale = (0.999 / distance).sqrt();
                    u *= scale;
                    v *= scale;
                }
                let ray =
                    (self.forward + self.right * (u * self.tangent) + self.up * (v * self.tangent))
                        .normalize();
                let b = camera.dot(ray);
                let discriminant = (b * b - camera.length_sqr() + 1.).max(0.);
                let p = camera + ray * (-b - discriminant.sqrt());
                self.colors[y * size + x] = shade(p.normalize(), ray, time, light);
            }
        }
    }
    pub fn sample(&self, point: Vector3) -> Vector3 {
        let ray = point - self.camera;
        let depth = ray.dot(self.forward).max(0.001);
        let u = ray.dot(self.right) / (depth * self.tangent);
        let v = ray.dot(self.up) / (depth * self.tangent);
        let x =
            ((u * 0.5 + 0.5) * (self.size - 1) as f32).clamp(0., (self.size - 1) as f32 - 0.0001);
        let y =
            ((v * 0.5 + 0.5) * (self.size - 1) as f32).clamp(0., (self.size - 1) as f32 - 0.0001);
        let ix = x as usize;
        let iy = y as usize;
        let fx = x - ix as f32;
        let fy = y - iy as f32;
        let a = self.colors[iy * self.size + ix] * (1. - fx)
            + self.colors[iy * self.size + ix + 1] * fx;
        let b = self.colors[(iy + 1) * self.size + ix] * (1. - fx)
            + self.colors[(iy + 1) * self.size + ix + 1] * fx;
        a * (1. - fy) + b * fy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_rays_match_the_analytic_surface_from_different_views() {
        let shader =
            |p: Vector3, _: Vector3, t: f32, _: Vector3| p * 0.2 + Vector3::new(t, 0.3, 0.4);
        let mut cache = VolumeCache::default();
        for camera in [Vector3::new(0., 0., -3.), Vector3::new(3., 2., 1.)] {
            cache.prepare(camera, Vector3::zero(), 0.5, 96, shader);
            let center = camera.normalize();
            let sampled = cache.sample(center);
            assert!(
                (sampled - shader(center, Vector3::zero(), 0.5, Vector3::zero())).length() < 0.003
            );
        }
    }
}
