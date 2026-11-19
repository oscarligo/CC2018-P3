use crate::noise::{mix, smoothstep, value_noise_3d};
use raylib::prelude::Vector3;
use std::sync::OnceLock;

const DISTRICT_COUNT: usize = 56;
const FACE_SIZE: usize = 513;
const EAST_WARP: f32 = 0.012;
const NORTH_WARP: f32 = 0.014;

pub(super) struct District {
    pub(super) center: Vector3,
    pub(super) east: Vector3,
    north: Vector3,
    density: f32,
    pub(super) spacing: f32,
}

impl District {
    pub(super) fn north_spacing(&self) -> f32 {
        self.spacing * 1.4
    }

    pub(super) fn block_direction(&self, x: f32, z: f32) -> Vector3 {
        let project = |x: f32, z: f32| {
            self.center * (1.0 - x * x - z * z).sqrt() + self.east * x + self.north * z
        };
        let mut point = project(x, z);
        // Compensar la curvatura de las avenidas para colocar cada edificio dentro de su manzana.
        for _ in 0..2 {
            let (region, neighborhood) = urban_noise(point);
            point = project(
                x - (neighborhood - 0.5) * EAST_WARP,
                z - (region - 0.5) * NORTH_WARP,
            );
        }
        point
    }
}

#[derive(Clone, Copy)]
struct CitySample {
    metal: f32,
    streets: f32,
    rooftops: f32,
}

impl CitySample {
    fn blend(self, other: Self, amount: f32) -> Self {
        Self {
            metal: mix(self.metal, other.metal, amount),
            streets: mix(self.streets, other.streets, amount),
            rooftops: mix(self.rooftops, other.rooftops, amount),
        }
    }
}

fn district(index: usize) -> District {
    // Espiral de Fibonacci perturbada con ruido para variar el tamaño de los distritos.
    let angle = index as f32 * 2.399_963_1;
    let height = 1.0 - 2.0 * (index as f32 + 0.5) / DISTRICT_COUNT as f32;
    let radial = (1.0 - height * height).sqrt();
    let (sin, cos) = angle.sin_cos();
    let point = Vector3::new(radial * cos, height, radial * sin);
    let seed = point * 7.0 + Vector3::new(8.3, 2.5, 4.1);
    let offset = Vector3::new(
        value_noise_3d(seed),
        value_noise_3d(seed + Vector3::new(7.0, 3.0, 2.0)),
        value_noise_3d(seed + Vector3::new(3.0, 1.0, 6.0)),
    ) - Vector3::new(0.5, 0.5, 0.5);
    let center = (point + offset * 0.43).normalize();
    let east = Vector3::new(0.0, 1.0, 0.0).cross(center).normalize();
    let north = center.cross(east);
    let (sin, cos) = (offset.z * 4.0).sin_cos();
    District {
        center,
        east: east * cos + north * sin,
        north: north * cos - east * sin,
        density: value_noise_3d(seed + Vector3::new(3.9, 5.1, 2.6)),
        spacing: 0.039 + offset.x * 0.025,
    }
}

fn districts() -> &'static [District; DISTRICT_COUNT] {
    static DISTRICTS: OnceLock<[District; DISTRICT_COUNT]> = OnceLock::new();
    DISTRICTS.get_or_init(|| std::array::from_fn(district))
}

pub(super) fn nearest_district(point: Vector3) -> &'static District {
    districts()
        .iter()
        .max_by(|a, b| point.dot(a.center).total_cmp(&point.dot(b.center)))
        .expect("Coruscant tiene distritos urbanos")
}

fn urban_noise(point: Vector3) -> (f32, f32) {
    (
        value_noise_3d(point * 6.5 + Vector3::new(0.8, 6.4, 3.7)),
        value_noise_3d(point * 22.0 + Vector3::new(2.4, 7.1, 5.9)),
    )
}

