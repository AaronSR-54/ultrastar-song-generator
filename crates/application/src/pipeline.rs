//! End-to-end stage execution: download, separate, transcribe, analyse pitch,
//! align and export.
//!
//! The orchestrator owns no UI and no threads: it runs inside
//! [`crate::JobRunner`], reports progress through [`StageProgress`] and
//! persists artifacts under the project folder so every stage can resume.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use usdx_domain::{SongDoc, SongHeader, Transcript};
use usdx_format::{validate, Severity, ValidationIssue};
use usdx_media::{
    convert_to_analysis_wav, probe, sanitize_filename, save_square_bytes, verify_analysis_wav,
    MediaDownloader, Toolchain,
};
use usdx_ml::{
    align_with_voiced, analyze_file, dictionary_syllabifier, estimate_tempo, load_vocals_16k,
    quantize, Alignment, AlignmentOptions, CharonSeparator, EnergyTrack, QuantizedNote,
    SeparationService, TempoCandidate, TranscriptionOptions, TranscriptionService, WhisperEngine,
    WHISPER_SAMPLE_RATE,
};
use usdx_ports::{CancelFlag, SeparationControl, Transcriber, TranscriptionControl};

use crate::jobs::JobContext;
use crate::lyrics::{
    display_labels, transcript_from_record, LyricsClient, LyricsError, LyricsQuery,
};
use crate::project::{atomic_write, now_unix_ms, Project, ProjectPaths, ProjectStore};
use crate::stages::{StageId, StageStatus};

/// Progress callback shared with the ML ports and the GUI.
type ProgressFn = Arc<dyn Fn(f32, &str) + Send + Sync>;
/// Progress callback expected by the ML control structs.
type FractionFn = Arc<dyn Fn(f32) + Send + Sync>;

