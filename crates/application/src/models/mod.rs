//! Local model catalog, storage, download and backend selection.

mod backend;
mod catalog;
mod download;
mod store;

pub use backend::{choose_backend, Backend, BackendChoice, BackendPreference, Hardware};
pub use catalog::{CatalogError, ModelCatalog, ModelPurpose, ModelSpec};
pub use download::ModelDownloader;
pub use store::ModelStore;

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected HTTP status: {0}")]
    HttpStatus(u16),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("not enough disk space: need {required} bytes, available {available}")]
    InsufficientSpace { required: u64, available: u64 },
    #[error("model download was cancelled")]
    Cancelled,
    #[error("invalid model catalog: {0}")]
    Catalog(#[from] CatalogError),
    #[error("invalid model specification: {0}")]
    InvalidSpec(String),
}
