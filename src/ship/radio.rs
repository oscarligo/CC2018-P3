//! Local car radio. Only the current MP3 is streamed; history stores paths and random offsets.
use raylib::prelude::*;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DIRECTORY: &str = "assets/radio";
pub const PANEL_HEIGHT: i32 = 146;
const PANEL_WIDTH: i32 = 352;

struct RadioTrack<'a> {
    // Field order matters: release the decoder before its backing bytes.
    music: Music<'a>,
    _encoded: Vec<u8>,
}
impl<'a> std::ops::Deref for RadioTrack<'a> {
    type Target = Music<'a>;
    fn deref(&self) -> &Self::Target {
        &self.music
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Selection {
    path: PathBuf,
    start: f32,
}

struct Shuffle {
    state: u64,
}
impl Shuffle {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            state: (seed ^ ((std::process::id() as u64) << 24)).max(1),
        }
    }
    fn unit(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 40) as f32 / 16_777_216.0
    }
    fn index(&mut self, len: usize) -> usize {
        ((self.unit() * len as f32) as usize).min(len - 1)
    }
    fn offset(&mut self, duration: f32) -> f32 {
        let earliest = (duration * 0.08).min(12.0);
        let latest = duration - (duration * 0.25).min(30.0);
        earliest + (latest - earliest).max(0.0) * self.unit()
    }
}

fn discover(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Button {
    Previous,
    Pause,
    Next,
}
fn panel(width: i32, height: i32) -> Rectangle {
    Rectangle::new(
        (width - PANEL_WIDTH - 16) as f32,
        (height - PANEL_HEIGHT - 16) as f32,
        PANEL_WIDTH as f32,
        PANEL_HEIGHT as f32,
    )
}
fn button_rect(area: Rectangle, index: usize) -> Rectangle {
    Rectangle::new(
        area.x + 12.0 + index as f32 * 110.0,
        area.y + 102.0,
        104.0,
        32.0,
    )
}
fn contains(rect: Rectangle, p: Vector2) -> bool {
    p.x >= rect.x && p.x < rect.x + rect.width && p.y >= rect.y && p.y < rect.y + rect.height
}
fn clock(seconds: f32) -> String {
    let total = seconds.max(0.0) as u32;
    format!("{:02}:{:02}", total / 60, total % 60)
}

struct RadioPlayer<'a> {
    audio: &'a RaylibAudio,
    visible: bool,
    paused: bool,
    music: Option<RadioTrack<'a>>,
    tracks: Vec<PathBuf>,
    history: Vec<Selection>,
    cursor: Option<usize>,
    broken: HashSet<PathBuf>,
    shuffle: Shuffle,
    message: String,
    volume: f32,
}

