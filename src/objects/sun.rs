use crate::framebuffer::Framebuffer;
use crate::load_objects::{Material, Mesh};
use crate::render::Renderer;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use crate::world::{WorldObject, WorldTransform, WorldUpdate};
use raylib::prelude::{Matrix, Vector3};
use std::f32::consts::{PI, TAU};

// ============================================================================
// Apariencia: Parámetros geométricos y paleta termodinámica del Sol
// ============================================================================
const SUN_RADIUS: f32 = 100.0;
// Mayor densidad geométrica para capturar los detalles del ruido superficial.
const LATITUDE_SEGMENTS: usize = 36;
const LONGITUDE_SEGMENTS: usize = 54;

// Paleta de temperatura solar (de zonas frías/manchas a fulguraciones incandescentes)
const SUNSPOT_COLOR: [f32; 3] = [0.28, 0.04, 0.01]; // Umbra / zonas magnéticas frías
const DEEP_PLASMA_COLOR: [f32; 3] = [0.78, 0.16, 0.02]; // Convección profunda (rojo carmesí)
const EQUATOR_COLOR: [f32; 3] = [1.00, 0.45, 0.03]; // Fotosfera media (naranja intenso)
const ACTIVE_PLASMA_COLOR: [f32; 3] = [1.00, 0.78, 0.12]; // Granulación activa (amarillo solar)
const FLARE_COLOR: [f32; 3] = [1.00, 0.98, 0.82]; // Fulguraciones / picos térmicos (blanco cálido)

// Micro-deformación superficial para dar relieve orgánico a la silueta
const SURFACE_DISPLACEMENT: f32 = 0.022;

// ============================================================================
// Comportamiento y Dinámica de Fluidos
// ============================================================================
const ROTATION_SPEED: f32 = 0.12;
const PLASMA_FLOW_SPEED: f32 = 0.65;
const PULSE_SPEED: f32 = 1.4;

pub struct Sun {
    mesh: Mesh,
    transform: WorldTransform,
    rotation: f32,
    time: f32,
    base_normals: Vec<Vector3>,
}

impl Sun {
    pub fn new(position: Vector3, scale: f32) -> Self {
        let (mesh, base_normals) = sphere_mesh();
        Self {
            mesh,
            transform: WorldTransform::new(position, Vector3::new(scale, scale, scale)),
            rotation: 0.0,
            time: 0.0,
            base_normals,
        }
    }

    pub fn position(&self) -> Vector3 {
        self.transform.position
    }

    pub fn set_position(&mut self, position: Vector3) {
        self.transform.position = position;
    }

    pub fn set_velocity(&mut self, velocity: Vector3) {
        self.transform.velocity = velocity;
    }

    pub fn set_scale(self, scale: f32) -> Self {
        let mut s = self;
        s.transform.scale = Vector3::new(scale, scale, scale);
        s
    }

    fn model_matrix(&self) -> Matrix {
        // Raylib aplica estas transformaciones de izquierda a derecha.
        Matrix::scale(
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        ) * Matrix::rotate_y(self.rotation)
            * Matrix::translate(
                self.transform.position.x,
                self.transform.position.y,
                self.transform.position.z,
            )
    }

