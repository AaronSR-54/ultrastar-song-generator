//! eframe application: one URL in, one playable UltraStar song out.
//!
//! The app deliberately exposes a single path: paste a YouTube link, press
//! Process, watch the stages, open the folder. Everything else (projects,
//! models, export) happens on its own.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Id, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use url::Url;
use usdx_application::{
    atomic_write, JobError, JobEvent, JobHandle, JobRunner, Pipeline, PipelineConfig,
    PipelineError, Project, ProjectPaths, ProjectSettings, ProjectStore, SourceInfo, StageId,
    StageProgress, StageStatus, EXPORTED_VIDEO,
};
use usdx_domain::{Note, NoteKind, Phrase, SongDoc, SongHeader};
use usdx_media::{DownloadError, MediaDownloader, ProcessError, Toolchain};

use crate::editor::{EditorError, EditorState, ExportCheck, NoteSelection};
use crate::icons::Icon;
use crate::native_bar;
use crate::playback::{AudioEngine, PlaybackError};
use crate::settings::{GuiSettings, RecentSong, WindowBar};
use crate::setup::{self, FinishedSong};
use crate::theme::{self, ButtonKind, Glyph};

const HOME_WIDTH: f32 = 560.0;
const FLOW_WIDTH: f32 = 520.0;
const STATUS_BAR_H: f32 = 30.0;
/// Steps kept on screen at once, counting the running one.
const STEP_SLOTS: usize = 3;
const STEP_ROW_H: f32 = 28.0;
/// Characters per second used when printing a step message.
const TYPE_CPS: f32 = 38.0;
/// Duration of one highlight pass over a finished message.
const SWEEP_SECONDS: f32 = 1.2;
/// Time between the start of one pass and the next.
const SWEEP_CYCLE: f32 = 4.2;
const TOAST_SECONDS: f32 = 3.0;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Working,
    Done,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Models,
    Pipeline,
    /// Fetches the source video for a song that was processed without one.
    Video,
}

/// One edit requested by the preview screen, applied once the borrow of the
/// editor ends.
#[derive(Debug, Clone, Copy)]
enum EditAction {
    Delete,
    Kind(NoteKind),
    ShiftGap(i64),
    AlignGapToCursor(f64),
    Undo,
    Redo,
}

/// What the running screen asks for once its borrow ends.
#[derive(Debug, Clone, Copy)]
enum FlowAction {
    Cancel,
    Retry,
    Back,
}

struct Session {
    store: ProjectStore,
    project: Project,
    config: PipelineConfig,
    toolchain: Toolchain,
}

impl Session {
    fn pipeline(&self) -> Pipeline {
        Pipeline::new(
            self.store.clone(),
            self.toolchain.clone(),
            self.config.clone(),
        )
    }

    fn paths(&self) -> &ProjectPaths {
        self.store.paths()
    }
}

struct PendingProcess {
    url: String,
    toolchain: Toolchain,
}

struct PipelineResult {
    project: Project,
    config: PipelineConfig,
    finished: Option<FinishedSong>,
}

struct RunningJob {
    handle: JobHandle,
    kind: JobKind,
    models: Arc<Mutex<Option<Result<PipelineConfig, JobError>>>>,
    pipeline: Arc<Mutex<Option<PipelineResult>>>,
    last_message: String,
    started: Instant,
    stage_fraction: f32,
}

/// Message shown at the bottom of the window. Confirmations fade on their own;
/// failures stay until the reader closes them.
#[derive(Clone)]
struct Toast {
    message: String,
    at: Instant,
    sticky: bool,
}

impl Toast {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            at: Instant::now(),
            sticky: false,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            at: Instant::now(),
            sticky: true,
        }
    }
}

