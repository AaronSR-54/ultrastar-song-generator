// Release builds are GUI apps: no console window behind the main window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if std::env::args().any(|argument| argument == "--selftest") {
        run_selftest()?;
        return Ok(());
    }

    tracing::info!("ultrastar-song-generator starting");
    usdx_desktop::run()?;
    Ok(())
}

/// Headless smoke test used by the release builds: detects external tools,
/// loads the embedded model catalog and creates/opens a throwaway project.
fn run_selftest() -> anyhow::Result<()> {
    use usdx_application::{ModelCatalog, ProjectSettings, ProjectStore, SourceInfo};
    use usdx_media::Toolchain;

    let toolchain = Toolchain::detect();
    println!(
        "tools: yt-dlp={:?} ffmpeg={:?} ffprobe={:?} node={:?} deno={:?}",
        toolchain.yt_dlp.as_ref().map(|tool| tool.version.clone()),
        toolchain.ffmpeg.as_ref().map(|tool| tool.version.clone()),
        toolchain.ffprobe.as_ref().map(|tool| tool.version.clone()),
        toolchain.node.as_ref().map(|tool| tool.version.clone()),
        toolchain.deno.as_ref().map(|tool| tool.version.clone()),
    );

    let catalog = ModelCatalog::embedded()?;
    let models: Vec<String> = catalog
        .models
        .iter()
        .map(|spec| format!("{} ({:?})", spec.id, spec.purpose))
        .collect();
    println!("models: {}", models.join(", "));

    let root = std::env::temp_dir().join(format!("usdx-selftest-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    let (store, project) = ProjectStore::create(
        root.clone(),
        SourceInfo {
            url: "https://example.com/selftest".into(),
            title: "Selftest".into(),
            ..SourceInfo::default()
        },
        ProjectSettings::default(),
    )?;
    let reopened = store.load()?;
    println!(
        "project: {} stages, {} settings, reopened={}",
        reopened.stages.len(),
        project.settings.separation_model,
        reopened.id == project.id
    );
    std::fs::remove_dir_all(&root)?;
    println!("selftest ok");
    Ok(())
}
