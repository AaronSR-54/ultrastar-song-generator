//! Port traits that decouple the application layer from concrete tooling.

mod cancel;
mod separation;
mod transcription;

pub use cancel::CancelFlag;
pub use separation::{SeparationControl, SeparationError, StemPaths, StemSeparator};
pub use transcription::{Transcriber, TranscriptionControl, TranscriptionError};
