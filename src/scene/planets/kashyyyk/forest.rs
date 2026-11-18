use super::{detail, surface, village};
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::{Mesh, load_obj};
use crate::rendering::projection::projected_sphere_radius;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::scene::world::{TransformAnchor, World, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Quaternion, Vector3};
use std::f32::consts::TAU;
use std::sync::OnceLock;

pub(super) const FOREST_SEED: u32 = 0x4B41_5348;
const TREE_COUNT: usize = 140;

struct TreeModel {
    mesh: Mesh,
    low_mesh: Mesh,
    radius: f32,
}

fn models() -> &'static [TreeModel; 2] {
    static MODELS: OnceLock<[TreeModel; 2]> = OnceLock::new();
    MODELS.get_or_init(|| {
        [
            "assets/kashyyyk/wroshyr.obj",
            "assets/kashyyyk/forest_tree.obj",
        ]
        .map(|path| {
            let mesh = load_obj(path)
                .unwrap_or_else(|error| panic!("No se pudo cargar el árbol {path}: {error}"));
            let radius = mesh
                .vertices
                .iter()
                .map(|v| v.position.length())
                .fold(0.0, f32::max);
            let low_mesh = detail::tree(path.ends_with("wroshyr.obj"));
            TreeModel {
                mesh,
                low_mesh,
                radius,
            }
        })
    })
}

pub(super) fn giant_mesh() -> &'static Mesh {
    &models()[0].mesh
}

pub(super) fn giant_low_mesh() -> &'static Mesh {
    &models()[0].low_mesh
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    direction: Vector3,
    size: f32,
    twist: f32,
    model: usize,
}

struct Random(u32);

impl Random {
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        // 24 bits: intervalo [0, 1), sin redondear u32::MAX a 1.0.
        (self.0 >> 8) as f32 / 16_777_216.0
    }
}

fn placements(seed: u32, count: usize) -> Vec<Placement> {
    let mut random = Random(seed.max(1));
    let mut trees: Vec<Placement> = Vec::with_capacity(count);
    // Límite para evitar un bucle infinito si cambian los parámetros del terreno.
    for _ in 0..count.saturating_mul(80) {
        if trees.len() == count {
            break;
        }
        let y = random.unit() * 2.0 - 1.0;
        let angle = random.unit() * TAU;
        let horizontal = (1.0 - y * y).sqrt();
        let direction = Vector3::new(horizontal * angle.cos(), y, horizontal * angle.sin());
        if village::is_reserved(direction) {
            continue;
        }
        let density = surface::forest_weight(direction);
        if density <= 0.0 || random.unit() > density {
            continue;
        }
        let model = if random.unit() < 0.13 { 0 } else { 1 };
        let size = if model == 0 {
            0.013 + random.unit() * 0.006
        } else {
            0.010 + random.unit() * 0.008
        };
        if trees.iter().any(|other| {
            let spacing = if model == 0 || other.model == 0 {
                0.12
            } else {
                0.055
            };
            (other.direction - direction).length() < spacing
        }) {
            continue;
        }
        trees.push(Placement {
            direction,
            size,
            twist: random.unit() * TAU,
            model,
        });
    }
    trees
}

pub(super) fn register(world: &mut World, parent: TransformAnchor, planet_radius: f32, lod: bool) {
    // Carga y valida ambos OBJ una sola vez, antes de crear las instancias.
    models();
    for placement in placements(FOREST_SEED, TREE_COUNT) {
        let mut tree = SurfaceTree::attached_to(parent.clone(), planet_radius, placement);
        tree.lod_enabled = lod;
        world.add(tree);
    }
}

struct SurfaceTree {
    transform: WorldTransform,
    model: usize,
    lod_enabled: bool,
    full_detail: bool,
}

impl SurfaceTree {
    fn attached_to(parent: TransformAnchor, planet_radius: f32, placement: Placement) -> Self {
        let size = planet_radius * placement.size;
        let mut transform = WorldTransform::new(Vector3::zero(), Vector3::new(size, size, size));
        // Una ligera penetración oculta la unión entre las raíces y la esfera.
        transform.set_parent(parent, placement.direction * 0.997);
        let up = Vector3::new(0.0, 1.0, 0.0);
        let align = Quaternion::from_vector3_to_vector3(up, placement.direction);
        let twist = Quaternion::from_axis_angle(up, placement.twist);
        transform.set_local_rotation((align * twist).normalize());
        Self {
            transform,
            model: placement.model,
            lod_enabled: true,
            full_detail: false,
        }
    }
}