fn city(point: Vector3, districts: &[District; DISTRICT_COUNT]) -> CitySample {
    let mut closest = &districts[0];
    let mut nearest = -2.0_f32;
    let mut next_nearest = -2.0_f32;
    for district in districts {
        let alignment = point.dot(district.center);
        if alignment > nearest {
            closest = district;
            next_nearest = nearest;
            nearest = alignment;
        } else {
            next_nearest = next_nearest.max(alignment);
        }
    }

    let x = point.dot(closest.east);
    let y = point.dot(closest.north);
    let distance = (x * x + y * y).sqrt();
    let (region, neighborhood) = urban_noise(point);
    let blocks = value_noise_3d(point * 94.0 + Vector3::new(9.1, 4.7, 2.6));
    let activity = smoothstep(0.25, 0.75, region * 0.62 + neighborhood * 0.38);
    let border = nearest - next_nearest;
    let inside = smoothstep(0.0005, 0.008, border);
    let hub = (1.0 - smoothstep(0.025, 0.17, distance)) * closest.density * inside;

    // Los tramos más activos enlazan distritos, con vías anulares en los centros densos.
    let connector = (1.0 - smoothstep(0.0006, 0.0042, border))
        * smoothstep(0.29, 0.67, neighborhood)
        * (0.12 + 0.34 * activity);
    let avenue = 1.0 - smoothstep(0.002, 0.010, x.abs().min(y.abs()));
    let ring_radius = closest.spacing * 2.5;
    let ring = (1.0 - smoothstep(0.0016, 0.009, (distance - ring_radius).abs()))
        * smoothstep(0.48, 0.72, closest.density);
    let street_distance = |coordinate: f32, spacing: f32| {
        let lane = coordinate / spacing;
        (lane - lane.round()).abs()
    };
    let street_x = street_distance(x + (neighborhood - 0.5) * EAST_WARP, closest.spacing);
    let street_y = street_distance(y + (region - 0.5) * NORTH_WARP, closest.north_spacing());
    let grid = (1.0 - smoothstep(0.035, 0.20, street_x.min(street_y))) * inside;

    let metal = 0.20 + neighborhood * 0.32 + blocks * 0.30 + (1.0 - grid) * 0.18;
    let streets = connector
        .max(avenue.max(ring) * inside * (0.10 + 0.38 * activity + 0.44 * hub))
        .max(grid * (0.05 + 0.48 * activity + 0.35 * hub));
    let glow = 0.025 + 0.12 * activity + 0.16 * hub;
    let rooftops = smoothstep(0.53, 0.85, blocks) * (1.0 - grid * 0.65);
    CitySample {
        metal,
        streets: (streets + glow).min(1.0),
        rooftops: rooftops * (0.06 + 0.32 * activity + 0.42 * hub) * inside,
    }
}

fn city_faces() -> &'static [Vec<CitySample>; 6] {
    static FACES: OnceLock<[Vec<CitySample>; 6]> = OnceLock::new();
    FACES.get_or_init(|| {
        let districts = districts();
        std::array::from_fn(|face| {
            let side = if face % 2 == 0 { 1.0 } else { -1.0 };
            let mut samples = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
            for row in 0..FACE_SIZE {
                let v = 2.0 * row as f32 / (FACE_SIZE - 1) as f32 - 1.0;
                for column in 0..FACE_SIZE {
                    let u = 2.0 * column as f32 / (FACE_SIZE - 1) as f32 - 1.0;
                    let point = match face / 2 {
                        0 => Vector3::new(side, u, v),
                        1 => Vector3::new(u, side, v),
                        _ => Vector3::new(u, v, side),
                    };
                    samples.push(city(point.normalize(), districts));
                }
            }
            samples
        })
    })
}

pub(super) fn prepare() {
    city_faces();
    district_map();
}

// El mapa local concentra la resolución donde la nave puede sobrevolar los edificios.
const DISTRICT_EXTENT: f32 = 0.23;

fn district_map() -> &'static Vec<CitySample> {
    static MAP: OnceLock<Vec<CitySample>> = OnceLock::new();
    MAP.get_or_init(|| {
        let site = super::district::site();
        let mut samples = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
        for row in 0..FACE_SIZE {
            for column in 0..FACE_SIZE {
                let offset = |index: usize| {
                    (2.0 * index as f32 / (FACE_SIZE - 1) as f32 - 1.0) * DISTRICT_EXTENT
                };
                let (x, z) = (offset(column), offset(row));
                let point =
                    site.center * (1.0 - x * x - z * z).sqrt() + site.east * x + site.north * z;
                let mut sample = city(point, districts());
                let (region, neighborhood) = urban_noise(point);
                let u = (x + (neighborhood - 0.5) * EAST_WARP) / site.spacing;
                let v = (z + (region - 0.5) * NORTH_WARP) / site.north_spacing();
                let edge = |value: f32| (value - value.round()).abs();
                let road = edge(u).min(edge(v));
                let curb = 1.0 - smoothstep(0.012, 0.042, (road - 0.12).abs());
                let parcel = smoothstep(0.12, 0.18, road);
                let seam = edge(u * 3.0).min(edge(v * 4.0));
                let panel = smoothstep(0.025, 0.10, seam) * parcel;
                sample.metal =
                    (0.32 + 0.52 * sample.metal + 0.24 * panel - 0.16 * curb).clamp(0.0, 1.0);
                sample.streets = (0.76 * sample.streets + 0.34 * curb).min(1.0);
                sample.rooftops += 0.11 * panel * smoothstep(0.40, 0.75, neighborhood);
                samples.push(sample);
            }
        }
        samples
    })
}

