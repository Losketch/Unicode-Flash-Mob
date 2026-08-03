use crate::font_loader::FontLoader;
use crate::json_config::{
    duration_to_frames, AnimationCurve, CharEntry, Color, EventType, FontConfig, GlyphSelector,
    GlyphSpec, Position, RenderConfig,
};
use crate::unicode_data::UnicodeDataManager;
use ab_glyph::Font;
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, unbounded};
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, RgbaImage};
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_PREVIEW_FONT_CACHE_ENTRIES: usize = 16;

static PREVIEW_FONT_CACHE: Lazy<Mutex<HashMap<String, Arc<FontLoader>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PREVIEW_UNICODE_CACHE: Lazy<Mutex<HashMap<String, Arc<UnicodeDataManager>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn load_preview_font(path: &Path, config: &FontConfig) -> Result<Arc<FontLoader>> {
    let resolved_path = crate::json_config::resolve_asset_reference(path);
    let key = format!(
        "{}|{}",
        resolved_path.to_string_lossy(),
        serde_json::to_string(config).context("Failed to serialize font settings")?
    );
    if let Some(font) = PREVIEW_FONT_CACHE
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview font cache lock poisoned"))?
        .get(&key)
        .cloned()
    {
        return Ok(font);
    }

    let font = Arc::new(
        FontLoader::from_path_with_config(&resolved_path, config)
            .with_context(|| format!("Failed to load preview font: {}", resolved_path.display()))?,
    );
    let mut cache = PREVIEW_FONT_CACHE
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview font cache lock poisoned"))?;
    if cache.len() >= MAX_PREVIEW_FONT_CACHE_ENTRIES && !cache.contains_key(&key) {
        cache.clear();
    }
    cache.insert(key, font.clone());
    Ok(font)
}

fn load_preview_fonts(
    paths: &[PathBuf],
    config: &FontConfig,
    enabled: bool,
) -> Result<Vec<Arc<FontLoader>>> {
    if !enabled {
        return Ok(Vec::new());
    }
    paths
        .iter()
        .map(|path| load_preview_font(path, config))
        .collect()
}

fn load_preview_unicode_manager() -> Arc<UnicodeDataManager> {
    let (data_path, blocks_path) = crate::unicode_data::find_unicode_data_files();
    let key = format!("{}|{}", data_path.display(), blocks_path.display());
    if let Ok(cache) = PREVIEW_UNICODE_CACHE.lock() {
        if let Some(manager) = cache.get(&key).cloned() {
            return manager;
        }
    }

    let manager = Arc::new(
        UnicodeDataManager::load(&data_path, &blocks_path)
            .unwrap_or_else(|_| UnicodeDataManager::empty()),
    );
    if let Ok(mut cache) = PREVIEW_UNICODE_CACHE.lock() {
        cache.insert(key, manager.clone());
    }
    manager
}

#[derive(Clone, Default)]
struct RenderState {
    current_bg_color: Color,
    text_colors: HashMap<String, Color>,
    text_positions: HashMap<String, Position>,
    animating_events: Vec<ActiveAnimation>,
    pause_frames_remaining: u64,
}

struct FrameData {
    index: u64,
    data: Vec<u8>,
}

#[derive(Clone, Copy)]
struct SegmentRenderTask {
    frame_offset: usize,
    segment_index: usize,
}

struct FrameJob {
    index: u64,
    entry_index: usize,
    background_color: Color,
    main_text_color: Color,
    main_position: Position,
    bottom_text_color: Color,
    bottom_position: Position,
    main_font_index: Option<usize>,
    main_glyphs: Vec<ResolvedGlyph>,
}

#[derive(Clone)]
struct ResolvedGlyph {
    selector: GlyphSelector,
    font_index: usize,
}

#[derive(Clone)]
struct ActiveAnimation {
    start_frame: u64,
    duration_frames: u64,
    curve: AnimationCurve,
    animation_type: AnimationType,
}

#[derive(Clone)]
enum AnimationType {
    MoveText {
        element_id: String,
        start_pos: Position,
        end_pos: Position,
    },
    ColorTransition {
        start_color: Color,
        end_color: Color,
    },
}

/// Precompose text alpha over the opaque video background. Glyph coverage is
/// applied later exactly once, preventing overlap from multiplying alpha.
fn flatten_color_over_background(fg: &Color, bg: &Color, opacity_scale: f32) -> (u8, u8, u8) {
    let opacity_scale = opacity_scale.clamp(0.0, 1.0);
    let alpha = ((fg.a as f32 * opacity_scale).round() as u16).min(255);
    let inv_alpha = 255u16 - alpha;

    let blend_channel = |foreground: u8, background: u8| -> u8 {
        let value = foreground as u32 * alpha as u32 + background as u32 * inv_alpha as u32;
        ((value + 127) / 255) as u8
    };

    (
        blend_channel(fg.r, bg.r),
        blend_channel(fg.g, bg.g),
        blend_channel(fg.b, bg.b),
    )
}

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    Color {
        r: ((a.r as f64) * (1.0 - t) + (b.r as f64) * t) as u8,
        g: ((a.g as f64) * (1.0 - t) + (b.g as f64) * t) as u8,
        b: ((a.b as f64) * (1.0 - t) + (b.b as f64) * t) as u8,
        a: ((a.a as f64) * (1.0 - t) + (b.a as f64) * t) as u8,
    }
}

fn lerp_position(a: Position, b: Position, t: f64) -> Position {
    Position {
        x: a.x * (1.0 - t) + b.x * t,
        y: a.y * (1.0 - t) + b.y * t,
    }
}

/// Center a glyph's typographic cell while preserving its bearings.
///
/// The returned y coordinate is the baseline used by `ab_glyph`.  Centering an
/// outline instead would move naturally raised/lowered characters (such as
/// modifier letters U+02C6 and U+02C7) into the visual center of the frame.
fn typographic_glyph_origin(
    center: (i32, i32),
    advance: f32,
    outline_center_x: Option<f32>,
    ascent: f32,
    descent: f32,
) -> (i32, i32) {
    // A zero-advance combining mark has no horizontal typographic cell to
    // center. Use only its outline center on that axis; its vertical placement
    // must still come from the common font baseline.
    let origin_x = if advance.abs() <= f32::EPSILON {
        center.0 - outline_center_x.unwrap_or(0.0).round() as i32
    } else {
        center.0 - (advance / 2.0).round() as i32
    };

    (
        origin_x,
        center.1 + ((ascent + descent) / 2.0).round() as i32,
    )
}

fn measure_text_width(text: &str, fonts: &[Arc<FontLoader>]) -> i32 {
    let mut width = 0i32;
    for c in text.chars() {
        if let Some(font) = fonts
            .iter()
            .find(|font| font.has_char(c))
            .or_else(|| fonts.last())
        {
            width += font.glyph_width(c) as i32;
        }
    }
    width
}

fn wrap_text(
    text: &str,
    max_width: i32,
    fonts: &[Arc<FontLoader>],
    _font_size: i32,
) -> Vec<String> {
    let mut result = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            result.push(String::new());
            continue;
        }

        if measure_text_width(line, fonts) <= max_width {
            result.push(line.to_string());
        } else {
            let mut current_line = String::new();
            let mut current_width = 0i32;

            for c in line.chars() {
                let Some(font) = fonts
                    .iter()
                    .find(|font| font.has_char(c))
                    .or_else(|| fonts.last())
                else {
                    continue;
                };
                let char_width = font.glyph_width(c) as i32;

                if !current_line.is_empty() && current_width + char_width > max_width {
                    result.push(current_line.clone());
                    current_line = c.to_string();
                    current_width = char_width;
                } else {
                    current_line.push(c);
                    current_width += char_width;
                }
            }

            if !current_line.is_empty() {
                result.push(current_line);
            }
        }
    }

    result
}

