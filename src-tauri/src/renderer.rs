use crate::config_validation::validate_render_config;
use crate::content_template::{ContentTemplateResolver, TemplateContext};
use crate::font_loader::FontLoader;
use crate::image_renderer;
use crate::json_config::{
    duration_to_frames, AnimationCurve, CharEntry, EventType, GlyphSpec, RenderConfig,
};
use crate::primitive_renderer;
use crate::scene::{
    AnimatableProperty, Color, ComponentPropertyValue, FontConfig, GlyphSelector, Position,
    Scale2D, SceneComponent,
};
use crate::scene_transform::{composite_affine, transform_matrix, SceneRenderContext};
use crate::typography_renderer::{
    CenteredUnicodeRequest, ExplicitGlyphLayoutMetrics, ExplicitGlyphRequest,
    PreparedUnicodeLayout, TextPlacement, TypographyRenderer,
};
use crate::unicode_data::UnicodeDataManager;
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
const MAX_PREVIEW_IMAGE_CACHE_ENTRIES: usize = 16;

static PREVIEW_FONT_CACHE: Lazy<Mutex<HashMap<String, Arc<FontLoader>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PREVIEW_UNICODE_CACHE: Lazy<Mutex<HashMap<String, Arc<UnicodeDataManager>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PREVIEW_IMAGE_CACHE: Lazy<Mutex<HashMap<String, Arc<RgbaImage>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn load_preview_image(path: &Path, enabled: bool) -> Result<Option<Arc<RgbaImage>>> {
    if !enabled {
        return Ok(None);
    }
    let resolved_path = crate::json_config::resolve_asset_reference(path);
    let key = resolved_path.to_string_lossy().to_string();
    if let Some(image) = PREVIEW_IMAGE_CACHE
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview image cache lock poisoned"))?
        .get(&key)
        .cloned()
    {
        return Ok(Some(image));
    }
    let image = image_renderer::load_image(&resolved_path)?;
    let mut cache = PREVIEW_IMAGE_CACHE
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview image cache lock poisoned"))?;
    if cache.len() >= MAX_PREVIEW_IMAGE_CACHE_ENTRIES && !cache.contains_key(&key) {
        cache.clear();
    }
    cache.insert(key, image.clone());
    Ok(Some(image))
}

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
    component_properties: HashMap<String, HashMap<AnimatableProperty, ComponentPropertyValue>>,
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
    component_states: HashMap<String, ComponentFrameState>,
}

#[derive(Clone)]
struct ComponentFrameState {
    color: Option<Color>,
    position: Position,
    scale: Option<Scale2D>,
    rotation: Option<f64>,
    opacity: Option<f64>,
    progress: Option<f64>,
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
    ComponentProperty {
        element_id: String,
        property: AnimatableProperty,
        start_value: ComponentPropertyValue,
        end_value: ComponentPropertyValue,
    },
    BackgroundColorTransition {
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

fn set_component_property(
    state: &mut RenderState,
    element_id: &str,
    property: AnimatableProperty,
    value: ComponentPropertyValue,
) {
    state
        .component_properties
        .entry(element_id.to_string())
        .or_default()
        .insert(property, value);
}

fn component_property<'a>(
    state: &'a RenderState,
    element_id: &str,
    property: AnimatableProperty,
) -> Option<&'a ComponentPropertyValue> {
    state
        .component_properties
        .get(element_id)
        .and_then(|properties| properties.get(&property))
}

fn component_number_property(
    state: &RenderState,
    element_id: &str,
    property: AnimatableProperty,
) -> Option<f64> {
    match component_property(state, element_id, property) {
        Some(ComponentPropertyValue::Number(value)) => Some(*value),
        _ => None,
    }
}

fn component_color_property(state: &RenderState, element_id: &str) -> Option<Color> {
    match component_property(state, element_id, AnimatableProperty::Color) {
        Some(ComponentPropertyValue::Color(color)) => Some(color.clone()),
        _ => None,
    }
}

