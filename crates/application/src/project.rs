use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::stages::{StageId, StageState, StageStatus, StageTransitionError};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceInfo {
    pub url: String,
    pub site: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub duration_ms: Option<u64>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub language: Option<String>,
    pub force_language: bool,
    pub download_video: bool,
    pub audio_quality: String,
    pub separation_model: String,
    pub whisper_model: String,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            language: None,
            force_language: false,
            download_video: false,
            audio_quality: "best".into(),
            separation_model: "htdemucs".into(),
            whisper_model: "medium".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub created_at_unix_ms: u64,
    pub source: SourceInfo,
    pub settings: ProjectSettings,
    pub stages: BTreeMap<StageId, StageState>,
}

impl Project {
    pub fn new(source: SourceInfo, settings: ProjectSettings) -> Self {
        Self::new_with_id(
            format!("{:x}-{}", now_unix_ms(), slugify(&source.title)),
            source,
            settings,
            now_unix_ms(),
        )
    }

    pub fn new_with_id(
        id: impl Into<String>,
        source: SourceInfo,
        settings: ProjectSettings,
        created_at_unix_ms: u64,
    ) -> Self {
        let stages = StageId::ALL
            .into_iter()
            .map(|stage| (stage, StageState::default()))
            .collect();
        Self {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            created_at_unix_ms,
            source,
            settings,
            stages,
        }
    }

    pub fn stage(&self, stage: StageId) -> &StageState {
        self.stages.get(&stage).expect("all stages are initialised")
    }

    pub fn stage_mut(&mut self, stage: StageId) -> &mut StageState {
        self.stages
            .get_mut(&stage)
            .expect("all stages are initialised")
    }

    /// Invalidates a stage and everything that depends on it.
    pub fn invalidate_from(
        &mut self,
        changed: StageId,
    ) -> Result<Vec<StageId>, StageTransitionError> {
        let mut invalidated = Vec::new();
        for stage in changed.dependents() {
            self.stage_mut(*stage).invalidate()?;
            invalidated.push(*stage);
        }
        Ok(invalidated)
    }

    /// First stage that is pending or failed, i.e. where a resume should start.
    pub fn next_stage(&self) -> Option<StageId> {
        StageId::ALL.into_iter().find(|stage| {
            matches!(
                self.stage(*stage).status,
                StageStatus::Pending | StageStatus::Failed
            )
        })
    }

