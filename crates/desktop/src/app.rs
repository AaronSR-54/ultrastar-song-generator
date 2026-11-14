//! eframe application: one URL in, one playable UltraStar song out.
//!
//! The app deliberately exposes a single path: paste a YouTube link, press
//! Process, watch the stages, open the folder. Everything else (projects,
//! models, export) happens on its own.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Id, Layout, Margin, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use url::Url;
use usdx_application::{
    atomic_write, JobError, JobEvent, JobHandle, JobRunner, Pipeline, PipelineConfig,
    PipelineError, PitchArtifacts, Project, ProjectPaths, ProjectSettings, ProjectStore,
    SourceInfo, StageId, StageProgress, StageStatus,
};
use usdx_domain::SongDoc;
use usdx_media::Toolchain;

use crate::editor::{EditorState, ExportCheck, NoteSelection};
use crate::playback::{AudioEngine, LoopRange, PlaybackError, Stem};
use crate::settings::GuiSettings;
use crate::setup::{self, FinishedSong};
use crate::theme::{self, ButtonKind, Glyph};
use crate::waveform::PeakPyramid;

const CARD_WIDTH: f32 = 560.0;
const TOAST_SECONDS: f32 = 3.0;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Working,
    Done,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Models,
    Pipeline,
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
    editor: Option<EditorState>,
    engine: AudioEngine,
    peaks: Option<PeakPyramid>,
    tempo_candidates: Vec<(f64, i64)>,
    toast: Option<(String, Instant)>,
    show_settings: bool,
    destination_text: String,
    confirm_close: bool,
    allow_close: bool,
    progress_spring: theme::Spring,
    check_spring: theme::Spring,
    last_frame: Instant,
    view_start_s: f64,
    view_span_s: f64,
    played_since_load: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self {
            runner: JobRunner::new(),
            settings: GuiSettings::load(),
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
            editor: None,
            engine: AudioEngine::new(),
            peaks: None,
            tempo_candidates: Vec::new(),
            toast: None,
            show_settings: false,
            destination_text: String::new(),
            confirm_close: false,
            allow_close: false,
            progress_spring: theme::Spring::new(0.0),
            check_spring: theme::Spring::new(0.0),
            last_frame: Instant::now(),
            view_start_s: 0.0,
            view_span_s: 30.0,
            played_since_load: false,
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
                self.peaks = None;
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

    fn retry(&mut self) {
        match self.working_mode {
            JobKind::Models => self.begin_processing(),
            JobKind::Pipeline => {
                self.error = None;
                self.start_pipeline_job();
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
        if stage_before != self.active_stage() {
            self.progress_spring.reset(0.0);
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
                            self.load_candidates();
                            if let Some(error) = self
                                .finished
                                .as_ref()
                                .and_then(|song| song.copy_error.clone())
                            {
                                self.notify(format!("No se pudo copiar al destino: {error}"));
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
        }
    }

    fn progress_target(&self) -> f32 {
        if let Some(job) = &self.job {
            job.stage_fraction.clamp(0.0, 1.0)
        } else if self.screen == Screen::Done {
            1.0
        } else {
            0.0
        }
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
        self.peaks = None;
        self.finished = None;
        self.url.clear();
        self.home_error = None;
        self.error = None;
        self.screen = Screen::Home;
        self.progress_spring.reset(0.0);
        self.check_spring.reset(0.0);
        self.view_start_s = 0.0;
        self.view_span_s = 30.0;
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

    fn load_candidates(&mut self) {
        self.tempo_candidates.clear();
        let Some(session) = &self.session else {
            return;
        };
        let Ok(bytes) = std::fs::read(session.paths().notes()) else {
            return;
        };
        let Ok(artifacts) = serde_json::from_slice::<PitchArtifacts>(&bytes) else {
            return;
        };
        self.tempo_candidates = artifacts
            .candidates
            .iter()
            .map(|candidate| (candidate.bpm_usdx, candidate.gap_ms))
            .collect();
    }

    fn load_audio(&mut self) {
        let message = {
            let Some(session) = &self.session else {
                return;
            };
            let mix = session.paths().analysis_wav();
            if !mix.exists() {
                self.peaks = None;
                return;
            }
            match charon_audio::AudioFile::read(&mix) {
                Ok(audio) => {
                    let mono = charon_audio::AudioBuffer::new(audio.to_mono(), audio.sample_rate);
                    let samples = mono.data.row(0).to_vec();
                    let peaks = PeakPyramid::from_mono(&samples, mono.sample_rate);
                    self.view_span_s = peaks.duration_s().clamp(5.0, 30.0);
                    self.peaks = Some(peaks);
                    self.engine.load(&mix).err().map(playback_message)
                }
                Err(error) => {
                    self.peaks = None;
                    Some(format!("No se pudo leer el audio de análisis: {error}"))
                }
            }
        };
        if let Some(message) = message {
            self.notify(message);
        }
    }

    fn save_document(&mut self) {
        let message = {
            let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
                return;
            };
            match serde_json::to_vec_pretty(editor.document()) {
                Ok(bytes) => match atomic_write(&session.paths().song(), &bytes) {
                    Ok(()) => {
                        editor.mark_saved();
                        "Guardado".to_owned()
                    }
                    Err(error) => format!("No se pudo guardar: {error}"),
                },
                Err(error) => format!("No se pudo serializar la canción: {error}"),
            }
        };
        self.notify(message);
    }

    fn autosave_if_needed(&mut self) {
        let now = Instant::now();
        let due = self
            .editor
            .as_ref()
            .map(|editor| editor.is_dirty() && editor.should_autosave(now))
            .unwrap_or(false);
        if due {
            self.save_document();
        }
    }

    fn export(&mut self) {
        let validation = {
            let Some(editor) = &mut self.editor else {
                self.notify("No hay canción para exportar");
                return;
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
            self.notify(message);
            return;
        }

        let destination = self.settings.destination_dir.clone();
        let outcome = {
            let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
                return;
            };
            let pipeline = session.pipeline();
            match pipeline.export_document(editor.document()) {
                Ok(outcome) => {
                    editor.mark_saved();
                    Ok((editor.document().clone(), outcome))
                }
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
                self.finished = Some(FinishedSong {
                    title: document.header.title.clone(),
                    artist: document.header.artist.clone(),
                    folder: outcome.folder,
                    copied_to,
                    copy_error: copy_error.clone(),
                });
                if let Some(error) = copy_error {
                    self.notify(format!("{detail} (pero no se pudo copiar: {error})"));
                } else {
                    self.notify(detail);
                }
            }
            Err(message) => self.notify(message),
        }
    }

    fn current_stem_path(&self, stem: Stem) -> Option<PathBuf> {
        let session = self.session.as_ref()?;
        let paths = session.paths();
        Some(match stem {
            Stem::Mix => paths.analysis_wav(),
            Stem::Vocals => paths.vocals(),
            Stem::Instrumental => paths.instrumental(),
        })
        .filter(|path| path.exists())
    }

    fn switch_stem(&mut self, stem: Stem) {
        let Some(path) = self.current_stem_path(stem) else {
            self.notify(format!("No hay stem {} disponible", stem.label()));
            return;
        };
        match self.engine.switch_stem(stem, &path) {
            Ok(()) => self.notify(format!("Stem: {}", stem.label())),
            Err(error) => self.notify(playback_message(error)),
        }
    }

    fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
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

        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed().as_secs_f32() < TOAST_SECONDS)
        {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        self.autosave_if_needed();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        if self.screen == Screen::Review {
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

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG))
            .show(ui, |ui| match self.screen {
                Screen::Home => self.home_screen(ui),
                Screen::Working => self.working_screen(ui),
                Screen::Done => self.done_screen(ui),
                Screen::Review => self.review_screen(ui),
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
// Screens
// ---------------------------------------------------------------------------

impl App {
    fn home_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut process = false;
        egui::Area::new(Id::unique("home"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.set_width(CARD_WIDTH);
                theme::card(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(58.0), Sense::hover());
                            let painter = ui.painter();
                            painter.rect_filled(rect, 17, theme::ACCENT);
                            painter.text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                "♪",
                                FontId::new(30.0, theme::semibold()),
                                Color32::WHITE,
                            );
                        }
                        ui.add_space(20.0);
                        ui.label(theme::display(29.0, "UltraStar Song Generator"));
                        ui.add_space(8.0);
                        ui.add(
                            egui::Label::new(theme::secondary(
                                "Pega un enlace de YouTube y conviértelo en una canción jugable.",
                            ))
                            .wrap(),
                        );
                    });

                    ui.add_space(26.0);
                    let response =
                        theme::text_field(ui, &mut self.url, "https://www.youtube.com/watch?v=…");
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        process = true;
                    }
                    if let Some(error) = &self.home_error {
                        ui.add_space(10.0);
                        ui.label(RichText::new(error).size(13.0).color(theme::DANGER));
                    }
                    ui.add_space(14.0);
                    if theme::primary_button(ui, "Procesar").clicked() {
                        process = true;
                    }

                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        if !self.models.can_process() {
                            ui.label(theme::caption(
                                "La primera vez se descarga el modelo de separación (~302 MB).",
                            ));
                            ui.add_space(3.0);
                        }
                        ui.label(theme::caption(
                            "La letra se busca online; el audio se procesa en tu equipo.",
                        ));
                    });
                });
            });

        if process {
            self.begin_processing();
        }
    }

    fn working_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Area::new(Id::unique("working"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.set_width(CARD_WIDTH);
                theme::card(ui, |ui| {
                    if let Some(error) = self.error.clone() {
                        self.error_body(ui, &error);
                    } else {
                        self.running_body(ui);
                    }
                });
            });
    }

    fn running_body(&mut self, ui: &mut egui::Ui) {
        let models_phase = self.working_mode == JobKind::Models;
        let title = if models_phase {
            "Preparando los modelos".to_owned()
        } else {
            "Procesando tu canción".to_owned()
        };
        ui.label(theme::display(24.0, title));
        ui.add_space(6.0);

        let subtitle = if models_phase {
            "Solo ocurre la primera vez. Los modelos se guardan en tu equipo.".to_owned()
        } else {
            self.session
                .as_ref()
                .map(|session| song_label(&session.project))
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| "Preparando el proyecto…".into())
        };
        ui.add(
            egui::Label::new(theme::secondary(subtitle))
                .wrap()
                .truncate(),
        );

        ui.add_space(20.0);
        theme::progress_bar(ui, self.progress_spring.value());
        ui.add_space(8.0);

        let percent = (self.progress_spring.value() * 100.0).round() as i32;
        let elapsed = self
            .job
            .as_ref()
            .map(|job| job.started.elapsed().as_secs())
            .unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label(theme::caption(format!("{percent}%")));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(theme::caption(format_elapsed(elapsed)));
            });
        });

        if models_phase {
            let message = self
                .job
                .as_ref()
                .map(|job| job.last_message.clone())
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "Descargando los modelos de IA…".into());
            ui.add_space(10.0);
            ui.label(theme::secondary(message));
            ui.add_space(10.0);
        } else {
            let description = self
                .active_stage()
                .map(stage_description)
                .unwrap_or("Preparando el proyecto…");
            ui.add_space(10.0);
            ui.label(theme::secondary(description));
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);
            self.stage_rows(ui);
            ui.add_space(6.0);
        }

        ui.add_space(18.0);
        ui.vertical_centered(|ui| {
            if theme::ghost_button(ui, "Cancelar").clicked() {
                self.cancel_job();
            }
        });
    }

    fn error_body(&mut self, ui: &mut egui::Ui, error: &str) {
        let models_phase = self.working_mode == JobKind::Models;
        ui.vertical_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            let painter = ui.painter();
            painter.circle_filled(rect.center(), 15.0, theme::DANGER.gamma_multiply(0.18));
            let shift = 6.0;
            painter.line_segment(
                [
                    rect.center() + Vec2::new(-shift, -shift),
                    rect.center() + Vec2::new(shift, shift),
                ],
                Stroke::new(2.0, theme::DANGER),
            );
            painter.line_segment(
                [
                    rect.center() + Vec2::new(shift, -shift),
                    rect.center() + Vec2::new(-shift, shift),
                ],
                Stroke::new(2.0, theme::DANGER),
            );
        });
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(theme::display(24.0, "No se pudo terminar"));
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(theme::secondary(if models_phase {
                    "No se pudieron descargar los modelos de IA."
                } else {
                    "La canción quedó a medias. Puedes reintentar sin empezar de cero."
                }))
                .wrap(),
            );
        });

        ui.add_space(18.0);
        egui::ScrollArea::vertical()
            .max_height(96.0)
            .show(ui, |ui| {
                theme::inset(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(error)
                                .monospace()
                                .size(12.0)
                                .color(theme::TEXT_2),
                        )
                        .wrap()
                        .selectable(true),
                    );
                });
            });

        if !models_phase {
            ui.add_space(12.0);
            self.stage_rows(ui);
        }

        ui.add_space(18.0);
        if theme::primary_button(ui, "Reintentar").clicked() {
            self.retry();
        }
        ui.add_space(8.0);
        if theme::secondary_button(ui, "Volver al inicio").clicked() {
            self.session = None;
            self.pending_process = None;
            self.error = None;
            self.screen = Screen::Home;
        }
    }

    fn stage_rows(&mut self, ui: &mut egui::Ui) {
        let current = self.active_stage();
        let current_index = current.and_then(|stage| {
            StageId::ALL
                .iter()
                .position(|candidate| *candidate == stage)
        });
        let failed = self.session.as_ref().and_then(|session| {
            StageId::ALL
                .into_iter()
                .find(|stage| session.project.stage(*stage).status == StageStatus::Failed)
        });
        let time = ui.input(|input| input.time);

        for (index, stage) in StageId::ALL.iter().enumerate() {
            let status = self
                .session
                .as_ref()
                .map(|session| session.project.stage(*stage).status)
                .unwrap_or(StageStatus::Pending);
            let is_current = current == Some(*stage);
            let is_done = matches!(status, StageStatus::Done | StageStatus::Skipped)
                || current_index.is_some_and(|current| index < current);
            let is_failed = status == StageStatus::Failed || failed == Some(*stage);
            let glyph = if is_failed {
                Glyph::Failed
            } else if is_current {
                Glyph::Running
            } else if is_done {
                Glyph::Done
            } else {
                Glyph::Pending
            };

            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                theme::stage_glyph(ui, rect.center(), 9.0, glyph, time);
                ui.add_space(2.0);
                ui.label(if is_done {
                    theme::secondary(stage_name(*stage))
                } else {
                    RichText::new(stage_name(*stage))
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (label, color) = if is_failed {
                        ("Error", theme::DANGER)
                    } else if is_current {
                        ("En curso", theme::ACCENT)
                    } else if is_done {
                        ("Listo", theme::SUCCESS)
                    } else {
                        ("", theme::TEXT_3)
                    };
                    if !label.is_empty() {
                        ui.label(RichText::new(label).size(12.5).color(color));
                    }
                });
            });
        }
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

        egui::Area::new(Id::unique("done"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.set_width(CARD_WIDTH);
                theme::card(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let scale = self.check_spring.value().clamp(0.0, 1.15);
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(84.0), Sense::hover());
                        if scale > 0.02 {
                            let painter = ui.painter();
                            painter.circle_filled(
                                rect.center(),
                                32.0 * scale,
                                theme::SUCCESS.gamma_multiply(0.16),
                            );
                            painter.circle_filled(rect.center(), 26.0 * scale, theme::SUCCESS);
                            theme::check_mark(painter, rect.center(), 15.0 * scale, Color32::WHITE);
                        }
                        ui.add_space(12.0);
                        ui.label(theme::display(26.0, "Canción lista"));
                        ui.add_space(6.0);
                        ui.label(theme::title(song_label_parts(&song.artist, &song.title)));
                    });

                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        theme::inset(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(theme::caption(if song.copied_to.is_some() {
                                    "Copiada en"
                                } else {
                                    "Guardada en"
                                }));
                                ui.add_space(2.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(song.display_folder().display().to_string())
                                            .size(12.5)
                                            .monospace()
                                            .color(theme::TEXT_2),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    });

                    if let Some(error) = &song.copy_error {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!("No se pudo copiar al destino: {error}"))
                                .size(12.5)
                                .color(theme::WARNING),
                        );
                    }

                    ui.add_space(20.0);
                    if theme::primary_button(ui, "Abrir carpeta").clicked() {
                        open = true;
                    }
                    ui.add_space(8.0);
                    if theme::secondary_button(ui, "Revisar y ajustar").clicked() {
                        review = true;
                    }
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if theme::ghost_button(ui, "Procesar otra canción").clicked() {
                            again = true;
                        }
                    });
                });
            });

        if open {
            if let Err(error) = setup::open_folder(song.display_folder()) {
                self.notify(format!("No se pudo abrir la carpeta: {error}"));
            }
        }
        if review {
            self.screen = Screen::Review;
        }
        if again {
            self.new_song();
        }
    }

    fn gear_area(&mut self, ctx: &egui::Context) {
        egui::Area::new(Id::unique("gear"))
            .anchor(Align2::RIGHT_TOP, Vec2::new(-22.0, 16.0))
            .show(ctx, |ui| {
                if theme::icon_button(ui, "⚙", "Ajustes").clicked() {
                    self.open_settings();
                }
            });
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some((message, at)) = self.toast.clone() else {
            return;
        };
        let age = at.elapsed().as_secs_f32();
        if age >= TOAST_SECONDS {
            self.toast = None;
            return;
        }
        let alpha = if age < 0.18 {
            age / 0.18
        } else if age > 2.5 {
            (1.0 - (age - 2.5) / 0.5).clamp(0.0, 1.0)
        } else {
            1.0
        };
        egui::Area::new(Id::unique("toast"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -26.0))
            .show(ctx, |ui| {
                ui.multiply_opacity(alpha);
                egui::Frame::NONE
                    .fill(theme::SURFACE_2)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(20)
                    .inner_margin(Margin::symmetric(18, 10))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(120),
                    })
                    .show(ui, |ui| {
                        ui.label(RichText::new(message).size(13.5));
                    });
            });
    }

    fn settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let frame = egui::Frame::NONE
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(20)
            .inner_margin(Margin::same(28))
            .shadow(egui::Shadow {
                offset: [0, 20],
                blur: 50,
                spread: 0,
                color: Color32::from_black_alpha(170),
            });

        let mut close = false;
        let modal = egui::Modal::new(Id::unique("settings_modal"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(520.0);
                ui.label(theme::display(24.0, "Ajustes"));
                ui.add_space(4.0);
                ui.label(theme::secondary("Solo lo imprescindible. El resto se decide solo."));
                ui.add_space(26.0);

                ui.label(theme::strong("Carpeta de destino"));
                ui.add_space(3.0);
                ui.add(
                    egui::Label::new(theme::caption(
                        "Cada canción terminada se copia aquí. Normalmente es la carpeta songs de UltraStar.",
                    ))
                    .wrap(),
                );
                ui.add_space(10.0);
                let width = (ui.available_width() - 104.0).max(180.0);
                ui.horizontal(|ui| {
                    theme::text_field_sized(
                        ui,
                        &mut self.destination_text,
                        "Sin definir",
                        width,
                        14.0,
                        10.0,
                    );
                    if theme::button_sized(
                        ui,
                        "Elegir…",
                        ButtonKind::Secondary,
                        Vec2::new(94.0, 40.0),
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
                    ui.add_space(4.0);
                    if theme::ghost_button(ui, "Quitar destino").clicked() {
                        self.destination_text.clear();
                        self.apply_destination();
                    }
                }

                ui.add_space(22.0);
                ui.separator();
                ui.add_space(14.0);
                ui.label(theme::strong("Letra y modelos"));
                ui.add_space(3.0);
                ui.label(theme::caption(
                    "La letra se busca en línea. Whisper solo se descarga si no se encuentra.",
                ));
                ui.add_space(10.0);
                model_row(
                    ui,
                    "Separación de voces",
                    "HTDemucs · 302 MB · siempre necesario",
                    self.models.separation_ready,
                );
                model_row(
                    ui,
                    "Transcripción (respaldo)",
                    "Whisper large-v3-turbo · 547 MB · solo si falta la letra",
                    self.models.whisper_ready,
                );

                ui.add_space(26.0);
                if theme::primary_button(ui, "Listo").clicked() {
                    close = true;
                }
            });

        if close || modal.should_close() {
            self.apply_destination();
            self.show_settings = false;
        }
    }

    fn close_modal(&mut self, ctx: &egui::Context) {
        if !self.confirm_close {
            return;
        }
        let busy = self.job.is_some();
        let mut exit = false;
        let mut save_exit = false;
        let mut cancel = false;

        let modal = egui::Modal::new(Id::unique("close_modal"))
            .frame(
                egui::Frame::NONE
                    .fill(theme::SURFACE)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .corner_radius(20)
                    .inner_margin(Margin::same(26)),
            )
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.label(theme::display(
                    22.0,
                    if busy {
                        "Hay un proceso en marcha"
                    } else {
                        "Hay cambios sin guardar"
                    },
                ));
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(theme::secondary(if busy {
                        "Si sales ahora, la canción quedará a medias. Podrás volver a procesarla."
                    } else {
                        "Guarda la letra y las notas antes de cerrar."
                    }))
                    .wrap(),
                );
                ui.add_space(20.0);
                if busy {
                    if theme::primary_button(ui, "Seguir procesando").clicked() {
                        cancel = true;
                    }
                    ui.add_space(8.0);
                    if theme::secondary_button(ui, "Salir igualmente").clicked() {
                        exit = true;
                    }
                } else {
                    if theme::primary_button(ui, "Guardar y salir").clicked() {
                        save_exit = true;
                    }
                    ui.add_space(8.0);
                    if theme::secondary_button(ui, "Descartar cambios").clicked() {
                        exit = true;
                    }
                    ui.add_space(6.0);
                    ui.vertical_centered(|ui| {
                        if theme::ghost_button(ui, "Cancelar").clicked() {
                            cancel = true;
                        }
                    });
                }
            });

        if modal.should_close() {
            cancel = true;
        }
        if cancel {
            self.confirm_close = false;
        }
        if save_exit {
            self.save_document();
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
// Review editor
// ---------------------------------------------------------------------------

impl App {
    fn review_screen(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let mut leave = false;
        let mut export = false;
        let mut save = false;
        let mut undo = false;
        let mut redo = false;
        let mut settings = false;

        ui.horizontal(|ui| {
            if theme::ghost_button(ui, "‹ Volver").clicked() {
                leave = true;
            }
            ui.add_space(4.0);
            if let Some(session) = &self.session {
                ui.vertical(|ui| {
                    ui.label(theme::title(song_label(&session.project)));
                    ui.label(theme::caption(format!(
                        "{} · {} notas",
                        session
                            .project
                            .source
                            .duration_ms
                            .map(format_duration)
                            .unwrap_or_else(|| "audio".into()),
                        self.editor
                            .as_ref()
                            .map(|editor| editor.document().note_count())
                            .unwrap_or(0)
                    )));
                });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::button_sized(
                    ui,
                    "Exportar",
                    ButtonKind::Primary,
                    Vec2::new(110.0, 36.0),
                    true,
                )
                .clicked()
                {
                    export = true;
                }
                if theme::button_sized(
                    ui,
                    "Guardar",
                    ButtonKind::Secondary,
                    Vec2::new(94.0, 36.0),
                    true,
                )
                .clicked()
                {
                    save = true;
                }
                if theme::icon_button(ui, "↷", "Rehacer").clicked() {
                    redo = true;
                }
                if theme::icon_button(ui, "↶", "Deshacer").clicked() {
                    undo = true;
                }
                if theme::icon_button(ui, "⚙", "Ajustes").clicked() {
                    settings = true;
                }
            });
        });

        if leave {
            self.save_document();
            self.screen = Screen::Done;
            return;
        }
        if save {
            self.save_document();
        }
        if export {
            self.export();
        }
        if settings {
            self.open_settings();
        }
        if undo || redo {
            if let Some(editor) = &mut self.editor {
                if undo {
                    editor.undo();
                }
                if redo {
                    editor.redo();
                }
            }
        }

        ui.add_space(12.0);

        let Some(mut editor) = self.editor.take() else {
            ui.centered_and_justified(|ui| {
                ui.label(theme::secondary(
                    "Sin canción: procesa un proyecto para generar notas.",
                ));
            });
            return;
        };

        let mut message: Option<String> = None;
        theme::inset(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(theme::caption("BPM"));
                let mut bpm = editor.header().bpm;
                if ui
                    .add(
                        egui::DragValue::new(&mut bpm)
                            .speed(0.5)
                            .range(1.0..=2000.0),
                    )
                    .changed()
                {
                    if let Err(error) = editor.set_bpm(bpm, Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
                ui.add_space(6.0);
                ui.label(theme::caption("GAP"));
                let mut gap = editor.header().gap_ms;
                if ui
                    .add(
                        egui::DragValue::new(&mut gap)
                            .speed(1.0)
                            .range(-600_000..=600_000)
                            .suffix(" ms"),
                    )
                    .changed()
                {
                    if let Err(error) = editor.set_gap_ms(gap, Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
                ui.add_space(6.0);
                ui.label(theme::caption("Zoom"));
                ui.add(
                    egui::Slider::new(&mut self.view_span_s, 2.0..=120.0)
                        .suffix(" s")
                        .show_value(false),
                );
            });
        });

        if let Some(error) = &editor.last_message {
            ui.add_space(6.0);
            ui.label(RichText::new(error).size(12.5).color(theme::DANGER));
        }

        if !self.tempo_candidates.is_empty() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(theme::caption("Tempo"));
                let candidates = self.tempo_candidates.clone();
                let mut applied: Option<(f64, i64)> = None;
                for (bpm_usdx, gap_ms) in candidates.iter().copied() {
                    let selected = (editor.header().bpm - bpm_usdx).abs() < 0.01
                        && editor.header().gap_ms == gap_ms;
                    if theme::pill(ui, &format!("{:.1} · {} ms", bpm_usdx, gap_ms), selected)
                        .clicked()
                    {
                        applied = Some((bpm_usdx, gap_ms));
                    }
                }
                if let Some((bpm, gap)) = applied {
                    if let Err(error) = editor.apply_tempo_candidate(bpm, gap, Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
            });
        }

        ui.add_space(10.0);
        self.waveform_ui(ui);
        ui.add_space(10.0);
        self.piano_roll_ui(ui, &mut editor);
        self.inspector_ui(ui, &mut editor);

        if let Some(message) = message {
            self.notify(message);
        }
        self.editor = Some(editor);
    }

    fn waveform_ui(&mut self, ui: &mut egui::Ui) {
        let desired = egui::vec2(ui.available_width(), 96.0);
        let (response, painter) = ui.allocate_painter(desired, egui::Sense::click());
        let rect = response.rect;
        painter.rect_filled(rect, 12, theme::FIELD);

        let duration = self
            .peaks
            .as_ref()
            .map(PeakPyramid::duration_s)
            .unwrap_or(0.0)
            .max(0.001);
        let span = self.view_span_s.min(duration).max(0.5);
        let start = self.view_start_s.clamp(0.0, (duration - span).max(0.0));
        let end = start + span;

        if let Some(peaks) = &self.peaks {
            let width = rect.width().max(16.0) as usize;
            let columns = peaks.columns(start, end, width);
            let mid = rect.center().y;
            let half = rect.height() * 0.4;
            for (index, peak) in columns.iter().enumerate() {
                let x = rect.left() + index as f32;
                let top = mid - peak.max.clamp(-1.0, 1.0) * half;
                let bottom = mid - peak.min.clamp(-1.0, 1.0) * half;
                painter.line_segment(
                    [egui::pos2(x, top), egui::pos2(x, bottom)],
                    Stroke::new(1.0, theme::WAVE.gamma_multiply(0.75)),
                );
            }
        }

        if response.clicked() {
            if let Some(position) = response.interact_pointer_pos() {
                let fraction = ((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                let seconds = start + fraction * span;
                self.engine.seek(seconds);
            }
        }

        let position = self.engine.clock().position_s();
        if position >= start && position <= end {
            let x = rect.left() + ((position - start) / span) as f32 * rect.width();
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                Stroke::new(1.5, Color32::WHITE),
            );
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if theme::icon_button(ui, "⏮", "Al principio").clicked() {
                self.engine.seek(0.0);
            }
            let label = if self.engine.clock().is_playing() {
                "⏸"
            } else {
                "▶"
            };
            if theme::icon_button(ui, label, "Reproducir").clicked() {
                self.engine.toggle();
            }
            if theme::icon_button(ui, "⏹", "Detener").clicked() {
                self.engine.pause();
                self.engine.seek(0.0);
            }
            ui.add_space(6.0);
            for stem in Stem::ALL {
                let selected = self.engine.clock().stem() == stem;
                if theme::pill(ui, stem.label(), selected).clicked() {
                    self.switch_stem(stem);
                }
            }
            ui.add_space(6.0);
            if theme::pill(ui, "bucle", self.engine.clock().is_looping()).clicked() {
                if self.engine.clock().is_looping() {
                    self.engine.set_loop(None);
                } else {
                    let start = self.engine.clock().position_s();
                    self.engine
                        .set_loop(Some(LoopRange::new(start, start + 4.0)));
                }
            }
        });
    }

    fn piano_roll_ui(&mut self, ui: &mut egui::Ui, editor: &mut EditorState) {
        let desired = egui::vec2(ui.available_width(), 232.0);
        let (response, painter) = ui.allocate_painter(desired, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 12, theme::FIELD);

        let header = editor.header().clone();
        let beat_ms = header.beat_duration_ms().max(1.0);
        let start_s = self.view_start_s;
        let span = self.view_span_s.max(1.0);
        let visible_beats = span * 1000.0 / beat_ms;

        let beats_to_x = |beat: f64| {
            rect.left() + (((beat * beat_ms / 1000.0) - start_s) / span) as f32 * rect.width()
        };
        let x_to_beats = |x: f32| {
            let seconds = start_s + ((x - rect.left()) / rect.width()) as f64 * span;
            seconds * 1000.0 / beat_ms
        };

        let min_pitch = editor
            .document()
            .notes()
            .map(|note| note.pitch)
            .min()
            .unwrap_or(-5)
            .min(-5);
        let max_pitch = editor
            .document()
            .notes()
            .map(|note| note.pitch)
            .max()
            .unwrap_or(10)
            .max(min_pitch + 12);
        let pitch_to_y = |pitch: i16| {
            rect.bottom()
                - ((pitch - min_pitch) as f32 / (max_pitch - min_pitch).max(1) as f32)
                    * rect.height()
        };

        let first_beat = (start_s * 1000.0 / beat_ms).floor();
        let last_beat = first_beat + visible_beats.ceil() + 1.0;
        let mut beat = first_beat;
        while beat <= last_beat {
            let x = beats_to_x(beat);
            if x >= rect.left() && x <= rect.right() {
                let strong = (beat as i64) % 4 == 0;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    Stroke::new(
                        1.0,
                        if strong {
                            Color32::from_rgb(0x2A, 0x2A, 0x33)
                        } else {
                            Color32::from_rgb(0x1C, 0x1C, 0x22)
                        },
                    ),
                );
            }
            beat += 1.0;
        }

        let selection = editor.selection();
        let mut clicked_note: Option<(usize, usize)> = None;
        let mut drag_note: Option<(usize, usize, u32)> = None;

        for (phrase_index, phrase) in editor.document().phrases.iter().enumerate() {
            for (note_index, note) in phrase.notes.iter().enumerate() {
                let x0 = beats_to_x(note.start_beat as f64);
                let x1 = beats_to_x(note.end_beat() as f64);
                let y = pitch_to_y(note.pitch);
                let height = (rect.height() / (max_pitch - min_pitch).max(1) as f32).max(8.0);
                let note_rect = Rect::from_min_max(
                    egui::pos2(x0, y - height / 2.0),
                    egui::pos2(x1.max(x0 + 3.0), y + height / 2.0),
                );
                if note_rect.intersects(rect) && !note.is_continuation() {
                    let selected = selection
                        == Some(NoteSelection {
                            phrase: phrase_index,
                            note: note_index,
                        });
                    let doubtful = note.confidence.map(|value| value < 0.55).unwrap_or(false);
                    let color = if selected {
                        theme::ORANGE
                    } else if doubtful {
                        theme::DANGER
                    } else {
                        Color32::from_rgb(0x3D, 0x9B, 0xFF)
                    };
                    painter.rect_filled(note_rect, 3.0, color);
                    if !note.text.is_empty() {
                        painter.text(
                            note_rect.center(),
                            Align2::CENTER_CENTER,
                            &note.text,
                            FontId::proportional(10.5),
                            Color32::from_rgb(0x08, 0x0C, 0x14),
                        );
                    }
                } else if note.is_continuation() && note_rect.intersects(rect) {
                    painter.rect_stroke(
                        note_rect,
                        3.0,
                        Stroke::new(1.0, Color32::from_rgb(0x55, 0x55, 0x60)),
                        StrokeKind::Inside,
                    );
                }

                if response.clicked() {
                    if let Some(position) = response.interact_pointer_pos() {
                        if note_rect.expand(2.0).contains(position) {
                            clicked_note = Some((phrase_index, note_index));
                        }
                    }
                }
                if response.dragged()
                    && selection
                        == Some(NoteSelection {
                            phrase: phrase_index,
                            note: note_index,
                        })
                {
                    if let Some(position) = response.interact_pointer_pos() {
                        drag_note = Some((
                            phrase_index,
                            note_index,
                            x_to_beats(position.x).max(0.0) as u32,
                        ));
                    }
                }
            }

            if let Some(break_marker) = &phrase.break_after {
                if let Some(last) = phrase.notes.last() {
                    let x = beats_to_x(last.end_beat() as f64);
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        Stroke::new(1.0, theme::ORANGE.gamma_multiply(0.7)),
                    );
                    let _ = break_marker;
                }
            }
        }

        if let Some((phrase, note)) = clicked_note {
            let _ = editor.select(phrase, note);
        }
        if let Some((phrase, note, beat)) = drag_note {
            if let Err(error) = editor.move_note(phrase, note, beat, Instant::now()) {
                self.notify(error.to_string());
            }
        }

        let position = self.engine.clock().position_s();
        let cursor_x = beats_to_x(position * 1000.0 / beat_ms);
        if cursor_x >= rect.left() && cursor_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(cursor_x, rect.top()),
                    egui::pos2(cursor_x, rect.bottom()),
                ],
                Stroke::new(1.5, Color32::from_white_alpha(210)),
            );
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui, editor: &mut EditorState) {
        let Some(selection) = editor.selection() else {
            ui.add_space(6.0);
            ui.label(theme::caption(
                "Selecciona una nota en el piano-roll para editarla.",
            ));
            return;
        };
        let Some(note) = editor.selected_note().cloned() else {
            return;
        };
        let mut message: Option<String> = None;
        let mut reload = false;

        ui.add_space(8.0);
        theme::inset(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(theme::caption(format!(
                    "Frase {} · nota {} · beat {} · {} beats · tono {}",
                    selection.phrase,
                    selection.note,
                    note.start_beat,
                    note.length_beats,
                    note.pitch
                )));
                if note.is_continuation() {
                    ui.label(RichText::new("continuación").size(12.5).color(theme::WAVE));
                }
                ui.separator();
                let mut text = note.text.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(120.0)
                            .font(FontId::proportional(13.5)),
                    )
                    .lost_focus()
                    && text != note.text
                {
                    if let Err(error) = editor.set_text(text, Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
                if theme::pill(ui, "↑", false).clicked() {
                    let _ = editor.bump_pitch(1, Instant::now());
                }
                if theme::pill(ui, "↓", false).clicked() {
                    let _ = editor.bump_pitch(-1, Instant::now());
                }
                if theme::pill(ui, "− beat", false).clicked() {
                    match editor.resize_note(
                        selection.phrase,
                        selection.note,
                        note.length_beats.saturating_sub(1),
                        Instant::now(),
                    ) {
                        Ok(length) => message = Some(format!("Longitud {length}")),
                        Err(error) => message = Some(error.to_string()),
                    }
                }
                if theme::pill(ui, "+ beat", false).clicked() {
                    match editor.resize_note(
                        selection.phrase,
                        selection.note,
                        note.length_beats + 1,
                        Instant::now(),
                    ) {
                        Ok(length) => message = Some(format!("Longitud {length}")),
                        Err(error) => message = Some(error.to_string()),
                    }
                }
                if theme::pill(ui, "←", false).clicked() {
                    let _ = editor.move_note(
                        selection.phrase,
                        selection.note,
                        note.start_beat.saturating_sub(1),
                        Instant::now(),
                    );
                }
                if theme::pill(ui, "→", false).clicked() {
                    let _ = editor.move_note(
                        selection.phrase,
                        selection.note,
                        note.start_beat + 1,
                        Instant::now(),
                    );
                }
                if theme::pill(ui, "~", false).clicked() {
                    let _ = editor.toggle_continuation(Instant::now());
                }
                if theme::pill(ui, "Borrar", false).clicked() {
                    if let Err(error) = editor.delete_selected(Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if theme::pill(ui, "+ nota", false).clicked() {
                    let phrase = selection.phrase;
                    let start = editor
                        .document()
                        .phrases
                        .get(phrase)
                        .and_then(|phrase| phrase.notes.last())
                        .map(|note| note.end_beat() + 1)
                        .unwrap_or(0);
                    if let Err(error) = editor.add_note(phrase, start, 1, 0, Instant::now()) {
                        message = Some(error.to_string());
                    }
                }
                if theme::pill(ui, "Recargar", false).clicked() {
                    reload = true;
                }
            });
        });

        if let Some(message) = message {
            self.notify(message);
        }
        if reload {
            self.load_editor();
        }
    }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

