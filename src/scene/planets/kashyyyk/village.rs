use super::{forest, surface};
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::{Mesh, load_obj};
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::scene::world::{TransformAnchor, World, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Quaternion, Vector3};
use std::f32::consts::PI;
use std::sync::OnceLock;

const SIZE_FRACTION: f32 = 0.022;
const CLEARANCE: f32 = 0.36;
const SECOND_TREE_X: f32 = 10.8;

fn secondary_root() -> Vector3 {
    let local_radius = 0.997 / SIZE_FRACTION;
    let x = 9.75;
    Vector3::new(
        x,
        (local_radius * local_radius - x * x).sqrt() - local_radius,
        0.0,
    )
}

fn site_rotation() -> Quaternion {
    static ROTATION: OnceLock<Quaternion> = OnceLock::new();
    *ROTATION.get_or_init(|| {
        let up = Vector3::new(0.0, 1.0, 0.0);
        let align = Quaternion::from_vector3_to_vector3(up, site_direction());
        (0..32)
            .map(|i| align * Quaternion::from_axis_angle(up, i as f32 * PI / 16.0))
            .max_by(|a, b| {
                let weight = |rotation: &Quaternion| {
                    let root = site_direction() * 0.997
                        + rotate(rotation.to_matrix(), secondary_root()) * SIZE_FRACTION;
                    surface::forest_weight(root.normalize())
                };
                weight(a).total_cmp(&weight(b))
            })
            .expect("La aldea necesita una orientación válida")
    })
}

/// Zona forestal próxima al hemisferio visible e iluminado al comenzar.
pub(super) fn site_direction() -> Vector3 {
    static SITE: OnceLock<Vector3> = OnceLock::new();
    *SITE.get_or_init(|| {
        let preferred = Vector3::new(1.0, -0.15, -0.45).normalize();
        (0..2048)
            .map(|i| {
                let y = 1.0 - 2.0 * (i as f32 + 0.5) / 2048.0;
                let angle = i as f32 * 2.399963;
                let radius = (1.0 - y * y).sqrt();
                Vector3::new(radius * angle.cos(), y, radius * angle.sin())
            })
            .filter(|point| surface::forest_weight(*point) > 0.35)
            .max_by(|a, b| a.dot(preferred).total_cmp(&b.dot(preferred)))
            .expect("Kashyyyk necesita una región forestal para la aldea")
    })
}

pub(super) fn is_reserved(direction: Vector3) -> bool {
    (direction - site_direction()).length() < CLEARANCE
}

pub(super) fn register(world: &mut World, parent: TransformAnchor, radius: f32) {
    world.add(Village::attached_to(parent, radius));
}

struct Village {
    transform: WorldTransform,
}

impl Village {
    fn attached_to(parent: TransformAnchor, radius: f32) -> Self {
        let size = radius * SIZE_FRACTION;
        let mut transform = WorldTransform::new(Vector3::zero(), Vector3::new(size, size, size));
        transform.set_parent(parent, site_direction() * 0.997);
        transform.set_local_rotation(site_rotation());
        Self { transform }
    }
}

fn rotate(matrix: Matrix, point: Vector3) -> Vector3 {
    Vector3::new(
        matrix.m0 * point.x + matrix.m4 * point.y + matrix.m8 * point.z,
        matrix.m1 * point.x + matrix.m5 * point.y + matrix.m9 * point.z,
        matrix.m2 * point.x + matrix.m6 * point.y + matrix.m10 * point.z,
    )
}

fn append(
    vertices: &mut Vec<crate::rendering::vertex::Vertex>,
    indices: &mut Vec<u32>,
    source: &Mesh,
    offset: Vector3,
    scale: f32,
    yaw: f32,
) {
    let rotation = Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), yaw);
    append_oriented(vertices, indices, source, offset, scale, rotation);
}

fn append_oriented(
    vertices: &mut Vec<crate::rendering::vertex::Vertex>,
    indices: &mut Vec<u32>,
    source: &Mesh,
    offset: Vector3,
    scale: f32,
    rotation: Quaternion,
) {
    let start = vertices.len() as u32;
    let rotation = rotation.to_matrix();
    vertices.extend(source.vertices.iter().map(|source| {
        let mut vertex = *source;
        vertex.position = offset + rotate(rotation, source.position * scale);
        vertex.normal = rotate(rotation, source.normal).normalize();
        // Todos los modelos comparten trees.mtl y la misma paleta.
        vertex.material_index = 1;
        vertex
    }));
    indices.extend(source.indices.iter().map(|index| start + index));
}

