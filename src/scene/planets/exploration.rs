//! Ureski's permanent spring and the living archive of Alfa Riddertal.
mod models;
mod surface;
use crate::{
    noise::value_noise_3d,
    objects::celestial::CelestialBody,
    rendering::{
        framebuffer::Framebuffer,
        projection::{project_vertex, projected_sphere_radius},
        render::Renderer,
        uniforms::Uniforms,
    },
    scene::world::{
        LocalOrbit, RenderPass, TransformAnchor, World, WorldObject, WorldTransform, WorldUpdate,
    },
};
use raylib::prelude::*;
use std::f32::consts::TAU;
pub const LIFE_SEED: u32 = 0x0A7E_2026;
const STATION_COUNT: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq)]
enum ModelKind {
    Canopy,
    Fern,
    Hopper,
    Skimmer,
    Station,
}
fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}
fn direction(q: Quaternion, p: Vector3) -> Vector3 {
    let m = q.to_matrix();
    v(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z,
    )
}
fn model_matrix(t: &WorldTransform) -> Matrix {
    Matrix::scale(t.scale.x, t.scale.y, t.scale.z)
        * t.rotation.to_matrix()
        * Matrix::translate(t.position.x, t.position.y, t.position.z)
}
pub(crate) fn customize(name: Option<&str>, body: CelestialBody, sun: Vector3) -> CelestialBody {
    match name {
        Some("Ureski") => {
            surface::prepare();
            body.tidally_locked(sun)
                .with_fragment_shader(surface::ureski)
        }
        Some("Alfa Riddertal") => body.with_volume_shader(surface::riddertal),
        _ => body,
    }
}
pub(crate) fn decorate(
    world: &mut World,
    name: Option<&str>,
    parent: TransformAnchor,
    radius: f32,
) {
    match name {
        Some("Ureski") => {
            for p in placements(LIFE_SEED) {
                world.add(Life::new(parent.clone(), radius, p));
            }
        }
        Some("Alfa Riddertal") => {
            for i in 0..STATION_COUNT {
                world.add(ArchiveStation::new(parent.clone(), radius, i));
            }
        }
        _ => {}
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    direction: Vector3,
    size: f32,
    phase: f32,
    kind: ModelKind,
}
fn placements(seed: u32) -> Vec<Placement> {
    let offset = v(
        (seed & 255) as f32,
        ((seed >> 8) & 255) as f32,
        (seed >> 16) as f32,
    ) * 0.03;
    let mut list = Vec::new();
    for i in 0..820 {
        let y = 1. - 2. * (i as f32 + 0.5) / 820.;
        let a = i as f32 * 2.399963 + (seed % 1000) as f32 * 0.03;
        let r = (1. - y * y).sqrt();
        let d = v(r * a.cos(), y, r * a.sin());
        if d.x.abs() > 0.245 {
            continue;
        }
        let n = value_noise_3d(d * 11. + offset);
        if n < 0.32 {
            continue;
        }
        list.push(Placement {
            direction: d,
            size: 0.033 + n * 0.027,
            phase: n * TAU * 3.,
            kind: if n > 0.53 {
                ModelKind::Canopy
            } else {
                ModelKind::Fern
            },
        });
    }
    for i in 0..58 {
        let a = i as f32 * TAU / 58.;
        let n = value_noise_3d(v(i as f32 * 0.73, 4., 8.) + offset);
        let x = (n - 0.5) * 0.32;
        let r = (1. - x * x).sqrt();
        list.push(Placement {
            direction: v(x, r * a.cos(), r * a.sin()),
            size: 0.041 + n * 0.022,
            phase: a * 4. + n * 7.,
            kind: if i % 3 == 0 {
                ModelKind::Skimmer
            } else {
                ModelKind::Hopper
            },
        });
    }
    list
}
struct Life {
    parent: TransformAnchor,
    radius: f32,
    transform: WorldTransform,
    placement: Placement,
    time: f32,
    full: bool,
}
impl Life {
    fn new(parent: TransformAnchor, radius: f32, p: Placement) -> Self {
        let size = radius * p.size;
        let mut t = WorldTransform::new(Vector3::zero(), v(size, size, size));
        t.set_parent(parent.clone(), p.direction * 0.996);
        t.set_local_rotation(
            Quaternion::from_vector3_to_vector3(v(0., 1., 0.), p.direction)
                * Quaternion::from_axis_angle(v(0., 1., 0.), p.phase),
        );
        Self {
            parent,
            radius,
            transform: t,
            placement: p,
            time: 0.,
            full: false,
        }
    }
    fn pose(&self) -> (Vector3, Quaternion) {
        let p = self.placement;
        let animal = matches!(p.kind, ModelKind::Hopper | ModelKind::Skimmer);
        let mut d = p.direction;
        let mut height = 0.996;
        let mut twist = p.phase;
        if animal {
            let tangent = v(0., -d.z, d.y).normalize();
            d = (d + tangent * (self.time * 0.7 + p.phase).sin() * 0.022).normalize();
            let jump = if p.kind == ModelKind::Hopper {
                (self.time * 3.7 + p.phase).sin().max(0.).powi(2) * 0.065
            } else {
                0.035 + (self.time * 2.1 + p.phase).sin() * 0.018
            };
            height = 1. + jump;
            twist += (self.time * 0.7 + p.phase).cos() * 0.55;
        }
        let align = Quaternion::from_vector3_to_vector3(v(0., 1., 0.), d);
        let sway = if animal {
            (self.time * 3.7 + p.phase).sin() * 0.12
        } else {
            (self.time * 1.8 + p.phase).sin() * 0.055
        };
        (
            d * height,
            (align
                * Quaternion::from_axis_angle(v(0., 1., 0.), twist)
                * Quaternion::from_axis_angle(v(0., 0., 1.), sway))
            .normalize(),
        )
    }
}
impl WorldObject for Life {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }
    fn bounding_radius(&self) -> Option<f32> {
        Some(3.2)
    }
    fn update(&mut self, u: &WorldUpdate) {
        self.time += u.frame_time;
        let (p, q) = self.pose();
        self.transform.set_local_position(p);
        self.transform.set_local_rotation(q);
    }
    fn draw(&mut self, r: &mut Renderer, fb: &mut Framebuffer, u: &Uniforms) {
        let center = self.parent.world_position();
        let view = u.camera_pos - center;
        let bound = self.transform.scale.x * 3.2;
        if (self.transform.position - center).dot(view) + bound * (view.length() + self.radius)
            < self.radius * self.radius
        {
            return;
        }
        let pixels = projected_sphere_radius(
            self.transform.position,
            self.transform.scale.x * 3.2,
            u,
            fb.height,
        );
        let full = pixels >= if self.full { 48. } else { 64. };
        if full != self.full {
            self.full = full;
            r.record_lod_switch();
        }
        let model = models::get(self.placement.kind, full);
        let mut matrix = model_matrix(&self.transform);
        if self.placement.kind == ModelKind::Hopper {
            let bounce = (self.time * 3.7 + self.placement.phase).sin();
            matrix =
                Matrix::scale(1. - bounce * 0.08, 1. + bounce * 0.16, 1. - bounce * 0.08) * matrix;
        } else if self.placement.kind == ModelKind::Skimmer {
            matrix = Matrix::scale(
                0.80 + 0.20 * (self.time * 10. + self.placement.phase).sin(),
                1.,
                1.,
            ) * matrix;
        }
        let u = Uniforms {
            model_matrix: matrix,
            ..*u
        };
        r.render_lit_fill(fb, &u, &model.solid);
        r.render(fb, &u, &model.glow);
    }
}
struct ArchiveStation {
    transform: WorldTransform,
    parent: TransformAnchor,
    radius: f32,
    time: f32,
    phase: f32,
    full: bool,
}
impl ArchiveStation {
    fn new(parent: TransformAnchor, radius: f32, index: usize) -> Self {
        let a = index as f32 * TAU / STATION_COUNT as f32;
        let inclination = -0.55 + index as f32 * 0.19;
        let axis = v(0., inclination.cos(), inclination.sin());
        let offset = v(
            a.cos(),
            -a.sin() * inclination.sin(),
            a.sin() * inclination.cos(),
        ) * (radius * (2.0 + (index % 2) as f32 * 0.28));
        let size = radius * 0.10;
        let mut t = WorldTransform::new(Vector3::zero(), v(size, size, size));
        t.set_parent_orbit(
            parent.clone(),
            LocalOrbit::new(
                offset,
                axis,
                if index.is_multiple_of(2) { 0.13 } else { -0.10 },
            ),
        );
        Self {
            transform: t,
            parent,
            radius,
            time: 0.,
            phase: a,
            full: false,
        }
    }
    fn cycle(&self) -> (bool, f32, bool) {
        let t = (self.time + self.phase * 1.7).rem_euclid(7.2);
        (
            t < 5.8,
            (self.time * 1.1 + self.phase).fract(),
            ((self.time + self.phase) / 3.6).floor() as i32 % 2 == 0,
        )
    }
}
impl WorldObject for ArchiveStation {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }
    fn bounding_radius(&self) -> Option<f32> {
        Some(2.5)
    }
    fn render_pass(&self) -> RenderPass {
        RenderPass::Transparent
    }
    fn update(&mut self, u: &WorldUpdate) {
        self.time += u.frame_time;
        self.transform.set_local_rotation(Quaternion::identity());
        let inward = (self.parent.world_position() - self.transform.position).normalize();
        self.transform
            .set_local_rotation(Quaternion::from_vector3_to_vector3(v(0., 0., 1.), inward));
    }
    fn draw(&mut self, r: &mut Renderer, fb: &mut Framebuffer, u: &Uniforms) {
        let pixels = projected_sphere_radius(
            self.transform.position,
            self.transform.scale.x * 2.5,
            u,
            fb.height,
        );
        let full = pixels >= if self.full { 12. } else { 18. };
        if full != self.full {
            self.full = full;
            r.record_lod_switch();
        }
        let model = models::get(ModelKind::Station, full);
        let station = Uniforms {
            model_matrix: model_matrix(&self.transform),
            ..*u
        };
        r.render_lit_fill(fb, &station, &model.solid);
        r.render(fb, &station, &model.glow);
        let (active, phase, outgoing) = self.cycle();
        if !active {
            return;
        }
        let center = self.parent.world_position();
        let radial = (self.transform.position - center).normalize();
        let a = self.transform.position - radial * self.transform.scale.x * 0.90;
        let b = center + radial * self.radius * 1.006;
        let color = if outgoing {
            v(0.20, 0.90, 1.)
        } else {
            v(1., 0.32, 0.72)
        };
        beam(fb, u, a, b, color, 0.42);
        for i in 0..3 {
            let t = (phase + i as f32 / 3.).fract();
            let t = if outgoing { t } else { 1. - t };
            let p = a + (b - a) * t;
            glow_world(fb, u, p, color, 4.5, 1.4);
        }
        glow_world(fb, u, b, color, 8., 0.85);
    }
}
fn project(p: Vector3, fb: &Framebuffer, u: &Uniforms) -> Option<(Vector2, f32)> {
    project_vertex(
        p,
        u.camera_pos,
        u.camera_target,
        u.camera_up,
        fb.width as f32,
        fb.height as f32,
    )
}
fn glow_world(fb: &mut Framebuffer, u: &Uniforms, p: Vector3, c: Vector3, radius: f32, power: f32) {
    let Some((p, depth)) = project(p, fb, u) else {
        return;
    };
    let extent = radius.ceil() as i32;
    for y in -extent..=extent {
        for x in -extent..=extent {
            let f = (1. - (x * x + y * y) as f32 / (radius * radius)).max(0.);
            fb.add_light(p.x as i32 + x, p.y as i32 + y, depth, c, power * f * f);
        }
    }
}
fn beam(fb: &mut Framebuffer, u: &Uniforms, a: Vector3, b: Vector3, c: Vector3, power: f32) {
    let (Some((a, da)), Some((b, db))) = (project(a, fb, u), project(b, fb, u)) else {
        return;
    };
    let steps = (b - a).length().ceil().clamp(1., 1200.) as usize;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let p = a + (b - a) * t;
        let depth = 1. / ((1. - t) / da + t / db);
        fb.add_light(p.x as i32, p.y as i32, depth, c, power * 1.8);
        for (x, y) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            fb.add_light(p.x as i32 + x, p.y as i32 + y, depth, c, power * 0.30);
        }
    }
}

