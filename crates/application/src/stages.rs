use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Pipeline stages, in dependency order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageId {
    Ingest,
    Separate,
    Transcribe,
    Pitch,
    Align,
    Export,
}

impl StageId {
    pub const ALL: [StageId; 6] = [
        StageId::Ingest,
        StageId::Separate,
        StageId::Transcribe,
        StageId::Pitch,
        StageId::Align,
        StageId::Export,
    ];

    /// Stages invalidated when this stage's inputs or configuration change.
    /// The first element is the stage itself.
    pub fn dependents(self) -> &'static [StageId] {
        match self {
            StageId::Ingest => &[
                StageId::Ingest,
                StageId::Separate,
                StageId::Transcribe,
                StageId::Pitch,
                StageId::Align,
                StageId::Export,
            ],
            StageId::Separate => &[
                StageId::Separate,
                StageId::Transcribe,
                StageId::Pitch,
                StageId::Align,
                StageId::Export,
            ],
            StageId::Transcribe => &[StageId::Transcribe, StageId::Align, StageId::Export],
            StageId::Pitch => &[StageId::Pitch, StageId::Align, StageId::Export],
            StageId::Align => &[StageId::Align, StageId::Export],
            StageId::Export => &[StageId::Export],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            StageId::Ingest => "ingest",
            StageId::Separate => "separate",
            StageId::Transcribe => "transcribe",
            StageId::Pitch => "pitch",
            StageId::Align => "align",
            StageId::Export => "export",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StageTransitionError {
    #[error("stage is {from:?} and cannot transition to {to:?}")]
    Invalid { from: StageStatus, to: StageStatus },
}

/// Persistent state of one pipeline stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StageState {
    pub status: StageStatus,
    pub input_hash: Option<String>,
    pub config_hash: Option<String>,
    pub error: Option<String>,
    pub started_at_unix_ms: Option<u64>,
    pub finished_at_unix_ms: Option<u64>,
    pub metrics: BTreeMap<String, f64>,
}

impl Default for StageState {
    fn default() -> Self {
        Self {
            status: StageStatus::Pending,
            input_hash: None,
            config_hash: None,
            error: None,
            started_at_unix_ms: None,
            finished_at_unix_ms: None,
            metrics: BTreeMap::new(),
        }
    }
}

impl StageState {
    pub fn start(&mut self, now_unix_ms: u64) -> Result<(), StageTransitionError> {
        self.transition(StageStatus::Pending, StageStatus::Running)?;
        self.started_at_unix_ms = Some(now_unix_ms);
        self.finished_at_unix_ms = None;
        self.error = None;
        Ok(())
    }

    pub fn succeed(&mut self, now_unix_ms: u64) -> Result<(), StageTransitionError> {
        self.transition(StageStatus::Running, StageStatus::Done)?;
        self.finished_at_unix_ms = Some(now_unix_ms);
        self.error = None;
        Ok(())
    }

    /// Marks the stage done and records the fingerprints of its inputs/config.
    pub fn succeed_with_hashes(
        &mut self,
        now_unix_ms: u64,
        input_hash: impl Into<String>,
        config_hash: impl Into<String>,
    ) -> Result<(), StageTransitionError> {
        self.succeed(now_unix_ms)?;
        self.input_hash = Some(input_hash.into());
        self.config_hash = Some(config_hash.into());
        Ok(())
    }

    pub fn fail(
        &mut self,
        message: impl Into<String>,
        now_unix_ms: u64,
    ) -> Result<(), StageTransitionError> {
        self.transition(StageStatus::Running, StageStatus::Failed)?;
        self.finished_at_unix_ms = Some(now_unix_ms);
        self.error = Some(message.into());
        Ok(())
    }

    /// Cancellation is not a failure: the stage returns to pending.
    pub fn cancel(&mut self) -> Result<(), StageTransitionError> {
        self.transition(StageStatus::Running, StageStatus::Pending)?;
        self.started_at_unix_ms = None;
        self.finished_at_unix_ms = None;
        self.error = None;
        Ok(())
    }

