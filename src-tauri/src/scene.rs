use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const DEFAULT_GLYPH_COMPONENT_ID: &str = "main";
pub const DEFAULT_TEXT_COMPONENT_ID: &str = "bottom";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Scale2D {
    pub x: f64,
    pub y: f64,
}

impl Default for Scale2D {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnimatableProperty {
    PositionX,
    PositionY,
    ScaleX,
    ScaleY,
    Rotation,
    Opacity,
    Color,
    Progress,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ComponentPropertyValue {
    Number(f64),
    Color(Color),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Transform2D {
    #[serde(default = "default_zero_position")]
    pub translation: Position,
    #[serde(default)]
    pub scale: Scale2D,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub anchor: Position,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            translation: default_zero_position(),
            scale: Scale2D::default(),
            rotation: 0.0,
            anchor: Position::default(),
        }
    }
}

fn default_zero_position() -> Position {
    Position { x: 0.0, y: 0.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 0.25,
            height: 0.25,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    #[default]
    Contain,
    Cover,
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProgressBarBorder {
    pub color: Color,
    pub width: u32,
}

impl Default for ProgressBarBorder {
    fn default() -> Self {
        Self {
            color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum FontSource {
    Path(PathBuf),
    Face {
        path: PathBuf,
        #[serde(default)]
        face_index: u32,
    },
}

impl FontSource {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Path(path) => path,
            Self::Face { path, .. } => path,
        }
    }

    pub fn path_mut(&mut self) -> &mut PathBuf {
        match self {
            Self::Path(path) => path,
            Self::Face { path, .. } => path,
        }
    }

    pub fn face_index(&self) -> u32 {
        match self {
            Self::Path(_) => 0,
            Self::Face { face_index, .. } => *face_index,
        }
    }
}

impl From<PathBuf> for FontSource {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
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
    pub fonts: Vec<FontSource>,
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
    pub fonts: Vec<FontSource>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageComponent {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub source: PathBuf,
    pub position: Position,
    pub size: Size,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub fit: ImageFit,
}

impl Default for ImageComponent {
    fn default() -> Self {
        Self {
            id: "image".to_string(),
            enabled: false,
            source: PathBuf::new(),
            position: Position::default(),
            size: Size::default(),
            opacity: 1.0,
            fit: ImageFit::Contain,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProgressBarComponent {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub position: Position,
    pub size: Size,
    #[serde(default)]
    pub progress: f64,
    pub background_color: Color,
    pub fill_color: Color,
    #[serde(default)]
    pub direction: ProgressDirection,
    #[serde(default)]
    pub border: Option<ProgressBarBorder>,
}

impl Default for ProgressBarComponent {
    fn default() -> Self {
        Self {
            id: "progress".to_string(),
            enabled: false,
            position: Position { x: 0.5, y: 0.9 },
            size: Size {
                width: 0.6,
                height: 0.04,
            },
            progress: 0.5,
            background_color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 96,
            },
            fill_color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 224,
            },
            direction: ProgressDirection::LeftToRight,
            border: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GroupComponent {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub transform: Transform2D,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub children: Vec<SceneComponent>,
}

impl Default for GroupComponent {
    fn default() -> Self {
        Self {
            id: "group".to_string(),
            enabled: false,
            transform: Transform2D::default(),
            opacity: 1.0,
            children: Vec::new(),
        }
    }
}

fn default_opacity() -> f32 {
    1.0
}

/// A renderable scene component. The enum stays intentionally concrete: each
/// variant exposes only the capabilities it actually owns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneComponent {
    Glyph(GlyphComponent),
    Text(TextComponent),
    Image(ImageComponent),
    ProgressBar(ProgressBarComponent),
    Group(GroupComponent),
}

impl SceneComponent {
    pub fn id(&self) -> &str {
        match self {
            Self::Glyph(c) => &c.id,
            Self::Text(c) => &c.id,
            Self::Image(c) => &c.id,
            Self::ProgressBar(c) => &c.id,
            Self::Group(c) => &c.id,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Glyph(c) => c.enabled,
            Self::Text(c) => c.enabled,
            Self::Image(c) => c.enabled,
            Self::ProgressBar(c) => c.enabled,
            Self::Group(c) => c.enabled,
        }
    }

    pub fn position(&self) -> &Position {
        match self {
            Self::Glyph(c) => &c.position,
            Self::Text(c) => &c.position,
            Self::Image(c) => &c.position,
            Self::ProgressBar(c) => &c.position,
            Self::Group(c) => &c.transform.translation,
        }
    }

    pub fn color(&self) -> Option<&Color> {
        match self {
            Self::Glyph(c) => Some(&c.color),
            Self::Text(c) => Some(&c.color),
            Self::Image(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn supports_animatable_property(&self, property: AnimatableProperty) -> bool {
        use AnimatableProperty as Property;
        match property {
            Property::PositionX | Property::PositionY => true,
            Property::ScaleX | Property::ScaleY | Property::Rotation => {
                matches!(self, Self::Group(_))
            }
            Property::Opacity => matches!(self, Self::Image(_) | Self::Group(_)),
            Property::Color => matches!(self, Self::Glyph(_) | Self::Text(_)),
            Property::Progress => matches!(self, Self::ProgressBar(_)),
        }
    }

    pub fn fonts(&self) -> Option<&[FontSource]> {
        match self {
            Self::Glyph(c) => Some(&c.fonts),
            Self::Text(c) => Some(&c.fonts),
            Self::Image(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn fonts_mut(&mut self) -> Option<&mut Vec<FontSource>> {
        match self {
            Self::Glyph(c) => Some(&mut c.fonts),
            Self::Text(c) => Some(&mut c.fonts),
            Self::Image(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn font(&self) -> Option<&FontConfig> {
        match self {
            Self::Glyph(c) => Some(&c.font),
            Self::Text(c) => Some(&c.font),
            Self::Image(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn font_mut(&mut self) -> Option<&mut FontConfig> {
        match self {
            Self::Glyph(c) => Some(&mut c.font),
            Self::Text(c) => Some(&mut c.font),
            Self::Image(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn image_source_mut(&mut self) -> Option<&mut PathBuf> {
        match self {
            Self::Image(c) => Some(&mut c.source),
            Self::Glyph(_) | Self::Text(_) | Self::ProgressBar(_) | Self::Group(_) => None,
        }
    }

    pub fn children(&self) -> &[SceneComponent] {
        match self {
            Self::Group(c) => &c.children,
            _ => &[],
        }
    }
}
