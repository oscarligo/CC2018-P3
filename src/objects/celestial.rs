use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use crate::interaction::Interaction;
use crate::physics::Collider;
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::{Material, Mesh};
use crate::rendering::projection::projected_sphere_radius;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::rendering::vertex::Vertex;
use crate::scene::world::{
    LocalOrbit, Orbit, TransformAnchor, WorldObject, WorldTransform, WorldUpdate,
};
use raylib::prelude::{Matrix, Quaternion, Vector3};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;
use std::time::Instant;

const BODY_SCALE: f32 = 165.0;
const HIGH_LATITUDE_SEGMENTS: usize = 36;
const HIGH_LONGITUDE_SEGMENTS: usize = 54;
const MEDIUM_LATITUDE_SEGMENTS: usize = 18;
const MEDIUM_LONGITUDE_SEGMENTS: usize = 27;
const LOW_LATITUDE_SEGMENTS: usize = 8;
const LOW_LONGITUDE_SEGMENTS: usize = 12;
const HIGH_LOD_ENTER_RADIUS: f32 = 150.0;
const HIGH_LOD_EXIT_RADIUS: f32 = 125.0;
const MEDIUM_LOD_ENTER_RADIUS: f32 = 32.0;
const MEDIUM_LOD_EXIT_RADIUS: f32 = 26.0;
const MEDIUM_UPDATE_INTERVAL: f32 = 1.0 / 30.0;
const LOW_UPDATE_INTERVAL: f32 = 1.0 / 10.0;
const LOD_TRANSITION_SECONDS: f32 = 1.2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SphereLod {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BodyShape {
    #[default]
    Sphere,
    Rocky,
    Oblate,
    Asteroid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodyRole {
    Star,
    Planet,
    Moon,
    Asteroid,
}

struct LodTransition {
    target: SphereLod,
    elapsed: f32,
    refining: bool,
    coarse_positions: Vec<Vector3>,
    fine_positions: Vec<Vector3>,
    mesh: Mesh,
}

impl LodTransition {
    fn new(from: SphereLod, target: SphereLod, shape: BodyShape) -> Self {
        let refining = lod_detail(target) > lod_detail(from);
        let topology = if refining { target } else { from };
        let coarse = if refining { from } else { target };
        let source = body_mesh(topology, shape);
        let fine_positions = source
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>();
        let coarse_positions = source
            .vertices
            .iter()
            .map(|vertex| project_to_lod_surface(vertex.position.normalize(), coarse, shape))
            .collect::<Vec<_>>();
        let mut vertices = source.vertices.clone();
        if refining {
            for (vertex, position) in vertices.iter_mut().zip(&coarse_positions) {
                vertex.position = *position;
            }
        }
        Self {
            target,
            elapsed: 0.0,
            refining,
            coarse_positions,
            fine_positions,
            mesh: Mesh {
                vertices,
                indices: source.indices.clone(),
                materials: vec![Material {
                    diffuse_texture: None,
                }],
            },
        }
    }

    fn advance(&mut self, frame_time: f32) -> bool {
        self.elapsed += frame_time;
        let progress = (self.elapsed / LOD_TRANSITION_SECONDS).clamp(0.0, 1.0);
        let progress = progress * progress * (3.0 - 2.0 * progress);
        for ((vertex, coarse), fine) in self
            .mesh
            .vertices
            .iter_mut()
            .zip(&self.coarse_positions)
            .zip(&self.fine_positions)
        {
            vertex.position = if self.refining {
                *coarse + (*fine - *coarse) * progress
            } else {
                *fine + (*coarse - *fine) * progress
            };
        }
        self.elapsed >= LOD_TRANSITION_SECONDS
    }
}

pub struct CelestialBody {
    colors: Vec<Vector3>,
    lod: SphereLod,
    lod_transition: Option<LodTransition>,
    pub(crate) transform: WorldTransform,
    kind: CelestialKind,
    role: BodyRole,
    shape: BodyShape,
    rotation: f32,
    time: f32,
    surface_elapsed: f32,
    surface_dirty: bool,
    primary_light: bool,
    light_direction: Vector3,
    style: CelestialStyle,
}

impl CelestialBody {
    pub fn new(kind: CelestialKind, position: Vector3) -> Self {
        Self::styled(kind, position, CelestialStyle::default())
    }

    pub fn styled(kind: CelestialKind, position: Vector3, style: CelestialStyle) -> Self {
        Self::new_with_light(kind, position, false, style)
    }

    pub fn new_primary_star(position: Vector3) -> Self {
        Self::new_primary_star_styled(position, CelestialStyle::default())
    }

    pub fn new_primary_star_styled(position: Vector3, style: CelestialStyle) -> Self {
        Self::new_with_light(CelestialKind::Star, position, true, style)
    }

    pub fn with_orbit(mut self, orbit: Orbit) -> Self {
        self.transform.set_orbit(orbit);
        self
    }

    pub(crate) fn with_parent_orbit(mut self, parent: TransformAnchor, orbit: LocalOrbit) -> Self {
        self.transform.set_parent_orbit(parent, orbit);
        self
    }

    pub fn with_shape(mut self, shape: BodyShape) -> Self {
        self.shape = shape;
        if shape == BodyShape::Asteroid && self.role == BodyRole::Planet {
            self.role = BodyRole::Asteroid;
        }
        self.colors
            .resize(body_mesh(self.lod, shape).vertices.len(), Vector3::zero());
        self.update_surface();
        self
    }

    pub fn with_role(mut self, role: BodyRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.transform
            .set_local_scale(Vector3::new(scale, scale, scale));
        self
    }

    fn new_with_light(
        kind: CelestialKind,
        position: Vector3,
        primary_light: bool,
        style: CelestialStyle,
    ) -> Self {
        let lod = SphereLod::Low;
        let shape = BodyShape::Sphere;
        let mesh = body_mesh(lod, shape);
        let mut body = Self {
            colors: vec![Vector3::zero(); mesh.vertices.len()],
            lod,
            lod_transition: None,
            transform: WorldTransform::new(
                position,
                Vector3::new(BODY_SCALE, BODY_SCALE, BODY_SCALE),
            ),
            kind,
            role: if primary_light {
                BodyRole::Star
            } else {
                BodyRole::Planet
            },
            shape,
            rotation: 0.0,
            time: 0.0,
            surface_elapsed: 0.0,
            surface_dirty: false,
            primary_light,
            light_direction: Vector3::new(0.0, 1.0, 0.0),
            style,
        };
        body.update_surface();
        body
    }

    fn model_matrix(&self) -> Matrix {
        Matrix::scale(
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        ) * self.transform.rotation.to_matrix()
            * Matrix::translate(
                self.transform.position.x,
                self.transform.position.y,
                self.transform.position.z,
            )
    }

    fn update_surface(&mut self) {
        let rotation = self.transform.rotation.to_matrix();
        let base_colors = match self.kind {
            CelestialKind::Rocky if self.shape == BodyShape::Asteroid => {
                Some(asteroid_base_colors(self.lod))
            }
            CelestialKind::Rocky => Some(rocky_base_colors(self.lod)),
            CelestialKind::Star | CelestialKind::GasGiant => None,
        };
        for (index, (color, vertex)) in self
            .colors
            .iter_mut()
            .zip(&body_mesh(self.lod, self.shape).vertices)
            .enumerate()
        {
            let point = vertex.position.normalize();
            let normal = vertex.normal;
            let world_normal = Vector3::new(
                rotation.m0 * normal.x + rotation.m4 * normal.y + rotation.m8 * normal.z,
                rotation.m1 * normal.x + rotation.m5 * normal.y + rotation.m9 * normal.z,
                rotation.m2 * normal.x + rotation.m6 * normal.y + rotation.m10 * normal.z,
            );
            *color = crate::celestial_shaders::shade(
                self.kind,
                point,
                world_normal,
                self.time,
                base_colors.map(|colors| colors[index]),
                self.light_direction,
                self.style,
            );
        }
    }

    fn select_lod(&mut self, projected_radius: f32) -> bool {
        if self.lod_transition.is_some() {
            return false;
        }
        let lod = match self.lod {
            SphereLod::High if projected_radius < MEDIUM_LOD_EXIT_RADIUS => SphereLod::Low,
            SphereLod::High if projected_radius < HIGH_LOD_EXIT_RADIUS => SphereLod::Medium,
            SphereLod::Medium if projected_radius >= HIGH_LOD_ENTER_RADIUS => SphereLod::High,
            SphereLod::Medium if projected_radius < MEDIUM_LOD_EXIT_RADIUS => SphereLod::Low,
            SphereLod::Low if projected_radius >= HIGH_LOD_ENTER_RADIUS => SphereLod::High,
            SphereLod::Low if projected_radius >= MEDIUM_LOD_ENTER_RADIUS => SphereLod::Medium,
            lod => lod,
        };
        if lod == self.lod {
            return false;
        }
        let transition = LodTransition::new(self.lod, lod, self.shape);
        if transition.refining {
            self.lod = lod;
            self.colors
                .resize(body_mesh(lod, self.shape).vertices.len(), Vector3::zero());
            self.surface_dirty = true;
        }
        self.lod_transition = Some(transition);
        true
    }

    fn advance_lod_transition(&mut self, frame_time: f32) {
        let Some(transition) = &mut self.lod_transition else {
            return;
        };
        if !transition.advance(frame_time) {
            return;
        }
        let target = transition.target;
        self.lod_transition = None;
        if self.lod != target {
            self.lod = target;
            self.colors.resize(
                body_mesh(target, self.shape).vertices.len(),
                Vector3::zero(),
            );
            self.surface_dirty = true;
        }
    }

    fn surface_update_interval(&self) -> f32 {
        match self.lod {
            SphereLod::High => 0.0,
            SphereLod::Medium => MEDIUM_UPDATE_INTERVAL,
            SphereLod::Low => LOW_UPDATE_INTERVAL,
        }
    }

    fn update_surface_if_needed(&mut self) -> bool {
        if !self.surface_dirty && self.surface_elapsed < self.surface_update_interval() {
            return false;
        }
        self.update_surface();
        self.surface_elapsed = 0.0;
        self.surface_dirty = false;
        true
    }

    fn set_light_position(&mut self, light_position: Option<Vector3>) {
        let Some(direction) = light_position.map(|position| position - self.transform.position)
        else {
            return;
        };
        if direction.length() <= f32::EPSILON {
            return;
        }
        let direction = direction.normalize();
        if (direction - self.light_direction).length() > 0.001 {
            self.light_direction = direction;
            self.surface_dirty = true;
        }
    }
}

impl WorldObject for CelestialBody {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn update(&mut self, update: &WorldUpdate) {
        self.time += update.frame_time * self.style.animation_speed;
        self.rotation = (self.rotation + self.style.rotation_speed * update.frame_time) % TAU;
        self.transform
            .set_local_rotation(Quaternion::from_axis_angle(
                Vector3::new(0.0, 1.0, 0.0),
                self.rotation,
            ));
        self.surface_elapsed += update.frame_time;
        self.advance_lod_transition(update.frame_time);
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(shape_bound(self.shape))
    }

    fn collider(&self) -> Option<Collider> {
        Some(Collider::Sphere {
            radius: shape_bound(self.shape),
        })
    }

    fn interaction(&self) -> Option<Interaction> {
        Some(match self.role {
            BodyRole::Star => Interaction {
                prompt: "[F] Examinar estrella",
                response: "La estrella emite la luz principal del sistema.",
            },
            BodyRole::Planet if matches!(self.kind, CelestialKind::GasGiant) => Interaction {
                prompt: "[F] Examinar gigante gaseoso",
                response: "Un gigante gaseoso rodeado por un sistema de anillos.",
            },
            BodyRole::Planet => Interaction {
                prompt: "[F] Examinar planeta rocoso",
                response: "Un mundo rocoso con superficie procedural y atmosfera.",
            },
            BodyRole::Moon => Interaction {
                prompt: "[F] Examinar luna",
                response: "Una luna mineral que mantiene una orbita local alrededor de su planeta.",
            },
            BodyRole::Asteroid => Interaction {
                prompt: "[F] Examinar asteroide",
                response: "Un cuerpo pequeno, alargado y cubierto de roca irregular.",
            },
        })
    }

    fn light_position(&self) -> Option<Vector3> {
        self.primary_light.then_some(self.transform.position)
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let scale = self
            .transform
            .scale
            .x
            .abs()
            .max(self.transform.scale.y.abs())
            .max(self.transform.scale.z.abs())
            * shape_bound(self.shape);
        let radius =
            projected_sphere_radius(self.transform.position, scale, uniforms, framebuffer.height);
        if self.select_lod(radius) {
            renderer.record_lod_switch();
        }
        self.set_light_position(uniforms.light_position);
        let surface_start = Instant::now();
        if self.update_surface_if_needed() {
            renderer.record_surface_update(surface_start.elapsed());
        }
        let body_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            camera_pos: uniforms.camera_pos,
            camera_target: uniforms.camera_target,
            camera_up: uniforms.camera_up,
            light_position: uniforms.light_position,
        };
        let mesh = self.lod_transition.as_ref().map_or_else(
            || body_mesh(self.lod, self.shape),
            |transition| &transition.mesh,
        );
        renderer.render_colored(framebuffer, &body_uniforms, mesh, &self.colors);
    }
}

fn lod_detail(lod: SphereLod) -> u8 {
    match lod {
        SphereLod::Low => 0,
        SphereLod::Medium => 1,
        SphereLod::High => 2,
    }
}

fn lod_segments(lod: SphereLod) -> (usize, usize) {
    match lod {
        SphereLod::High => (HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS),
        SphereLod::Medium => (MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS),
        SphereLod::Low => (LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS),
    }
}

fn project_to_lod_surface(direction: Vector3, lod: SphereLod, shape: BodyShape) -> Vector3 {
    let (latitude_segments, longitude_segments) = lod_segments(lod);
    let latitude = direction.y.clamp(-1.0, 1.0).acos() / PI * latitude_segments as f32;
    let longitude =
        direction.z.atan2(direction.x).rem_euclid(TAU) / TAU * longitude_segments as f32;
    let latitude_index = (latitude.floor() as usize).min(latitude_segments - 1);
    let longitude_index = (longitude.floor() as usize).min(longitude_segments - 1);
    let latitude_fraction = latitude - latitude_index as f32;
    let longitude_fraction = longitude - longitude_index as f32;
    let row = longitude_segments + 1;
    let current = latitude_index * row + longitude_index;
    let next = current + row;
    let mesh = body_mesh(lod, shape);
    let (a, b, c) = if latitude_fraction + longitude_fraction <= 1.0 {
        (current, current + 1, next)
    } else {
        (current + 1, next + 1, next)
    };
    let a = mesh.vertices[a].position;
    let b = mesh.vertices[b].position;
    let c = mesh.vertices[c].position;
    let normal = (b - a).cross(c - a);
    let denominator = normal.dot(direction);
    if denominator.abs() <= f32::EPSILON {
        direction * shape_radius(direction, shape)
    } else {
        direction * (normal.dot(a) / denominator)
    }
}

pub(crate) fn body_mesh(lod: SphereLod, shape: BodyShape) -> &'static Mesh {
    match shape {
        BodyShape::Sphere => sphere_mesh(lod),
        BodyShape::Rocky => rocky_mesh(lod),
        BodyShape::Oblate => oblate_mesh(lod),
        BodyShape::Asteroid => asteroid_mesh(lod),
    }
}

