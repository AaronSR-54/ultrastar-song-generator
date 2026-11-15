//! Desktop app: one-step pipeline, synced preview and export.

pub mod app;
pub mod editor;
pub mod icons;
pub mod native_bar;
pub mod playback;
pub mod settings;
pub mod setup;
pub mod theme;

pub use app::App;
pub use editor::{
    EditorError, EditorState, ExportCheck, NoteSelection, AUTOSAVE_DELAY, UNDO_LIMIT,
};
pub use playback::{AudioEngine, LoopRange, PlaybackClock, PlaybackError, Stem};
pub use settings::{GuiSettings, WindowBar};
pub use setup::FinishedSong;

/// Starts the eframe application.
pub fn run() -> eframe::Result<()> {
    let settings = GuiSettings::load();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([860.0, 640.0])
            .with_decorations(settings.window_bar.is_native())
            .with_title("UltraStar Song Generator"),
        ..Default::default()
    };
    eframe::run_native(
        "UltraStar Song Generator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