pub struct App {
    runner: JobRunner,
    settings: GuiSettings,
    session: Option<Session>,
    pending_process: Option<PendingProcess>,
    job: Option<RunningJob>,
    working_mode: JobKind,
    screen: Screen,
    url: String,
    home_error: Option<String>,
    error: Option<String>,
    finished: Option<FinishedSong>,
    models: setup::Models,
    toolchain: Toolchain,
    editor: Option<EditorState>,
    engine: AudioEngine,
    toast: Option<Toast>,
    show_settings: bool,
    destination_text: String,
    confirm_close: bool,
    allow_close: bool,
    progress_spring: theme::Spring,
    check_spring: theme::Spring,
    /// The step message on screen and when it appeared, for the cross-fade.
    step_shown: Option<(String, Instant)>,
    /// The message that is leaving, kept only while it fades out.
    step_fading: Option<(String, Instant)>,
    toast_spring: theme::Spring,
    last_frame: Instant,
    screen_started: Instant,
    last_screen: Screen,
    screen_age: f32,
    played_since_load: bool,
    bar_applied: Option<WindowBar>,
    native_tinted: Option<WindowBar>,
    maximized: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let settings = GuiSettings::load();
        let bar = settings.window_bar;
        Self {
            runner: JobRunner::new(),
            settings,
            session: None,
            pending_process: None,
            job: None,
            working_mode: JobKind::Pipeline,
            screen: Screen::Home,
            url: String::new(),
            home_error: None,
            error: None,
            finished: None,
            models: setup::models(),
            toolchain: Toolchain::detect(),
            editor: None,
            engine: AudioEngine::new(),
            toast: None,
            show_settings: false,
            destination_text: String::new(),
            confirm_close: false,
            allow_close: false,
            progress_spring: theme::Spring::new(0.0),
            check_spring: theme::Spring::new(0.0),
            step_shown: None,
            step_fading: None,
            toast_spring: theme::Spring::new(0.0),
            last_frame: Instant::now(),
            screen_started: Instant::now(),
            last_screen: Screen::Home,
            screen_age: 0.0,
            played_since_load: false,
            bar_applied: Some(bar),
            native_tinted: None,
            maximized: false,
        }
    }

    // -- processing ---------------------------------------------------------

    fn begin_processing(&mut self) {
        if self.job.is_some() {
            return;
        }
        self.home_error = None;
        self.error = None;

        let url = self.url.trim().to_owned();
        if url.is_empty() {
            self.home_error = Some("Pega el enlace del vídeo de YouTube.".into());
            self.screen = Screen::Home;
            return;
        }
        match Url::parse(&url) {
            Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {}
            _ => {
                self.home_error = Some(
                    "Ese enlace no parece válido. Copia la dirección completa del vídeo.".into(),
                );
                self.screen = Screen::Home;
                return;
            }
        }

        let toolchain = Toolchain::detect();
        if let Err(message) = preflight(&toolchain) {
            self.home_error = Some(message);
            self.screen = Screen::Home;
            return;
        }

        self.models = setup::models();
        if self.models.can_process() {
            let config =
                PipelineConfig::new(self.models.separation.clone(), self.models.whisper.clone())
                    .maybe_offline(setup::offline());
            self.start_project(url, toolchain, config);
        } else {
            self.start_models(url, toolchain);
        }
    }

    fn start_models(&mut self, url: String, toolchain: Toolchain) {
        self.pending_process = Some(PendingProcess { url, toolchain });
        self.working_mode = JobKind::Models;
        self.error = None;
        self.finished = None;
        self.progress_spring.reset(0.0);
        self.step_shown = None;
        self.step_fading = None;

        let models = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&models);
        let handle = self
            .runner
            .spawn(StageId::Ingest, move |context| async move {
                let outcome = setup::ensure_separation(&context).await.map(|separation| {
                    PipelineConfig::new(separation, setup::models().whisper)
                        .maybe_offline(setup::offline())
                });
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(outcome.clone());
                }
                outcome.map(|_| ())
            });

        match handle {
            Ok(handle) => {
                self.screen = Screen::Working;
                self.job = Some(RunningJob {
                    handle,
                    kind: JobKind::Models,
                    models,
                    pipeline: Arc::new(Mutex::new(None)),
                    last_message: String::new(),
                    started: Instant::now(),
                    stage_fraction: 0.0,
                });
            }
            Err(error) => {
                self.screen = Screen::Home;
                self.home_error = Some(format!(
                    "No se pudo iniciar la descarga de modelos: {error}"
                ));
            }
        }
    }

    fn start_project(&mut self, url: String, toolchain: Toolchain, config: PipelineConfig) {
        let root = setup::unique_project_root(&url);
        let source = SourceInfo {
            url,
            ..SourceInfo::default()
        };
        match ProjectStore::create(root, source, default_settings()) {
            Ok((store, project)) => {
                self.session = Some(Session {
                    store,
                    project,
                    config,
                    toolchain,
                });
                self.pending_process = None;
                self.editor = None;
                self.finished = None;
                self.start_pipeline_job();
            }
            Err(error) => {
                self.screen = Screen::Home;
                self.home_error = Some(format!(
                    "No se pudo preparar la carpeta del proyecto: {error}"
                ));
            }
        }
    }

    fn start_pipeline_job(&mut self) {
        if self.job.is_some() {
            return;
        }
        let Some(session) = &self.session else {
            self.screen = Screen::Home;
            self.home_error = Some("No hay ninguna canción preparada.".into());
            return;
        };

        let store = session.store.clone();
        let toolchain = session.toolchain.clone();
        let config = session.config.clone();
        let mut project = session.project.clone();
        let destination = self.settings.destination_dir.clone();

        let pipeline_slot = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&pipeline_slot);
        let handle = self
            .runner
            .spawn(StageId::Ingest, move |context| async move {
                let progress = StageProgress::from_job(&context);
                let mut config = config;
                let mut pipeline = Pipeline::new(store.clone(), toolchain.clone(), config.clone());
                let mut outcome = pipeline.run_pending(&mut project, &progress).await;

                // Online lyrics come first. Whisper is downloaded on demand so the
                // extra 547 MB are only spent when the lookup finds nothing usable.
                if matches!(&outcome, Err(PipelineError::WhisperModelMissing { .. })) {
                    context.report(0.0, "Descargando Whisper para transcribir…");
                    match setup::ensure_whisper(&context).await {
                        Ok(path) => {
                            config.whisper_model = path;
                            let _ = project.stage_mut(StageId::Transcribe).invalidate();
                            pipeline =
                                Pipeline::new(store.clone(), toolchain.clone(), config.clone());
                            outcome = pipeline.run_pending(&mut project, &progress).await;
                        }
                        Err(JobError::Cancelled) => outcome = Err(PipelineError::Cancelled),
                        Err(error) => {
                            outcome = Err(PipelineError::Stage {
                                stage: StageId::Transcribe,
                                message: error.to_string(),
                            });
                        }
                    }
                }

                let mut finished = None;
                if outcome.is_ok() {
                    if let Some(document) = read_song_document(&pipeline.paths().song()) {
                        let folder = pipeline.export_folder(&document);
                        let (copied_to, copy_error) = match &destination {
                            Some(root) => match setup::copy_song_folder(&folder, root) {
                                Ok(path) => (Some(path), None),
                                Err(error) => (None, Some(error.to_string())),
                            },
                            None => (None, None),
                        };
                        finished = Some(FinishedSong {
                            title: document.header.title.clone(),
                            artist: document.header.artist.clone(),
                            folder,
                            copied_to,
                            copy_error,
                        });
                    }
                }

                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(PipelineResult {
                        project,
                        config,
                        finished,
                    });
                }
                match outcome {
                    Ok(_) => Ok(()),
                    Err(PipelineError::Cancelled) => Err(JobError::Cancelled),
                    Err(error) => Err(JobError::Failed(error.to_string())),
                }
            });

        match handle {
            Ok(handle) => {
                self.error = None;
                self.working_mode = JobKind::Pipeline;
                self.screen = Screen::Working;
                self.progress_spring.reset(0.0);
                self.step_shown = None;
                self.step_fading = None;
                self.job = Some(RunningJob {
                    handle,
                    kind: JobKind::Pipeline,
                    models: Arc::new(Mutex::new(None)),
                    pipeline: pipeline_slot,
                    last_message: String::new(),
                    started: Instant::now(),
                    stage_fraction: 0.0,
                });
            }
            Err(error) => {
                self.error = Some(format!("No se pudo iniciar el proceso: {error}"));
                self.screen = Screen::Working;
            }
        }
    }

    fn cancel_job(&mut self) {
        if let Some(job) = &self.job {
            job.handle.cancel();
            self.notify("Cancelando…");
        }
    }

    /// Fetches the music video for a song that was processed without one. The
    /// export runs afterwards, so the `.txt` gains its `#VIDEO` line.
    fn start_video_job(&mut self) {
        if self.job.is_some() {
            return;
        }
        let Some(session) = &self.session else {
            self.notify_error("No hay ninguna canción preparada.");
            return;
        };
        let url = session.project.source.url.clone();
        if url.is_empty() {
            self.notify_error("Esta canción no guarda el enlace original.");
            return;
        }
        let source_dir = session.paths().source_dir();
        let target = session.paths().video();
        let toolchain = session.toolchain.clone();

        let handle = self
            .runner
            .spawn(StageId::Ingest, move |context| async move {
                context.report(0.05, "Descargando el vídeo…");
                let downloader = MediaDownloader::new(toolchain);
                let video = downloader
                    .download_video_required(&url, &source_dir, Some(context.cancel_flag()))
                    .await
                    .map_err(|error| match error {
                        DownloadError::Process(ProcessError::Cancelled) => JobError::Cancelled,
                        other => JobError::Failed(other.to_string()),
                    })?;
                context.report(0.9, "Guardando el vídeo…");
                if video != target {
                    std::fs::copy(&video, &target).map_err(|error| {
                        JobError::Failed(format!("No se pudo guardar el vídeo: {error}"))
                    })?;
                }
                context.report(1.0, "Vídeo listo");
                Ok(())
            });

        match handle {
            Ok(handle) => {
                self.error = None;
                self.working_mode = JobKind::Video;
                self.screen = Screen::Working;
                self.progress_spring.reset(0.0);
                self.step_shown = None;
                self.step_fading = None;
                self.job = Some(RunningJob {
                    handle,
                    kind: JobKind::Video,
                    models: Arc::new(Mutex::new(None)),
                    pipeline: Arc::new(Mutex::new(None)),
                    last_message: String::new(),
                    started: Instant::now(),
                    stage_fraction: 0.0,
                });
            }
            Err(error) => self.notify_error(format!("No se pudo iniciar la descarga: {error}")),
        }
    }

    fn retry(&mut self) {
        match self.working_mode {
            JobKind::Models => self.begin_processing(),
            JobKind::Pipeline => {
                self.error = None;
                self.start_pipeline_job();
            }
            JobKind::Video => {
                self.error = None;
                self.start_video_job();
            }
        }
    }

    fn poll_job(&mut self) {
        let mut finished = None;
        let mut refresh = false;
        let stage_before = self.active_stage();

        if let Some(job) = self.job.as_mut() {
            loop {
                match job.handle.try_recv() {
                    Some(JobEvent::Progress {
                        fraction, message, ..
                    }) => {
                        job.stage_fraction = fraction;
                        job.last_message = message;
                        // The pipeline reports a stage-level fraction; refresh
                        // the project at its boundaries to learn which stage is
                        // actually running.
                        if fraction <= 0.01 || fraction >= 1.0 {
                            refresh = true;
                        }
                    }
                    Some(JobEvent::Finished(outcome)) => {
                        let models = job.models.lock().ok().and_then(|mut slot| slot.take());
                        let pipeline = job.pipeline.lock().ok().and_then(|mut slot| slot.take());
                        finished = Some((job.kind, outcome, models, pipeline));
                        break;
                    }
                    None => break,
                }
            }
        }

        if refresh {
            self.refresh_project();
        }
        // Park the spring where the new stage starts, so the bar continues from
        // the same point. When the pipeline runs out of stages, let it finish.
        if stage_before != self.active_stage() {
            if let Some(index) = self.active_stage().and_then(stage_index) {
                self.progress_spring
                    .reset(index as f32 / StageId::ALL.len() as f32);
            }
        }
        if finished.is_some() {
            self.job = None;
        }
        if let Some((kind, outcome, models, pipeline)) = finished {
            self.finish_job(kind, outcome, models, pipeline);
        }
    }

    fn active_stage(&self) -> Option<StageId> {
        let job = self.job.as_ref()?;
        if job.kind != JobKind::Pipeline {
            return None;
        }
        let session = self.session.as_ref()?;
        StageId::ALL
            .into_iter()
            .find(|stage| session.project.stage(*stage).status == StageStatus::Running)
            .or_else(|| session.project.next_stage())
    }

    fn refresh_project(&mut self) {
        let Some(session) = &mut self.session else {
            return;
        };
        if let Ok(project) = session.store.load() {
            session.project = project;
        }
    }

    fn finish_job(
        &mut self,
        kind: JobKind,
        outcome: Result<(), JobError>,
        models: Option<Result<PipelineConfig, JobError>>,
        pipeline: Option<PipelineResult>,
    ) {
        match kind {
            JobKind::Models => match outcome {
                Ok(()) => {
                    if let Some(Ok(config)) = models {
                        self.models = setup::models();
                        if let Some(pending) = self.pending_process.take() {
                            self.start_project(pending.url, pending.toolchain, config);
                        }
                    } else {
                        self.error = Some("No se pudieron preparar los modelos de IA.".into());
                    }
                }
                Err(JobError::Cancelled) => {
                    self.pending_process = None;
                    self.screen = Screen::Home;
                    self.notify("Preparación cancelada");
                }
                Err(JobError::Busy) => {
                    self.error = Some("Otra tarea sigue en marcha.".into());
                }
                Err(JobError::Failed(message)) => self.error = Some(message),
            },
            JobKind::Pipeline => {
                if let Some(result) = pipeline {
                    if let Some(session) = &mut self.session {
                        session.project = result.project;
                        session.config = result.config;
                    }
                    match outcome {
                        Ok(()) => {
                            self.finished = result.finished;
                            self.error = None;
                            self.screen = Screen::Done;
                            self.load_editor();
                            self.load_audio();
                            if let Some(error) = self
                                .finished
                                .as_ref()
                                .and_then(|song| song.copy_error.clone())
                            {
                                self.notify_error(format!("No se pudo copiar al destino: {error}"));
                            }
                        }
                        Err(JobError::Cancelled) => {
                            self.screen = Screen::Home;
                            self.notify("Procesamiento cancelado");
                        }
                        Err(JobError::Busy) => {
                            self.error = Some("Otra tarea sigue en marcha.".into());
                        }
                        Err(JobError::Failed(message)) => self.error = Some(message),
                    }
                } else {
                    self.error = Some("El proceso terminó sin resultado.".into());
                }
            }
            JobKind::Video => {
                self.screen = Screen::Done;
                match outcome {
                    Ok(()) => {
                        if let Some(session) = &mut self.session {
                            if !session.project.settings.download_video {
                                session.project.settings.download_video = true;
                                let _ = session.store.save(&session.project);
                            }
                        }
                        if self.export() {
                            self.notify("Vídeo añadido y exportado");
                        }
                    }
                    Err(JobError::Cancelled) => self.notify("Descarga del vídeo cancelada"),
                    Err(JobError::Busy) => {
                        self.notify_error("Otra tarea sigue en marcha.");
                    }
                    Err(JobError::Failed(message)) => {
                        self.notify_error(format!("No se pudo añadir el vídeo: {message}"));
                    }
                }
            }
        }
    }

    fn progress_target(&self) -> f32 {
        let Some(job) = &self.job else {
            return if self.screen == Screen::Done {
                1.0
            } else {
                0.0
            };
        };
        let fraction = job.stage_fraction.clamp(0.0, 1.0);
        match job.kind {
            JobKind::Pipeline => {
                (self.stage_offset() + fraction / StageId::ALL.len() as f32).min(1.0)
            }
            JobKind::Models | JobKind::Video => fraction,
        }
    }

    /// Share of the pipeline that is already behind the running stage.
    fn stage_offset(&self) -> f32 {
        let index = self
            .active_stage()
            .and_then(stage_index)
            .unwrap_or(StageId::ALL.len() - 1);
        index as f32 / StageId::ALL.len() as f32
    }

    fn new_song(&mut self) {
        if self.job.is_some() {
            return;
        }
        self.engine.pause();
        self.engine.seek(0.0);
        self.session = None;
        self.pending_process = None;
        self.editor = None;
        self.finished = None;
        self.url.clear();
        self.home_error = None;
        self.error = None;
        self.screen = Screen::Home;
        self.progress_spring.reset(0.0);
        self.check_spring.reset(0.0);
    }

    // -- project artifacts --------------------------------------------------

    fn load_editor(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        let path = session.paths().song();
        self.editor = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SongDoc>(&bytes).ok())
            .map(EditorState::new);
    }

    fn load_audio(&mut self) {
        let path = {
            let Some(session) = &self.session else {
                return;
            };
            let mix = session.paths().analysis_wav();
            if !mix.exists() {
                return;
            }
            mix
        };
        if let Err(error) = self.engine.load(&path) {
            self.notify_error(playback_message(error));
        }
    }

    fn save_document(&mut self, quiet: bool) {
        let (message, failed) = {
            let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
                return;
            };
            match serde_json::to_vec_pretty(editor.document()) {
                Ok(bytes) => match atomic_write(&session.paths().song(), &bytes) {
                    Ok(()) => {
                        editor.mark_saved();
                        ("Guardado".to_owned(), false)
                    }
                    Err(error) => (format!("No se pudo guardar: {error}"), true),
                },
                Err(error) => (format!("No se pudo serializar la canción: {error}"), true),
            }
        };
        if failed {
            self.notify_error(message);
        } else if !quiet {
            self.notify(message);
        }
    }

    /// Writes the UltraStar folder and copies it to the destination. Returns
    /// whether the export succeeded, so actions that chain into it can tell.
    fn export(&mut self) -> bool {
        let validation = {
            let Some(editor) = &mut self.editor else {
                self.notify_error("No hay canción para exportar");
                return false;
            };
            match editor.check_export() {
                ExportCheck::Ready => None,
                _ => Some(
                    editor
                        .last_message
                        .clone()
                        .unwrap_or_else(|| "La validación impide exportar.".into()),
                ),
            }
        };
        if let Some(message) = validation {
            self.notify_error(message);
            return false;
        }

        let destination = self.settings.destination_dir.clone();
        let outcome = {
            let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
                return false;
            };
            let pipeline = session.pipeline();
            match pipeline.export_document(editor.document()) {
                Ok(outcome) => Ok((editor.document().clone(), outcome)),
                Err(error) => {
                    let message = error.to_string();
                    editor.adopt_export_failure(message.clone());
                    Err(message)
                }
            }
        };

        match outcome {
            Ok((document, outcome)) => {
                let (copied_to, copy_error) = match &destination {
                    Some(root) => match setup::copy_song_folder(&outcome.folder, root) {
                        Ok(path) => (Some(path), None),
                        Err(error) => (None, Some(error.to_string())),
                    },
                    None => (None, None),
                };
                let detail = copied_to
                    .as_ref()
                    .map(|path| format!("Exportado y copiado a {}", path.display()))
                    .unwrap_or_else(|| format!("Exportado a {}", outcome.folder.display()));
                self.settings.remember(RecentSong {
                    label: song_label_parts(&document.header.artist, &document.header.title),
                    folder: outcome.folder.clone(),
                });
                self.settings.save();
                self.finished = Some(FinishedSong {
                    title: document.header.title.clone(),
                    artist: document.header.artist.clone(),
                    folder: outcome.folder,
                    copied_to,
                    copy_error: copy_error.clone(),
                });
                if let Some(error) = copy_error {
                    self.notify_error(format!("{detail} (pero no se pudo copiar: {error})"));
                } else {
                    self.notify(detail);
                }
                // The exported `.txt` and the editable document must agree;
                // a failed write leaves a sticky error on top of the toast.
                self.save_document(true);
                true
            }
            Err(message) => {
                self.notify_error(message);
                false
            }
        }
    }

    fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast::info(message));
    }

    /// A failure, which stays on screen until it is dismissed.
    fn notify_error(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast::error(message));
    }

    fn open_settings(&mut self) {
        self.models = setup::models();
        self.destination_text = self
            .settings
            .destination_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.show_settings = true;
    }

    fn apply_destination(&mut self) {
        let text = self.destination_text.trim();
        let value = if text.is_empty() {
            None
        } else {
            Some(PathBuf::from(text))
        };
        if self.settings.destination_dir != value {
            self.settings.destination_dir = value;
            self.settings.save();
        }
    }

    // -- frame loop ---------------------------------------------------------

    fn tick(&mut self, ctx: &egui::Context) {
        let elapsed = self.last_frame.elapsed().as_secs_f32().min(0.25);
        self.last_frame = Instant::now();

        self.engine.poll();
        if let Some(target) = self.engine.clock_mut().advance(elapsed as f64) {
            self.engine.seek(target);
        }
        if self.engine.clock().is_playing() {
            self.played_since_load = true;
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        self.poll_job();

        if self.screen != self.last_screen {
            self.last_screen = self.screen;
            self.screen_started = Instant::now();
            self.screen_age = 0.0;
            ctx.request_repaint();
        } else {
            self.screen_age = self.screen_started.elapsed().as_secs_f32();
        }

        let target = self.progress_target();
        self.progress_spring.update(target, elapsed, 0.35, 1.0);
        self.check_spring.update(
            if self.screen == Screen::Done {
                1.0
            } else {
                0.0
            },
            elapsed,
            0.32,
            0.72,
        );

        let toast_target = self
            .toast
            .as_ref()
            .map(|toast| toast.sticky || toast.at.elapsed().as_secs_f32() < TOAST_SECONDS)
            .unwrap_or(false);
        self.toast_spring
            .update(if toast_target { 1.0 } else { 0.0 }, elapsed, 0.26, 1.0);

        // Edits save themselves two seconds after the last key press, without
        // a toast for every stroke.
        if self.screen == Screen::Preview {
            let due = self
                .editor
                .as_ref()
                .is_some_and(|editor| editor.is_dirty() && editor.should_autosave(Instant::now()));
            if due {
                self.save_document(true);
            }
        }

        if self.job.is_some() || self.screen_age < 0.6 || self.toast.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    /// Eased progress of the current screen's entrance animation.
    fn intro(&self) -> f32 {
        theme::ease_out_cubic((self.screen_age / 0.3).clamp(0.0, 1.0))
    }

    /// Vertical offset that lets the content settle into place while fading in.
    fn intro_lift(&self) -> f32 {
        (1.0 - theme::ease_out_quint((self.screen_age / 0.36).clamp(0.0, 1.0))) * 12.0
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        self.apply_window_bar(&ctx, frame);
        if self.screen == Screen::Preview {
            self.handle_shortcuts(&ctx);
        }

        let dirty = self.editor.as_ref().is_some_and(EditorState::is_dirty);
        if ctx.input(|input| input.viewport().close_requested())
            && !self.allow_close
            && (dirty || self.job.is_some())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }

        self.chrome_bar(ui, &ctx);
        self.status_bar(ui);

        let intro = self.intro();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ui, |ui| {
                ui.multiply_opacity(intro);
                theme::background(ui);
                match self.screen {
                    Screen::Home => self.home_screen(ui),
                    Screen::Working => self.working_screen(ui),
                    Screen::Done => self.done_screen(ui),
                    Screen::Preview => self.preview_screen(ui),
                }
            });

        if matches!(self.screen, Screen::Home | Screen::Done) {
            self.gear_area(&ctx);
        }
        self.toast_ui(&ctx);
        self.settings_modal(&ctx);
        self.close_modal(&ctx);
    }
}