    /// Actualiza dinámicamente el color y relieve de cada vértice simulando:
    /// 1. Dos capas de ruido 3D en direcciones opuestas (UV/Domain scrolling).
    /// 2. Rotación diferencial (el ecuador rota más rápido que los polos).
    /// 3. Oscurecimiento de limbo (Limb Darkening) respecto al observador.
    fn animate_surface(&mut self, camera_pos: Vector3) {
        let sun_pos = self.transform.position;
        let to_cam = normalize_vec3(Vector3::new(
            camera_pos.x - sun_pos.x,
            camera_pos.y - sun_pos.y,
            camera_pos.z - sun_pos.z,
        ));

        let sin_r = self.rotation.sin();
        let cos_r = self.rotation.cos();
        let flow = self.time * PLASMA_FLOW_SPEED;
        let breathing = 1.0 + (self.time * PULSE_SPEED).sin() * 0.008;

        for (vertex, &normal) in self.mesh.vertices.iter_mut().zip(self.base_normals.iter()) {
            // Rotación diferencial: el ecuador solar gira más rápido que las latitudes altas
            let lat_factor = 1.0 - normal.y * normal.y;
            let diff_angle = flow * 0.35 * lat_factor;
            let (sin_d, cos_d) = diff_angle.sin_cos();

            let warped_pos = Vector3::new(
                normal.x * cos_d - normal.z * sin_d,
                normal.y,
                normal.x * sin_d + normal.z * cos_d,
            );

            // Muestreo de turbulencia en 2 escalas y direcciones opuestas (FBM ligero)
            let p1 = Vector3::new(
                warped_pos.x * 3.2 + flow * 0.4,
                warped_pos.y * 3.2 - flow * 0.25,
                warped_pos.z * 3.2 + flow * 0.3,
            );
            let p2 = Vector3::new(
                warped_pos.x * 7.5 - flow * 0.55,
                warped_pos.y * 7.5 + flow * 0.45,
                warped_pos.z * 7.5 - flow * 0.35,
            );

            let macro_cells = value_noise_3d(p1);
            let micro_granules = value_noise_3d(p2);

            // Bandas convectivas latitudinales
            let bands =
                ((normal.y * 9.0 + macro_cells * 2.2 - flow * 0.5).sin() * 0.5 + 0.5) * 0.18;

            // Energía térmica combinada [0.0 .. 1.0]
            let mut heat = (macro_cells * 0.58 + micro_granules * 0.32 + bands).clamp(0.0, 1.0);

            // Contraste no lineal (curva en S suave) para acentuar manchas solares y fulguraciones
            heat = heat * heat * (3.0 - 2.0 * heat);

            // Desplazamiento físico del vértice según la actividad térmica
            let displacement = SUN_RADIUS * breathing + (heat - 0.5) * SURFACE_DISPLACEMENT;
            vertex.position = normal * displacement;

            // Normal en espacio mundo (considerando la rotación Y actual) para Limb Darkening
            let world_normal = Vector3::new(
                normal.x * cos_r + normal.z * sin_r,
                normal.y,
                -normal.x * sin_r + normal.z * cos_r,
            );

            // Efecto Limb Darkening: el centro del disco solar se ve más brillante y caliente que el borde
            let view_dot =
                (world_normal.x * to_cam.x + world_normal.y * to_cam.y + world_normal.z * to_cam.z)
                    .clamp(0.0, 1.0);
            let limb_factor = view_dot.powf(0.45);

            let effective_heat = (heat * (0.72 + 0.38 * limb_factor)).clamp(0.0, 1.0);
            let base_color = thermal_color_ramp(effective_heat);

            // Oscurecimiento suave hacia el borde para dar volumen esférico 3D sin depender de luz externa
            let edge_tint = 0.55 + 0.45 * limb_factor;
            vertex.color = Vector3::new(
                base_color.x * edge_tint,
                base_color.y * edge_tint * (0.85 + 0.15 * limb_factor),
                base_color.z * edge_tint * limb_factor,
            );
        }
    }
}

impl Sun {
    // Mantiene exactamente la firma original mutable &mut self para set_scale
    pub fn set_scale_mut(&mut self, scale: f32) {
        self.transform.scale = Vector3::new(scale, scale, scale);
    }
}

impl Default for Sun {
    fn default() -> Self {
        Self::new(Vector3::zero(), 1.5)
    }
}

impl WorldObject for Sun {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn update(&mut self, update: &WorldUpdate) {
        self.rotation = (self.rotation + ROTATION_SPEED * update.frame_time) % TAU;
        self.time += update.frame_time;
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        // Actualiza la convección de plasma y el oscurecimiento de limbo antes de rasterizar
        self.animate_surface(uniforms.camera_pos);

        let sun_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            camera_pos: uniforms.camera_pos,
            camera_target: uniforms.camera_target,
            camera_up: uniforms.camera_up,
        };
        renderer.render(
            framebuffer,
            &sun_uniforms,
            &self.mesh,
            crate::shaders::FragmentShader::Material,
        );
    }
}

fn sphere_mesh() -> (Mesh, Vec<Vector3>) {
    let row_size = LONGITUDE_SEGMENTS + 1;
    let total_vertices = (LATITUDE_SEGMENTS + 1) * row_size;
    let mut vertices = Vec::with_capacity(total_vertices);
    let mut base_normals = Vec::with_capacity(total_vertices);
    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);

    for latitude in 0..=LATITUDE_SEGMENTS {
        let v = latitude as f32 / LATITUDE_SEGMENTS as f32;
        let phi = v * PI;
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let theta = longitude as f32 / LONGITUDE_SEGMENTS as f32 * TAU;
            let normal = Vector3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());

            let mut vertex = Vertex::new(normal * SUN_RADIUS);
            let initial_heat = value_noise_3d(normal * 3.5);
            vertex.color = thermal_color_ramp(initial_heat);

            vertices.push(vertex);
            base_normals.push(normal);
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let current = (latitude * row_size + longitude) as u32;
            let next = current + row_size as u32;
            indices.extend_from_slice(&[current, next, current + 1, current + 1, next, next + 1]);
        }
    }

    (
        Mesh {
            vertices,
            indices,
            materials: vec![Material {
                diffuse_texture: None,
            }],
        },
        base_normals,
    )
}

