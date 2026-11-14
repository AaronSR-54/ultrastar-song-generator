use std::io;
use std::path::Path;
use std::process::ExitStatus;
use std::time::Duration;

use tokio::process::Command;
use usdx_ports::CancelFlag;

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn is_success(&self) -> bool {
        self.status.success()
    }

    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("failed to start process: {0}")]
    Spawn(#[from] io::Error),
    #[error("process timed out after {0:?}")]
    Timeout(Duration),
    #[error("process was cancelled")]
    Cancelled,
    #[error("process failed with {status}: {stderr}")]
    Failed { status: ExitStatus, stderr: String },
}

/// Runs a command with a hard timeout and cooperative cancellation.
///
/// The child is killed when the future is dropped (`kill_on_drop`), so a
/// timeout or cancellation does not leave orphan processes behind.
pub async fn run(
    mut command: Command,
    timeout: Duration,
    cancel: Option<CancelFlag>,
) -> Result<ProcessOutput, ProcessError> {
    command.kill_on_drop(true);

    let output = command.output();
    let cancel_wait = async move {
        if let Some(cancel) = cancel {
            while !cancel.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } else {
            std::future::pending::<()>().await;
        }
    };

    let result = tokio::time::timeout(timeout, async {
        tokio::select! {
            output = output => output.map_err(ProcessError::Spawn),
            _ = cancel_wait => Err(ProcessError::Cancelled),
        }
    })
    .await;

    match result {
        Ok(Ok(output)) => Ok(ProcessOutput {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(ProcessError::Timeout(timeout)),
    }
}

/// Like [`run`], but turns a non-zero exit status into [`ProcessError::Failed`].
pub async fn run_checked(
    command: Command,
    timeout: Duration,
    cancel: Option<CancelFlag>,
) -> Result<ProcessOutput, ProcessError> {
    let output = run(command, timeout, cancel).await?;
    if output.is_success() {
        Ok(output)
    } else {
        Err(ProcessError::Failed {
            status: output.status,
            stderr: truncate(&output.stderr_lossy(), 2_000),
        })
    }
}

/// Removes partial download files for `stem` from `directory`.
pub fn cleanup_partials(directory: impl AsRef<Path>, stem: &str) -> io::Result<usize> {
    let directory = directory.as_ref();
    if !directory.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(stem)
            && (name.ends_with(".part") || name.ends_with(".ytdl") || name.ends_with(".temp"))
        {
            std::fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut result: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        result.push_str("...");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        #[cfg(windows)]
        {
            let mut command = Command::new("cmd");
            command.args(["/C", script]);
            command
        }
        #[cfg(not(windows))]
        {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            command
        }
    }

    #[tokio::test]
    async fn successful_process_captures_stdout() {
        let output = run(shell("echo hello"), Duration::from_secs(5), None)
            .await
            .unwrap();
        assert!(output.is_success());
        assert!(output.stdout_lossy().contains("hello"));
    }

    #[tokio::test]
    async fn failed_process_is_reported_with_stderr() {
        let script = if cfg!(windows) {
            "echo bad 1>&2 & exit 3"
        } else {
            "echo bad >&2; exit 3"
        };
        let error = run_checked(shell(script), Duration::from_secs(5), None)
            .await
            .unwrap_err();
        match error {
            ProcessError::Failed { status, stderr } => {
                assert_eq!(status.code(), Some(3));
                assert!(stderr.contains("bad"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_kills_the_child() {
        let script = if cfg!(windows) {
            "ping -n 5 127.0.0.1 > nul"
        } else {
            "sleep 5"
        };
        let error = run(shell(script), Duration::from_millis(150), None)
            .await
            .unwrap_err();
        assert!(matches!(error, ProcessError::Timeout(_)));
    }

    #[tokio::test]
    async fn pre_cancelled_process_does_not_run() {
        let cancel = CancelFlag::default();
        cancel.cancel();
        let script = if cfg!(windows) {
            "ping -n 5 127.0.0.1 > nul"
        } else {
            "sleep 5"
        };
        let error = run(shell(script), Duration::from_secs(5), Some(cancel))
            .await
            .unwrap_err();
        assert!(matches!(error, ProcessError::Cancelled));
    }

    #[test]
    fn cleanup_removes_only_partial_files() {
        let directory = tempfile::tempdir().unwrap();
        for name in [
            "audio-source.mp3",
            "audio-source.mp3.part",
            "audio-source.mp3.ytdl",
            "audio-source.mp3.temp",
            "other.mp3.part",
        ] {
            std::fs::write(directory.path().join(name), b"x").unwrap();
        }
        let removed = cleanup_partials(directory.path(), "audio-source").unwrap();
        assert_eq!(removed, 3);
        assert!(directory.path().join("audio-source.mp3").exists());
        assert!(directory.path().join("other.mp3.part").exists());
    }
}