// ---------------------------------------------------------------------------
// Window chrome
// ---------------------------------------------------------------------------

impl App {
    fn apply_window_bar(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let mode = self.settings.window_bar;
        if self.bar_applied != Some(mode) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(mode.is_native()));
            self.bar_applied = Some(mode);
            self.native_tinted = None;
        }
        if mode.is_native() && self.native_tinted != Some(mode) {
            native_bar::tint(frame);
            self.native_tinted = Some(mode);
        }
    }

    fn chrome_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mode = self.settings.window_bar;
        if mode.is_native() {
            return;
        }
        let height = mode.height();
        egui::Panel::top(Id::unique("window_chrome"))
            .exact_size(height)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let controls = 46.0 * 3.0;
                let drag_rect = Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right() - controls, rect.bottom()),
                );
                let drag = ui.interact(
                    drag_rect,
                    Id::unique("chrome_drag"),
                    Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    self.toggle_maximized(ctx);
                }

                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.spacing_mut().interact_size.y = 0.0;
                ui.horizontal_centered(|ui| {
                    ui.add_space(theme::GAP_M);
                    let (mark, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    crate::icons::paint(Icon::MusicNote, ui.painter(), mark, theme::ACCENT_TEXT);
                    ui.add_space(theme::GAP_S);
                    if mode == WindowBar::Minimal {
                        ui.label(theme::micro("USDX"));
                    } else {
                        ui.label(theme::micro("UltraStar // Song Generator"));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::chrome_button(ui, Icon::Close, true, height).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let icon = if self.maximized {
                            Icon::Restore
                        } else {
                            Icon::Maximize
                        };
                        if theme::chrome_button(ui, icon, false, height).clicked() {
                            self.toggle_maximized(ctx);
                        }
                        if theme::chrome_button(ui, Icon::Minimize, false, height).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    fn toggle_maximized(&mut self, ctx: &egui::Context) {
        self.maximized = !self.maximized;
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized));
        ctx.request_repaint();
    }
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------

impl App {
    fn home_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut process = false;
        let intro = self.intro();
        let lift = self.intro_lift();
        let width = flow_width(&ctx, HOME_WIDTH);
        egui::Area::new(Id::unique("home"))
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, lift))
            .show(&ctx, |ui| {
                ui.multiply_opacity(intro);
                ui.set_width(width);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                    theme::logo(ui.painter(), rect);
                    ui.add_space(theme::GAP_L);
                    ui.label(theme::display(theme::T_HERO, "De enlace a canción jugable"));
                    ui.add_space(theme::GAP_S);
                    ui.label(theme::secondary(
                        "Pega un enlace de YouTube y pulsa Generar. La letra y el audio van solos.",
                    ));
                    ui.add_space(theme::GAP_XL);
                });

                let mut submit = false;
                ui.horizontal(|ui| {
                    let button_width = 116.0;
                    let field_width =
                        (ui.available_width() - button_width - theme::GAP_S).max(160.0);
                    let response = theme::text_field_sized(
                        ui,
                        &mut self.url,
                        "Pega el enlace del vídeo de YouTube…",
                        field_width,
                        theme::T_TITLE,
                        theme::GAP_M,
                    );
                    if self.url.is_empty()
                        && !self.show_settings
                        && ui.memory(|memory| memory.focused().is_none())
                    {
                        response.request_focus();
                    }
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        submit = true;
                    }
                    ui.add_space(theme::GAP_S);
                    let height = response.rect.height();
                    if theme::button_sized_with_icon(
                        ui,
                        "Generar",
                        Some(Icon::ArrowRight),
                        ButtonKind::Primary,
                        Vec2::new(button_width, height),
                        true,
                    )
                    .clicked()
                    {
                        submit = true;
                    }
                });
                if let Some(error) = &self.home_error {
                    ui.add_space(theme::GAP_M);
                    theme::error_row(ui, error);
                }
                if !self.models.can_process() {
                    ui.add_space(theme::GAP_L);
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Label::new(theme::caption(
                                "La primera vez se descarga el modelo de voz (unos 300 MB).",
                            ))
                            .wrap(),
                        );
                    });
                }
                if submit {
                    process = true;
                }
            });

        if process {
            self.begin_processing();
        }
    }

    fn working_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let intro = self.intro();
        let lift = self.intro_lift();
        let width = flow_width(&ctx, FLOW_WIDTH);
        let error = self.error.clone();
        let mut action = None;
        egui::Area::new(Id::unique("working"))
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, lift))
            .show(&ctx, |ui| {
                ui.multiply_opacity(intro);
                ui.set_width(width);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                action = match &error {
                    Some(error) => self.flow_error(ui, error),
                    None => self.flow_running(ui),
                };
            });

        match action {
            Some(FlowAction::Cancel) => self.cancel_job(),
            Some(FlowAction::Retry) => self.retry(),
            Some(FlowAction::Back) => {
                if self.working_mode == JobKind::Video {
                    self.error = None;
                    self.screen = Screen::Done;
                } else {
                    self.session = None;
                    self.pending_process = None;
                    self.error = None;
                    self.progress_spring.reset(0.0);
                    self.screen = Screen::Home;
                }
            }
            None => {}
        }
    }

    /// The bar is the anchor of this screen: it sits on the window's vertical
    /// centre in every state, the numbers just above it and the steps below, so
    /// a new step never pushes it around.
    fn flow_running(&mut self, ui: &mut egui::Ui) -> Option<FlowAction> {
        let time = ui.input(|input| input.time);
        let fraction = self.progress_spring.value().clamp(0.0, 1.0);
        let percent = (fraction * 100.0).round() as i32;
        let elapsed = self
            .job
            .as_ref()
            .map(|job| job.started.elapsed().as_secs())
            .unwrap_or(0);

        // Everything below the bar is mirrored above it, so the bar lands on
        // the window's centre line whatever the steps are doing.
        let steps_h = STEP_SLOTS as f32 * STEP_ROW_H;
        let row_h = theme::BTN_H_GHOST;
        ui.add_space((theme::GAP_S + steps_h - row_h - theme::GAP_M).max(0.0));

        let width = ui.available_width();
        let mut cancel = false;
        let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::hover());
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.label(
            theme::mono(
                format!("{percent}% · {}", format_elapsed(elapsed)),
                theme::T_CAPTION,
            )
            .color(theme::TEXT_3),
        );
        row.with_layout(Layout::right_to_left(Align::Center), |ui| {
            cancel = theme::ghost_button(ui, "Cancelar").clicked();
        });

        ui.add_space(theme::GAP_M);
        theme::progress_bar(ui, None, fraction, time as f32);
        ui.add_space(theme::GAP_S);

        let (steps_rect, _) = ui.allocate_exact_size(Vec2::new(width, steps_h), Sense::hover());
        let mut steps = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(steps_rect)
                .layout(Layout::top_down(Align::Center)),
        );
        steps.spacing_mut().item_spacing = Vec2::ZERO;
        self.step_message(&mut steps, true);
        cancel.then_some(FlowAction::Cancel)
    }

    /// Label of the step that matters right now, and whether it failed.
    fn step_text(&self) -> (String, bool) {
        let failed = self.error.is_some();
        let label = match self.working_mode {
            JobKind::Pipeline => {
                let statuses: [StageStatus; 6] = std::array::from_fn(|index| {
                    self.session
                        .as_ref()
                        .map(|session| session.project.stage(StageId::ALL[index]).status)
                        .unwrap_or_default()
                });
                let last_known = statuses
                    .iter()
                    .rposition(|status| *status != StageStatus::Pending)
                    .unwrap_or(0);
                let index = self
                    .active_stage()
                    .and_then(stage_index)
                    .unwrap_or(last_known);
                stage_step_label(StageId::ALL[index]).to_owned()
            }
            JobKind::Models => "descargando los modelos de IA".to_owned(),
            JobKind::Video => "descargando el vídeo de YouTube".to_owned(),
        };
        if failed {
            (format!("{label}."), true)
        } else {
            (label, false)
        }
    }

    /// Prints the running step like a console line: the old message leaves
    /// upwards while the new one is typed in, and a highlight sweeps across it
    /// once it is complete.
    fn step_message(&mut self, ui: &mut egui::Ui, live: bool) {
        let (label, failed) = self.step_text();
        let now = Instant::now();
        match &self.step_shown {
            Some((shown, _)) if *shown == label => {}
            Some((shown, _)) => {
                self.step_fading = Some((shown.clone(), now));
                self.step_shown = Some((label, now));
            }
            None => self.step_shown = Some((label, now)),
        }
        let Some((text, since)) = self.step_shown.clone() else {
            return;
        };

        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, STEP_ROW_H), Sense::hover());
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let font = FontId::monospace(theme::T_BODY);
        let elapsed = since.elapsed().as_secs_f32();
        let mut repaint = false;

        if let Some((previous, left_at)) = self.step_fading.clone() {
            let gone =
                theme::ease_out_cubic((left_at.elapsed().as_secs_f32() / 0.18).clamp(0.0, 1.0));
            if gone < 1.0 {
                repaint = true;
                let color = theme::TEXT_3.gamma_multiply(1.0 - gone);
                let galley = painter.layout_no_wrap(previous, font.clone(), color);
                painter.galley(
                    Pos2::new(
                        rect.center().x - galley.size().x * 0.5,
                        rect.center().y - galley.size().y * 0.5 - 3.0 * gone,
                    ),
                    galley,
                    color,
                );
            } else {
                self.step_fading = None;
            }
        }

        // Resting text sits a step darker than white, so the single pass of
        // light over the finished line has somewhere to go.
        let color = if failed {
            theme::DANGER
        } else {
            theme::TEXT.gamma_multiply(0.74)
        };
        let full = painter.layout_no_wrap(text.clone(), font.clone(), color);
        let left = rect.center().x - full.size().x * 0.5;
        let top = rect.center().y - full.size().y * 0.5;
        let total = text.chars().count();
        let typed = (elapsed * TYPE_CPS).floor() as usize;
        let typed = typed.min(total);
        if typed > 0 {
            let visible: String = text.chars().take(typed).collect();
            painter.text(
                Pos2::new(left, top),
                Align2::LEFT_TOP,
                visible,
                font.clone(),
                color,
            );
        }

        if typed < total {
            repaint = true;
        } else if live && !failed {
            // A band of light crosses the finished line every few seconds, like
            // a console echo, so a long stage never looks frozen.
            let done_at = total as f32 / TYPE_CPS;
            let since_done = (elapsed - done_at).max(0.0);
            let phase = since_done % SWEEP_CYCLE;
            if phase < SWEEP_SECONDS {
                let progress = theme::ease_out_cubic(phase / SWEEP_SECONDS);
                let band = (full.size().x * 0.55).max(60.0);
                let travel = -band + progress * (full.size().x + band * 2.0);
                let glow_rect = Rect::from_min_size(
                    Pos2::new(left + travel, rect.top()),
                    Vec2::new(band, rect.height()),
                );
                let glow = painter.with_clip_rect(glow_rect.intersect(rect));
                glow.text(
                    Pos2::new(left, top),
                    Align2::LEFT_TOP,
                    text.clone(),
                    font,
                    Color32::WHITE,
                );
            }
            repaint = true;
        }

        if repaint {
            ui.ctx().request_repaint();
        }
    }

    fn flow_error(&mut self, ui: &mut egui::Ui, error: &str) -> Option<FlowAction> {
        self.step_message(ui, false);
        ui.add_space(theme::GAP_S);
        ui.label(theme::micro("// error"));
        ui.add_space(theme::GAP_S);
        let max_error = (ui.ctx().viewport_rect().height() * 0.32).max(60.0);
        egui::ScrollArea::vertical()
            .max_height(max_error)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(error)
                            .monospace()
                            .size(theme::T_CAPTION)
                            .color(theme::TEXT_2),
                    )
                    .wrap()
                    .selectable(true),
                );
            });
        ui.add_space(theme::GAP_L);
        let width = ((ui.available_width() - theme::GAP_S) / 2.0).max(120.0);
        let mut action = None;
        ui.horizontal(|ui| {
            if theme::button_sized(
                ui,
                "Reintentar",
                ButtonKind::Primary,
                Vec2::new(width, theme::BTN_H),
                true,
            )
            .clicked()
            {
                action = Some(FlowAction::Retry);
            }
            ui.add_space(theme::GAP_S);
            let back = if self.working_mode == JobKind::Video {
                "Volver a la canción"
            } else {
                "Volver al inicio"
            };
            if theme::button_sized(
                ui,
                back,
                ButtonKind::Secondary,
                Vec2::new(width, theme::BTN_H),
                true,
            )
            .clicked()
            {
                action = Some(FlowAction::Back);
            }
        });
        action
    }

    fn done_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let Some(song) = self.finished.clone() else {
            self.screen = Screen::Home;
            return;
        };
        let mut open = false;
        let mut review = false;
        let mut again = false;
        let mut add_video = false;
        let has_video = song.display_folder().join(EXPORTED_VIDEO).exists();
        let intro = self.intro();
        let lift = self.intro_lift();
        let width = flow_width(&ctx, FLOW_WIDTH);

        egui::Area::new(Id::unique("done"))
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, lift))
            .show(&ctx, |ui| {
                ui.multiply_opacity(intro);
                ui.set_width(width);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.vertical_centered(|ui| {
                    let progress = self.check_spring.value().clamp(0.0, 1.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(88.0), Sense::hover());
                    if progress > 0.01 {
                        let painter = ui.painter();
                        painter.rect_filled(
                            Rect::from_center_size(rect.center(), Vec2::splat(58.0 * progress)),
                            0,
                            theme::SUCCESS.gamma_multiply(0.14),
                        );
                        painter.rect_filled(
                            Rect::from_center_size(rect.center(), Vec2::splat(44.0 * progress)),
                            0,
                            theme::SUCCESS,
                        );
                        theme::check_mark_progress(
                            painter,
                            rect.center(),
                            13.0 * progress,
                            Color32::WHITE,
                            ((progress - 0.2) / 0.65).clamp(0.0, 1.0),
                        );
                    }
                    ui.add_space(theme::GAP_L);
                    ui.label(theme::display(theme::T_HERO, "Canción lista"));
                    ui.add_space(theme::GAP_XS);
                    ui.label(theme::secondary(song_label_parts(
                        &song.artist,
                        &song.title,
                    )));
                });

                ui.add_space(theme::GAP_L);
                ui.vertical_centered(|ui| {
                    ui.label(theme::caption(if song.copied_to.is_some() {
                        "copiada en"
                    } else {
                        "guardada en"
                    }));
                    ui.add_space(theme::GAP_XS);
                    ui.add(
                        egui::Label::new(
                            RichText::new(song.display_folder().display().to_string())
                                .size(theme::T_CAPTION)
                                .monospace()
                                .color(theme::TEXT_3),
                        )
                        .truncate(),
                    );
                });

                if let Some(error) = &song.copy_error {
                    ui.add_space(theme::GAP_S);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("No se pudo copiar al destino: {error}"))
                                .size(theme::T_CAPTION)
                                .color(theme::WARNING),
                        )
                        .wrap(),
                    );
                }

                ui.add_space(theme::GAP_XL);
                if theme::button_with_icon(
                    ui,
                    "Abrir carpeta",
                    Some(Icon::Folder),
                    ButtonKind::Primary,
                    true,
                )
                .clicked()
                {
                    open = true;
                }
                ui.add_space(theme::GAP_S);
                if theme::button_with_icon(
                    ui,
                    "Vista previa",
                    Some(Icon::Play),
                    ButtonKind::Secondary,
                    true,
                )
                .clicked()
                {
                    review = true;
                }
                if !has_video {
                    ui.add_space(theme::GAP_S);
                    if theme::button_with_icon(
                        ui,
                        "Añadir vídeo",
                        Some(Icon::Download),
                        ButtonKind::Secondary,
                        true,
                    )
                    .on_hover_text("Lo descarga del enlace original y vuelve a exportar")
                    .clicked()
                    {
                        add_video = true;
                    }
                }
                ui.add_space(theme::GAP_M);
                ui.vertical_centered(|ui| {
                    if theme::ghost_button(ui, "Procesar otra canción").clicked() {
                        again = true;
                    }
                });
            });

        if open {
            if let Err(error) = setup::open_folder(song.display_folder()) {
                self.notify_error(format!("No se pudo abrir la carpeta: {error}"));
            }
        }
        if review {
            if let Some(start) = self.editor.as_ref().and_then(first_note_start) {
                self.engine.seek(start);
            }
            self.screen = Screen::Preview;
        }
        if again {
            self.new_song();
        }
        if add_video {
            self.start_video_job();
        }
    }

    fn gear_area(&mut self, ctx: &egui::Context) {
        let intro = self.intro();
        let bar = self.settings.window_bar;
        egui::Area::new(Id::unique("gear"))
            .anchor(
                Align2::RIGHT_TOP,
                Vec2::new(-theme::GAP_XL, bar.height() + theme::GAP_M),
            )
            .show(ctx, |ui| {
                ui.multiply_opacity(intro);
                if theme::icon_button(ui, Icon::Settings, "Ajustes").clicked() {
                    self.open_settings();
                }
            });
    }

    /// Bottom bar: what the app needs to work, and where songs land.
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom(Id::unique("status_bar"))
            .exact_size(STATUS_BAR_H)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 0.0;
                ui.horizontal_centered(|ui| {
                    ui.add_space(theme::GAP_XL);
                    tool_status(ui, &self.toolchain);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(theme::GAP_XL);
                        let destination = self
                            .settings
                            .destination_dir
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Sin destino".into());
                        ui.label(theme::mono(destination, theme::T_MICRO).color(theme::TEXT_3));
                    });
                });
            });
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some(toast) = self.toast.clone() else {
            return;
        };
        let age = toast.at.elapsed().as_secs_f32();
        if !toast.sticky && age >= TOAST_SECONDS + 0.4 {
            self.toast = None;
            return;
        }
        let rise = self.toast_spring.value().clamp(0.0, 1.0);
        if !toast.sticky && age > TOAST_SECONDS && rise <= 0.01 {
            self.toast = None;
            return;
        }
        let mut dismiss = false;
        egui::Area::new(Id::unique("toast"))
            .order(egui::Order::Foreground)
            .anchor(
                Align2::CENTER_BOTTOM,
                Vec2::new(0.0, -(STATUS_BAR_H + 24.0) + (1.0 - rise) * 14.0),
            )
            .show(ctx, |ui| {
                ui.multiply_opacity(rise);
                let stroke = if toast.sticky {
                    Stroke::new(1.0, theme::DANGER.gamma_multiply(0.55))
                } else {
                    Stroke::new(1.0, theme::BORDER)
                };
                egui::Frame::NONE
                    .fill(theme::SURFACE_2)
                    .stroke(stroke)
                    .corner_radius(theme::RADIUS)
                    .inner_margin(Margin::symmetric(theme::GAP_L as i8, theme::GAP_S as i8))
                    .shadow(theme::shadow())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&toast.message)
                                    .size(theme::T_BODY)
                                    .color(theme::TEXT),
                            );
                            if toast.sticky {
                                ui.add_space(theme::GAP_XS);
                                if theme::icon_button_sized(
                                    ui,
                                    Icon::Close,
                                    "Cerrar el aviso",
                                    Vec2::splat(26.0),
                                )
                                .clicked()
                                {
                                    dismiss = true;
                                }
                            }
                        });
                    });
            });
        if dismiss {
            self.toast = None;
        }
    }

    fn settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            ctx.animate_bool_with_time(Id::unique("settings_enter"), false, 0.0);
            return;
        }
        let frame = egui::Frame::NONE
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(theme::RADIUS)
            .inner_margin(Margin::same(theme::GAP_XL as i8))
            .shadow(theme::modal_shadow());
        let enter = ctx.animate_bool_with_time(Id::unique("settings_enter"), true, 0.18);

        let mut close = false;
        let modal = egui::Modal::new(Id::unique("settings_modal"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.multiply_opacity(enter);
                ui.set_width(500.0);
                // The settings body can outgrow a short screen: it scrolls
                // between the title and the footer, which stays in place.
                let body_max =
                    (ctx.viewport_rect().height() - 160.0).max(320.0);
                egui::ScrollArea::vertical()
                    .max_height(body_max)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                ui.label(theme::micro("// sistema"));
                ui.add_space(theme::GAP_S);
                ui.label(theme::display(theme::T_DISPLAY, "Ajustes"));
                ui.add_space(theme::GAP_XS);
                ui.label(theme::secondary("Solo lo imprescindible. El resto se decide solo."));
                ui.add_space(theme::GAP_XL);

                ui.label(theme::strong("Carpeta de destino"));
                ui.add_space(theme::GAP_XS);
                ui.add(
                    egui::Label::new(theme::caption(
                        "Cada canción terminada se copia aquí. Normalmente es la carpeta songs de UltraStar.",
                    ))
                    .wrap(),
                );
                ui.add_space(theme::GAP_S);
                let width = (ui.available_width() - 112.0).max(180.0);
                ui.horizontal(|ui| {
                    theme::text_field_sized(
                        ui,
                        &mut self.destination_text,
                        "Sin definir",
                        width,
                        theme::T_BODY,
                        theme::GAP_S,
                    );
                    if theme::button_sized_with_icon(
                        ui,
                        "Elegir",
                        Some(Icon::Folder),
                        ButtonKind::Secondary,
                        Vec2::new(104.0, theme::BTN_H_SM),
                        true,
                    )
                    .clicked()
                    {
                        if let Some(directory) = rfd::FileDialog::new()
                            .set_title("Elegir carpeta de destino")
                            .pick_folder()
                        {
                            self.destination_text = directory.display().to_string();
                            self.apply_destination();
                        }
                    }
                });
                if self.settings.destination_dir.is_some() {
                    ui.add_space(theme::GAP_XS);
                    if theme::ghost_button(ui, "Quitar destino").clicked() {
                        self.destination_text.clear();
                        self.apply_destination();
                    }
                }

                ui.add_space(theme::GAP_L);
                theme::divider(ui);
                ui.add_space(theme::GAP_M);
                ui.label(theme::strong("Barra de ventana"));
                ui.add_space(theme::GAP_XS);
                ui.label(theme::caption(
                    "Sistema usa la barra del sistema con los colores de la app. Integrada y Mínima se dibujan dentro de la ventana.",
                ));
                ui.add_space(theme::GAP_S);
                let options = [
                    ("Sistema", WindowBar::Native),
                    ("Integrada", WindowBar::Integrated),
                    ("Mínima", WindowBar::Minimal),
                ];
                if let Some(mode) = theme::segmented(ui, &options, self.settings.window_bar) {
                    self.settings.window_bar = mode;
                    self.settings.save();
                    self.native_tinted = None;
                }

                ui.add_space(theme::GAP_XL);
                theme::divider(ui);
                ui.add_space(theme::GAP_M);
                ui.label(theme::strong("Letra y modelos"));
                ui.add_space(theme::GAP_XS);
                ui.label(theme::caption(
                    "La letra se busca en línea. Whisper solo se descarga si no se encuentra.",
                ));
                ui.add_space(theme::GAP_M);
                model_row(
                    ui,
                    "Separación de voces",
                    "HTDemucs · 302 MB · siempre necesario",
                    self.models.separation_ready,
                );
                ui.add_space(theme::GAP_XS);
                model_row(
                    ui,
                    "Transcripción (respaldo)",
                    "Whisper large-v3-turbo · 547 MB · solo si falta la letra",
                    self.models.whisper_ready,
                );
                    });

                ui.add_space(theme::GAP_XL);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::button_sized(
                        ui,
                        "Listo",
                        ButtonKind::Primary,
                        Vec2::new(160.0, theme::BTN_H_SM),
                        true,
                    )
                    .clicked()
                    {
                        close = true;
                    }
                });
            });

        if close || modal.should_close() {
            self.apply_destination();
            self.show_settings = false;
        }
    }

    fn close_modal(&mut self, ctx: &egui::Context) {
        if !self.confirm_close {
            ctx.animate_bool_with_time(Id::unique("close_enter"), false, 0.0);
            return;
        }
        let busy = self.job.is_some();
        let mut exit = false;
        let mut save_exit = false;
        let mut cancel = false;
        let enter = ctx.animate_bool_with_time(Id::unique("close_enter"), true, 0.18);

        let modal = egui::Modal::new(Id::unique("close_modal"))
            .frame(
                egui::Frame::NONE
                    .fill(theme::SURFACE)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(theme::RADIUS)
                    .inner_margin(Margin::same(theme::GAP_XL as i8))
                    .shadow(theme::modal_shadow()),
            )
            .show(ctx, |ui| {
                ui.multiply_opacity(enter);
                ui.set_width(400.0);
                ui.label(theme::micro("// atención"));
                ui.add_space(theme::GAP_S);
                ui.label(theme::display(
                    theme::T_DISPLAY,
                    if busy {
                        "Hay un proceso en marcha"
                    } else {
                        "Hay cambios sin guardar"
                    },
                ));
                ui.add_space(theme::GAP_S);
                ui.add(
                    egui::Label::new(theme::secondary(if busy {
                        "Si sales ahora, la canción quedará a medias. Podrás volver a procesarla."
                    } else {
                        "Guarda la letra y las notas antes de cerrar."
                    }))
                    .wrap(),
                );
                ui.add_space(theme::GAP_XL);
                // Desktop footer: the two decisions sit together at the right,
                // Escape and a click outside are the cancel path.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (primary, secondary) = if busy {
                        ("Seguir procesando", "Salir igualmente")
                    } else {
                        ("Guardar y salir", "Descartar cambios")
                    };
                    if theme::button_sized(
                        ui,
                        primary,
                        ButtonKind::Primary,
                        Vec2::new(176.0, theme::BTN_H_SM),
                        true,
                    )
                    .clicked()
                    {
                        if busy {
                            cancel = true;
                        } else {
                            save_exit = true;
                        }
                    }
                    ui.add_space(theme::GAP_S);
                    if theme::button_sized(
                        ui,
                        secondary,
                        ButtonKind::Secondary,
                        Vec2::new(168.0, theme::BTN_H_SM),
                        true,
                    )
                    .clicked()
                    {
                        exit = true;
                    }
                });
            });

        if modal.should_close() {
            cancel = true;
        }
        if cancel {
            self.confirm_close = false;
        }
        if save_exit {
            self.save_document(false);
            exit = true;
        }
        if exit {
            self.allow_close = true;
            self.confirm_close = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

// ---------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------

impl App {
    fn preview_screen(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .inner_margin(Margin::symmetric(theme::GAP_XL as i8, theme::GAP_M as i8))
            .show(ui, |ui| self.preview_body(ui));
    }

    fn preview_body(&mut self, ui: &mut egui::Ui) {
        let mut leave = false;
        let mut export = false;
        let mut settings = false;
        let mut save = false;
        let mut action: Option<EditAction> = None;

        ui.horizontal(|ui| {
            if theme::button_sized_with_leading_icon(
                ui,
                "Volver",
                Icon::ChevronLeft,
                ButtonKind::Ghost,
                Vec2::new(88.0, 32.0),
                true,
            )
            .clicked()
            {
                leave = true;
            }
            ui.add_space(theme::GAP_S);
            if let Some(session) = &self.session {
                ui.vertical(|ui| {
                    ui.label(theme::micro("// vista previa"));
                    ui.label(theme::display(
                        theme::T_DISPLAY,
                        song_label(&session.project),
                    ));
                    let meta = self
                        .editor
                        .as_ref()
                        .map(|editor| self.preview_meta(editor))
                        .unwrap_or_default();
                    ui.label(theme::mono(meta, theme::T_CAPTION));
                });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::Settings, "Ajustes").clicked() {
                    settings = true;
                }
                if theme::button_sized_with_icon(
                    ui,
                    "Exportar",
                    Some(Icon::Download),
                    ButtonKind::Primary,
                    Vec2::new(120.0, theme::BTN_H_SM),
                    true,
                )
                .clicked()
                {
                    export = true;
                }
                let dirty = self.editor.as_ref().is_some_and(EditorState::is_dirty);
                if theme::button_sized(
                    ui,
                    if dirty { "Guardar" } else { "Guardado" },
                    ButtonKind::Secondary,
                    Vec2::new(104.0, theme::BTN_H_SM),
                    dirty,
                )
                .clicked()
                {
                    save = true;
                }
                if theme::icon_button(ui, Icon::Redo, "Rehacer (Ctrl+Y)").clicked() {
                    action = Some(EditAction::Redo);
                }
                if theme::icon_button(ui, Icon::Undo, "Deshacer (Ctrl+Z)").clicked() {
                    action = Some(EditAction::Undo);
                }
            });
        });

        ui.add_space(theme::GAP_M);
        theme::divider(ui);
        ui.add_space(theme::GAP_M);

        let Some(mut editor) = self.editor.take() else {
            ui.centered_and_justified(|ui| {
                ui.label(theme::secondary(
                    "Sin canción: procesa un proyecto para generar notas.",
                ));
            });
            if leave {
                self.screen = Screen::Done;
            }
            return;
        };

        let duration = self.preview_duration(&editor);
        let position = self.engine.clock().position_s().clamp(0.0, duration);
        let phrase = phrase_at(editor.document(), position);
        let sounding_note =
            phrase.and_then(|index| active_note_index(editor.document(), index, position));
        let next_note =
            phrase.and_then(|index| upcoming_note_index(editor.document(), index, position));

        ui.horizontal(|ui| {
            ui.label(theme::micro("// frase actual"));
            ui.add_space(theme::GAP_M);
            if let Some((phrase_index, note_index)) = phrase.zip(sounding_note) {
                if let Some(note) = editor
                    .document()
                    .phrases
                    .get(phrase_index)
                    .and_then(|p| p.notes.get(note_index))
                {
                    let (start, end) = note_span(editor.header(), note);
                    let text = if note.text.trim().is_empty() {
                        "·"
                    } else {
                        note.text.trim()
                    };
                    egui::Frame::NONE
                        .fill(theme::ACCENT.gamma_multiply(0.18))
                        .stroke(Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.55)))
                        .inner_margin(Margin::symmetric(theme::GAP_S as i8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                theme::mono(
                                    format!(
                                        "SONANDO «{text}»  ·  {} ({:+})  ·  {start:.2}s–{end:.2}s",
                                        pitch_name(note.pitch),
                                        note.pitch
                                    ),
                                    theme::T_MICRO,
                                )
                                .color(theme::ACCENT_TEXT),
                            );
                        });
                }
            } else if let Some((phrase_index, note_index)) = phrase.zip(next_note) {
                if let Some(note) = editor
                    .document()
                    .phrases
                    .get(phrase_index)
                    .and_then(|p| p.notes.get(note_index))
                {
                    let (start, _) = note_span(editor.header(), note);
                    let wait = (start - position).max(0.0);
                    let text = note.text.trim();
                    ui.label(
                        theme::mono(
                            format!(
                                "en silencio  ·  próxima «{text}» ({}) en {wait:.1}s",
                                pitch_name(note.pitch)
                            ),
                            theme::T_MICRO,
                        )
                        .color(theme::TEXT_3),
                    );
                }
            } else {
                ui.label(theme::mono("en silencio", theme::T_MICRO).color(theme::TEXT_3));
            }
        });
        ui.add_space(theme::GAP_S);

        let selection = editor.selection();
        let mut picked: Option<(NoteSelection, f64)> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(theme::GAP_XS, theme::GAP_XS);
            match phrase {
                Some(phrase_index) => {
                    if let Some(phrase_ref) = editor.document().phrases.get(phrase_index) {
                        for (index, note) in phrase_ref.notes.iter().enumerate() {
                            let (start, end) = note_span(editor.header(), note);
                            let active = sounding_note == Some(index);
                            let sung = end <= position;
                            let color = if active {
                                theme::ACCENT
                            } else if sung {
                                theme::ACCENT_TEXT
                            } else if note.is_continuation() {
                                theme::TEXT_3
                            } else {
                                theme::TEXT
                            };
                            let text = if note.text.is_empty() {
                                "·"
                            } else {
                                note.text.as_str()
                            };
                            let response = ui.add(
                                egui::Label::new(
                                    theme::display(theme::T_DISPLAY, text).color(color),
                                )
                                .sense(Sense::click()),
                            );
                            let current = NoteSelection {
                                phrase: phrase_index,
                                note: index,
                            };
                            if selection == Some(current) {
                                ui.painter().hline(
                                    response.rect.x_range(),
                                    response.rect.bottom(),
                                    Stroke::new(2.0, theme::ACCENT),
                                );
                            } else if response.hovered() {
                                ui.painter().hline(
                                    response.rect.x_range(),
                                    response.rect.bottom() - 1.0,
                                    Stroke::new(1.0, theme::TEXT_2),
                                );
                            }
                            if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if response.clicked() {
                                picked = Some((current, start));
                            }
                        }
                    }
                }
                None => {
                    ui.label(theme::secondary("Instrumental"));
                }
            }
        });
        if let Some((target, start)) = picked {
            editor.select(target.phrase, target.note).ok();
            self.engine.seek(start);
        }

        ui.add_space(theme::GAP_S);
        edit_bar(ui, &editor, position, &mut action);

        ui.add_space(theme::GAP_S);
        let reserved = 96.0;
        let notes_height = (ui.available_height() - reserved).max(190.0);
        if let Some((target, start)) = self.notes_window_ui(ui, &editor, notes_height) {
            editor.select(target.phrase, target.note).ok();
            self.engine.seek(start);
        }

        ui.add_space(theme::GAP_S);
        let position = self.engine.clock().position_s().clamp(0.0, duration);
        // The transport sits on the same surface as every other block.
        egui::Frame::NONE
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(theme::RADIUS)
            .inner_margin(Margin::symmetric(theme::GAP_M as i8, theme::GAP_S as i8))
            .show(ui, |ui| {
                self.scrubber_ui(ui, position, duration);
                ui.add_space(theme::GAP_S);

                ui.horizontal(|ui| {
                    if theme::icon_button(ui, Icon::SkipBack, "Al principio").clicked() {
                        self.engine.pause();
                        self.engine.seek(0.0);
                    }
                    let (icon, tooltip) = if self.engine.clock().is_playing() {
                        (Icon::Pause, "Pausa")
                    } else {
                        (Icon::Play, "Reproducir")
                    };
                    if theme::icon_button(ui, icon, tooltip).clicked() {
                        self.engine.toggle();
                    }
                    if theme::icon_button(ui, Icon::Stop, "Detener").clicked() {
                        self.engine.pause();
                        self.engine.seek(0.0);
                    }
                    ui.add_space(theme::GAP_L);
                    ui.label(theme::micro("velocidad"));
                    ui.add_space(theme::GAP_S);
                    let rates = [
                        ("0,5×", 0.5_f32),
                        ("0,75×", 0.75),
                        ("1×", 1.0),
                        ("1,25×", 1.25),
                    ];
                    if let Some(rate) = theme::segmented(ui, &rates, self.engine.clock().rate()) {
                        self.engine.set_rate(rate);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(theme::micro("espacio · play / ← → · 5 s"));
                    });
                });
            });

        if let Some(action) = action {
            let now = Instant::now();
            let outcome = match action {
                EditAction::Delete => editor.delete_selected(now),
                EditAction::Kind(kind) => editor.toggle_kind(kind, now).map(|_| ()),
                EditAction::ShiftGap(delta_ms) => {
                    let next = editor
                        .header()
                        .gap_ms
                        .saturating_add(delta_ms)
                        .clamp(-600_000, 600_000);
                    editor.set_gap_ms(next, now)
                }
                EditAction::AlignGapToCursor(cursor_s) => {
                    let target_beat = editor
                        .selected_note()
                        .or_else(|| {
                            editor
                                .document()
                                .phrases
                                .first()
                                .and_then(|phrase| phrase.notes.first())
                        })
                        .map(|note| note.start_beat);
                    if let Some(start_beat) = target_beat {
                        let bpm = editor.header().bpm.max(1.0);
                        let beat_ms = 60_000.0 / (bpm * 4.0);
                        let next = (cursor_s * 1000.0 - start_beat as f64 * beat_ms)
                            .round()
                            .clamp(-600_000.0, 600_000.0) as i64;
                        editor.set_gap_ms(next, now)
                    } else {
                        Ok(())
                    }
                }
                EditAction::Undo => {
                    editor.undo();
                    Ok(())
                }
                EditAction::Redo => {
                    editor.redo();
                    Ok(())
                }
            };
            if let Err(error) = outcome {
                self.notify_error(format!("No se pudo editar: {error}"));
            }
        }

        self.editor = Some(editor);

        if save {
            self.save_document(false);
        }
        if leave {
            self.engine.pause();
            self.screen = Screen::Done;
        }
        if export {
            self.export();
        }
        if settings {
            self.open_settings();
        }
    }

    fn preview_duration(&self, editor: &EditorState) -> f64 {
        let engine = self.engine.clock().duration_s();
        if engine > 1.0 {
            return engine;
        }
        let source = self
            .session
            .as_ref()
            .and_then(|session| session.project.source.duration_ms)
            .map(|milliseconds| milliseconds as f64 / 1000.0);
        source.unwrap_or_else(|| document_span(editor.document()).max(1.0))
    }

    fn preview_meta(&self, editor: &EditorState) -> String {
        let duration = self.preview_duration(editor);
        format!(
            "{}  ·  {} notas  ·  {} frases",
            format_clock(duration),
            editor.document().note_count(),
            editor.document().phrases.len()
        )
    }

    /// Paints the pitch lane. Returns the note the pointer clicked, if any, so
    /// the caller can select it; dragging keeps seeking.
    fn notes_window_ui(
        &mut self,
        ui: &mut egui::Ui,
        editor: &EditorState,
        height: f32,
    ) -> Option<(NoteSelection, f64)> {
        let desired = egui::vec2(ui.available_width(), height);
        let (response, painter) = ui.allocate_painter(desired, Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0, theme::FIELD);
        painter.rect_stroke(rect, 0, Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
        // Same corner marks as every other block, so the canvas belongs to the
        // same drawing.
        theme::brackets(&painter, rect, theme::BORDER_HOVER);

        let duration = self.preview_duration(editor);
        let position = self.engine.clock().position_s().clamp(0.0, duration);
        let span = 9.0_f64.min(duration.max(1.0));
        let start = (position - span * 0.35).clamp(0.0, (duration - span).max(0.0));
        let end = start + span;

        let inner = Rect::from_min_max(
            rect.min + Vec2::new(14.0, 30.0),
            rect.max - Vec2::new(14.0, 22.0),
        );

        painter.text(
            rect.left_top() + Vec2::new(14.0, 10.0),
            Align2::LEFT_TOP,
            "// NOTAS SILABEADAS",
            FontId::monospace(theme::T_MICRO),
            theme::TEXT_3,
        );
        let phrase = phrase_at(editor.document(), position);
        let phrase_label = phrase
            .map(|index| {
                format!(
                    "FRASE {:02}/{:02}",
                    index + 1,
                    editor.document().phrases.len()
                )
            })
            .unwrap_or_else(|| "INSTRUMENTAL".into());
        painter.text(
            rect.right_top() + Vec2::new(-14.0, 10.0),
            Align2::RIGHT_TOP,
            phrase_label,
            FontId::monospace(theme::T_MICRO),
            theme::TEXT_3,
        );

        let x_of = |seconds: f64| -> f32 {
            inner.left() + ((seconds - start) / span).clamp(0.0, 1.0) as f32 * inner.width()
        };
        let time_of = |x: f32| -> f64 {
            start + ((x - inner.left()) / inner.width()).clamp(0.0, 1.0) as f64 * span
        };

        // Time ruler.
        let mut tick = (start / 1.0).ceil();
        while tick <= end {
            let x = x_of(tick);
            let major = tick.round() as i64 % 2 == 0;
            painter.vline(
                x,
                egui::Rangef::new(
                    inner.bottom() + 2.0,
                    inner.bottom() + if major { 8.0 } else { 5.0 },
                ),
                Stroke::new(
                    1.0,
                    theme::TEXT_3.gamma_multiply(if major { 0.65 } else { 0.35 }),
                ),
            );
            if major && x > inner.left() + 12.0 && x < inner.right() - 12.0 {
                painter.text(
                    egui::pos2(x, inner.bottom() + 10.0),
                    Align2::CENTER_TOP,
                    format!("{}:{:02}", (tick / 60.0) as u64, (tick % 60.0) as u64),
                    FontId::monospace(theme::T_MICRO),
                    theme::TEXT_3,
                );
            }
            tick += 1.0;
        }

        let document = editor.document();
        // The melody usually sits in a narrow band: trim the tails so the lane
        // uses its height instead of showing a thin line of blocks.
        let mut pitches: Vec<i16> = document.notes().map(|note| note.pitch).collect();
        let (min_pitch, max_pitch) = if pitches.is_empty() {
            (-5, 7)
        } else {
            pitches.sort_unstable();
            let low = pitches[(pitches.len() as f32 * 0.10) as usize];
            let high = pitches[((pitches.len() - 1) as f32 * 0.90) as usize];
            let center = (low + high) / 2;
            let range = (high - low + 4).clamp(10, 14);
            (center - range / 2, center - range / 2 + range)
        };
        let range = (max_pitch - min_pitch).max(1) as f32;
        let note_height = (inner.height() / range * 1.5).clamp(12.0, 26.0);
        let y_of = |pitch: i16| -> f32 {
            let pitch = pitch.clamp(min_pitch, max_pitch);
            inner.bottom() - ((pitch - min_pitch) as f32 / range) * inner.height()
        };

        let active_phrase = phrase;
        let active_note =
            active_phrase.and_then(|index| active_note_index(document, index, position));

        if let Some((phrase_index, note_index)) = active_phrase.zip(active_note) {
            if let Some(note) = document
                .phrases
                .get(phrase_index)
                .and_then(|p| p.notes.get(note_index))
            {
                let text = if note.text.trim().is_empty() {
                    "·"
                } else {
                    note.text.trim()
                };
                painter.text(
                    egui::pos2(rect.center().x, rect.top() + 10.0),
                    Align2::CENTER_TOP,
                    format!("SONANDO: «{text}» · {}", pitch_name(note.pitch)),
                    FontId::monospace(theme::T_MICRO),
                    theme::ACCENT_TEXT,
                );
            }
        }

        // Pitch grid: keeps the empty register readable as a note canvas.
        for pitch in min_pitch..=max_pitch {
            let y = y_of(pitch);
            let base = if pitch % 12 == 0 {
                theme::BORDER
            } else if pitch % 2 == 0 {
                theme::GRID_MINOR
            } else {
                theme::GRID_FAINT
            };
            painter.hline(inner.x_range(), y, Stroke::new(1.0, base));
            if pitch % 2 == 0 {
                painter.text(
                    egui::pos2(inner.left() + 4.0, y - 1.0),
                    Align2::LEFT_BOTTOM,
                    pitch_name(pitch),
                    FontId::monospace(9.5),
                    theme::TEXT_3.gamma_multiply(if pitch % 12 == 0 { 0.85 } else { 0.45 }),
                );
            }
        }

        let selection = editor.selection();
        let mut hits: Vec<(Rect, NoteSelection, f64)> = Vec::new();
        for (phrase_index, phrase_ref) in document.phrases.iter().enumerate() {
            if let Some(first) = phrase_ref.notes.first() {
                let (phrase_start, _) = note_span(editor.header(), first);
                if phrase_start > start && phrase_start < end {
                    painter.vline(
                        x_of(phrase_start),
                        inner.y_range(),
                        Stroke::new(1.0, theme::ACCENT_TEXT.gamma_multiply(0.16)),
                    );
                }
            }
            for (note_index, note) in phrase_ref.notes.iter().enumerate() {
                let (note_start, note_end) = note_span(editor.header(), note);
                if note_end < start || note_start > end {
                    continue;
                }
                let x0 = x_of(note_start);
                let x1 = x_of(note_end).max(x0 + 3.0);
                let y = y_of(note.pitch);
                let note_rect = Rect::from_min_max(
                    egui::pos2(x0, y - note_height / 2.0),
                    egui::pos2(x1, y + note_height / 2.0),
                );
                let is_active =
                    active_phrase == Some(phrase_index) && active_note == Some(note_index);
                let in_current_phrase = active_phrase == Some(phrase_index);
                let past = note_end <= position;
                // Freestyle is not scored, so it reads as an outline; rap and
                // golden notes keep a coloured bar under the block.
                let freestyle = note.kind == NoteKind::Freestyle;

                let (fill, stroke, text_color) = if is_active {
                    (
                        theme::ACCENT,
                        Stroke::new(1.0, theme::ACCENT_TEXT),
                        theme::ON_ACCENT,
                    )
                } else if in_current_phrase {
                    (
                        if freestyle {
                            theme::FIELD
                        } else {
                            theme::ACCENT.gamma_multiply(0.10)
                        },
                        Stroke::new(
                            1.0,
                            if freestyle {
                                theme::TEXT_3
                            } else {
                                theme::ACCENT.gamma_multiply(0.55)
                            },
                        ),
                        if freestyle {
                            theme::TEXT_2
                        } else {
                            theme::ACCENT_TEXT
                        },
                    )
                } else if past {
                    (
                        theme::SURFACE_2,
                        Stroke::new(1.0, theme::BORDER),
                        theme::TEXT_3,
                    )
                } else {
                    (
                        theme::FIELD,
                        Stroke::new(
                            1.0,
                            if freestyle {
                                theme::TEXT_3
                            } else {
                                theme::ACCENT.gamma_multiply(0.28)
                            },
                        ),
                        theme::TEXT_2,
                    )
                };
                painter.rect(note_rect, 0, fill, stroke, StrokeKind::Inside);
                if let Some(color) = kind_color(note.kind) {
                    painter.rect_filled(
                        Rect::from_min_size(
                            egui::pos2(note_rect.left(), note_rect.bottom() - 2.0),
                            Vec2::new(note_rect.width().max(2.0), 2.0),
                        ),
                        0,
                        color,
                    );
                }
                if note_rect.width() > 14.0 {
                    painter.text(
                        note_rect.center(),
                        Align2::CENTER_CENTER,
                        if note.text.is_empty() {
                            "·"
                        } else {
                            note.text.trim()
                        },
                        FontId::proportional(theme::T_CAPTION),
                        text_color,
                    );
                }
                if is_active {
                    let callout = format!(
                        "{} · {}",
                        if note.text.trim().is_empty() {
                            "·"
                        } else {
                            note.text.trim()
                        },
                        pitch_name(note.pitch)
                    );
                    let badge_y = (note_rect.top() - 6.0).max(inner.top() + 8.0);
                    painter.text(
                        egui::pos2(note_rect.center().x, badge_y),
                        Align2::CENTER_BOTTOM,
                        callout,
                        FontId::monospace(theme::T_MICRO),
                        theme::ACCENT_TEXT,
                    );
                }
                let current = NoteSelection {
                    phrase: phrase_index,
                    note: note_index,
                };
                if selection == Some(current) {
                    painter.rect_stroke(
                        note_rect.expand(1.5),
                        0,
                        Stroke::new(1.5, theme::TEXT),
                        StrokeKind::Outside,
                    );
                }
                hits.push((note_rect, current, note_start));
            }
        }

        // Playhead.
        let playhead_x = x_of(position);
        painter.vline(playhead_x, inner.y_range(), Stroke::new(1.5, theme::ACCENT));
        painter.rect_filled(
            Rect::from_center_size(
                egui::pos2(playhead_x, inner.top() - 2.0),
                Vec2::new(5.0, 7.0),
            ),
            0,
            theme::ACCENT,
        );

        let mut picked = None;
        if let Some(pointer) = response.interact_pointer_pos() {
            if response.clicked() {
                // A click on a block selects it; anywhere else it is a seek.
                match hits.iter().find(|(hit, _, _)| hit.contains(pointer)) {
                    Some((_, selection, note_start)) => picked = Some((*selection, *note_start)),
                    None => self.engine.seek(time_of(pointer.x)),
                }
            } else if response.dragged() {
                self.engine.seek(time_of(pointer.x));
            }
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
        picked
    }

    fn scrubber_ui(&mut self, ui: &mut egui::Ui, position: f64, duration: f64) {
        let mut seek = None;
        ui.horizontal(|ui| {
            ui.label(theme::mono(format_time(position), theme::T_CAPTION).color(theme::TEXT_2));
            ui.add_space(theme::GAP_S);
            let width = (ui.available_width() - 82.0).max(120.0);
            let (response, painter) =
                ui.allocate_painter(egui::vec2(width, 18.0), Sense::click_and_drag());
            let rect = response.rect;
            let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 4.0));
            painter.rect_filled(track, 0, theme::TRACK);
            let fraction = (position / duration.max(0.001)).clamp(0.0, 1.0) as f32;
            painter.rect_filled(
                Rect::from_min_size(track.min, Vec2::new(track.width() * fraction, 4.0)),
                0,
                theme::ACCENT,
            );
            let handle_x = track.left() + track.width() * fraction;
            painter.rect_filled(
                Rect::from_center_size(egui::pos2(handle_x, rect.center().y), Vec2::new(4.0, 14.0)),
                0,
                Color32::WHITE,
            );
            for step in 0..=10 {
                let x = track.left() + track.width() * step as f32 / 10.0;
                painter.vline(
                    x,
                    egui::Rangef::new(track.bottom() + 3.0, track.bottom() + 5.0),
                    Stroke::new(1.0, theme::TEXT_3.gamma_multiply(0.4)),
                );
            }
            if response.clicked() || response.dragged() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let fraction =
                        ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64;
                    seek = Some(fraction * duration);
                }
            }
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
            ui.add_space(theme::GAP_S);
            ui.label(theme::mono(format_time(duration), theme::T_CAPTION).color(theme::TEXT_3));
        });
        if let Some(seconds) = seek {
            self.engine.seek(seconds);
        }
    }
}