fn oblate_mesh(lod: SphereLod) -> &'static Mesh {
    static HIGH: OnceLock<Mesh> = OnceLock::new();
    static MEDIUM: OnceLock<Mesh> = OnceLock::new();
    static LOW: OnceLock<Mesh> = OnceLock::new();
    match lod {
        SphereLod::High => {
            HIGH.get_or_init(|| build_oblate_mesh(HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS))
        }
        SphereLod::Medium => MEDIUM
            .get_or_init(|| build_oblate_mesh(MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS)),
        SphereLod::Low => {
            LOW.get_or_init(|| build_oblate_mesh(LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS))
        }
    }
}

fn asteroid_mesh(lod: SphereLod) -> &'static Mesh {
    static HIGH: OnceLock<Mesh> = OnceLock::new();
    static MEDIUM: OnceLock<Mesh> = OnceLock::new();
    static LOW: OnceLock<Mesh> = OnceLock::new();
    match lod {
        SphereLod::High => HIGH
            .get_or_init(|| build_asteroid_mesh(HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS)),
        SphereLod::Medium => MEDIUM.get_or_init(|| {
            build_asteroid_mesh(MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS)
        }),
        SphereLod::Low => {
            LOW.get_or_init(|| build_asteroid_mesh(LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS))
        }
    }
}

