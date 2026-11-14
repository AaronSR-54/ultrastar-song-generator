//! Multi-level peak pyramid for the waveform view.
//!
//! Level 0 stores one min/max pair every 256 samples; each following level
//! halves the resolution. Drawing a 6-minute song therefore reads at most one
//! peak per screen column instead of decoding or scanning 17 million samples.

use std::time::Instant;

/// Samples per peak at level 0.
pub const BASE_BUCKET: usize = 256;
/// Resolution factor between levels.
pub const LEVEL_FACTOR: usize = 4;
/// Stop adding levels below this many peaks; enough for a 4K screen.
pub const MIN_LEVEL_PEAKS: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Peak {
    pub min: f32,
    pub max: f32,
}

impl Peak {
    pub fn amplitude(&self) -> f32 {
        self.max.max(-self.min)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeakPyramid {
    levels: Vec<Vec<Peak>>,
    sample_rate: u32,
    samples: usize,
    build_ms: f64,
}

impl PeakPyramid {
    /// Builds the pyramid from mono samples. Complexity is O(n) overall because
    /// each level halves the data.
    pub fn from_mono(samples: &[f32], sample_rate: u32) -> Self {
        let started = Instant::now();
        let mut base = Vec::with_capacity(samples.len() / BASE_BUCKET + 1);
        for chunk in samples.chunks(BASE_BUCKET) {
            let mut peak = Peak { min: 0.0, max: 0.0 };
            for sample in chunk {
                let value = if sample.is_finite() { *sample } else { 0.0 };
                peak.min = peak.min.min(value);
                peak.max = peak.max.max(value);
            }
            base.push(peak);
        }

        let mut levels = vec![base];
        while levels.last().map(|level| level.len()).unwrap_or(0) > MIN_LEVEL_PEAKS {
            let previous = levels.last().expect("levels is not empty");
            let mut next = Vec::with_capacity(previous.len() / LEVEL_FACTOR + 1);
            for chunk in previous.chunks(LEVEL_FACTOR) {
                let mut peak = Peak { min: 0.0, max: 0.0 };
                for value in chunk {
                    peak.min = peak.min.min(value.min);
                    peak.max = peak.max.max(value.max);
                }
                next.push(peak);
            }
            levels.push(next);
        }

        Self {
            levels,
            sample_rate,
            samples: samples.len(),
            build_ms: started.elapsed().as_secs_f64() * 1000.0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn duration_s(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.samples as f64 / self.sample_rate as f64
        }
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    pub fn build_ms(&self) -> f64 {
        self.build_ms
    }

    pub fn peaks(&self, level: usize) -> &[Peak] {
        self.levels
            .get(level)
            .or_else(|| self.levels.last())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Chooses the coarsest level that still has at least one peak per column.
    pub fn level_for_width(&self, width: usize) -> usize {
        let width = width.max(1);
        self.levels
            .iter()
            .position(|level| level.len() <= width.saturating_mul(2))
            .unwrap_or(self.levels.len().saturating_sub(1))
    }

    /// Picks the peaks that cover `[start_s, end_s)` for a viewport of `width`
    /// columns. Cost is O(width), independent of the song length.
    pub fn columns(&self, start_s: f64, end_s: f64, width: usize) -> Vec<Peak> {
        let width = width.max(1);
        let level = self.level_for_width(width);
        let peaks = self.peaks(level);
        if peaks.is_empty() || end_s <= start_s {
            return vec![Peak::default(); width];
        }
        let samples_per_peak = BASE_BUCKET * LEVEL_FACTOR.pow(level as u32);
        let start_index = (start_s * self.sample_rate as f64 / samples_per_peak as f64).floor();
        let end_index = (end_s * self.sample_rate as f64 / samples_per_peak as f64).ceil();
        let start_index = start_index.max(0.0) as usize;
        let end_index = (end_index as usize).min(peaks.len());

        let span = (end_index.saturating_sub(start_index)).max(1);
        (0..width)
            .map(|column| {
                let from = start_index + column * span / width;
                let to = start_index + (column + 1) * span / width;
                let to = to.max(from + 1).min(peaks.len());
                if from >= peaks.len() {
                    return Peak::default();
                }
                let slice = &peaks[from..to];
                let mut peak = Peak { min: 0.0, max: 0.0 };
                for value in slice {
                    peak.min = peak.min.min(value.min);
                    peak.max = peak.max.max(value.max);
                }
                peak
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(seconds: f64, sample_rate: u32) -> Vec<f32> {
        let samples = (seconds * sample_rate as f64) as usize;
        (0..samples)
            .map(|index| (index as f32 * 0.01).sin() * 0.5)
            .collect()
    }

    #[test]
    fn pyramid_has_several_levels_and_keeps_extremes() {
        let samples = song(60.0, 48_000);
        let pyramid = PeakPyramid::from_mono(&samples, 48_000);
        assert!(pyramid.level_count() > 2);
        let base = pyramid.peaks(0);
        let minimum = base.iter().fold(0.0_f32, |value, peak| value.min(peak.min));
        let maximum = base.iter().fold(0.0_f32, |value, peak| value.max(peak.max));
        assert!(minimum <= -0.45, "{minimum}");
        assert!(maximum >= 0.45, "{maximum}");
        assert!((pyramid.duration_s() - 60.0).abs() < 0.01);
    }

    #[test]
    fn six_minute_song_builds_fast_and_draws_in_linear_time() {
        let samples = song(360.0, 48_000);
        let started = Instant::now();
        let pyramid = PeakPyramid::from_mono(&samples, 48_000);
        let build = started.elapsed();
        assert!(
            build < std::time::Duration::from_secs(2),
            "peak build took {build:?}"
        );
        assert!(pyramid.build_ms() < 2000.0);

        let started = Instant::now();
        let columns = pyramid.columns(30.0, 90.0, 1200);
        let draw = started.elapsed();
        assert_eq!(columns.len(), 1200);
        assert!(
            draw < std::time::Duration::from_millis(200),
            "waveform draw took {draw:?}"
        );
    }

    #[test]
    fn level_choice_keeps_at_most_two_peaks_per_column() {
        let samples = song(120.0, 48_000);
        let pyramid = PeakPyramid::from_mono(&samples, 48_000);
        for width in [100, 800, 1920] {
            let level = pyramid.level_for_width(width);
            assert!(pyramid.peaks(level).len() <= width * 2);
        }
    }

    #[test]
    fn columns_cover_the_requested_range() {
        let mut samples = vec![0.0_f32; 48_000 * 10];
        // Loud blip at 4 s.
        for sample in samples.iter_mut().skip(48_000 * 4).take(480) {
            *sample = 1.0;
        }
        let pyramid = PeakPyramid::from_mono(&samples, 48_000);
        let columns = pyramid.columns(0.0, 10.0, 100);
        let loudest = columns
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.amplitude().total_cmp(&b.1.amplitude()))
            .unwrap();
        assert!((loudest.0 as i32 - 40).abs() <= 2, "column {}", loudest.0);
        assert!(loudest.1.amplitude() > 0.9);
    }

    #[test]
    fn empty_audio_is_safe() {
        let pyramid = PeakPyramid::from_mono(&[], 48_000);
        assert_eq!(pyramid.duration_s(), 0.0);
        let columns = pyramid.columns(0.0, 1.0, 10);
        assert_eq!(columns.len(), 10);
        assert!(columns.iter().all(|peak| peak.amplitude() == 0.0));
    }
}
