//! Port traits that decouple the application layer from concrete tooling.

mod cancel;
mod separation;

pub use cancel::CancelFlag;
pub use separation::{SeparationControl, SeparationError, StemPaths, StemSeparator};
