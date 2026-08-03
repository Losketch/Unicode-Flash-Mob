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
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl From<Color> for (u8, u8, u8, u8) {
    fn from(c: Color) -> Self {
        (c.r, c.g, c.b, c.a)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Default for Position {
    fn default() -> Self {
        Position { x: 0.5, y: 0.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FontConfig {
    pub size: f32,
    /// OpenType feature tags, for example { "kern": 0, "liga": 1 }.
    #[serde(default)]
    pub font_feature_settings: BTreeMap<String, u32>,
    /// Variable-font axis coordinates, for example { "wght": 650, "wdth": 90 }.
    #[serde(default)]
    pub font_variation_settings: BTreeMap<String, f32>,
    /// Let the renderer set the `opsz` axis from the configured font size.
    #[serde(default = "default_true")]
    pub font_optical_sizing: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            size: 512.0,
            font_feature_settings: BTreeMap::new(),
            font_variation_settings: BTreeMap::new(),
            font_optical_sizing: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TextElement {
    pub id: String,
    pub fonts: Vec<PathBuf>,
    pub content: String,
    pub position: Position,
    pub color: Color,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub wrap: bool,
    #[serde(default)]
    pub max_width: f64,
}

fn default_true() -> bool {
    true
}

impl Default for TextElement {
    fn default() -> Self {
        let fonts = bundled_font_paths("fonts/NotoSansTest-Regular.ttf");
        TextElement {
            id: "main".to_string(),
            fonts,
            content: "{char}".to_string(),
            position: Position::default(),
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 128,
            },
            enabled: true,
            align: TextAlign::Left,
            wrap: false,
            max_width: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    SetBackgroundColor {
        color: Color,
    },
    SetTextColor {
        element_id: String,
        color: Color,
    },
    SetTextPosition {
        element_id: String,
        position: Position,
    },
    MoveTextPosition {
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

/// A glyph selector used by `characters[].code_point`.
///
/// Selectors may be concatenated (for example `U+0033U+0034`) and can carry
/// an optional zero-based main-font index (`U+0033{1}`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlyphSelector {
    CodePoint(u32),
    Name(String),
    Index(u16),
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
        } else if bytes[cursor].is_ascii_hexdigit() {
            // Keep accepting the legacy plain hexadecimal form.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RenderConfig {
    #[serde(default = "default_output_path")]
    pub output_path: PathBuf,
    pub resolution: (u32, u32),
    pub fps: f64,
    pub background_colors: Vec<Color>,
    pub dynamic_background: bool,
    pub background_color: Color,
    pub fixed_background: bool,

    pub main_font: FontConfig,
    pub bottom_font: FontConfig,

    pub text_x_offset: i32,
    pub text_y_offset: i32,

    pub overlay_enabled: bool,

    pub main_text: TextElement,
    pub bottom_text: TextElement,

    pub characters: Vec<CharEntry>,

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
        let main_fonts = bundled_font_paths("fonts/NotoSansTest-Regular.ttf");
        let bottom_fonts = bundled_font_paths("fonts/IBMPlexSans-Bold.ttf");
        let bottom_text_enabled = !bottom_fonts.is_empty();

        RenderConfig {
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
            main_font: FontConfig {
                size: 512.0,
                ..FontConfig::default()
            },
            bottom_font: FontConfig {
                size: 42.0,
                ..FontConfig::default()
            },
            text_x_offset: 0,
            text_y_offset: 0,
            overlay_enabled: true,
            main_text: TextElement {
                id: "main".to_string(),
                fonts: main_fonts,
                content: "{char}".to_string(),
                position: Position { x: 0.5, y: 0.5 },
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 128,
                },
                enabled: true,
                align: TextAlign::Center,
                wrap: false,
                max_width: 0.0,
            },
            bottom_text: TextElement {
                id: "bottom".to_string(),
                fonts: bottom_fonts,
                content: "{code} {description}".to_string(),
                position: Position { x: 0.05, y: 0.95 },
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 128,
                },
                enabled: bottom_text_enabled,
                align: TextAlign::Left,
                wrap: true,
                max_width: 0.9,
            },
            characters: Vec::new(),
            events: Vec::new(),
            ffmpeg: FfmpegConfig::default(),
            music_path: None,
            title: "Unicode Flash Mob".to_string(),
        }
    }
}

impl RenderConfig {
    fn normalize_bundled_asset_references(&mut self) {
        for font_path in self
            .main_text
            .fonts
            .iter_mut()
            .chain(self.bottom_text.fonts.iter_mut())
        {
            *font_path = portable_asset_reference(font_path);
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
        let mut config: Self = serde_json::from_str(&content)?;
        config.normalize_bundled_asset_references();
        Ok(config)
    }

    pub fn to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut portable_config = self.clone();
        portable_config.normalize_bundled_asset_references();
        let content = serde_json::to_string_pretty(&portable_config)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod font_config_tests {
    use super::{
        portable_asset_reference, CharEntry, Event, EventType, FontConfig, GlyphSelector,
        RenderConfig,
    };
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
