use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::process::std_command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    YtDlp,
    Ffmpeg,
    Ffprobe,
    Deno,
    Node,
}

impl ToolKind {
    pub fn executable(self) -> &'static str {
        match self {
            ToolKind::YtDlp => "yt-dlp",
            ToolKind::Ffmpeg => "ffmpeg",
            ToolKind::Ffprobe => "ffprobe",
            ToolKind::Deno => "deno",
            ToolKind::Node => "node",
        }
    }

    pub fn version_arg(self) -> &'static str {
        match self {
            ToolKind::YtDlp | ToolKind::Deno | ToolKind::Node => "--version",
            ToolKind::Ffmpeg | ToolKind::Ffprobe => "-version",
        }
    }

    pub fn minimum_version(self) -> &'static str {
        match self {
            ToolKind::YtDlp => "2024.01.01",
            ToolKind::Ffmpeg | ToolKind::Ffprobe => "4.0",
            ToolKind::Deno => "2.0.0",
            ToolKind::Node => "18.0.0",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub kind: ToolKind,
    pub path: PathBuf,
    /// Short version string, e.g. `2025.09.26`, `8.0` or `22.14.0`.
    pub version: String,
}

impl Tool {
    pub fn version_at_least(&self, minimum: &str) -> bool {
        version_compare(&self.version, minimum) != std::cmp::Ordering::Less
    }

    pub fn is_supported(&self) -> bool {
        self.version_at_least(self.kind.minimum_version())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Toolchain {
    pub yt_dlp: Option<Tool>,
    pub ffmpeg: Option<Tool>,
    pub ffprobe: Option<Tool>,
    pub deno: Option<Tool>,
    pub node: Option<Tool>,
}

impl Toolchain {
    pub fn detect() -> Self {
        Self::detect_with(&HashMap::new())
    }

    /// Detection with explicit paths, useful for tests and user overrides.
    pub fn detect_with(overrides: &HashMap<ToolKind, PathBuf>) -> Self {
        let yt_dlp = detect_tool(ToolKind::YtDlp, overrides);
        let ffmpeg = detect_tool(ToolKind::Ffmpeg, overrides);
        let ffprobe = detect_ffprobe(overrides, ffmpeg.as_ref());
        let deno = detect_tool(ToolKind::Deno, overrides);
        let node = detect_tool(ToolKind::Node, overrides);

        Self {
            yt_dlp,
            ffmpeg,
            ffprobe,
            deno,
            node,
        }
    }

    pub fn js_runtime(&self) -> Option<&Tool> {
        self.deno.as_ref().or(self.node.as_ref())
    }

    pub fn js_runtime_name(&self) -> Option<&'static str> {
        if self.deno.is_some() {
            Some("deno")
        } else if self.node.is_some() {
            Some("node")
        } else {
            None
        }
    }

    pub fn issues(&self) -> Vec<ToolchainIssue> {
        let mut issues = Vec::new();
        check_required(&mut issues, self.yt_dlp.as_ref(), ToolKind::YtDlp);
        check_required(&mut issues, self.ffmpeg.as_ref(), ToolKind::Ffmpeg);
        check_required(&mut issues, self.ffprobe.as_ref(), ToolKind::Ffprobe);

        match self.js_runtime() {
            Some(tool) if tool.is_supported() => {}
            Some(tool) => issues.push(ToolchainIssue {
                tool: tool.kind.executable(),
                message: format!(
                    "{} {} is too old; {} or newer is required",
                    tool.kind.executable(),
                    tool.version,
                    tool.kind.minimum_version()
                ),
            }),
            None => issues.push(ToolchainIssue {
                tool: "javascript runtime",
                message: "install Deno 2+ (recommended) or Node 18+ so yt-dlp can solve YouTube challenges"
                    .into(),
            }),
        }

        issues
    }

    pub fn ensure_ready(&self) -> Result<(), ToolchainError> {
        let issues = self.issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(ToolchainError { issues })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainIssue {
    pub tool: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ToolchainError {
    pub issues: Vec<ToolchainIssue>,
}

impl std::fmt::Display for ToolchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "toolchain is not ready: ")?;
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{}: {}", issue.tool, issue.message)?;
        }
        Ok(())
    }
}

fn check_required(issues: &mut Vec<ToolchainIssue>, tool: Option<&Tool>, kind: ToolKind) {
    match tool {
        Some(tool) if tool.is_supported() => {}
        Some(tool) => issues.push(ToolchainIssue {
            tool: kind.executable(),
            message: format!(
                "{} {} is too old; {} or newer is required",
                kind.executable(),
                tool.version,
                kind.minimum_version()
            ),
        }),
        None => issues.push(ToolchainIssue {
            tool: kind.executable(),
            message: format!("{} was not found in PATH", kind.executable()),
        }),
    }
}

