use std::path::PathBuf;

use usdx_media::{
    convert_to_analysis_wav, download_cover, probe, verify_analysis_wav, MediaDownloader, Toolchain,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://www.youtube.com/watch?v=BaW_jenozKc".to_string());
    let root = PathBuf::from(
        std::env::args()
            .nth(2)
            .unwrap_or_else(|| "ingest-smoke".to_string()),
    );
    let source = root.join("source");
    let work = root.join("work");
    std::fs::create_dir_all(&source)?;
    std::fs::create_dir_all(&work)?;

    let toolchain = Toolchain::detect();
    toolchain.ensure_ready()?;
    let downloader = MediaDownloader::new(toolchain);

    let metadata = downloader.fetch_metadata(&url, None).await?;
    println!("metadata: {metadata:?}");

    let audio = downloader.download_audio(&url, &source, None).await?;
    println!("audio: {}", audio.display());

    let cover = source.join("cover.jpg");
    download_cover(&metadata, &cover, None).await?;
    println!("cover: {}", cover.display());

    let analysis = work.join("audio-48k-mono.wav");
    let toolchain = &downloader.toolchain;
    let ffmpeg = toolchain.ffmpeg.as_ref().ok_or("ffmpeg missing")?;
    let ffprobe = toolchain.ffprobe.as_ref().ok_or("ffprobe missing")?;
    convert_to_analysis_wav(&ffmpeg.path, &audio, &analysis, None).await?;
    let source_probe = probe(&ffprobe.path, &audio, None).await?;
    let analysis_probe = probe(&ffprobe.path, &analysis, None).await?;
    verify_analysis_wav(source_probe, analysis_probe)?;
    println!("analysis: {}", analysis.display());
    println!("source probe: {source_probe:?}");
    println!("analysis probe: {analysis_probe:?}");

    let want_video = std::env::var("USDX_VIDEO").as_deref() == Ok("1");
    let video = downloader
        .download_video(&url, &source, want_video, None)
        .await?;
    println!("video: {video:?}");
    Ok(())
}
