//! Zero-configuration helpers: where projects and models live, how a URL
//! becomes a folder name, and how a finished song reaches its destination.

use std::path::{Path, PathBuf};
use std::process::Command;

use usdx_application::{
    now_unix_ms, JobContext, JobError, ModelCatalog, ModelDownloader, ModelError, ModelSpec,
    ModelStore,
};
use usdx_media::sanitize_filename;

/// Models are found here, in order, before anything is downloaded.
pub fn model_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(directory) = std::env::var_os("USDX_MODELS_DIR") {
        directories.push(PathBuf::from(directory));
    }
    directories.push(std::env::temp_dir().join("usdx-models"));
    directories.push(ModelStore::default_root());
    if let Ok(current) = std::env::current_dir() {
        directories.push(current.join("models"));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            directories.push(parent.join("models"));
        }
    }
    directories
}

/// Root where missing models are downloaded (persistent, unlike a temp dir).
pub fn download_root() -> PathBuf {
    std::env::var_os("USDX_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(ModelStore::default_root)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    pub separation: PathBuf,
    pub whisper: PathBuf,
    pub separation_ready: bool,
    pub whisper_ready: bool,
}

impl Models {
    /// Separation is the only model needed to start: Whisper is downloaded on
    /// demand, and only when the online lyrics lookup finds nothing usable.
    pub fn can_process(&self) -> bool {
        self.separation_ready
    }
}

/// Resolves both models without touching the network.
pub fn models() -> Models {
    let catalog = ModelCatalog::embedded().ok();
    let separation = locate(
        catalog.as_ref().and_then(|catalog| catalog.get("htdemucs")),
        "htdemucs.onnx",
        316_446_953,
        "USDX_HTDEMUCS_MODEL",
    );
    let whisper = locate(
        catalog
            .as_ref()
            .and_then(|catalog| catalog.get("whisper-large-v3-turbo-q5")),
        "ggml-large-v3-turbo-q5_0.bin",
        574_041_195,
        "USDX_WHISPER_MODEL",
    );
    Models {
        separation: separation.0,
        whisper: whisper.0,
        separation_ready: separation.1,
        whisper_ready: whisper.1,
    }
}

fn locate(
    spec: Option<&ModelSpec>,
    fallback_name: &str,
    fallback_size: u64,
    env: &str,
) -> (PathBuf, bool) {
    let (name, size) = spec
        .map(|spec| (spec.filename.as_str(), spec.size_bytes))
        .unwrap_or((fallback_name, fallback_size));

    if let Some(path) = std::env::var_os(env).map(PathBuf::from) {
        if present(&path, size) {
            return (path, true);
        }
    }
    for directory in model_directories() {
        let path = directory.join(name);
        if present(&path, size) {
            return (path, true);
        }
    }
    (download_root().join(name), false)
}

fn present(path: &Path, size: u64) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && (size == 0 || metadata.len() == size))
        .unwrap_or(false)
}

/// `USDX_OFFLINE=1` skips the online lyrics lookup.
pub fn offline() -> bool {
    std::env::var("USDX_OFFLINE")
        .map(|value| !value.trim().is_empty() && value.trim() != "0")
        .unwrap_or(false)
}

/// Downloads the separation model if it is missing.
pub async fn ensure_separation(context: &JobContext) -> Result<PathBuf, JobError> {
    let found = models();
    if found.separation_ready {
        return Ok(found.separation);
    }
    download(context, "htdemucs", "HTDemucs").await
}

/// Downloads Whisper, the fallback used only when online lyrics are missing.
pub async fn ensure_whisper(context: &JobContext) -> Result<PathBuf, JobError> {
    let found = models();
    if found.whisper_ready {
        return Ok(found.whisper);
    }
    download(context, "whisper-large-v3-turbo-q5", "Whisper").await
}

/// Resumes partial files and verifies their SHA-256 before using them.
async fn download(context: &JobContext, id: &str, label: &str) -> Result<PathBuf, JobError> {
    let catalog = ModelCatalog::embedded()
        .map_err(|error| JobError::Failed(format!("catálogo de modelos: {error}")))?;
    let spec: ModelSpec = catalog
        .get(id)
        .cloned()
        .ok_or_else(|| JobError::Failed(format!("el catálogo de modelos no incluye {label}")))?;
    let downloader = ModelDownloader::new(ModelStore::new(download_root()));
    let mut last_percent = -1_i32;

    context.report(0.0, format!("Descargando {label}…"));
    let path = downloader
        .download(
            &spec,
            |downloaded, total| {
                let fraction = if total == 0 {
                    0.0
                } else {
                    downloaded as f64 / total as f64
                };
                let percent = (fraction * 100.0) as i32;
                if percent != last_percent {
                    last_percent = percent;
                    context.report(fraction as f32, format!("Descargando {label} · {percent}%"));
                }
            },
            Some(context.cancel_flag()),
        )
        .await
        .map_err(|error| match error {
            ModelError::Cancelled => JobError::Cancelled,
            other => JobError::Failed(format!("No se pudo descargar {label}: {other}")),
        })?;
    Ok(path)
}

