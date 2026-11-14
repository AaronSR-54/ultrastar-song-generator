//! eframe application: project view, stage actions and the review editor.

use std::path::PathBuf;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use usdx_application::{
    atomic_write, JobError, JobEvent, JobHandle, JobRunner, Pipeline, PipelineConfig,
    PitchArtifacts, Project, ProjectPaths, ProjectSettings, ProjectStore, SourceInfo, StageId,
    StageProgress, StageStatus,
};
use usdx_domain::SongDoc;
use usdx_media::{sanitize_filename, Toolchain};

use crate::editor::{EditorState, ExportCheck};
use crate::playback::{AudioEngine, LoopRange, PlaybackError, Stem};
use crate::waveform::PeakPyramid;

const STAGE_ACTIONS_WIDTH: f32 = 260.0;

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

struct RunningJob {
    handle: JobHandle,
    stage: Option<StageId>,
    result: Arc<Mutex<Option<Project>>>,
    started: Instant,
}

pub struct App {
    runner: JobRunner,
    session: Option<Session>,
    job: Option<RunningJob>,
    editor: Option<EditorState>,
    engine: AudioEngine,
    peaks: Option<PeakPyramid>,
    project_path: String,
    source_url: String,
    separation_model: String,
    whisper_model: String,
    tempo_candidates: Vec<(f64, i64)>,
    status: String,
    progress: f32,
    view_start_s: f64,
    view_span_s: f64,
    confirm_close: bool,
    allow_close: bool,
    last_frame: Instant,
    played_since_load: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            runner: JobRunner::new(),
            session: None,
            job: None,
            editor: None,
            engine: AudioEngine::new(),
            peaks: None,
            project_path: String::new(),
            source_url: String::new(),
            separation_model: std::env::var("USDX_HTDEMUCS_MODEL").unwrap_or_default(),
            whisper_model: std::env::var("USDX_WHISPER_MODEL").unwrap_or_default(),
            tempo_candidates: Vec::new(),
            status: "Abre un proyecto o crea uno nuevo".into(),
            progress: 0.0,
            view_start_s: 0.0,
            view_span_s: 30.0,
            confirm_close: false,
            allow_close: false,
            last_frame: Instant::now(),
            played_since_load: false,
        }
    }

    fn open_project(&mut self) {
        let root = PathBuf::from(self.project_path.trim());
        match ProjectStore::open(root) {
            Ok((store, project)) => {
                let config =
                    PipelineConfig::new(self.separation_model.clone(), self.whisper_model.clone());
                self.session = Some(Session {
                    store,
                    project,
                    config,
                    toolchain: Toolchain::detect(),
                });
                self.load_editor();
                self.load_audio();
                self.load_candidates();
                self.status = "Proyecto abierto".into();
            }
            Err(error) => self.status = format!("No se pudo abrir el proyecto: {error}"),
        }
    }

    fn create_project(&mut self) {
        let url = self.source_url.trim();
        if url.is_empty() {
            self.status = "Introduce una URL de YouTube".into();
            return;
        }
        let root = if self.project_path.trim().is_empty() {
            let slug = sanitize_filename(&format!("project-{}", usdx_application::now_unix_ms()));
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(slug)
        } else {
            PathBuf::from(self.project_path.trim())
        };
        let source = SourceInfo {
            url: url.to_string(),
            ..SourceInfo::default()
        };
        match ProjectStore::create(root.clone(), source, default_settings()) {
            Ok((store, project)) => {
                self.project_path = root.display().to_string();
                let config =
                    PipelineConfig::new(self.separation_model.clone(), self.whisper_model.clone());
                self.session = Some(Session {
                    store,
                    project,
                    config,
                    toolchain: Toolchain::detect(),
                });
                self.editor = None;
                self.peaks = None;
                self.status = "Proyecto creado".into();
            }
            Err(error) => self.status = format!("No se pudo crear el proyecto: {error}"),
        }
    }

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
        let Some(session) = &self.session else {
            return;
        };
        let paths = session.paths();
        let mix = paths.analysis_wav();
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
                if let Err(error) = self.engine.load(&mix) {
                    self.status = playback_message(error);
                }
            }
            Err(error) => {
                self.peaks = None;
                self.status = format!("No se pudo leer el audio de análisis: {error}");
            }
        }
    }

    fn start_stage(&mut self, stage: StageId) {
        if self.job.is_some() {
            self.status = "Ya hay una etapa en ejecución".into();
            return;
        }
        let Some(session) = &self.session else {
            self.status = "Abre un proyecto primero".into();
            return;
        };
        let pipeline = session.pipeline();
        let mut project = session.project.clone();
        let result = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&result);
        let handle = self.runner.spawn(stage, move |context| async move {
            let progress = StageProgress::from_job(&context);
            let outcome = pipeline.run_stage(stage, &mut project, &progress).await;
            if let Ok(mut slot) = slot.lock() {
                *slot = Some(project);
            }
            match outcome {
                Ok(()) => Ok(()),
                Err(usdx_application::PipelineError::Cancelled) => Err(JobError::Cancelled),
                Err(error) => Err(JobError::Failed(error.to_string())),
            }
        });
        match handle {
            Ok(handle) => {
                self.job = Some(RunningJob {
                    handle,
                    stage: Some(stage),
                    result,
                    started: Instant::now(),
                });
                self.status = format!("Ejecutando {}", stage.label());
                self.progress = 0.0;
                if let Some(session) = &mut self.session {
                    let _ = session
                        .project
                        .stage_mut(stage)
                        .start(usdx_application::now_unix_ms());
                }
            }
            Err(error) => self.status = format!("No se pudo iniciar la etapa: {error}"),
        }
    }

    fn start_all(&mut self) {
        if self.job.is_some() {
            self.status = "Ya hay una etapa en ejecución".into();
            return;
        }
        let Some(session) = &self.session else {
            self.status = "Abre un proyecto primero".into();
            return;
        };
        let pipeline = session.pipeline();
        let mut project = session.project.clone();
        let result = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&result);
        let handle = self
            .runner
            .spawn(StageId::Ingest, move |context| async move {
                let progress = StageProgress::from_job(&context);
                let outcome = pipeline.run_pending(&mut project, &progress).await;
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(project);
                }
                match outcome {
                    Ok(_) => Ok(()),
                    Err(usdx_application::PipelineError::Cancelled) => Err(JobError::Cancelled),
                    Err(error) => Err(JobError::Failed(error.to_string())),
                }
            });
        match handle {
            Ok(handle) => {
                self.job = Some(RunningJob {
                    handle,
                    stage: None,
                    result,
                    started: Instant::now(),
                });
                self.status = "Procesando todas las etapas".into();
                self.progress = 0.0;
            }
            Err(error) => self.status = format!("No se pudo iniciar el proceso: {error}"),
        }
    }

    fn poll_job(&mut self) {
        let Some(job) = &mut self.job else {
            return;
        };
        loop {
            match job.handle.try_recv() {
                Some(JobEvent::Progress {
                    stage,
                    fraction,
                    message,
                }) => {
                    self.progress = fraction;
                    self.status =
                        format!("{}: {message} ({:.0}%)", stage.label(), fraction * 100.0);
                }
                Some(JobEvent::Finished(outcome)) => {
                    if let Ok(mut slot) = job.result.lock() {
                        if let Some(project) = slot.take() {
                            if let Some(session) = &mut self.session {
                                session.project = project;
                            }
                        }
                    }
                    self.status = match outcome {
                        Ok(()) => "Etapa completada".into(),
                        Err(JobError::Cancelled) => "Cancelado".into(),
                        Err(JobError::Busy) => "Otra tarea sigue en marcha".into(),
                        Err(JobError::Failed(message)) => format!("Fallo: {message}"),
                    };
                    self.job = None;
                    self.load_editor();
                    self.load_audio();
                    self.load_candidates();
                    return;
                }
                None => return,
            }
        }
    }

    fn cancel_job(&mut self) {
        if let Some(job) = &self.job {
            job.handle.cancel();
            self.status = "Cancelando…".into();
        }
    }

    fn save_document(&mut self) {
        let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
            return;
        };
        let bytes = match serde_json::to_vec_pretty(editor.document()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("No se pudo serializar la canción: {error}");
                return;
            }
        };
        match atomic_write(&session.paths().song(), &bytes) {
            Ok(()) => {
                editor.mark_saved();
                self.status = "Guardado".into();
            }
            Err(error) => self.status = format!("No se pudo guardar: {error}"),
        }
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
        let (Some(session), Some(editor)) = (&self.session, &mut self.editor) else {
            self.status = "No hay canción para exportar".into();
            return;
        };
        if !matches!(editor.check_export(), ExportCheck::Ready) {
            self.status = editor
                .last_message
                .clone()
                .unwrap_or_else(|| "La validación impide exportar".into());
            return;
        }
        let pipeline = session.pipeline();
        match pipeline.export_document(editor.document()) {
            Ok(outcome) => {
                editor.mark_saved();
                self.status = format!("Exportado a {}", outcome.folder.display());
            }
            Err(error) => {
                let message = error.to_string();
                editor.adopt_export_failure(message.clone());
                self.status = message;
            }
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
            self.status = format!("No hay stem {} disponible", stem.label());
            return;
        };
        match self.engine.switch_stem(stem, &path) {
            Ok(()) => self.status = format!("Stem: {}", stem.label()),
            Err(error) => self.status = playback_message(error),
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        let elapsed = self.last_frame.elapsed().as_secs_f64().min(0.25);
        self.last_frame = Instant::now();
        self.engine.poll();
        if let Some(target) = self.engine.clock_mut().advance(elapsed) {
            self.engine.seek(target);
        }
        if self.engine.clock().is_playing() {
            self.played_since_load = true;
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        self.poll_job();
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.autosave_if_needed();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        self.handle_shortcuts(&ctx);

        if ctx.input(|input| input.viewport().close_requested())
            && !self.allow_close
            && self
                .editor
                .as_ref()
                .map(EditorState::is_dirty)
                .unwrap_or(false)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }

        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Proyecto:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.project_path)
                        .desired_width(320.0)
                        .hint_text("ruta del proyecto"),
                );
                if ui.button("Abrir").clicked() {
                    self.open_project();
                }
                ui.separator();
                ui.label("YouTube:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.source_url)
                        .desired_width(320.0)
                        .hint_text("https://www.youtube.com/watch?v=..."),
                );
                if ui.button("Crear proyecto").clicked() {
                    self.create_project();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Modelo separación:");
                ui.add(egui::TextEdit::singleline(&mut self.separation_model).desired_width(280.0));
                ui.label("Modelo Whisper:");
                ui.add(egui::TextEdit::singleline(&mut self.whisper_model).desired_width(280.0));
                if ui.button("Aplicar modelos").clicked() {
                    let config = PipelineConfig::new(
                        self.separation_model.clone(),
                        self.whisper_model.clone(),
                    );
                    if let Some(session) = &mut self.session {
                        session.config = config;
                        self.status = "Modelos actualizados".into();
                    }
                }
            });
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Exportar").clicked() {
                        self.export();
                    }
                    if ui.button("Rehacer").clicked() {
                        if let Some(editor) = &mut self.editor {
                            editor.redo();
                        }
                    }
                    if ui.button("Deshacer").clicked() {
                        if let Some(editor) = &mut self.editor {
                            editor.undo();
                        }
                    }
                    if ui.button("Guardar").clicked() {
                        self.save_document();
                    }
                });
            });
            if let Some(project) = &self.session.as_ref().map(|session| &session.project) {
                ui.label(format!(
                    "{} — {} · audio {}",
                    project.source.artist.clone().unwrap_or_default(),
                    project.source.title,
                    project
                        .source
                        .duration_ms
                        .map(|value| value / 1000)
                        .unwrap_or(0)
                ));
            }
        });

        egui::Panel::left("stages")
            .exact_size(STAGE_ACTIONS_WIDTH)
            .show(ui, |ui| {
                ui.heading("Etapas");
                ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                ui.separator();
                let mut requested: Option<StageId> = None;
                if let Some(session) = &self.session {
                    for stage in StageId::ALL {
                        let state = session.project.stage(stage);
                        ui.horizontal(|ui| {
                            let color = stage_color(state.status);
                            ui.colored_label(color, "●");
                            ui.label(format!("{:<11}", stage.label()));
                            ui.label(format!("{:?}", state.status));
                        });
                        if let Some(error) = &state.error {
                            ui.small(error.clone());
                        }
                        ui.horizontal(|ui| {
                            let enabled = self.job.is_none();
                            if ui
                                .add_enabled(enabled, egui::Button::new("Ejecutar"))
                                .clicked()
                            {
                                requested = Some(stage);
                            }
                            if matches!(state.status, StageStatus::Failed)
                                && ui
                                    .add_enabled(enabled, egui::Button::new("Reintentar"))
                                    .clicked()
                            {
                                requested = Some(stage);
                            }
                        });
                        ui.separator();
                    }
                } else {
                    ui.label("Sin proyecto abierto");
                }
                if self.job.is_none() && ui.button("Procesar todo").clicked() {
                    self.start_all();
                }
                if let Some(job) = &self.job {
                    let stage = job.stage.map(StageId::label).unwrap_or("todas las etapas");
                    ui.label(format!(
                        "En curso: {stage} ({:.0} s)",
                        job.started.elapsed().as_secs_f64()
                    ));
                    if ui.button("Cancelar").clicked() {
                        self.cancel_job();
                    }
                }
                if let Some(stage) = requested {
                    self.start_stage(stage);
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            self.editor_ui(ui);
        });

        if self.confirm_close {
            egui::Window::new("Cerrar")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Hay cambios sin guardar.");
                    ui.horizontal(|ui| {
                        if ui.button("Guardar y cerrar").clicked() {
                            self.save_document();
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Descartar").clicked() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancelar").clicked() {
                            self.confirm_close = false;
                        }
                    });
                });
        }
    }
}

