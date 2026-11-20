//! Weapon state machine: physical key edges, continuous decay, explicit authorization.
pub use super::doomsday_audio::DoomsdayAudio;
use crate::{
    rendering::{
        framebuffer::Framebuffer,
        projection::{project_vertex, projected_sphere_radius},
        uniforms::Uniforms,
    },
    scene::world::{TravelTarget, World},
};
use raylib::prelude::*;
use std::{collections::VecDeque, f32::consts::TAU};

pub const TAP_GAIN: f32 = 0.065;
pub const BASE_DRAIN: f32 = 0.17;
pub const FINAL_DRAIN: f32 = 0.25;
pub const MIN_CHARGE_SECONDS: f32 = 15.0;
const FLIGHT_TIME: f32 = 2.4;
const RUPTURE_TIME: f32 = 1.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Charging,
    Confirm,
    Flight,
    Rupture,
    Aftermath,
    Aborting,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Tap,
    Ready,
    Abort,
    Launch,
    Arrival,
    Explosion,
}
#[derive(Default)]
pub struct Controls {
    pub activate: bool,
    pub tap_count: u8,
    pub left: bool,
    pub right: bool,
    pub toggle: bool,
    pub confirm: bool,
    pub cancel: bool,
}
impl Controls {
    pub fn read(w: &mut RaylibHandle) -> Self {
        let mut controls = Self {
            cancel: !w.is_window_focused(),
            ..Default::default()
        };
        // raylib queues physical PRESS events, excluding OS REPEAT events. Unlike
        // down-state comparisons, this preserves press/release pairs between frames.
        while let Some(key) = w.get_key_pressed() {
            match key {
                KeyboardKey::KEY_J => controls.tap_count = controls.tap_count.saturating_add(1),
                KeyboardKey::KEY_B => controls.activate = true,
                KeyboardKey::KEY_LEFT => controls.left = true,
                KeyboardKey::KEY_RIGHT => controls.right = true,
                KeyboardKey::KEY_UP | KeyboardKey::KEY_DOWN => controls.toggle = !controls.toggle,
                KeyboardKey::KEY_ENTER => controls.confirm = true,
                KeyboardKey::KEY_ESCAPE => controls.cancel = true,
                _ => {}
            }
        }
        controls
    }
}