    pub fn is_complete(&self) -> bool {
        StageId::ALL
            .into_iter()
            .all(|stage| self.stage(stage).is_done())
    }
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_dash = false;
    for character in value.chars() {
        let mapped = match character {
            'a'..='z' | '0'..='9' => Some(character),
            'A'..='Z' => Some(character.to_ascii_lowercase()),
            _ => None,
        };
        if let Some(character) = mapped {
            slug.push(character);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "song".into()
    } else {
        slug
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("project already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("project not found at {0}")]
    NotFound(PathBuf),
    #[error("unsupported project schema version {0}")]
    UnsupportedSchema(u32),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid project JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stage transition: {0}")]
    Transition(#[from] StageTransitionError),
}

/// Canonical paths inside a project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPaths {
    pub root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn source_dir(&self) -> PathBuf {
        self.root.join("source")
    }

    pub fn work_dir(&self) -> PathBuf {
        self.root.join("work")
    }

    pub fn stems_dir(&self) -> PathBuf {
        self.work_dir().join("stems")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("output")
    }

    pub fn project_file(&self) -> PathBuf {
        self.root.join("project.json")
    }

    pub fn source_audio(&self) -> PathBuf {
        self.source_dir().join("audio-source.mp3")
    }

    pub fn analysis_wav(&self) -> PathBuf {
        self.work_dir().join("audio-48k-mono.wav")
    }

    pub fn vocals(&self) -> PathBuf {
        self.stems_dir().join("vocals.wav")
    }

    pub fn instrumental(&self) -> PathBuf {
        self.stems_dir().join("instrumental.wav")
    }

    pub fn cover(&self) -> PathBuf {
        self.source_dir().join("cover.jpg")
    }

    pub fn video(&self) -> PathBuf {
        self.source_dir().join("video.mp4")
    }

    pub fn transcript(&self) -> PathBuf {
        self.work_dir().join("transcript.json")
    }

    pub fn pitch(&self) -> PathBuf {
        self.work_dir().join("pitch.json")
    }

    pub fn notes(&self) -> PathBuf {
        self.work_dir().join("notes.json")
    }

    pub fn peaks(&self) -> PathBuf {
        self.work_dir().join("peaks.bin")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for directory in [
            self.root.clone(),
            self.source_dir(),
            self.work_dir(),
            self.stems_dir(),
            self.output_dir(),
        ] {
            fs::create_dir_all(directory)?;
        }
        Ok(())
    }
}

/// Reads, creates and saves projects on disk.
#[derive(Debug, Clone)]
pub struct ProjectStore {
    paths: ProjectPaths,
}

impl ProjectStore {
    pub fn create(
        root: impl Into<PathBuf>,
        source: SourceInfo,
        settings: ProjectSettings,
    ) -> Result<(Self, Project), ProjectError> {
        let paths = ProjectPaths::new(root);
        if paths.project_file().exists() {
            return Err(ProjectError::AlreadyExists(paths.root));
        }
        paths.ensure_dirs()?;
        let project = Project::new(source, settings);
        let store = Self { paths };
        store.save(&project)?;
        Ok((store, project))
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<(Self, Project), ProjectError> {
        let paths = ProjectPaths::new(root);
        if !paths.project_file().exists() && !backup_path(&paths.project_file()).exists() {
            return Err(ProjectError::NotFound(paths.root));
        }
        let store = Self { paths };
        let project = store.load()?;
        Ok((store, project))
    }

    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    pub fn load(&self) -> Result<Project, ProjectError> {
        let project: Project = self.read_json(&self.paths.project_file())?;
        if project.schema_version != SCHEMA_VERSION {
            return Err(ProjectError::UnsupportedSchema(project.schema_version));
        }
        Ok(project)
    }

    pub fn save(&self, project: &Project) -> Result<(), ProjectError> {
        if project.schema_version != SCHEMA_VERSION {
            return Err(ProjectError::UnsupportedSchema(project.schema_version));
        }
        let bytes = serde_json::to_vec_pretty(project)?;
        atomic_write(&self.paths.project_file(), &bytes)?;
        Ok(())
    }

    fn read_json<T: for<'de> Deserialize<'de>>(&self, path: &Path) -> Result<T, ProjectError> {
        recover_backup(path)?;
        match fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(ProjectError::NotFound(path.to_path_buf()))
            }
            Err(error) => Err(ProjectError::Io(error)),
        }
    }
}

fn backup_path(path: &Path) -> PathBuf {
    sibling(path, "bak")
}

fn temp_path(path: &Path) -> PathBuf {
    sibling(path, "tmp")
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project.json");
    parent.join(format!(".{name}.{suffix}"))
}

/// Writes bytes through a temporary file, keeping a backup on Windows where
/// `rename` cannot replace an existing file atomically.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = temp_path(path);
    let backup = backup_path(path);

    {
        let mut file = File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp, path)?;
    }

    #[cfg(windows)]
    {
        if path.exists() {
            let _ = fs::remove_file(&backup);
            fs::rename(path, &backup)?;
            if let Err(error) = fs::rename(&temp, path) {
                let _ = fs::rename(&backup, path);
                let _ = fs::remove_file(&temp);
                return Err(error);
            }
            let _ = fs::remove_file(&backup);
        } else {
            fs::rename(&temp, path)?;
        }
    }

    Ok(())
}

fn recover_backup(path: &Path) -> std::io::Result<()> {
    let backup = backup_path(path);
    if !path.exists() && backup.exists() {
        fs::rename(&backup, path)?;
    }
    Ok(())
}

/// Debounced autosave helper. The caller marks dirty on every edit and asks
/// whether enough time has passed to flush.
#[derive(Debug, Clone)]
pub struct Autosave {
    delay: Duration,
    dirty_since: Option<Instant>,
}

