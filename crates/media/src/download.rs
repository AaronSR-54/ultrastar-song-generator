use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::process::Command;
use url::Url;
use usdx_ports::CancelFlag;

use crate::metadata::{parse_yt_dlp_json, MetadataError, VideoMetadata};
use crate::process::{cleanup_partials, run_checked, ProcessError};
use crate::tools::{Toolchain, ToolchainError};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("missing tool: {0}")]
    MissingTool(&'static str),
    #[error("toolchain is not ready: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("process error: {0}")]
    Process(#[from] ProcessError),
    #[error("metadata error: {0}")]
    Metadata(#[from] MetadataError),
    #[error("yt-dlp did not produce {0}")]
    MissingOutput(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// High-level media downloader backed by the `yt-dlp` and `ffmpeg` binaries.
pub struct MediaDownloader {
    pub toolchain: Toolchain,
    pub metadata_timeout: Duration,
    pub download_timeout: Duration,
    js_runtimes_flag: bool,
}

impl MediaDownloader {
    pub fn new(toolchain: Toolchain) -> Self {
        let js_runtimes_flag = detect_js_runtimes_flag(&toolchain);
        Self {
            toolchain,
            metadata_timeout: Duration::from_secs(120),
            download_timeout: Duration::from_secs(30 * 60),
            js_runtimes_flag,
        }
    }

    pub fn ensure_ready(&self) -> Result<(), ToolchainError> {
        self.toolchain.ensure_ready()
    }

    pub async fn fetch_metadata(
        &self,
        url: &str,
        cancel: Option<CancelFlag>,
    ) -> Result<VideoMetadata, DownloadError> {
        let url = validate_url(url)?;
        let yt_dlp = self
            .toolchain
            .yt_dlp
            .as_ref()
            .ok_or(DownloadError::MissingTool("yt-dlp"))?;
        self.require_js_runtime()?;

        let mut command = Command::new(&yt_dlp.path);
        command.args([
            "--dump-single-json",
            "--no-playlist",
            "--no-warnings",
            "--skip-download",
        ]);
        if self.js_runtimes_flag {
            add_js_runtime(&mut command, &self.toolchain);
        }
        command.arg(url.as_str());

        let output = run_checked(command, self.metadata_timeout, cancel).await?;
        Ok(parse_yt_dlp_json(&output.stdout_lossy())?)
    }

    pub async fn download_audio(
        &self,
        url: &str,
        source_dir: impl AsRef<Path>,
        cancel: Option<CancelFlag>,
    ) -> Result<PathBuf, DownloadError> {
        let url = validate_url(url)?;
        let source_dir = source_dir.as_ref();
        std::fs::create_dir_all(source_dir)?;
        let yt_dlp = self
            .toolchain
            .yt_dlp
            .as_ref()
            .ok_or(DownloadError::MissingTool("yt-dlp"))?;
        self.require_js_runtime()?;

        let output_template = source_dir.join("audio-source.%(ext)s");
        let mut command = Command::new(&yt_dlp.path);
        command.args([
            "-f",
            "bestaudio/best",
            "-x",
            "--audio-format",
            "mp3",
            "--audio-quality",
            "0",
            "--no-playlist",
            "--no-warnings",
            "--newline",
            "-o",
        ]);
        command.arg(&output_template);
        if self.js_runtimes_flag {
            add_js_runtime(&mut command, &self.toolchain);
        }
        command.arg(url.as_str());

        let result = run_checked(command, self.download_timeout, cancel).await;
        if result.is_err() {
            let _ = cleanup_partials(source_dir, "audio-source");
        }
        result?;

        let expected = source_dir.join("audio-source.mp3");
        if expected.exists() {
            return Ok(expected);
        }
        find_output(source_dir, "audio-source")
            .ok_or_else(|| DownloadError::MissingOutput("audio-source.mp3".into()))
    }

    /// Downloads the source video when enabled. A missing or failed video is
    /// reported as `Ok(None)` so the rest of the pipeline can continue.
    pub async fn download_video(
        &self,
        url: &str,
        source_dir: impl AsRef<Path>,
        enabled: bool,
        cancel: Option<CancelFlag>,
    ) -> Result<Option<PathBuf>, DownloadError> {
        if !enabled {
            return Ok(None);
        }
        let url = validate_url(url)?;
        let source_dir = source_dir.as_ref();
        std::fs::create_dir_all(source_dir)?;
        let Some(yt_dlp) = self.toolchain.yt_dlp.as_ref() else {
            return Ok(None);
        };
        let output_template = source_dir.join("video.%(ext)s");
        let mut command = Command::new(&yt_dlp.path);
        command.args([
            "-f",
            "bv*+ba/b",
            "--merge-output-format",
            "mp4",
            "--no-playlist",
            "--no-warnings",
            "-o",
        ]);
        command.arg(&output_template);
        if self.js_runtimes_flag {
            add_js_runtime(&mut command, &self.toolchain);
        }
        command.arg(url.as_str());

        match run_checked(command, self.download_timeout, cancel).await {
            Ok(_) => Ok(find_output(source_dir, "video")),
            Err(ProcessError::Cancelled) => Err(DownloadError::Process(ProcessError::Cancelled)),
            Err(error) => {
                let _ = cleanup_partials(source_dir, "video");
                tracing::warn!(%error, "video download failed; continuing without video");
                Ok(None)
            }
        }
    }

    fn require_js_runtime(&self) -> Result<(), DownloadError> {
        if self.toolchain.js_runtime().is_some() {
            Ok(())
        } else {
            Err(DownloadError::MissingTool("Deno or Node"))
        }
    }
}

pub fn validate_url(input: &str) -> Result<Url, DownloadError> {
    let url = Url::parse(input.trim()).map_err(|_| DownloadError::InvalidUrl(input.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(DownloadError::InvalidUrl(input.to_string()));
    }
    if url.host_str().is_none() {
        return Err(DownloadError::InvalidUrl(input.to_string()));
    }
    Ok(url)
}

fn add_js_runtime(command: &mut Command, toolchain: &Toolchain) {
    if let Some(runtime) = toolchain.js_runtime_name() {
        command.args(["--js-runtimes", runtime]);
    }
}

/// Older yt-dlp versions do not know `--js-runtimes`. Probe `--help` once per
/// downloader so the app works with both generations of the binary.
fn detect_js_runtimes_flag(toolchain: &Toolchain) -> bool {
    let Some(yt_dlp) = toolchain.yt_dlp.as_ref() else {
        return false;
    };
    let Ok(output) = std::process::Command::new(&yt_dlp.path)
        .arg("--help")
        .output()
    else {
        return false;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.contains("--js-runtimes")
}

fn find_output(directory: &Path, stem: &str) -> Option<PathBuf> {
    let prefix = format!("{stem}.");
    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(&prefix)
                        && !name.ends_with(".part")
                        && !name.ends_with(".ytdl")
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_http_and_https_only() {
        assert!(validate_url("https://www.youtube.com/watch?v=abc").is_ok());
        assert!(validate_url("http://example.com/video").is_ok());
        assert!(validate_url("ftp://example.com/video").is_err());
        assert!(validate_url("not a url").is_err());
        assert!(validate_url("file:///tmp/video.mp4").is_err());
    }

    #[test]
    fn missing_tools_are_reported() {
        let downloader = MediaDownloader::new(Toolchain::default());
        assert!(downloader.ensure_ready().is_err());
    }

    #[tokio::test]
    async fn metadata_requires_yt_dlp() {
        let downloader = MediaDownloader::new(Toolchain::default());
        let error = downloader
            .fetch_metadata("https://example.com/video", None)
            .await
            .unwrap_err();
        assert!(matches!(error, DownloadError::MissingTool("yt-dlp")));
    }

    #[tokio::test]
    async fn disabled_video_is_a_no_op() {
        let downloader = MediaDownloader::new(Toolchain::default());
        let directory = tempfile::tempdir().unwrap();
        let result = downloader
            .download_video("https://example.com/video", directory.path(), false, None)
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
