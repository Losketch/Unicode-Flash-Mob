//! FFmpeg executable discovery shared by GUI commands and the renderer.

use std::path::{Path, PathBuf};

/// Resolve the FFmpeg executable in a stable priority order:
///
/// 1. an explicitly configured path that exists;
/// 2. the bundled executable shipped with the application;
/// 3. `ffmpeg` from the process `PATH`.
pub fn resolve_ffmpeg_path(configured: Option<&Path>) -> PathBuf {
    if let Some(path) = configured.filter(|path| path.is_file()) {
        return path.to_path_buf();
    }

    #[cfg(target_os = "windows")]
    let executable = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    let executable = "ffmpeg";

    let bundled = crate::json_config::asset_path(Path::new("ffmpeg/bin").join(executable));
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from("ffmpeg")
    }
}