fn mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(|| {
        let [platform, hut, bridge] = ["platform", "hut", "bridge"].map(|name| {
            let path = format!("assets/kashyyyk/{name}.obj");
            load_obj(&path)
                .unwrap_or_else(|error| panic!("No se pudo cargar la aldea {path}: {error}"))
        });
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let trunk = forest::giant_mesh();
        append(
            &mut vertices,
            &mut indices,
            trunk,
            Vector3::zero(),
            1.0,
            0.0,
        );
        // El segundo árbol baja y se inclina siguiendo la superficie curva.
        let local_radius = 0.997 / SIZE_FRACTION;
        let root = secondary_root();
        let up = Vector3::new(0.0, 1.0, 0.0);
        let radial = (root + up * local_radius).normalize();
        let orientation =
            Quaternion::from_vector3_to_vector3(up, radial) * Quaternion::from_axis_angle(up, 0.6);
        append_oriented(&mut vertices, &mut indices, trunk, root, 1.0, orientation);
        append(
            &mut vertices,
            &mut indices,
            &platform,
            Vector3::new(0.0, 3.2, 0.0),
            1.0,
            0.0,
        );
        append(
            &mut vertices,
            &mut indices,
            &platform,
            Vector3::new(SECOND_TREE_X, 3.23, 0.0),
            0.85,
            0.0,
        );
        append(
            &mut vertices,
            &mut indices,
            &bridge,
            Vector3::new(3.5, 3.4, 0.0),
            1.0,
            0.0,
        );
        for (x, z, yaw) in [
            (0.0, -2.35, 0.0),
            (-2.25, 0.0, -PI / 2.0),
            (SECOND_TREE_X, -2.05, 0.0),
            (SECOND_TREE_X + 1.9, 0.0, PI / 2.0),
        ] {
            append(
                &mut vertices,
                &mut indices,
                &hut,
                Vector3::new(x, 3.4, z),
                0.9,
                yaw,
            );
        }
        Mesh {
            vertices,
            indices,
            materials: platform.materials,
        }
    })
}

impl WorldObject for Village {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }
    fn bounding_radius(&self) -> Option<f32> {
        static RADIUS: OnceLock<f32> = OnceLock::new();
        Some(*RADIUS.get_or_init(|| {
            mesh()
                .vertices
                .iter()
                .map(|v| v.position.length())
                .fold(0.0, f32::max)
        }))
    }
    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let transform = &self.transform;
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
        renderer.render_lit(framebuffer, &uniforms, mesh());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn village_and_both_roots_follow_the_orbiting_rotating_planet() {
        use crate::celestial_shaders::CelestialKind;
        use crate::objects::celestial::CelestialBody;
        use crate::scene::world::{Orbit, WorldUpdate};
        let initial = Vector3::new(300.0, 0.0, 0.0);
        let planet = CelestialBody::new(CelestialKind::Rocky, initial)
            .with_scale(120.0)
            .with_orbit(Orbit::new(
                Vector3::zero(),
                initial,
                Vector3::new(0.0, 1.0, 0.0),
                0.05,
            ));
        let mut village = Village::attached_to(planet.transform.anchor(), 120.0);
        let original = village.transform.position;
        let mut world = World::new();
        world.add(planet);
        for _ in 0..120 {
            world.update(&WorldUpdate {
                frame_time: 1.0 / 60.0,
            });
        }
        village.transform.set_local_rotation(site_rotation());
        let center = Vector3::new(300.0 * 0.1_f32.cos(), 0.0, -300.0 * 0.1_f32.sin());
        let radial = village.transform.position - center;
        let rotation = village.transform.rotation.to_matrix();
        assert!(rotate(rotation, Vector3::new(0.0, 1.0, 0.0)).dot(radial.normalize()) > 0.9999);
        assert!((radial.length() - 120.0 * 0.997).abs() < 0.001);
        let root = radial + rotate(rotation, secondary_root()) * village.transform.scale.x;
        assert!((root.length() - 120.0 * 0.997).abs() < 0.001);
        assert!((village.transform.position - original).length() > 1.0);
        assert!((village.transform.scale.x - 120.0 * SIZE_FRACTION).abs() < 0.0001);
    }

    #[test]
    fn village_site_is_forested_and_visible_during_initial_approach() {
        let site = site_direction();
        assert!(surface::forest_weight(site) > 0.35);
        assert!(is_reserved(site));
        assert!(!is_reserved(-site));
        let sunlight = Vector3::new(600.0, -180.0, 300.0).normalize();
        let approach = Vector3::new(600.0, -180.0, -1200.0).normalize();
        assert!(site.dot(sunlight) > 0.4);
        assert!(site.dot(approach) > 0.4);
        let root =
            site * 0.997 + rotate(site_rotation().to_matrix(), secondary_root()) * SIZE_FRACTION;
        assert!(surface::forest_weight(root.normalize()) > 0.1);
        assert!((root.length() - 0.997).abs() < 0.0001);
    }

    #[test]
    fn village_models_load_with_valid_geometry_and_shared_palette() {
        for name in ["platform", "hut", "bridge"] {
            let model = load_obj(&format!("assets/kashyyyk/{name}.obj")).unwrap();
            assert!(!model.indices.is_empty());
            assert_eq!(model.indices.len() % 3, 0);
            assert!(model.materials[1].diffuse_texture.is_some());
            for ids in model.indices.chunks_exact(3) {
                let [a, b, c] = [ids[0], ids[1], ids[2]].map(|id| model.vertices[id as usize]);
                let normal = (b.position - a.position).cross(c.position - a.position);
                assert!(normal.length() > 0.00001);
                assert!(normal.normalize().dot(a.normal) > 0.99);
            }
        }
        let village = mesh();
        assert!(village.indices.len() / 3 < 3500);
        assert!(village.vertices.iter().all(|v| v.material_index == 1));
    }
}
