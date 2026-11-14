//! Energy and sonority analysis used to discard Whisper hallucinations.
//!
//! Whisper tends to emit text for instrumental passages. The vocal stem gives
//! us a cheap and reliable signal: if a segment has almost no energy, or no
//! periodicity in the voice range, the text is not singing.

use serde::{Deserialize, Serialize};

/// A word produced by the ASR engine, before filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawWord {
    pub text: String,
    pub start_s: f64,
    pub end_s: f64,
    pub confidence: f32,
}

/// A segment produced by the ASR engine, before filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSegment {
    pub start_s: f64,
    pub end_s: f64,
    pub no_speech_probability: f32,
    pub words: Vec<RawWord>,
}

impl RawSegment {
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SegmentStats {
    pub rms: f32,
    pub peak: f32,
    pub voiced_fraction: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HallucinationOptions {
    /// Segments below this RMS are considered instrumental or silent.
    pub min_rms: f32,
    /// Minimum fraction of periodic frames in the voice range.
    pub min_voiced_fraction: f64,
    /// Segments whose no-speech probability is above this are discarded.
    pub max_no_speech_probability: f32,
    /// How many identical consecutive lines are allowed.
    pub max_repetitions: usize,
    /// Discard well-known Whisper filler lines.
    pub drop_known_phrases: bool,
}

impl Default for HallucinationOptions {
    fn default() -> Self {
        Self {
            min_rms: 0.004,
            min_voiced_fraction: 0.10,
            max_no_speech_probability: 0.6,
            max_repetitions: 2,
            drop_known_phrases: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscardReport {
    pub silent: usize,
    pub no_speech: usize,
    pub repeated: usize,
    pub known_phrase: usize,
    pub empty: usize,
}

impl DiscardReport {
    pub fn total(&self) -> usize {
        self.silent + self.no_speech + self.repeated + self.known_phrase + self.empty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterOutcome {
    pub segments: Vec<RawSegment>,
    pub report: DiscardReport,
}

/// Frame-level energy and periodicity of the vocal stem.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyTrack {
    sample_rate: u32,
    hop_s: f64,
    rms: Vec<f32>,
    peak: Vec<f32>,
    voiced: Vec<bool>,
}

impl EnergyTrack {
    /// Analyses mono samples. The signal is decimated to 8 kHz for the
    /// autocorrelation pass, which keeps the cost low for a full song.
    pub fn analyze(samples: &[f32], sample_rate: u32) -> Self {
        let hop_s = 0.04;
        let factor = (sample_rate / 8_000).max(1) as usize;
        let decimated: Vec<f32> = samples
            .chunks(factor)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect();
        let decimated_rate = (sample_rate as usize / factor) as u32;
        let frame = (decimated_rate as f64 * hop_s).round() as usize;
        if frame < 8 || decimated.is_empty() {
            return Self {
                sample_rate,
                hop_s,
                rms: Vec::new(),
                peak: Vec::new(),
                voiced: Vec::new(),
            };
        }

        let min_lag = (decimated_rate / 1_000).max(2) as usize;
        let max_lag = ((decimated_rate / 80) as usize).min(frame / 2);
        let mut rms = Vec::new();
        let mut peaks = Vec::new();
        let mut voiced = Vec::new();

        for start in (0..decimated.len().saturating_sub(frame / 2)).step_by(frame) {
            let end = (start + frame).min(decimated.len());
            let window = &decimated[start..end];
            let energy =
                window.iter().map(|value| value * value).sum::<f32>() / window.len() as f32;
            let frame_rms = energy.sqrt();
            let frame_peak = window
                .iter()
                .fold(0.0_f32, |peak, value| peak.max(value.abs()));
            rms.push(frame_rms);
            peaks.push(frame_peak);

            let is_voiced = if frame_rms < 0.001 {
                false
            } else {
                let mean = window.iter().sum::<f32>() / window.len() as f32;
                let centered: Vec<f32> = window.iter().map(|value| value - mean).collect();
                let zero = centered.iter().map(|value| value * value).sum::<f32>();
                if zero <= f32::EPSILON || max_lag <= min_lag {
                    false
                } else {
                    let mut best = 0.0_f32;
                    for lag in min_lag..=max_lag {
                        let mut sum = 0.0_f32;
                        for index in 0..centered.len() - lag {
                            sum += centered[index] * centered[index + lag];
                        }
                        best = best.max((sum / zero).clamp(0.0, 1.0));
                    }
                    best > 0.35
                }
            };
            voiced.push(is_voiced);
        }

        Self {
            sample_rate,
            hop_s,
            rms,
            peak: peaks,
            voiced,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.rms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rms.is_empty()
    }

    pub fn stats(&self, start_s: f64, end_s: f64) -> SegmentStats {
        if self.rms.is_empty() || end_s <= start_s {
            return SegmentStats::default();
        }
        let start = (start_s / self.hop_s).floor().max(0.0) as usize;
        let end = ((end_s / self.hop_s).ceil().max(0.0) as usize).max(start + 1);
        let end = end.min(self.rms.len());
        if start >= end {
            return SegmentStats::default();
        }
        let frames = &self.rms[start..end];
        let rms = frames.iter().sum::<f32>() / frames.len() as f32;
        let peak = self.peak[start..end]
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);
        let voiced = self.voiced[start..end]
            .iter()
            .filter(|value| **value)
            .count() as f64
            / (end - start) as f64;
        SegmentStats {
            rms,
            peak,
            voiced_fraction: voiced,
        }
    }
}

/// Removes segments that cannot be singing.
pub fn filter_segments(
    segments: &[RawSegment],
    track: &EnergyTrack,
    options: HallucinationOptions,
) -> FilterOutcome {
    let mut report = DiscardReport::default();
    let mut kept: Vec<RawSegment> = Vec::new();
    let mut previous_text = String::new();
    let mut run_length = 0_usize;

    for segment in segments {
        if segment.words.is_empty() || segment.text().trim().is_empty() {
            report.empty += 1;
            continue;
        }
        if segment.no_speech_probability > options.max_no_speech_probability {
            report.no_speech += 1;
            continue;
        }
        let stats = track.stats(segment.start_s, segment.end_s);
        if stats.rms < options.min_rms {
            report.silent += 1;
            continue;
        }
        if stats.voiced_fraction < options.min_voiced_fraction {
            report.silent += 1;
            continue;
        }

        let normalized = normalize_text(&segment.text());
        if options.drop_known_phrases && is_known_hallucination(&normalized) {
            report.known_phrase += 1;
            continue;
        }

        if normalized == previous_text {
            run_length += 1;
        } else {
            previous_text = normalized;
            run_length = 1;
        }
        if run_length > options.max_repetitions {
            report.repeated += 1;
            continue;
        }

        kept.push(segment.clone());
    }

    FilterOutcome {
        segments: kept,
        report,
    }
}

/// Lowercase, strip punctuation and collapse whitespace.
pub fn normalize_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut last_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() || character == '\'' {
            normalized.push(character);
            last_space = false;
        } else if !last_space && !normalized.is_empty() {
            normalized.push(' ');
            last_space = true;
        }
    }
    normalized.trim().to_string()
}

/// Filler lines Whisper emits on silence. Matching is done after normalising.
pub fn is_known_hallucination(normalized: &str) -> bool {
    const PHRASES: &[&str] = &[
        "gracias por ver el video",
        "gracias por ver el vídeo",
        "subtitulos realizados por la comunidad de amara org",
        "subtitulos por la comunidad de amara org",
        "thanks for watching",
        "thank you for watching",
        "please subscribe",
        "subscribe to my channel",
        "subtitles by the amara org community",
        "amara org",
        "music",
        "applause",
        "laughter",
        "silence",
    ];
    PHRASES.contains(&normalized)
        || normalized.starts_with("subtitulos realizados por")
        || normalized.starts_with("subtitles by")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f64, seconds: f64, sample_rate: u32) -> Vec<f32> {
        let samples = (sample_rate as f64 * seconds) as usize;
        (0..samples)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * hz as f32 * time).sin() * 0.2
            })
            .collect()
    }

