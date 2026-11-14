//! Application orchestration: project store, stage runner and invalidation.

pub mod hash;
pub mod jobs;
pub mod models;
pub mod pipeline;
pub mod project;
pub mod stages;

pub use hash::{hash_bytes, hash_file, hash_json};
pub use jobs::{
    CancelFlag, JobContext, JobError, JobEvent, JobHandle, JobRunner, ProgressReporter,
};
pub use models::{
    choose_backend, Backend, BackendChoice, BackendPreference, CatalogError, Hardware,
    ModelCatalog, ModelDownloader, ModelError, ModelPurpose, ModelSpec, ModelStore,
};
pub use pipeline::{
    ExportOutcome, Pipeline, PipelineConfig, PipelineError, PitchArtifacts, StageProgress,
    EXPORTED_AUDIO, EXPORTED_COVER, EXPORTED_VIDEO, HEADER_AUDIO,
};
pub use project::{
    atomic_write, now_unix_ms, slugify, Autosave, Project, ProjectError, ProjectPaths,
    ProjectSettings, ProjectStore, SourceInfo, SCHEMA_VERSION,
};
pub use stages::{StageId, StageState, StageStatus, StageTransitionError};
