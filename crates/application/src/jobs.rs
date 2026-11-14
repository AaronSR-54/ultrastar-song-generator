use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tokio::runtime::Handle;

use crate::stages::StageId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JobError {
    #[error("another heavy job is already running")]
    Busy,
    #[error("job cancelled")]
    Cancelled,
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent {
    Progress {
        stage: StageId,
        fraction: f32,
        message: String,
    },
    Finished(Result<(), JobError>),
}

#[derive(Debug, Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct ProgressReporter {
    stage: StageId,
    sender: Sender<JobEvent>,
}

impl ProgressReporter {
    /// Reports progress in the range 0.0..=1.0.
    pub fn report(&self, fraction: f32, message: impl Into<String>) {
        let fraction = fraction.clamp(0.0, 1.0);
        let _ = self.sender.send(JobEvent::Progress {
            stage: self.stage,
            fraction,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone)]
pub struct JobContext {
    stage: StageId,
    cancel: CancelFlag,
    progress: ProgressReporter,
}

impl JobContext {
    pub fn stage(&self) -> StageId {
        self.stage
    }

    pub fn cancel_flag(&self) -> CancelFlag {
        self.cancel.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn check_cancelled(&self) -> Result<(), JobError> {
        if self.is_cancelled() {
            Err(JobError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub fn report(&self, fraction: f32, message: impl Into<String>) {
        self.progress.report(fraction, message);
    }
}

pub struct JobHandle {
    cancel: CancelFlag,
    receiver: Receiver<JobEvent>,
}

impl JobHandle {
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn try_recv(&self) -> Option<JobEvent> {
        self.receiver.try_recv().ok()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<JobEvent, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Runs one heavy job at a time on a dedicated Tokio runtime thread.
pub struct JobRunner {
    handle: Handle,
    _keepalive: Sender<()>,
    busy: Arc<AtomicBool>,
}

impl JobRunner {
    pub fn new() -> Self {
        let (keepalive_tx, keepalive_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        thread::Builder::new()
            .name("usdx-jobs".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("failed to build job runtime");
                let _ = ready_tx.send(runtime.handle().clone());
                let _ = keepalive_rx.recv();
                runtime.shutdown_timeout(Duration::from_secs(5));
            })
            .expect("failed to spawn job runtime thread");

        let handle = ready_rx.recv().expect("job runtime thread failed to start");

        Self {
            handle,
            _keepalive: keepalive_tx,
            busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts a job. Returns [`JobError::Busy`] if another heavy job is active.
    pub fn spawn<F, Fut>(&self, stage: StageId, job: F) -> Result<JobHandle, JobError>
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), JobError>> + Send + 'static,
    {
        if self.busy.swap(true, Ordering::AcqRel) {
            return Err(JobError::Busy);
        }

        let (sender, receiver) = mpsc::channel();
        let cancel = CancelFlag::default();
        let context = JobContext {
            stage,
            cancel: cancel.clone(),
            progress: ProgressReporter {
                stage,
                sender: sender.clone(),
            },
        };

        let busy = Arc::clone(&self.busy);
        self.handle.spawn(async move {
            let _guard = BusyGuard(busy);
            let result = job(context).await;
            let _ = sender.send(JobEvent::Finished(result));
        });

        Ok(JobHandle { cancel, receiver })
    }
}

impl Default for JobRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next_event(handle: &JobHandle) -> JobEvent {
        handle
            .recv_timeout(Duration::from_secs(3))
            .expect("job event")
    }

    #[test]
    fn job_reports_progress_and_finishes() {
        let runner = JobRunner::new();
        let handle = runner
            .spawn(StageId::Pitch, |context| async move {
                context.report(0.5, "half");
                Ok(())
            })
            .unwrap();

        assert!(matches!(
            next_event(&handle),
            JobEvent::Progress { fraction, .. } if (fraction - 0.5).abs() < f32::EPSILON
        ));
        assert!(matches!(next_event(&handle), JobEvent::Finished(Ok(()))));
    }

    #[test]
    fn only_one_heavy_job_runs_at_a_time() {
        let runner = JobRunner::new();
        let _handle = runner
            .spawn(StageId::Separate, |_context| async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                Ok(())
            })
            .unwrap();

        let error = runner
            .spawn(StageId::Transcribe, |_context| async { Ok(()) })
            .err()
            .expect("second job must be rejected");
        assert_eq!(error, JobError::Busy);
    }

    #[test]
    fn cancellation_is_observed_by_the_job() {
        let runner = JobRunner::new();
        let handle = runner
            .spawn(StageId::Transcribe, |context| async move {
                while !context.is_cancelled() {
                    tokio::task::yield_now().await;
                }
                Err(JobError::Cancelled)
            })
            .unwrap();

        handle.cancel();
        assert!(matches!(
            next_event(&handle),
            JobEvent::Finished(Err(JobError::Cancelled))
        ));
    }

    #[test]
    fn progress_is_clamped() {
        let runner = JobRunner::new();
        let handle = runner
            .spawn(StageId::Align, |context| async move {
                context.report(-3.0, "low");
                context.report(4.0, "high");
                Ok(())
            })
            .unwrap();

        assert!(matches!(
            next_event(&handle),
            JobEvent::Progress { fraction, .. } if fraction == 0.0
        ));
        assert!(matches!(
            next_event(&handle),
            JobEvent::Progress { fraction, .. } if fraction == 1.0
        ));
        assert!(matches!(next_event(&handle), JobEvent::Finished(Ok(()))));
    }
}