#[derive(Default)]
pub struct Doomsday {
    pub phase: Phase,
    pub charge: f32,
    pub bay_open: f32,
    pub target: Option<TravelTarget>,
    pub launch_selected: bool,
    clock: f32,
    timer: f32,
    charge_time: f32,
    tap_flash: f32,
    taps: VecDeque<f32>,
    cues: Vec<Cue>,
    origin: Vector3,
    normal: Vector3,
    notice: Option<(&'static str, f32)>,
}

impl Doomsday {
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
    pub fn charging(&self) -> bool {
        self.phase == Phase::Charging
    }
    pub fn bomb_visible(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Charging | Phase::Confirm | Phase::Aborting
        )
    }
    pub fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }
    pub fn flight_progress(&self) -> f32 {
        (self.timer / FLIGHT_TIME).clamp(0., 1.)
    }
    fn transition(&mut self, phase: Phase) {
        self.phase = phase;
        self.timer = 0.;
    }
    fn abort(&mut self) {
        self.launch_selected = false;
        self.cues.push(Cue::Abort);
        self.transition(Phase::Aborting);
    }
    pub fn restored(&mut self) {
        self.notice = Some((
            "Sistema restaurado. Los planetas vuelven a estar disponibles.",
            4.,
        ));
    }

    pub fn update(
        &mut self,
        dt: f32,
        input: &Controls,
        aim: Option<TravelTarget>,
        world: &mut World,
        origin: Vector3,
    ) {
        self.clock += dt;
        self.timer += dt;
        self.tap_flash = (self.tap_flash - dt * 5.).max(0.);
        while self.taps.front().is_some_and(|t| self.clock - t > 1.) {
            self.taps.pop_front();
        }
        if let Some((_, remaining)) = &mut self.notice {
            *remaining -= dt;
            if *remaining <= 0. {
                self.notice = None;
            }
        }
        if let Some(t) = self.target {
            if let Some(current) = world.target_by_id(t.id) {
                self.target = Some(current);
            } else if matches!(
                self.phase,
                Phase::Charging | Phase::Confirm | Phase::Flight | Phase::Rupture
            ) {
                self.abort();
            }
        }
        let desired = if matches!(
            self.phase,
            Phase::Charging | Phase::Confirm | Phase::Flight | Phase::Rupture
        ) {
            1.
        } else {
            0.
        };
        self.bay_open += (desired - self.bay_open) * (1. - (-5. * dt).exp());
        match self.phase {
            Phase::Idle => {
                if input.activate {
                    if let Some(target) = aim {
                        self.target = Some(target);
                        self.charge = 0.;
                        self.charge_time = 0.;
                        self.taps.clear();
                        self.launch_selected = false;
                        self.transition(Phase::Charging);
                    } else {
                        self.notice = Some((
                            "Apunta a un planeta cercano. Acércate o usa 3.14 para viajar.",
                            4.,
                        ));
                    }
                }
            }
            Phase::Charging => {
                if input.cancel || input.activate {
                    self.abort();
                    return;
                }
                let count = input.tap_count.min(16);
                let pressing =
                    count > 0 || self.taps.back().is_some_and(|t| self.clock - t <= 0.35);
                if pressing && (self.charge > 0. || count > 0) {
                    self.charge_time += dt;
                }
                if count == 0 {
                    self.drain(dt);
                    if self.charge <= 0. {
                        self.charge_time = 0.;
                    }
                }
                for i in 0..count {
                    // Approximate event spacing within this frame instead of
                    // granting an artificial burst advantage at lower frame rates.
                    self.drain(dt / count as f32);
                    // The reactor must stabilize for fifteen real seconds even
                    // when the player can press faster than the required rhythm.
                    let stabilized = TAP_GAIN
                        + (1. - TAP_GAIN) * (self.charge_time / MIN_CHARGE_SECONDS).clamp(0., 1.);
                    self.charge = (self.charge + TAP_GAIN).min(stabilized);
                    self.tap_flash = 1.;
                    self.taps
                        .push_back(self.clock - dt + (i + 1) as f32 * dt / count as f32);
                    if i == 0 {
                        self.cues.push(Cue::Tap);
                    }
                    if self.charge >= 1. {
                        self.launch_selected = false;
                        self.cues.push(Cue::Ready);
                        self.transition(Phase::Confirm);
                        break;
                    }
                }
            }
            Phase::Confirm => {
                if input.cancel || input.activate {
                    self.abort();
                    return;
                }
                if input.left {
                    self.launch_selected = true;
                }
                if input.right {
                    self.launch_selected = false;
                }
                if input.toggle {
                    self.launch_selected = !self.launch_selected;
                }
                // A full separate update is required before Enter can authorize.
                if input.confirm {
                    if self.launch_selected {
                        self.origin = origin;
                        self.cues.push(Cue::Launch);
                        self.transition(Phase::Flight);
                    } else {
                        self.abort();
                    }
                }
            }
            Phase::Flight => {
                if self.timer >= FLIGHT_TIME {
                    if let Some(target) = self.target {
                        self.normal = (self.origin - target.position).normalize();
                    }
                    self.cues.push(Cue::Arrival);
                    self.transition(Phase::Rupture);
                }
            }
            Phase::Rupture => {
                if self.timer >= RUPTURE_TIME {
                    if let Some(t) = self.target.and_then(|t| world.destroy_planet(t.id)) {
                        self.target = Some(t);
                        self.cues.push(Cue::Explosion);
                        self.transition(Phase::Aftermath);
                    } else {
                        self.abort();
                    }
                }
            }
            Phase::Aftermath => {
                if self.timer > 5. {
                    self.charge = 0.;
                    self.target = None;
                    self.transition(Phase::Idle);
                }
            }
            Phase::Aborting => {
                self.charge = (self.charge - dt * 0.95).max(0.);
                if self.timer > 1.2 {
                    self.charge = 0.;
                    self.target = None;
                    self.transition(Phase::Idle);
                }
            }
        }
    }

    pub fn shake(&self) -> f32 {
        match self.phase {
            Phase::Charging => self.charge.powi(3) * 0.008,
            Phase::Rupture => 0.018,
            Phase::Aftermath => (1. - self.timer / 1.4).max(0.) * 0.04,
            _ => 0.,
        }
    }
    fn drain(&mut self, dt: f32) {
        // Exact solution of dq/dt = -(a + b*q²), clamped at an empty reactor.
        let factor = (FINAL_DRAIN / BASE_DRAIN).sqrt();
        let angle = (self.charge * factor).atan() - (BASE_DRAIN * FINAL_DRAIN).sqrt() * dt;
        self.charge = if angle > 0. { angle.tan() / factor } else { 0. };
    }
    fn project(p: Vector3, fb: &Framebuffer, u: &Uniforms) -> Option<(Vector2, f32)> {
        project_vertex(
            p,
            u.camera_pos,
            u.camera_target,
            u.camera_up,
            fb.width as f32,
            fb.height as f32,
        )
    }
    pub fn draw_effects(&self, fb: &mut Framebuffer, u: &Uniforms, origin: Vector3) {
        use super::alien::{beam, glow};
        let amber = Vector3::new(1., 0.23, 0.035);
        if matches!(self.phase, Phase::Charging | Phase::Confirm)
            && let Some((p, z)) = Self::project(origin, fb, u)
        {
            glow(fb, p, z, 10. + self.charge * 20., amber, self.charge * 0.75);
        }
        let Some(target) = self.target else {
            return;
        };
        if self.phase == Phase::Flight {
            let t = self.flight_progress();
            let normal = (self.origin - target.position).normalize();
            let end = target.position + normal * target.radius;
            let p = self.origin + (end - self.origin) * t;
            if let Some((screen, z)) = Self::project(p, fb, u) {
                let radius = (target.radius * 0.025 * 520. / z).clamp(3., 24.);
                glow(fb, screen, z, radius * 3., amber, 0.8);
                glow(fb, screen, z, radius, Vector3::one(), 1.3);
            }
            beam(
                fb,
                u,
                p - (end - self.origin).normalize() * target.radius * 0.22,
                p,
                amber,
                2.,
            );
        }
        if self.phase == Phase::Rupture {
            let progress = (self.timer / RUPTURE_TIME).clamp(0., 1.);
            let normal = self.normal;
            let reference = if normal.y.abs() < 0.9 {
                Vector3::new(0., 1., 0.)
            } else {
                Vector3::new(1., 0., 0.)
            };
            let right = normal.cross(reference).normalize();
            let up = normal.cross(right);
            for branch in 0..9 {
                let a = branch as f32 * TAU / 9.;
                let tangent = right * a.cos() + up * a.sin();
                let mut previous = target.position + normal * target.radius * 1.008;
                for segment in 1..=10 {
                    let arc = segment as f32 / 10. * progress * 2.8;
                    let jitter =
                        (segment as f32 * 3.7 + branch as f32 * 2.1).sin() * 0.035 * arc.sin();
                    let d = (normal * arc.cos() + tangent * arc.sin() + up * jitter).normalize();
                    let next = target.position + d * target.radius * 1.008;
                    beam(fb, u, previous, next, amber, 0.9 + progress);
                    previous = next;
                }
            }
            let impact = target.position + normal * target.radius * 1.015;
            if let Some((p, z)) = Self::project(impact, fb, u) {
                glow(fb, p, z, 25. + progress * 70., amber, 0.4 + progress * 0.4);
            }
        }
        if self.phase == Phase::Aftermath && self.timer < 2.3 {
            // Shockwave in the plane facing the camera, with proper scene depth.
            let normal = (u.camera_pos - target.position).normalize();
            let right = u.camera_up.cross(normal).normalize();
            let up = normal.cross(right);
            let radius = target.radius * (0.5 + self.timer * 2.6);
            for i in 0..80 {
                let at = |j: usize| {
                    let a = j as f32 * TAU / 80.;
                    target.position + (right * a.cos() + up * a.sin()) * radius
                };
                beam(
                    fb,
                    u,
                    at(i),
                    at(i + 1),
                    amber,
                    ((1. - self.timer / 2.3) * 2.).max(0.3),
                );
            }
        }
    }
}

#[path = "doomsday_hud.rs"]
pub(crate) mod hud;
#[path = "doomsday_preview.rs"]
mod preview;
pub use preview::preview;
#[cfg(test)]
#[path = "doomsday_tests.rs"]
mod tests;
