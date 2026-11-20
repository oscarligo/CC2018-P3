use crate::{
    physics::ObjectId,
    rendering::uniforms::Uniforms,
    scene::world::{TravelTarget, World},
};
use raylib::prelude::*;

#[derive(Default)]
pub struct PlanetFinder {
    target: Option<ObjectId>,
    previous: Option<ObjectId>,
}

impl PlanetFinder {
    pub fn toggle(&mut self, targets: &[TravelTarget]) {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize;
        self.select(targets, seed);
    }

    fn select(&mut self, targets: &[TravelTarget], seed: usize) {
        if self.target.take().is_some() {
            return;
        }
        let candidates: Vec<_> = targets
            .iter()
            .filter(|t| targets.len() == 1 || Some(t.id) != self.previous)
            .collect();
        if !candidates.is_empty() {
            self.target = Some(candidates[seed % candidates.len()].id);
            self.previous = self.target;
        }
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, world: &World, u: &Uniforms) {
        let target = self.target.and_then(|id| world.target_by_id(id));
        let label = if let Some(target) = target {
            super::doomsday::hud::draw_target_marker(d, target, u);
            let forward = (u.camera_target - u.camera_pos).normalize();
            let delta = target.position - u.camera_pos;
            let direction = if delta.dot(forward) < 0.0 {
                " (detras)"
            } else {
                ""
            };
            format!("[N] Quitar marca: {}{}", target.name, direction)
        } else {
            "[N] Buscar planeta".to_owned()
        };
        d.draw_text(&label, 16, 90, 16, Color::new(255, 119, 86, 255));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggles_off_then_selects_a_different_planet_and_handles_empty_system() {
        let targets = [0, 1].map(|id| TravelTarget {
            id: ObjectId::new(id),
            name: "Planeta",
            position: Vector3::zero(),
            radius: 1.0,
        });
        let mut finder = PlanetFinder::default();
        finder.select(&targets, 0);
        assert_eq!(finder.target, Some(targets[0].id));
        finder.select(&targets, 0);
        assert_eq!(finder.target, None);
        finder.select(&targets, 0);
        assert_eq!(finder.target, Some(targets[1].id));
        finder.select(&targets, 0);
        finder.select(&[], 0);
        assert_eq!(finder.target, None);
        finder.select(&targets[..1], 0);
        assert_eq!(finder.target, Some(targets[0].id));
    }
}
