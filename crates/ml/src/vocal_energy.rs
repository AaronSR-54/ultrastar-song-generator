use std::path::Path;

use charon_audio::AudioFile;
use usdx_ports::SeparationError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VocalEnergy {
    pub rms: f64,
    pub peak: f32,
    pub voiced_fraction: f64,
}

impl VocalEnergy {
    pub fn is_voice_missing(&self) -> bool {
        self.rms < 0.003 || self.voiced_fraction < 0.02
    }
}

/// Measures RMS, peak and the fraction of 50 ms frames above the voice floor.
pub fn analyze_vocal_energy(path: impl AsRef<Path>) -> Result<VocalEnergy, SeparationError> {
    let audio = AudioFile::read(path.as_ref())
        .map_err(|error| SeparationError::Failed(error.to_string()))?;
    let mono = audio.to_mono();
    let samples = mono.row(0);
    if samples.is_empty() {
        return Ok(VocalEnergy {
            rms: 0.0,
            peak: 0.0,
            voiced_fraction: 0.0,
        });
    }

    let mut sum_squares = 0.0_f64;
    let mut peak = 0.0_f32;
    for value in samples.iter() {
        sum_squares += (*value as f64) * (*value as f64);
        peak = peak.max(value.abs());
    }
    let rms = (sum_squares / samples.len() as f64).sqrt();

    let frame_size = (audio.sample_rate as usize / 20).max(1);
    let threshold = 0.005_f64;
    let mut frames = 0_usize;
    let mut active = 0_usize;
    for start in (0..samples.len()).step_by(frame_size) {
        let end = (start + frame_size).min(samples.len());
        let frame = samples.slice(ndarray::s![start..end]);
        let frame_rms = (frame
            .iter()
            .map(|value| (*value as f64).powi(2))
            .sum::<f64>()
            / frame.len() as f64)
            .sqrt();
        frames += 1;
        if frame_rms > threshold {
            active += 1;
        }
    }

    Ok(VocalEnergy {
        rms,
        peak,
        voiced_fraction: if frames == 0 {
            0.0
        } else {
            active as f64 / frames as f64
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use charon_audio::AudioBuffer;
    use ndarray::Array2;

    fn write_audio(path: &Path, samples: Vec<f32>, sample_rate: u32) {
        let buffer = AudioBuffer::new(
            Array2::from_shape_vec((1, samples.len()), samples).unwrap(),
            sample_rate,
        );
        AudioFile::write_wav(path, &buffer).unwrap();
    }

    #[test]
    fn detects_silence_as_missing_voice() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("silence.wav");
        write_audio(&path, vec![0.0; 24_000], 48_000);
        let energy = analyze_vocal_energy(&path).unwrap();
        assert_eq!(energy.rms, 0.0);
        assert_eq!(energy.peak, 0.0);
        assert!(energy.is_voice_missing());
    }

    #[test]
    fn detects_a_tone_as_voice() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tone.wav");
        let sample_rate = 48_000;
        let samples: Vec<f32> = (0..sample_rate)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * time).sin() * 0.3
            })
            .collect();
        write_audio(&path, samples, sample_rate);
        let energy = analyze_vocal_energy(&path).unwrap();
        assert!(energy.rms > 0.1);
        assert!(energy.peak > 0.25);
        assert!(energy.voiced_fraction > 0.9);
        assert!(!energy.is_voice_missing());
    }
}
