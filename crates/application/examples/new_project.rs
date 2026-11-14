use usdx_application::{ProjectSettings, ProjectStore, SourceInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "project".to_string());
    let source = SourceInfo {
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
        site: Some("YouTube".into()),
        title: "My Song".into(),
        artist: Some("Artist".into()),
        duration_ms: Some(215_000),
        thumbnail_url: None,
    };
    let (store, project) = ProjectStore::create(&root, source, ProjectSettings::default())?;
    println!("{}", serde_json::to_string_pretty(&project)?);
    eprintln!("project: {}", store.paths().project_file().display());
    Ok(())
}
