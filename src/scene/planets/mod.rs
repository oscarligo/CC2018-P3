mod kashyyyk;

use crate::scene::world::World;

/// Registra los planetas independientes.
pub fn register_all(world: &mut World) {
    kashyyyk::registrar(world);
}