fn note_span(header: &SongHeader, note: &Note) -> (f64, f64) {
    let beat_ms = header.beat_duration_ms().max(1.0);
    let start = (header.gap_ms as f64 + note.start_beat as f64 * beat_ms) / 1000.0;
    let end = (header.gap_ms as f64 + note.end_beat() as f64 * beat_ms) / 1000.0;
    let start = start.max(0.0);
    (start, end.max(start + 0.01))
}

fn phrase_span(header: &SongHeader, phrase: &Phrase) -> (f64, f64) {
    let Some(first) = phrase.notes.first() else {
        return (0.0, 0.0);
    };
    let Some(last) = phrase.notes.last() else {
        return (0.0, 0.0);
    };
    (note_span(header, first).0, note_span(header, last).1)
}

fn phrase_at(document: &SongDoc, position: f64) -> Option<usize> {
    let mut next = None;
    for (index, phrase) in document.phrases.iter().enumerate() {
        let (start, end) = phrase_span(&document.header, phrase);
        if position >= start && position <= end {
            return Some(index);
        }
        if start > position && next.is_none() {
            next = Some(index);
        }
    }
    next.or_else(|| document.phrases.len().checked_sub(1))
}

fn active_note_index(document: &SongDoc, phrase_index: usize, position: f64) -> Option<usize> {
    document
        .phrases
        .get(phrase_index)?
        .notes
        .iter()
        .position(|note| {
            let (start, end) = note_span(&document.header, note);
            position >= start && position < end
        })
}

