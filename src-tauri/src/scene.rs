use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const DEFAULT_GLYPH_COMPONENT_ID: &str = "main";
pub const DEFAULT_TEXT_COMPONENT_ID: &str = "bottom";

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
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl From<Color> for (u8, u8, u8, u8) {
    fn from(color: Color) -> Self {
        (color.r, color.g, color.b, color.a)
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
        Self { x: 0.5, y: 0.5 }
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
        Self {
            size: 512.0,
            font_feature_settings: BTreeMap::new(),
            font_variation_settings: BTreeMap::new(),
            font_optical_sizing: true,
        }
    }
}

pub fn default_true() -> bool {
    true
}

fn default_glyph_content() -> String {
    "{glyph}".to_string()
}

/// A glyph selector resolved by `GlyphComponent`. This is a scene-domain type,
/// not a configuration concern: renderers and font backends may use it directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlyphSelector {
    CodePoint(u32),
    Name(String),
    Index(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GlyphComponent {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_glyph_content")]
    pub content: String,
    pub position: Position,
    pub color: Color,
    pub fonts: Vec<PathBuf>,
    pub font: FontConfig,
    /// Draw U+25CC behind combining marks when the current entry is a mark.
    #[serde(default = "default_true")]
    pub overlay_combining_mark: bool,
}

impl Default for GlyphComponent {
    fn default() -> Self {
        Self {
            id: DEFAULT_GLYPH_COMPONENT_ID.to_string(),
            enabled: true,
            content: default_glyph_content(),
            position: Position { x: 0.5, y: 0.5 },
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 128,
            },
            fonts: Vec::new(),
            font: FontConfig {
                size: 512.0,
                ..FontConfig::default()
            },
            overlay_combining_mark: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TextComponent {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub content: String,
    pub position: Position,
    pub color: Color,
    pub fonts: Vec<PathBuf>,
    pub font: FontConfig,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub wrap: bool,
    #[serde(default)]
    pub max_width: f64,
}

impl Default for TextComponent {
    fn default() -> Self {
        Self {
            id: DEFAULT_TEXT_COMPONENT_ID.to_string(),
            enabled: false,
            content: "{code}\n{description}".to_string(),
            position: Position { x: 0.05, y: 0.95 },
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 128,
            },
            fonts: Vec::new(),
            font: FontConfig {
                size: 42.0,
                ..FontConfig::default()
            },
            align: TextAlign::Left,
            wrap: true,
            max_width: 0.9,
        }
    }
}

/// A renderable scene component. The enum is intentionally small: new component
/// kinds can be added without changing the scene container or template system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneComponent {
    Glyph(GlyphComponent),
    Text(TextComponent),
}

impl SceneComponent {
    pub fn id(&self) -> &str {
        match self {
            Self::Glyph(component) => &component.id,
            Self::Text(component) => &component.id,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Glyph(component) => component.enabled,
            Self::Text(component) => component.enabled,
        }
    }

    pub fn position(&self) -> &Position {
        match self {
            Self::Glyph(component) => &component.position,
            Self::Text(component) => &component.position,
        }
    }

    pub fn color(&self) -> &Color {
        match self {
            Self::Glyph(component) => &component.color,
            Self::Text(component) => &component.color,
        }
    }

    /// Font paths owned by font-backed components. Future non-text components
    /// can return `None` without inheriting irrelevant font state.
    pub fn fonts(&self) -> Option<&[PathBuf]> {
        match self {
            Self::Glyph(component) => Some(&component.fonts),
            Self::Text(component) => Some(&component.fonts),
        }
    }

    pub fn fonts_mut(&mut self) -> Option<&mut Vec<PathBuf>> {
        match self {
            Self::Glyph(component) => Some(&mut component.fonts),
            Self::Text(component) => Some(&mut component.fonts),
        }
    }

    pub fn font(&self) -> Option<&FontConfig> {
        match self {
            Self::Glyph(component) => Some(&component.font),
            Self::Text(component) => Some(&component.font),
        }
    }

    pub fn font_mut(&mut self) -> Option<&mut FontConfig> {
        match self {
            Self::Glyph(component) => Some(&mut component.font),
            Self::Text(component) => Some(&mut component.font),
        }
    }

    pub fn as_glyph(&self) -> Option<&GlyphComponent> {
        match self {
            Self::Glyph(component) => Some(component),
            Self::Text(_) => None,
        }
    }

    pub fn as_glyph_mut(&mut self) -> Option<&mut GlyphComponent> {
        match self {
            Self::Glyph(component) => Some(component),
            Self::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> Option<&TextComponent> {
        match self {
            Self::Text(component) => Some(component),
            Self::Glyph(_) => None,
        }
    }

    pub fn as_text_mut(&mut self) -> Option<&mut TextComponent> {
        match self {
            Self::Text(component) => Some(component),
            Self::Glyph(_) => None,
        }
    }
}
