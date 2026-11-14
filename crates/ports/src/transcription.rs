use std::fmt;
use std::path::Path;
use std::sync::Arc;

use usdx_domain::Transcript;

use crate::CancelFlag;

#[derive(Clone)]
pub struct TranscriptionControl {
    pub cancel: CancelFlag,
    pub progress: Arc<dyn Fn(f32) + Send + Sync>,
}

impl TranscriptionControl {
    pub fn new(cancel: CancelFlag, progress: Arc<dyn Fn(f32) + Send + Sync>) -> Self {
        Self { cancel, progress }
    }

    pub fn report(&self, fraction: f32) {
        (self.progress)(fraction.clamp(0.0, 1.0));
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

impl fmt::Debug for TranscriptionControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TranscriptionControl")
            .field("cancel", &self.cancel)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    #[error("transcription failed: {0}")]
    Failed(String),
    #[error("transcription was cancelled")]
    Cancelled,
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("audio has no samples")]
    EmptyAudio,
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
}

pub trait Transcriber: Send + Sync {
    fn transcribe(
        &self,
        vocals: &Path,
        language: Option<&str>,
        control: TranscriptionControl,
    ) -> Result<Transcript, TranscriptionError>;
}