impl WorldObject for SurfaceTree {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(models()[self.model].radius)
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let model = &models()[self.model];
        let radius = projected_sphere_radius(
            self.transform.position,
            model.radius * self.transform.scale.x.abs(),
            uniforms,
            framebuffer.height,
        );
        let full = !self.lod_enabled || detail::choose_full(self.full_detail, radius, 18.0, 12.0);
        if full != self.full_detail {
            renderer.record_lod_switch();
        }
        self.full_detail = full;
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
        renderer.render_lit(
            framebuffer,
            &uniforms,
            if full { &model.mesh } else { &model.low_mesh },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celestial_shaders::CelestialKind;
    use crate::objects::celestial::CelestialBody;
    use crate::scene::world::{Orbit, WorldUpdate};

    #[test]
    fn forest_lod_reduces_submitted_triangles_and_restores_detail_nearby() {
        use raylib::prelude::Color;
        let parent = WorldTransform::new(Vector3::zero(), Vector3::new(120.0, 120.0, 120.0));
        let mut placement = placements(FOREST_SEED, 1)[0];
        placement.model = 0;
        let mut tree = SurfaceTree::attached_to(parent.anchor(), 120.0, placement);
        let mut framebuffer = Framebuffer::new(80, 60, Color::BLACK);
        let mut renderer = Renderer::default();
        let mut uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: tree.transform.position + Vector3::new(0.0, 0.0, 2000.0),
            camera_target: tree.transform.position,
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        tree.draw(&mut renderer, &mut framebuffer, &uniforms);
        let reduced = renderer.metrics().triangles;
        tree.lod_enabled = false;
        renderer.begin_frame();
        tree.draw(&mut renderer, &mut framebuffer, &uniforms);
        assert!(reduced * 10 < renderer.metrics().triangles);
        tree.lod_enabled = true;
        uniforms.camera_pos = tree.transform.position + Vector3::new(0.0, 0.0, 30.0);
        renderer.begin_frame();
        tree.draw(&mut renderer, &mut framebuffer, &uniforms);
        assert_eq!(
            renderer.metrics().triangles,
            models()[0].mesh.indices.len() / 3
        );
        for model in models() {
            assert!(
                model
                    .low_mesh
                    .vertices
                    .iter()
                    .all(|v| v.position.length() <= model.radius + 0.001)
            );
        }
    }

    #[test]
    fn forest_is_repeatable_varied_and_confined_to_suitable_land() {
        let first = placements(FOREST_SEED, TREE_COUNT);
        assert_eq!(first, placements(FOREST_SEED, TREE_COUNT));
        assert_ne!(first, placements(FOREST_SEED + 1, TREE_COUNT));
        assert_eq!(first.len(), TREE_COUNT);
        assert!(first.iter().any(|p| p.model == 0));
        assert!(first.iter().any(|p| p.model == 1));
        for p in &first {
            assert!(surface::forest_weight(p.direction) > 0.0);
            assert!(!village::is_reserved(p.direction));
            assert!((p.direction.length() - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn obj_models_are_small_triangulated_and_have_outward_normals() {
        for (model, limit) in models().iter().zip([600, 80]) {
            assert!(model.mesh.indices.len() / 3 <= limit);
            assert_eq!(model.mesh.indices.len() % 3, 0);
            assert!(
                model
                    .mesh
                    .materials
                    .iter()
                    .any(|m| m.diffuse_texture.is_some())
            );
            for triangle in model.mesh.indices.chunks_exact(3) {
                let a = model.mesh.vertices[triangle[0] as usize];
                let b = model.mesh.vertices[triangle[1] as usize];
                let c = model.mesh.vertices[triangle[2] as usize];
                let normal = (b.position - a.position).cross(c.position - a.position);
                assert!(normal.length() > 0.00001);
                assert!(normal.normalize().dot(a.normal) > 0.99);
                assert!((a.normal.length() - 1.0).abs() < 0.00001);
            }
        }
    }

    fn direction(matrix: Matrix, point: Vector3) -> Vector3 {
        Vector3::new(
            matrix.m0 * point.x + matrix.m4 * point.y + matrix.m8 * point.z,
            matrix.m1 * point.x + matrix.m5 * point.y + matrix.m9 * point.z,
            matrix.m2 * point.x + matrix.m6 * point.y + matrix.m10 * point.z,
        )
    }

    #[test]
    fn trees_stay_radial_and_follow_planet_rotation() {
        let center = Vector3::new(100.0, 20.0, -50.0);
        let parent = CelestialBody::new(CelestialKind::Rocky, center)
            .with_scale(120.0)
            .with_orbit(Orbit::new(
                Vector3::zero(),
                center,
                Vector3::new(0.0, 1.0, 0.0),
                0.05,
            ));
        let placement = placements(FOREST_SEED, 1)[0];
        let mut tree = SurfaceTree::attached_to(parent.transform.anchor(), 120.0, placement);
        let original = tree.transform.position;
        let mut world = World::new();
        world.add(parent);
        for _ in 0..120 {
            world.update(&WorldUpdate {
                frame_time: 1.0 / 60.0,
            });
        }
        // Resuelve el árbol conservado fuera del mundo para examinar su pose.
        let up = Vector3::new(0.0, 1.0, 0.0);
        let align = Quaternion::from_vector3_to_vector3(up, placement.direction);
        tree.transform.set_local_rotation(
            (align * Quaternion::from_axis_angle(up, placement.twist)).normalize(),
        );
        let (sin, cos) = 0.1_f32.sin_cos();
        let orbit_center = Vector3::new(
            center.x * cos + center.z * sin,
            center.y,
            center.z * cos - center.x * sin,
        );
        let radial = tree.transform.position - orbit_center;
        let tree_up = direction(tree.transform.rotation.to_matrix(), up);
        assert!(tree_up.dot(radial.normalize()) > 0.9999);
        assert!((radial.length() - 120.0 * 0.997).abs() < 0.001);
        assert!((tree.transform.position - original).length() > 1.0);
        assert!((tree.transform.scale.x - 120.0 * placement.size).abs() < 0.0001);
    }
}