fn upcoming_note_index(document: &SongDoc, phrase_index: usize, position: f64) -> Option<usize> {
    document
        .phrases
        .get(phrase_index)?
        .notes
        .iter()
        .position(|note| note_span(&document.header, note).0 >= position - 0.02)
}

fn document_span(document: &SongDoc) -> f64 {
    document
        .phrases
        .last()
        .and_then(|phrase| phrase.notes.last())
        .map(|note| note_span(&document.header, note).1)
        .unwrap_or(0.0)
}

fn first_note_start(editor: &EditorState) -> Option<f64> {
    editor
        .document()
        .phrases
        .first()?
        .notes
        .first()
        .map(|note| note_span(editor.header(), note).0)
}

fn format_clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}
// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

impl App {
    /// Space toggles playback, arrows seek, Home goes back to the start, and the
    /// edit keys act on the selected syllable.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // A focused text field owns the keyboard; a focused button owns Space,
        // but the transport keys keep working.
        if ctx.text_edit_focused() {
            return;
        }
        let focused = ctx.memory(|memory| memory.focused().is_some());
        let (toggle, back, forward, home, escape, delete, freestyle, rap, normal, undo, redo) = ctx
            .input(|input| {
                (
                    input.key_pressed(egui::Key::Space) && !focused,
                    input.key_pressed(egui::Key::ArrowLeft) && !input.modifiers.shift,
                    input.key_pressed(egui::Key::ArrowRight) && !input.modifiers.shift,
                    input.key_pressed(egui::Key::Home),
                    input.key_pressed(egui::Key::Escape),
                    input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
                    input.key_pressed(egui::Key::F),
                    input.key_pressed(egui::Key::R),
                    input.key_pressed(egui::Key::N),
                    input.modifiers.command
                        && input.key_pressed(egui::Key::Z)
                        && !input.modifiers.shift,
                    input.modifiers.command
                        && (input.key_pressed(egui::Key::Y)
                            || (input.key_pressed(egui::Key::Z) && input.modifiers.shift)),
                )
            });