impl<'a> RadioPlayer<'a> {
    pub fn new(audio: &'a RaylibAudio) -> Self {
        Self {
            audio,
            visible: false,
            paused: false,
            music: None,
            tracks: Vec::new(),
            history: Vec::new(),
            cursor: None,
            broken: HashSet::new(),
            shuffle: Shuffle::new(),
            message: String::new(),
            volume: 0.35,
        }
    }
    fn current(&self) -> Option<&Selection> {
        self.cursor.and_then(|i| self.history.get(i))
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            if let Some(music) = &self.music {
                music.pause_stream();
            }
            return;
        }
        self.tracks = discover(Path::new(DIRECTORY)).unwrap_or_default();
        self.broken.clear();
        if self.music.is_none() {
            self.random_track();
        } else if !self.paused
            && let Some(music) = &self.music
        {
            music.resume_stream();
        }
    }

    fn open(&mut self, path: &Path, start: Option<f32>) -> Option<f32> {
        // Rust opens Unicode Windows paths correctly. Keep only this compressed MP3 alive;
        // raylib still decodes small streaming buffers rather than the entire playlist.
        let encoded = match fs::read(path) {
            Ok(bytes) if bytes.len() <= i32::MAX as usize => bytes,
            _ => {
                self.broken.insert(path.to_owned());
                return None;
            }
        };
        let mut music = match self.audio.new_music_from_memory(".mp3", &encoded) {
            Ok(music) => music,
            Err(error) => {
                eprintln!("Radio: no se pudo abrir {}: {error}", path.display());
                self.broken.insert(path.to_owned());
                return None;
            }
        };
        let duration = music.get_time_length();
        if !duration.is_finite() || duration < 0.1 {
            self.broken.insert(path.to_owned());
            return None;
        }
        let start = start
            .unwrap_or_else(|| self.shuffle.offset(duration))
            .clamp(0.0, (duration - 0.05).max(0.0));
        music.set_looping(false);
        music.set_volume(self.volume);
        if let Some(old) = self.music.take() {
            old.stop_stream();
        }
        music.play_stream();
        music.seek_stream(start);
        music.update_stream();
        if self.paused {
            music.pause_stream();
        }
        self.music = Some(RadioTrack {
            music,
            _encoded: encoded,
        });
        self.message.clear();
        Some(start)
    }

    fn remember(&mut self, selection: Selection) {
        if let Some(cursor) = self.cursor {
            self.history.truncate(cursor + 1);
        }
        self.history.push(selection);
        if self.history.len() > 128 {
            self.history.remove(0);
        }
        self.cursor = Some(self.history.len() - 1);
    }

    fn random_track(&mut self) {
        let current = self.current().map(|s| s.path.clone());
        let mut candidates = self
            .tracks
            .iter()
            .filter(|p| !self.broken.contains(*p) && Some(*p) != current.as_ref())
            .cloned()
            .collect::<Vec<_>>();
        // A one-song station can still choose a fresh point within that song.
        if candidates.is_empty() {
            candidates = self
                .tracks
                .iter()
                .filter(|p| !self.broken.contains(*p))
                .cloned()
                .collect();
        }
        while !candidates.is_empty() {
            let index = self.shuffle.index(candidates.len());
            let path = candidates.swap_remove(index);
            if let Some(start) = self.open(&path, None) {
                self.remember(Selection { path, start });
                return;
            }
        }
        self.message = if self.tracks.is_empty() {
            "Agrega MP3 en assets/radio"
        } else {
            "No hay MP3 reproducibles"
        }
        .to_owned();
        if !self.paused && self.music.as_ref().is_some_and(|m| !m.is_stream_playing()) {
            self.music = None;
        }
    }

    fn previous(&mut self) {
        let Some(current) = self.cursor else {
            self.random_track();
            return;
        };
        for index in (0..=current.saturating_sub(1)).rev() {
            let selection = self.history[index].clone();
            if self.open(&selection.path, Some(selection.start)).is_some() {
                self.cursor = Some(index);
                return;
            }
        }
    }
    fn next(&mut self) {
        if let Some(cursor) = self.cursor {
            for index in cursor + 1..self.history.len() {
                let selection = self.history[index].clone();
                if self.open(&selection.path, Some(selection.start)).is_some() {
                    self.cursor = Some(index);
                    return;
                }
            }
        }
        self.random_track();
    }
    fn pause(&mut self) {
        if self.music.is_none() {
            self.paused = false;
            self.random_track();
            return;
        }
        self.paused = !self.paused;
        if let Some(music) = &self.music {
            if self.paused {
                music.pause_stream();
            } else {
                music.resume_stream();
            }
        }
    }

    fn tick(&mut self) {
        if self.visible
            && !self.paused
            && let Some(music) = &self.music
        {
            music.update_stream();
            if !music.is_stream_playing() {
                self.next();
            }
        }
    }
    fn status(&self, revision: u64) -> RadioStatus {
        RadioStatus {
            revision,
            visible: self.visible,
            paused: self.paused,
            has_music: self.music.is_some(),
            current: self.current().cloned(),
            position: self.music.as_ref().map_or(0.0, |m| m.get_time_played()),
            duration: self.music.as_ref().map_or(0.0, |m| m.get_time_length()),
            message: self.message.clone(),
        }
    }
}

