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
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1040.0, 700.0])
        .with_min_inner_size([860.0, 640.0])
        .with_decorations(settings.window_bar.is_native())
        .with_title("UltraStar Song Generator");
    // The packaged app and the installers share this icon. A decode failure is
    // not fatal: the window then falls back to the platform default.
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png")) {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "UltraStar Song Generator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    /// The installers and the window use the same PNG; a broken asset would
    /// silently leave the window with the platform default icon.
    #[test]
    fn embedded_window_icon_decodes() {
        let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png"))
            .expect("assets/icon.png must be a valid PNG");
        assert_eq!(icon.width, 1024);
        assert_eq!(icon.height, 1024);
    }
}