        let mut failure = None;
        if let Some(editor) = self.editor.as_mut() {
            let now = Instant::now();
            let outcome = if delete {
                editor.delete_selected(now)
            } else if freestyle {
                editor.toggle_kind(NoteKind::Freestyle, now).map(|_| ())
            } else if rap {
                editor.toggle_kind(NoteKind::Rap, now).map(|_| ())
            } else if normal {
                editor.toggle_kind(NoteKind::Normal, now).map(|_| ())
            } else {
                Ok(())
            };
            if undo {
                editor.undo();
            }
            if redo {
                editor.redo();
            }
            if let Err(error) = outcome {
                if !matches!(error, EditorError::NoSelection) {
                    failure = Some(format!("No se pudo editar: {error}"));
                }
            }
        }
        if let Some(message) = failure {
            self.notify_error(message);
        }

        if toggle {
            self.engine.toggle();
        }
        if home {
            self.engine.seek(0.0);
        }
        if back {
            let position = (self.engine.clock().position_s() - 5.0).max(0.0);
            self.engine.seek(position);
        }
        if forward {
            let position = self.engine.clock().position_s() + 5.0;
            self.engine.seek(position);
        }
        if escape {
            self.engine.pause();
            self.screen = Screen::Done;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn preflight(toolchain: &Toolchain) -> Result<(), String> {
    let mut problems = Vec::new();
    let required = [
        ("yt-dlp", toolchain.yt_dlp.as_ref()),
        ("ffmpeg", toolchain.ffmpeg.as_ref()),
        ("ffprobe", toolchain.ffprobe.as_ref()),
    ];
    for (name, tool) in required {
        match tool {
            None => problems.push(format!("falta {name}")),
            Some(tool) if !tool.is_supported() => {
                problems.push(format!("{name} está desactualizado"));
            }
            _ => {}
        }
    }
    match toolchain.js_runtime() {
        None => problems.push("instala Node 18+ o Deno 2+".into()),
        Some(tool) if !tool.is_supported() => {
            problems.push(format!("{} está desactualizado", tool.kind.executable()))
        }
        _ => {}
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Para descargar de YouTube necesito: {}. Reinicia la app después de instalarlo.",
            problems.join(", ")
        ))
    }
}