fn rocky_mesh(lod: SphereLod) -> &'static Mesh {
    static HIGH: OnceLock<Mesh> = OnceLock::new();
    static MEDIUM: OnceLock<Mesh> = OnceLock::new();
    static LOW: OnceLock<Mesh> = OnceLock::new();
    match lod {
        SphereLod::High => {
            HIGH.get_or_init(|| build_rocky_mesh(HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS))
        }
        SphereLod::Medium => MEDIUM
            .get_or_init(|| build_rocky_mesh(MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS)),
        SphereLod::Low => {
            LOW.get_or_init(|| build_rocky_mesh(LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS))
        }
    }
}

pub(crate) fn shape_radius(direction: Vector3, shape: BodyShape) -> f32 {
    match shape {
        BodyShape::Sphere => 1.0,
        BodyShape::Rocky => {
            let broad = crate::noise::fbm(direction * 1.7 + Vector3::new(2.4, -1.1, 0.7), 4);
            let detail = crate::noise::fbm(direction * 7.3 + Vector3::new(-0.8, 3.2, 1.6), 3);
            0.88 + broad * 0.20 + detail * 0.04
        }
        BodyShape::Oblate => {
            const EQUATORIAL_RADIUS: f32 = 1.05;
            const POLAR_RADIUS: f32 = 0.82;
            1.0 / ((direction.x * direction.x + direction.z * direction.z)
                / (EQUATORIAL_RADIUS * EQUATORIAL_RADIUS)
                + direction.y * direction.y / (POLAR_RADIUS * POLAR_RADIUS))
                .sqrt()
        }
        BodyShape::Asteroid => {
            const AXIS_X: f32 = 1.28;
            const AXIS_Y: f32 = 0.72;
            const AXIS_Z: f32 = 0.88;
            let ellipsoid = 1.0
                / (direction.x * direction.x / (AXIS_X * AXIS_X)
                    + direction.y * direction.y / (AXIS_Y * AXIS_Y)
                    + direction.z * direction.z / (AXIS_Z * AXIS_Z))
                    .sqrt();
            let broad = crate::noise::fbm(direction * 2.3 + Vector3::new(-3.1, 0.8, 2.6), 4);
            let detail = crate::noise::fbm(direction * 9.1 + Vector3::new(1.7, -2.2, 0.4), 2);
            ellipsoid * (0.82 + broad * 0.28 + detail * 0.08)
        }
    }
}

