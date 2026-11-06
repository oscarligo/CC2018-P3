use raylib::prelude::Vector3;

pub fn value_noise_3d(point: Vector3) -> f32 {
    let cell_x = point.x.floor() as i32;
    let cell_y = point.y.floor() as i32;
    let cell_z = point.z.floor() as i32;
    let local_x = point.x - point.x.floor();
    let local_y = point.y - point.y.floor();
    let local_z = point.z - point.z.floor();
    let x = smooth_curve(local_x);
    let y = smooth_curve(local_y);
    let z = smooth_curve(local_z);

    let x00 = mix(
        hash_3d(cell_x, cell_y, cell_z),
        hash_3d(cell_x + 1, cell_y, cell_z),
        x,
    );
    let x10 = mix(
        hash_3d(cell_x, cell_y + 1, cell_z),
        hash_3d(cell_x + 1, cell_y + 1, cell_z),
        x,
    );
    let x01 = mix(
        hash_3d(cell_x, cell_y, cell_z + 1),
        hash_3d(cell_x + 1, cell_y, cell_z + 1),
        x,
    );
    let x11 = mix(
        hash_3d(cell_x, cell_y + 1, cell_z + 1),
        hash_3d(cell_x + 1, cell_y + 1, cell_z + 1),
        x,
    );
    mix(mix(x00, x10, y), mix(x01, x11, y), z)
}

pub fn fbm(mut point: Vector3, octaves: usize) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut total_amplitude = 0.0;
    for _ in 0..octaves {
        value += value_noise_3d(point) * amplitude;
        total_amplitude += amplitude;
        point *= 2.03;
        amplitude *= 0.5;
    }
    value / total_amplitude.max(f32::EPSILON)
}

pub fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let amount = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    amount * amount * (3.0 - 2.0 * amount)
}

pub fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn smooth_curve(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn hash_3d(x: i32, y: i32, z: i32) -> f32 {
    let mut value = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(z.wrapping_mul(2_147_483_647));
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    value ^= value >> 16;
    (value & 0x7fff_ffff) as f32 / 2_147_483_647.0
}