impl App {
    /// Keyboard shortcuts from the design: space, arrows, `+`/`-`, `Supr`,
    /// `Ctrl+Z`/`Ctrl+Y`. Text fields keep the keyboard while focused.
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
                self.status = error.to_string();
            }
        }
        if undo {
            editor.undo();
        }
        if redo {
            editor.redo();
        }
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = &mut self.editor else {
            ui.centered_and_justified(|ui| {
                ui.label("Sin canción: procesa el proyecto para generar notas.");
            });
            return;
        };

        ui.horizontal(|ui| {
            ui.label("BPM:");
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
                    self.status = error.to_string();
                }
            }
            ui.label("GAP (ms):");
            let mut gap = editor.header().gap_ms;
            if ui
                .add(
                    egui::DragValue::new(&mut gap)
                        .speed(1.0)
                        .range(-600_000..=600_000),
                )
                .changed()
            {
                if let Err(error) = editor.set_gap_ms(gap, Instant::now()) {
                    self.status = error.to_string();
                }
            }
            ui.separator();
            ui.label("Zoom:");
            if ui
                .add(egui::Slider::new(&mut self.view_span_s, 2.0..=120.0).suffix(" s"))
                .changed()
            {
                self.view_start_s = self
                    .view_start_s
                    .clamp(0.0, self.engine.clock().duration_s());
            }
        });

        if let Some(error) = editor.last_message.clone() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }

        if !self.tempo_candidates.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label("Tempo:");
                let candidates = self.tempo_candidates.clone();
                let mut applied: Option<(f64, i64)> = None;
                for (bpm_usdx, gap_ms) in candidates.iter().copied() {
                    let selected = (editor.header().bpm - bpm_usdx).abs() < 0.01
                        && editor.header().gap_ms == gap_ms;
                    if ui
                        .selectable_label(selected, format!("{:.1} / {} ms", bpm_usdx, gap_ms))
                        .clicked()
                    {
                        applied = Some((bpm_usdx, gap_ms));
                    }
                }
                if let Some((bpm, gap)) = applied {
                    if let Err(error) = editor.apply_tempo_candidate(bpm, gap, Instant::now()) {
                        self.status = error.to_string();
                    }
                }
            });
        }

        self.waveform_ui(ui);
        self.piano_roll_ui(ui);
        self.inspector_ui(ui);
    }

    fn waveform_ui(&mut self, ui: &mut egui::Ui) {
        let desired = egui::vec2(ui.available_width(), 90.0);
        let (response, painter) = ui.allocate_painter(desired, egui::Sense::click());
        let rect = response.rect;
        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(20, 22, 28));

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
            let half = rect.height() * 0.45;
            for (index, peak) in columns.iter().enumerate() {
                let x = rect.left() + index as f32;
                let top = mid - peak.max.clamp(-1.0, 1.0) * half;
                let bottom = mid - peak.min.clamp(-1.0, 1.0) * half;
                painter.line_segment(
                    [egui::pos2(x, top), egui::pos2(x, bottom)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 150, 220)),
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
                egui::Stroke::new(1.0, egui::Color32::from_rgb(250, 200, 80)),
            );
        }

        ui.horizontal(|ui| {
            if ui.button("⏮").clicked() {
                self.engine.seek(0.0);
            }
            let label = if self.engine.clock().is_playing() {
                "⏸"
            } else {
                "▶"
            };
            if ui.button(label).clicked() {
                self.engine.toggle();
            }
            if ui.button("⏹").clicked() {
                self.engine.pause();
                self.engine.seek(0.0);
            }
            for stem in Stem::ALL {
                let selected = self.engine.clock().stem() == stem;
                if ui.selectable_label(selected, stem.label()).clicked() {
                    self.switch_stem(stem);
                }
            }
            if ui
                .selectable_label(self.engine.clock().is_looping(), "bucle")
                .clicked()
            {
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

    fn piano_roll_ui(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let desired = egui::vec2(ui.available_width(), 240.0);
        let (response, painter) = ui.allocate_painter(desired, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(16, 18, 22));

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

        // Grid: one line per beat.
        let first_beat = (start_s * 1000.0 / beat_ms).floor();
        let last_beat = first_beat + visible_beats.ceil() + 1.0;
        let mut beat = first_beat;
        while beat <= last_beat {
            let x = beats_to_x(beat);
            if x >= rect.left() && x <= rect.right() {
                let strong = (beat as i64) % 4 == 0;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(
                        1.0,
                        if strong {
                            egui::Color32::from_rgb(60, 64, 74)
                        } else {
                            egui::Color32::from_rgb(32, 35, 42)
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
                let note_rect = egui::Rect::from_min_max(
                    egui::pos2(x0, y - height / 2.0),
                    egui::pos2(x1.max(x0 + 3.0), y + height / 2.0),
                );
                if note_rect.intersects(rect) && !note.is_continuation() {
                    let selected = selection
                        == Some(crate::editor::NoteSelection {
                            phrase: phrase_index,
                            note: note_index,
                        });
                    let doubtful = note.confidence.map(|value| value < 0.55).unwrap_or(false);
                    let color = if selected {
                        egui::Color32::from_rgb(250, 170, 70)
                    } else if doubtful {
                        egui::Color32::from_rgb(220, 90, 90)
                    } else {
                        egui::Color32::from_rgb(80, 180, 140)
                    };
                    painter.rect_filled(note_rect, 3.0, color);
                    if !note.text.is_empty() {
                        painter.text(
                            note_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &note.text,
                            egui::FontId::proportional(11.0),
                            egui::Color32::BLACK,
                        );
                    }
                } else if note.is_continuation() && note_rect.intersects(rect) {
                    painter.rect_stroke(
                        note_rect,
                        3.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 120, 130)),
                        egui::StrokeKind::Inside,
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
                        == Some(crate::editor::NoteSelection {
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
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 120, 220)),
                    );
                    let _ = break_marker;
                }
            }
        }

        if let Some((phrase, note)) = clicked_note {
            let _ = editor.select(phrase, note);
        }
        if let Some((phrase, note, beat)) = drag_note {
            if let Ok(applied) = editor.move_note(phrase, note, beat, Instant::now()) {
                self.status = format!("Nota movida al beat {applied}");
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
                egui::Stroke::new(1.0, egui::Color32::from_rgb(250, 200, 80)),
            );
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let Some(selection) = editor.selection() else {
            ui.label("Selecciona una nota en el piano-roll");
            return;
        };
        ui.separator();
        let Some(note) = editor.selected_note().cloned() else {
            ui.label("Selecciona una nota en el piano-roll");
            return;
        };
        let mut message: Option<String> = None;
        let mut reload = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Frase {} nota {} · beat {} · {} beats · pitch {}",
                selection.phrase, selection.note, note.start_beat, note.length_beats, note.pitch
            ));
            if note.is_continuation() {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "continuación");
            }
            ui.separator();
            let mut text = note.text.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut text).desired_width(140.0))
                .lost_focus()
                && text != note.text
            {
                if let Err(error) = editor.set_text(text, Instant::now()) {
                    message = Some(error.to_string());
                }
            }
            if ui.button("⏶").clicked() {
                let _ = editor.bump_pitch(1, Instant::now());
            }
            if ui.button("⏷").clicked() {
                let _ = editor.bump_pitch(-1, Instant::now());
            }
            if ui.button("+ beat").clicked() {
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
            if ui.button("- beat").clicked() {
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
            if ui.button("→").clicked() {
                let _ = editor.move_note(
                    selection.phrase,
                    selection.note,
                    note.start_beat + 1,
                    Instant::now(),
                );
            }
            if ui.button("←").clicked() {
                let _ = editor.move_note(
                    selection.phrase,
                    selection.note,
                    note.start_beat.saturating_sub(1),
                    Instant::now(),
                );
            }
            if ui.button("~").clicked() {
                let _ = editor.toggle_continuation(Instant::now());
            }
            if ui.button("Borrar").clicked() {
                if let Err(error) = editor.delete_selected(Instant::now()) {
                    message = Some(error.to_string());
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ nota").clicked() {
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
            if ui.button("Actualizar documento").clicked() {
                reload = true;
            }
        });
        if let Some(message) = message {
            self.status = message;
        }
        if reload {
            self.load_editor();
        }
    }
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

fn stage_color(status: StageStatus) -> egui::Color32 {
    match status {
        StageStatus::Pending => egui::Color32::GRAY,
        StageStatus::Running => egui::Color32::from_rgb(250, 200, 80),
        StageStatus::Done => egui::Color32::from_rgb(90, 200, 130),
        StageStatus::Failed => egui::Color32::from_rgb(230, 90, 90),
        StageStatus::Skipped => egui::Color32::from_rgb(160, 140, 200),
    }
}

fn playback_message(error: PlaybackError) -> String {
    format!("Reproducción: {error}")
}