fn shape_bound(shape: BodyShape) -> f32 {
    match shape {
        BodyShape::Sphere => 1.0,
        BodyShape::Rocky => 1.12,
        BodyShape::Oblate => 1.05,
        BodyShape::Asteroid => 1.52,
    }
}

fn build_rocky_mesh(latitude_segments: usize, longitude_segments: usize) -> Mesh {
    let mut mesh = build_sphere_mesh(latitude_segments, longitude_segments);
    for vertex in &mut mesh.vertices {
        vertex.position = vertex.normal * shape_radius(vertex.normal, BodyShape::Rocky);
    }

    recalculate_normals(&mut mesh);
    mesh
}

fn build_oblate_mesh(latitude_segments: usize, longitude_segments: usize) -> Mesh {
    let mut mesh = build_sphere_mesh(latitude_segments, longitude_segments);
    for vertex in &mut mesh.vertices {
        vertex.position = Vector3::new(
            vertex.position.x * 1.05,
            vertex.position.y * 0.82,
            vertex.position.z * 1.05,
        );
    }
    recalculate_normals(&mut mesh);
    mesh
}

fn build_asteroid_mesh(latitude_segments: usize, longitude_segments: usize) -> Mesh {
    let mut mesh = build_sphere_mesh(latitude_segments, longitude_segments);
    for vertex in &mut mesh.vertices {
        vertex.position = vertex.normal * shape_radius(vertex.normal, BodyShape::Asteroid);
    }
    recalculate_normals(&mut mesh);
    mesh
}

