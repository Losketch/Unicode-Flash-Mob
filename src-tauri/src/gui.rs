use crate::extractor;
use crate::json_config::RenderConfig;
use crate::renderer;
use crate::unicode_data;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Emitter, Manager};

#[tauri::command]
pub fn load_config(path: String) -> Result<RenderConfig, String> {
    let config_path = PathBuf::from(&path);
    RenderConfig::from_file(&config_path).map_err(|e| format!("Failed to load config: {e}"))
}

#[tauri::command]
pub fn save_config(path: String, config: RenderConfig) -> Result<(), String> {
    let config_path = PathBuf::from(&path);
    config
        .to_file(&config_path)
        .map_err(|e| format!("Failed to save config: {e}"))
}

#[tauri::command]
pub fn get_default_config() -> RenderConfig {
    RenderConfig::default()
}

#[tauri::command]
pub async fn render_video(
    window: tauri::Window,
    config: RenderConfig,
    task_id: Option<String>,
) -> Result<String, String> {
    let window_for_progress = window.clone();
    let progress_callback: Arc<dyn Fn(u64, u64) + Send + Sync> =
        Arc::new(move |current: u64, total: u64| {
            let progress = if total > 0 {
                current as f64 / total as f64
            } else {
                0.0
            };
            let _ = window_for_progress.emit(
                "render-progress",
                serde_json::json!({
                    "taskId": task_id.as_deref(),
                    "progress": progress,
                    "current": current,
                    "total": total,
                }),
            );
        });

    tokio::task::spawn_blocking(move || {
        renderer::render_video_with_progress(&config, Some(progress_callback))
            .map(|_| "Video rendered successfully".to_string())
            .map_err(|e| format!("Render failed: {e:#}"))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn render_frame_preview(
    config: RenderConfig,
    entry_index: usize,
    max_dimension: Option<u32>,
) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        renderer::render_frame_png(&config, entry_index, max_dimension.unwrap_or(960))
            .map_err(|e| format!("Preview render failed: {e:#}"))
    })
    .await
    .map_err(|e| format!("Preview task failed: {e}"))?
}

#[tauri::command]
pub async fn extract_characters(
    font_files: Vec<String>,
    output_path: String,
    video_output: Option<String>,
) -> Result<RenderConfig, String> {
    let font_paths: Vec<PathBuf> = font_files.iter().map(PathBuf::from).collect();
    let requested_output = PathBuf::from(&output_path);
    let video = video_output.map(PathBuf::from);

    tokio::task::spawn_blocking(move || {
        let cwd =
            std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
        // Keep generated configs outside src-tauri during `tauri dev` to avoid watcher restarts.
        let output = if requested_output.is_absolute() {
            requested_output
        } else if requested_output == Path::new("unicode_config.json") {
            cwd.parent()
                .unwrap_or(&cwd)
                .join("output")
                .join("unicode_config.json")
        } else {
            cwd.join(requested_output)
        };

        let config = extractor::extract_to_config(&font_paths, &output, video.as_deref())
            .map_err(|e| format!("Failed to extract characters: {e}"))?;

        Ok(config)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub fn get_unicode_description(code_point: u32) -> String {
    let (data_path, blocks_path) = unicode_data::find_unicode_data_files();
    let unicode_manager = unicode_data::UnicodeDataManager::load(&data_path, &blocks_path)
        .unwrap_or_else(|_| unicode_data::UnicodeDataManager::empty());
    unicode_manager.get_description(code_point)
}

#[tauri::command]
pub fn list_font_characters(font_path: String) -> Result<Vec<u32>, String> {
    let configured_path = PathBuf::from(font_path);
    let path = crate::json_config::resolve_asset_reference(&configured_path);
    extractor::collect_font_codepoints(&path)
        .map(|codepoints| codepoints.into_iter().collect())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_encoder(hevc: bool) -> Result<Option<String>, String> {
    let ffmpeg = crate::ffmpeg::resolve_ffmpeg_path(None);
    Ok(crate::renderer::detect_encoder(&ffmpeg, hevc))
}

#[tauri::command]
pub fn test_encoder_command(encoder: String) -> Result<bool, String> {
    let ffmpeg = crate::ffmpeg::resolve_ffmpeg_path(None);
    crate::renderer::test_encoder(&ffmpeg, &encoder).map_err(|e| e.to_string())
}

pub fn run_gui() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir()?;
            crate::json_config::set_resource_dir(resource_dir);
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            get_default_config,
            render_video,
            render_frame_preview,
            extract_characters,
            get_unicode_description,
            list_font_characters,
            get_encoder,
            test_encoder_command,
        ])
        .run(tauri::generate_context!())
}
