use crate::content_template::ContentTemplate;
use crate::scene::{
    AnimatableProperty, Color, ComponentPropertyValue, FontConfig, FontSource, GlyphComponent,
    GlyphSelector, Position, SceneComponent, TextAlign, TextComponent,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Build a default output filename like `20260713_134455_output.mp4`.
pub fn default_output_filename() -> String {
    let now = chrono::Local::now();
    now.format("%Y%m%d_%H%M%S_output.mp4").to_string()
}

/// Default output directory used by `extract` and `config` subcommands.
pub const DEFAULT_OUTPUT_DIR: &str = "./output";

static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

#[cfg(feature = "gui")]
pub(crate) fn set_resource_dir(path: PathBuf) {
    let _ = RESOURCE_DIR.set(path);
}

/// Resolve an optional application asset in development, packaged GUI, or CLI layouts.
pub fn asset_path(relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();

    if let Some(resource_dir) = RESOURCE_DIR.get() {
        let bundled = resource_dir.join("assets").join(relative);
        if bundled.exists() {
            return bundled;
        }
    }
    let development = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(relative);
    if development.exists() {
        return development;
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(executable_dir) = executable.parent() {
            #[cfg(target_os = "linux")]
            if let Some(executable_name) = executable.file_name() {
                let mut linux_resource_dirs = vec![
                    PathBuf::from("/usr/lib").join(executable_name),
                    executable_dir.join("../lib").join(executable_name),
                ];
                if let Some(app_dir) = std::env::var_os("APPDIR") {
                    linux_resource_dirs.insert(
                        0,
                        PathBuf::from(app_dir).join("usr/lib").join(executable_name),
                    );
                }
                for resource_dir in linux_resource_dirs {
                    let bundled = resource_dir.join("assets").join(relative);
                    if bundled.exists() {
                        return bundled;
                    }
                }
            }

            for bundled in [
                executable_dir
                    .join("resources")
                    .join("assets")
                    .join(relative),
                executable_dir.join("../Resources/assets").join(relative),
                executable_dir.join("assets").join(relative),
            ] {
                if bundled.exists() {
                    return bundled;
                }
            }
        }
    }

    PathBuf::from("assets").join(relative)
}

fn bundled_asset_reference(relative: &Path) -> PathBuf {
    Path::new("assets").join(relative)
}

fn is_bundled_asset_reference(path: &Path) -> bool {
    path.components().next().is_some_and(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("assets")
    })
}

fn is_bare_command_reference(path: &Path) -> bool {
    !path.is_absolute() && path.components().count() == 1
}

fn config_base_dir(path: &Path) -> PathBuf {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(parent))
            .unwrap_or_else(|_| parent.to_path_buf())
    }
}

fn resolve_config_reference(
    path: &Path,
    base_dir: &Path,
    preserve_bundled: bool,
    preserve_bare_command: bool,
) -> PathBuf {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || (preserve_bundled && is_bundled_asset_reference(path))
        || (preserve_bare_command && is_bare_command_reference(path))
    {
        return path.to_path_buf();
    }
    base_dir.join(path)
}

fn portable_config_reference(
    path: &Path,
    base_dir: &Path,
    preserve_bundled: bool,
    preserve_bare_command: bool,
) -> PathBuf {
    if path.as_os_str().is_empty()
        || (preserve_bundled && is_bundled_asset_reference(path))
        || (preserve_bare_command && is_bare_command_reference(path))
    {
        return path.to_path_buf();
    }
    if path.is_absolute() {
        if let Ok(relative) = path.strip_prefix(base_dir) {
            if !relative.as_os_str().is_empty() {
                return relative.to_path_buf();
            }
        }
    }
    path.to_path_buf()
}

fn portable_asset_reference(path: &Path) -> PathBuf {
    if let Ok(relative) = path.strip_prefix("assets") {
        return bundled_asset_reference(relative);
    }

    if let Some(resource_dir) = RESOURCE_DIR.get() {
        if let Ok(relative) = path.strip_prefix(resource_dir.join("assets")) {
            return bundled_asset_reference(relative);
        }
    }

    let development_assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    if let Ok(relative) = path.strip_prefix(development_assets) {
        return bundled_asset_reference(relative);
    }

    if let Some(relative) = asset_relative_path(path) {
        if asset_path(&relative).exists() {
            return bundled_asset_reference(&relative);
        }
    }

    path.to_path_buf()
}