fn recalculate_normals(mesh: &mut Mesh) {
    let mut normals = vec![Vector3::zero(); mesh.vertices.len()];
    for triangle in mesh.indices.chunks_exact(3) {
        let a = mesh.vertices[triangle[0] as usize].position;
        let b = mesh.vertices[triangle[1] as usize].position;
        let c = mesh.vertices[triangle[2] as usize].position;
        let normal = (b - a).cross(c - a);
        for index in triangle {
            normals[*index as usize] += normal;
        }
    }
    for (vertex, normal) in mesh.vertices.iter_mut().zip(normals) {
        vertex.normal = if normal.length() > f32::EPSILON {
            normal.normalize()
        } else {
            vertex.position.normalize()
        };
    }
}

pub(crate) fn sphere_mesh(lod: SphereLod) -> &'static Mesh {
    static HIGH: OnceLock<Mesh> = OnceLock::new();
    static MEDIUM: OnceLock<Mesh> = OnceLock::new();
    static LOW: OnceLock<Mesh> = OnceLock::new();
    match lod {
        SphereLod::High => {
            HIGH.get_or_init(|| build_sphere_mesh(HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS))
        }
        SphereLod::Medium => MEDIUM
            .get_or_init(|| build_sphere_mesh(MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS)),
        SphereLod::Low => {
            LOW.get_or_init(|| build_sphere_mesh(LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS))
        }
    }
}

