mod coruscant;
mod kashyyyk;

use crate::scene::world::World;

pub(crate) fn benchmark_kashyyyk(captures: bool) -> Result<(), Box<dyn std::error::Error>> {
    kashyyyk::benchmark(captures)
}

/// Registra los planetas independientes.
pub fn register_all(world: &mut World) {
    kashyyyk::registrar(world);
    coruscant::registrar(world);
}
