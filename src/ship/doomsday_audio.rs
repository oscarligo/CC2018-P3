//! Synthesized original effects. The reactor pitch follows actual charge, including decay.
use super::doomsday::{Cue, Doomsday};
use raylib::prelude::*;
use std::f32::consts::TAU;

pub struct DoomsdayAudio<'a> {
    hum: Sound<'a>,
    effects: Vec<Sound<'a>>,
    pitch: f32,
    gain: f32,
}
impl<'a> DoomsdayAudio<'a> {
    pub fn new(audio: &'a RaylibAudio) -> Self {
        let load = |bytes: Vec<u8>| {
            let wave = audio
                .new_wave_from_memory(".wav", &bytes)
                .expect("Sintetizar reactor");
            audio
                .new_sound_from_wave(&wave)
                .expect("Cargar sonido del reactor")
        };
        let hum = load(synthesize(None));
        hum.set_volume(0.0);
        let effects = [
            Cue::Tap,
            Cue::Ready,
            Cue::Abort,
            Cue::Launch,
            Cue::Arrival,
            Cue::Explosion,
        ]
        .into_iter()
        .map(|cue| {
            let s = load(synthesize(Some(cue)));
            s.set_volume(if cue == Cue::Tap { 0.20 } else { 0.55 });
            s
        })
        .collect();
        Self {
            hum,
            effects,
            pitch: 0.6,
            gain: 0.0,
        }
    }
    pub fn update(&mut self, weapon: &Doomsday, cues: &[Cue], dt: f32) {
        let blend = 1. - (-14. * dt).exp();
        let desired = if weapon.charging() {
            0.23 + weapon.charge * 0.15
        } else {
            0.0
        };
        self.gain += (desired - self.gain) * blend;
        self.pitch += ((0.6 + weapon.charge.powf(1.4) * 3.6) - self.pitch) * blend;
        self.hum.set_pitch(self.pitch);
        self.hum.set_volume(self.gain);
        if self.gain > 0.002 {
            if !self.hum.is_playing() {
                self.hum.play();
            }
        } else {
            self.hum.stop();
        }
        for cue in cues {
            if *cue == Cue::Explosion {
                continue;
            }
            let index = match cue {
                Cue::Tap => 0,
                Cue::Ready => 1,
                Cue::Abort => 2,
                Cue::Launch => 3,
                Cue::Arrival => 4,
                Cue::Explosion => 5,
            };
            if *cue == Cue::Tap {
                self.effects[index].set_pitch(0.8 + weapon.charge * 1.5);
            }
            self.effects[index].play();
        }
    }
}

pub fn synthesize(cue: Option<Cue>) -> Vec<u8> {
    const RATE: usize = 22_050;
    let duration = match cue {
        None => 12.,
        Some(Cue::Tap) => 0.09,
        Some(Cue::Ready) => 0.8,
        Some(Cue::Abort) => 1.2,
        Some(Cue::Launch) => 1.,
        Some(Cue::Arrival) => 0.6,
        Some(Cue::Explosion) => 3.5,
    };
    let count = (duration * RATE as f32) as usize;
    let mut pcm = Vec::with_capacity(count * 4);
    let mut phase = 0.;
    let mut seed = 0xC4E6_A123_u32;
    let mut filtered = 0.;
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let p = t / duration;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let noise = (seed >> 8) as f32 / 8_388_608. - 1.;
        filtered += (noise - filtered) * 0.065;
        let (freq, amp, hiss) = match cue {
            None => (160., 0.42 * (0.82 + 0.18 * (t * TAU * 4.).sin()), 0.025),
            Some(Cue::Tap) => (
                280. + 1100. * (-t * 48.).exp(),
                (-t * 38.).exp() * 0.6,
                0.04,
            ),
            Some(Cue::Ready) => (660. + (p * 3.).floor() * 220., (1. - p) * 0.6, 0.),
            Some(Cue::Abort) => (100. + 750. * (1. - p).powi(2), (1. - p) * 0.6, 0.15),
            Some(Cue::Launch) => (90. + 600. * p, (-p * 3.).exp() * 0.8, 0.8),
            Some(Cue::Arrival) => (70. + 150. * (-t * 14.).exp(), (-t * 6.).exp() * 0.7, 0.6),
            Some(Cue::Explosion) => (35. + 100. * (-t * 4.).exp(), (-t * 1.5).exp() * 0.85, 1.6),
        };
        phase = (phase + TAU * freq / RATE as f32) % TAU;
        let envelope = (t / 0.005).min(1.) * ((duration - t) / 0.015).min(1.);
        let tone = (phase.sin() * 0.65
            + (phase * 2.).sin() * 0.20
            + (phase * 3.).sin() * 0.10
            + filtered * hiss)
            * amp
            * envelope;
        for gain in [0.94, 1.0] {
            let sample = (tone * gain).clamp(-0.94, 0.94);
            pcm.extend_from_slice(&((sample * 32767.) as i16).to_le_bytes());
        }
    }
    let len = pcm.len() as u32;
    let mut wav = Vec::with_capacity(pcm.len() + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(len + 36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&(RATE as u32).to_le_bytes());
    wav.extend_from_slice(&(RATE as u32 * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&len.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reactor_and_all_cues_are_stereo_pcm_with_headroom() {
        for cue in [
            None,
            Some(Cue::Tap),
            Some(Cue::Ready),
            Some(Cue::Abort),
            Some(Cue::Launch),
            Some(Cue::Arrival),
            Some(Cue::Explosion),
        ] {
            let wav = synthesize(cue);
            assert_eq!(&wav[..4], b"RIFF");
            assert_eq!(&wav[8..16], b"WAVEfmt ");
            assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2);
            assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
            assert_eq!(
                u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize,
                wav.len() - 44
            );
            let samples = wav[44..]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>();
            let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!(peak > 1000 && peak < 32767, "{cue:?}: {peak}");
            assert!(samples.chunks_exact(2).any(|s| s[0] != s[1]));
        }
    }
}
