use super::{architecture, surface};
use crate::noise::value_noise_3d;
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::Mesh;
use crate::rendering::projection::projected_sphere_radius;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::scene::world::{TransformAnchor, World, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Vector3};

const DISTRICT_SEED: u32 = 0x434F_5255;
const FOUNDATION_DEPTH: f32 = 8.0;
const FULL_DETAIL_ENTER_PX: f32 = 108.0;
const FULL_DETAIL_EXIT_PX: f32 = 88.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Building {
    Tower,
    Platform,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    kind: Building,
    direction: Vector3,
    width: f32,
    height: f32,
    brightness: f32,
}

pub(super) fn site() -> &'static surface::District {
    static SITE: std::sync::OnceLock<&'static surface::District> = std::sync::OnceLock::new();
    SITE.get_or_init(|| surface::nearest_district(Vector3::new(-0.76, 0.16, -0.63)))
}

fn placements(site: &surface::District, seed: u32, radius: f32) -> Vec<Placement> {
    let seed_offset = Vector3::new(
        (seed & 0x3ff) as f32,
        ((seed >> 10) & 0x3ff) as f32,
        (seed >> 20) as f32,
    ) * 0.03125;
    let mut buildings = Vec::new();
    for row in -3..3 {
        for column in -4..4 {
            let cell = Vector3::new(column as f32, row as f32, 0.0) + seed_offset;
            let density = value_noise_3d(cell * 0.68);
            let jitter_x = (value_noise_3d(cell + Vector3::new(3.0, 7.0, 4.0)) - 0.5) * 0.18;
            let jitter_z = (value_noise_3d(cell + Vector3::new(9.0, 2.0, 3.0)) - 0.5) * 0.18;
            let x = (column as f32 + 0.5 + jitter_x) * site.spacing;
            let z = (row as f32 + 0.5 + jitter_z) * site.north_spacing();
            let extent = x * x + z * z;
            if extent > 0.18 * 0.18 || density < 0.26 {
                continue;
            }
            let direction = site.block_direction(x, z);
            if !std::ptr::eq(surface::nearest_district(direction), site) {
                continue;
            }

            let variation = value_noise_3d(cell + Vector3::new(6.4, 8.3, 1.6));
            let (kind, width, height) = if density > 0.52 {
                let core = 1.0 - extent.sqrt() / 0.18;
                (
                    Building::Tower,
                    0.40 + variation * 0.23,
                    6.5 + core * 6.5 + variation * 5.0,
                )
            } else {
                (
                    Building::Platform,
                    0.51 + variation * 0.21,
                    3.7 + variation * 3.4,
                )
            };
            buildings.push(Placement {
                kind,
                direction,
                width: width * radius * site.spacing,
                height,
                brightness: 0.68 + 0.32 * value_noise_3d(cell + Vector3::new(3.0, 6.0, 11.0)),
            });
        }
    }
    buildings
}

struct SurfaceDistrict {
    transform: WorldTransform,
    geometry: architecture::Architecture,
    distant_geometry: architecture::Architecture,
    full_detail: bool,
    bounds: f32,
}

pub(super) fn register(world: &mut World, parent: TransformAnchor, radius: f32) {
    world.add(SurfaceDistrict::attached_to(parent, radius));
}

impl SurfaceDistrict {
    fn attached_to(parent: TransformAnchor, radius: f32) -> Self {
        let site = site();
        let tower = [architecture::tower(), architecture::distant_tower()];
        let platform = [architecture::platform(), architecture::distant_platform()];
        let mut geometry = architecture::Architecture::empty();
        let mut distant_geometry = architecture::Architecture::empty();

        // Las posiciones se calculan en el espacio local del planeta, antes de agrupar las mallas.
        for placement in placements(site, DISTRICT_SEED, radius) {
            let up = placement.direction.normalize();
            let forward = site.east.cross(up).normalize();
            let right = up.cross(forward);
            let origin = (up - site.center) * radius;
            let models = match placement.kind {
                Building::Tower => &tower,
                Building::Platform => &platform,
            };
            let place = |point: Vector3| {
                let vertical_scale = if point.y < 0.0 {
                    FOUNDATION_DEPTH
                } else {
                    placement.height
                };
                origin
                    + right * (point.x * placement.width)
                    + up * (point.y * vertical_scale)
                    + forward * (point.z * placement.width)
            };
            let normal = |direction: Vector3| {
                (right * direction.x
                    + up * (direction.y * placement.width / placement.height)
                    + forward * direction.z)
                    .normalize()
            };
            for (batch, model) in [&mut geometry, &mut distant_geometry]
                .into_iter()
                .zip(models)
            {
                append(&mut batch.shell, &model.shell, place, normal, 1.0);
                append(
                    &mut batch.lights,
                    &model.lights,
                    place,
                    normal,
                    placement.brightness,
                );
            }
        }
        let bounds = geometry
            .shell
            .vertices
            .iter()
            .chain(&geometry.lights.vertices)
            .chain(&distant_geometry.shell.vertices)
            .chain(&distant_geometry.lights.vertices)
            .map(|vertex| vertex.position.length())
            .fold(0.0, f32::max);

        let mut transform = WorldTransform::new(Vector3::zero(), Vector3::new(1.0, 1.0, 1.0));
        transform.set_parent(parent, site.center);
        Self {
            transform,
            geometry,
            distant_geometry,
            full_detail: false,
            bounds,
        }
    }
}