fn asset_relative_path(path: &Path) -> Option<PathBuf> {
    let components: Vec<_> = path.components().collect();
    let asset_index = components.iter().position(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("assets")
    })?;
    let relative =
        components[asset_index + 1..]
            .iter()
            .fold(PathBuf::new(), |mut result, component| {
                result.push(component.as_os_str());
                result
            });
    (!relative.as_os_str().is_empty()).then_some(relative)
}

/// Resolve a stable `assets/...` configuration path to the current Tauri
/// resource directory. Existing user-selected paths are returned unchanged.
pub fn resolve_asset_reference(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }

    if let Some(relative) = asset_relative_path(path) {
        let resolved = asset_path(relative);
        if resolved.exists() {
            return resolved;
        }
    }

    path.to_path_buf()
}

pub(crate) fn bundled_font_paths(relative: impl AsRef<Path>) -> Vec<FontSource> {
    let relative = relative.as_ref();
    asset_path(relative)
        .is_file()
        .then(|| FontSource::from(bundled_asset_reference(relative)))
        .into_iter()
        .collect()
}

fn resolve_component_config_paths(components: &mut [SceneComponent], base_dir: &Path) {
    for component in components {
        if let Some(fonts) = component.fonts_mut() {
            for font in fonts {
                let path = font.path_mut();
                *path = resolve_config_reference(path, base_dir, true, false);
            }
        }
        if let Some(source) = component.image_source_mut() {
            *source = resolve_config_reference(source, base_dir, true, false);
        }
        if let SceneComponent::Group(group) = component {
            resolve_component_config_paths(&mut group.children, base_dir);
        }
    }
}

fn portable_component_config_paths(components: &mut [SceneComponent], base_dir: &Path) {
    for component in components {
        if let Some(fonts) = component.fonts_mut() {
            for font in fonts {
                let path = font.path_mut();
                *path = portable_config_reference(path, base_dir, true, false);
            }
        }
        if let Some(source) = component.image_source_mut() {
            *source = portable_config_reference(source, base_dir, true, false);
        }
        if let SceneComponent::Group(group) = component {
            portable_component_config_paths(&mut group.children, base_dir);
        }
    }
}

fn default_glyph_component() -> GlyphComponent {
    GlyphComponent {
        fonts: bundled_font_paths("fonts/NotoSansTest-Regular.ttf"),
        ..GlyphComponent::default()
    }
}

fn default_text_component() -> TextComponent {
    let fonts = bundled_font_paths("fonts/IBMPlexSans-Bold.ttf");
    TextComponent {
        enabled: !fonts.is_empty(),
        fonts,
        ..TextComponent::default()
    }
}

