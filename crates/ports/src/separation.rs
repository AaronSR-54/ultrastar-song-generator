use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::CancelFlag;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StemPaths {
    pub vocals: PathBuf,
    pub instrumental: PathBuf,
}

#[derive(Clone)]
pub struct SeparationControl {
    pub cancel: CancelFlag,
    pub progress: Arc<dyn Fn(f32) + Send + Sync>,
}

impl SeparationControl {
    pub fn new(cancel: CancelFlag, progress: Arc<dyn Fn(f32) + Send + Sync>) -> Self {
        Self { cancel, progress }
    }

    pub fn report(&self, fraction: f32) {
        (self.progress)(fraction.clamp(0.0, 1.0));
    }
}

impl fmt::Debug for SeparationControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeparationControl")
            .field("cancel", &self.cancel)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SeparationError {
    #[error("separation failed: {0}")]
    Failed(String),
    #[error("separation was cancelled")]
    Cancelled,
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing stem in model output: {0}")]
    MissingStem(String),
    #[error("stem shape mismatch: {0}")]
    ShapeMismatch(String),
}

pub trait StemSeparator: Send + Sync {
    fn separate(
        &self,
        input: &Path,
        output_dir: &Path,
        control: SeparationControl,
    ) -> Result<StemPaths, SeparationError>;
}
