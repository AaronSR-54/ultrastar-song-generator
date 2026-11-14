//! Media ingestion: yt-dlp/ffmpeg orchestration, metadata and cover art.

pub mod audio;
pub mod cover;
pub mod download;
pub mod filename;
pub mod metadata;
pub mod process;
pub mod tools;

pub use audio::{convert_to_analysis_wav, probe, verify_analysis_wav, AudioError, AudioProbe};
pub use cover::{
    download_cover, replace_cover, save_square_bytes, save_square_cover, CoverError, COVER_SIZE,
};
pub use download::{validate_url, DownloadError, MediaDownloader};
pub use filename::{sanitize_filename, unique_path};
pub use metadata::{parse_yt_dlp_json, MetadataError, VideoMetadata};
pub use process::{
    cleanup_partials, run, run_checked, std_command, tokio_command, ProcessError, ProcessOutput,
};
pub use tools::{Tool, ToolKind, Toolchain, ToolchainError, ToolchainIssue};