    pub fn skip(&mut self) -> Result<(), StageTransitionError> {
        self.transition(StageStatus::Pending, StageStatus::Skipped)?;
        self.started_at_unix_ms = None;
        self.finished_at_unix_ms = None;
        self.error = None;
        Ok(())
    }

    pub fn invalidate(&mut self) -> Result<(), StageTransitionError> {
        if self.status == StageStatus::Running {
            return Err(StageTransitionError::Invalid {
                from: StageStatus::Running,
                to: StageStatus::Pending,
            });
        }
        self.status = StageStatus::Pending;
        self.started_at_unix_ms = None;
        self.finished_at_unix_ms = None;
        self.error = None;
        self.metrics.clear();
        Ok(())
    }

    pub fn is_done(&self) -> bool {
        matches!(self.status, StageStatus::Done | StageStatus::Skipped)
    }

    fn transition(
        &mut self,
        from: StageStatus,
        to: StageStatus,
    ) -> Result<(), StageTransitionError> {
        if self.status != from {
            return Err(StageTransitionError::Invalid {
                from: self.status,
                to,
            });
        }
        self.status = to;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions() {
        let mut stage = StageState::default();
        stage.start(1).unwrap();
        assert_eq!(stage.status, StageStatus::Running);
        stage.succeed(2).unwrap();
        assert_eq!(stage.status, StageStatus::Done);
        assert_eq!(stage.finished_at_unix_ms, Some(2));
        assert!(stage.is_done());
    }

    #[test]
    fn hashes_are_recorded_on_success() {
        let mut stage = StageState::default();
        stage.start(1).unwrap();
        stage
            .succeed_with_hashes(2, "input-hash", "config-hash")
            .unwrap();
        assert_eq!(stage.status, StageStatus::Done);
        assert_eq!(stage.input_hash.as_deref(), Some("input-hash"));
        assert_eq!(stage.config_hash.as_deref(), Some("config-hash"));
    }

    #[test]
    fn failure_and_retry() {
        let mut stage = StageState::default();
        stage.start(1).unwrap();
        stage.fail("boom", 2).unwrap();
        assert_eq!(stage.status, StageStatus::Failed);
        assert_eq!(stage.error.as_deref(), Some("boom"));
        stage.invalidate().unwrap();
        assert_eq!(stage.status, StageStatus::Pending);
        stage.start(3).unwrap();
        assert_eq!(stage.status, StageStatus::Running);
    }

    #[test]
    fn cancellation_returns_to_pending() {
        let mut stage = StageState::default();
        stage.start(1).unwrap();
        stage.cancel().unwrap();
        assert_eq!(stage.status, StageStatus::Pending);
        assert!(stage.error.is_none());
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut stage = StageState::default();
        assert!(stage.succeed(1).is_err());
        stage.start(1).unwrap();
        assert!(stage.start(2).is_err());
        assert!(stage.invalidate().is_err());
    }

    #[test]
    fn skip_is_terminal_until_invalidated() {
        let mut stage = StageState::default();
        stage.skip().unwrap();
        assert_eq!(stage.status, StageStatus::Skipped);
        assert!(stage.is_done());
        stage.invalidate().unwrap();
        assert_eq!(stage.status, StageStatus::Pending);
    }

    #[test]
    fn invalidation_matrix_matches_the_design() {
        assert_eq!(StageId::Ingest.dependents().len(), 6);
        assert_eq!(
            StageId::Transcribe.dependents(),
            &[StageId::Transcribe, StageId::Align, StageId::Export]
        );
        assert_eq!(
            StageId::Pitch.dependents(),
            &[StageId::Pitch, StageId::Align, StageId::Export]
        );
        assert_eq!(
            StageId::Align.dependents(),
            &[StageId::Align, StageId::Export]
        );
        assert_eq!(StageId::Export.dependents(), &[StageId::Export]);
    }
}
