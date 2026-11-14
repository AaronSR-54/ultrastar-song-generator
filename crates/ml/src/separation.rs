use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use charon_audio::{AudioBuffer, AudioFile, Control, Separator, SeparatorConfig};
use ndarray::{s, Array2, Axis};
use usdx_ports::{SeparationControl, SeparationError, StemPaths, StemSeparator};

/// Local HTDemucs implementation backed by `charon-audio` and ONNX Runtime.
pub struct CharonSeparator {
    separator: Mutex<Separator>,
    model_path: PathBuf,
}

impl CharonSeparator {
    pub fn new(model_path: impl Into<PathBuf>) -> Result<Self, SeparationError> {
        let model_path = model_path.into();
        if !model_path.exists() {
            return Err(SeparationError::Failed(format!(
                "model not found: {}",
                model_path.display()
            )));
        }
        let config = SeparatorConfig::htdemucs(&model_path).with_progress(false);
        let separator =
            Separator::new(config).map_err(|error| SeparationError::Failed(error.to_string()))?;
        Ok(Self {
            separator: Mutex::new(separator),
            model_path,
        })
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

impl StemSeparator for CharonSeparator {
    fn separate(
        &self,
        input: &Path,
        output_dir: &Path,
        control: SeparationControl,
    ) -> Result<StemPaths, SeparationError> {
        let audio =
            AudioFile::read(input).map_err(|error| SeparationError::Failed(error.to_string()))?;
        std::fs::create_dir_all(output_dir)?;

        let progress = control.progress.clone();
        let charon_control = Control::new().with_progress(move |value| {
            progress(value.fraction() as f32);
        });

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let our_cancel = control.cancel.clone();
        let charon_cancel = charon_control.cancel_token().clone();
        let bridge = std::thread::spawn(move || {
            while !stop_thread.load(Ordering::Acquire) {
                if our_cancel.is_cancelled() {
                    charon_cancel.cancel();
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        let result = {
            let separator = self
                .separator
                .lock()
                .map_err(|_| SeparationError::Failed("separation mutex poisoned".into()))?;
            separator.separate_with(&audio, &charon_control)
        };

        stop.store(true, Ordering::Release);
        let _ = bridge.join();

        if control.cancel.is_cancelled() {
            return Err(SeparationError::Cancelled);
        }

        let stems = result.map_err(|error| SeparationError::Failed(error.to_string()))?;
        let vocals = stems
            .get("vocals")
            .ok_or_else(|| SeparationError::MissingStem("vocals".into()))?
            .clone();

        let channels = vocals.channels();
        let samples = audio.samples();
        let mut instrumental_data = Array2::<f32>::zeros((channels, vocals.samples()));
        let mut found_any = false;
        for name in ["drums", "bass", "other"] {
            if let Some(stem) = stems.get(name) {
                if stem.data.dim() != instrumental_data.dim() {
                    return Err(SeparationError::ShapeMismatch(format!(
                        "{name}: {:?} != {:?}",
                        stem.data.dim(),
                        instrumental_data.dim()
                    )));
                }
                instrumental_data += &stem.data;
                found_any = true;
            }
        }
        if !found_any {
            if audio.data.dim() != vocals.data.dim() {
                return Err(SeparationError::ShapeMismatch(
                    "cannot subtract vocals from the mix".into(),
                ));
            }
            instrumental_data = &audio.data - &vocals.data;
        }

        let instrumental = AudioBuffer::new(instrumental_data, vocals.sample_rate);
        let vocals = fit_length(vocals, samples, audio.sample_rate)?;
        let instrumental = fit_length(instrumental, samples, audio.sample_rate)?;
        let vocals = match_channel_count(vocals, audio.channels())?;
        let instrumental = match_channel_count(instrumental, audio.channels())?;

        if vocals.channels() != audio.channels() {
            return Err(SeparationError::ShapeMismatch(
                "vocals channel count differs from the mix".into(),
            ));
        }
        if instrumental.channels() != audio.channels() {
            return Err(SeparationError::ShapeMismatch(
                "instrumental channel count differs from the mix".into(),
            ));
        }

        let vocals_path = output_dir.join("vocals.wav");
        let instrumental_path = output_dir.join("instrumental.wav");
        AudioFile::write_wav(&vocals_path, &vocals)
            .map_err(|error| SeparationError::Failed(error.to_string()))?;
        AudioFile::write_wav(&instrumental_path, &instrumental)
            .map_err(|error| SeparationError::Failed(error.to_string()))?;

        Ok(StemPaths {
            vocals: vocals_path,
            instrumental: instrumental_path,
        })
    }
}

/// HTDemucs emits stereo stems even for a mono mix (the analysis WAV is mono),
/// and the exported stems must keep the channel count of the project audio.
fn match_channel_count(
    buffer: AudioBuffer,
    target_channels: usize,
) -> Result<AudioBuffer, SeparationError> {
    if buffer.channels() == target_channels || target_channels == 0 {
        return Ok(buffer);
    }
    let sample_rate = buffer.sample_rate;
    match (buffer.channels(), target_channels) {
        (2, 1) => {
            let mut mixed = buffer.data.row(0).to_owned();
            mixed += &buffer.data.row(1);
            mixed /= 2.0;
            Ok(AudioBuffer::new(mixed.insert_axis(Axis(0)), sample_rate))
        }
        (1, 2) => {
            let duplicated =
                ndarray::concatenate(Axis(0), &[buffer.data.view(), buffer.data.view()])
                    .map_err(|error| SeparationError::Failed(error.to_string()))?;
            Ok(AudioBuffer::new(duplicated, sample_rate))
        }
        (channels, target) => Err(SeparationError::ShapeMismatch(format!(
            "cannot convert {channels} channels to {target}"
        ))),
    }
}

fn fit_length(
    mut buffer: AudioBuffer,
    samples: usize,
    sample_rate: u32,
) -> Result<AudioBuffer, SeparationError> {
    if buffer.sample_rate != sample_rate {
        buffer = buffer
            .resample(sample_rate)
            .map_err(|error| SeparationError::Failed(error.to_string()))?;
    }
    if buffer.samples() > samples {
        buffer.data = buffer.data.slice(s![.., ..samples]).to_owned();
    } else if buffer.samples() < samples {
        let padding = Array2::<f32>::zeros((buffer.channels(), samples - buffer.samples()));
        buffer.data = ndarray::concatenate(Axis(1), &[buffer.data.view(), padding.view()])
            .map_err(|error| SeparationError::Failed(error.to_string()))?;
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_length_truncates_and_pads() {
        let long = AudioBuffer::new(Array2::ones((1, 10)), 48_000);
        let fitted = fit_length(long, 6, 48_000).unwrap();
        assert_eq!(fitted.samples(), 6);
        assert_eq!(fitted.data[[0, 0]], 1.0);

        let short = AudioBuffer::new(Array2::ones((1, 3)), 48_000);
        let fitted = fit_length(short, 6, 48_000).unwrap();
        assert_eq!(fitted.samples(), 6);
        assert_eq!(fitted.data[[0, 5]], 0.0);
    }

    #[test]
    fn missing_model_is_reported() {
        let error = CharonSeparator::new("definitely-not-a-model.onnx").err();
        assert!(matches!(error, Some(SeparationError::Failed(_))));
    }

    #[test]
    fn stereo_stems_are_downmixed_for_a_mono_mix() {
        let stereo = AudioBuffer::new(
            Array2::from_shape_vec((2, 3), vec![1.0, 0.5, -1.0, 0.0, 0.5, 1.0]).unwrap(),
            48_000,
        );
        let mono = match_channel_count(stereo, 1).unwrap();
        assert_eq!(mono.channels(), 1);
        assert_eq!(mono.samples(), 3);
        assert!((mono.data[[0, 0]] - 0.5).abs() < f32::EPSILON);
        assert!((mono.data[[0, 1]] - 0.5).abs() < f32::EPSILON);
        assert!((mono.data[[0, 2]] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mono_stems_are_duplicated_for_a_stereo_mix() {
        let mono = AudioBuffer::new(
            Array2::from_shape_vec((1, 2), vec![0.25, -0.25]).unwrap(),
            48_000,
        );
        let stereo = match_channel_count(mono, 2).unwrap();
        assert_eq!(stereo.channels(), 2);
        assert_eq!(stereo.data[[0, 0]], 0.25);
        assert_eq!(stereo.data[[1, 0]], 0.25);
        assert_eq!(stereo.data[[1, 1]], -0.25);
    }
}
