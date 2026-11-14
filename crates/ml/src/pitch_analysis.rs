use std::path::Path;

use charon_audio::AudioFile;
use pitch_core::{PitchTracker, PyinEstimator};
use serde::{Deserialize, Serialize};
use usdx_domain::{clamp_pitch, hz_to_midi, midi_to_hz};

use usdx_ports::SeparationError;

pub const TARGET_SAMPLE_RATE: u32 = 48_000;
pub const HOP_SECONDS: f64 = 0.02;
const PYIN_BATCH_SAMPLES: usize = 16_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchFrame {
    pub time_s: f64,
    pub hz: f32,
    pub confidence: f32,
    pub voiced: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchNote {
    pub start_s: f64,
    pub end_s: f64,
    pub pitch: i16,
    pub freq_hz: f32,
    pub confidence: f32,
}

impl PitchNote {
    pub fn duration_s(&self) -> f64 {
        (self.end_s - self.start_s).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchAnalysis {
    pub sample_rate: u32,
    pub frames: Vec<PitchFrame>,
    pub notes: Vec<PitchNote>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchOptions {
    pub min_confidence: f32,
    pub median_window: usize,
    pub min_note_s: f64,
    pub max_gap_s: f64,
    pub semitone_threshold: f32,
    pub change_frames: usize,
}

impl Default for PitchOptions {
    fn default() -> Self {
        Self {
            min_confidence: 0.25,
            median_window: 5,
            min_note_s: 0.08,
            max_gap_s: 0.05,
            semitone_threshold: 1.0,
            change_frames: 3,
        }
    }
}

pub fn analyze_file(path: impl AsRef<Path>) -> Result<PitchAnalysis, SeparationError> {
    analyze_file_with(path, PitchOptions::default())
}

pub fn analyze_file_with(
    path: impl AsRef<Path>,
    options: PitchOptions,
) -> Result<PitchAnalysis, SeparationError> {
    let audio = AudioFile::read(path.as_ref())
        .map_err(|error| SeparationError::Failed(error.to_string()))?;
    let mono = audio.to_mono();
    let mut samples: Vec<f32> = mono.row(0).to_vec();
    let sample_rate = audio.sample_rate;

    // pYIN processes 16 000-sample batches and keeps the remainder buffered.
    // Pad the tail so the final partial batch is not silently discarded.
    let remainder = samples.len() % PYIN_BATCH_SAMPLES;
    if remainder != 0 {
        samples.resize(samples.len() + (PYIN_BATCH_SAMPLES - remainder), 0.0);
    }

    let estimator =
        PyinEstimator::new().map_err(|error| SeparationError::Failed(error.to_string()))?;
    let mut tracker = PitchTracker::new(estimator, sample_rate, 4096)
        .map_err(|error| SeparationError::Failed(error.to_string()))?;
    let raw_frames = tracker
        .process(&samples)
        .map_err(|error| SeparationError::Failed(error.to_string()))?;

    let frames: Vec<PitchFrame> = raw_frames
        .into_iter()
        .filter(|frame| !frame.is_preliminary)
        .map(|frame| PitchFrame {
            time_s: frame.time_s as f64,
            hz: frame.pitch_hz,
            confidence: frame.confidence,
            voiced: frame.pitch_hz > 0.0 && frame.confidence >= options.min_confidence,
        })
        .collect();

    let notes = segment_notes(&frames, options);
    Ok(PitchAnalysis {
        sample_rate: TARGET_SAMPLE_RATE,
        frames,
        notes,
    })
}

/// Groups voiced frames into notes, splitting on sustained semitone changes.
pub fn segment_notes(frames: &[PitchFrame], options: PitchOptions) -> Vec<PitchNote> {
    let midi: Vec<Option<f32>> = frames
        .iter()
        .map(|frame| {
            if frame.voiced && frame.hz > 0.0 {
                Some(hz_to_midi(frame.hz as f64) as f32)
            } else {
                None
            }
        })
        .collect();
    let filtered = median_filter(&midi, options.median_window);

    let max_gap_frames = (options.max_gap_s / HOP_SECONDS).ceil() as usize;
    let min_note_frames = (options.min_note_s / HOP_SECONDS).ceil() as usize;
    let mut notes = Vec::new();
    let mut index = 0;

    while index < filtered.len() {
        if filtered[index].is_none() {
            index += 1;
            continue;
        }
        let run_start = index;
        let mut last_voiced = index;

        while index < filtered.len() {
            if filtered[index].is_some() {
                last_voiced = index;
            } else if index.saturating_sub(last_voiced) > max_gap_frames {
                break;
            }
            index += 1;
        }
        let run_end = (last_voiced + 1).min(filtered.len());

        let mut note_start = run_start;
        let mut change_start: Option<usize> = None;
        let mut cursor = run_start;
        while cursor < run_end {
            let Some(value) = filtered[cursor] else {
                cursor += 1;
                continue;
            };
            if cursor > note_start {
                let reference = median_of_first(&filtered, note_start, cursor, 10);
                let differs = reference.is_some_and(|reference| {
                    (value - reference).abs() >= options.semitone_threshold
                });
                if differs {
                    let start = *change_start.get_or_insert(cursor);
                    if cursor - start + 1 >= options.change_frames {
                        if start > note_start {
                            push_note(
                                &mut notes,
                                frames,
                                &filtered,
                                note_start,
                                start,
                                min_note_frames,
                            );
                        }
                        note_start = start;
                        change_start = None;
                    }
                } else {
                    change_start = None;
                }
            }
            cursor += 1;
        }
        push_note(
            &mut notes,
            frames,
            &filtered,
            note_start,
            run_end,
            min_note_frames,
        );
        index = run_end;
    }

    notes
}

fn push_note(
    notes: &mut Vec<PitchNote>,
    frames: &[PitchFrame],
    filtered: &[Option<f32>],
    start: usize,
    end: usize,
    min_frames: usize,
) {
    let values: Vec<f32> = filtered[start..end]
        .iter()
        .filter_map(|value| *value)
        .collect();
    if values.len() < min_frames || end <= start {
        return;
    }
    let center = median_sorted(&values);
    let pitch = clamp_pitch(center.round() as i16 - 60);
    let confidence = frames[start..end]
        .iter()
        .map(|frame| frame.confidence)
        .sum::<f32>()
        / (end - start) as f32;
    let start_s = frames[start].time_s;
    let end_s = frames[end - 1].time_s + HOP_SECONDS;
    notes.push(PitchNote {
        start_s,
        end_s,
        pitch,
        freq_hz: midi_to_hz(center as f64) as f32,
        confidence,
    });
}

fn median_filter(values: &[Option<f32>], window: usize) -> Vec<Option<f32>> {
    if window <= 1 {
        return values.to_vec();
    }
    let radius = window / 2;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (*value)?;
            let start = index.saturating_sub(radius);
            let end = (index + radius + 1).min(values.len());
            let mut neighbours: Vec<f32> = values[start..end]
                .iter()
                .filter_map(|value| *value)
                .collect();
            if neighbours.is_empty() {
                return None;
            }
            neighbours.sort_by(|left, right| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            });
            Some(neighbours[neighbours.len() / 2])
        })
        .collect()
}

/// Median of the first `max_frames` voiced frames of a note. This is a stable
/// reference so a sustained step is detected before the median drifts.
fn median_of_first(
    values: &[Option<f32>],
    start: usize,
    cursor: usize,
    max_frames: usize,
) -> Option<f32> {
    let end = (start + max_frames).min(cursor);
    let mut neighbours: Vec<f32> = values[start..end]
        .iter()
        .filter_map(|value| *value)
        .collect();
    if neighbours.is_empty() {
        return None;
    }
    neighbours.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Some(neighbours[neighbours.len() / 2])
}

fn median_sorted(values: &[f32]) -> f32 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    sorted[sorted.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use charon_audio::{AudioBuffer, AudioFile};
    use ndarray::Array2;

    fn write_tone(path: &Path, sample_rate: u32, seconds: f64, hz: f64) {
        let samples = (sample_rate as f64 * seconds) as usize;
        let data: Vec<f32> = (0..samples)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * hz as f32 * time).sin() * 0.3
            })
            .collect();
        let buffer = AudioBuffer::new(
            Array2::from_shape_vec((1, samples), data).unwrap(),
            sample_rate,
        );
        AudioFile::write_wav(path, &buffer).unwrap();
    }

    fn write_two_tones(path: &Path) {
        let sample_rate = 48_000;
        let tone = |hz: f64, seconds: f64| {
            let samples = (sample_rate as f64 * seconds) as usize;
            (0..samples).map(move |index| {
                let time = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * hz as f32 * time).sin() * 0.3
            })
        };
        let data: Vec<f32> = tone(220.0, 0.6).chain(tone(440.0, 0.6)).collect();
        let buffer = AudioBuffer::new(
            Array2::from_shape_vec((1, data.len()), data).unwrap(),
            sample_rate,
        );
        AudioFile::write_wav(path, &buffer).unwrap();
    }

    #[test]
    fn detects_concert_a_as_pitch_nine() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a440.wav");
        write_tone(&path, 48_000, 1.0, 440.0);
        let analysis = analyze_file(&path).unwrap();
        assert!(!analysis.notes.is_empty());
        let note = analysis
            .notes
            .iter()
            .max_by(|left, right| left.duration_s().total_cmp(&right.duration_s()))
            .unwrap();
        assert_eq!(note.pitch, 9);
        assert!((note.freq_hz - 440.0).abs() < 10.0);
    }

    #[test]
    fn splits_a_semitone_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("two-tones.wav");
        write_two_tones(&path);
        let analysis = analyze_file(&path).unwrap();
        let pitches: Vec<i16> = analysis.notes.iter().map(|note| note.pitch).collect();
        assert!(pitches.contains(&-3), "{pitches:?}");
        assert!(pitches.contains(&9), "{pitches:?}");
    }

    #[test]
    fn vibrato_stays_in_one_note() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vibrato.wav");
        let sample_rate = 48_000;
        let samples = sample_rate as usize;
        let data: Vec<f32> = (0..samples)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                let midi = 69.0 + 0.3 * (2.0 * std::f32::consts::PI * 5.0 * time).sin();
                let hz = 440.0 * 2.0_f32.powf((midi - 69.0) / 12.0);
                (2.0 * std::f32::consts::PI * hz * time).sin() * 0.3
            })
            .collect();
        let buffer = AudioBuffer::new(
            Array2::from_shape_vec((1, samples), data).unwrap(),
            sample_rate,
        );
        AudioFile::write_wav(&path, &buffer).unwrap();

        let analysis = analyze_file(&path).unwrap();
        let long_notes = analysis
            .notes
            .iter()
            .filter(|note| note.duration_s() > 0.2)
            .count();
        assert!(long_notes <= 1, "{:?}", analysis.notes);
    }

    #[test]
    fn silence_has_no_notes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("silence.wav");
        let buffer = AudioBuffer::new(Array2::zeros((1, 48_000)), 48_000);
        AudioFile::write_wav(&path, &buffer).unwrap();
        let analysis = analyze_file(&path).unwrap();
        assert!(analysis.notes.is_empty());
    }

    #[test]
    fn analysis_is_deterministic_json() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tone.wav");
        write_tone(&path, 48_000, 0.5, 440.0);
        let first = analyze_file(&path).unwrap();
        let second = analyze_file(&path).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn median_filter_ignores_unvoiced_neighbours() {
        let values = vec![Some(1.0), None, Some(3.0), Some(5.0), None];
        let filtered = median_filter(&values, 5);
        assert_eq!(filtered[0], Some(3.0));
        assert_eq!(filtered[1], None);
    }
}
