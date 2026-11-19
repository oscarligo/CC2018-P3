//! Original, synthesized stereo effects. No downloads, audio callbacks or unsafe buffers.
use super::alien::Cue;
use raylib::prelude::{RaylibAudio, Sound};
use std::f32::consts::TAU;

pub struct AlienAudio<'a> {
    charge: Sound<'a>,
    sneeze: Sound<'a>,
    laser: Sound<'a>,
    chase: Sound<'a>,
    impact: Sound<'a>,
    paused: bool,
    resume: [bool; 5],
}
impl<'a> AlienAudio<'a> {
    pub fn new(audio: &'a RaylibAudio) -> Self {
        let load = |cue| {
            let wave = audio
                .new_wave_from_memory(".wav", &synthesize(cue))
                .expect("Sintetizar audio del ovni");
            let sound = audio
                .new_sound_from_wave(&wave)
                .expect("Cargar audio del ovni");
            sound.set_volume(0.48);
            sound
        };
        Self {
            charge: load(Cue::Charge),
            sneeze: load(Cue::Sneeze),
            laser: load(Cue::Laser),
            chase: load(Cue::Chase),
            impact: load(Cue::Impact),
            paused: false,
            resume: [false; 5],
        }
    }
    pub fn set_impact_gain(&self, gain: f32) {
        self.impact.set_volume(0.48 * gain);
    }
    pub fn play(&self, cues: &[Cue]) {
        for cue in cues {
            match cue {
                Cue::Charge => {
                    self.sneeze.stop();
                    self.charge.play();
                }
                Cue::Sneeze => {
                    self.charge.stop();
                    self.sneeze.play();
                }
                Cue::Laser => self.laser.play(),
                Cue::Impact => self.impact.play(),
                Cue::Chase => {
                    self.charge.stop();
                    self.sneeze.stop();
                    self.chase.play();
                }
            }
        }
    }
    pub fn set_paused(&mut self, paused: bool) {
        if paused == self.paused {
            return;
        }
        for (i, sound) in [
            &self.charge,
            &self.sneeze,
            &self.laser,
            &self.chase,
            &self.impact,
        ]
        .into_iter()
        .enumerate()
        {
            if paused {
                self.resume[i] = sound.is_playing();
                sound.pause();
            } else if self.resume[i] {
                sound.resume();
            }
        }
        self.paused = paused;
    }
    pub fn stop(&mut self) {
        self.charge.stop();
        self.sneeze.stop();
        self.laser.stop();
        self.chase.stop();
        self.impact.stop();
        self.resume = [false; 5];
    }
}

/// PCM16 stereo with equal-power ear gains, interaural delays, moving filtering and short echoes.
/// This simulates motion around headphones; it is not a personalized HRTF/"4D" engine.
pub fn synthesize(cue: Cue) -> Vec<u8> {
    const RATE: usize = 44_100;
    let duration = match cue {
        Cue::Charge => 1.05,
        Cue::Sneeze => 3.8,
        Cue::Laser => 0.30,
        Cue::Chase => 0.40,
        Cue::Impact => 0.85,
    };
    let count = (duration * RATE as f32) as usize;
    let mut mono = Vec::with_capacity(count);
    let mut phase = 0_f32;
    let mut noise = 0x0A11_EC47_u32;
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let p = t / duration;
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        let hiss = ((noise >> 8) as f32 / 8_388_608. - 1.) * 0.22;
        let (frequency, envelope, air) = match cue {
            Cue::Impact => (
                65.0 + 140.0 * (-t * 12.0).exp(),
                (-t * 5.0).exp() * 0.85,
                2.3 * (-t * 5.0).exp(),
            ),
            Cue::Charge => (
                220. + p * p * 1250.,
                p.powf(0.65) * 0.54,
                (t * 35.).sin().abs() * 0.40,
            ),
            Cue::Sneeze => (
                1150. + (t * 19.).sin() * 480. + (t * 5.7).sin() * 250.,
                (t / 0.025).min(1.) * (1. - p).powf(0.35) * 0.60,
                if t < 0.16 {
                    2.8 * (1. - t / 0.16)
                } else {
                    0.12
                },
            ),
            Cue::Laser => (
                340. + 2100. * (-t * 19.).exp(),
                (-t * 11.).exp() * (t / 0.005).min(1.) * 0.65,
                0.04,
            ),
            Cue::Chase => (
                600. + (p * 3.).floor() * 230.,
                (std::f32::consts::PI * p).sin().max(0.) * 0.5,
                0.,
            ),
        };
        phase = (phase + TAU * frequency / RATE as f32).rem_euclid(TAU);
        let voice = phase.sin() * 0.73 + (phase * 2.).sin() * 0.17 + (phase * 3.).sin() * 0.06;
        // Smooth endpoints eliminate clicks, including charge -> burst transitions.
        let edge = (t / 0.008).min(1.) * ((duration - t) / 0.02).min(1.);
        mono.push((voice + hiss * air) * envelope * edge);
    }
    let bytes = (count * 4) as u32;
    let mut wav = Vec::with_capacity(bytes as usize + 44);
    wav.extend(b"RIFF");
    wav.extend((bytes + 36).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend((RATE as u32).to_le_bytes());
    wav.extend((RATE as u32 * 4).to_le_bytes());
    wav.extend(4_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    let mut filtered = [0_f32; 2];
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let orbit = match cue {
            Cue::Sneeze => t * TAU * 1.65,
            Cue::Laser => t * TAU * 2.8 - 1.2,
            _ => t * TAU * 0.75,
        };
        let pan = orbit.sin() * 0.92;
        for (ear, filter) in filtered.iter_mut().enumerate() {
            let side = if ear == 0 { -1. } else { 1. };
            let delay = ((1. - side * pan) * 0.00031 * RATE as f32) as usize;
            let sample = mono.get(i.wrapping_sub(delay)).copied().unwrap_or(0.);
            let brightness = 0.25 + 0.7 * ((1. + side * pan) * 0.5);
            *filter += (sample - *filter) * brightness;
            let gain = ((1. + side * pan) * 0.5).sqrt();
            let echo_delay = ((0.065 + ear as f32 * 0.039) * RATE as f32) as usize;
            let echo = mono.get(i.wrapping_sub(echo_delay)).copied().unwrap_or(0.) * 0.13;
            let distance = 0.82 + 0.18 * orbit.cos();
            let value = ((*filter * gain + echo) * distance).clamp(-0.92, 0.92);
            wav.extend(((value * 32767.) as i16).to_le_bytes());
        }
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effects_are_valid_stereo_with_motion_and_headroom() {
        for cue in [
            Cue::Charge,
            Cue::Sneeze,
            Cue::Laser,
            Cue::Chase,
            Cue::Impact,
        ] {
            let wav = synthesize(cue);
            assert_eq!(&wav[..4], b"RIFF");
            assert_eq!(&wav[8..12], b"WAVE");
            assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2);
            let declared = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
            assert_eq!(declared, wav.len() - 44);
            let samples = wav[44..]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>();
            assert!(samples.iter().all(|s| s.unsigned_abs() < 31000));
            assert!(
                samples
                    .chunks_exact(2)
                    .any(|s| (s[0] as i32 - s[1] as i32).abs() > 2000)
            );
        }
    }
}
