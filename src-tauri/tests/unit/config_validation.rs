use super::*;
use crate::content_template::ContentTemplate;
use crate::json_config::{CharEntry, FfmpegConfig, RenderConfig};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("unicode-flash-mob-{nonce}-{name}"))
}

fn one_character() -> CharEntry {
    CharEntry {
        code_point: "U+0041".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: Some(1),
    }
}

#[test]
fn full_render_validation_rejects_output_directory() {
    let directory = unique_temp_path("output-dir");
    std::fs::create_dir_all(&directory).unwrap();
    let config = RenderConfig {
        output_path: directory.clone(),
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    assert!(format!("{error:#}").contains("Output path points to a directory"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn full_render_validation_rejects_missing_music() {
    let missing = unique_temp_path("missing-music.wav");
    let config = RenderConfig {
        music_path: Some(missing.clone()),
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("Music file not found"));
    assert!(message.contains(missing.to_string_lossy().as_ref()));
}

#[test]
fn full_render_validation_rejects_zero_frame_queue() {
    let config = RenderConfig {
        characters: vec![one_character()],
        ffmpeg: FfmpegConfig {
            max_inflight_frames: 0,
            ..FfmpegConfig::default()
        },
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    assert!(format!("{error:#}").contains("max_inflight_frames must be at least 1"));
}

#[test]
fn full_render_validation_rejects_zero_encoding_processes() {
    let config = RenderConfig {
        characters: vec![one_character()],
        ffmpeg: FfmpegConfig {
            encoding_processes: 0,
            ..FfmpegConfig::default()
        },
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    assert!(format!("{error:#}").contains("encoding_processes must be at least 1"));
}

#[test]
fn full_render_validation_rejects_output_without_extension() {
    let config = RenderConfig {
        output_path: unique_temp_path("output-without-extension"),
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    assert!(format!("{error:#}").contains("Output path must include a file extension"));
}

#[test]
fn full_render_validation_rejects_missing_explicit_ffmpeg_path() {
    let missing = unique_temp_path("tools").join("ffmpeg-custom");
    let config = RenderConfig {
        characters: vec![one_character()],
        ffmpeg: FfmpegConfig {
            path: missing.clone(),
            ..FfmpegConfig::default()
        },
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, true).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("FFmpeg executable not found"));
    assert!(message.contains(missing.to_string_lossy().as_ref()));
}

#[test]
fn preview_validation_rejects_missing_external_template_executable() {
    let missing = unique_temp_path("tools").join("template-runner");
    let mut config = RenderConfig::default();
    config.content_templates.insert(
        "external".to_string(),
        ContentTemplate::External {
            executable: missing.clone(),
            args: Vec::new(),
        },
    );
    let error = validate_render_config(&config, false).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("External content template"));
    assert!(message.contains("executable not found"));
    assert!(message.contains(missing.to_string_lossy().as_ref()));
}