fn interpolate_property_value(
    start: &ComponentPropertyValue,
    end: &ComponentPropertyValue,
    t: f64,
) -> ComponentPropertyValue {
    match (start, end) {
        (ComponentPropertyValue::Number(start), ComponentPropertyValue::Number(end)) => {
            ComponentPropertyValue::Number(start * (1.0 - t) + end * t)
        }
        (ComponentPropertyValue::Color(start), ComponentPropertyValue::Color(end)) => {
            ComponentPropertyValue::Color(lerp_color(start.clone(), end.clone(), t))
        }
        _ => end.clone(),
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

fn push_animation(
    state: &mut RenderState,
    start_frame: u64,
    duration_frames: u64,
    curve: AnimationCurve,
    animation_type: AnimationType,
) {
    state.animating_events.push(ActiveAnimation {
        start_frame,
        duration_frames,
        curve,
        animation_type,
    });
}

fn update_animations(state: &mut RenderState, frame: u64) {
    let animations = std::mem::take(&mut state.animating_events);

    for anim in animations {
        let elapsed = frame.saturating_sub(anim.start_frame);
        if elapsed >= anim.duration_frames {
            match &anim.animation_type {
                AnimationType::ComponentProperty {
                    element_id,
                    property,
                    end_value,
                    ..
                } => {
                    set_component_property(state, element_id, *property, end_value.clone());
                }
                AnimationType::BackgroundColorTransition { end_color, .. } => {
                    state.current_bg_color = end_color.clone();
                }
            }
            continue;
        }

        let t = anim
            .curve
            .apply(elapsed as f64 / anim.duration_frames as f64);
        match &anim.animation_type {
            AnimationType::ComponentProperty {
                element_id,
                property,
                start_value,
                end_value,
            } => {
                let value = interpolate_property_value(start_value, end_value, t);
                set_component_property(state, element_id, *property, value);
            }
            AnimationType::BackgroundColorTransition {
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
        if event.frame != frame {
            continue;
        }
        match &event.event_type {
            EventType::SetBackgroundColor { color } => {
                state.current_bg_color = color.clone();
            }
            EventType::SetComponentColor { element_id, color } => {
                set_component_property(
                    state,
                    element_id,
                    AnimatableProperty::Color,
                    ComponentPropertyValue::Color(color.clone()),
                );
            }
            EventType::SetComponentPosition {
                element_id,
                position,
            } => {
                set_component_property(
                    state,
                    element_id,
                    AnimatableProperty::PositionX,
                    ComponentPropertyValue::Number(position.x),
                );
                set_component_property(
                    state,
                    element_id,
                    AnimatableProperty::PositionY,
                    ComponentPropertyValue::Number(position.y),
                );
            }
            EventType::MoveComponentPosition {
                element_id,
                start_position,
                end_position,
                duration,
                curve,
            } => {
                let duration_frames = duration_to_frames(*duration, config.fps);
                push_animation(
                    state,
                    frame,
                    duration_frames,
                    curve.clone(),
                    AnimationType::ComponentProperty {
                        element_id: element_id.clone(),
                        property: AnimatableProperty::PositionX,
                        start_value: ComponentPropertyValue::Number(start_position.x),
                        end_value: ComponentPropertyValue::Number(end_position.x),
                    },
                );
                push_animation(
                    state,
                    frame,
                    duration_frames,
                    curve.clone(),
                    AnimationType::ComponentProperty {
                        element_id: element_id.clone(),
                        property: AnimatableProperty::PositionY,
                        start_value: ComponentPropertyValue::Number(start_position.y),
                        end_value: ComponentPropertyValue::Number(end_position.y),
                    },
                );
            }
            EventType::SetComponentProperty {
                element_id,
                property,
                value,
            } => {
                set_component_property(state, element_id, *property, value.clone());
            }
            EventType::AnimateComponentProperty {
                element_id,
                property,
                start_value,
                end_value,
                duration,
                curve,
            } => {
                push_animation(
                    state,
                    frame,
                    duration_to_frames(*duration, config.fps),
                    curve.clone(),
                    AnimationType::ComponentProperty {
                        element_id: element_id.clone(),
                        property: *property,
                        start_value: start_value.clone(),
                        end_value: end_value.clone(),
                    },
                );
            }
            EventType::ColorTransition {
                start_color,
                end_color,
                duration,
                curve,
            } => {
                let duration_frames = duration_to_frames(*duration, config.fps);
                push_animation(
                    state,
                    frame,
                    duration_frames,
                    curve.clone(),
                    AnimationType::BackgroundColorTransition {
                        start_color: start_color.clone(),
                        end_color: end_color.clone(),
                    },
                );
            }
            EventType::Pause { duration } => {
                state.pause_frames_remaining = state
                    .pause_frames_remaining
                    .saturating_add(duration_to_frames(*duration, config.fps));
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

pub fn render_video(config: &RenderConfig) -> Result<()> {
    render_video_with_progress(config, None)
}

/// Render a single configured character entry as a PNG. This deliberately
/// uses the same glyph parsing, fallback selection, and drawing path as video
/// rendering so CLI and GUI previews match the final result.
fn scale_component_font_sizes(components: &mut [SceneComponent], scale: f32) {
    for component in components {
        if let Some(font) = component.font_mut() {
            font.size *= scale;
        }
        if let SceneComponent::Group(group) = component {
            scale_component_font_sizes(&mut group.children, scale);
        }
    }
}

fn collect_preview_resources(
    components: &[SceneComponent],
    parent_enabled: bool,
    fonts_out: &mut HashMap<String, Vec<Arc<FontLoader>>>,
    images_out: &mut HashMap<String, Arc<RgbaImage>>,
) -> Result<()> {
    for component in components {
        let enabled = parent_enabled && component.enabled();
        let fonts = match (component.fonts(), component.font()) {
            (Some(paths), Some(font)) => load_preview_fonts(paths, font, enabled)?,
            _ => Vec::new(),
        };
        fonts_out.insert(component.id().to_string(), fonts);
        if let SceneComponent::Image(image) = component {
            if let Some(asset) = load_preview_image(&image.source, enabled)? {
                images_out.insert(component.id().to_string(), asset);
            }
        }
        if let SceneComponent::Group(group) = component {
            collect_preview_resources(&group.children, enabled, fonts_out, images_out)?;
        }
    }
    Ok(())
}

fn collect_render_resources(
    components: &[SceneComponent],
    parent_enabled: bool,
    fonts_out: &mut HashMap<String, Vec<Arc<FontLoader>>>,
    images_out: &mut HashMap<String, Arc<RgbaImage>>,
) -> Result<()> {
    for component in components {
        let enabled = parent_enabled && component.enabled();
        let fonts = match (component.fonts(), component.font()) {
            (Some(paths), Some(font)) => load_configured_fonts(
                paths,
                font,
                &format!("component {}", component.id()),
                enabled,
            )?,
            _ => Vec::new(),
        };
        fonts_out.insert(component.id().to_string(), fonts);
        if let SceneComponent::Image(image) = component {
            if enabled {
                let resolved = crate::json_config::resolve_asset_reference(&image.source);
                let asset = image_renderer::load_image(&resolved).with_context(|| {
                    format!("Failed to load component {} image", component.id())
                })?;
                images_out.insert(component.id().to_string(), asset);
            }
        }
        if let SceneComponent::Group(group) = component {
            collect_render_resources(&group.children, enabled, fonts_out, images_out)?;
        }
    }
    Ok(())
}

pub fn render_frame_png(
    config: &RenderConfig,
    entry_index: usize,
    max_dimension: u32,
) -> Result<Vec<u8>> {
    let mut normalized_config = config.clone();
    normalized_config.normalize_scene();
    let config = &normalized_config;
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
        scale_component_font_sizes(&mut preview_config.components, scale);
        preview_config.text_x_offset = (preview_config.text_x_offset as f32 * scale).round() as i32;
        preview_config.text_y_offset = (preview_config.text_y_offset as f32 * scale).round() as i32;
    }

    let mut component_fonts = HashMap::new();
    let mut component_images = HashMap::new();
    collect_preview_resources(
        &preview_config.components,
        true,
        &mut component_fonts,
        &mut component_images,
    )?;

    let unicode_manager = load_preview_unicode_manager();
    let template_resolver = ContentTemplateResolver::new(preview_config.content_templates.clone());
    let background_color = entry
        .background_color
        .clone()
        .unwrap_or_else(|| preview_config.background_color.clone());
    let state = RenderState {
        current_bg_color: preview_config.background_color.clone(),
        ..RenderState::default()
    };
    let component_states = resolve_component_frame_states(&preview_config, &entry, &state);
    let (width, height) = preview_config.resolution;
    let frame_ctx = FrameRenderCtx {
        config: &preview_config,
        entry: &entry,
        background_color: &background_color,
        component_states: &component_states,
        component_fonts: &component_fonts,
        component_images: &component_images,
        template_resolver: &template_resolver,
        unicode_manager: &unicode_manager,
        width,
        height,
    };
    let mut typography = TypographyRenderer::new(&component_fonts)?;
    let raw = render_frame_parallel(&frame_ctx, &mut typography)?;
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
    component_fonts: HashMap<String, Vec<Arc<FontLoader>>>,
    component_images: HashMap<String, Arc<RgbaImage>>,
    template_resolver: Arc<ContentTemplateResolver>,
    ffmpeg_path: PathBuf,
    encoder: String,
    music_path: Option<PathBuf>,
    progress: Arc<dyn ProgressReporter>,
}

#[derive(Clone, Copy)]
struct FrameRenderer<'a> {
    config: &'a RenderConfig,
    component_fonts: &'a HashMap<String, Vec<Arc<FontLoader>>>,
    component_images: &'a HashMap<String, Arc<RgbaImage>>,
    template_resolver: &'a ContentTemplateResolver,
    unicode_manager: &'a UnicodeDataManager,
    width: u32,
    height: u32,
}

impl<'a> FrameRenderer<'a> {
    fn new(config: &'a RenderConfig, prepared: &'a PreparedRender) -> Self {
        Self {
            config,
            component_fonts: &prepared.component_fonts,
            component_images: &prepared.component_images,
            template_resolver: prepared.template_resolver.as_ref(),
            unicode_manager: prepared.unicode_manager.as_ref(),
            width: prepared.width,
            height: prepared.height,
        }
    }

    fn render_job(self, job: &FrameJob, typography: &mut TypographyRenderer) -> Result<FrameData> {
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
            background_color: &job.background_color,
            component_states: &job.component_states,
            component_fonts: self.component_fonts,
            component_images: self.component_images,
            template_resolver: self.template_resolver,
            unicode_manager: self.unicode_manager,
            width: self.width,
            height: self.height,
        };

        render_frame_parallel(&frame_ctx, typography).map(|data| FrameData {
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

    let mut component_fonts = HashMap::new();
    let mut component_images = HashMap::new();
    collect_render_resources(
        &config.components,
        true,
        &mut component_fonts,
        &mut component_images,
    )?;

    let template_resolver = Arc::new(ContentTemplateResolver::new(
        config.content_templates.clone(),
    ));

    let ffmpeg_path = crate::ffmpeg::resolve_ffmpeg_path(Some(config.ffmpeg.path.as_path()));

    let encoder = if config.ffmpeg.encoder == "auto" {
        println!("Detecting FFmpeg encoder...");
        detect_encoder(&ffmpeg_path, false)
            .context("No working H.264 encoder was found in FFmpeg")?
    } else {
        config.ffmpeg.encoder.clone()
    };
    println!("Encoder: {}", encoder);

    let music_path = config
        .music_path
        .as_ref()
        .map(|music| {
            let resolved = crate::json_config::resolve_asset_reference(music);
            if !resolved.is_file() {
                anyhow::bail!("Music file not found: {}", resolved.display());
            }
            Ok(resolved)
        })
        .transpose()?;

    let progress: Arc<dyn ProgressReporter> = match progress_callback {
        Some(callback) => Arc::new(CallbackProgressReporter::new(total_frames, callback)),
        None => Arc::new(IndicatifProgressReporter::new(total_frames)),
    };

    Ok(PreparedRender {
        width,
        height,
        total_frames,
        unicode_manager,
        component_fonts,
        component_images,
        template_resolver,
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
                let typography =
                    TypographyRenderer::new(frame_renderer.component_fonts).map_err(|error| {
                        format!("Failed to initialize typography renderer: {error:#}")
                    });
                match typography {
                    Ok(mut typography) => {
                        while let Ok(job) = jobs.recv() {
                            let rendered = frame_renderer
                                .render_job(&job, &mut typography)
                                .map_err(|error| {
                                    format!("Failed to render frame {}: {error:#}", job.index)
                                });
                            if results.send(rendered).is_err() {
                                break;
                            }
                        }
                    }
                    Err(message) => {
                        while let Ok(job) = jobs.recv() {
                            if results
                                .send(Err(format!(
                                    "Failed to render frame {}: {message}",
                                    job.index
                                )))
                                .is_err()
                            {
                                break;
                            }
                        }
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
    let mut normalized_config = config.clone();
    normalized_config.normalize_scene();
    let config = &normalized_config;
    let prepared = prepare_render(config, progress_callback)?;

    if config.ffmpeg.encoding_processes.max(1) > 1 && prepared.total_frames > 1 {
        render_video_multi_process(config, &prepared)
    } else {
        render_video_single_process(config, &prepared)
    }
}

/// Precompute every mutable decision before starting parallel encoders.
fn precompute_frame_schedule(config: &RenderConfig) -> Vec<FrameJob> {
    let mut frames = Vec::new();
    let mut state = initial_render_state(config);
    let mut color_index = 0usize;
    let mut entry_index = 0usize;
    let mut frame_in_char = 0u64;
    let mut frame_index = 0u64;

    while let Some(job) = next_frame_job(
        config,
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
    let frames = precompute_frame_schedule(config);
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

        // A shared renderer pool feeds completed frames directly to each encoder queue.
        let mut render_handles = Vec::new();
        for _ in 0..render_worker_count {
            let tasks = task_receiver.clone();
            let outputs = Arc::clone(&rendered_senders);
            let permits = Arc::clone(&permit_receivers);
            render_handles.push(scope.spawn(move || -> Result<()> {
                let mut typography = TypographyRenderer::new(frame_renderer.component_fonts)?;
                while let Ok(task) = tasks.recv() {
                    permits[task.segment_index].recv().map_err(|_| {
                        anyhow::anyhow!(
                            "FFmpeg encoder {} frame-permit channel closed unexpectedly",
                            task.segment_index
                        )
                    })?;
                    let job = &frames_ref[task.frame_offset];
                    let rendered =
                        frame_renderer
                            .render_job(job, &mut typography)
                            .map_err(|error| {
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

fn resolve_component_frame_states(
    config: &RenderConfig,
    entry: &CharEntry,
    state: &RenderState,
) -> HashMap<String, ComponentFrameState> {
    let primary_glyph_id = config
        .primary_glyph_component()
        .map(|component| component.id.as_str());
    let mut states = HashMap::new();
    resolve_component_frame_states_recursive(
        &config.components,
        primary_glyph_id,
        entry,
        state,
        &mut states,
    );
    states
}

fn resolve_component_frame_states_recursive(
    components: &[SceneComponent],
    primary_glyph_id: Option<&str>,
    entry: &CharEntry,
    state: &RenderState,
    out: &mut HashMap<String, ComponentFrameState>,
) {
    for component in components {
        let id = component.id();
        let is_primary_glyph = primary_glyph_id == Some(id);
        let color = component.color().map(|base_color| {
            component_color_property(state, id)
                .or_else(|| {
                    if is_primary_glyph {
                        entry.text_color.clone()
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| base_color.clone())
        });

        let base_position = component.position();
        let entry_position = is_primary_glyph
            .then_some(entry.position.as_ref())
            .flatten();
        let position = Position {
            x: component_number_property(state, id, AnimatableProperty::PositionX)
                .or_else(|| entry_position.map(|position| position.x))
                .unwrap_or(base_position.x),
            y: component_number_property(state, id, AnimatableProperty::PositionY)
                .or_else(|| entry_position.map(|position| position.y))
                .unwrap_or(base_position.y),
        };

        let (scale, rotation, opacity, progress) = match component {
            SceneComponent::Group(group) => (
                Some(Scale2D {
                    x: component_number_property(state, id, AnimatableProperty::ScaleX)
                        .unwrap_or(group.transform.scale.x),
                    y: component_number_property(state, id, AnimatableProperty::ScaleY)
                        .unwrap_or(group.transform.scale.y),
                }),
                Some(
                    component_number_property(state, id, AnimatableProperty::Rotation)
                        .unwrap_or(group.transform.rotation),
                ),
                Some(
                    component_number_property(state, id, AnimatableProperty::Opacity)
                        .unwrap_or(f64::from(group.opacity)),
                ),
                None,
            ),
            SceneComponent::Image(image) => (
                None,
                None,
                Some(
                    component_number_property(state, id, AnimatableProperty::Opacity)
                        .unwrap_or(f64::from(image.opacity)),
                ),
                None,
            ),
            SceneComponent::ProgressBar(progress) => (
                None,
                None,
                None,
                Some(
                    component_number_property(state, id, AnimatableProperty::Progress)
                        .unwrap_or(progress.progress),
                ),
            ),
            SceneComponent::Glyph(_) | SceneComponent::Text(_) => (None, None, None, None),
        };

        out.insert(
            id.to_string(),
            ComponentFrameState {
                color,
                position,
                scale,
                rotation,
                opacity,
                progress,
            },
        );
        if let SceneComponent::Group(group) = component {
            resolve_component_frame_states_recursive(
                &group.children,
                primary_glyph_id,
                entry,
                state,
                out,
            );
        }
    }
}

/// Resolve one explicit glyph selector against a component's font fallback list.
fn resolve_glyph_spec(spec: GlyphSpec, fonts: &[Arc<FontLoader>]) -> Option<ResolvedGlyph> {
    if fonts.is_empty() {
        return None;
    }
    if let Some(font_index) = spec.font_index {
        let font_index = font_index.min(fonts.len() - 1);
        let selector = if fonts[font_index].has_selector(&spec.selector) {
            spec.selector
        } else {
            GlyphSelector::Name(".notdef".to_string())
        };
        return Some(ResolvedGlyph {
            selector,
            font_index,
        });
    }
    match fonts
        .iter()
        .position(|font| font.has_selector(&spec.selector))
    {
        Some(font_index) => Some(ResolvedGlyph {
            selector: spec.selector,
            font_index,
        }),
        None => Some(ResolvedGlyph {
            selector: GlyphSelector::Name(".notdef".to_string()),
            font_index: fonts.len() - 1,
        }),
    }
}

/// Resolve a glyph-selector expression against one component's font fallback list.
fn resolve_glyphs(expression: &str, fonts: &[Arc<FontLoader>]) -> Vec<ResolvedGlyph> {
    CharEntry::glyph_specs_from(expression)
        .into_iter()
        .filter_map(|spec| resolve_glyph_spec(spec, fonts))
        .collect()
}

fn next_frame_job(
    config: &RenderConfig,
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

        let background_color = entry
            .background_color
            .clone()
            .unwrap_or_else(|| state.current_bg_color.clone());
        let component_states = resolve_component_frame_states(config, entry, state);
        let job = FrameJob {
            index: *frame_index,
            entry_index: *entry_index,
            background_color,
            component_states,
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
    background_color: &'a Color,
    component_states: &'a HashMap<String, ComponentFrameState>,
    component_fonts: &'a HashMap<String, Vec<Arc<FontLoader>>>,
    component_images: &'a HashMap<String, Arc<RgbaImage>>,
    template_resolver: &'a ContentTemplateResolver,
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

fn frame_state_color(state: &ComponentFrameState) -> Result<&Color> {
    state
        .color
        .as_ref()
        .context("Typography component is missing its frame color")
}

struct CombiningOverlayRequest<'a, 'ctx> {
    ctx: &'a FrameRenderCtx<'ctx>,
    state: &'a ComponentFrameState,
    font: &'a FontLoader,
    font_config: &'a FontConfig,
    codepoint: u32,
    anchor: (i32, i32),
}

fn render_combining_overlay(
    request: CombiningOverlayRequest<'_, '_>,
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    if !request.ctx.config.overlay_enabled
        || !request
            .ctx
            .unicode_manager
            .is_combining_mark(request.codepoint)
    {
        return Ok(());
    }

    let glyph_id = request
        .font
        .glyph_id_for_selector(&GlyphSelector::CodePoint(0x25CC));
    let metrics =
        typography.explicit_glyph_layout_metrics(request.font, glyph_id.0, request.font_config)?;
    let (origin_x, origin_y) = typographic_glyph_origin(
        request.anchor,
        metrics.advance,
        metrics.outline_center_x,
        metrics.ascent,
        metrics.descent,
    );
    let base_color = frame_state_color(request.state)?;
    let overlay_color = Color {
        r: base_color.r,
        g: base_color.g,
        b: base_color.b,
        a: u16::from(base_color.a).div_ceil(2) as u8,
    };
    let handled = typography.render_explicit_glyph(
        ExplicitGlyphRequest {
            font: request.font,
            glyph_id: glyph_id.0,
            config: request.font_config,
            position: (origin_x as f32, origin_y as f32),
            color: overlay_color.clone(),
        },
        image,
    )?;
    if !handled {
        let fallback_color =
            flatten_color_over_background(&overlay_color, request.ctx.background_color, 1.0);
        request
            .font
            .render_glyph_id_to_image(glyph_id, image, origin_x, origin_y, fallback_color);
    }
    Ok(())
}

struct PreparedExplicitGlyph<'a> {
    font: &'a FontLoader,
    glyph_id: ab_glyph::GlyphId,
    metrics: ExplicitGlyphLayoutMetrics,
}

fn prepare_explicit_glyphs<'a>(
    glyphs: &[ResolvedGlyph],
    fonts: &'a [Arc<FontLoader>],
    font_config: &FontConfig,
    typography: &TypographyRenderer,
) -> Result<Vec<PreparedExplicitGlyph<'a>>> {
    glyphs
        .iter()
        .map(|glyph| {
            let font = fonts
                .get(glyph.font_index)
                .or_else(|| fonts.last())
                .context("Glyph component is enabled without a loaded font")?;
            let glyph_id = font.glyph_id_for_selector(&glyph.selector);
            let metrics =
                typography.explicit_glyph_layout_metrics(font, glyph_id.0, font_config)?;
            Ok(PreparedExplicitGlyph {
                font,
                glyph_id,
                metrics,
            })
        })
        .collect()
}

struct PreparedGlyphRenderRequest<'a> {
    glyphs: &'a [PreparedExplicitGlyph<'a>],
    font_config: &'a FontConfig,
    color: &'a Color,
    background_color: &'a Color,
    origin: (i32, i32),
}

fn render_resolved_glyphs(
    request: PreparedGlyphRenderRequest<'_>,
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    let fallback_color =
        flatten_color_over_background(request.color, request.background_color, 1.0);
    let mut cursor_x = request.origin.0 as f32;

    for glyph in request.glyphs {
        let handled = typography.render_explicit_glyph(
            ExplicitGlyphRequest {
                font: glyph.font,
                glyph_id: glyph.glyph_id.0,
                config: request.font_config,
                position: (cursor_x, request.origin.1 as f32),
                color: request.color.clone(),
            },
            image,
        )?;
        if !handled {
            glyph.font.render_glyph_id_to_image(
                glyph.glyph_id,
                image,
                cursor_x.round() as i32,
                request.origin.1,
                fallback_color,
            );
        }
        cursor_x += glyph.metrics.advance;
    }
    Ok(())
}

fn resolve_component_content(ctx: &FrameRenderCtx<'_>, content: &str) -> Result<String> {
    let character = ctx.entry.selected_unicode_text();
    let template_context = TemplateContext {
        character: &character,
        glyph_expression: ctx.entry.code_point.trim(),
        description: &ctx.entry.description,
    };
    ctx.template_resolver.resolve(content, &template_context)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GlyphSequenceSegment {
    Unicode {
        text: String,
        font_index: Option<usize>,
    },
    Exact(GlyphSpec),
}

fn flush_unicode_segment(
    segments: &mut Vec<GlyphSequenceSegment>,
    unicode: &mut String,
    font_index: &mut Option<usize>,
) {
    if !unicode.is_empty() {
        segments.push(GlyphSequenceSegment::Unicode {
            text: std::mem::take(unicode),
            font_index: *font_index,
        });
    }
    *font_index = None;
}

fn glyph_sequence_segments(specs: &[GlyphSpec]) -> Vec<GlyphSequenceSegment> {
    let mut segments = Vec::new();
    let mut unicode = String::new();
    let mut unicode_font_index: Option<usize> = None;

    for spec in specs {
        match &spec.selector {
            GlyphSelector::CodePoint(value) => {
                let Some(character) = char::from_u32(*value) else {
                    flush_unicode_segment(&mut segments, &mut unicode, &mut unicode_font_index);
                    segments.push(GlyphSequenceSegment::Exact(spec.clone()));
                    continue;
                };
                if !unicode.is_empty() && unicode_font_index != spec.font_index {
                    flush_unicode_segment(&mut segments, &mut unicode, &mut unicode_font_index);
                }
                if unicode.is_empty() {
                    unicode_font_index = spec.font_index;
                }
                unicode.push(character);
            }
            GlyphSelector::Name(_) | GlyphSelector::Index(_) => {
                flush_unicode_segment(&mut segments, &mut unicode, &mut unicode_font_index);
                segments.push(GlyphSequenceSegment::Exact(spec.clone()));
            }
        }
    }
    flush_unicode_segment(&mut segments, &mut unicode, &mut unicode_font_index);
    segments
}

fn unicode_shaping_sequence(specs: &[GlyphSpec]) -> Option<String> {
    if specs.is_empty() || specs.iter().any(|spec| spec.font_index.is_some()) {
        return None;
    }
    specs
        .iter()
        .map(|spec| match spec.selector {
            GlyphSelector::CodePoint(value) => char::from_u32(value),
            GlyphSelector::Name(_) | GlyphSelector::Index(_) => None,
        })
        .collect()
}

enum PreparedMixedSegment<'a> {
    Unicode(Box<PreparedUnicodeLayout>),
    Exact(PreparedExplicitGlyph<'a>),
}

impl PreparedMixedSegment<'_> {
    fn width(&self) -> f32 {
        match self {
            Self::Unicode(layout) => layout.width(),
            Self::Exact(glyph) => glyph.metrics.advance,
        }
    }
}

fn render_mixed_glyph_segments(
    ctx: &FrameRenderCtx<'_>,
    component: &crate::scene::GlyphComponent,
    state: &ComponentFrameState,
    fonts: &[Arc<FontLoader>],
    specs: &[GlyphSpec],
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    let default_font_size = fonts
        .first()
        .map(|font| font.render_size())
        .unwrap_or(component.font.size);
    let center = (
        (state.position.x * ctx.width as f64) as f32 + ctx.config.text_x_offset as f32,
        (state.position.y * ctx.height as f64) as f32 + ctx.config.text_y_offset as f32,
    );
    let segments = glyph_sequence_segments(specs);
    let mut prepared = Vec::with_capacity(segments.len());

    for segment in segments {
        match segment {
            GlyphSequenceSegment::Unicode { text, font_index } => {
                let font_size = font_index
                    .and_then(|index| fonts.get(index).or(fonts.last()))
                    .map(|font| font.render_size())
                    .unwrap_or(default_font_size);
                let request = CenteredUnicodeRequest {
                    component_id: &component.id,
                    config: &component.font,
                    font_size,
                    text: &text,
                    font_index,
                    color: frame_state_color(state)?.clone(),
                    center,
                };
                if let Some(layout) = typography.prepare_centered_unicode_sequence(&request)? {
                    prepared.push(PreparedMixedSegment::Unicode(Box::new(layout)));
                }
            }
            GlyphSequenceSegment::Exact(spec) => {
                let Some(resolved) = resolve_glyph_spec(spec, fonts) else {
                    continue;
                };
                let mut exact = prepare_explicit_glyphs(
                    std::slice::from_ref(&resolved),
                    fonts,
                    &component.font,
                    typography,
                )?;
                if let Some(glyph) = exact.pop() {
                    prepared.push(PreparedMixedSegment::Exact(glyph));
                }
            }
        }
    }

    let total_width: f32 = prepared.iter().map(PreparedMixedSegment::width).sum();
    let mut cursor_x = center.0 - total_width / 2.0;
    for segment in &prepared {
        match segment {
            PreparedMixedSegment::Unicode(layout) => {
                let width = layout.width();
                typography.render_prepared_unicode_sequence(
                    layout.as_ref(),
                    (cursor_x + width / 2.0, center.1),
                    image,
                )?;
                cursor_x += width;
            }
            PreparedMixedSegment::Exact(glyph) => {
                let anchor_x = cursor_x + glyph.metrics.advance / 2.0;
                let (origin_x, origin_y) = typographic_glyph_origin(
                    (anchor_x.round() as i32, center.1.round() as i32),
                    glyph.metrics.advance,
                    glyph.metrics.outline_center_x,
                    glyph.metrics.ascent,
                    glyph.metrics.descent,
                );
                render_resolved_glyphs(
                    PreparedGlyphRenderRequest {
                        glyphs: std::slice::from_ref(glyph),
                        font_config: &component.font,
                        color: frame_state_color(state)?,
                        background_color: ctx.background_color,
                        origin: (origin_x, origin_y),
                    },
                    typography,
                    image,
                )?;
                cursor_x += glyph.metrics.advance;
            }
        }
    }
    Ok(())
}

fn render_glyph_component(
    ctx: &FrameRenderCtx<'_>,
    component: &crate::scene::GlyphComponent,
    state: &ComponentFrameState,
    fonts: &[Arc<FontLoader>],
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    let expression = resolve_component_content(ctx, &component.content)?;
    let parsed_specs = CharEntry::glyph_specs_from(&expression);
    let standalone_combining_overlay = ctx.config.overlay_enabled
        && component.overlay_combining_mark
        && parsed_specs.len() == 1
        && parsed_specs
            .first()
            .is_some_and(|spec| match spec.selector {
                GlyphSelector::CodePoint(value) => ctx.unicode_manager.is_combining_mark(value),
                GlyphSelector::Name(_) | GlyphSelector::Index(_) => false,
            });
    if !standalone_combining_overlay {
        if let Some(text) = unicode_shaping_sequence(&parsed_specs) {
            let center_x =
                (state.position.x * ctx.width as f64) as f32 + ctx.config.text_x_offset as f32;
            let center_y =
                (state.position.y * ctx.height as f64) as f32 + ctx.config.text_y_offset as f32;
            let font_size = fonts
                .first()
                .map(|font| font.render_size())
                .unwrap_or(component.font.size);
            return typography.render_centered_unicode_sequence(
                CenteredUnicodeRequest {
                    component_id: &component.id,
                    config: &component.font,
                    font_size,
                    text: &text,
                    font_index: None,
                    color: frame_state_color(state)?.clone(),
                    center: (center_x, center_y),
                },
                image,
            );
        }

        let segments = glyph_sequence_segments(&parsed_specs);
        let has_unicode = segments
            .iter()
            .any(|segment| matches!(segment, GlyphSequenceSegment::Unicode { .. }));
        if has_unicode {
            return render_mixed_glyph_segments(
                ctx,
                component,
                state,
                fonts,
                &parsed_specs,
                typography,
                image,
            );
        }
    }

    let glyphs = resolve_glyphs(&expression, fonts);
    if glyphs.is_empty() {
        return Ok(());
    }

    let prepared = prepare_explicit_glyphs(&glyphs, fonts, &component.font, typography)?;
    let primary = prepared
        .first()
        .context("Glyph component is enabled without a loaded font")?;
    let anchor_x = (state.position.x * ctx.width as f64) as i32;
    let anchor_y = (state.position.y * ctx.height as f64) as i32;
    let glyph_advance: f32 = prepared.iter().map(|glyph| glyph.metrics.advance).sum();
    let outline_center_x = (glyph_advance.abs() <= f32::EPSILON)
        .then_some(primary.metrics.outline_center_x)
        .flatten();
    let (origin_x, origin_y) = typographic_glyph_origin(
        (
            anchor_x + ctx.config.text_x_offset,
            anchor_y + ctx.config.text_y_offset,
        ),
        glyph_advance,
        outline_center_x,
        primary.metrics.ascent,
        primary.metrics.descent,
    );

    let overlay_codepoint = if component.overlay_combining_mark {
        CharEntry::glyph_specs_from(&expression)
            .into_iter()
            .find_map(|spec| match spec.selector {
                GlyphSelector::CodePoint(value) => Some(value),
                GlyphSelector::Name(_) | GlyphSelector::Index(_) => None,
            })
    } else {
        None
    };
    if let Some(codepoint) = overlay_codepoint {
        let dotted_circle = GlyphSelector::CodePoint(0x25CC);
        if let Some(overlay_font) = fonts.iter().find(|font| font.has_selector(&dotted_circle)) {
            render_combining_overlay(
                CombiningOverlayRequest {
                    ctx,
                    state,
                    font: overlay_font,
                    font_config: &component.font,
                    codepoint,
                    anchor: (
                        anchor_x + ctx.config.text_x_offset,
                        anchor_y + ctx.config.text_y_offset,
                    ),
                },
                typography,
                image,
            )?;
        }
    }

    render_resolved_glyphs(
        PreparedGlyphRenderRequest {
            glyphs: &prepared,
            font_config: &component.font,
            color: frame_state_color(state)?,
            background_color: ctx.background_color,
            origin: (origin_x, origin_y),
        },
        typography,
        image,
    )
}

fn render_text_component(
    ctx: &FrameRenderCtx<'_>,
    component: &crate::scene::TextComponent,
    state: &ComponentFrameState,
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    let text = resolve_component_content(ctx, &component.content)?;
    typography.render_text(
        component,
        &text,
        frame_state_color(state)?.clone(),
        TextPlacement {
            origin_x: (state.position.x * ctx.width as f64) as f32,
            last_baseline_y: (state.position.y * ctx.height as f64) as f32,
            canvas_width: ctx.width,
        },
        image,
    )
}

fn render_leaf_component(
    ctx: &FrameRenderCtx<'_>,
    component: &SceneComponent,
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    let state = ctx
        .component_states
        .get(component.id())
        .context("Missing component frame state")?;
    let fonts = ctx
        .component_fonts
        .get(component.id())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    match component {
        SceneComponent::Glyph(glyph) => {
            render_glyph_component(ctx, glyph, state, fonts, typography, image)?;
        }
        SceneComponent::Text(text) => {
            render_text_component(ctx, text, state, typography, image)?;
        }
        SceneComponent::Image(image_component) => {
            let source = ctx
                .component_images
                .get(component.id())
                .context("Missing prepared image asset")?;
            let mut resolved = image_component.clone();
            if let Some(opacity) = state.opacity {
                resolved.opacity = opacity as f32;
            }
            image_renderer::render_image_component(
                &resolved,
                &state.position,
                source.as_ref(),
                image,
            );
        }
        SceneComponent::ProgressBar(progress) => {
            let mut resolved = progress.clone();
            if let Some(value) = state.progress {
                resolved.progress = value;
            }
            primitive_renderer::render_progress_bar(&resolved, &state.position, image);
        }
        SceneComponent::Group(_) => unreachable!("groups are handled by recursive traversal"),
    }
    Ok(())
}

fn render_scene_component(
    ctx: &FrameRenderCtx<'_>,
    component: &SceneComponent,
    inherited: SceneRenderContext,
    typography: &mut TypographyRenderer,
    image: &mut RgbaImage,
) -> Result<()> {
    if !component.enabled() {
        return Ok(());
    }
    if let SceneComponent::Group(group) = component {
        let state = ctx
            .component_states
            .get(component.id())
            .context("Missing group frame state")?;
        let mut transform = group.transform.clone();
        if let Some(scale) = &state.scale {
            transform.scale = scale.clone();
        }
        if let Some(rotation) = state.rotation {
            transform.rotation = rotation;
        }
        let local = transform_matrix(&transform, &state.position, ctx.width, ctx.height);
        let opacity = state.opacity.unwrap_or(f64::from(group.opacity)) as f32;
        let child_context = inherited.child(local, opacity);
        if child_context.opacity <= 0.0 {
            return Ok(());
        }
        for child in &group.children {
            render_scene_component(ctx, child, child_context, typography, image)?;
        }
        return Ok(());
    }

    if inherited.transform.is_identity() && (inherited.opacity - 1.0).abs() <= f32::EPSILON {
        return render_leaf_component(ctx, component, typography, image);
    }

    let mut layer = RgbaImage::new(ctx.width, ctx.height);
    render_leaf_component(ctx, component, typography, &mut layer)?;
    composite_affine(&layer, image, inherited.transform, inherited.opacity);
    Ok(())
}

fn render_frame_parallel(
    ctx: &FrameRenderCtx<'_>,
    typography: &mut TypographyRenderer,
) -> Result<Vec<u8>> {
    let mut image = RgbaImage::new(ctx.width, ctx.height);
    fill_frame_background(&mut image, ctx.background_color);
    let root = SceneRenderContext::default();
    for component in &ctx.config.components {
        render_scene_component(ctx, component, root, typography, &mut image)?;
    }
    Ok(image.into_raw())
}

#[cfg(test)]
#[path = "../tests/unit/renderer.rs"]
mod tests;