impl App {
    /// Space, arrows, `+`/`-`, `Supr`, `Ctrl+Z`/`Ctrl+Y`. Text fields keep the
    /// keyboard while focused.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (toggle, left, right, up, down, delete, undo, redo, rewind, forward) =
            ctx.input(|input| {
                (
                    input.key_pressed(egui::Key::Space),
                    input.key_pressed(egui::Key::ArrowLeft) && !input.modifiers.shift,
                    input.key_pressed(egui::Key::ArrowRight) && !input.modifiers.shift,
                    input.key_pressed(egui::Key::ArrowUp),
                    input.key_pressed(egui::Key::ArrowDown),
                    input.key_pressed(egui::Key::Delete),
                    input.modifiers.command && input.key_pressed(egui::Key::Z),
                    input.modifiers.command && input.key_pressed(egui::Key::Y),
                    input.key_pressed(egui::Key::ArrowLeft) && input.modifiers.shift,
                    input.key_pressed(egui::Key::ArrowRight) && input.modifiers.shift,
                )
            });

        if toggle {
            self.engine.toggle();
        }
        if rewind {
            let position = (self.engine.clock().position_s() - 2.0).max(0.0);
            self.engine.seek(position);
        }
        if forward {
            let position = self.engine.clock().position_s() + 2.0;
            self.engine.seek(position);
        }
        let mut message = None;
        {
            let Some(editor) = &mut self.editor else {
                return;
            };
            if left {
                editor.select_relative(false);
            }
            if right {
                editor.select_relative(true);
            }
            if up {
                let _ = editor.bump_pitch(1, Instant::now());
            }
            if down {
                let _ = editor.bump_pitch(-1, Instant::now());
            }
            if delete {
                if let Err(error) = editor.delete_selected(Instant::now()) {
                    message = Some(error.to_string());
                }
            }
            if undo {
                editor.undo();
            }
            if redo {
                editor.redo();
            }
        }
        if let Some(message) = message {
            self.notify(message);
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

fn stage_name(stage: StageId) -> &'static str {
    match stage {
        StageId::Ingest => "Descargar",
        StageId::Separate => "Separar voces",
        StageId::Transcribe => "Buscar letra",
        StageId::Pitch => "Detectar notas",
        StageId::Align => "Alinear",
        StageId::Export => "Exportar",
    }
}

fn stage_description(stage: StageId) -> &'static str {
    match stage {
        StageId::Ingest => "Descargando el audio y preparando el análisis…",
        StageId::Separate => "Separando la voz de la música…",
        StageId::Transcribe => "Buscando la letra; si no aparece, se transcribe…",
        StageId::Pitch => "Detectando las notas de la melodía…",
        StageId::Align => "Colocando cada sílaba en su nota…",
        StageId::Export => "Escribiendo la carpeta de UltraStar…",
    }
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

fn format_duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn model_row(ui: &mut Ui, name: &str, detail: &str, ready: bool) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            4.0,
            if ready { theme::SUCCESS } else { theme::TEXT_3 },
        );
        ui.vertical(|ui| {
            ui.label(theme::secondary(name));
            ui.label(theme::caption(detail));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (label, color) = if ready {
                ("Listo", theme::SUCCESS)
            } else {
                ("Pendiente", theme::TEXT_3)
            };
            ui.label(RichText::new(label).size(12.5).color(color));
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
        ..ProjectSettings::default()
    }
}

fn playback_message(error: PlaybackError) -> String {
    format!("Reproducción: {error}")
}
