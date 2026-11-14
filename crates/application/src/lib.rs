//! Application orchestration: project store, stage runner and invalidation.

pub mod hash;
pub mod jobs;
pub mod project;
pub mod stages;

pub use hash::{hash_bytes, hash_file, hash_json};
pub use jobs::{
    CancelFlag, JobContext, JobError, JobEvent, JobHandle, JobRunner, ProgressReporter,
};
pub use project::{
    atomic_write, now_unix_ms, slugify, Autosave, Project, ProjectError, ProjectPaths,
    ProjectSettings, ProjectStore, SourceInfo, SCHEMA_VERSION,
};
pub use stages::{StageId, StageState, StageStatus, StageTransitionError};