fn update_animations(state: &mut RenderState, frame: u64) {
    let animations = std::mem::take(&mut state.animating_events);

    for anim in animations {
        let elapsed = frame.saturating_sub(anim.start_frame);
        if elapsed >= anim.duration_frames {
            match &anim.animation_type {
                AnimationType::MoveText {
                    element_id,
                    end_pos,
                    ..
                } => {
                    state
                        .text_positions
                        .insert(element_id.clone(), end_pos.clone());
                }
                AnimationType::ColorTransition { end_color, .. } => {
                    state.current_bg_color = end_color.clone();
                }
            }
            continue;
        }

        let t = anim
            .curve
            .apply(elapsed as f64 / anim.duration_frames as f64);
        match &anim.animation_type {
            AnimationType::MoveText {
                element_id,
                start_pos,
                end_pos,
            } => {
                let position = lerp_position(start_pos.clone(), end_pos.clone(), t);
                state.text_positions.insert(element_id.clone(), position);
            }
            AnimationType::ColorTransition {
                start_color,
                end_color,
            } => {
                state.current_bg_color = lerp_color(start_color.clone(), end_color.clone(), t);
            }
        }
        state.animating_events.push(anim);
    }
}

fn process_events(
    config: &RenderConfig,
    frame: u64,
    state: &mut RenderState,
    color_index: &mut usize,
    is_new_char: bool,
) {
    for event in &config.events {
        if event.frame == frame {
            match &event.event_type {
                EventType::SetBackgroundColor { color } => {
                    state.current_bg_color = color.clone();
                }
                EventType::SetTextColor { element_id, color } => {
                    state.text_colors.insert(element_id.clone(), color.clone());
                }
                EventType::SetTextPosition {
                    element_id,
                    position,
                } => {
                    state
                        .text_positions
                        .insert(element_id.clone(), position.clone());
                }
                EventType::MoveTextPosition {
                    element_id,
                    start_position,
                    end_position,
                    duration,
                    curve,
                } => {
                    let duration_frames = duration_to_frames(*duration, config.fps);
                    state.animating_events.push(ActiveAnimation {
                        start_frame: frame,
                        duration_frames,
                        curve: curve.clone(),
                        animation_type: AnimationType::MoveText {
                            element_id: element_id.clone(),
                            start_pos: start_position.clone(),
                            end_pos: end_position.clone(),
                        },
                    });
                }
                EventType::ColorTransition {
                    start_color,
                    end_color,
                    duration,
                    curve,
                } => {
                    let duration_frames = duration_to_frames(*duration, config.fps);
                    state.animating_events.push(ActiveAnimation {
                        start_frame: frame,
                        duration_frames,
                        curve: curve.clone(),
                        animation_type: AnimationType::ColorTransition {
                            start_color: start_color.clone(),
                            end_color: end_color.clone(),
                        },
                    });
                }
                EventType::Pause { duration } => {
                    state.pause_frames_remaining = state
                        .pause_frames_remaining
                        .saturating_add(duration_to_frames(*duration, config.fps));
                }
            }
        }
    }

    if config.dynamic_background
        && !config.fixed_background
        && config.events.is_empty()
        && is_new_char
    {
        state.current_bg_color = config.background_colors[*color_index].clone();
        *color_index = (*color_index + 1) % config.background_colors.len();
    }
}

pub fn detect_encoder(ffmpeg_path: &Path, hevc: bool) -> Option<String> {
    let candidates: &[&str] = if hevc {
        &["hevc_qsv", "hevc_nvenc", "libx265"]
    } else {
        &["h264_qsv", "h264_nvenc", "libx264"]
    };
    find_working_encoder(ffmpeg_path, candidates)
}

pub fn detect_hardware_encoder(ffmpeg_path: &Path) -> Option<String> {
    find_working_encoder(ffmpeg_path, &["h264_qsv", "h264_nvenc"])
}

fn find_working_encoder(ffmpeg_path: &Path, candidates: &[&str]) -> Option<String> {
    let output = Command::new(ffmpeg_path)
        .args(["-hide_banner", "-encoders"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let available = String::from_utf8_lossy(&output.stdout);
    candidates
        .iter()
        .copied()
        .find(|encoder| {
            available.contains(encoder) && test_encoder(ffmpeg_path, encoder).unwrap_or(false)
        })
        .map(str::to_owned)
}

pub fn test_encoder(ffmpeg_path: &Path, encoder: &str) -> Result<bool> {
    let status = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=1920x1080:rate=5:duration=1",
            "-pix_fmt",
            "yuv420p",
            "-frames:v",
            "5",
            "-c:v",
            encoder,
            "-f",
            "null",
            "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("Failed to start FFmpeg for encoder test: {encoder}"))?;
    Ok(status.success())
}

pub trait ProgressReporter: Send + Sync {
    fn inc(&self, n: u64);
    fn finish(&self, message: &str);
}

struct IndicatifProgressReporter {
    bar: ProgressBar,
}

impl IndicatifProgressReporter {
    fn new(total: u64) -> Self {
        let bar = ProgressBar::new(total);
        if let Ok(style) = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta})")
        {
            bar.set_style(style.progress_chars("=> "));
        }
        Self { bar }
    }
}

impl ProgressReporter for IndicatifProgressReporter {
    fn inc(&self, n: u64) {
        self.bar.inc(n);
    }

    fn finish(&self, message: &str) {
        self.bar.finish_with_message(message.to_string());
    }
}

struct CallbackProgressReporter {
    current: AtomicU64,
    total: u64,
    callback: Arc<dyn Fn(u64, u64) + Send + Sync>,
}

impl CallbackProgressReporter {
    fn new(total: u64, callback: Arc<dyn Fn(u64, u64) + Send + Sync>) -> Self {
        Self {
            current: AtomicU64::new(0),
            total,
            callback,
        }
    }
}

impl ProgressReporter for CallbackProgressReporter {
    fn inc(&self, n: u64) {
        let current = self.current.fetch_add(n, Ordering::Relaxed) + n;
        (self.callback)(current.min(self.total), self.total);
    }

    fn finish(&self, _message: &str) {
        (self.callback)(self.total, self.total);
    }
}

fn validate_position(position: &Position, label: &str) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
        anyhow::bail!("{label} must contain finite coordinates");
    }
    Ok(())
}

fn validate_render_config(config: &RenderConfig, require_frames: bool) -> Result<()> {
    let (width, height) = config.resolution;
    if width == 0 || height == 0 {
        anyhow::bail!("Resolution must be greater than zero");
    }
    if !config.fps.is_finite() || config.fps <= 0.0 {
        anyhow::bail!("FPS must be a finite value greater than zero");
    }
    if require_frames && config.total_frames() == 0 {
        anyhow::bail!("The render contains no frames");
    }
    if require_frames && config.output_path.as_os_str().is_empty() {
        anyhow::bail!("Output path must not be empty");
    }
    if config.dynamic_background
        && !config.fixed_background
        && config.events.is_empty()
        && config.background_colors.is_empty()
    {
        anyhow::bail!("Dynamic background requires at least one color");
    }
    if config.main_text.id.trim().is_empty() || config.bottom_text.id.trim().is_empty() {
        anyhow::bail!("Text element IDs must not be empty");
    }
    if config.main_text.id == config.bottom_text.id {
        anyhow::bail!("Main and bottom text must use distinct element IDs");
    }

    for (label, element, font) in [
        ("main text", &config.main_text, &config.main_font),
        ("bottom text", &config.bottom_text, &config.bottom_font),
    ] {
        validate_position(&element.position, label)?;
        if !element.max_width.is_finite() || element.max_width < 0.0 {
            anyhow::bail!("{label} max_width must be finite and non-negative");
        }
        if element.enabled {
            if !font.size.is_finite() || font.size <= 0.0 {
                anyhow::bail!("{label} font size must be finite and greater than zero");
            }
            if element.fonts.is_empty() {
                anyhow::bail!("No {label} font configured");
            }
        }
    }

    for (index, entry) in config.characters.iter().enumerate() {
        if let Some(position) = &entry.position {
            validate_position(position, &format!("character {index} position"))?;
        }
    }
    for (index, event) in config.events.iter().enumerate() {
        match &event.event_type {
            EventType::SetTextPosition { position, .. } => {
                validate_position(position, &format!("event {index} position"))?;
            }
            EventType::MoveTextPosition {
                start_position,
                end_position,
                duration,
                ..
            } => {
                validate_position(start_position, &format!("event {index} start position"))?;
                validate_position(end_position, &format!("event {index} end position"))?;
                if !duration.is_finite() || *duration < 0.0 {
                    anyhow::bail!("event {index} duration must be finite and non-negative");
                }
            }
            EventType::ColorTransition { duration, .. } | EventType::Pause { duration } => {
                if !duration.is_finite() || *duration < 0.0 {
                    anyhow::bail!("event {index} duration must be finite and non-negative");
                }
            }
            EventType::SetBackgroundColor { .. } | EventType::SetTextColor { .. } => {}
        }
    }

    if require_frames {
        if config.ffmpeg.path.as_os_str().is_empty() {
            anyhow::bail!("FFmpeg path must not be empty");
        }
        if config.ffmpeg.encoder.trim().is_empty() {
            anyhow::bail!("FFmpeg encoder must not be empty");
        }
        if config.ffmpeg.pixel_format.trim().is_empty() {
            anyhow::bail!("FFmpeg pixel format must not be empty");
        }
        if config.ffmpeg.crf > 51 {
            anyhow::bail!("FFmpeg CRF must be between 0 and 51");
        }
    }
    Ok(())
}

