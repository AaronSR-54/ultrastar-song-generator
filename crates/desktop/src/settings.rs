//! Small preferences file that survives restarts.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowBar {
    /// Native OS title bar, tinted with the app palette where the OS allows it.
    Native,
    /// Chrome drawn by the app: brand, status and window controls.
    #[default]
    Integrated,
    /// Thin app chrome with just the window controls.
    Minimal,
}

impl WindowBar {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Sistema",
            Self::Integrated => "Integrada",
            Self::Minimal => "Mínima",
        }
    }

    pub fn is_native(self) -> bool {
        matches!(self, Self::Native)
    }

    pub fn height(self) -> f32 {
        match self {
            Self::Native => 0.0,
            Self::Integrated => 38.0,
            Self::Minimal => 28.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSong {
    /// Artist and title as the finished song wrote them.
    pub label: String,
    /// Folder the song was exported to.
    pub folder: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiSettings {
    /// Folder where finished songs are copied. Empty means "keep everything
    /// inside the project folder".
    pub destination_dir: Option<PathBuf>,
    /// How the window chrome is presented.
    pub window_bar: WindowBar,
    /// Last songs exported, newest first, three at most.
    pub recent: Vec<RecentSong>,
}

impl GuiSettings {
    /// Remembers one exported song, so the home screen can offer it again.
    /// Keeps three at most, newest first. Saves nothing: the caller decides.
    pub fn remember(&mut self, song: RecentSong) {
        self.recent.retain(|old| old.folder != song.folder);
        self.recent.insert(0, song);
        self.recent.truncate(3);
    }

    pub fn load() -> Self {
        Self::load_from(&settings_path()).unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self) {
        let _ = self.save_to(&settings_path());
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        usdx_application::atomic_write(path, &bytes)
    }
}

pub fn settings_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "ultrastar-song-generator")
        .map(|dirs| dirs.config_dir().join("settings.json"))
        .unwrap_or_else(|| PathBuf::from("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let settings = GuiSettings {
            destination_dir: Some(PathBuf::from("C:\\UltraStar\\songs")),
            window_bar: WindowBar::Minimal,
            recent: vec![RecentSong {
                label: "Queen — Bohemian Rhapsody".into(),
                folder: PathBuf::from("C:\\UltraStar\\songs\\Queen - Bohemian Rhapsody"),
            }],
        };
        settings.save_to(&path).unwrap();
        let loaded = GuiSettings::load_from(&path).unwrap();
        assert_eq!(loaded, settings);
    }

    #[test]
    fn recent_songs_stay_short_and_newest_first() {
        let mut settings = GuiSettings::default();
        for index in 0..5 {
            settings.remember(RecentSong {
                label: format!("Song {index}"),
                folder: PathBuf::from(format!("/songs/{index}")),
            });
        }
        assert_eq!(settings.recent.len(), 3);
        assert_eq!(settings.recent[0].label, "Song 4");
        // Remembering an older folder again moves it to the front, once.
        settings.remember(RecentSong {
            label: "Song 2".into(),
            folder: PathBuf::from("/songs/2"),
        });
        assert_eq!(settings.recent.len(), 3);
        assert_eq!(settings.recent[0].label, "Song 2");
    }

    #[test]
    fn missing_or_corrupt_settings_fall_back_to_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert!(GuiSettings::load_from(&path).is_none());
        std::fs::write(&path, b"not json").unwrap();
        assert!(GuiSettings::load_from(&path).is_none());
    }
}