#[derive(Default)]
struct RadioStatus {
    revision: u64,
    visible: bool,
    paused: bool,
    has_music: bool,
    current: Option<Selection>,
    position: f32,
    duration: f32,
    message: String,
}
enum Request {
    Toggle,
    Control(Button),
    Shutdown,
}

/// Main-thread UI. MP3 I/O, seeking and streaming belong exclusively to the worker.
pub struct CarRadio<'scope> {
    requests: std::sync::mpsc::Sender<(u64, Request)>,
    snapshots: std::sync::mpsc::Receiver<RadioStatus>,
    worker: Option<std::thread::ScopedJoinHandle<'scope, ()>>,
    status: RadioStatus,
    revision: u64,
    visible: bool,
    paused: bool,
    hover: Option<Button>,
}
impl<'scope> CarRadio<'scope> {
    pub fn new<'env>(
        audio: &'env RaylibAudio,
        scope: &'scope std::thread::Scope<'scope, 'env>,
    ) -> Self {
        Self::with_volume(audio, scope, 0.35)
    }
    fn with_volume<'env>(
        audio: &'env RaylibAudio,
        scope: &'scope std::thread::Scope<'scope, 'env>,
        volume: f32,
    ) -> Self {
        let (requests, commands) = std::sync::mpsc::channel();
        let (updates, snapshots) = std::sync::mpsc::channel();
        let worker = scope.spawn(move || {
            let mut player = RadioPlayer::new(audio);
            player.volume = volume;
            let mut revision = 0;
            let mut published = std::time::Instant::now();
            loop {
                let changed = match commands.recv_timeout(std::time::Duration::from_millis(5)) {
                    Ok((_, Request::Shutdown))
                    | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    Ok((id, request)) => {
                        revision = id;
                        match request {
                            Request::Toggle => player.toggle(),
                            Request::Control(Button::Previous) => player.previous(),
                            Request::Control(Button::Pause) => player.pause(),
                            Request::Control(Button::Next) => player.next(),
                            Request::Shutdown => unreachable!(),
                        }
                        true
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
                };
                player.tick();
                if changed || published.elapsed() >= std::time::Duration::from_millis(50) {
                    if updates.send(player.status(revision)).is_err() {
                        break;
                    }
                    published = std::time::Instant::now();
                }
            }
            // Music and its backing bytes are released here, before the shared audio device.
        });
        Self {
            requests,
            snapshots,
            worker: Some(worker),
            status: RadioStatus::default(),
            revision: 0,
            visible: false,
            paused: false,
            hover: None,
        }
    }
    fn pending(&self) -> bool {
        self.status.revision < self.revision
    }
    fn send(&mut self, request: Request) {
        self.revision += 1;
        let _ = self.requests.send((self.revision, request));
    }
    fn poll(&mut self) {
        while let Ok(status) = self.snapshots.try_recv() {
            if status.revision >= self.revision {
                self.visible = status.visible;
                self.paused = status.paused;
                self.status = status;
            }
        }
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
    pub fn is_playing(&self) -> bool {
        self.visible && !self.paused && self.status.has_music
    }
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.send(Request::Toggle);
    }
    fn control(&mut self, button: Button) {
        if button == Button::Pause {
            self.paused = !self.paused;
        }
        self.send(Request::Control(button));
    }
    /// Capture only pointer input over the radio; no decoder work happens here.
    pub fn update(&mut self, window: &RaylibHandle) -> bool {
        self.poll();
        if !self.visible {
            return false;
        }
        let p = window.get_mouse_position();
        let area = panel(window.get_screen_width(), window.get_screen_height());
        self.hover = [Button::Previous, Button::Pause, Button::Next]
            .into_iter()
            .enumerate()
            .find_map(|(i, button)| contains(button_rect(area, i), p).then_some(button));
        if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && let Some(button) = self.hover
        {
            self.control(button);
        }
        contains(area, p)
    }
    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        if !self.visible {
            return;
        }
        let area = panel(d.get_screen_width(), d.get_screen_height());
        let x = area.x as i32;
        let y = area.y as i32;
        let blue = Color::new(139, 209, 255, 255);
        let steel = Color::new(83, 103, 125, 255);
        d.draw_rectangle(
            x + 4,
            y + 5,
            PANEL_WIDTH,
            PANEL_HEIGHT,
            Color::new(0, 0, 0, 125),
        );
        d.draw_rectangle(x, y, PANEL_WIDTH, PANEL_HEIGHT, Color::new(24, 33, 47, 250));
        d.draw_rectangle_lines(x, y, PANEL_WIDTH, PANEL_HEIGHT, steel);
        d.draw_line(
            x + 1,
            y + 1,
            x + PANEL_WIDTH - 2,
            y + 1,
            Color::new(153, 173, 192, 255),
        );
        for (sx, sy) in [
            (x + 6, y + 6),
            (x + PANEL_WIDTH - 7, y + 6),
            (x + 6, y + PANEL_HEIGHT - 7),
            (x + PANEL_WIDTH - 7, y + PANEL_HEIGHT - 7),
        ] {
            d.draw_circle(sx, sy, 2.0, steel);
            d.draw_line(sx - 1, sy, sx + 1, sy, Color::new(6, 12, 20, 255));
        }
        d.draw_text("GALAXY FM", x + 14, y + 11, 14, blue);
        d.draw_text(
            "STEREO / fm",
            x + 234,
            y + 13,
            10,
            Color::new(149, 169, 189, 255),
        );
        d.draw_rectangle(x + 12, y + 33, 328, 59, Color::new(4, 17, 29, 255));
        d.draw_rectangle_lines(x + 12, y + 33, 328, 59, Color::new(53, 90, 123, 255));
        for row in (y + 35..y + 90).step_by(4) {
            d.draw_line(x + 14, row, x + 337, row, Color::new(14, 38, 57, 150));
        }
        let title = if self.status.has_music {
            self.status
                .current
                .as_ref()
                .and_then(|s| s.path.file_stem())
                .map(|s| s.to_string_lossy().replace('_', " "))
                .unwrap_or_default()
        } else {
            self.status.message.clone()
        };
        let mut fitted = title.clone();
        while d.measure_text(&fitted, 16) > 306 && !fitted.is_empty() {
            fitted.pop();
        }
        if fitted != title {
            for _ in 0..3 {
                fitted.pop();
            }
            fitted.push_str("...");
        }
        d.draw_text(&fitted, x + 22, y + 44, 16, blue);
        let (position, duration) = (self.status.position, self.status.duration);
        let state = if self.pending() {
            "SINTONIZANDO..."
        } else if !self.status.has_music {
            "SIN SENAL"
        } else if self.paused {
            "PAUSA"
        } else {
            "EN EL AIRE"
        };
        d.draw_text(state, x + 22, y + 70, 11, Color::new(112, 167, 205, 255));
        d.draw_text(
            &format!("{} / {}", clock(position), clock(duration)),
            x + 211,
            y + 70,
            11,
            blue,
        );
        for (i, button) in [Button::Previous, Button::Pause, Button::Next]
            .into_iter()
            .enumerate()
        {
            let r = button_rect(area, i);
            let bx = r.x as i32;
            let by = r.y as i32;
            d.draw_rectangle(
                bx,
                by,
                r.width as i32,
                r.height as i32,
                if self.hover == Some(button) {
                    Color::new(42, 72, 103, 255)
                } else {
                    Color::new(37, 49, 66, 255)
                },
            );
            d.draw_rectangle_lines(bx, by, r.width as i32, r.height as i32, steel);
            let label = match button {
                Button::Previous => "ANTERIOR",
                Button::Pause if self.paused => "REANUDAR",
                Button::Pause => "PAUSAR",
                Button::Next => "SIGUIENTE",
            };
            let width = d.measure_text(label, 12);
            d.draw_text(label, bx + (104 - width) / 2, by + 11, 12, blue);
        }
    }
}
impl Drop for CarRadio<'_> {
    fn drop(&mut self) {
        let _ = self.requests.send((self.revision, Request::Shutdown));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Diagnose native MP3 operations without playing audio or opening a game window.
pub(crate) fn benchmark() {
    let audio = RaylibAudio::init_audio_device().expect("audio para medicion");
    println!("archivo,lectura_ms,decoder_ms,salto_ms,buffer_ms,total_ms");
    for path in discover(Path::new(DIRECTORY)).unwrap_or_default() {
        let start = std::time::Instant::now();
        let encoded = fs::read(&path).unwrap();
        let read = start.elapsed();
        let mark = std::time::Instant::now();
        let mut music = audio
            .new_music_from_memory(".mp3", &encoded)
            .expect("MP3 local");
        let decoder = mark.elapsed();
        music.set_volume(0.0);
        music.set_looping(false);
        music.play_stream();
        let mark = std::time::Instant::now();
        music.seek_stream(music.get_time_length() * 0.68);
        let seek = mark.elapsed();
        let mark = std::time::Instant::now();
        music.update_stream();
        let buffer = mark.elapsed();
        println!(
            "{:?},{:.3},{:.3},{:.3},{:.3},{:.3}",
            path.file_name().unwrap(),
            read.as_secs_f64() * 1000.0,
            decoder.as_secs_f64() * 1000.0,
            seek.as_secs_f64() * 1000.0,
            buffer.as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 1000.0
        );
        music.stop_stream();
    }
    measure_worker(&audio);
}

/// Exercise real MP3 transitions while measuring only work done on the caller's thread.
fn measure_worker(audio: &RaylibAudio) {
    use std::time::{Duration, Instant};
    std::thread::scope(|scope| {
        let mut radio = CarRadio::with_volume(audio, scope, 0.0);
        let mut longest = Duration::ZERO;
        let mut polls = 0;
        for step in 0..6 {
            let mark = Instant::now();
            if step == 0 {
                radio.toggle();
            } else {
                radio.control(Button::Next);
            }
            longest = longest.max(mark.elapsed());
            let deadline = Instant::now() + Duration::from_secs(15);
            while radio.pending() {
                let mark = Instant::now();
                radio.poll();
                longest = longest.max(mark.elapsed());
                polls += 1;
                assert!(
                    Instant::now() < deadline,
                    "El trabajador de radio no responde"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(radio.status.has_music);
            assert!(radio.status.current.as_ref().unwrap().start > 0.0);
        }
        // Queue controls during a slow seek. Old snapshots must not overwrite newer intent.
        radio.control(Button::Next);
        radio.control(Button::Pause);
        radio.toggle();
        let deadline = Instant::now() + Duration::from_secs(15);
        while radio.pending() {
            radio.poll();
            assert!(!radio.visible());
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(radio.paused);
        assert!(!radio.is_playing());
        println!(
            "Radio async: {polls} actualizaciones durante 6 cambios; mayor trabajo en hilo principal: {:.3} ms",
            longest.as_secs_f64() * 1000.0
        );
    });
}

/// Render the real three-button interface without showing a window or playing sound.
pub(crate) fn preview() {
    use super::vehicle::Vehicle;
    use crate::rendering::{framebuffer::Framebuffer, render::Renderer};
    let (mut window, thread) = raylib::init()
        .size(800, 600)
        .title("Radio preview")
        .hidden()
        .build();
    let audio = RaylibAudio::init_audio_device().expect("audio para captura");
    std::thread::scope(|scope| {
        let mut radio = CarRadio::with_volume(&audio, scope, 0.0);
        radio.toggle();
        let mut car = Vehicle::car();
        let u = car.preview_uniforms(0.24);
        let mut fb = Framebuffer::new(800, 600, Color::new(5, 10, 20, 255));
        crate::scene::Starfield::new().draw(&mut fb, &u);
        car.draw(&mut Renderer::default(), &mut fb, &u, 0.0);
        for _ in 0..100 {
            radio.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
            fb.present(&mut window, &thread, None, |d| radio.draw(d));
        }
        fs::create_dir_all("target/previews").expect("crear capturas");
        window.take_screenshot(&thread, "target/previews/radio.png");
        println!("target/previews/radio.png");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn random_positions_are_varied_nonzero_and_leave_time_to_listen() {
        let mut rng = Shuffle { state: 1234567 };
        for duration in [0.2, 4.0, 20.0, 240.0, 1800.0] {
            let offsets = (0..100).map(|_| rng.offset(duration)).collect::<Vec<_>>();
            assert!(
                offsets
                    .iter()
                    .all(|t| *t > 0.0 && *t <= duration - (duration * 0.25).min(30.0))
            );
            assert!(
                offsets
                    .windows(2)
                    .any(|pair| (pair[0] - pair[1]).abs() > duration * 0.01)
            );
        }
    }
    #[test]
    fn scanner_only_accepts_mp3_files() {
        let directory = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("radio-scan-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        for name in ["first.mp3", "SECOND.MP3", "cover.png"] {
            fs::write(directory.join(name), b"test").unwrap();
        }
        fs::create_dir_all(directory.join("folder.mp3")).unwrap();
        let files = discover(&directory).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.is_file()));
        for name in ["first.mp3", "SECOND.MP3", "cover.png"] {
            fs::remove_file(directory.join(name)).unwrap();
        }
        fs::remove_dir(directory.join("folder.mp3")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn radio_has_exactly_three_nonoverlapping_controls_in_the_bottom_right() {
        let area = panel(800, 600);
        assert_eq!(area.x + area.width, 784.0);
        assert_eq!(area.y + area.height, 584.0);
        for i in 0..3 {
            let r = button_rect(area, i);
            let center = Vector2::new(r.x + r.width / 2.0, r.y + r.height / 2.0);
            assert!(contains(area, center));
            assert_eq!(
                (0..3)
                    .filter(|&j| contains(button_rect(area, j), center))
                    .count(),
                1
            );
        }
    }
    #[test]
    #[ignore = "Requiere dispositivo de audio y los MP3 locales; se ejecuta sin volumen"]
    fn local_mp3_stream_seeks_and_previous_restores_history() {
        let audio = RaylibAudio::init_audio_device().unwrap();
        let mut radio = RadioPlayer::new(&audio);
        radio.volume = 0.0;
        radio.toggle();
        assert!(radio.visible);
        assert!(radio.music.is_some(), "MP3 disponibles en assets/radio");
        let first = radio.current().unwrap().clone();
        assert!(first.start > 0.0);
        let position = radio.music.as_ref().unwrap().get_time_played();
        assert!((position - first.start).abs() < 0.5);
        radio.pause();
        assert!(radio.paused);
        radio.next();
        let next = radio.current().unwrap().clone();
        if radio.tracks.len() > 1 {
            assert_ne!(first.path, next.path);
        }
        assert!(radio.paused);
        assert!(next.start > 0.0);
        radio.previous();
        assert_eq!(radio.current().unwrap(), &first);
        assert!(radio.paused);
        radio.toggle();
        assert!(!radio.visible);
        if let Some(path) = radio
            .tracks
            .iter()
            .find(|p| !p.to_string_lossy().is_ascii())
            .cloned()
        {
            assert!(
                radio.open(&path, None).is_some(),
                "Cargar MP3 con acentos sin renombrarlo"
            );
            assert!(radio.music.as_ref().unwrap().get_time_played() > 0.0);
        }
        drop(radio);
        measure_worker(&audio);
    }
}