fn sample_city(point: Vector3) -> CitySample {
    // Cada cara se genera a partir de la misma dirección 3D, incluidos sus bordes.
    let (x, y, z) = (point.x.abs(), point.y.abs(), point.z.abs());
    let (face, u, v) = if x >= y && x >= z {
        (usize::from(point.x < 0.0), point.y / x, point.z / x)
    } else if y >= z {
        (2 + usize::from(point.y < 0.0), point.x / y, point.z / y)
    } else {
        (4 + usize::from(point.z < 0.0), point.x / z, point.y / z)
    };
    sample_grid(&city_faces()[face], u, v)
}

fn sample_grid(samples: &[CitySample], u: f32, v: f32) -> CitySample {
    let last = FACE_SIZE - 1;
    let u = (u + 1.0) * (last as f32 * 0.5);
    let v = (v + 1.0) * (last as f32 * 0.5);
    let (x, y) = (u as usize, v as usize);
    let (next_x, next_y) = ((x + 1).min(last), (y + 1).min(last));
    let (u, v) = (u - x as f32, v - y as f32);
    let top = samples[y * FACE_SIZE + x].blend(samples[y * FACE_SIZE + next_x], u);
    let bottom = samples[next_y * FACE_SIZE + x].blend(samples[next_y * FACE_SIZE + next_x], u);
    top.blend(bottom, v)
}

pub(super) fn shade(point: Vector3, normal: Vector3, _time: f32, light: Vector3) -> Vector3 {
    let site = super::district::site();
    let mut city = sample_city(point);
    if point.dot(site.center) > 0.94 {
        let u = point.dot(site.east) / DISTRICT_EXTENT;
        let v = point.dot(site.north) / DISTRICT_EXTENT;
        let extent = u.abs().max(v.abs());
        if extent < 1.0 {
            let local = sample_grid(district_map(), u, v);
            city = local.blend(city, smoothstep(0.82, 1.0, extent));
        }
    }
    let sun = normal.x * light.x + normal.y * light.y + normal.z * light.z;
    let daylight = sun.max(0.0);
    let night = 1.0 - smoothstep(-0.12, 0.40, sun);
    let metal = 0.10 + city.metal * (0.44 + daylight * 0.90);
    let amber = city.streets * (0.27 + 0.63 * night);
    let cool = city.rooftops * (0.20 + 0.65 * night);
    Vector3::new(
        (0.23 * metal + 0.99 * amber + 0.55 * cool).min(1.0),
        (0.27 * metal + 0.63 * amber + 0.78 * cool).min(1.0),
        (0.34 * metal + 0.29 * amber + cool).min(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_is_continuous_across_cube_edges() {
        let epsilon = 0.000_01;
        for sign in [-1.0, 1.0] {
            for along in [-0.83, -0.24, 0.31, 0.92] {
                for (a, b) in [
                    (
                        Vector3::new(sign + epsilon, sign, along),
                        Vector3::new(sign - epsilon, sign, along),
                    ),
                    (
                        Vector3::new(sign + epsilon, along, -sign),
                        Vector3::new(sign - epsilon, along, -sign),
                    ),
                    (
                        Vector3::new(along, sign + epsilon, sign),
                        Vector3::new(along, sign - epsilon, sign),
                    ),
                ] {
                    let a = sample_city(a.normalize());
                    let b = sample_city(b.normalize());
                    assert!((a.metal - b.metal).abs() < 0.006);
                    assert!((a.streets - b.streets).abs() < 0.006);
                    assert!((a.rooftops - b.rooftops).abs() < 0.006);
                }
            }
        }
    }
}