fn default_scene_components() -> Vec<SceneComponent> {
    vec![
        SceneComponent::Glyph(default_glyph_component()),
        SceneComponent::Text(default_text_component()),
    ]
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationCurve {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
}

impl AnimationCurve {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            AnimationCurve::Linear => t,
            AnimationCurve::EaseIn => t * t,
            AnimationCurve::EaseOut => t * (2.0 - t),
            AnimationCurve::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            AnimationCurve::Bounce => {
                const N1: f64 = 7.5625;
                const D1: f64 = 2.75;
                let t = t.min(1.0);
                if t < 1.0 / D1 {
                    N1 * t * t
                } else if t < 2.0 / D1 {
                    N1 * (t - 1.5 / D1) * (t - 1.5 / D1) + 0.75
                } else if t < 2.5 / D1 {
                    N1 * (t - 2.25 / D1) * (t - 2.25 / D1) + 0.9375
                } else {
                    N1 * (t - 2.625 / D1) * (t - 2.625 / D1) + 0.984375
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct LegacyTextElement {
    pub id: String,
    pub fonts: Vec<PathBuf>,
    pub content: String,
    pub position: Position,
    pub color: Color,
    #[serde(default = "crate::scene::default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub wrap: bool,
    #[serde(default)]
    pub max_width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    SetBackgroundColor {
        color: Color,
    },
    #[serde(rename = "set_component_color", alias = "set_text_color")]
    SetComponentColor {
        element_id: String,
        color: Color,
    },
    #[serde(rename = "set_component_position", alias = "set_text_position")]
    SetComponentPosition {
        element_id: String,
        position: Position,
    },
    #[serde(rename = "move_component_position", alias = "move_text_position")]
    MoveComponentPosition {
        element_id: String,
        start_position: Position,
        end_position: Position,
        duration: f64,
        curve: AnimationCurve,
    },
    SetComponentProperty {
        element_id: String,
        property: AnimatableProperty,
        value: ComponentPropertyValue,
    },
    AnimateComponentProperty {
        element_id: String,
        property: AnimatableProperty,
        start_value: ComponentPropertyValue,
        end_value: ComponentPropertyValue,
        duration: f64,
        curve: AnimationCurve,
    },
    SetFontFeature {
        element_id: String,
        tag: String,
        value: u32,
    },
    SetFontVariation {
        element_id: String,
        axis: String,
        value: f32,
    },
    AnimateFontVariation {
        element_id: String,
        axis: String,
        start_value: f32,
        end_value: f32,
        duration: f64,
        curve: AnimationCurve,
    },
    ColorTransition {
        start_color: Color,
        end_color: Color,
        duration: f64,
        curve: AnimationCurve,
    },
    Pause {
        duration: f64,
    },
}

pub(crate) fn duration_to_frames(duration: f64, fps: f64) -> u64 {
    if !duration.is_finite() || !fps.is_finite() || duration <= 0.0 || fps <= 0.0 {
        return 0;
    }
    (duration * fps).round().clamp(0.0, u64::MAX as f64) as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Event {
    pub frame: u64,
    pub event_type: EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharEntry {
    pub code_point: String,
    pub description: String,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub position: Option<Position>,
    pub duration_frames: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlyphSpec {
    pub selector: GlyphSelector,
    pub font_index: Option<usize>,
}

fn parse_font_index(input: &[u8], cursor: &mut usize) -> Option<usize> {
    if *cursor >= input.len() || input[*cursor] != b'{' {
        return None;
    }
    let start = *cursor + 1;
    let end = input[start..].iter().position(|&b| b == b'}')? + start;
    let value = std::str::from_utf8(&input[start..end]).ok()?.parse().ok()?;
    *cursor = end + 1;
    Some(value)
}

fn parse_codepoint_spec(
    value: &str,
    input: &[u8],
    cursor: &mut usize,
    prefix_len: usize,
) -> Option<GlyphSpec> {
    let original_cursor = *cursor;
    *cursor += prefix_len;
    let start = *cursor;
    while *cursor < input.len() && input[*cursor].is_ascii_hexdigit() {
        *cursor += 1;
    }
    if *cursor == start {
        *cursor = original_cursor;
        return None;
    }

    let codepoint = match u32::from_str_radix(&value[start..*cursor], 16) {
        Ok(codepoint) => codepoint,
        Err(_) => {
            *cursor = original_cursor;
            return None;
        }
    };
    let font_index = parse_font_index(input, cursor);
    Some(GlyphSpec {
        selector: GlyphSelector::CodePoint(codepoint),
        font_index,
    })
}

fn is_selector_separator(byte: u8) -> bool {
    byte == b',' || byte.is_ascii_whitespace()
}

/// Legacy selectors may be written as bare hexadecimal (for example `45`).
/// Accept that form only as a complete selector token so ordinary text such
/// as `not-a-selector` is not partially interpreted as unrelated code points.
fn is_bare_hex_token(input: &[u8], cursor: usize) -> bool {
    if cursor > 0 && !is_selector_separator(input[cursor - 1]) {
        return false;
    }

    let mut end = cursor;
    while end < input.len() && input[end].is_ascii_hexdigit() {
        end += 1;
    }

    if end < input.len() && input[end] == b'{' {
        let index_start = end + 1;
        let mut index_end = index_start;
        while index_end < input.len() && input[index_end].is_ascii_digit() {
            index_end += 1;
        }
        if index_end == index_start || input.get(index_end) != Some(&b'}') {
            return false;
        }
        end = index_end + 1;
    }

    end == input.len()
        || input
            .get(end)
            .is_some_and(|byte| is_selector_separator(*byte) || *byte == b'/' || *byte == b'#')
        || ["U+", "u+", "0x", "0X"]
            .into_iter()
            .any(|prefix| input[end..].starts_with(prefix.as_bytes()))
}

fn parse_glyph_specs(value: &str) -> Vec<GlyphSpec> {
    let bytes = value.as_bytes();
    let mut cursor = 0usize;
    let mut specs = Vec::new();

    while cursor < bytes.len() {
        let prefix_len = ["U+", "u+", "0x", "0X"].into_iter().find_map(|prefix| {
            bytes[cursor..]
                .starts_with(prefix.as_bytes())
                .then_some(prefix.len())
        });

        if let Some(prefix_len) = prefix_len {
            if let Some(spec) = parse_codepoint_spec(value, bytes, &mut cursor, prefix_len) {
                specs.push(spec);
                continue;
            }
        } else if bytes[cursor] == b'/' {
            cursor += 1;
            let start = cursor;
            while cursor < bytes.len() {
                let starts_next_codepoint = ["U+", "u+", "0x", "0X"]
                    .into_iter()
                    .any(|prefix| bytes[cursor..].starts_with(prefix.as_bytes()));
                if bytes[cursor] == b'/'
                    || bytes[cursor] == b'#'
                    || bytes[cursor] == b'{'
                    || bytes[cursor] == b','
                    || bytes[cursor].is_ascii_whitespace()
                    || starts_next_codepoint
                {
                    break;
                }
                cursor += 1;
            }
            if cursor > start {
                let name = value[start..cursor].to_string();
                let font_index = parse_font_index(bytes, &mut cursor);
                specs.push(GlyphSpec {
                    selector: GlyphSelector::Name(name),
                    font_index,
                });
                continue;
            }
        } else if bytes[cursor] == b'#' {
            cursor += 1;
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor > start {
                if let Ok(index) = value[start..cursor].parse::<u16>() {
                    let font_index = parse_font_index(bytes, &mut cursor);
                    specs.push(GlyphSpec {
                        selector: GlyphSelector::Index(index),
                        font_index,
                    });
                    continue;
                }
            }
        } else if bytes[cursor].is_ascii_hexdigit() && is_bare_hex_token(bytes, cursor) {
            // Keep accepting the legacy plain hexadecimal form, but only as a
            // delimited token rather than hexadecimal-looking fragments of text.
            if let Some(spec) = parse_codepoint_spec(value, bytes, &mut cursor, 0) {
                specs.push(spec);
                continue;
            }
        }

        // Avoid an infinite loop for malformed input.
        cursor += 1;
    }
    specs
}

impl CharEntry {
    pub fn glyph_specs_from(value: &str) -> Vec<GlyphSpec> {
        let specs = parse_glyph_specs(value.trim());
        if specs.is_empty() {
            vec![GlyphSpec {
                selector: GlyphSelector::CodePoint(0),
                font_index: None,
            }]
        } else {
            specs
        }
    }

    /// Unicode text represented by the current selector expression. Explicit
    /// glyph-name/index selectors have no Unicode scalar and are omitted.
    pub fn selected_unicode_text(&self) -> String {
        let selected: String = parse_glyph_specs(self.code_point.trim())
            .into_iter()
            .filter_map(|spec| match spec.selector {
                GlyphSelector::CodePoint(codepoint) => char::from_u32(codepoint),
                GlyphSelector::Name(_) | GlyphSelector::Index(_) => None,
            })
            .collect();

        if selected.is_empty() {
            self.code_point.clone()
        } else {
            selected
        }
    }

    pub fn duration(&self) -> u64 {
        self.duration_frames.unwrap_or(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FfmpegConfig {
    pub path: PathBuf,
    pub crf: u32,
    pub preset: String,
    pub encoder: String,
    pub pixel_format: String,
    pub parallel_workers: usize,
    /// Limits complete RGBA frames waiting to be written in order.
    #[serde(default = "default_max_inflight_frames")]
    pub max_inflight_frames: usize,
    /// Splits the precomputed frame schedule across parallel FFmpeg processes.
    #[serde(default = "default_encoding_processes")]
    pub encoding_processes: usize,
}

fn default_max_inflight_frames() -> usize {
    2
}

fn default_encoding_processes() -> usize {
    1
}

impl Default for FfmpegConfig {
    fn default() -> Self {
        FfmpegConfig {
            path: PathBuf::from("ffmpeg"),
            crf: 18,
            preset: "fast".to_string(),
            encoder: "auto".to_string(),
            pixel_format: "yuv420p".to_string(),
            parallel_workers: 0,
            max_inflight_frames: default_max_inflight_frames(),
            encoding_processes: default_encoding_processes(),
        }
    }
}

const LEGACY_SCHEMA_VERSION: u32 = 3;
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

// v3 is the published fixed-slot schema that predates the stable v4 scene model.
// Configurations without schema metadata are treated as v3 input for migration.
fn default_deserialized_schema_version() -> u32 {
    LEGACY_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RenderConfig {
    #[serde(default = "default_deserialized_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_output_path")]
    pub output_path: PathBuf,
    pub resolution: (u32, u32),
    pub fps: f64,
    pub background_colors: Vec<Color>,
    pub dynamic_background: bool,
    pub background_color: Color,
    pub fixed_background: bool,

    /// Ordered scene components. Array order is the paint order.
    #[serde(default)]
    pub components: Vec<SceneComponent>,

    /// Named templates available to any component content string.
    #[serde(default)]
    pub content_templates: BTreeMap<String, ContentTemplate>,

    // Kept for v3 configuration loading only. `normalize_scene` consumes these
    // fields and new configurations never serialize them.
    #[serde(default, rename = "main_font", skip_serializing_if = "Option::is_none")]
    pub(crate) legacy_main_font: Option<FontConfig>,
    #[serde(
        default,
        rename = "bottom_font",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) legacy_bottom_font: Option<FontConfig>,
    #[serde(default, rename = "main_text", skip_serializing_if = "Option::is_none")]
    pub(crate) legacy_main_text: Option<LegacyTextElement>,
    #[serde(
        default,
        rename = "bottom_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) legacy_bottom_text: Option<LegacyTextElement>,

    /// Global pixel offsets applied to typography layers.
    #[serde(default)]
    pub text_x_offset: i32,
    #[serde(default)]
    pub text_y_offset: i32,

    #[serde(default = "crate::scene::default_true")]
    pub overlay_enabled: bool,

    pub characters: Vec<CharEntry>,

    #[serde(default)]
    pub events: Vec<Event>,

    pub ffmpeg: FfmpegConfig,

    pub music_path: Option<PathBuf>,

    pub title: String,
}

fn default_output_path() -> PathBuf {
    PathBuf::from(DEFAULT_OUTPUT_DIR).join(default_output_filename())
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            schema_version: CURRENT_SCHEMA_VERSION,
            output_path: default_output_path(),
            resolution: (1920, 1080),
            fps: 30.0,
            background_colors: vec![
                Color {
                    r: 171,
                    g: 223,
                    b: 86,
                    a: 255,
                },
                Color {
                    r: 109,
                    g: 231,
                    b: 78,
                    a: 255,
                },
                Color {
                    r: 104,
                    g: 245,
                    b: 159,
                    a: 255,
                },
                Color {
                    r: 0,
                    g: 190,
                    b: 157,
                    a: 255,
                },
                Color {
                    r: 0,
                    g: 203,
                    b: 129,
                    a: 255,
                },
                Color {
                    r: 168,
                    g: 253,
                    b: 154,
                    a: 255,
                },
                Color {
                    r: 153,
                    g: 254,
                    b: 169,
                    a: 255,
                },
                Color {
                    r: 152,
                    g: 252,
                    b: 202,
                    a: 255,
                },
                Color {
                    r: 152,
                    g: 254,
                    b: 235,
                    a: 255,
                },
                Color {
                    r: 151,
                    g: 236,
                    b: 253,
                    a: 255,
                },
                Color {
                    r: 51,
                    g: 226,
                    b: 253,
                    a: 255,
                },
                Color {
                    r: 52,
                    g: 181,
                    b: 223,
                    a: 255,
                },
                Color {
                    r: 0,
                    g: 149,
                    b: 224,
                    a: 255,
                },
                Color {
                    r: 205,
                    g: 155,
                    b: 255,
                    a: 255,
                },
                Color {
                    r: 171,
                    g: 155,
                    b: 255,
                    a: 255,
                },
                Color {
                    r: 238,
                    g: 154,
                    b: 255,
                    a: 255,
                },
                Color {
                    r: 255,
                    g: 154,
                    b: 240,
                    a: 255,
                },
                Color {
                    r: 254,
                    g: 154,
                    b: 204,
                    a: 255,
                },
                Color {
                    r: 255,
                    g: 154,
                    b: 170,
                    a: 255,
                },
                Color {
                    r: 252,
                    g: 171,
                    b: 154,
                    a: 255,
                },
                Color {
                    r: 251,
                    g: 201,
                    b: 154,
                    a: 255,
                },
                Color {
                    r: 253,
                    g: 236,
                    b: 153,
                    a: 255,
                },
                Color {
                    r: 238,
                    g: 254,
                    b: 153,
                    a: 255,
                },
                Color {
                    r: 207,
                    g: 255,
                    b: 155,
                    a: 255,
                },
            ],
            dynamic_background: true,
            background_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            fixed_background: false,
            components: default_scene_components(),
            content_templates: BTreeMap::new(),
            legacy_main_font: None,
            legacy_bottom_font: None,
            legacy_main_text: None,
            legacy_bottom_text: None,
            text_x_offset: 0,
            text_y_offset: 0,
            overlay_enabled: true,
            characters: Vec::new(),
            events: Vec::new(),
            ffmpeg: FfmpegConfig::default(),
            music_path: None,
            title: "Unicode Flash Mob".to_string(),
        }
    }
}

fn find_glyph_component(components: &[SceneComponent]) -> Option<&GlyphComponent> {
    for component in components {
        match component {
            SceneComponent::Glyph(glyph) => return Some(glyph),
            SceneComponent::Group(group) => {
                if let Some(glyph) = find_glyph_component(&group.children) {
                    return Some(glyph);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_glyph_component_mut(components: &mut [SceneComponent]) -> Option<&mut GlyphComponent> {
    for component in components {
        match component {
            SceneComponent::Glyph(glyph) => return Some(glyph),
            SceneComponent::Group(group) => {
                if let Some(glyph) = find_glyph_component_mut(&mut group.children) {
                    return Some(glyph);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_text_component_mut(components: &mut [SceneComponent]) -> Option<&mut TextComponent> {
    for component in components {
        match component {
            SceneComponent::Text(text) => return Some(text),
            SceneComponent::Group(group) => {
                if let Some(text) = find_text_component_mut(&mut group.children) {
                    return Some(text);
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_component_assets(components: &mut [SceneComponent]) {
    for component in components {
        if let Some(fonts) = component.fonts_mut() {
            for font in fonts {
                let path = font.path_mut();
                *path = portable_asset_reference(path);
            }
        }
        if let Some(source) = component.image_source_mut() {
            *source = portable_asset_reference(source);
        }
        if let SceneComponent::Group(group) = component {
            normalize_component_assets(&mut group.children);
        }
    }
}

impl RenderConfig {
    /// Upgrade the published v3 fixed-slot layout to the current v4 scene.
    /// Current v4 component arrays are left untouched. Calling this repeatedly
    /// is harmless.
    pub fn normalize_scene(&mut self) {
        // Never reinterpret or downgrade a configuration written by a newer
        // schema. Callers that render/save will reject it with a clear error.
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return;
        }

        let has_legacy_slots = self.legacy_main_font.is_some()
            || self.legacy_bottom_font.is_some()
            || self.legacy_main_text.is_some()
            || self.legacy_bottom_text.is_some();
        let should_migrate_legacy =
            self.schema_version < CURRENT_SCHEMA_VERSION || has_legacy_slots;

        if should_migrate_legacy && self.components.is_empty() {
            let mut glyph = default_glyph_component();
            if let Some(legacy) = self.legacy_main_text.take() {
                glyph.id = legacy.id;
                glyph.enabled = legacy.enabled;
                glyph.position = legacy.position;
                glyph.color = legacy.color;
                glyph.fonts = legacy.fonts.into_iter().map(FontSource::from).collect();
            }
            if let Some(font) = self.legacy_main_font.take() {
                glyph.font = font;
            }

            let mut text = default_text_component();
            if let Some(legacy) = self.legacy_bottom_text.take() {
                text.id = legacy.id;
                text.enabled = legacy.enabled;
                text.content = legacy.content;
                text.position = legacy.position;
                text.color = legacy.color;
                text.fonts = legacy.fonts.into_iter().map(FontSource::from).collect();
                text.align = legacy.align;
                text.wrap = legacy.wrap;
                text.max_width = legacy.max_width;
            }
            if let Some(font) = self.legacy_bottom_font.take() {
                text.font = font;
            }

            self.components.push(SceneComponent::Glyph(glyph));
            self.components.push(SceneComponent::Text(text));
        }

        self.legacy_main_font = None;
        self.legacy_bottom_font = None;
        self.legacy_main_text = None;
        self.legacy_bottom_text = None;
        self.schema_version = CURRENT_SCHEMA_VERSION;
    }

    pub fn primary_glyph_component(&self) -> Option<&GlyphComponent> {
        find_glyph_component(&self.components)
    }

    pub fn primary_glyph_component_mut(&mut self) -> Option<&mut GlyphComponent> {
        find_glyph_component_mut(&mut self.components)
    }

    pub fn primary_text_component_mut(&mut self) -> Option<&mut TextComponent> {
        find_text_component_mut(&mut self.components)
    }

    fn normalize_bundled_asset_references(&mut self) {
        normalize_component_assets(&mut self.components);
        if let Some(music_path) = self.music_path.as_mut() {
            *music_path = portable_asset_reference(music_path);
        }
    }

    fn resolve_config_references(&mut self, base_dir: &Path) {
        resolve_component_config_paths(&mut self.components, base_dir);
        if let Some(music_path) = self.music_path.as_mut() {
            *music_path = resolve_config_reference(music_path, base_dir, true, false);
        }
        self.output_path = resolve_config_reference(&self.output_path, base_dir, false, false);
        self.ffmpeg.path = resolve_config_reference(&self.ffmpeg.path, base_dir, true, true);
        for template in self.content_templates.values_mut() {
            if let ContentTemplate::External { executable, .. } = template {
                *executable = resolve_config_reference(executable, base_dir, true, true);
            }
        }
    }

    fn make_config_references_portable(&mut self, base_dir: &Path) {
        portable_component_config_paths(&mut self.components, base_dir);
        if let Some(music_path) = self.music_path.as_mut() {
            *music_path = portable_config_reference(music_path, base_dir, true, false);
        }
        self.output_path = portable_config_reference(&self.output_path, base_dir, false, false);
        self.ffmpeg.path = portable_config_reference(&self.ffmpeg.path, base_dir, true, true);
        for template in self.content_templates.values_mut() {
            if let ContentTemplate::External { executable, .. } = template {
                *executable = portable_config_reference(executable, base_dir, true, true);
            }
        }
    }

    pub fn total_frames(&self) -> u64 {
        let mut total = self
            .characters
            .iter()
            .fold(0u64, |sum, entry| sum.saturating_add(entry.duration()));
        let mut pauses: Vec<_> = self
            .events
            .iter()
            .filter_map(|event| match &event.event_type {
                EventType::Pause { duration } => Some((event.frame, *duration)),
                _ => None,
            })
            .collect();
        pauses.sort_unstable_by_key(|(frame, _)| *frame);
        for (frame, duration) in pauses {
            if frame < total {
                total = total.saturating_add(duration_to_frames(duration, self.fps));
            }
        }
        total
    }

    pub fn from_file(path: &Path) -> Result<Self, serde_json::Error> {
        let content = std::fs::read_to_string(path).map_err(serde_json::Error::io)?;
        let value: serde_json::Value = serde_json::from_str(&content)?;
        let schema_version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(LEGACY_SCHEMA_VERSION as u64);
        if schema_version > CURRENT_SCHEMA_VERSION as u64 {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Unsupported configuration schema {schema_version}; this build supports up to {CURRENT_SCHEMA_VERSION}"
                ),
            )));
        }

        let mut config: Self = serde_json::from_value(value)?;
        config.normalize_scene();
        config.normalize_bundled_asset_references();
        config.resolve_config_references(&config_base_dir(path));
        Ok(config)
    }

    pub fn to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Unsupported configuration schema {}; this build supports up to {}",
                    self.schema_version, CURRENT_SCHEMA_VERSION
                ),
            ));
        }

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut portable_config = self.clone();
        portable_config.normalize_scene();
        portable_config.normalize_bundled_asset_references();
        portable_config.make_config_references_portable(&config_base_dir(path));
        let content = serde_json::to_string_pretty(&portable_config)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
#[path = "../tests/unit/json_config.rs"]
mod font_config_tests;
