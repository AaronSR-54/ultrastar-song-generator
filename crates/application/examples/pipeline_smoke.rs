//! Headless end-to-end run used for the acceptance and performance checks.
//!
//! Environment variables:
//! - `USDX_PROJECT_DIR` (required): folder of the project to create/reuse.
//! - `USDX_SOURCE_URL` or `USDX_SOURCE_AUDIO` (one required): YouTube URL or a
//!   local audio file. With a local file the ingest stage is skipped and the
//!   file is encoded to `source/audio-source.mp3` with ffmpeg.
//! - `USDX_HTDEMUCS_MODEL`, `USDX_WHISPER_MODEL` (required): local model paths.
//! - `USDX_LANGUAGE` (optional): ISO code forced for transcription.
//! - `USDX_COVER` (optional): image copied to `source/cover.jpg` in local mode.
//!
//! Run:
//! ```bash
//! cargo run -p usdx-application --example pipeline_smoke
//! ```

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use usdx_application::{
    Pipeline, PipelineConfig, PitchArtifacts, ProjectSettings, ProjectStore, SourceInfo, StageId,
    StageProgress, StageStatus,
};
use usdx_domain::SongDoc;
use usdx_media::Toolchain;

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env(name).map(PathBuf::from)
}

/// Progress callback expected by [`StageProgress::new`].
type ProgressFn = Arc<dyn Fn(f32, &str) + Send + Sync>;

fn progress_printer() -> ProgressFn {
    let last = AtomicU8::new(255);
    Arc::new(move |fraction, message| {
        let bucket = (fraction * 20.0).round() as u8;
        if bucket != last.swap(bucket, Ordering::Relaxed) {
            println!("    {:>3.0}% {}", fraction * 100.0, message);
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = env_path("USDX_PROJECT_DIR").ok_or("USDX_PROJECT_DIR is required")?;
    let separation_model =
        env_path("USDX_HTDEMUCS_MODEL").ok_or("USDX_HTDEMUCS_MODEL is required")?;
    let whisper_model = env_path("USDX_WHISPER_MODEL").ok_or("USDX_WHISPER_MODEL is required")?;
    let source_url = env("USDX_SOURCE_URL");
    let source_audio = env_path("USDX_SOURCE_AUDIO");
    if source_url.is_none() && source_audio.is_none() {
        return Err("USDX_SOURCE_URL or USDX_SOURCE_AUDIO is required".into());
    }

    let settings = ProjectSettings {
        language: env("USDX_LANGUAGE"),
        force_language: env("USDX_LANGUAGE").is_some(),
        download_video: false,
        audio_quality: "best".into(),
        separation_model: "htdemucs".into(),
        whisper_model: "whisper-large-v3-turbo-q5".into(),
    };

    let (store, mut project) = if project_dir.join("project.json").exists() {
        println!("reusing project {}", project_dir.display());
        ProjectStore::open(project_dir.clone())?
    } else {
        println!("creating project {}", project_dir.display());
        ProjectStore::create(
            project_dir.clone(),
            SourceInfo {
                url: source_url.clone().unwrap_or_default(),
                title: env("USDX_TITLE").unwrap_or_else(|| "Smoke test".into()),
                artist: env("USDX_ARTIST").or_else(|| Some("UltraStar".into())),
                ..SourceInfo::default()
            },
            settings.clone(),
        )?
    };
    project.settings = settings;

    let toolchain = Toolchain::detect();
    println!(
        "tools: yt-dlp={:?} ffmpeg={:?} ffprobe={:?} node={:?}",
        toolchain.yt_dlp.as_ref().map(|tool| &tool.version),
        toolchain.ffmpeg.as_ref().map(|tool| &tool.version),
        toolchain.ffprobe.as_ref().map(|tool| &tool.version),
        toolchain.node.as_ref().map(|tool| &tool.version),
    );

    let paths = store.paths();
    if let Some(audio) = &source_audio {
        paths.ensure_dirs()?;
        let ffmpeg = toolchain
            .ffmpeg
            .as_ref()
            .ok_or("ffmpeg is required for local audio")?
            .path
            .clone();
        let output = paths.source_dir().join("audio-source.mp3");
        println!("encoding {} -> {}", audio.display(), output.display());
        let status = Command::new(&ffmpeg)
            .args(["-y", "-i"])
            .arg(audio)
            .args(["-vn", "-ac", "2", "-c:a", "libmp3lame", "-q:a", "2"])
            .arg(&output)
            .status()?;
        if !status.success() {
            return Err(format!("ffmpeg failed with {status}").into());
        }
        usdx_media::convert_to_analysis_wav(&ffmpeg, &output, &paths.analysis_wav(), None).await?;
        if let Some(cover) = env_path("USDX_COVER") {
            std::fs::copy(cover, paths.cover())?;
        }
        let ingest = project.stage_mut(StageId::Ingest);
        if ingest.status == StageStatus::Pending {
            ingest.skip()?;
        }
        store.save(&project)?;
    }

    let pipeline = Pipeline::new(
        store.clone(),
        toolchain,
        PipelineConfig::new(separation_model, whisper_model),
    );

    let mut timings: Vec<(StageId, f64)> = Vec::new();
    for stage in StageId::ALL {
        if project.stage(stage).status != StageStatus::Pending
            && project.stage(stage).status != StageStatus::Failed
        {
            println!(
                "{}: {:?}, skipping",
                stage.label(),
                project.stage(stage).status
            );
            continue;
        }
        println!("== {} ==", stage.label());
        let started = Instant::now();
        let progress =
            StageProgress::new(usdx_application::CancelFlag::default(), progress_printer());
        match pipeline.run_stage(stage, &mut project, &progress).await {
            Ok(()) => {
                let elapsed = started.elapsed().as_secs_f64();
                timings.push((stage, elapsed));
                println!("   ok in {elapsed:.1}s");
            }
            Err(error) => {
                eprintln!(
                    "   FAILED after {:.1}s: {error}",
                    started.elapsed().as_secs_f64()
                );
                return Err(error.into());
            }
        }
    }

    println!("\nTimings:");
    for (stage, seconds) in &timings {
        println!("  {:<11} {:>8.1}s", stage.label(), seconds);
    }
    let total: f64 = timings.iter().map(|(_, seconds)| seconds).sum();
    println!("  {:<11} {:>8.1}s", "total", total);

    if paths.song().exists() {
        let document: SongDoc = serde_json::from_slice(&std::fs::read(paths.song())?)?;
        let artifacts: PitchArtifacts = serde_json::from_slice(&std::fs::read(paths.notes())?)?;
        println!(
            "\nsong: {} notes in {} phrases, language {:?}, bpm {:.2}, gap {} ms, {} tempo candidates",
            document.note_count(),
            document.phrases.len(),
            document.header.language,
            document.header.bpm,
            document.header.gap_ms,
            artifacts.candidates.len()
        );
        let output = paths.output_dir();
        if output.exists() {
            for entry in std::fs::read_dir(output)? {
                let entry = entry?;
                println!("exported: {}", entry.path().display());
            }
        }
    }

    Ok(())
}