/// Root of the working projects. Everything inside is resumable.
pub fn projects_root() -> PathBuf {
    if let Some(directory) = std::env::var_os("USDX_PROJECTS_DIR") {
        return PathBuf::from(directory);
    }
    directories::ProjectDirs::from("", "", "ultrastar-song-generator")
        .map(|directories| directories.data_dir().join("projects"))
        .unwrap_or_else(|| PathBuf::from("projects"))
}

/// One folder per URL: the YouTube id when there is one, otherwise a
/// timestamp. Existing folders are never overwritten.
pub fn unique_project_root(url: &str) -> PathBuf {
    let base = projects_root();
    let stem =
        youtube_id(url).unwrap_or_else(|| sanitize_filename(&format!("song-{}", now_unix_ms())));
    let mut root = base.join(&stem);
    let mut index = 2_u32;
    while root.exists() {
        root = base.join(format!("{stem}-{index}"));
        index += 1;
    }
    root
}

/// Extracts the video id from the usual YouTube URL shapes.
pub fn youtube_id(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let id = if host == "youtu.be" || host.ends_with(".youtu.be") {
        parsed
            .path_segments()?
            .find(|segment| !segment.is_empty())?
            .to_owned()
    } else if let Some((_, value)) = parsed.query_pairs().find(|(key, _)| key.as_ref() == "v") {
        value.into_owned()
    } else {
        let segments: Vec<&str> = parsed
            .path_segments()?
            .filter(|segment| !segment.is_empty())
            .collect();
        match segments.as_slice() {
            [kind, id, ..] if matches!(*kind, "shorts" | "live" | "embed" | "v") => {
                (*id).to_owned()
            }
            _ => return None,
        }
    };
    let valid = (6..=32).contains(&id.len())
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    valid.then_some(id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedSong {
    pub title: String,
    pub artist: String,
    /// Folder produced by the export stage.
    pub folder: PathBuf,
    /// Folder inside the user's destination, when copying succeeded.
    pub copied_to: Option<PathBuf>,
    pub copy_error: Option<String>,
}

impl FinishedSong {
    /// Folder the "Open folder" action should reveal.
    pub fn display_folder(&self) -> &Path {
        self.copied_to.as_deref().unwrap_or(&self.folder)
    }
}

/// Copies every file of the exported song into `destination_root`, keeping the
/// folder name. Existing files are overwritten but nothing is deleted.
pub fn copy_song_folder(source: &Path, destination_root: &Path) -> std::io::Result<PathBuf> {
    let name = source
        .file_name()
        .ok_or_else(|| std::io::Error::other("la carpeta exportada no tiene nombre"))?;
    let target = destination_root.join(name);
    if let (Ok(source), Ok(target)) = (
        std::fs::canonicalize(source),
        std::fs::canonicalize(&target),
    ) {
        if source == target {
            return Ok(target);
        }
    }
    copy_directory(source, &target)?;
    Ok(target)
}

fn copy_directory(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &destination)?;
        }
    }
    Ok(())
}

/// Opens a folder in the system file manager.
pub fn open_folder(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    command.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_youtube_ids() {
        assert_eq!(
            youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
        assert_eq!(
            youtube_id("https://youtu.be/dQw4w9WgXcQ?t=12"),
            Some("dQw4w9WgXcQ".into())
        );
        assert_eq!(
            youtube_id("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
        assert_eq!(
            youtube_id("https://www.youtube.com/live/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".into())
        );
        assert_eq!(youtube_id("https://example.com/watch?v=abc"), None);
    }

    #[test]
    fn copies_existing_files_without_deleting_others() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source").join("Artista - Luna");
        std::fs::create_dir_all(source.join("sub")).unwrap();
        std::fs::write(source.join("song.txt"), b"new").unwrap();
        std::fs::write(source.join("sub").join("audio.mp3"), b"audio").unwrap();

        let destination = directory.path().join("destination");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("notes.txt"), b"keep").unwrap();

        let target = copy_song_folder(&source, &destination).unwrap();
        assert_eq!(std::fs::read(target.join("song.txt")).unwrap(), b"new");
        assert_eq!(
            std::fs::read(target.join("sub").join("audio.mp3")).unwrap(),
            b"audio"
        );
        assert!(destination.join("notes.txt").exists());
    }
}
