use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Cpu,
    Cuda,
    CoreMl,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Backend::Cpu => "CPU",
            Backend::Cuda => "CUDA",
            Backend::CoreMl => "CoreML",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hardware {
    pub cuda: bool,
    pub coreml: bool,
}

impl Hardware {
    pub fn detect() -> Self {
        let cuda = Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
            .map(|output| output.status.success() && !output.stdout.is_empty())
            .unwrap_or(false);
        Self {
            cuda,
            coreml: cfg!(target_os = "macos"),
        }
    }

    pub fn gpu_backend(&self) -> Option<Backend> {
        if self.cuda {
            Some(Backend::Cuda)
        } else if self.coreml {
            Some(Backend::CoreMl)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendChoice {
    pub selected: Backend,
    pub fallback_reason: Option<String>,
}

/// Selects a backend, trying GPU first when allowed and falling back to CPU.
///
/// `try_init` is injected so callers can initialise `ort`/whisper and tests can
/// simulate an out-of-memory GPU.
pub fn choose_backend<F>(
    preference: BackendPreference,
    hardware: Hardware,
    mut try_init: F,
) -> BackendChoice
where
    F: FnMut(Backend) -> Result<(), String>,
{
    let gpu = hardware.gpu_backend();
    let mut reasons = Vec::new();

    let candidates = match preference {
        BackendPreference::Cpu => vec![Backend::Cpu],
        BackendPreference::Auto | BackendPreference::Gpu => match gpu {
            Some(gpu) => vec![gpu, Backend::Cpu],
            None => {
                reasons.push("no GPU detected".to_string());
                vec![Backend::Cpu]
            }
        },
    };

    for backend in candidates {
        match try_init(backend) {
            Ok(()) => {
                let fallback_reason = if backend == Backend::Cpu && !reasons.is_empty() {
                    Some(reasons.join("; "))
                } else {
                    None
                };
                return BackendChoice {
                    selected: backend,
                    fallback_reason,
                };
            }
            Err(error) => reasons.push(format!("{}: {error}", backend.label())),
        }
    }

    BackendChoice {
        selected: Backend::Cpu,
        fallback_reason: Some(reasons.join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_preference_does_not_try_gpu() {
        let choice = choose_backend(
            BackendPreference::Cpu,
            Hardware {
                cuda: true,
                coreml: false,
            },
            |backend| {
                assert_eq!(backend, Backend::Cpu);
                Ok(())
            },
        );
        assert_eq!(choice.selected, Backend::Cpu);
        assert!(choice.fallback_reason.is_none());
    }

    #[test]
    fn auto_prefers_gpu_when_it_initialises() {
        let choice = choose_backend(
            BackendPreference::Auto,
            Hardware {
                cuda: true,
                coreml: false,
            },
            |_| Ok(()),
        );
        assert_eq!(choice.selected, Backend::Cuda);
        assert!(choice.fallback_reason.is_none());
    }

    #[test]
    fn failed_gpu_falls_back_to_cpu_with_reason() {
        let choice = choose_backend(
            BackendPreference::Gpu,
            Hardware {
                cuda: true,
                coreml: false,
            },
            |backend| {
                if backend == Backend::Cuda {
                    Err("out of memory".into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(choice.selected, Backend::Cpu);
        assert!(choice
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("out of memory")));
    }

    #[test]
    fn gpu_preference_without_gpu_reports_it() {
        let choice = choose_backend(
            BackendPreference::Gpu,
            Hardware {
                cuda: false,
                coreml: false,
            },
            |_| Ok(()),
        );
        assert_eq!(choice.selected, Backend::Cpu);
        assert!(choice
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("no GPU")));
    }

    #[test]
    fn coreml_is_used_on_macos_hardware() {
        let choice = choose_backend(
            BackendPreference::Auto,
            Hardware {
                cuda: false,
                coreml: true,
            },
            |_| Ok(()),
        );
        assert_eq!(choice.selected, Backend::CoreMl);
    }
}