fn rocky_base_colors(lod: SphereLod) -> &'static [Vector3] {
    static HIGH: OnceLock<Vec<Vector3>> = OnceLock::new();
    static MEDIUM: OnceLock<Vec<Vector3>> = OnceLock::new();
    static LOW: OnceLock<Vec<Vector3>> = OnceLock::new();
    let colors = match lod {
        SphereLod::High => HIGH.get_or_init(|| build_rocky_base_colors(SphereLod::High)),
        SphereLod::Medium => MEDIUM.get_or_init(|| build_rocky_base_colors(SphereLod::Medium)),
        SphereLod::Low => LOW.get_or_init(|| build_rocky_base_colors(SphereLod::Low)),
    };
    colors.as_slice()
}

fn asteroid_base_colors(lod: SphereLod) -> &'static [Vector3] {
    static HIGH: OnceLock<Vec<Vector3>> = OnceLock::new();
    static MEDIUM: OnceLock<Vec<Vector3>> = OnceLock::new();
    static LOW: OnceLock<Vec<Vector3>> = OnceLock::new();
    let colors = match lod {
        SphereLod::High => HIGH.get_or_init(|| build_asteroid_base_colors(SphereLod::High)),
        SphereLod::Medium => MEDIUM.get_or_init(|| build_asteroid_base_colors(SphereLod::Medium)),
        SphereLod::Low => LOW.get_or_init(|| build_asteroid_base_colors(SphereLod::Low)),
    };
    colors.as_slice()
}

fn build_asteroid_base_colors(lod: SphereLod) -> Vec<Vector3> {
    body_mesh(lod, BodyShape::Asteroid)
        .vertices
        .iter()
        .map(|vertex| {
            let point = vertex.position.normalize();
            let mineral = crate::noise::fbm(point * 6.2 + Vector3::new(1.3, -2.7, 0.9), 3);
            let dark = Vector3::new(0.22, 0.20, 0.18);
            let light = Vector3::new(0.52, 0.47, 0.40);
            dark + (light - dark) * mineral
        })
        .collect()
}

fn build_rocky_base_colors(lod: SphereLod) -> Vec<Vector3> {
    sphere_mesh(lod)
        .vertices
        .iter()
        .map(|vertex| {
            crate::celestial_shaders::base_color(CelestialKind::Rocky, vertex.normal)
                .expect("rocky bodies provide a base color")
        })
        .collect()
}

