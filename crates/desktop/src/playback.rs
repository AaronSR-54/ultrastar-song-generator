//! Audio playback with rodio.
//!
//! The device-free [`PlaybackClock`] owns every rule (position, loop, stem,
//! play/pause) so it can be tested headless. [`AudioEngine`] mirrors that state
//! into rodio and degrades to a silent clock when no audio device is available.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

/// Which audio file is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stem {
    #[default]
    Mix,
    Vocals,
    Instrumental,
}

impl Stem {
    pub const ALL: [Stem; 3] = [Stem::Mix, Stem::Vocals, Stem::Instrumental];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mix => "mix",
            Self::Vocals => "vocals",
            Self::Instrumental => "instrumental",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopRange {
    pub start_s: f64,
    pub end_s: f64,
}

impl LoopRange {
    pub fn new(start_s: f64, end_s: f64) -> Self {
        Self {
            start_s: start_s.max(0.0),
            end_s: end_s.max(start_s.max(0.0)),
        }
    }

    pub fn duration_s(&self) -> f64 {
        (self.end_s - self.start_s).max(0.0)
    }
}

/// Position, transport and loop state, independent from the audio device.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackClock {
    position_s: f64,
    duration_s: f64,
    playing: bool,
    loop_range: Option<LoopRange>,
    stem: Stem,
    rate: f32,
}

impl PlaybackClock {
    pub fn new(duration_s: f64) -> Self {
        Self {
            position_s: 0.0,
            duration_s: duration_s.max(0.0),
            playing: false,
            loop_range: None,
            stem: Stem::Mix,
            rate: 1.0,
        }
    }

    pub fn position_s(&self) -> f64 {
        self.position_s
    }