pub fn render_video(config: &RenderConfig) -> Result<()> {
    render_video_with_progress(config, None)
}

/// Render a single configured character entry as a PNG. This deliberately
/// uses the same glyph parsing, fallback selection, and drawing path as video
/// rendering so CLI and GUI previews match the final result.
pub fn render_frame_png(
    config: &RenderConfig,
    entry_index: usize,
    max_dimension: u32,
) -> Result<Vec<u8>> {
    validate_render_config(config, false)?;
    let entry = config
        .characters
        .get(entry_index)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Character index {entry_index} is out of range"))?;
    let mut preview_config = config.clone();
    let (source_width, source_height) = config.resolution;
    let largest_edge = source_width.max(source_height);
    if max_dimension > 0 && largest_edge > max_dimension {
        let scale = max_dimension as f32 / largest_edge as f32;
        preview_config.resolution = (
            (source_width as f32 * scale).round().max(1.0) as u32,
            (source_height as f32 * scale).round().max(1.0) as u32,
        );
        preview_config.main_font.size *= scale;
        preview_config.bottom_font.size *= scale;
        preview_config.text_x_offset = (preview_config.text_x_offset as f32 * scale).round() as i32;
        preview_config.text_y_offset = (preview_config.text_y_offset as f32 * scale).round() as i32;
    }

    let main_fonts = load_preview_fonts(
        &preview_config.main_text.fonts,
        &preview_config.main_font,
        preview_config.main_text.enabled,
    )?;
    let bottom_fonts = load_preview_fonts(
        &preview_config.bottom_text.fonts,
        &preview_config.bottom_font,
        preview_config.bottom_text.enabled,
    )?;

    let unicode_manager = load_preview_unicode_manager();
    let main_glyphs = resolve_glyphs(&entry, &main_fonts);
    let main_font_index = main_glyphs.first().map(|glyph| glyph.font_index);
    let background_color = entry
        .background_color
        .clone()
        .unwrap_or_else(|| preview_config.background_color.clone());
    let main_text_color = entry
        .text_color
        .clone()
        .unwrap_or_else(|| preview_config.main_text.color.clone());
    let main_position = entry
        .position
        .clone()
        .unwrap_or_else(|| preview_config.main_text.position.clone());
    let (width, height) = preview_config.resolution;
    let frame_ctx = FrameRenderCtx {
        config: &preview_config,
        entry: &entry,
        frame_index: 0,
        background_color: &background_color,
        main_text_color: &main_text_color,
        main_position: &main_position,
        bottom_text_color: &preview_config.bottom_text.color,
        bottom_position: &preview_config.bottom_text.position,
        main_font_index,
        main_glyphs: &main_glyphs,
        main_fonts: &main_fonts,
        bottom_fonts: &bottom_fonts,
        unicode_manager: &unicode_manager,
        width,
        height,
    };
    let raw = render_frame_parallel(&frame_ctx)?;
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&raw, width, height, ColorType::Rgba8.into())
        .context("Failed to encode preview PNG")?;
    Ok(png)
}

pub fn render_frame_to_file(
    config: &RenderConfig,
    entry_index: usize,
    output: &Path,
    max_dimension: u32,
) -> Result<()> {
    let png = render_frame_png(config, entry_index, max_dimension)?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create preview directory: {}", parent.display()))?;
    }
    std::fs::write(output, png)
        .with_context(|| format!("Failed to write preview PNG: {}", output.display()))
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn create(path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create temporary directory: {}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            eprintln!(
                "Warning: failed to clean up temporary directory {}: {}",
                self.path.display(),
                error
            );
        }
    }
}

struct PreparedRender {
    width: u32,
    height: u32,
    total_frames: u64,
    unicode_manager: Arc<UnicodeDataManager>,
    main_fonts: Vec<Arc<FontLoader>>,
    bottom_fonts: Vec<Arc<FontLoader>>,
    ffmpeg_path: PathBuf,
    encoder: String,
    music_path: Option<PathBuf>,
    progress: Arc<dyn ProgressReporter>,
}

#[derive(Clone, Copy)]
struct FrameRenderer<'a> {
    config: &'a RenderConfig,
    main_fonts: &'a [Arc<FontLoader>],
    bottom_fonts: &'a [Arc<FontLoader>],
    unicode_manager: &'a UnicodeDataManager,
    width: u32,
    height: u32,
}

impl<'a> FrameRenderer<'a> {
    fn new(config: &'a RenderConfig, prepared: &'a PreparedRender) -> Self {
        Self {
            config,
            main_fonts: &prepared.main_fonts,
            bottom_fonts: &prepared.bottom_fonts,
            unicode_manager: prepared.unicode_manager.as_ref(),
            width: prepared.width,
            height: prepared.height,
        }
    }

    fn render_job(self, job: &FrameJob) -> Result<FrameData> {
        let entry = self
            .config
            .characters
            .get(job.entry_index)
            .with_context(|| {
                format!(
                    "Frame {} references missing character {}",
                    job.index, job.entry_index
                )
            })?;
        let frame_ctx = FrameRenderCtx {
            config: self.config,
            entry,
            frame_index: job.index,
            background_color: &job.background_color,
            main_text_color: &job.main_text_color,
            main_position: &job.main_position,
            bottom_text_color: &job.bottom_text_color,
            bottom_position: &job.bottom_position,
            main_font_index: job.main_font_index,
            main_glyphs: &job.main_glyphs,
            main_fonts: self.main_fonts,
            bottom_fonts: self.bottom_fonts,
            unicode_manager: self.unicode_manager,
            width: self.width,
            height: self.height,
        };

        render_frame_parallel(&frame_ctx).map(|data| FrameData {
            index: job.index,
            data,
        })
    }
}