fn build_sphere_mesh(latitude_segments: usize, longitude_segments: usize) -> Mesh {
    let row_size = longitude_segments + 1;
    let mut vertices = Vec::with_capacity((latitude_segments + 1) * row_size);
    let mut indices = Vec::with_capacity(latitude_segments * longitude_segments * 6);

    for latitude in 0..=latitude_segments {
        let phi = latitude as f32 / latitude_segments as f32 * PI;
        for longitude in 0..=longitude_segments {
            let theta = longitude as f32 / longitude_segments as f32 * TAU;
            let normal = Vector3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            let mut vertex = Vertex::new(normal);
            vertex.normal = normal;
            vertices.push(vertex);
        }
    }

    for latitude in 0..latitude_segments {
        for longitude in 0..longitude_segments {
            let current = (latitude * row_size + longitude) as u32;
            let next = current + row_size as u32;
            indices.extend_from_slice(&[current, current + 1, next, current + 1, next + 1, next]);
        }
    }

    Mesh {
        vertices,
        indices,
        materials: vec![Material {
            diffuse_texture: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sphere_has_the_expected_topology() {
        let mesh = sphere_mesh(SphereLod::High);
        assert_eq!(
            mesh.vertices.len(),
            (HIGH_LATITUDE_SEGMENTS + 1) * (HIGH_LONGITUDE_SEGMENTS + 1)
        );
        assert_eq!(
            mesh.indices.len(),
            HIGH_LATITUDE_SEGMENTS * HIGH_LONGITUDE_SEGMENTS * 6
        );
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.normal.length() - 1.0).abs() < 0.001)
        );
        assert!(mesh.indices.chunks_exact(3).all(|indices| {
            let a = mesh.vertices[indices[0] as usize].position;
            let b = mesh.vertices[indices[1] as usize].position;
            let c = mesh.vertices[indices[2] as usize].position;
            let face_normal = (b - a).cross(c - a);
            face_normal.length() < 0.001 || face_normal.dot(a + b + c) > 0.0
        }));
    }

    #[test]
    fn lod_transitions_keep_silhouette_error_near_one_pixel() {
        fn silhouette_error(radius: f32, longitude_segments: usize) -> f32 {
            radius * (1.0 - (PI / longitude_segments as f32).cos())
        }

        assert!(silhouette_error(MEDIUM_LOD_ENTER_RADIUS, LOW_LONGITUDE_SEGMENTS) <= 1.1);
        assert!(silhouette_error(HIGH_LOD_ENTER_RADIUS, MEDIUM_LONGITUDE_SEGMENTS) <= 1.1);
    }

    #[test]
    fn celestial_bodies_share_the_same_geometry() {
        assert!(std::ptr::eq(
            sphere_mesh(SphereLod::High),
            sphere_mesh(SphereLod::High)
        ));
        let body = CelestialBody::new(CelestialKind::Rocky, Vector3::zero());
        assert_eq!(
            body.colors.len(),
            sphere_mesh(SphereLod::Low).vertices.len()
        );
        assert!(std::ptr::eq(
            rocky_base_colors(SphereLod::Low),
            rocky_base_colors(SphereLod::Low)
        ));
    }

    #[test]
    fn lod_selection_uses_projected_size() {
        let mut body = CelestialBody::new(CelestialKind::Rocky, Vector3::zero());
        body.select_lod(MEDIUM_LOD_ENTER_RADIUS);
        assert_eq!(body.lod, SphereLod::Medium);
        body.advance_lod_transition(LOD_TRANSITION_SECONDS);
        body.select_lod(HIGH_LOD_ENTER_RADIUS);
        assert_eq!(body.lod, SphereLod::High);
        assert_eq!(
            body.colors.len(),
            sphere_mesh(SphereLod::High).vertices.len()
        );
    }

    #[test]
    fn lod_hysteresis_prevents_threshold_oscillation() {
        let mut body = CelestialBody::new(CelestialKind::Rocky, Vector3::zero());
        assert!(!body.select_lod(MEDIUM_LOD_ENTER_RADIUS - 1.0));
        assert!(body.select_lod(MEDIUM_LOD_ENTER_RADIUS));
        body.advance_lod_transition(LOD_TRANSITION_SECONDS);
        assert!(!body.select_lod(MEDIUM_LOD_EXIT_RADIUS + 1.0));
        assert!(body.select_lod(MEDIUM_LOD_EXIT_RADIUS - 1.0));
        body.advance_lod_transition(LOD_TRANSITION_SECONDS);

        body.select_lod(HIGH_LOD_ENTER_RADIUS);
        body.advance_lod_transition(LOD_TRANSITION_SECONDS);
        assert_eq!(body.lod, SphereLod::High);
        assert!(!body.select_lod(HIGH_LOD_EXIT_RADIUS + 1.0));
        assert!(body.select_lod(HIGH_LOD_EXIT_RADIUS - 1.0));
        body.advance_lod_transition(LOD_TRANSITION_SECONDS);
        assert_eq!(body.lod, SphereLod::Medium);
    }

    #[test]
    fn lod_transition_morphs_between_both_surfaces() {
        let mut transition =
            LodTransition::new(SphereLod::Low, SphereLod::Medium, BodyShape::Sphere);
        let index = 5 * (MEDIUM_LONGITUDE_SEGMENTS + 1) + 3;
        let fine = transition.mesh.vertices[index].normal;
        let coarse = project_to_lod_surface(fine, SphereLod::Low, BodyShape::Sphere);

        assert!((transition.mesh.vertices[index].position - coarse).length() < 0.0001);
        assert!((fine - coarse).length() > 0.0001);
        assert!(transition.advance(LOD_TRANSITION_SECONDS));
        assert!((transition.mesh.vertices[index].position - fine).length() < 0.0001);
    }

    #[test]
    fn rocky_shape_is_static_shared_and_non_spherical() {
        let mesh = body_mesh(SphereLod::High, BodyShape::Rocky);
        assert!(std::ptr::eq(
            mesh,
            body_mesh(SphereLod::High, BodyShape::Rocky)
        ));
        let (minimum, maximum) = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.length())
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(minimum, maximum), radius| (minimum.min(radius), maximum.max(radius)),
            );
        assert!(maximum - minimum > 0.05);
        assert!(maximum <= shape_bound(BodyShape::Rocky));
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.normal.length() - 1.0).abs() < 0.001)
        );
    }

    #[test]
    fn oblate_shape_has_a_wider_equator_than_poles() {
        let mesh = body_mesh(SphereLod::High, BodyShape::Oblate);
        let equatorial_radius = mesh
            .vertices
            .iter()
            .map(|vertex| {
                (vertex.position.x * vertex.position.x + vertex.position.z * vertex.position.z)
                    .sqrt()
            })
            .fold(0.0, f32::max);
        let polar_radius = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.y.abs())
            .fold(0.0, f32::max);

        assert!((equatorial_radius - 1.05).abs() < 0.001);
        assert!((polar_radius - 0.82).abs() < 0.001);
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.normal.length() - 1.0).abs() < 0.001)
        );
    }

    #[test]
    fn asteroid_shape_is_elongated_irregular_and_bounded() {
        let mesh = body_mesh(SphereLod::High, BodyShape::Asteroid);
        let extent_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.x.abs())
            .fold(0.0, f32::max);
        let extent_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.y.abs())
            .fold(0.0, f32::max);
        let maximum_radius = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.length())
            .fold(0.0, f32::max);

        assert!(extent_x > extent_y * 1.4);
        assert!(maximum_radius <= shape_bound(BodyShape::Asteroid));
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.normal.length() - 1.0).abs() < 0.001)
        );
    }

    #[test]
    fn low_lod_updates_at_a_reduced_rate() {
        let mut body = CelestialBody::new(CelestialKind::GasGiant, Vector3::zero());
        body.surface_elapsed = LOW_UPDATE_INTERVAL * 0.5;
        assert!(!body.update_surface_if_needed());
        body.surface_elapsed = LOW_UPDATE_INTERVAL;
        assert!(body.update_surface_if_needed());
    }

    #[test]
    fn only_primary_star_exposes_its_light_position() {
        let position = Vector3::new(1.0, 2.0, 3.0);
        let primary = CelestialBody::new_primary_star(position);
        let secondary = CelestialBody::new(CelestialKind::Star, position);
        assert_eq!(primary.light_position(), Some(position));
        assert_eq!(secondary.light_position(), None);
    }

    #[test]
    fn moon_role_has_its_own_interaction() {
        let moon = CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
            .with_shape(BodyShape::Asteroid)
            .with_role(BodyRole::Moon);

        assert_eq!(moon.interaction().unwrap().prompt, "[F] Examinar luna");
    }
}