fn append(
    destination: &mut Mesh,
    model: &Mesh,
    place: impl Fn(Vector3) -> Vector3,
    normal: impl Fn(Vector3) -> Vector3,
    brightness: f32,
) {
    let first = destination.vertices.len() as u32;
    destination
        .vertices
        .extend(model.vertices.iter().map(|vertex| {
            let mut vertex = *vertex;
            vertex.position = place(vertex.position);
            vertex.normal = normal(vertex.normal);
            vertex.color *= brightness;
            vertex
        }));
    destination
        .indices
        .extend(model.indices.iter().map(|index| first + index));
}

impl WorldObject for SurfaceDistrict {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(self.bounds)
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let transform = &self.transform;
        let scale = transform
            .scale
            .x
            .abs()
            .max(transform.scale.y.abs())
            .max(transform.scale.z.abs());
        let projected = projected_sphere_radius(
            transform.position,
            self.bounds * scale,
            uniforms,
            framebuffer.height,
        );
        let threshold = if self.full_detail {
            FULL_DETAIL_EXIT_PX
        } else {
            FULL_DETAIL_ENTER_PX
        };
        let full_detail = projected >= threshold;
        if full_detail != self.full_detail {
            renderer.record_lod_switch();
            self.full_detail = full_detail;
        }
        let geometry = if full_detail {
            &self.geometry
        } else {
            &self.distant_geometry
        };
        let uniforms = Uniforms {
            model_matrix: Matrix::scale(transform.scale.x, transform.scale.y, transform.scale.z)
                * transform.rotation.to_matrix()
                * Matrix::translate(
                    transform.position.x,
                    transform.position.y,
                    transform.position.z,
                ),
            ..*uniforms
        };
        renderer.render_lit(framebuffer, &uniforms, &geometry.shell);
        renderer.render(framebuffer, &uniforms, &geometry.lights);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celestial_shaders::{CelestialKind, CelestialStyle};
    use crate::objects::celestial::CelestialBody;
    use crate::scene::world::{Orbit, WorldUpdate};
    use raylib::prelude::Quaternion;

    #[test]
    fn layout_repeats_with_the_same_seed_and_stays_inside_the_district() {
        let site = site();
        let layout = placements(site, DISTRICT_SEED, 140.0);
        assert_eq!(layout, placements(site, DISTRICT_SEED, 140.0));
        assert_ne!(layout, placements(site, DISTRICT_SEED + 1, 140.0));
        assert!(layout.iter().any(|lot| lot.kind == Building::Tower));
        assert!(layout.iter().any(|lot| lot.kind == Building::Platform));
        for lot in layout {
            assert!((lot.direction.length() - 1.0).abs() < 0.000_01);
            assert!(std::ptr::eq(surface::nearest_district(lot.direction), site));
        }
    }

    #[test]
    fn district_keeps_its_surface_position_as_the_planet_orbits_and_spins() {
        let radius = 140.0;
        let initial = Vector3::new(750.0, 0.0, 0.0);
        let up = Vector3::new(0.0, 1.0, 0.0);
        let planet = CelestialBody::styled(
            CelestialKind::Rocky,
            initial,
            CelestialStyle {
                rotation_speed: 0.12,
                ..CelestialStyle::default()
            },
        )
        .with_scale(radius)
        .with_orbit(Orbit::new(Vector3::zero(), initial, up, 0.05));
        let mut district = SurfaceDistrict::attached_to(planet.transform.anchor(), radius);
        let mut world = World::new();
        world.add(planet);
        world.update(&WorldUpdate { frame_time: 2.0 });
        district
            .transform
            .set_local_rotation(Quaternion::identity());

        let planet_position = initial.rotate_by_axis_angle(up, 0.10);
        let radial = site().center.rotate_by_axis_angle(up, 0.24);
        assert!(
            (district.transform.position - (planet_position + radial * radius)).length() < 0.001
        );
        assert!(
            (site()
                .center
                .rotate_by_quaternion(district.transform.rotation)
                - radial)
                .length()
                < 0.001
        );
    }
}
