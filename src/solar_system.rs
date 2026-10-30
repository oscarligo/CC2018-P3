use crate::world::World;

/// Compone el mundo a partir de los catálogos colaborativos. Al agregar una
/// entidad nueva solo se modifica `planets/mod.rs` u `objects/mod.rs`.
pub fn build_solar_system() -> World {
    let mut world = World::new();
    crate::planets::register_all(&mut world);
    crate::objects::register_all(&mut world);

    world
}
