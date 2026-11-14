use std::path::Path;

use usdx_ports::{SeparationControl, SeparationError, StemPaths, StemSeparator};

/// Retries a separation on a fallback engine when the primary runs out of
/// memory (for example, a CUDA session that failed to allocate).
pub struct SeparationService<S> {
    primary: S,
    fallback: Option<S>,
}

impl<S: StemSeparator> SeparationService<S> {
    pub fn new(primary: S, fallback: Option<S>) -> Self {
        Self { primary, fallback }
    }

    pub fn separate(
        &self,
        input: &Path,
        output_dir: &Path,
        control: SeparationControl,
    ) -> Result<(StemPaths, Option<String>), SeparationError> {
        match self.primary.separate(input, output_dir, control.clone()) {
            Ok(paths) => Ok((paths, None)),
            Err(error) if is_out_of_memory(&error) => {
                let Some(fallback) = &self.fallback else {
                    return Err(error);
                };
                control.report(0.0);
                let paths = fallback.separate(input, output_dir, control)?;
                Ok((
                    paths,
                    Some(format!(
                        "primary separation backend ran out of memory ({error}); retried on CPU"
                    )),
                ))
            }
            Err(error) => Err(error),
        }
    }
}

fn is_out_of_memory(error: &SeparationError) -> bool {
    match error {
        SeparationError::Failed(message) => {
            let message = message.to_lowercase();
            message.contains("out of memory")
                || message.contains("oom")
                || message.contains("memory allocation")
                || message.contains("not enough memory")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use usdx_ports::CancelFlag;

    use super::*;

    struct FakeSeparator {
        fail_with: Option<&'static str>,
        reports: Vec<f32>,
        wait_for_cancel: bool,
    }

    impl StemSeparator for FakeSeparator {
        fn separate(
            &self,
            _input: &Path,
            output_dir: &Path,
            control: SeparationControl,
        ) -> Result<StemPaths, SeparationError> {
            if let Some(message) = self.fail_with {
                return Err(SeparationError::Failed(message.into()));
            }
            if self.wait_for_cancel {
                while !control.cancel.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(5));
                }
                return Err(SeparationError::Cancelled);
            }
            for value in &self.reports {
                control.report(*value);
            }
            Ok(StemPaths {
                vocals: output_dir.join("vocals.wav"),
                instrumental: output_dir.join("instrumental.wav"),
            })
        }
    }

    #[test]
    fn reports_progress() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&values);
        let service = SeparationService::new(
            FakeSeparator {
                fail_with: None,
                reports: vec![0.25, 0.75, 1.0],
                wait_for_cancel: false,
            },
            None,
        );
        let control = SeparationControl::new(
            CancelFlag::default(),
            Arc::new(move |value| sink.lock().unwrap().push(value)),
        );
        let (result, fallback) = service
            .separate(Path::new("in.wav"), Path::new("."), control)
            .unwrap();
        assert!(result.vocals.ends_with("vocals.wav"));
        assert!(fallback.is_none());
        assert_eq!(*values.lock().unwrap(), vec![0.25, 0.75, 1.0]);
    }

    #[test]
    fn retries_on_out_of_memory() {
        let service = SeparationService::new(
            FakeSeparator {
                fail_with: Some("CUDA out of memory"),
                reports: vec![],
                wait_for_cancel: false,
            },
            Some(FakeSeparator {
                fail_with: None,
                reports: vec![1.0],
                wait_for_cancel: false,
            }),
        );
        let control = SeparationControl::new(CancelFlag::default(), Arc::new(|_| {}));
        let (_, fallback) = service
            .separate(Path::new("in.wav"), Path::new("."), control)
            .unwrap();
        assert!(fallback.unwrap().contains("out of memory"));
    }

    #[test]
    fn does_not_retry_other_errors() {
        let service = SeparationService::new(
            FakeSeparator {
                fail_with: Some("model is corrupt"),
                reports: vec![],
                wait_for_cancel: false,
            },
            Some(FakeSeparator {
                fail_with: None,
                reports: vec![1.0],
                wait_for_cancel: false,
            }),
        );
        let control = SeparationControl::new(CancelFlag::default(), Arc::new(|_| {}));
        let error = service
            .separate(Path::new("in.wav"), Path::new("."), control)
            .unwrap_err();
        assert!(matches!(error, SeparationError::Failed(message) if message.contains("corrupt")));
    }

    #[test]
    fn cancellation_is_cooperative() {
        let cancel = CancelFlag::default();
        let cancel_thread = cancel.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            cancel_thread.cancel();
        });
        let service = SeparationService::new(
            FakeSeparator {
                fail_with: None,
                reports: vec![],
                wait_for_cancel: true,
            },
            None,
        );
        let control = SeparationControl::new(cancel.clone(), Arc::new(|_| {}));
        let error = service
            .separate(Path::new("in.wav"), Path::new("."), control)
            .unwrap_err();
        handle.join().unwrap();
        assert!(matches!(error, SeparationError::Cancelled));
        assert!(cancel.is_cancelled());
    }

    #[test]
    fn cancellation_flag_is_shared() {
        let flag = CancelFlag::default();
        let clone = flag.clone();
        assert!(!flag.is_cancelled());
        clone.cancel();
        assert!(flag.is_cancelled());
    }
}
