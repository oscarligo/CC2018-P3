use crate::scene::world::World;
use raylib::prelude::{Music, RaylibAudio, Vector3};

pub struct AmbientAudio<'a> {
    tracks: [Music<'a>; 3],
    explosion: Music<'a>,
    volumes: [f32; 3],
    destruction_count: u64,
}

// Choose the closest surface within three planet radii or 1.5 sun radii.
fn ambience(planet: Option<(f32, f32)>, sun: Option<(f32, f32)>) -> usize {
    let near = |body: Option<(f32, f32)>, range: f32| {
        body.filter(|(distance, radius)| *distance <= radius * range)
    };
    match (near(planet, 3.0), near(sun, 1.5)) {
        (Some((planet, _)), Some((sun, _))) if sun < planet => 2,
        (Some(_), _) => 1,
        (_, Some(_)) => 2,
        _ => 0,
    }
}

impl<'a> AmbientAudio<'a> {
    pub fn new(audio: &'a RaylibAudio) -> Self {
        let tracks = ["space", "planet", "sun"].map(|name| {
            let mut music = audio
                .new_music(&format!("assets/audio/{name}.mp3"))
                .expect("Cargar ambientacion");
            music.set_looping(true);
            music.set_volume(0.0);
            music
        });
        let mut explosion = audio
            .new_music("assets/audio/explosion.mp3")
            .expect("Cargar explosion");
        explosion.set_looping(false);
        explosion.set_volume(0.55);
        Self {
            tracks,
            explosion,
            volumes: [0.0; 3],
            destruction_count: 0,
        }
    }

    pub fn update(&mut self, world: &World, position: Vector3, radio_on: bool, dt: f32) {
        let exploded = world.destruction_count != self.destruction_count;
        self.destruction_count = world.destruction_count;
        let planet = world.navigation_target(None, position).map(|body| {
            (
                (body.position - position).length() - body.radius,
                body.radius,
            )
        });
        let sun = world
            .primary_light_bounds()
            .map(|(center, radius)| ((center - position).length() - radius, radius));
        let next = (!radio_on).then(|| ambience(planet, sun));
        for (index, (track, volume)) in self.tracks.iter().zip(&mut self.volumes).enumerate() {
            let active = next == Some(index);
            *volume = fade_volume(*volume, active, dt);
            track.set_volume(*volume);
            if active && !track.is_stream_playing() {
                track.play_stream();
            }
            if !active && *volume == 0.0 {
                track.stop_stream();
            } else if track.is_stream_playing() {
                track.update_stream();
            }
        }
        self.explosion
            .set_volume(if radio_on { 0.11 } else { 0.55 });
        if exploded {
            self.explosion.stop_stream();
            self.explosion.play_stream();
        }
        if self.explosion.is_stream_playing() {
            self.explosion.update_stream();
        }
    }
}

// A full fade takes two seconds, regardless of frame rate.
fn fade_volume(current: f32, active: bool, dt: f32) -> f32 {
    let target = if active { 0.25 } else { 0.0 };
    let step = 0.125 * dt.max(0.0);
    current + (target - current).clamp(-step, step)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fades_are_gradual_reversible_and_frame_rate_independent() {
        let mut volume = 0.0;
        for _ in 0..60 {
            volume = fade_volume(volume, true, 1.0 / 60.0);
        }
        assert!((volume - 0.125).abs() < 0.00001);
        assert!((volume - fade_volume(0.0, true, 1.0)).abs() < 0.00001);
        assert!(fade_volume(volume, false, 0.1) < volume);
        assert_eq!(fade_volume(volume, false, 2.0), 0.0);
        assert_eq!(fade_volume(volume, true, 3.0), 0.25);
        assert_eq!(fade_volume(volume, true, -1.0), volume);
    }

    #[test]
    fn ambience_follows_nearest_surface_and_proximity() {
        assert_eq!(ambience(None, None), 0);
        assert_eq!(ambience(Some((301., 100.)), None), 0);
        assert_eq!(ambience(Some((300., 100.)), None), 1);
        assert_eq!(ambience(None, Some((50., 100.))), 2);
        assert_eq!(ambience(None, Some((150., 100.))), 2);
        assert_eq!(ambience(None, Some((151., 100.))), 0);
        assert_eq!(ambience(Some((20., 10.)), Some((50., 100.))), 1);
        assert_eq!(ambience(Some((20., 10.)), Some((5., 100.))), 2);
    }
}
