//! Small preferences file that survives restarts.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiSettings {
    /// Folder where finished songs are copied. Empty means "keep everything
    /// inside the project folder".
    pub destination_dir: Option<PathBuf>,
}

impl GuiSettings {
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
        };
        settings.save_to(&path).unwrap();
        let loaded = GuiSettings::load_from(&path).unwrap();
        assert_eq!(loaded, settings);
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
