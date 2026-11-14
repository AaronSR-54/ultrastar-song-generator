//! Desktop editor: project view, waveform, piano roll and export.

pub mod app;
pub mod editor;
pub mod playback;
pub mod waveform;

pub use app::App;
pub use editor::{
    EditorError, EditorState, ExportCheck, NoteSelection, AUTOSAVE_DELAY, UNDO_LIMIT,
};
pub use playback::{AudioEngine, LoopRange, PlaybackClock, PlaybackError, Stem};
pub use waveform::{Peak, PeakPyramid};

/// Starts the eframe application.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("UltraStar Song Generator"),
        ..Default::default()
    };
    eframe::run_native(
        "UltraStar Song Generator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