fn read_song_document(path: &std::path::Path) -> Option<SongDoc> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn stage_index(stage: StageId) -> Option<usize> {
    StageId::ALL
        .iter()
        .position(|candidate| *candidate == stage)
}

/// Sentence shown while the stage runs, in the calm lowercase of the flow.
fn stage_step_label(stage: StageId) -> &'static str {
    match stage {
        StageId::Ingest => "descargando el vídeo de YouTube",
        StageId::Separate => "separando la voz de la música",
        StageId::Transcribe => "obteniendo la letra",
        StageId::Pitch => "detectando las notas",
        StageId::Align => "alineando la letra con la música",
        StageId::Export => "exportando la canción",
    }
}

/// Width of a centered column, capped so small windows keep their margins.
fn flow_width(ctx: &egui::Context, max: f32) -> f32 {
    (ctx.viewport_rect().width() - 64.0).clamp(260.0, max)
}

fn song_label(project: &Project) -> String {
    song_label_parts(
        project.source.artist.as_deref().unwrap_or_default(),
        &project.source.title,
    )
}

fn song_label_parts(artist: &str, title: &str) -> String {
    if title.trim().is_empty() {
        if artist.trim().is_empty() {
            "Canción sin título".into()
        } else {
            artist.to_owned()
        }
    } else if artist.trim().is_empty() {
        title.to_owned()
    } else {
        format!("{artist} — {title}")
    }
}

fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {:02} s", seconds / 60, seconds % 60)
    }
}

/// `m:ss.d` used by the preview clock and scrubber.
fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let minutes = (seconds / 60.0).floor() as u64;
    let rest = seconds - minutes as f64 * 60.0;
    format!("{minutes}:{rest:04.1}")
}

/// One line of the status bar: whether the external tools the pipeline needs
/// are on the machine, and which version of each.
fn tool_status(ui: &mut Ui, toolchain: &Toolchain) {
    let tools = [
        ("yt-dlp", toolchain.yt_dlp.as_ref()),
        ("ffmpeg", toolchain.ffmpeg.as_ref()),
        ("ffprobe", toolchain.ffprobe.as_ref()),
    ];
    let ready = tools.iter().all(|(_, tool)| tool.is_some());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_center_size(rect.center(), Vec2::splat(5.0)),
        0,
        if ready {
            theme::SUCCESS
        } else {
            theme::WARNING
        },
    );
    ui.add_space(theme::GAP_XS);
    for (index, (name, tool)) in tools.into_iter().enumerate() {
        if index > 0 {
            ui.label(theme::mono("·", theme::T_MICRO).color(theme::TEXT_3));
        }
        match tool {
            Some(tool) => {
                ui.label(theme::mono(
                    format!("{name} {}", tool.version),
                    theme::T_MICRO,
                ));
            }
            None => {
                ui.label(
                    RichText::new(format!("{name} no encontrado"))
                        .size(theme::T_MICRO)
                        .monospace()
                        .color(theme::WARNING),
                );
            }
        }
    }
}

/// Actions for the selected syllable and global note offset (`#GAP`).
fn edit_bar(ui: &mut Ui, editor: &EditorState, position: f64, action: &mut Option<EditAction>) {
    ui.horizontal(|ui| {
        ui.label(theme::micro("// edición"));
        if let Some(note) = editor.selected_note() {
            let text = if note.is_continuation() {
                "~"
            } else {
                note.text.as_str()
            };
            ui.label(theme::strong(text));
            ui.label(
                theme::mono(pitch_name(note.pitch), theme::T_CAPTION).color(theme::ACCENT_TEXT),
            );
            ui.label(theme::caption(kind_label(note.kind)));
            ui.add_space(theme::GAP_S);

            let freestyle = note.kind == NoteKind::Freestyle;
            if theme::button_sized(
                ui,
                "No cantable",
                if freestyle {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                Vec2::new(104.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Se ve en pantalla pero no puntúa (F)")
            .clicked()
            {
                *action = Some(EditAction::Kind(NoteKind::Freestyle));
            }

            let rap = matches!(note.kind, NoteKind::Rap | NoteKind::RapGolden);
            if theme::button_sized(
                ui,
                "Rap",
                if rap {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                Vec2::new(56.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Se canta hablado; puntúa por tiempo (R)")
            .clicked()
            {
                *action = Some(EditAction::Kind(NoteKind::Rap));
            }

            if theme::button_sized(
                ui,
                "Normal",
                if note.kind == NoteKind::Normal {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                Vec2::new(74.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Vuelve a nota normal (N)")
            .clicked()
            {
                *action = Some(EditAction::Kind(NoteKind::Normal));
            }

            if theme::button_sized_with_icon(
                ui,
                "Borrar",
                Some(Icon::Trash),
                ButtonKind::Ghost,
                Vec2::new(88.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Quita la sílaba de la canción (Supr)")
            .clicked()
            {
                *action = Some(EditAction::Delete);
            }
        } else {
            ui.label(theme::caption(
                "Haz clic en una sílaba o bloque para editarla.",
            ));
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let align_label = if editor.selected_note().is_some() {
                "Traer nota aquí"
            } else {
                "Traer inicio aquí"
            };
            if theme::button_sized(
                ui,
                align_label,
                ButtonKind::Secondary,
                Vec2::new(124.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text(
                "Ajusta el desfase para que la nota seleccionada (o la primera) empiece en la posición actual del audio",
            )
            .clicked()
            {
                *action = Some(EditAction::AlignGapToCursor(position));
            }
            if theme::button_sized(
                ui,
                "+500",
                ButtonKind::Ghost,
                Vec2::new(48.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Retrasa todas las notas 500 ms")
            .clicked()
            {
                *action = Some(EditAction::ShiftGap(500));
            }
            if theme::button_sized(
                ui,
                "+50",
                ButtonKind::Ghost,
                Vec2::new(42.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Retrasa todas las notas 50 ms")
            .clicked()
            {
                *action = Some(EditAction::ShiftGap(50));
            }
            ui.label(
                theme::mono(format!("{:+} ms", editor.header().gap_ms), theme::T_CAPTION)
                    .color(theme::TEXT),
            );
            if theme::button_sized(
                ui,
                "−50",
                ButtonKind::Ghost,
                Vec2::new(42.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Adelanta todas las notas 50 ms")
            .clicked()
            {
                *action = Some(EditAction::ShiftGap(-50));
            }
            if theme::button_sized(
                ui,
                "−500",
                ButtonKind::Ghost,
                Vec2::new(48.0, theme::BTN_H_GHOST),
                true,
            )
            .on_hover_text("Adelanta todas las notas 500 ms")
            .clicked()
            {
                *action = Some(EditAction::ShiftGap(-500));
            }
            ui.label(theme::micro("// desfase"));
        });
    });
}

fn pitch_name(pitch: i16) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let midi = pitch as i32 + 60;
    let index = midi.rem_euclid(12) as usize;
    let octave = midi.div_euclid(12) - 1;
    format!("{}{octave}", NAMES[index])
}

fn kind_label(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Normal => "normal",
        NoteKind::Golden => "dorada",
        NoteKind::Freestyle => "no cantable",
        NoteKind::Rap => "rap",
        NoteKind::RapGolden => "rap dorada",
    }
}

fn kind_color(kind: NoteKind) -> Option<Color32> {
    match kind {
        NoteKind::Normal => None,
        NoteKind::Golden | NoteKind::Rap | NoteKind::RapGolden => Some(theme::WARNING),
        NoteKind::Freestyle => Some(theme::TEXT_3),
    }
}

fn model_row(ui: &mut Ui, name: &str, detail: &str, ready: bool) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
        let glyph = if ready { Glyph::Done } else { Glyph::Pending };
        theme::stage_glyph(ui, rect.center(), 6.0, glyph);
        ui.vertical(|ui| {
            ui.label(theme::strong(name));
            ui.label(theme::caption(detail));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (label, color) = if ready {
                ("Listo", theme::SUCCESS)
            } else {
                ("Pendiente", theme::TEXT_3)
            };
            ui.label(RichText::new(label).size(theme::T_CAPTION).color(color));
        });
    });
}

/// Project defaults, honouring the optional `USDX_LANGUAGE` override so the
/// language documented in the user guide also applies to projects created from
/// the GUI.
fn default_settings() -> ProjectSettings {
    let language = std::env::var("USDX_LANGUAGE")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    ProjectSettings {
        force_language: language.is_some(),
        language,
        // UltraStar shows the music video, so it is part of a finished song.
        download_video: true,
        ..ProjectSettings::default()
    }
}

fn playback_message(error: PlaybackError) -> String {
    format!("Reproducción: {error}")
}