fn prepare_render(
    config: &RenderConfig,
    progress_callback: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<PreparedRender> {
    validate_render_config(config, true)?;
    let (width, height) = config.resolution;
    let total_frames = config.total_frames();

    println!("Rendering video: {}", config.output_path.display());
    println!(
        "Resolution: {}x{}, FPS: {}, Total frames: {}",
        width, height, config.fps, total_frames
    );

    if let Some(parent) = config
        .output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    let (data_path, blocks_path) = crate::unicode_data::find_unicode_data_files();
    let unicode_manager = Arc::new(
        UnicodeDataManager::load(&data_path, &blocks_path).unwrap_or_else(|error| {
            eprintln!("Warning: failed to load Unicode data: {}", error);
            UnicodeDataManager::empty()
        }),
    );

    let main_fonts = load_configured_fonts(
        &config.main_text.fonts,
        &config.main_font,
        "main text",
        config.main_text.enabled,
    )?;
    let bottom_fonts = load_configured_fonts(
        &config.bottom_text.fonts,
        &config.bottom_font,
        "bottom text",
        config.bottom_text.enabled,
    )?;

    let ffmpeg_path = crate::ffmpeg::resolve_ffmpeg_path(Some(config.ffmpeg.path.as_path()));

    let encoder = if config.ffmpeg.encoder == "auto" {
        println!("Detecting FFmpeg encoder...");
        detect_encoder(&ffmpeg_path, false)
            .context("No working H.264 encoder was found in FFmpeg")?
    } else {
        config.ffmpeg.encoder.clone()
    };
    println!("Encoder: {}", encoder);

    let music_path = config.music_path.as_ref().and_then(|music| {
        let resolved = crate::json_config::resolve_asset_reference(music);
        if resolved.exists() {
            Some(resolved)
        } else {
            eprintln!("Warning: music file not found: {}", music.display());
            None
        }
    });

    let progress: Arc<dyn ProgressReporter> = match progress_callback {
        Some(callback) => Arc::new(CallbackProgressReporter::new(total_frames, callback)),
        None => Arc::new(IndicatifProgressReporter::new(total_frames)),
    };

    Ok(PreparedRender {
        width,
        height,
        total_frames,
        unicode_manager,
        main_fonts,
        bottom_fonts,
        ffmpeg_path,
        encoder,
        music_path,
        progress,
    })
}

fn load_configured_fonts(
    paths: &[PathBuf],
    font_config: &FontConfig,
    role: &str,
    enabled: bool,
) -> Result<Vec<Arc<FontLoader>>> {
    if !enabled {
        return Ok(Vec::new());
    }
    let fonts = paths
        .iter()
        .map(|font_path| {
            let resolved_path = crate::json_config::resolve_asset_reference(font_path);
            FontLoader::from_path_with_config(&resolved_path, font_config)
                .map(Arc::new)
                .with_context(|| format!("Failed to load {role} font: {}", resolved_path.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(fonts)
}

fn spawn_ffmpeg(config: &RenderConfig, prepared: &PreparedRender) -> Result<std::process::Child> {
    let mut command = Command::new(&prepared.ffmpeg_path);
    add_rawvideo_input_args(&mut command, prepared.width, prepared.height, config.fps);

    if let Some(music) = prepared.music_path.as_deref() {
        command.arg("-i").arg(music);
    }

    add_stream_mapping_args(&mut command, prepared.music_path.is_some());

    let threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    add_encoder_args(&mut command, config, &prepared.encoder, threads, true);

    if prepared.music_path.is_some() {
        command.arg("-shortest");
        command.arg("-c:a").arg("aac");
    }

    command.arg(&config.output_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start ffmpeg")
}

fn render_worker_settings(config: &RenderConfig) -> (usize, usize) {
    let max_inflight_frames = config.ffmpeg.max_inflight_frames.max(1);
    let requested_workers = if config.ffmpeg.parallel_workers > 0 {
        config.ffmpeg.parallel_workers
    } else {
        std::thread::available_parallelism()
            .map(|count| count.get() / 2)
            .unwrap_or(2)
            .max(1)
    };
    let worker_count = requested_workers.min(max_inflight_frames).max(1);
    (worker_count, max_inflight_frames)
}

fn run_render_loop<W: Write>(
    config: &RenderConfig,
    prepared: &PreparedRender,
    writer: &mut W,
) -> Result<()> {
    let (worker_count, max_inflight_frames) = render_worker_settings(config);
    println!(
        "Render threads: {}, max in-flight frames: {}",
        worker_count, max_inflight_frames
    );

    let (job_sender, job_receiver) = bounded::<FrameJob>(max_inflight_frames);
    let (result_sender, result_receiver) = unbounded::<std::result::Result<FrameData, String>>();
    let frame_renderer = FrameRenderer::new(config, prepared);

    std::thread::scope(|scope| -> Result<()> {
        for _ in 0..worker_count {
            let jobs = job_receiver.clone();
            let results = result_sender.clone();
            scope.spawn(move || {
                while let Ok(job) = jobs.recv() {
                    let rendered = frame_renderer.render_job(&job).map_err(|error| {
                        format!("Failed to render frame {}: {error:#}", job.index)
                    });
                    if results.send(rendered).is_err() {
                        break;
                    }
                }
            });
        }
        drop(result_sender);

        let mut state = initial_render_state(config);
        let mut color_index = 0usize;
        let mut entry_index = 0usize;
        let mut frame_in_char = 0u64;
        let mut next_job_index = 0u64;
        let mut expected_write_index = 0u64;
        let mut inflight = 0usize;
        let mut source_exhausted = false;
        let mut completed_out_of_order = BTreeMap::<u64, Vec<u8>>::new();

        while inflight < max_inflight_frames && !source_exhausted {
            match next_frame_job(
                config,
                &prepared.main_fonts,
                &mut state,
                &mut color_index,
                &mut entry_index,
                &mut frame_in_char,
                &mut next_job_index,
            ) {
                Some(job) => {
                    job_sender
                        .send(job)
                        .context("Render worker thread exited unexpectedly")?;
                    inflight += 1;
                }
                None => source_exhausted = true,
            }
        }

        while inflight > 0 {
            let rendered = result_receiver
                .recv()
                .context("All render worker threads exited while frames are pending")?;
            let frame = rendered.map_err(anyhow::Error::msg)?;
            completed_out_of_order.insert(frame.index, frame.data);

            while let Some(data) = completed_out_of_order.remove(&expected_write_index) {
                writer.write_all(&data).with_context(|| {
                    format!("Failed to write frame {} to ffmpeg", expected_write_index)
                })?;
                expected_write_index += 1;
                inflight -= 1;
                prepared.progress.inc(1);

                if !source_exhausted {
                    match next_frame_job(
                        config,
                        &prepared.main_fonts,
                        &mut state,
                        &mut color_index,
                        &mut entry_index,
                        &mut frame_in_char,
                        &mut next_job_index,
                    ) {
                        Some(job) => {
                            job_sender
                                .send(job)
                                .context("Render worker thread exited unexpectedly")?;
                            inflight += 1;
                        }
                        None => source_exhausted = true,
                    }
                }
            }
        }

        drop(job_sender);
        writer.flush().context("Failed to flush ffmpeg stdin")
    })
}

fn render_video_single_process(config: &RenderConfig, prepared: &PreparedRender) -> Result<()> {
    let mut process = spawn_ffmpeg(config, prepared)?;
    let stdin = process.stdin.take().context("Failed to get ffmpeg stdin")?;
    let mut writer = BufWriter::new(stdin);

    let render_result = run_render_loop(config, prepared, &mut writer);
    drop(writer);
    if let Err(error) = render_result {
        let _ = process.kill();
        let _ = process.wait();
        return Err(error);
    }
    let status = process.wait().context("Failed to wait for ffmpeg")?;
    if !status.success() {
        anyhow::bail!("ffmpeg exited with status {:?}", status.code());
    }
    prepared.progress.finish("Rendering complete");

    println!("Video saved: {}", config.output_path.display());
    Ok(())
}

/// `progress_callback` receives `(completed_frames, total_frames)`.
pub fn render_video_with_progress(
    config: &RenderConfig,
    progress_callback: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<()> {
    let prepared = prepare_render(config, progress_callback)?;

    if config.ffmpeg.encoding_processes.max(1) > 1 && prepared.total_frames > 1 {
        render_video_multi_process(config, &prepared)
    } else {
        render_video_single_process(config, &prepared)
    }
}

/// Precompute every mutable decision before starting parallel encoders.
fn precompute_frame_schedule(
    config: &RenderConfig,
    main_fonts: &[Arc<FontLoader>],
) -> Vec<FrameJob> {
    let mut frames = Vec::new();
    let mut state = initial_render_state(config);
    let mut color_index = 0usize;
    let mut entry_index = 0usize;
    let mut frame_in_char = 0u64;
    let mut frame_index = 0u64;

    while let Some(job) = next_frame_job(
        config,
        main_fonts,
        &mut state,
        &mut color_index,
        &mut entry_index,
        &mut frame_in_char,
        &mut frame_index,
    ) {
        frames.push(job);
    }

    frames
}

fn add_rawvideo_input_args(cmd: &mut Command, width: u32, height: u32, fps: f64) {
    // Keep FFmpeg's raw-frame queue minimal; application-side backpressure is authoritative.
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("warning")
        .arg("-nostdin")
        .arg("-thread_queue_size")
        .arg("1")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pixel_format")
        .arg("rgba")
        .arg("-video_size")
        .arg(format!("{}x{}", width, height))
        .arg("-framerate")
        .arg(fps.to_string())
        .arg("-analyzeduration")
        .arg("0")
        .arg("-probesize")
        .arg("32")
        .arg("-i")
        .arg("-");
}

fn add_stream_mapping_args(cmd: &mut Command, include_audio: bool) {
    cmd.arg("-map").arg("0:v:0");
    if include_audio {
        cmd.arg("-map").arg("1:a:0?");
    }
}

/// `faststart` is enabled only for the final output, not temporary segments.
fn add_encoder_args(
    cmd: &mut Command,
    config: &RenderConfig,
    encoder: &str,
    threads_per_process: usize,
    faststart: bool,
) {
    if encoder.contains("qsv") {
        cmd.arg("-c:v").arg(encoder);
        cmd.arg("-global_quality")
            .arg(config.ffmpeg.crf.clamp(0, 51).to_string());
        cmd.arg("-pix_fmt").arg("nv12");
    } else if encoder.contains("nvenc") {
        cmd.arg("-c:v").arg(encoder);
        cmd.arg("-cq").arg(config.ffmpeg.crf.to_string());
        cmd.arg("-pix_fmt").arg(&config.ffmpeg.pixel_format);
    } else if encoder.contains("x264") || encoder.contains("x265") {
        cmd.arg("-c:v").arg(encoder);
        cmd.arg("-crf").arg(config.ffmpeg.crf.to_string());
        cmd.arg("-preset").arg(&config.ffmpeg.preset);
        cmd.arg("-tune").arg("animation");
        cmd.arg("-pix_fmt").arg(&config.ffmpeg.pixel_format);
        if encoder.contains("x264") {
            cmd.arg("-profile:v").arg("high");
            cmd.arg("-x264-params")
                .arg("deblock=-1,-1:fast_pskip=0:aq-strength=0.8:ref=4:rc-lookahead=30:b-adapt=2");
        }
        cmd.arg("-threads").arg(threads_per_process.to_string());
    } else {
        cmd.arg("-c:v").arg(encoder);
        cmd.arg("-pix_fmt").arg(&config.ffmpeg.pixel_format);
        cmd.arg("-threads").arg(threads_per_process.to_string());
    }

    let rounded_fps = config.fps.round().max(1.0) as i32;
    cmd.arg("-g").arg((rounded_fps * 2).to_string());
    cmd.arg("-keyint_min").arg(rounded_fps.to_string());

    cmd.arg("-color_primaries").arg("bt709");
    cmd.arg("-color_trc").arg("bt709");
    cmd.arg("-colorspace").arg("bt709");

    if faststart {
        cmd.arg("-movflags").arg("+faststart");
    }
}

struct SegmentEncoderConfig<'a> {
    render_config: &'a RenderConfig,
    ffmpeg_path: &'a Path,
    encoder: &'a str,
    output_path: &'a Path,
    width: u32,
    height: u32,
    threads_per_process: usize,
}

struct SegmentFrameStream {
    rendered_frames: crossbeam_channel::Receiver<std::result::Result<FrameData, String>>,
    frame_permits: crossbeam_channel::Sender<()>,
    expected_start: u64,
    expected_end: u64,
    progress: Arc<dyn ProgressReporter>,
}

fn write_encoded_segment(
    encoder_config: SegmentEncoderConfig<'_>,
    frame_stream: SegmentFrameStream,
) -> Result<()> {
    let SegmentEncoderConfig {
        render_config,
        ffmpeg_path,
        encoder,
        output_path,
        width,
        height,
        threads_per_process,
    } = encoder_config;

    let SegmentFrameStream {
        rendered_frames,
        frame_permits,
        expected_start,
        expected_end,
        progress,
    } = frame_stream;

    let mut cmd = Command::new(ffmpeg_path);
    add_rawvideo_input_args(&mut cmd, width, height, render_config.fps);
    add_stream_mapping_args(&mut cmd, false);
    add_encoder_args(&mut cmd, render_config, encoder, threads_per_process, false);
    cmd.arg(output_path);

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to start segment encoder: {}", output_path.display()))?;

    let stdin = child
        .stdin
        .take()
        .context("Failed to get segment encoder stdin")?;

    let mut writer = BufWriter::new(stdin);

    let write_result = (|| -> Result<()> {
        let mut expected_index = expected_start;
        let mut completed_out_of_order = BTreeMap::<u64, Vec<u8>>::new();

        while let Ok(rendered) = rendered_frames.recv() {
            let frame = rendered.map_err(anyhow::Error::msg)?;

            if frame.index < expected_start || frame.index >= expected_end {
                anyhow::bail!(
                    "Segment received out-of-range frame {} (expected {}..{})",
                    frame.index,
                    expected_start,
                    expected_end
                );
            }

            completed_out_of_order.insert(frame.index, frame.data);

            while let Some(data) = completed_out_of_order.remove(&expected_index) {
                writer.write_all(&data).with_context(|| {
                    format!(
                        "Failed to write frame {} to segment encoder",
                        expected_index
                    )
                })?;

                let _ = frame_permits.send(());
                expected_index += 1;
                progress.inc(1);
            }
        }

        if expected_index != expected_end {
            anyhow::bail!(
                "Segment frame range incomplete: wrote {}, expected {}",
                expected_index,
                expected_end
            );
        }

        writer
            .flush()
            .context("Failed to flush segment encoder stdin")?;

        Ok(())
    })();

    drop(writer);

    if let Err(error) = write_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }

    let status = child.wait().context("Failed to wait for segment encoder")?;

    if !status.success() {
        anyhow::bail!(
            "Segment encoding failed: {} (exit code {:?})",
            output_path.display(),
            status.code()
        );
    }

    Ok(())
}

fn concat_file_entry(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file '{}'\n", normalized.replace('\'', "'\\''"))
}

fn concatenate_segments(
    ffmpeg_path: &Path,
    segment_paths: &[std::path::PathBuf],
    list_path: &Path,
    output_path: &Path,
) -> Result<()> {
    let mut list = String::new();
    for path in segment_paths {
        list.push_str(&concat_file_entry(path));
    }
    std::fs::write(list_path, list).with_context(|| {
        format!(
            "Failed to write FFmpeg concat list: {}",
            list_path.display()
        )
    })?;

    let status = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("warning")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(list_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-c")
        .arg("copy")
        .arg(output_path)
        .status()
        .context("Failed to start FFmpeg segment concatenation")?;
    if !status.success() {
        anyhow::bail!(
            "FFmpeg segment concatenation failed (exit code {:?})",
            status.code()
        );
    }
    Ok(())
}

fn mux_music(
    ffmpeg_path: &Path,
    video_path: &Path,
    music_path: &Path,
    output_path: &Path,
) -> Result<()> {
    let status = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("warning")
        .arg("-i")
        .arg(video_path)
        .arg("-i")
        .arg(music_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("1:a:0?")
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg(output_path)
        .status()
        .context("Failed to start FFmpeg audio mux")?;
    if !status.success() {
        anyhow::bail!("FFmpeg audio mux failed (exit code {:?})", status.code());
    }
    Ok(())
}

fn render_video_multi_process(config: &RenderConfig, prepared: &PreparedRender) -> Result<()> {
    println!("Precomputing frame schedule...");
    let frames = precompute_frame_schedule(config, &prepared.main_fonts);
    let frame_renderer = FrameRenderer::new(config, prepared);

    let process_count = config
        .ffmpeg
        .encoding_processes
        .max(1)
        .min(frames.len().max(1));
    let chunk_size = frames.len().div_ceil(process_count);
    let segment_ranges: Vec<_> = (0..frames.len())
        .step_by(chunk_size)
        .map(|start| start..(start + chunk_size).min(frames.len()))
        .collect();
    let available_threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    let requested_render_workers = if config.ffmpeg.parallel_workers > 0 {
        config.ffmpeg.parallel_workers
    } else {
        (available_threads / 2).max(1)
    };
    let render_worker_count = requested_render_workers
        .min(available_threads.max(1))
        .min(frames.len().max(1))
        .max(1);
    let encoder_thread_budget = available_threads
        .saturating_sub(render_worker_count.min(available_threads - 1))
        .max(process_count);
    let threads_per_process = (encoder_thread_budget / process_count).max(1);
    let requested_frame_queue = config.ffmpeg.max_inflight_frames.max(1);
    let minimum_queue_per_process = usize::from(frames.len() > process_count) + 1;
    let minimum_pipeline_queue = process_count * minimum_queue_per_process;
    let actual_frame_queue = requested_frame_queue.max(minimum_pipeline_queue);
    let base_queue_capacity = actual_frame_queue / process_count;
    let extra_queue_slots = actual_frame_queue % process_count;

    let output_parent = config
        .output_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let output_stem = config
        .output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let temp_dir = TemporaryDirectory::create(output_parent.join(format!(
        ".{}.ufm-segments-{}",
        output_stem,
        uuid::Uuid::new_v4()
    )))?;

    let segment_paths: Vec<_> = (0..process_count)
        .map(|index| temp_dir.path().join(format!("segment-{:04}.mp4", index)))
        .collect();

    println!(
        "Pipeline: {} Rust render thread(s) → {} FFmpeg encoder(s)",
        render_worker_count, process_count
    );

    std::thread::scope(|scope| -> Result<()> {
        // Metadata jobs are cheap; per-encoder bounded channels limit complete RGBA frames.
        let (task_sender, task_receiver) = unbounded::<SegmentRenderTask>();
        let mut rendered_senders = Vec::new();
        let mut rendered_receivers = Vec::new();
        let mut permit_senders = Vec::new();
        let mut permit_receivers = Vec::new();
        for segment_index in 0..process_count {
            let capacity = base_queue_capacity + usize::from(segment_index < extra_queue_slots);
            let (sender, receiver) = bounded::<std::result::Result<FrameData, String>>(capacity);
            rendered_senders.push(sender);
            rendered_receivers.push(receiver);

            // A permit covers a frame from the start of rasterization until it
            // has been written to FFmpeg. This also bounds the writer's local
            // out-of-order map, not merely the channel itself.
            let (permit_sender, permit_receiver) = bounded::<()>(capacity);
            for _ in 0..capacity {
                permit_sender
                    .send(())
                    .context("Failed to initialize frame-permit channel")?;
            }
            permit_senders.push(permit_sender);
            permit_receivers.push(permit_receiver);
        }

        // Each FFmpeg owns a dedicated consumer thread. It can block on stdin
        // without blocking rendering or any other encoder.
        let mut encoder_handles = Vec::new();
        for (index, (receiver, permit_sender)) in rendered_receivers
            .into_iter()
            .zip(permit_senders)
            .enumerate()
        {
            let segment_path = &segment_paths[index];
            let progress = Arc::clone(&prepared.progress);

            let range = segment_ranges[index].clone();
            let expected_start = frames[range.start].index;
            let expected_end = frames[range.end - 1].index + 1;

            let ffmpeg_path = prepared.ffmpeg_path.as_path();
            let encoder = prepared.encoder.as_str();
            let width = prepared.width;
            let height = prepared.height;

            encoder_handles.push(scope.spawn(move || {
                write_encoded_segment(
                    SegmentEncoderConfig {
                        render_config: config,
                        ffmpeg_path,
                        encoder,
                        output_path: segment_path,
                        width,
                        height,
                        threads_per_process,
                    },
                    SegmentFrameStream {
                        rendered_frames: receiver,
                        frame_permits: permit_sender,
                        expected_start,
                        expected_end,
                        progress,
                    },
                )
            }));
        }

        let rendered_senders = Arc::new(rendered_senders);
        let permit_receivers = Arc::new(permit_receivers);
        let frames_ref = &frames;

        // Shared renderer pool: parallel_workers now applies to multi-process
        // mode too. Completed frames go directly to their encoder's own queue.
        let mut render_handles = Vec::new();
        for _ in 0..render_worker_count {
            let tasks = task_receiver.clone();
            let outputs = Arc::clone(&rendered_senders);
            let permits = Arc::clone(&permit_receivers);
            render_handles.push(scope.spawn(move || -> Result<()> {
                while let Ok(task) = tasks.recv() {
                    permits[task.segment_index].recv().map_err(|_| {
                        anyhow::anyhow!(
                            "FFmpeg encoder {} frame-permit channel closed unexpectedly",
                            task.segment_index
                        )
                    })?;
                    let job = &frames_ref[task.frame_offset];
                    let rendered = frame_renderer.render_job(job).map_err(|error| {
                        format!("Failed to render frame {}: {error:#}", job.index)
                    });

                    outputs[task.segment_index].send(rendered).map_err(|_| {
                        anyhow::anyhow!(
                            "FFmpeg encoder {} input channel closed unexpectedly",
                            task.segment_index
                        )
                    })?;
                }
                Ok(())
            }));
        }
        drop(task_receiver);

        // Interleave segments so every FFmpeg receives early work instead of
        // filling the first segment's bounded queue and starving the others.
        let longest_segment = segment_ranges
            .iter()
            .map(|range| range.len())
            .max()
            .unwrap_or(0);
        for offset in 0..longest_segment {
            for (segment_index, range) in segment_ranges.iter().enumerate() {
                let frame_offset = range.start + offset;
                if frame_offset < range.end {
                    task_sender
                        .send(SegmentRenderTask {
                            frame_offset,
                            segment_index,
                        })
                        .context("Render task queue closed unexpectedly")?;
                }
            }
        }
        drop(task_sender);

        for handle in render_handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Rust render thread panicked"))??;
        }
        // Closing the final senders gives every encoder an explicit EOF once
        // all rendered frames already in its queue have been consumed.
        drop(rendered_senders);
        drop(permit_receivers);

        for handle in encoder_handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("FFmpeg encoder thread panicked"))??;
        }
        Ok(())
    })?;

    prepared
        .progress
        .finish("Encoding segments, concatenating...");
    let concat_list = temp_dir.path().join("segments.txt");
    if let Some(music) = prepared.music_path.as_deref() {
        let joined_video = temp_dir.path().join("joined-video.mp4");
        concatenate_segments(
            &prepared.ffmpeg_path,
            &segment_paths,
            &concat_list,
            &joined_video,
        )?;
        mux_music(
            &prepared.ffmpeg_path,
            &joined_video,
            music,
            &config.output_path,
        )?;
    } else {
        concatenate_segments(
            &prepared.ffmpeg_path,
            &segment_paths,
            &concat_list,
            &config.output_path,
        )?;
    }

    println!("Video saved: {}", config.output_path.display());
    Ok(())
}

fn initial_render_state(config: &RenderConfig) -> RenderState {
    RenderState {
        current_bg_color: config.background_color.clone(),
        ..RenderState::default()
    }
}

/// Resolve one frame lazily so memory scales with `max_inflight_frames`, not video length.
fn resolve_glyphs(entry: &CharEntry, main_fonts: &[Arc<FontLoader>]) -> Vec<ResolvedGlyph> {
    if main_fonts.is_empty() {
        return Vec::new();
    }
    entry
        .glyph_specs()
        .into_iter()
        .map(|spec: GlyphSpec| {
            if let Some(font_index) = spec.font_index {
                let font_index = font_index.min(main_fonts.len() - 1);
                let selector = if main_fonts[font_index].has_selector(&spec.selector) {
                    spec.selector
                } else {
                    GlyphSelector::Name(".notdef".to_string())
                };
                return ResolvedGlyph {
                    selector,
                    font_index,
                };
            }
            match main_fonts
                .iter()
                .position(|font| font.has_selector(&spec.selector))
            {
                Some(font_index) => ResolvedGlyph {
                    selector: spec.selector,
                    font_index,
                },
                None => ResolvedGlyph {
                    selector: GlyphSelector::Name(".notdef".to_string()),
                    font_index: main_fonts.len() - 1,
                },
            }
        })
        .collect()
}

fn next_frame_job(
    config: &RenderConfig,
    main_fonts: &[Arc<FontLoader>],
    state: &mut RenderState,
    color_index: &mut usize,
    entry_index: &mut usize,
    frame_in_char: &mut u64,
    frame_index: &mut u64,
) -> Option<FrameJob> {
    loop {
        let entry = config.characters.get(*entry_index)?;
        let duration = entry.duration();

        if *frame_in_char >= duration {
            *entry_index += 1;
            *frame_in_char = 0;
            continue;
        }

        let is_new_char = *frame_in_char == 0;
        process_events(config, *frame_index, state, color_index, is_new_char);
        update_animations(state, *frame_index);
        let is_paused = state.pause_frames_remaining > 0;

        // Control code points use the same main-font GlyphId path as every
        // other selector. If the font has no mapped control glyph, the normal
        // fallback resolves to that font's .notdef glyph.
        let main_glyphs = resolve_glyphs(entry, main_fonts);
        let main_font_index = main_glyphs.first().map(|glyph| glyph.font_index);
        let background_color = entry
            .background_color
            .clone()
            .unwrap_or_else(|| state.current_bg_color.clone());
        let main_text_color = state
            .text_colors
            .get(&config.main_text.id)
            .cloned()
            .or_else(|| entry.text_color.clone())
            .unwrap_or_else(|| config.main_text.color.clone());
        let main_position = state
            .text_positions
            .get(&config.main_text.id)
            .cloned()
            .or_else(|| entry.position.clone())
            .unwrap_or_else(|| config.main_text.position.clone());
        let bottom_text_color = state
            .text_colors
            .get(&config.bottom_text.id)
            .cloned()
            .unwrap_or_else(|| config.bottom_text.color.clone());
        let bottom_position = state
            .text_positions
            .get(&config.bottom_text.id)
            .cloned()
            .unwrap_or_else(|| config.bottom_text.position.clone());
        let job = FrameJob {
            index: *frame_index,
            entry_index: *entry_index,
            background_color,
            main_text_color,
            main_position,
            bottom_text_color,
            bottom_position,
            main_font_index,
            main_glyphs,
        };

        if is_paused {
            state.pause_frames_remaining -= 1;
        } else {
            *frame_in_char += 1;
        }
        *frame_index += 1;
        return Some(job);
    }
}

struct FrameRenderCtx<'a> {
    config: &'a RenderConfig,
    entry: &'a CharEntry,
    frame_index: u64,
    background_color: &'a Color,
    main_text_color: &'a Color,
    main_position: &'a Position,
    bottom_text_color: &'a Color,
    bottom_position: &'a Position,
    main_font_index: Option<usize>,
    main_glyphs: &'a [ResolvedGlyph],
    main_fonts: &'a [Arc<FontLoader>],
    bottom_fonts: &'a [Arc<FontLoader>],
    unicode_manager: &'a UnicodeDataManager,
    width: u32,
    height: u32,
}

fn fill_frame_background(image: &mut RgbaImage, color: &Color) {
    for pixel in image.pixels_mut() {
        pixel[0] = color.r;
        pixel[1] = color.g;
        pixel[2] = color.b;
        pixel[3] = 255;
    }
}

fn render_combining_overlay(
    ctx: &FrameRenderCtx,
    font: &FontLoader,
    codepoint: u32,
    anchor_x: i32,
    anchor_y: i32,
    image: &mut RgbaImage,
) {
    if !ctx.config.overlay_enabled || !ctx.unicode_manager.is_combining_mark(codepoint) {
        return;
    }

    let overlay_char = '\u{25CC}';
    let overlay_color =
        flatten_color_over_background(ctx.main_text_color, ctx.background_color, 0.5);
    let overlay_glyph = font.font().glyph_id(overlay_char);
    let overlay_glyph_scaled =
        overlay_glyph.with_scale_and_position(font.px_scale(), ab_glyph::Point { x: 0.0, y: 0.0 });
    let (x, y) = if let Some(outline) = font.font().outline_glyph(overlay_glyph_scaled) {
        let bounds = outline.px_bounds();
        let center_x = (bounds.min.x + bounds.max.x) / 2.0;
        let center_y = (bounds.min.y + bounds.max.y) / 2.0;
        (
            anchor_x - center_x as i32 + ctx.config.text_x_offset,
            anchor_y - center_y as i32 + ctx.config.text_y_offset,
        )
    } else {
        let width = font.glyph_width(overlay_char) as i32;
        let (ascent, descent) = font.metrics();
        let baseline =
            anchor_y + ((ascent + descent) / 2.0).round() as i32 + ctx.config.text_y_offset;
        (anchor_x - width / 2 + ctx.config.text_x_offset, baseline)
    };

    font.render_to_image(overlay_char, image, x, y, overlay_color);
}

fn render_main_glyphs(
    ctx: &FrameRenderCtx,
    image: &mut RgbaImage,
    origin_x: i32,
    origin_y: i32,
) -> Result<()> {
    let fallback_color =
        flatten_color_over_background(ctx.main_text_color, ctx.background_color, 1.0);
    let advanced_color = (
        ctx.main_text_color.r,
        ctx.main_text_color.g,
        ctx.main_text_color.b,
        ctx.main_text_color.a,
    );
    let mut cursor_x = origin_x;

    for glyph in ctx.main_glyphs {
        let font = ctx
            .main_fonts
            .get(glyph.font_index)
            .or_else(|| ctx.main_fonts.last())
            .context("Main text is enabled without a loaded font")?;
        let glyph_id = font.glyph_id_for_selector(&glyph.selector);
        let handled_advanced = match glyph.selector {
            GlyphSelector::CodePoint(value) => char::from_u32(value)
                .map(|character| {
                    font.render_advanced_to_image(
                        character,
                        image,
                        cursor_x,
                        origin_y,
                        advanced_color,
                    )
                })
                .transpose()?
                .unwrap_or(false),
            GlyphSelector::Name(_) | GlyphSelector::Index(_) => font
                .render_advanced_glyph_to_image(
                    glyph_id,
                    image,
                    cursor_x,
                    origin_y,
                    advanced_color,
                )?,
        };

        if !handled_advanced {
            font.render_glyph_id_to_image(glyph_id, image, cursor_x, origin_y, fallback_color);
        }
        cursor_x += font.glyph_width_id(glyph_id).round() as i32;
    }
    Ok(())
}

fn format_text_template(template: &str, entry: &CharEntry) -> String {
    let selected_text: String = entry
        .glyph_specs()
        .into_iter()
        .filter_map(|spec| match spec.selector {
            GlyphSelector::CodePoint(codepoint) => char::from_u32(codepoint),
            GlyphSelector::Name(_) | GlyphSelector::Index(_) => None,
        })
        .collect();
    let selected_text = if selected_text.is_empty() {
        entry.code_point.as_str()
    } else {
        selected_text.as_str()
    };

    template
        .replace("{char}", selected_text)
        .replace("{code}", entry.code_point.trim())
        .replace("{description}", entry.description.as_str())
}

fn render_bottom_information(ctx: &FrameRenderCtx, image: &mut RgbaImage) -> Result<()> {
    if !ctx.config.bottom_text.enabled {
        return Ok(());
    }

    let bottom_color = ctx.bottom_text_color;
    let blended = flatten_color_over_background(bottom_color, ctx.background_color, 1.0);
    let text = format_text_template(&ctx.config.bottom_text.content, ctx.entry);
    if text.is_empty() {
        return Ok(());
    }

    let font_size = ctx.config.bottom_font.size as i32;
    let base_x = (ctx.bottom_position.x * ctx.width as f64) as i32;
    let base_y = (ctx.bottom_position.y * ctx.height as f64) as i32;
    let max_width = if ctx.config.bottom_text.max_width > 0.0 {
        (ctx.config.bottom_text.max_width * ctx.width as f64) as i32
    } else {
        ctx.width as i32 - base_x
    };
    let lines = if ctx.config.bottom_text.wrap {
        wrap_text(&text, max_width, ctx.bottom_fonts, font_size)
    } else {
        text.lines().map(str::to_owned).collect()
    };
    let line_height = font_size + 5;
    let first_line_y = base_y - (lines.len() as i32 - 1) * line_height;

    for (line_index, line) in lines.iter().enumerate() {
        let line_y = first_line_y + line_index as i32 * line_height;
        let line_width = measure_text_width(line, ctx.bottom_fonts);
        let start_x = match ctx.config.bottom_text.align {
            crate::json_config::TextAlign::Left => base_x,
            crate::json_config::TextAlign::Center => base_x + (max_width - line_width) / 2,
            crate::json_config::TextAlign::Right => base_x + max_width - line_width,
        };

        let mut character_x = start_x;
        for character in line.chars() {
            let font = ctx
                .bottom_fonts
                .iter()
                .find(|font| font.has_char(character))
                .or_else(|| ctx.bottom_fonts.last())
                .context("Bottom text is enabled without a loaded font")?;
            let character_width = font.glyph_width(character) as i32;
            let character_y = line_y - font.metrics().0 as i32;
            let handled_advanced = font.render_advanced_to_image(
                character,
                image,
                character_x,
                character_y,
                (
                    bottom_color.r,
                    bottom_color.g,
                    bottom_color.b,
                    bottom_color.a,
                ),
            )?;
            if !handled_advanced {
                font.render_to_image(character, image, character_x, character_y, blended);
            }
            character_x += character_width;
        }
    }
    Ok(())
}

fn render_frame_parallel(ctx: &FrameRenderCtx) -> Result<Vec<u8>> {
    let codepoint = ctx.entry.code_point_value();
    let character = char::from_u32(codepoint).unwrap_or('\u{FFFD}');
    let mut image = RgbaImage::new(ctx.width, ctx.height);
    fill_frame_background(&mut image, ctx.background_color);

    if ctx.config.main_text.enabled {
        let primary_font = ctx
            .main_font_index
            .and_then(|index| ctx.main_fonts.get(index))
            .or_else(|| ctx.main_fonts.first())
            .context("Main text is enabled without a loaded font")?;
        let anchor_x = (ctx.main_position.x * ctx.width as f64) as i32;
        let anchor_y = (ctx.main_position.y * ctx.height as f64) as i32;
        let glyph_advance: f32 = ctx
            .main_glyphs
            .iter()
            .map(|glyph| {
                let font = ctx
                    .main_fonts
                    .get(glyph.font_index)
                    .or_else(|| ctx.main_fonts.last())
                    .unwrap_or(primary_font);
                font.glyph_width_id(font.glyph_id_for_selector(&glyph.selector))
            })
            .sum();
        let outline_center_x = if glyph_advance.abs() <= f32::EPSILON {
            let glyph = primary_font.font().glyph_id(character);
            let scaled = glyph.with_scale_and_position(
                primary_font.px_scale(),
                ab_glyph::Point { x: 0.0, y: 0.0 },
            );
            primary_font.font().outline_glyph(scaled).map(|outline| {
                let bounds = outline.px_bounds();
                (bounds.min.x + bounds.max.x) / 2.0
            })
        } else {
            None
        };
        let (ascent, descent) = primary_font.metrics();
        let (origin_x, origin_y) = typographic_glyph_origin(
            (
                anchor_x + ctx.config.text_x_offset,
                anchor_y + ctx.config.text_y_offset,
            ),
            glyph_advance,
            outline_center_x,
            ascent,
            descent,
        );

        render_combining_overlay(ctx, primary_font, codepoint, anchor_x, anchor_y, &mut image);
        render_main_glyphs(ctx, &mut image, origin_x, origin_y)?;
    }
    render_bottom_information(ctx, &mut image)?;
    Ok(image.into_raw())
}

#[cfg(test)]
mod tests {
    use super::{
        add_rawvideo_input_args, add_stream_mapping_args, concat_file_entry, format_text_template,
        initial_render_state, next_frame_job, render_frame_png, typographic_glyph_origin,
    };
    use crate::json_config::{CharEntry, Color, Event, EventType, Position, RenderConfig};
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn ffmpeg_stream_maps_follow_all_inputs() {
        let mut command = Command::new("ffmpeg");
        add_rawvideo_input_args(&mut command, 1920, 1080, 30.0);
        command.arg("-i").arg("music.m4a");
        add_stream_mapping_args(&mut command, true);

        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let music_input = args
            .windows(2)
            .position(|pair| pair[0] == "-i" && pair[1] == "music.m4a")
            .expect("music input should be present");
        let first_map = args
            .iter()
            .position(|arg| arg == "-map")
            .expect("stream mapping should be present");

        assert!(first_map > music_input + 1);
        let mappings: Vec<&str> = args[first_map..first_map + 4]
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(mappings, ["-map", "0:v:0", "-map", "1:a:0?"]);
    }

    #[test]
    fn typographic_center_uses_advance_and_baseline() {
        assert_eq!(
            typographic_glyph_origin((960, 540), 201.0, None, 512.0, -128.0),
            (859, 732),
        );
    }

    #[test]
    fn zero_advance_mark_is_visually_centered_horizontally() {
        assert_eq!(
            typographic_glyph_origin((960, 540), 0.0, Some(-35.0), 512.0, -128.0),
            (995, 732),
        );
    }

    #[test]
    fn concat_list_uses_ffmpeg_friendly_windows_path() {
        assert_eq!(
            concat_file_entry(Path::new(r"C:\render parts\segment-0001.mp4")),
            "file 'C:/render parts/segment-0001.mp4'\n",
        );
    }

    #[test]
    fn pause_event_adds_frames_without_advancing_the_character() {
        let mut config = RenderConfig {
            fps: 1.0,
            characters: vec![CharEntry {
                code_point: "U+0041".to_string(),
                description: String::new(),
                background_color: None,
                text_color: None,
                position: None,
                duration_frames: Some(3),
            }],
            events: vec![Event {
                frame: 1,
                event_type: EventType::Pause { duration: 2.0 },
            }],
            ..Default::default()
        };

        config.main_text.enabled = false;
        config.bottom_text.enabled = false;

        let mut state = initial_render_state(&config);
        let mut color_index = 0;
        let mut entry_index = 0;
        let mut frame_in_char = 0;
        let mut frame_index = 0;
        let mut jobs = Vec::new();

        while let Some(job) = next_frame_job(
            &config,
            &[],
            &mut state,
            &mut color_index,
            &mut entry_index,
            &mut frame_in_char,
            &mut frame_index,
        ) {
            jobs.push(job);
        }

        assert_eq!(jobs.len() as u64, config.total_frames());
        assert_eq!(jobs.len(), 5);
    }

    #[test]
    fn character_overrides_and_bottom_events_are_resolved_per_frame() {
        let mut config = RenderConfig::default();
        config.main_text.enabled = false;
        config.bottom_text.enabled = false;
        config.characters = vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: Some(Color {
                r: 1,
                g: 2,
                b: 3,
                a: 4,
            }),
            position: Some(Position { x: 0.2, y: 0.3 }),
            duration_frames: Some(1),
        }];
        config.events = vec![
            Event {
                frame: 0,
                event_type: EventType::SetTextColor {
                    element_id: config.bottom_text.id.clone(),
                    color: Color {
                        r: 5,
                        g: 6,
                        b: 7,
                        a: 8,
                    },
                },
            },
            Event {
                frame: 0,
                event_type: EventType::SetTextPosition {
                    element_id: config.bottom_text.id.clone(),
                    position: Position { x: 0.7, y: 0.8 },
                },
            },
        ];

        let mut state = initial_render_state(&config);
        let mut color_index = 0;
        let mut entry_index = 0;
        let mut frame_in_char = 0;
        let mut frame_index = 0;
        let job = next_frame_job(
            &config,
            &[],
            &mut state,
            &mut color_index,
            &mut entry_index,
            &mut frame_in_char,
            &mut frame_index,
        )
        .unwrap();

        assert_eq!(job.main_text_color.r, 1);
        assert_eq!(job.main_position.x, 0.2);
        assert_eq!(job.bottom_text_color.r, 5);
        assert_eq!(job.bottom_position.x, 0.7);
    }

    #[test]
    fn text_template_resolves_character_code_and_description() {
        let entry = CharEntry {
            code_point: "U+0041".to_string(),
            description: "LATIN CAPITAL LETTER A".to_string(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: None,
        };

        assert_eq!(
            format_text_template("{char} | {code} | {description}", &entry),
            "A | U+0041 | LATIN CAPITAL LETTER A"
        );
    }

    #[test]
    fn disabled_text_layers_do_not_require_fonts_for_preview() {
        let mut config = RenderConfig {
            resolution: (8, 8),
            characters: vec![CharEntry {
                code_point: "U+0041".to_string(),
                description: String::new(),
                background_color: None,
                text_color: None,
                position: None,
                duration_frames: Some(1),
            }],
            ..Default::default()
        };

        config.main_text.enabled = false;
        config.main_text.fonts.clear();
        config.bottom_text.enabled = false;
        config.bottom_text.fonts.clear();

        assert!(!render_frame_png(&config, 0, 8).unwrap().is_empty());
    }
}