/// Rampa de color de cuerpo negro / plasma solar con 5 paradas térmicas
fn thermal_color_ramp(t: f32) -> Vector3 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.22 {
        color_lerp(SUNSPOT_COLOR, DEEP_PLASMA_COLOR, t / 0.22)
    } else if t < 0.50 {
        color_lerp(DEEP_PLASMA_COLOR, EQUATOR_COLOR, (t - 0.22) / 0.28)
    } else if t < 0.80 {
        color_lerp(EQUATOR_COLOR, ACTIVE_PLASMA_COLOR, (t - 0.50) / 0.30)
    } else {
        color_lerp(ACTIVE_PLASMA_COLOR, FLARE_COLOR, (t - 0.80) / 0.20)
    }
}

/// Ruido 3D continuo determinista libre de texturas externas (trilinear value noise)
fn value_noise_3d(p: Vector3) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;

    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    let fz = p.z - p.z.floor();

    // Interpolación suave Hermite (smoothstep)
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let uz = fz * fz * (3.0 - 2.0 * fz);

    let c000 = hash_3d(ix, iy, iz);
    let c100 = hash_3d(ix + 1, iy, iz);
    let c010 = hash_3d(ix, iy + 1, iz);
    let c110 = hash_3d(ix + 1, iy + 1, iz);
    let c001 = hash_3d(ix, iy, iz + 1);
    let c101 = hash_3d(ix + 1, iy, iz + 1);
    let c011 = hash_3d(ix, iy + 1, iz + 1);
    let c111 = hash_3d(ix + 1, iy + 1, iz + 1);

    let x00 = c000 + (c100 - c000) * ux;
    let x10 = c010 + (c110 - c010) * ux;
    let x01 = c001 + (c101 - c001) * ux;
    let x11 = c011 + (c111 - c011) * ux;

    let y0 = x00 + (x10 - x00) * uy;
    let y1 = x01 + (x11 - x01) * uy;

    y0 + (y1 - y0) * uz
}

/// Hash entero rápido sin alocaciones ni trigonometría pesada
#[inline]
fn hash_3d(x: i32, y: i32, z: i32) -> f32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(z.wrapping_mul(2147483647));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n = n ^ (n >> 16);
    (n & 0x7fffffff) as f32 / 2147483647.0
}

#[inline]
fn normalize_vec3(v: Vector3) -> Vector3 {
    let len_sq = v.x * v.x + v.y * v.y + v.z * v.z;
    if len_sq > 1e-6 {
        let inv_len = 1.0 / len_sq.sqrt();
        Vector3::new(v.x * inv_len, v.y * inv_len, v.z * inv_len)
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    }
}

fn color_lerp(from: [f32; 3], to: [f32; 3], amount: f32) -> Vector3 {
    Vector3::new(
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
        from[2] + (to[2] - from[2]) * amount,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shaders::vertex_shader;

    #[test]
    fn rotation_and_scale_keep_the_sun_at_its_world_position() {
        let position = Vector3::new(120.0, -40.0, 500.0);
        let mut sun = Sun::new(position, 1.0);

        for (rotation, scale) in [(0.0, 2.0), (PI / 2.0, 1.0), (PI, 0.5)] {
            sun.rotation = rotation;
            sun.set_scale_mut(scale);
            let uniforms = Uniforms {
                model_matrix: sun.model_matrix(),
                camera_pos: Vector3::zero(),
                camera_target: Vector3::new(0.0, 0.0, 1.0),
                camera_up: Vector3::new(0.0, 1.0, 0.0),
            };
            let center = vertex_shader(&Vertex::new(Vector3::zero()), &uniforms);
            assert!((center.transformed_position - position).length() < 0.001);

            let surface =
                vertex_shader(&Vertex::new(Vector3::new(SUN_RADIUS, 0.0, 0.0)), &uniforms);
            let radius = (surface.transformed_position - position).length();
            assert!((radius - SUN_RADIUS * scale).abs() < 0.001);
        }
    }
}