/// Audio file name inside the exported song folder.
pub const EXPORTED_AUDIO: &str = "audio.mp3";
/// Cover file name inside the exported song folder.
pub const EXPORTED_COVER: &str = "cover.jpg";
/// Video file name inside the exported song folder.
pub const EXPORTED_VIDEO: &str = "video.mp4";
/// `#MP3` value used by the generated document.
pub const HEADER_AUDIO: &str = "audio.mp3";

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("stage {stage:?} failed: {message}")]
    Stage { stage: StageId, message: String },
    #[error("export blocked by validation: {message}")]
    Validation {
        message: String,
        phrase: Option<usize>,
        note: Option<usize>,
    },
    #[error("job cancelled")]
    Cancelled,
    /// No online lyrics were found and the local fallback model is not
    /// downloaded yet. The app uses this to fetch Whisper on demand.
    #[error("the local Whisper model is required but missing: {}", path.display())]
    WhisperModelMissing { path: PathBuf },
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("project error: {0}")]
    Project(#[from] crate::project::ProjectError),
    #[error("invalid stage state: {0}")]
    State(#[from] crate::stages::StageTransitionError),
}

impl PipelineError {
    fn stage(stage: StageId, error: impl std::fmt::Display) -> Self {
        Self::Stage {
            stage,
            message: error.to_string(),
        }
    }
}

/// Progress and cancellation for one stage, cheap to clone and `'static`.
#[derive(Clone)]
pub struct StageProgress {
    cancel: CancelFlag,
    reporter: Option<ProgressFn>,
}

impl StageProgress {
    pub fn new(cancel: CancelFlag, reporter: ProgressFn) -> Self {
        Self {
            cancel,
            reporter: Some(reporter),
        }
    }

    /// In-memory progress sink for tests and the headless CLI.
    pub fn silent() -> Self {
        Self {
            cancel: CancelFlag::default(),
            reporter: None,
        }
    }

    pub fn from_job(context: &JobContext) -> Self {
        let context = context.clone();
        Self::new(
            context.cancel_flag(),
            Arc::new(move |fraction, message| context.report(fraction, message)),
        )
    }

    pub fn report(&self, fraction: f32, message: &str) {
        if let Some(reporter) = &self.reporter {
            reporter(fraction.clamp(0.0, 1.0), message);
        }
    }

    pub fn cancel_flag(&self) -> CancelFlag {
        self.cancel.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn check_cancelled(&self) -> Result<(), PipelineError> {
        if self.is_cancelled() {
            Err(PipelineError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Progress callback for the ML ports, scaled into `[offset, offset+span]`.
    fn reporter(&self, offset: f32, span: f32, message: &'static str) -> FractionFn {
        let progress = self.clone();
        Arc::new(move |fraction| progress.report(offset + fraction.clamp(0.0, 1.0) * span, message))
    }
}

/// Paths to the local models. Everything else is discovered from the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineConfig {
    pub separation_model: PathBuf,
    pub whisper_model: PathBuf,
    /// Look the lyrics up online before falling back to Whisper.
    pub online_lyrics: bool,
}

impl PipelineConfig {
    pub fn new(separation_model: impl Into<PathBuf>, whisper_model: impl Into<PathBuf>) -> Self {
        Self {
            separation_model: separation_model.into(),
            whisper_model: whisper_model.into(),
            online_lyrics: true,
        }
    }

    /// Keeps the whole pipeline offline.
    pub fn offline(self) -> Self {
        self.maybe_offline(true)
    }

    pub fn maybe_offline(mut self, offline: bool) -> Self {
        self.online_lyrics = !offline;
        self
    }
}

/// Tempo, header and quantized notes persisted by the pitch stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchArtifacts {
    pub header: SongHeader,
    pub candidates: Vec<TempoCandidate>,
    pub notes: Vec<QuantizedNote>,
    pub adjustments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOutcome {
    pub folder: PathBuf,
    pub song_file: PathBuf,
    pub warnings: Vec<String>,
}

pub struct Pipeline {
    store: ProjectStore,
    toolchain: Toolchain,
    config: PipelineConfig,
}

impl Pipeline {
    pub fn new(store: ProjectStore, toolchain: Toolchain, config: PipelineConfig) -> Self {
        Self {
            store,
            toolchain,
            config,
        }
    }

    pub fn store(&self) -> &ProjectStore {
        &self.store
    }

    pub fn paths(&self) -> &ProjectPaths {
        self.store.paths()
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Runs one stage, updating its persisted state and saving the project.
    pub async fn run_stage(
        &self,
        stage: StageId,
        project: &mut Project,
        progress: &StageProgress,
    ) -> Result<(), PipelineError> {
        progress.check_cancelled()?;
        let now = now_unix_ms();
        if project.stage(stage).status == StageStatus::Running {
            return Err(PipelineError::State(
                crate::stages::StageTransitionError::Invalid {
                    from: StageStatus::Running,
                    to: StageStatus::Running,
                },
            ));
        }
        project.stage_mut(stage).invalidate()?;
        project.stage_mut(stage).start(now)?;
        self.store.save(project)?;
        progress.report(0.0, stage.label());

        let outcome = self.execute(stage, project, progress).await;

        match outcome {
            Ok(()) => {
                project.stage_mut(stage).succeed(now_unix_ms())?;
                self.store.save(project)?;
                progress.report(1.0, stage.label());
                Ok(())
            }
            Err(PipelineError::Cancelled) => {
                project.stage_mut(stage).cancel()?;
                self.store.save(project)?;
                Err(PipelineError::Cancelled)
            }
            Err(error) => {
                project
                    .stage_mut(stage)
                    .fail(error.to_string(), now_unix_ms())?;
                self.store.save(project)?;
                Err(error)
            }
        }
    }

    /// Runs every pending or failed stage in dependency order.
    pub async fn run_pending(
        &self,
        project: &mut Project,
        progress: &StageProgress,
    ) -> Result<Vec<StageId>, PipelineError> {
        let mut ran = Vec::new();
        while let Some(stage) = project.next_stage() {
            self.run_stage(stage, project, progress).await?;
            ran.push(stage);
        }
        Ok(ran)
    }

    async fn execute(
        &self,
        stage: StageId,
        project: &mut Project,
        progress: &StageProgress,
    ) -> Result<(), PipelineError> {
        match stage {
            StageId::Ingest => self.ingest(project, progress).await,
            StageId::Separate => self.separate(progress),
            StageId::Transcribe => self.transcribe(project, progress).await,
            StageId::Pitch => self.pitch(project, progress),
            StageId::Align => self.align(project, progress),
            StageId::Export => {
                let document = read_song(self.paths())?;
                let outcome = self.export_document(&document)?;
                progress.report(1.0, &format!("wrote {}", outcome.folder.display()));
                Ok(())
            }
        }
    }

    async fn ingest(
        &self,
        project: &mut Project,
        progress: &StageProgress,
    ) -> Result<(), PipelineError> {
        let stage = StageId::Ingest;
        if project.source.url.trim().is_empty() {
            return Err(PipelineError::stage(stage, "the project has no source URL"));
        }
        let paths = self.paths();
        paths.ensure_dirs()?;
        let downloader = MediaDownloader::new(self.toolchain.clone());
        progress.report(0.05, "checking yt-dlp, ffmpeg and the JS runtime");
        downloader
            .ensure_ready()
            .map_err(|error| PipelineError::stage(stage, error))?;

        progress.report(0.1, "reading metadata");
        let metadata = downloader
            .fetch_metadata(&project.source.url, Some(progress.cancel_flag()))
            .await
            .map_err(|error| PipelineError::stage(stage, error))?;
        let (title, artist) = display_labels(&metadata.title, metadata.artist.as_deref());
        project.source.title = title;
        project.source.artist = (!artist.is_empty()).then_some(artist);
        project.source.duration_ms = Some(metadata.duration_ms);
        project.source.site = metadata.site.clone();
        project.source.thumbnail_url = metadata.thumbnail_url.clone();

        progress.report(0.2, "downloading audio");
        let audio = downloader
            .download_audio(
                &project.source.url,
                paths.source_dir(),
                Some(progress.cancel_flag()),
            )
            .await
            .map_err(|error| PipelineError::stage(stage, error))?;
        progress.report(0.55, "downloading video");
        let video = downloader
            .download_video(
                &project.source.url,
                paths.source_dir(),
                project.settings.download_video,
                Some(progress.cancel_flag()),
            )
            .await
            .map_err(|error| PipelineError::stage(stage, error))?;
        if let Some(video) = &video {
            // yt-dlp already writes `source/video.<ext>`; copying a file onto
            // itself fails on Windows.
            if video != &paths.video() {
                std::fs::copy(video, paths.video())?;
            }
        }

        if let Some(url) = &metadata.thumbnail_url {
            progress.report(0.65, "downloading cover");
            match download_cover(url, &paths.cover()).await {
                Ok(()) => {}
                Err(error) => tracing::warn!(%error, "cover download failed; continuing"),
            }
        }

        progress.report(0.72, "converting analysis audio");
        let ffmpeg = self.require_tool(stage, "ffmpeg", self.toolchain.ffmpeg.as_ref())?;
        let ffprobe = self.require_tool(stage, "ffprobe", self.toolchain.ffprobe.as_ref())?;
        convert_to_analysis_wav(
            &ffmpeg,
            &audio,
            &paths.analysis_wav(),
            Some(progress.cancel_flag()),
        )
        .await
        .map_err(|error| PipelineError::stage(stage, error))?;

        progress.report(0.95, "verifying analysis audio");
        let source_probe = probe(&ffprobe, &audio, Some(progress.cancel_flag()))
            .await
            .map_err(|error| PipelineError::stage(stage, error))?;
        let analysis_probe = probe(
            &ffprobe,
            &paths.analysis_wav(),
            Some(progress.cancel_flag()),
        )
        .await
        .map_err(|error| PipelineError::stage(stage, error))?;
        verify_analysis_wav(source_probe, analysis_probe)
            .map_err(|error| PipelineError::stage(stage, error))?;
        Ok(())
    }

    fn require_tool(
        &self,
        stage: StageId,
        name: &str,
        tool: Option<&usdx_media::Tool>,
    ) -> Result<PathBuf, PipelineError> {
        tool.map(|tool| tool.path.clone())
            .ok_or_else(|| PipelineError::stage(stage, format!("{name} was not found in PATH")))
    }

    fn separate(&self, progress: &StageProgress) -> Result<(), PipelineError> {
        let stage = StageId::Separate;
        progress.report(0.05, "loading the separation model");
        let separator = CharonSeparator::new(&self.config.separation_model)
            .map_err(|error| PipelineError::stage(stage, error))?;
        let service = SeparationService::new(separator, None);
        let control = SeparationControl {
            cancel: progress.cancel_flag(),
            progress: progress.reporter(0.1, 0.85, "separating stems"),
        };
        service
            .separate(
                &self.paths().analysis_wav(),
                &self.paths().stems_dir(),
                control,
            )
            .map_err(|error| PipelineError::stage(stage, error))?;
        Ok(())
    }

    async fn transcribe(
        &self,
        project: &Project,
        progress: &StageProgress,
    ) -> Result<(), PipelineError> {
        let stage = StageId::Transcribe;

        if self.config.online_lyrics {
            progress.report(0.02, "looking up the lyrics online");
            let language_hint = if project.settings.force_language {
                project.settings.language.as_deref()
            } else {
                None
            };
            let query = LyricsQuery::from_source(
                &project.source.title,
                project.source.artist.as_deref(),
                project.source.duration_ms,
            );
            let cancel = progress.cancel_flag();
            match LyricsClient::new().lookup(&query, Some(&cancel)).await {
                Ok(Some(record)) => {
                    if let Some(transcript) = transcript_from_record(&record, language_hint) {
                        write_json(&self.paths().transcript(), &transcript)?;
                        progress.report(0.95, "lyrics found online");
                        return Ok(());
                    }
                    tracing::info!(
                        track = %record.track_name,
                        "the lyrics record has no timings; falling back to Whisper"
                    );
                }
                Ok(None) => {
                    tracing::info!("no online lyrics found; falling back to Whisper");
                }
                Err(LyricsError::Cancelled) => return Err(PipelineError::Cancelled),
                Err(error) => {
                    tracing::warn!(%error, "lyrics lookup failed; falling back to Whisper");
                }
            }
        }

        // Whisper is downloaded on demand only when the online lookup fails.
        if !self.config.whisper_model.exists() {
            return Err(PipelineError::WhisperModelMissing {
                path: self.config.whisper_model.clone(),
            });
        }

        progress.report(0.1, "loading the Whisper model");
        let engine = WhisperEngine::new(&self.config.whisper_model)
            .map_err(|error| PipelineError::stage(stage, error))?;
        let service = TranscriptionService::new(engine, TranscriptionOptions::default());
        let forced = if project.settings.force_language {
            project.settings.language.as_deref()
        } else {
            None
        };
        let control = TranscriptionControl::new(
            progress.cancel_flag(),
            progress.reporter(0.1, 0.85, "transcribing vocals"),
        );
        let transcript = service
            .transcribe(&self.paths().vocals(), forced, control)
            .map_err(|error| PipelineError::stage(stage, error))?;
        write_json(&self.paths().transcript(), &transcript)?;
        Ok(())
    }

    fn pitch(&self, project: &Project, progress: &StageProgress) -> Result<(), PipelineError> {
        let stage = StageId::Pitch;
        progress.report(0.05, "analysing pitch");
        progress.check_cancelled()?;
        let analysis = analyze_file(self.paths().vocals())
            .map_err(|error| PipelineError::stage(stage, error))?;
        progress.report(0.6, "estimating tempo");
        let tempo = estimate_tempo(self.paths().analysis_wav())
            .map_err(|error| PipelineError::stage(stage, error))?;
        progress.report(0.8, "quantizing notes");

        let mut header = base_header(project);
        header.bpm = SongHeader::bpm_from_tempo(tempo.best.bpm_musical);
        header.gap_ms = tempo.best.gap_ms;
        let quantized = quantize(&analysis.notes, &header);
        let artifacts = PitchArtifacts {
            header,
            candidates: tempo.candidates,
            notes: quantized.notes,
            adjustments: quantized.adjustments.len(),
        };
        write_json(&self.paths().notes(), &artifacts)?;
        Ok(())
    }

    fn align(&self, project: &Project, progress: &StageProgress) -> Result<(), PipelineError> {
        let stage = StageId::Align;
        progress.report(0.1, "reading transcription and notes");
        let transcript: Transcript = read_json(&self.paths().transcript(), stage)?;
        let artifacts: PitchArtifacts = read_json(&self.paths().notes(), stage)?;
        progress.report(0.2, "measuring the vocal track");
        let voiced_segments = load_vocals_16k(self.paths().vocals())
            .map(|samples| {
                EnergyTrack::analyze(&samples, WHISPER_SAMPLE_RATE).voiced_segments(1.2, 1.0)
            })
            .unwrap_or_default();

        progress.report(0.4, "aligning syllables");

        let mut header = artifacts.header;
        header.title = project.source.title.clone();
        header.artist = project.source.artist.clone().unwrap_or_default();
        header.audio = HEADER_AUDIO.to_string();
        header.language = Some(transcript.language.clone());
        if header.title.is_empty() {
            header.title = "Untitled".into();
        }
        if header.artist.is_empty() {
            header.artist = "Unknown".into();
        }
        let quantized = usdx_ml::QuantizedSong {
            notes: artifacts.notes,
            adjustments: Vec::new(),
        };
        let alignment: Alignment = align_with_voiced(
            &transcript,
            &quantized,
            &header,
            dictionary_syllabifier(),
            AlignmentOptions::default(),
            Some(&voiced_segments),
        );
        progress.report(0.9, "writing song document");
        let document = alignment.to_song_doc(header);
        write_json(&self.paths().song(), &document)?;
        Ok(())
    }

    /// Folder that [`Self::export_document`] writes for `document`.
    pub fn export_folder(&self, document: &SongDoc) -> PathBuf {
        let folder_name = format!(
            "{} - {}",
            sanitize_filename(&document.header.artist),
            sanitize_filename(&document.header.title)
        );
        self.paths().output_dir().join(folder_name.trim())
    }

    /// Validates and writes the UltraStar folder. Also used by the editor, so
    /// a validation failure can select the offending note.
    pub fn export_document(&self, document: &SongDoc) -> Result<ExportOutcome, PipelineError> {
        let report = validate(document);
        if let Some(issue) = report
            .issues
            .iter()
            .find(|issue| issue.severity == Severity::Error)
        {
            return Err(validation_error(issue));
        }

        let paths = self.paths();
        let folder = self.export_folder(document);
        std::fs::create_dir_all(&folder)?;

        copy_if_present(
            &paths.source_dir().join("audio-source.mp3"),
            &folder.join(EXPORTED_AUDIO),
        )?;
        copy_if_present(&paths.cover(), &folder.join(EXPORTED_COVER))?;
        let has_video = copy_if_present(&paths.video(), &folder.join(EXPORTED_VIDEO))?;

        // The folder is the truth: UltraStar only gets a `#VIDEO` line when the
        // file is really next to the song. A stale tag is dropped instead of
        // leaving the game looking for a video that is not there.
        let mut document = document.clone();
        document.header.video = has_video.then(|| EXPORTED_VIDEO.to_string());

        let song_file = folder.join(format!(
            "{} - {}.txt",
            sanitize_filename(&document.header.artist),
            sanitize_filename(&document.header.title)
        ));
        usdx_format::write_file(&song_file, &document).map_err(|error| PipelineError::Stage {
            stage: StageId::Export,
            message: error.to_string(),
        })?;

        Ok(ExportOutcome {
            folder,
            song_file,
            warnings: report
                .issues
                .iter()
                .filter(|issue| issue.severity == Severity::Warning)
                .map(|issue| issue.to_string())
                .collect(),
        })
    }
}

fn validation_error(issue: &ValidationIssue) -> PipelineError {
    PipelineError::Validation {
        message: issue.message.clone(),
        phrase: issue.phrase,
        note: issue.note,
    }
}

fn base_header(project: &Project) -> SongHeader {
    SongHeader {
        title: project.source.title.clone(),
        artist: project.source.artist.clone().unwrap_or_default(),
        audio: HEADER_AUDIO.into(),
        language: project.settings.language.clone(),
        cover: Some(EXPORTED_COVER.into()),
        // The export stage writes `#VIDEO` only when the file was downloaded
        // and copied, so this stays empty here.
        video: None,
        ..SongHeader::default()
    }
}

async fn download_cover(url: &str, output: &Path) -> Result<(), String> {
    let response = reqwest::get(url).await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("cover request returned {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    save_square_bytes(&bytes, output, 512).map_err(|error| error.to_string())
}

fn copy_if_present(source: &Path, destination: &Path) -> Result<bool, PipelineError> {
    if !source.exists() {
        return Ok(false);
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, destination)?;
    Ok(true)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), PipelineError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| PipelineError::Stage {
        stage: StageId::Align,
        message: error.to_string(),
    })?;
    atomic_write(path, &bytes)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
    stage: StageId,
) -> Result<T, PipelineError> {
    let bytes = std::fs::read(path).map_err(|error| PipelineError::Stage {
        stage,
        message: format!("{} is missing or unreadable: {error}", path.display()),
    })?;
    serde_json::from_slice(&bytes).map_err(|error| PipelineError::Stage {
        stage,
        message: format!("{} is not valid JSON: {error}", path.display()),
    })
}

fn read_song(paths: &ProjectPaths) -> Result<SongDoc, PipelineError> {
    read_json(&paths.song(), StageId::Export)
}

#[cfg(test)]
mod tests {
    use super::*;
    use usdx_domain::{Note, NoteKind, Phrase, TranscriptLine, TranscriptWord};
    use usdx_ml::{QuantizedNote, QuantizedSong};
    use usdx_ports::CancelFlag;

    fn pipeline(root: &Path) -> Pipeline {
        let paths = ProjectPaths::new(root);
        paths.ensure_dirs().unwrap();
        let store = ProjectStore::open(root)
            .map(|(store, _)| store)
            .unwrap_or_else(|_| {
                ProjectStore::create(
                    root,
                    crate::project::SourceInfo::default(),
                    Default::default(),
                )
                .unwrap()
                .0
            });
        Pipeline::new(
            store,
            Toolchain {
                yt_dlp: None,
                ffmpeg: None,
                ffprobe: None,
                deno: None,
                node: None,
            },
            PipelineConfig::new("missing-demucs.onnx", "missing-whisper.bin"),
        )
    }

    fn project() -> Project {
        Project::new(
            crate::project::SourceInfo {
                url: "https://example.com/video".into(),
                title: "Luna".into(),
                artist: Some("Artista".into()),
                ..Default::default()
            },
            Default::default(),
        )
    }

    fn word(text: &str, start: f64, end: f64) -> TranscriptWord {
        TranscriptWord {
            text: text.into(),
            start_s: start,
            end_s: end,
            confidence: 0.9,
            ..TranscriptWord::default()
        }
    }

    fn document() -> SongDoc {
        SongDoc {
            header: SongHeader {
                title: "Luna".into(),
                artist: "Artista".into(),
                audio: HEADER_AUDIO.into(),
                bpm: 480.0,
                gap_ms: 0,
                ..SongHeader::default()
            },
            phrases: vec![Phrase {
                notes: vec![Note {
                    kind: NoteKind::Normal,
                    start_beat: 0,
                    length_beats: 2,
                    pitch: 0,
                    text: "ho".into(),
                    starts_word: true,
                    confidence: Some(0.9),
                }],
                break_after: None,
            }],
            has_end_marker: true,
            is_duet: false,
        }
    }

    #[tokio::test]
    async fn align_stage_reads_artifacts_and_writes_the_song() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let mut project = project();
        let transcript = Transcript {
            language: "es".into(),
            language_confidence: 0.9,
            lines: vec![TranscriptLine::new(vec![word("hola", 0.0, 0.5)])],
            discarded_segments: 0,
            word_timed: true,
        };
        write_json(&pipeline.paths().transcript(), &transcript).unwrap();
        let artifacts = PitchArtifacts {
            header: SongHeader {
                bpm: 120.0,
                gap_ms: 0,
                ..SongHeader::default()
            },
            candidates: Vec::new(),
            notes: vec![
                QuantizedNote {
                    start_beat: 0,
                    length_beats: 2,
                    pitch: 0,
                    confidence: 0.9,
                    adjusted: false,
                },
                QuantizedNote {
                    start_beat: 2,
                    length_beats: 2,
                    pitch: 0,
                    confidence: 0.9,
                    adjusted: false,
                },
            ],
            adjustments: 0,
        };
        write_json(&pipeline.paths().notes(), &artifacts).unwrap();

        pipeline
            .run_stage(StageId::Align, &mut project, &StageProgress::silent())
            .await
            .unwrap();

        let document: SongDoc = read_json(&pipeline.paths().song(), StageId::Align).unwrap();
        assert_eq!(document.header.artist, "Artista");
        assert_eq!(document.header.language.as_deref(), Some("es"));
        assert_eq!(document.note_count(), 2);
        assert_eq!(document.phrases[0].notes[0].text, "ho");
        assert_eq!(project.stage(StageId::Align).status, StageStatus::Done);
    }

    #[tokio::test]
    async fn export_writes_the_song_folder_and_reports_validation_errors() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let document = document();

        let outcome = pipeline.export_document(&document).unwrap();
        assert!(outcome.folder.ends_with("Artista - Luna"));
        assert!(outcome.song_file.exists());
        let written = std::fs::read_to_string(&outcome.song_file).unwrap();
        assert!(written.contains("#ARTIST:Artista"));
        assert!(written.contains("#MP3:audio.mp3"));

        let mut broken = document;
        broken.phrases[0].notes[0].text.clear();
        let error = pipeline.export_document(&broken).unwrap_err();
        match error {
            PipelineError::Validation { phrase, note, .. } => {
                assert_eq!(phrase, Some(0));
                assert_eq!(note, Some(0));
                assert_eq!(broken.phrases[0].notes[0].text, "");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn export_writes_the_video_tag_only_when_the_video_exists() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let document = document();

        // Without a downloaded video, no stale `#VIDEO` survives.
        let mut stale = document.clone();
        stale.header.video = Some(EXPORTED_VIDEO.into());
        let outcome = pipeline.export_document(&stale).unwrap();
        assert!(!outcome.folder.join(EXPORTED_VIDEO).exists());
        let written = std::fs::read_to_string(&outcome.song_file).unwrap();
        assert!(!written.contains("#VIDEO"));

        // With one, the file is copied and the header points at it.
        std::fs::write(pipeline.paths().video(), b"fake video").unwrap();
        let outcome = pipeline.export_document(&document).unwrap();
        assert!(outcome.folder.join(EXPORTED_VIDEO).exists());
        let written = std::fs::read_to_string(&outcome.song_file).unwrap();
        assert!(written.contains("#VIDEO:video.mp4"));
    }

    #[tokio::test]
    async fn a_failed_stage_is_persisted_with_its_cause() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let mut project = project();
        let error = pipeline
            .run_stage(StageId::Separate, &mut project, &StageProgress::silent())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("missing-demucs.onnx"), "{error}");
        assert_eq!(project.stage(StageId::Separate).status, StageStatus::Failed);
        assert!(project
            .stage(StageId::Separate)
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("missing-demucs.onnx"));
    }

    #[tokio::test]
    async fn cancelled_stages_return_to_pending() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let mut project = project();
        let progress = StageProgress::new(CancelFlag::default(), Arc::new(|_, _| {}));
        progress.cancel_flag().cancel();
        let error = pipeline
            .run_stage(StageId::Pitch, &mut project, &progress)
            .await
            .unwrap_err();
        assert!(matches!(error, PipelineError::Cancelled));
        assert_eq!(project.stage(StageId::Pitch).status, StageStatus::Pending);
    }

    #[tokio::test]
    async fn ingest_without_a_url_fails_before_spawning_tools() {
        let directory = tempfile::tempdir().unwrap();
        let pipeline = pipeline(directory.path());
        let mut project = project();
        project.source.url.clear();
        let error = pipeline
            .run_stage(StageId::Ingest, &mut project, &StageProgress::silent())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("source URL"));
    }

    #[test]
    fn quantization_inputs_round_trip_through_json() {
        let notes = QuantizedSong {
            notes: vec![QuantizedNote {
                start_beat: 3,
                length_beats: 2,
                pitch: -1,
                confidence: 0.7,
                adjusted: true,
            }],
            adjustments: Vec::new(),
        };
        let artifacts = PitchArtifacts {
            header: SongHeader::default(),
            candidates: Vec::new(),
            notes: notes.notes,
            adjustments: 0,
        };
        let json = serde_json::to_string(&artifacts).unwrap();
        let parsed: PitchArtifacts = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, artifacts);
    }
}