pub(crate) fn preview() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("target/previews")?;
    for (name, label) in [("Ureski", "ureski"), ("Alfa Riddertal", "riddertal")] {
        for (view, time) in [
            ("orbita", 0.),
            ("animacion", 1.2),
            ("superficie", 0.),
            ("noche", 0.),
        ] {
            let mut world = crate::scene::build_solar_system();
            world.update(&WorldUpdate { frame_time: time });
            let sun = world.primary_light_position().unwrap();
            let body = world
                .navigation_target(Some(name), Vector3::zero())
                .unwrap();
            let light = (sun - body.position).normalize();
            let lock = Quaternion::from_vector3_to_vector3(v(1., 0., 0.), light);
            let (offset, focus) = if name == "Ureski" {
                let site = placements(LIFE_SEED)
                    .into_iter()
                    .find(|p| p.kind == ModelKind::Hopper)
                    .unwrap();
                if view == "superficie" {
                    let d = direction(lock, site.direction);
                    (d * 1.32 + direction(lock, v(0., 0.09, 0.01)), d * 1.035)
                } else if view == "noche" {
                    (-light * 3.1, Vector3::zero())
                } else {
                    (
                        direction(lock, v(0.18, 0.35, -1.)).normalize() * 3.35,
                        Vector3::zero(),
                    )
                }
            } else if view == "superficie" {
                (v(0., 0.32, -2.7), Vector3::zero())
            } else {
                (v(0.25, 0.45, -3.8), Vector3::zero())
            };
            let mut fb = Framebuffer::new(1000, 750, Color::new(3, 5, 13, 255));
            let mut u = Uniforms {
                model_matrix: Matrix::identity(),
                camera_pos: body.position + offset * body.radius,
                camera_target: body.position + focus * body.radius,
                camera_up: v(0., 1., 0.),
                light_position: Some(sun),
            };
            let mut center = body.position;
            let mut renderer = Renderer::default();
            // Let the real LOD morph settle; capture the same scene a player sees after approaching.
            for _ in 0..2 {
                fb.clear();
                world.draw(&mut renderer, &mut fb, &u);
                world.update(&WorldUpdate { frame_time: 0.7 });
                let current = world
                    .navigation_target(Some(name), Vector3::zero())
                    .unwrap()
                    .position;
                u.camera_pos += current - center;
                u.camera_target += current - center;
                center = current;
            }
            fb.clear();
            crate::scene::Starfield::new().draw(&mut fb, &u);
            renderer.begin_frame();
            let start = std::time::Instant::now();
            world.draw(&mut renderer, &mut fb, &u);
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            let path = format!("target/previews/persona3-{label}-{view}.png");
            fb.save_png(&path)?;
            println!(
                "{path} | {elapsed:.2} ms CPU | {} triangulos | vertices {:.2} ms / raster {:.2} ms",
                renderer.metrics().triangles,
                renderer.metrics().vertex.as_secs_f64() * 1000.,
                renderer.metrics().rasterization.as_secs_f64() * 1000.
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{celestial_shaders::CelestialKind, scene::world::Orbit};
    #[test]
    fn seeded_life_stays_in_the_temperate_belt() {
        let a = placements(LIFE_SEED);
        assert_eq!(a, placements(LIFE_SEED));
        assert_ne!(a, placements(LIFE_SEED + 1));
        assert!(a.len() > 150);
        assert!(
            a.iter()
                .all(|p| p.direction.x.abs() < 0.25 && (p.direction.length() - 1.).abs() < 0.001)
        );
        for k in [
            ModelKind::Canopy,
            ModelKind::Fern,
            ModelKind::Hopper,
            ModelKind::Skimmer,
        ] {
            assert!(a.iter().any(|p| p.kind == k));
        }
    }
    #[test]
    fn fauna_really_jumps_while_plants_remain_rooted() {
        let parent = WorldTransform::new(Vector3::zero(), v(100., 100., 100.));
        for kind in [ModelKind::Hopper, ModelKind::Skimmer, ModelKind::Canopy] {
            let p = placements(LIFE_SEED)
                .into_iter()
                .find(|p| p.kind == kind)
                .unwrap();
            let mut life = Life::new(parent.anchor(), 100., p);
            let mut min = f32::MAX;
            let mut max = 0_f32;
            for _ in 0..120 {
                life.update(&WorldUpdate {
                    frame_time: 1. / 60.,
                });
                let height = life.transform.position.length();
                min = min.min(height);
                max = max.max(height);
            }
            if kind == ModelKind::Canopy {
                assert!(max - min < 0.001);
            } else {
                assert!(max - min > 2.);
            }
        }
    }
    #[test]
    fn ureski_keeps_the_same_subsolar_land_through_its_orbit() {
        let sun = Vector3::zero();
        let center = v(700., 100., 0.);
        let body = CelestialBody::new(CelestialKind::Rocky, center)
            .named("Ureski")
            .with_scale(100.)
            .with_orbit(Orbit::new(sun, center, v(0., 1., 0.), 0.12))
            .tidally_locked(sun);
        let mut probe = WorldTransform::new(Vector3::zero(), v(1., 1., 1.));
        probe.set_parent(body.transform.anchor(), v(1., 0., 0.));
        let mut world = World::new();
        world.add(body);
        for _ in 0..180 {
            world.update(&WorldUpdate { frame_time: 0.5 });
            probe.set_local_rotation(Quaternion::identity());
            let center = world
                .navigation_target(Some("Ureski"), Vector3::zero())
                .unwrap()
                .position;
            assert!(
                (probe.position - center)
                    .normalize()
                    .dot((sun - center).normalize())
                    > 0.9999
            );
        }
    }
    #[test]
    fn archive_stations_orbit_and_exchange_packets_in_both_directions() {
        let parent = CelestialBody::new(CelestialKind::GasGiant, v(400., 0., 0.)).with_scale(100.);
        let mut station = ArchiveStation::new(parent.transform.anchor(), 100., 0);
        let initial = station.transform.position;
        let mut world = World::new();
        world.add(parent);
        // Register station in the real transform update pipeline; its original anchor remains observable.
        let mut outgoing = false;
        let mut incoming = false;
        let mut silent = false;
        for _ in 0..180 {
            station.update(&WorldUpdate { frame_time: 0.1 });
            let (active, phase, send) = station.cycle();
            outgoing |= active && send;
            incoming |= active && !send;
            silent |= !active;
            assert!((0.0..1.0).contains(&phase));
        }
        assert!(outgoing && incoming && silent);
        let anchor = station.transform.anchor();
        world.add(station);
        world.update(&WorldUpdate { frame_time: 3. });
        assert!((anchor.world_position() - initial).length() > 20.);
        assert!(((anchor.world_position() - v(400., 0., 0.)).length() - 200.).abs() < 0.01);
    }
}
