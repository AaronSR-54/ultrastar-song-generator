use std::f32::consts::PI;
use std::path::Path;

use charon_audio::AudioFile;
use realfft::RealFftPlanner;
use serde::{Deserialize, Serialize};
use usdx_ports::SeparationError;

const ANALYSIS_RATE: u32 = 22_050;
const FRAME_SIZE: usize = 1024;
const HOP_SIZE: usize = 512;
const MIN_BPM: f64 = 60.0;
const MAX_BPM: f64 = 180.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoCandidate {
    pub bpm_musical: f64,
    pub bpm_usdx: f64,
    pub gap_ms: i64,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoEstimate {
    pub best: TempoCandidate,
    pub candidates: Vec<TempoCandidate>,
    pub confidence: f32,
}

/// Estimates musical tempo and beat-0 offset from the onset envelope.
pub fn estimate_tempo(path: impl AsRef<Path>) -> Result<TempoEstimate, SeparationError> {
    let audio = AudioFile::read(path.as_ref())
        .map_err(|error| SeparationError::Failed(error.to_string()))?;
    let audio = audio
        .resample(ANALYSIS_RATE)
        .map_err(|error| SeparationError::Failed(error.to_string()))?;
    let mono = audio.to_mono();
    let mut samples: Vec<f32> = mono.row(0).to_vec();
    if samples.len() < FRAME_SIZE * 2 {
        return Err(SeparationError::Failed(
            "audio is too short for tempo estimation".into(),
        ));
    }

    let envelope = onset_envelope(&mut samples);
    let fps = ANALYSIS_RATE as f64 / HOP_SIZE as f64;
    let mut raw_candidates = autocorrelation_candidates(&envelope, fps);
    if raw_candidates.is_empty() {
        return Err(SeparationError::Failed(
            "could not find a tempo candidate".into(),
        ));
    }
    raw_candidates.sort_by(|left, right| right.1.total_cmp(&left.1));
    let max_score = raw_candidates[0].1.max(f32::EPSILON);

    let mut candidates: Vec<TempoCandidate> = Vec::new();
    for (bpm, score) in raw_candidates.into_iter().take(12) {
        for variant in [bpm, bpm * 2.0, bpm / 2.0] {
            if !(MIN_BPM..=MAX_BPM).contains(&variant) {
                continue;
            }
            if candidates
                .iter()
                .any(|candidate| (candidate.bpm_musical - variant).abs() < 1.0)
            {
                continue;
            }
            let (gap_ms, phase_score) = best_phase(&envelope, fps, variant);
            let prior = tempo_prior(variant);
            candidates.push(TempoCandidate {
                bpm_musical: variant,
                bpm_usdx: variant * 4.0,
                gap_ms,
                score: (score / max_score) * phase_score * prior,
            });
        }
    }

    candidates.sort_by(|left, right| right.score.total_cmp(&left.score));
    candidates.truncate(5);
    let best = candidates
        .first()
        .cloned()
        .ok_or_else(|| SeparationError::Failed("no usable tempo candidate".into()))?;
    let second = candidates
        .get(1)
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    let confidence = if best.score + second > 0.0 {
        (best.score / (best.score + second)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    Ok(TempoEstimate {
        best,
        candidates,
        confidence,
    })
}

fn onset_envelope(samples: &mut Vec<f32>) -> Vec<f32> {
    let window: Vec<f32> = (0..FRAME_SIZE)
        .map(|index| 0.5 - 0.5 * (2.0 * PI * index as f32 / (FRAME_SIZE - 1) as f32).cos())
        .collect();
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(FRAME_SIZE);
    let mut input = r2c.make_input_vec();
    let mut output = r2c.make_output_vec();
    let mut previous = vec![0.0_f32; output.len()];
    let mut envelope = Vec::new();

    let padded = samples.len().div_ceil(HOP_SIZE) * HOP_SIZE + FRAME_SIZE;
    samples.resize(padded, 0.0);

    for start in (0..samples.len().saturating_sub(FRAME_SIZE)).step_by(HOP_SIZE) {
        for (index, value) in input.iter_mut().enumerate() {
            *value = samples[start + index] * window[index];
        }
        if r2c.process(&mut input, &mut output).is_err() {
            continue;
        }
        let mut flux = 0.0_f32;
        for (index, bin) in output.iter().enumerate() {
            let magnitude = bin.norm();
            flux += (magnitude - previous[index]).max(0.0);
            previous[index] = magnitude;
        }
        envelope.push(flux);
    }

    let maximum = envelope.iter().copied().fold(0.0_f32, f32::max);
    if maximum > 0.0 {
        for value in &mut envelope {
            *value /= maximum;
        }
    }
    envelope
}

fn autocorrelation_candidates(envelope: &[f32], fps: f64) -> Vec<(f64, f32)> {
    let min_lag = ((fps * 60.0 / MAX_BPM).floor() as usize).max(1);
    let max_lag = ((fps * 60.0 / MIN_BPM).ceil() as usize).min(envelope.len() / 2);
    if max_lag <= min_lag {
        return Vec::new();
    }

    let mut scores = Vec::new();
    for lag in min_lag..=max_lag {
        let mut sum = 0.0_f64;
        for index in 0..envelope.len() - lag {
            sum += envelope[index] as f64 * envelope[index + lag] as f64;
        }
        let score = (sum / (envelope.len() - lag) as f64) as f32;
        scores.push((lag, score));
    }

    let mut candidates = Vec::new();
    for index in 1..scores.len().saturating_sub(1) {
        let (_, score) = scores[index];
        if score <= 0.0 {
            continue;
        }
        if score >= scores[index - 1].1 && score >= scores[index + 1].1 {
            let lag = scores[index].0 as f64;
            let bpm = 60.0 * fps / lag;
            if (MIN_BPM..=MAX_BPM).contains(&bpm) {
                candidates.push((bpm, score));
            }
        }
    }
    if candidates.is_empty() {
        if let Some((lag, score)) = scores
            .iter()
            .max_by(|left, right| left.1.total_cmp(&right.1))
        {
            candidates.push((60.0 * fps / *lag as f64, *score));
        }
    }
    candidates
}

fn best_phase(envelope: &[f32], fps: f64, bpm: f64) -> (i64, f32) {
    let beat_frames = fps * 60.0 / bpm;
    if beat_frames <= 0.0 {
        return (0, 0.0);
    }
    let search = beat_frames.ceil() as usize;
    let mut best_offset = 0_usize;
    let mut best_score = f32::NEG_INFINITY;

    for offset in 0..search {
        let mut position = offset as f64;
        let mut sum = 0.0_f32;
        let mut count = 0_usize;
        while position < envelope.len() as f64 {
            let index = (position.round() as usize).min(envelope.len() - 1);
            sum += envelope[index];
            count += 1;
            position += beat_frames;
        }
        let score = if count == 0 { 0.0 } else { sum / count as f32 };
        if score > best_score {
            best_score = score;
            best_offset = offset;
        }
    }

    let gap_ms = (best_offset as f64 / fps * 1000.0).round() as i64;
    (gap_ms, best_score.max(0.0))
}

/// Mild prior centred around 100 BPM to avoid choosing an octave by accident.
fn tempo_prior(bpm: f64) -> f32 {
    let octaves = (bpm / 100.0).ln() / std::f64::consts::LN_2;
    (-0.5 * (octaves / 0.6).powi(2)).exp() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use charon_audio::{AudioBuffer, AudioFile};
    use ndarray::Array2;

    fn write_click_track(path: &Path, bpm: f64) -> Result<(), SeparationError> {
        let sample_rate = ANALYSIS_RATE;
        let seconds = 8.0;
        let samples = (sample_rate as f64 * seconds) as usize;
        let mut data = vec![0.0_f32; samples];
        let beat_samples = (sample_rate as f64 * 60.0 / bpm).round() as usize;
        let click_samples = (sample_rate as f64 * 0.02) as usize;
        let mut beat = 0_usize;
        while beat < samples {
            for offset in 0..click_samples.min(samples - beat) {
                let envelope = 1.0 - offset as f32 / click_samples as f32;
                let time = offset as f32 / sample_rate as f32;
                data[beat + offset] += (2.0 * PI * 1_000.0 * time).sin() * envelope * 0.8;
            }
            beat += beat_samples;
        }
        let buffer = AudioBuffer::new(
            Array2::from_shape_vec((1, samples), data).unwrap(),
            sample_rate,
        );
        AudioFile::write_wav(path, &buffer)
            .map_err(|error| SeparationError::Failed(error.to_string()))
    }

    #[test]
    fn estimates_click_tempos() {
        let directory = tempfile::tempdir().unwrap();
        for expected in [80.0, 100.0, 120.0] {
            let path = directory.path().join(format!("click-{expected}.wav"));
            write_click_track(&path, expected).unwrap();
            let estimate = estimate_tempo(&path).unwrap();
            assert!(
                (estimate.best.bpm_musical - expected).abs() < 2.0,
                "expected {expected}, best {:?}, candidates {:?}",
                estimate.best,
                estimate.candidates
            );
            assert!((estimate.best.bpm_usdx - expected * 4.0).abs() < 8.0);
            assert!(estimate.confidence > 0.0);
        }
    }

    #[test]
    fn prior_penalises_octave_errors() {
        assert!(tempo_prior(100.0) > tempo_prior(50.0));
        assert!(tempo_prior(100.0) > tempo_prior(200.0));
    }
}
