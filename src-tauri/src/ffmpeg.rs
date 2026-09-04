//! FFmpeg executable discovery shared by GUI commands and the renderer.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the FFmpeg executable in a stable priority order:
///
/// 1. an explicitly configured path or non-default bare command;
/// 2. for the default `ffmpeg` command, the bundled executable when present;
/// 3. `ffmpeg` from the process `PATH`.
///
/// Missing explicit path-like references are intentionally preserved so full
/// configuration validation can report them instead of silently falling back.
pub fn resolve_ffmpeg_path(configured: Option<&Path>) -> PathBuf {
    #[cfg(target_os = "windows")]
    let executable = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    let executable = "ffmpeg";

    if let Some(path) = configured.filter(|path| !path.as_os_str().is_empty()) {
        if path.is_file() {
            return path.to_path_buf();
        }
        let is_default_command = path == Path::new("ffmpeg") || path == Path::new("ffmpeg.exe");
        if !is_default_command {
            // Preserve an explicitly configured command/path. Full render
            // validation catches missing path-like values, while a bare command
            // is intentionally allowed to resolve through the process PATH.
            return path.to_path_buf();
        }
    }

    let bundled = crate::json_config::asset_path(Path::new("ffmpeg/bin").join(executable));
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from("ffmpeg")
    }
}

/// Verify that the resolved FFmpeg executable can actually be started.
///
/// Configuration validation intentionally handles path shape/existence while
/// this runtime check covers PATH commands and broken executables.
pub fn check_ffmpeg_runtime(configured: Option<&Path>) -> Result<(PathBuf, String)> {
    let ffmpeg_path = resolve_ffmpeg_path(configured);
    let output = Command::new(&ffmpeg_path)
        .arg("-version")
        .output()
        .with_context(|| format!("Failed to start FFmpeg: {}", ffmpeg_path.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "FFmpeg runtime check failed with status {}: {}{}",
            output.status,
            ffmpeg_path.display(),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", stderr.trim())
            }
        );
    }
    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("version unavailable")
        .to_string();
    Ok((ffmpeg_path, version))
}

#[cfg(test)]
#[path = "../tests/unit/ffmpeg.rs"]
mod tests;