    fn noise(seconds: f64, sample_rate: u32) -> Vec<f32> {
        let mut state = 0x1234_5678_u32;
        let samples = (sample_rate as f64 * seconds) as usize;
        (0..samples)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 8_388_608.0 - 1.0) * 0.2
            })
            .collect()
    }

    fn segment(start_s: f64, end_s: f64, text: &str) -> RawSegment {
        RawSegment {
            start_s,
            end_s,
            no_speech_probability: 0.1,
            words: vec![RawWord {
                text: text.into(),
                start_s,
                end_s,
                confidence: 0.9,
            }],
        }
    }

    #[test]
    fn silence_has_no_energy_or_sonority() {
        let track = EnergyTrack::analyze(&vec![0.0; 16_000], 16_000);
        let stats = track.stats(0.0, 1.0);
        assert_eq!(stats.rms, 0.0);
        assert_eq!(stats.voiced_fraction, 0.0);
    }

    #[test]
    fn a_tone_is_voiced_but_noise_is_not() {
        let tone_track = EnergyTrack::analyze(&tone(220.0, 1.0, 16_000), 16_000);
        let tone_stats = tone_track.stats(0.0, 1.0);
        assert!(tone_stats.rms > 0.1);
        assert!(tone_stats.voiced_fraction > 0.8, "{tone_stats:?}");

        let noise_track = EnergyTrack::analyze(&noise(1.0, 16_000), 16_000);
        let noise_stats = noise_track.stats(0.0, 1.0);
        assert!(noise_stats.rms > 0.05);
        assert!(noise_stats.voiced_fraction < 0.2, "{noise_stats:?}");
    }

    #[test]
    fn instrumental_and_silent_segments_are_discarded() {
        let mut samples = vec![0.0_f32; 16_000];
        samples.extend(tone(220.0, 1.0, 16_000));
        let track = EnergyTrack::analyze(&samples, 16_000);

        let segments = vec![segment(0.0, 0.5, "no hay voz"), segment(1.0, 1.5, "canta")];
        let outcome = filter_segments(&segments, &track, HallucinationOptions::default());
        assert_eq!(outcome.segments.len(), 1);
        assert_eq!(outcome.segments[0].text(), "canta");
        assert_eq!(outcome.report.silent, 1);
    }

    #[test]
    fn no_speech_probability_discards_segments() {
        let samples = tone(220.0, 2.0, 16_000);
        let track = EnergyTrack::analyze(&samples, 16_000);
        let mut bad = segment(0.5, 1.0, "texto inventado");
        bad.no_speech_probability = 0.95;
        let outcome = filter_segments(&[bad], &track, HallucinationOptions::default());
        assert!(outcome.segments.is_empty());
        assert_eq!(outcome.report.no_speech, 1);
    }

    #[test]
    fn repetition_is_bounded_and_known_phrases_are_dropped() {
        let samples = tone(220.0, 6.0, 16_000);
        let track = EnergyTrack::analyze(&samples, 16_000);
        let segments: Vec<_> = (0..5)
            .map(|index| segment(index as f64, index as f64 + 0.5, "misma frase"))
            .collect();
        let outcome = filter_segments(&segments, &track, HallucinationOptions::default());
        assert_eq!(outcome.segments.len(), 2);
        assert_eq!(outcome.report.repeated, 3);

        let filler = segment(0.0, 0.5, "Thanks for watching!");
        let outcome = filter_segments(&[filler], &track, HallucinationOptions::default());
        assert!(outcome.segments.is_empty());
        assert_eq!(outcome.report.known_phrase, 1);
    }

    #[test]
    fn normalisation_ignores_case_and_punctuation() {
        assert_eq!(normalize_text("  ¡Hola,   MUNDO! "), "hola mundo");
        assert_eq!(normalize_text("don't"), "don't");
        assert!(is_known_hallucination(&normalize_text(
            "Gracias por ver el vídeo."
        )));
    }
}
