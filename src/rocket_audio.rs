use raylib::prelude::{Music, RaylibAudio};

const ROCKET_AUDIO_PATH: &str = "assets/audio/rocket.mp3";
const ROCKET_MAX_VOLUME: f32 = 0.2;
const ROCKET_FADE_IN_SPEED: f32 = 2.5;
const ROCKET_FADE_OUT_SPEED: f32 = 1.2;
const BRAKING_AUDIO_PATH: &str = "assets/audio/braking.mp3";
const BRAKING_MAX_VOLUME: f32 = 0.05;
const BRAKING_FADE_IN_SPEED: f32 = 3.0;
const BRAKING_FADE_OUT_SPEED: f32 = 0.1;
const IDLE_AUDIO_PATH: &str = "assets/audio/idle.mp3";
const IDLE_MAX_VOLUME: f32 = 0.05;
const IDLE_FADE_IN_SPEED: f32 = 0.1;
const IDLE_FADE_OUT_SPEED: f32 = 0.01;
const MOVING_AUDIO_PATH: &str = "assets/audio/moving.mp3";
const MOVING_MAX_VOLUME: f32 = 0.12;
const MOVING_FADE_IN_RATE: f32 = 5.0;
const MOVING_FADE_OUT_RATE: f32 = 4.0;
const MOVING_SILENCE_THRESHOLD: f32 = 0.001;
const MOVING_END_FADE_SECONDS: f32 = 0.8;
const FLYING_AUDIO_PATH: &str = "assets/audio/flying.mp3";
const FLYING_MAX_VOLUME: f32 = 0.08;
const FLYING_FADE_IN_SPEED: f32 = 0.4;
const FLYING_FADE_OUT_SPEED: f32 = 0.5;

pub struct RocketAudio<'audio> {
    music: Music<'audio>,
    volume: f32,
    braking_music: Music<'audio>,
    braking_volume: f32,
    idle_music: Music<'audio>,
    idle_volume: f32,
    moving_music: Music<'audio>,
    moving_volume: f32,
    moving_was_active: bool,
    moving_completed: bool,
    moving_duration: f32,
    flying_music: Music<'audio>,
    flying_volume: f32,
}

impl<'audio> RocketAudio<'audio> {
    pub fn new(audio: &'audio RaylibAudio) -> Self {
        let mut music = audio
            .new_music(ROCKET_AUDIO_PATH)
            .expect("Failed to load rocket audio");
        music.set_looping(true);
        music.set_volume(0.0);
        let mut braking_music = audio
            .new_music(BRAKING_AUDIO_PATH)
            .expect("Failed to load braking audio");
        braking_music.set_looping(true);
        braking_music.set_volume(0.0);
        let mut idle_music = audio
            .new_music(IDLE_AUDIO_PATH)
            .expect("Failed to load idle audio");
        idle_music.set_looping(true);
        idle_music.set_volume(0.0);
        let mut moving_music = audio
            .new_music(MOVING_AUDIO_PATH)
            .expect("Failed to load moving audio");
        moving_music.set_looping(false);
        moving_music.set_volume(0.0);
        let moving_duration = moving_music.get_time_length();
        let mut flying_music = audio
            .new_music(FLYING_AUDIO_PATH)
            .expect("Failed to load flying audio");
        flying_music.set_looping(true);
        flying_music.set_volume(0.0);
        Self {
            music,
            volume: 0.0,
            braking_music,
            braking_volume: 0.0,
            idle_music,
            idle_volume: 0.0,
            moving_music,
            moving_volume: 0.0,
            moving_was_active: false,
            moving_completed: false,
            moving_duration,
            flying_music,
            flying_volume: 0.0,
        }
    }

    pub fn moving_duration(&self) -> f32 {
        self.moving_duration
    }

