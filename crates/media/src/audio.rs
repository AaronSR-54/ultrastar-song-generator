use std::path::Path;
use std::time::Duration;

use usdx_ports::CancelFlag;

use crate::process::{run_checked, tokio_command, ProcessError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioProbe {
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("process error: {0}")]
    Process(#[from] ProcessError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid ffprobe JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("audio stream not found in {0}")]
    MissingStream(String),
    #[error("unexpected audio format: {0}")]
    UnexpectedFormat(String),
}

pub async fn probe(
    ffprobe: &Path,
    media: &Path,
    cancel: Option<CancelFlag>,
) -> Result<AudioProbe, AudioError> {
    let mut command = tokio_command(ffprobe);
    command.args([
        "-v",
        "error",
        "-select_streams",
        "a:0",
        "-show_entries",
        "stream=sample_rate,channels,duration:format=duration",
        "-of",
        "json",
    ]);
    command.arg(media);

    let output = run_checked(command, Duration::from_secs(60), cancel).await?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let stream = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .and_then(|streams| streams.first())
        .ok_or_else(|| AudioError::MissingStream(media.display().to_string()))?;

    let sample_rate = stream
        .get("sample_rate")
        .and_then(|value| {
            value
                .as_str()
                .and_then(|s| s.parse::<u32>().ok())
                .or_else(|| value.as_u64().map(|v| v as u32))
        })
        .ok_or_else(|| AudioError::MissingStream(media.display().to_string()))?;

    let channels = stream
        .get("channels")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as u16)
        .ok_or_else(|| AudioError::MissingStream(media.display().to_string()))?;

    let duration_seconds = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| {
            stream
                .get("duration")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| value.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    Ok(AudioProbe {
        duration_ms: (duration_seconds * 1000.0).round() as u64,
        sample_rate,
        channels,
    })
}

/// Converts any source audio into the mono 48 kHz PCM WAV used by analysis.
pub async fn convert_to_analysis_wav(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    cancel: Option<CancelFlag>,
) -> Result<(), AudioError> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut command = tokio_command(ffmpeg);
    command.args(["-y", "-i"]);
    command.arg(input);
    command.args(["-vn", "-ac", "1", "-ar", "48000", "-c:a", "pcm_s16le"]);
    command.arg(output);
    run_checked(command, Duration::from_secs(10 * 60), cancel).await?;
    Ok(())
}

pub fn verify_analysis_wav(source: AudioProbe, analysis: AudioProbe) -> Result<(), AudioError> {
    if analysis.channels != 1 {
        return Err(AudioError::UnexpectedFormat(format!(
            "analysis WAV must be mono, got {} channels",
            analysis.channels
        )));
    }
    if analysis.sample_rate != 48_000 {
        return Err(AudioError::UnexpectedFormat(format!(
            "analysis WAV must be 48 kHz, got {} Hz",
            analysis.sample_rate
        )));
    }
    let difference = source.duration_ms.abs_diff(analysis.duration_ms);
    if difference > 250 {
        return Err(AudioError::UnexpectedFormat(format!(
            "analysis WAV duration differs by {difference} ms"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::run_checked;
    use crate::tools::Toolchain;

    #[tokio::test]
    async fn converts_and_verifies_a_generated_wav() {
        let toolchain = Toolchain::detect();
        let (Some(ffmpeg), Some(ffprobe)) = (toolchain.ffmpeg.as_ref(), toolchain.ffprobe.as_ref())
        else {
            eprintln!("ffmpeg/ffprobe not available; skipping audio conversion test");
            return;
        };

        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.wav");
        let analysis = directory.path().join("analysis.wav");

        let mut generate = tokio_command(&ffmpeg.path);
        generate.args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-ac",
            "2",
            "-ar",
            "44100",
        ]);
        generate.arg(&input);
        run_checked(generate, Duration::from_secs(60), None)
            .await
            .unwrap();

        let source_probe = probe(&ffprobe.path, &input, None).await.unwrap();
        assert_eq!(source_probe.sample_rate, 44_100);
        assert_eq!(source_probe.channels, 2);

        convert_to_analysis_wav(&ffmpeg.path, &input, &analysis, None)
            .await
            .unwrap();
        let analysis_probe = probe(&ffprobe.path, &analysis, None).await.unwrap();
        assert_eq!(analysis_probe.sample_rate, 48_000);
        assert_eq!(analysis_probe.channels, 1);
        verify_analysis_wav(source_probe, analysis_probe).unwrap();
    }

    #[test]
    fn verification_rejects_wrong_format_and_duration() {
        let source = AudioProbe {
            duration_ms: 10_000,
            sample_rate: 44_100,
            channels: 2,
        };
        assert!(verify_analysis_wav(
            source,
            AudioProbe {
                duration_ms: 10_000,
                sample_rate: 48_000,
                channels: 2,
            }
        )
        .is_err());
        assert!(verify_analysis_wav(
            source,
            AudioProbe {
                duration_ms: 10_000,
                sample_rate: 44_100,
                channels: 1,
            }
        )
        .is_err());
        assert!(verify_analysis_wav(
            source,
            AudioProbe {
                duration_ms: 10_500,
                sample_rate: 48_000,
                channels: 1,
            }
        )
        .is_err());
    }
}
