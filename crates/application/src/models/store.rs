use std::path::{Path, PathBuf};

use crate::hash::hash_file;

use super::{ModelError, ModelSpec};

#[derive(Debug, Clone)]
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        directories::ProjectDirs::from("", "", "ultrastar-song-generator")
            .map(|dirs| dirs.data_dir().join("models"))
            .unwrap_or_else(|| PathBuf::from("models"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
    }

    pub fn path_for(&self, spec: &ModelSpec) -> PathBuf {
        self.root.join(&spec.filename)
    }

    pub fn part_path(&self, spec: &ModelSpec) -> PathBuf {
        self.root.join(format!("{}.part", spec.filename))
    }

    pub fn ensure_root(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)
    }

    pub fn verify(&self, spec: &ModelSpec) -> std::io::Result<bool> {
        let path = self.path_for(spec);
        if !path.exists() {
            return Ok(false);
        }
        Ok(hash_file(path)? == spec.sha256)
    }

    pub fn is_present(&self, spec: &ModelSpec) -> bool {
        self.verify(spec).unwrap_or(false)
    }

    /// Removes the model and any partial download. Returns whether a file was removed.
    pub fn delete(&self, spec: &ModelSpec) -> std::io::Result<bool> {
        let mut removed = false;
        for path in [self.path_for(spec), self.part_path(spec)] {
            if path.exists() {
                std::fs::remove_file(path)?;
                removed = true;
            }
        }
        Ok(removed)
    }

    pub fn check_space(&self, required: u64) -> Result<u64, ModelError> {
        self.ensure_root()?;
        let available = fs4::available_space(&self.root)?;
        if available < required {
            Err(ModelError::InsufficientSpace {
                required,
                available,
            })
        } else {
            Ok(available)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelPurpose;

    fn spec(content: &[u8]) -> ModelSpec {
        ModelSpec {
            id: "test".into(),
            name: "Test".into(),
            purpose: ModelPurpose::Separation,
            filename: "test.onnx".into(),
            url: "https://example.com/test.onnx".into(),
            size_bytes: content.len() as u64,
            sha256: crate::hash::hash_bytes(content),
            license: "MIT".into(),
            requirements: "CPU".into(),
        }
    }

    #[test]
    fn paths_are_inside_the_store_root() {
        let store = ModelStore::new("models");
        let spec = spec(b"abc");
        assert_eq!(store.path_for(&spec), PathBuf::from("models/test.onnx"));
        assert_eq!(
            store.part_path(&spec),
            PathBuf::from("models/test.onnx.part")
        );
    }

    #[test]
    fn verifies_and_deletes_files() {
        let directory = tempfile::tempdir().unwrap();
        let store = ModelStore::new(directory.path());
        let spec = spec(b"model bytes");
        assert!(!store.is_present(&spec));

        std::fs::write(store.path_for(&spec), b"model bytes").unwrap();
        assert!(store.is_present(&spec));

        std::fs::write(store.part_path(&spec), b"partial").unwrap();
        assert!(store.delete(&spec).unwrap());
        assert!(!store.path_for(&spec).exists());
        assert!(!store.part_path(&spec).exists());
        assert!(!store.delete(&spec).unwrap());
    }

    #[test]
    fn detects_a_corrupt_model() {
        let directory = tempfile::tempdir().unwrap();
        let store = ModelStore::new(directory.path());
        let spec = spec(b"expected");
        std::fs::write(store.path_for(&spec), b"corrupt").unwrap();
        assert!(!store.is_present(&spec));
    }

    #[test]
    fn insufficient_space_is_reported() {
        let directory = tempfile::tempdir().unwrap();
        let store = ModelStore::new(directory.path());
        let error = store.check_space(u64::MAX).unwrap_err();
        assert!(matches!(error, ModelError::InsufficientSpace { .. }));
    }

    #[test]
    fn default_root_is_not_empty() {
        assert!(!ModelStore::default_root().as_os_str().is_empty());
    }
}