    pub fn duration_s(&self) -> f64 {
        self.duration_s
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn stem(&self) -> Stem {
        self.stem
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// Playback rate: 1.0 is normal, values below slow the clock down.
    pub fn set_rate(&mut self, rate: f32) {
        if rate.is_finite() && rate > 0.0 {
            self.rate = rate.clamp(0.25, 2.0);
        }
    }

    pub fn loop_range(&self) -> Option<LoopRange> {
        self.loop_range
    }

    pub fn set_duration(&mut self, duration_s: f64) {
        self.duration_s = duration_s.max(0.0);
        self.position_s = self.position_s.min(self.duration_s);
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    /// Absolute seek, clamped to the song. Returns the applied position.
    pub fn seek(&mut self, seconds: f64) -> f64 {
        if seconds.is_finite() {
            self.position_s = seconds.clamp(0.0, self.duration_s);
        }
        self.position_s
    }

    /// Moves the playhead by `elapsed_s`; returns a seek target when a loop
    /// wrapped, so the engine can seek the device too.
    pub fn advance(&mut self, elapsed_s: f64) -> Option<f64> {
        if !self.playing || elapsed_s <= 0.0 || self.duration_s <= 0.0 {
            return None;
        }
        self.position_s += elapsed_s * self.rate as f64;
        if let Some(range) = self.loop_range {
            if self.position_s >= range.end_s {
                self.position_s = range.start_s;
                return Some(range.start_s);
            }
        }
        if self.position_s >= self.duration_s {
            self.position_s = self.duration_s;
            self.playing = false;
        }
        None
    }

    pub fn set_loop(&mut self, range: Option<LoopRange>) {
        self.loop_range = range.filter(|range| range.duration_s() > 0.0);
    }

    /// Switching stems never loses the position.
    pub fn set_stem(&mut self, stem: Stem) -> f64 {
        self.stem = stem;
        self.position_s
    }

    pub fn is_looping(&self) -> bool {
        self.loop_range.is_some()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("no audio output device: {0}")]
    NoDevice(String),
    #[error("cannot open {path}: {message}")]
    Decode { path: PathBuf, message: String },
    #[error("seek failed: {0}")]
    Seek(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// Thin rodio wrapper. Missing devices are reported but do not stop the editor.
pub struct AudioEngine {
    _stream: Option<MixerDeviceSink>,
    player: Option<Player>,
    clock: PlaybackClock,
    current_path: Option<PathBuf>,
    pub error: Option<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(stream) => {
                let player = Player::connect_new(stream.mixer());
                player.pause();
                Self {
                    _stream: Some(stream),
                    player: Some(player),
                    clock: PlaybackClock::new(0.0),
                    current_path: None,
                    error: None,
                }
            }
            Err(error) => Self {
                _stream: None,
                player: None,
                clock: PlaybackClock::new(0.0),
                current_path: None,
                error: Some(error.to_string()),
            },
        }
    }

    pub fn clock(&self) -> &PlaybackClock {
        &self.clock
    }

    pub fn clock_mut(&mut self) -> &mut PlaybackClock {
        &mut self.clock
    }

    pub fn is_available(&self) -> bool {
        self.player.is_some()
    }

    /// Loads a track and keeps the current logical position when possible.
    pub fn load(&mut self, path: &Path) -> Result<(), PlaybackError> {
        let keep = self.clock.position_s();
        let was_playing = self.clock.is_playing();

        let Some(player) = &self.player else {
            self.current_path = Some(path.to_path_buf());
            return Err(PlaybackError::NoDevice(
                self.error.clone().unwrap_or_else(|| "unknown".into()),
            ));
        };

        let file = std::fs::File::open(path)?;
        let decoder = Decoder::try_from(file).map_err(|error| PlaybackError::Decode {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let duration = decoder
            .total_duration()
            .map(|value| value.as_secs_f64())
            .unwrap_or_else(|| self.clock.duration_s());
        player.clear();
        player.append(decoder);
        player.set_speed(self.clock.rate());
        player
            .try_seek(Duration::from_secs_f64(keep))
            .map_err(|error| PlaybackError::Seek(error.to_string()))?;
        if was_playing {
            player.play();
        } else {
            player.pause();
        }

        self.clock.set_duration(duration);
        self.clock.seek(keep);
        self.current_path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    pub fn play(&mut self) {
        self.clock.play();
        if let Some(player) = &self.player {
            player.play();
        }
    }

    pub fn pause(&mut self) {
        self.clock.pause();
        if let Some(player) = &self.player {
            player.pause();
        }
    }

    pub fn toggle(&mut self) {
        if self.clock.is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn seek(&mut self, seconds: f64) {
        let position = self.clock.seek(seconds);
        if let Some(player) = &self.player {
            let _ = player.try_seek(Duration::from_secs_f64(position));
        }
    }

    /// Polls rodio's position into the clock and applies loop wrapping.
    pub fn poll(&mut self) {
        if let (Some(player), true) = (&self.player, self.clock.is_playing()) {
            let position = player.get_pos().as_secs_f64();
            if position > 0.0 {
                self.clock.seek(position);
            }
        }
    }

    pub fn set_loop(&mut self, range: Option<LoopRange>) {
        self.clock.set_loop(range);
    }

    /// Changes playback speed. rodio resamples, so pitch follows the rate.
    pub fn set_rate(&mut self, rate: f32) {
        self.clock.set_rate(rate);
        if let Some(player) = &self.player {
            player.set_speed(self.clock.rate());
        }
    }

    pub fn switch_stem(&mut self, stem: Stem, path: &Path) -> Result<(), PlaybackError> {
        self.clock.set_stem(stem);
        if self.current_path.as_deref() == Some(path) {
            return Ok(());
        }
        self.load(path)
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_seeks_keep_the_clock_exactly_in_sync() {
        let mut clock = PlaybackClock::new(200.0);
        clock.play();
        let targets = [12.5, 0.0, 199.0, 87.25, 3.0, 150.5, 42.0, 199.9, 1.0, 75.0];
        for target in targets {
            let applied = clock.seek(target);
            assert!((applied - target).abs() < 1e-9, "{target} vs {applied}");
            assert!((clock.position_s() - target).abs() < 1e-9);
        }
        assert_eq!(clock.position_s(), 75.0);
    }

    #[test]
    fn seek_is_clamped_to_the_song() {
        let mut clock = PlaybackClock::new(10.0);
        assert_eq!(clock.seek(-3.0), 0.0);
        assert_eq!(clock.seek(99.0), 10.0);
        clock.set_duration(5.0);
        assert_eq!(clock.position_s(), 5.0);
    }

    #[test]
    fn loop_wraps_back_to_the_start() {
        let mut clock = PlaybackClock::new(30.0);
        clock.play();
        clock.set_loop(Some(LoopRange::new(10.0, 12.0)));
        clock.seek(11.5);
        let wrapped = clock.advance(0.8);
        assert_eq!(wrapped, Some(10.0));
        assert_eq!(clock.position_s(), 10.0);
        assert!(clock.is_looping());

        clock.set_loop(None);
        assert!(clock.advance(0.8).is_none());
        assert_eq!(clock.position_s(), 10.8);
    }

    #[test]
    fn playback_stops_at_the_end_without_a_loop() {
        let mut clock = PlaybackClock::new(3.0);
        clock.play();
        clock.seek(2.5);
        assert!(clock.advance(1.0).is_none());
        assert_eq!(clock.position_s(), 3.0);
        assert!(!clock.is_playing());
    }

    #[test]
    fn switching_stems_preserves_the_position() {
        let mut clock = PlaybackClock::new(120.0);
        clock.seek(63.0);
        clock.play();
        assert_eq!(clock.set_stem(Stem::Vocals), 63.0);
        assert_eq!(clock.stem(), Stem::Vocals);
        assert_eq!(clock.position_s(), 63.0);
        assert!(clock.is_playing());
        assert_eq!(clock.set_stem(Stem::Instrumental), 63.0);
        assert_eq!(clock.stem(), Stem::Instrumental);
    }

    #[test]
    fn toggling_pause_does_not_move_the_position() {
        let mut clock = PlaybackClock::new(50.0);
        clock.seek(7.0);
        clock.toggle();
        assert!(clock.is_playing());
        clock.advance(1.0);
        assert!((clock.position_s() - 8.0).abs() < 1e-9);
        clock.toggle();
        assert!(!clock.is_playing());
        clock.advance(5.0);
        assert!((clock.position_s() - 8.0).abs() < 1e-9);
    }

    #[test]
    fn loop_ranges_are_normalised() {
        let range = LoopRange::new(-4.0, 9.0);
        assert_eq!(range.start_s, 0.0);
        let mut clock = PlaybackClock::new(20.0);
        clock.set_loop(Some(LoopRange::new(5.0, 5.0)));
        assert!(!clock.is_looping());
    }

    #[test]
    fn playback_rate_scales_the_clock() {
        let mut clock = PlaybackClock::new(60.0);
        clock.play();
        clock.set_rate(0.5);
        assert!((clock.rate() - 0.5).abs() < f32::EPSILON);
        clock.advance(2.0);
        assert!((clock.position_s() - 1.0).abs() < 1e-9);
        clock.set_rate(1.5);
        clock.advance(2.0);
        assert!((clock.position_s() - 4.0).abs() < 1e-9);
        // Out-of-range rates are ignored.
        clock.set_rate(0.0);
        assert!((clock.rate() - 1.5).abs() < f32::EPSILON);
        clock.set_rate(f32::NAN);
        assert!((clock.rate() - 1.5).abs() < f32::EPSILON);
    }
}