fn detect_tool(kind: ToolKind, overrides: &HashMap<ToolKind, PathBuf>) -> Option<Tool> {
    if let Some(path) = overrides.get(&kind) {
        if !path.exists() {
            return None;
        }
        return detect_at(kind, path);
    }
    detect_at(kind, Path::new(kind.executable()))
}

fn detect_at(kind: ToolKind, path: &Path) -> Option<Tool> {
    let output = std_command(path).arg(kind.version_arg()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stdout.trim().is_empty() {
        stderr.as_ref()
    } else {
        stdout.as_ref()
    };
    let version = parse_version(kind, text)?;
    Some(Tool {
        kind,
        path: path.to_path_buf(),
        version,
    })
}

fn detect_ffprobe(overrides: &HashMap<ToolKind, PathBuf>, ffmpeg: Option<&Tool>) -> Option<Tool> {
    if let Some(tool) = detect_tool(ToolKind::Ffprobe, overrides) {
        return Some(tool);
    }
    let ffmpeg = ffmpeg?;
    let name = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    let sibling = ffmpeg.path.with_file_name(name);
    if sibling.exists() {
        detect_at(ToolKind::Ffprobe, &sibling)
    } else {
        None
    }
}

/// Extracts a short version from the first meaningful line of `--version`.
pub fn parse_version(kind: ToolKind, output: &str) -> Option<String> {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let token = match kind {
        ToolKind::YtDlp => line.split_whitespace().next(),
        ToolKind::Ffmpeg | ToolKind::Ffprobe => line
            .split_whitespace()
            .nth(2)
            .or_else(|| line.split_whitespace().next()),
        ToolKind::Deno => line.split_whitespace().nth(1),
        ToolKind::Node => line.split_whitespace().next(),
    }?;
    let token = token.trim_start_matches('v');
    let version = token
        .split(['-', '+', ' ', '('])
        .next()
        .unwrap_or(token)
        .trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn version_compare(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts: Vec<u64> = left
        .split(['.', '-', '_'])
        .filter_map(|part| part.parse().ok())
        .collect();
    let right_parts: Vec<u64> = right
        .split(['.', '-', '_'])
        .filter_map(|part| part.parse().ok())
        .collect();
    for index in 0..left_parts.len().max(right_parts.len()) {
        let left = left_parts.get(index).copied().unwrap_or(0);
        let right = right_parts.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(kind: ToolKind, version: &str) -> Tool {
        Tool {
            kind,
            path: PathBuf::from(kind.executable()),
            version: version.into(),
        }
    }

    #[test]
    fn parses_yt_dlp_version() {
        assert_eq!(
            parse_version(ToolKind::YtDlp, "2025.09.26\n"),
            Some("2025.09.26".into())
        );
    }

    #[test]
    fn parses_ffmpeg_version() {
        assert_eq!(
            parse_version(
                ToolKind::Ffmpeg,
                "ffmpeg version 8.0-essentials_build-www.gyan.dev Copyright (c) 2000-2025\n"
            ),
            Some("8.0".into())
        );
    }

    #[test]
    fn parses_node_and_deno_versions() {
        assert_eq!(
            parse_version(ToolKind::Node, "v22.14.0\n"),
            Some("22.14.0".into())
        );
        assert_eq!(
            parse_version(
                ToolKind::Deno,
                "deno 2.1.4 (stable, release, x86_64-pc-windows-msvc)\n"
            ),
            Some("2.1.4".into())
        );
    }

    #[test]
    fn old_versions_are_rejected() {
        assert!(!tool(ToolKind::YtDlp, "2023.12.31").is_supported());
        assert!(tool(ToolKind::YtDlp, "2025.09.26").is_supported());
        assert!(!tool(ToolKind::Node, "16.20.0").is_supported());
        assert!(tool(ToolKind::Node, "18.0.0").is_supported());
    }

    #[test]
    fn version_comparison_handles_missing_parts() {
        assert_eq!(version_compare("8.0", "4.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_compare("4", "4.0.0"), std::cmp::Ordering::Equal);
        assert_eq!(version_compare("2.0.0", "2.1"), std::cmp::Ordering::Less);
    }

    #[test]
    fn missing_tools_produce_actionable_issues() {
        let toolchain = Toolchain::default();
        let issues = toolchain.issues();
        assert!(issues.iter().any(|issue| issue.tool == "yt-dlp"));
        assert!(issues.iter().any(|issue| issue.tool == "ffmpeg"));
        assert!(issues.iter().any(|issue| issue.message.contains("Deno")));
        assert!(toolchain.ensure_ready().is_err());
    }

    #[test]
    fn missing_override_path_is_not_detected() {
        let mut overrides = HashMap::new();
        overrides.insert(
            ToolKind::YtDlp,
            PathBuf::from("C:/definitely/does/not/exist/yt-dlp.exe"),
        );
        let toolchain = Toolchain::detect_with(&overrides);
        assert!(toolchain.yt_dlp.is_none());
    }
}
