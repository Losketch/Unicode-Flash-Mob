use crate::content_template::ContentTemplate;
use crate::scene::{
    Color, FontConfig, GlyphComponent, GlyphSelector, Position, SceneComponent, TextAlign,
    TextComponent,
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

pub(crate) fn bundled_font_paths(relative: impl AsRef<Path>) -> Vec<PathBuf> {
    let relative = relative.as_ref();
    asset_path(relative)
        .is_file()
        .then(|| bundled_asset_reference(relative))
        .into_iter()
        .collect()
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

    pub fn glyph_specs(&self) -> Vec<GlyphSpec> {
        let specs = parse_glyph_specs(self.code_point.trim());
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

    pub fn code_point_value(&self) -> u32 {
        let s = self.code_point.trim();
        if let Some(GlyphSpec {
            selector: GlyphSelector::CodePoint(value),
            ..
        }) = parse_glyph_specs(s).into_iter().next()
        {
            return value;
        }
        if let Some(hex) = s.strip_prefix("U+").or_else(|| s.strip_prefix("u+")) {
            u32::from_str_radix(hex, 16).unwrap_or(0)
        } else if let Some(stripped) = s.strip_prefix("0x") {
            u32::from_str_radix(stripped, 16).unwrap_or(0)
        } else {
            u32::from_str_radix(s, 16).unwrap_or(0)
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

pub const CURRENT_SCHEMA_VERSION: u32 = 4;
const LEGACY_SCHEMA_VERSION: u32 = 3;

// Configurations written before schema v4 had no schema_version field. Using
// v3 as the deserialization default lets us distinguish those files from an
// intentional v4 scene with an empty `components` array.
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

    /// Legacy global pixel offsets retained until per-component transforms land.
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

impl RenderConfig {
    /// Upgrade legacy v3 fixed text slots into the ordered scene-component model.
    /// Calling this repeatedly is harmless.
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
                glyph.fonts = legacy.fonts;
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
                text.fonts = legacy.fonts;
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
        self.components.iter().find_map(SceneComponent::as_glyph)
    }

    pub fn primary_glyph_component_mut(&mut self) -> Option<&mut GlyphComponent> {
        self.components
            .iter_mut()
            .find_map(SceneComponent::as_glyph_mut)
    }

    pub fn primary_text_component(&self) -> Option<&TextComponent> {
        self.components.iter().find_map(SceneComponent::as_text)
    }

    pub fn primary_text_component_mut(&mut self) -> Option<&mut TextComponent> {
        self.components
            .iter_mut()
            .find_map(SceneComponent::as_text_mut)
    }

    fn normalize_bundled_asset_references(&mut self) {
        for component in &mut self.components {
            if let Some(fonts) = component.fonts_mut() {
                for font_path in fonts {
                    *font_path = portable_asset_reference(font_path);
                }
            }
        }
        if let Some(music_path) = self.music_path.as_mut() {
            *music_path = portable_asset_reference(music_path);
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

    pub fn duration(&self) -> f64 {
        if self.fps.is_finite() && self.fps > 0.0 {
            self.total_frames() as f64 / self.fps
        } else {
            0.0
        }
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
        let content = serde_json::to_string_pretty(&portable_config)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod font_config_tests {
    use super::{portable_asset_reference, CharEntry, Event, EventType, RenderConfig};
    use crate::scene::{Color, FontConfig, GlyphSelector, Position, TextAlign};
    use std::path::{Path, PathBuf};

    #[test]
    fn bundled_asset_paths_are_serialized_portably() {
        let development_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("fonts")
            .join("Example.ttf");
        assert_eq!(
            portable_asset_reference(&development_path),
            PathBuf::from("assets/fonts/Example.ttf")
        );
    }

    #[test]
    fn legacy_size_only_config_gets_advanced_defaults() {
        let config: FontConfig = serde_json::from_str(r#"{"size":128.0}"#).unwrap();
        assert_eq!(config.size, 128.0);
        assert!(config.font_feature_settings.is_empty());
        assert!(config.font_variation_settings.is_empty());
        assert!(config.font_optical_sizing);
    }

    #[test]
    fn default_config_serializes_scene_schema_without_legacy_slots() {
        let config = RenderConfig::default();
        let value = serde_json::to_value(&config).unwrap();

        assert_eq!(
            value["schema_version"].as_u64(),
            Some(super::CURRENT_SCHEMA_VERSION as u64)
        );
        assert_eq!(value["components"].as_array().map(Vec::len), Some(2));
        assert_eq!(value["components"][0]["type"].as_str(), Some("glyph"));
        assert_eq!(value["components"][1]["type"].as_str(), Some("text"));
        assert!(value.get("main_text").is_none());
        assert!(value.get("bottom_text").is_none());
        assert!(value.get("main_font").is_none());
        assert!(value.get("bottom_font").is_none());
    }

    #[test]
    fn future_schema_is_not_downgraded_by_normalization() {
        let mut config = RenderConfig {
            schema_version: super::CURRENT_SCHEMA_VERSION + 1,
            ..RenderConfig::default()
        };
        config.normalize_scene();
        assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION + 1);
    }

    #[test]
    fn missing_schema_version_is_treated_as_legacy_v3() {
        let mut value = serde_json::to_value(RenderConfig::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("schema_version");
        object.remove("components");

        let mut config: RenderConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.schema_version, super::LEGACY_SCHEMA_VERSION);
        config.normalize_scene();

        assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
        assert_eq!(config.components.len(), 2);
    }

    #[test]
    fn intentional_empty_v4_scene_stays_empty() {
        let mut config = RenderConfig::default();
        config.components.clear();
        config.normalize_scene();
        assert!(config.components.is_empty());
    }

    #[test]
    fn legacy_fixed_slots_migrate_to_ordered_components() {
        let mut config = RenderConfig {
            schema_version: super::LEGACY_SCHEMA_VERSION,
            components: Vec::new(),
            legacy_main_text: Some(super::LegacyTextElement {
                id: "legacy-main".to_string(),
                fonts: vec![PathBuf::from("main.ttf")],
                content: "ignored-by-v3-main-renderer".to_string(),
                position: Position { x: 0.25, y: 0.4 },
                color: Color {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 4,
                },
                enabled: true,
                align: TextAlign::Center,
                wrap: true,
                max_width: 0.5,
            }),
            legacy_main_font: Some(FontConfig {
                size: 321.0,
                ..FontConfig::default()
            }),
            legacy_bottom_text: Some(super::LegacyTextElement {
                id: "legacy-bottom".to_string(),
                fonts: vec![PathBuf::from("bottom.ttf")],
                content: "{code} :: {description}".to_string(),
                position: Position { x: 0.1, y: 0.9 },
                color: Color {
                    r: 5,
                    g: 6,
                    b: 7,
                    a: 8,
                },
                enabled: true,
                align: TextAlign::Right,
                wrap: false,
                max_width: 0.8,
            }),
            legacy_bottom_font: Some(FontConfig {
                size: 37.0,
                ..FontConfig::default()
            }),
            ..RenderConfig::default()
        };

        config.normalize_scene();

        assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
        assert_eq!(config.components.len(), 2);
        let glyph = config.primary_glyph_component().unwrap();
        assert_eq!(glyph.id, "legacy-main");
        assert_eq!(glyph.content, "{glyph}");
        assert_eq!(glyph.fonts, vec![PathBuf::from("main.ttf")]);
        assert_eq!(glyph.font.size, 321.0);
        let text = config.components[1].as_text().unwrap();
        assert_eq!(text.id, "legacy-bottom");
        assert_eq!(text.content, "{code} :: {description}");
        assert_eq!(text.font.size, 37.0);
        assert_eq!(text.align, TextAlign::Right);
    }

    #[test]
    fn legacy_event_names_deserialize_to_component_events() {
        let event_type: EventType = serde_json::from_str(
            r#"{"set_text_position":{"element_id":"caption","position":{"x":0.2,"y":0.3}}}"#,
        )
        .unwrap();

        match event_type {
            EventType::SetComponentPosition {
                element_id,
                position,
            } => {
                assert_eq!(element_id, "caption");
                assert_eq!(position.x, 0.2);
                assert_eq!(position.y, 0.3);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_concatenated_glyph_selectors_and_font_indices() {
        let entry = CharEntry {
            code_point: "U+0033{0}/at{0}#0{1}".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: None,
        };
        let specs = entry.glyph_specs();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].selector, GlyphSelector::CodePoint(0x33));
        assert_eq!(specs[0].font_index, Some(0));
        assert_eq!(specs[1].selector, GlyphSelector::Name("at".to_string()));
        assert_eq!(specs[1].font_index, Some(0));
        assert_eq!(specs[2].selector, GlyphSelector::Index(0));
        assert_eq!(specs[2].font_index, Some(1));
    }

    #[test]
    fn selected_unicode_text_ignores_explicit_glyph_selectors() {
        let entry = CharEntry {
            code_point: "U+0041/fooU+0042#3".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: None,
        };
        assert_eq!(entry.selected_unicode_text(), "AB");

        let literal = CharEntry {
            code_point: "not-a-selector".to_string(),
            ..entry
        };
        assert_eq!(literal.selected_unicode_text(), "not-a-selector");

        let hex_like_literal = CharEntry {
            code_point: "face-to-face".to_string(),
            ..literal
        };
        assert_eq!(hex_like_literal.selected_unicode_text(), "face-to-face");
    }

    #[test]
    fn accepts_all_codepoint_prefix_forms() {
        let entry = CharEntry {
            code_point: "U+0041u+0042,0x43 0X44 45".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: None,
        };
        let values: Vec<_> = entry
            .glyph_specs()
            .into_iter()
            .map(|spec| spec.selector)
            .collect();
        assert_eq!(
            values,
            vec![
                GlyphSelector::CodePoint(0x41),
                GlyphSelector::CodePoint(0x42),
                GlyphSelector::CodePoint(0x43),
                GlyphSelector::CodePoint(0x44),
                GlyphSelector::CodePoint(0x45),
            ]
        );
    }

    #[test]
    fn separates_named_glyphs_from_delimited_codepoints() {
        let entry = CharEntry {
            code_point: "/A U+0042,/C".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: None,
        };
        let selectors: Vec<_> = entry
            .glyph_specs()
            .into_iter()
            .map(|spec| spec.selector)
            .collect();
        assert_eq!(
            selectors,
            vec![
                GlyphSelector::Name("A".to_string()),
                GlyphSelector::CodePoint(0x42),
                GlyphSelector::Name("C".to_string()),
            ]
        );
    }

    #[test]
    fn reachable_pause_events_extend_total_frames() {
        let config = RenderConfig {
            fps: 10.0,
            characters: vec![CharEntry {
                code_point: "U+0041".to_string(),
                description: String::new(),
                background_color: None,
                text_color: None,
                position: None,
                duration_frames: Some(5),
            }],
            events: vec![Event {
                frame: 2,
                event_type: EventType::Pause { duration: 0.3 },
            }],
            ..Default::default()
        };
        assert_eq!(config.total_frames(), 8);
    }

    #[test]
    fn unreachable_pause_events_do_not_extend_total_frames() {
        let config = RenderConfig {
            fps: 10.0,
            characters: vec![CharEntry {
                code_point: "U+0041".to_string(),
                description: String::new(),
                background_color: None,
                text_color: None,
                position: None,
                duration_frames: Some(5),
            }],
            events: vec![Event {
                frame: 5,
                event_type: EventType::Pause { duration: 1.0 },
            }],
            ..Default::default()
        };
        assert_eq!(config.total_frames(), 5);
    }
}