    pub fn update(
        &mut self,
        thrust: bool,
        braking: bool,
        moving: bool,
        can_accelerate: bool,
        speed_ratio: f32,
        frame_time: f32,
    ) {
        update_loop(
            &self.music,
            &mut self.volume,
            thrust && !braking && moving,
            ROCKET_MAX_VOLUME,
            ROCKET_FADE_IN_SPEED,
            ROCKET_FADE_OUT_SPEED,
            frame_time,
        );
        update_loop(
            &self.braking_music,
            &mut self.braking_volume,
            braking && moving,
            BRAKING_MAX_VOLUME,
            BRAKING_FADE_IN_SPEED,
            BRAKING_FADE_OUT_SPEED,
            frame_time,
        );
        update_loop(
            &self.idle_music,
            &mut self.idle_volume,
            !moving,
            IDLE_MAX_VOLUME,
            IDLE_FADE_IN_SPEED,
            IDLE_FADE_OUT_SPEED,
            frame_time,
        );
        update_loop(
            &self.flying_music,
            &mut self.flying_volume,
            moving && !thrust && !braking,
            FLYING_MAX_VOLUME,
            FLYING_FADE_IN_SPEED,
            FLYING_FADE_OUT_SPEED,
            frame_time,
        );
        self.update_moving(
            thrust && !braking,
            can_accelerate,
            speed_ratio,
            frame_time,
        );
    }

    fn update_moving(
        &mut self,
        thrust: bool,
        can_accelerate: bool,
        speed_ratio: f32,
        frame_time: f32,
    ) {
        if thrust && can_accelerate && !self.moving_was_active {
            let resume_position = self.moving_duration * speed_ratio.clamp(0.0, 1.0);
            self.moving_completed = false;
            self.moving_music.stop_stream();
            self.moving_music.seek_stream(resume_position);
            self.moving_music.play_stream();
        }

        if self.moving_music.is_stream_playing() {
            self.moving_music.update_stream();
        } else if thrust && self.moving_was_active {
            self.moving_completed = true;
        }

        let end_envelope = if thrust && self.moving_music.is_stream_playing() {
            let remaining = (self.moving_duration - self.moving_music.get_time_played()).max(0.0);
            let progress = (remaining / MOVING_END_FADE_SECONDS).clamp(0.0, 1.0);
            progress * progress * (3.0 - 2.0 * progress)
        } else if self.moving_completed {
            0.0
        } else {
            1.0
        };
        let target = if thrust {
            MOVING_MAX_VOLUME * end_envelope
        } else {
            0.0
        };
        let fade_rate = if target > self.moving_volume {
            MOVING_FADE_IN_RATE
        } else {
            MOVING_FADE_OUT_RATE
        };
        self.moving_volume = smooth_towards(self.moving_volume, target, fade_rate, frame_time);
        self.moving_music.set_volume(self.moving_volume);

        if !thrust
            && self.moving_volume <= MOVING_SILENCE_THRESHOLD
            && self.moving_music.is_stream_playing()
        {
            self.moving_volume = 0.0;
            self.moving_music.set_volume(0.0);
            self.moving_music.stop_stream();
        }
        self.moving_was_active = thrust;
    }
}

fn update_loop(
    music: &Music,
    volume: &mut f32,
    active: bool,
    max_volume: f32,
    fade_in_speed: f32,
    fade_out_speed: f32,
    frame_time: f32,
) {
    if active && !music.is_stream_playing() {
        music.play_stream();
    }
    if music.is_stream_playing() {
        music.update_stream();
    }

    let target = if active { max_volume } else { 0.0 };
    let fade_speed = if active {
        fade_in_speed
    } else {
        fade_out_speed
    };
    *volume = move_towards(*volume, target, fade_speed * frame_time);
    music.set_volume(*volume);

    if !active && *volume <= f32::EPSILON && music.is_stream_playing() {
        music.stop_stream();
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}

fn smooth_towards(current: f32, target: f32, rate: f32, frame_time: f32) -> f32 {
    target + (current - target) * (-rate * frame_time).exp()
}