impl Autosave {
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            dirty_since: None,
        }
    }

    pub fn mark_dirty(&mut self, now: Instant) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
    }

    pub fn should_save(&self, now: Instant) -> bool {
        self.dirty_since
            .map(|dirty| now.saturating_duration_since(dirty) >= self.delay)
            .unwrap_or(false)
    }

    pub fn mark_saved(&mut self) {
        self.dirty_since = None;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> SourceInfo {
        SourceInfo {
            url: "https://example.com/watch?v=abc".into(),
            title: "My Song".into(),
            ..SourceInfo::default()
        }
    }

    fn all_done(project: &mut Project) {
        for stage in StageId::ALL {
            let state = project.stage_mut(stage);
            state.status = StageStatus::Done;
        }
    }

    #[test]
    fn create_and_reopen_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("song");
        let (store, project) =
            ProjectStore::create(&root, source(), ProjectSettings::default()).unwrap();

        assert!(store.paths().source_dir().is_dir());
        assert!(store.paths().work_dir().is_dir());
        assert!(store.paths().stems_dir().is_dir());
        assert!(store.paths().output_dir().is_dir());
        assert_eq!(project.schema_version, SCHEMA_VERSION);
        assert_eq!(project.next_stage(), Some(StageId::Ingest));
        assert!(!project.is_complete());

        let (_store, reopened) = ProjectStore::open(&root).unwrap();
        assert_eq!(reopened, project);
    }

    #[test]
    fn existing_project_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("song");
        ProjectStore::create(&root, source(), ProjectSettings::default()).unwrap();
        let error = ProjectStore::create(&root, source(), ProjectSettings::default())
            .expect_err("must fail");
        assert!(matches!(error, ProjectError::AlreadyExists(_)));
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("song");
        let (store, mut project) =
            ProjectStore::create(&root, source(), ProjectSettings::default()).unwrap();
        project.schema_version = 999;
        let bytes = serde_json::to_vec(&project).unwrap();
        std::fs::write(store.paths().project_file(), bytes).unwrap();
        let error = ProjectStore::open(&root).expect_err("must fail");
        assert!(matches!(error, ProjectError::UnsupportedSchema(999)));
    }

    #[test]
    fn truncated_temp_file_does_not_corrupt_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("song");
        let (store, project) =
            ProjectStore::create(&root, source(), ProjectSettings::default()).unwrap();
        std::fs::write(
            sibling(&store.paths().project_file(), "tmp"),
            b"{ this is truncated",
        )
        .unwrap();
        let (_store, reopened) = ProjectStore::open(&root).unwrap();
        assert_eq!(reopened, project);
    }

    #[test]
    fn backup_is_restored_when_main_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("song");
        let (store, project) =
            ProjectStore::create(&root, source(), ProjectSettings::default()).unwrap();
        let main = store.paths().project_file();
        std::fs::rename(&main, sibling(&main, "bak")).unwrap();
        let (_store, reopened) = ProjectStore::open(&root).unwrap();
        assert_eq!(reopened, project);
    }

    #[test]
    fn invalidation_matrix_follows_stage_dependencies() {
        let mut project = Project::new_with_id("id", source(), ProjectSettings::default(), 0);
        all_done(&mut project);

        let invalidated = project.invalidate_from(StageId::Transcribe).unwrap();
        assert_eq!(
            invalidated,
            vec![StageId::Transcribe, StageId::Align, StageId::Export]
        );
        assert_eq!(project.stage(StageId::Ingest).status, StageStatus::Done);
        assert_eq!(project.stage(StageId::Separate).status, StageStatus::Done);
        assert_eq!(project.stage(StageId::Pitch).status, StageStatus::Done);
        assert_eq!(
            project.stage(StageId::Transcribe).status,
            StageStatus::Pending
        );
        assert_eq!(project.stage(StageId::Align).status, StageStatus::Pending);
        assert_eq!(project.stage(StageId::Export).status, StageStatus::Pending);

        all_done(&mut project);
        let invalidated = project.invalidate_from(StageId::Ingest).unwrap();
        assert_eq!(invalidated.len(), 6);
        assert!(StageId::ALL
            .into_iter()
            .all(|stage| project.stage(stage).status == StageStatus::Pending));
    }

    #[test]
    fn next_stage_resumes_at_failure() {
        let mut project = Project::new_with_id("id", source(), ProjectSettings::default(), 0);
        project.stage_mut(StageId::Ingest).status = StageStatus::Done;
        project.stage_mut(StageId::Separate).status = StageStatus::Failed;
        assert_eq!(project.next_stage(), Some(StageId::Separate));
    }

    #[test]
    fn autosave_waits_for_the_debounce_delay() {
        let now = Instant::now();
        let mut autosave = Autosave::new(Duration::from_secs(2));
        assert!(!autosave.should_save(now));
        autosave.mark_dirty(now);
        assert!(!autosave.should_save(now + Duration::from_millis(1500)));
        assert!(autosave.should_save(now + Duration::from_secs(2)));
        autosave.mark_saved();
        assert!(!autosave.is_dirty());
        assert!(!autosave.should_save(now + Duration::from_secs(10)));
    }

    #[test]
    fn slugify_is_portable() {
        assert_eq!(slugify("My Song: Part 2"), "my-song-part-2");
        assert_eq!(slugify("  ¡Hola, mundo!  "), "hola-mundo");
        assert_eq!(slugify("???"), "song");
    }
}
