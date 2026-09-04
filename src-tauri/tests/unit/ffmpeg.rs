use super::resolve_ffmpeg_path;
use std::path::{Path, PathBuf};

#[test]
fn explicit_bare_ffmpeg_command_is_preserved() {
    let command = Path::new("custom-ffmpeg");
    assert_eq!(resolve_ffmpeg_path(Some(command)), PathBuf::from(command));
}

#[test]
fn explicit_missing_ffmpeg_path_is_not_silently_replaced() {
    let configured = PathBuf::from("definitely-missing-tools").join("ffmpeg-custom");
    assert_eq!(resolve_ffmpeg_path(Some(&configured)), configured);
}
