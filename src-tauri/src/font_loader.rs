use crate::json_config::{FontConfig, GlyphSelector};
use ab_glyph::{Font, FontArc, GlyphId, Point, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::RgbaImage;
use resvg::{tiny_skia, usvg};
use std::collections::{hash_map::Entry, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct AdvancedFontSettings {
    features: std::collections::BTreeMap<String, u32>,
    variations: std::collections::BTreeMap<String, f32>,
    optical_sizing: bool,
}

struct SvgFontRenderer {
    fontdb: Arc<usvg::fontdb::Database>,
    family: String,
    settings: AdvancedFontSettings,
    is_color_font: bool,
    font_size: f32,
}

const SYNTHETIC_GLYPH_CODEPOINT: u32 = 0xF0000;
const SYNTHETIC_GLYPH_TEXT: &str = "\u{F0000}";
const MAX_SYNTHETIC_GLYPH_RENDERERS: usize = 8;

struct AdvancedGlyphRendererCache {
    source_data: Arc<[u8]>,
    config: FontConfig,
    is_color_font: bool,
    renderers: Mutex<HashMap<u16, Arc<SvgFontRenderer>>>,
}

impl AdvancedGlyphRendererCache {
    fn renderer_for(&self, glyph: GlyphId) -> Result<Arc<SvgFontRenderer>> {
        if let Some(renderer) = self
            .renderers
            .lock()
            .map_err(|_| anyhow::anyhow!("Advanced glyph renderer cache lock poisoned"))?
            .get(&glyph.0)
            .cloned()
        {
            return Ok(renderer);
        }

        let remapped_font = remap_cmap_to_glyph(self.source_data.as_ref(), glyph)?;
        let renderer = Arc::new(SvgFontRenderer::new(
            &remapped_font,
            &self.config,
            self.is_color_font,
        )?);

        let mut cache = self
            .renderers
            .lock()
            .map_err(|_| anyhow::anyhow!("Advanced glyph renderer cache lock poisoned"))?;
        if cache.len() >= MAX_SYNTHETIC_GLYPH_RENDERERS && !cache.contains_key(&glyph.0) {
            // Each entry owns a remapped copy of the font. Keep the cache small
            // so long #index sequences do not retain one full font per glyph.
            cache.clear();
        }
        Ok(cache
            .entry(glyph.0)
            .or_insert_with(|| renderer.clone())
            .clone())
    }
}

pub struct FontLoader {
    font: FontArc,
    px_scale: PxScale,
    scale_factor: f32,
    ascent: f32,
    descent: f32,
    glyph_names: HashMap<String, GlyphId>,
    glyph_count: u16,
    advanced_renderer: Option<SvgFontRenderer>,
    advanced_glyph_renderers: Option<AdvancedGlyphRendererCache>,
}

impl FontLoader {
    pub fn from_path(path: &Path, font_size: f32) -> Result<Self> {
        Self::from_path_with_config(
            path,
            &FontConfig {
                size: font_size,
                ..FontConfig::default()
            },
        )
    }

    /// Load a font with OpenType shaping and variable-font settings.
    pub fn from_path_with_config(path: &Path, config: &FontConfig) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read font file: {}", path.display()))?;
        Self::from_bytes_with_config(&data, config)
    }

    pub fn from_bytes(data: &[u8], font_size: f32) -> Result<Self> {
        Self::from_bytes_with_config(
            data,
            &FontConfig {
                size: font_size,
                ..FontConfig::default()
            },
        )
    }

    fn from_bytes_with_config(data: &[u8], config: &FontConfig) -> Result<Self> {
        validate_settings(config)?;
        let font_size = config.size;
        let font = FontArc::try_from_vec(data.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;

        let units_per_em = font.units_per_em().unwrap_or(1000.0);
        let default_scale = PxScale::from(units_per_em);
        let scaled_default = font.as_scaled(default_scale);
        let raw_ascent = scaled_default.ascent();

        // Normalize by ascent so different UPM values produce comparable visual sizes.
        let corrected_scale = if raw_ascent > 0.0 {
            font_size * units_per_em / raw_ascent
        } else {
            font_size
        };

        let px_scale = PxScale::from(corrected_scale);
        let scaled = font.as_scaled(px_scale);

        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let scale_factor = corrected_scale / font_size;

        let face = ttf_parser::Face::parse(data, 0)
            .map_err(|_| anyhow::anyhow!("Failed to parse font OpenType tables"))?;
        let mut glyph_names = HashMap::new();
        for index in 0..face.number_of_glyphs() {
            let glyph_id = ttf_parser::GlyphId(index);
            if let Some(name) = face.glyph_name(glyph_id) {
                glyph_names.insert(name.to_string(), GlyphId(index));
            }
        }
        let has_advanced_tables = [b"COLR", b"CBDT", b"sbix", b"SVG ", b"fvar"]
            .iter()
            .any(|tag| {
                face.raw_face()
                    .table(ttf_parser::Tag::from_bytes(tag))
                    .is_some()
            });
        let has_explicit_settings =
            !config.font_feature_settings.is_empty() || !config.font_variation_settings.is_empty();
        let is_color_font = has_advanced_tables
            && [b"COLR", b"CBDT", b"sbix", b"SVG "].iter().any(|tag| {
                face.raw_face()
                    .table(ttf_parser::Tag::from_bytes(tag))
                    .is_some()
            });
        let advanced_renderer = if has_advanced_tables || has_explicit_settings {
            Some(SvgFontRenderer::new(data, config, is_color_font)?)
        } else {
            None
        };
        let advanced_glyph_renderers =
            advanced_renderer
                .as_ref()
                .map(|_| AdvancedGlyphRendererCache {
                    source_data: Arc::from(data),
                    config: config.clone(),
                    is_color_font,
                    renderers: Mutex::new(HashMap::new()),
                });

        Ok(Self {
            font,
            px_scale,
            scale_factor,
            ascent,
            descent,
            glyph_names,
            glyph_count: face.number_of_glyphs(),
            advanced_renderer,
            advanced_glyph_renderers,
        })
    }

    pub fn metrics(&self) -> (f32, f32) {
        (self.ascent, self.descent)
    }

    pub fn font(&self) -> &FontArc {
        &self.font
    }

    pub fn px_scale(&self) -> PxScale {
        self.px_scale
    }

    pub fn has_char(&self, c: char) -> bool {
        self.font.glyph_id(c) != ab_glyph::GlyphId(0)
    }

    pub fn glyph_id(&self, c: char) -> ab_glyph::GlyphId {
        self.font.glyph_id(c)
    }

    pub fn glyph_id_for_selector(&self, selector: &GlyphSelector) -> GlyphId {
        match selector {
            GlyphSelector::CodePoint(cp) => char::from_u32(*cp)
                .map(|c| self.font.glyph_id(c))
                .unwrap_or(GlyphId(0)),
            GlyphSelector::Name(name) => self.glyph_names.get(name).copied().unwrap_or(GlyphId(0)),
            GlyphSelector::Index(index) => GlyphId(*index),
        }
    }

    pub fn has_selector(&self, selector: &GlyphSelector) -> bool {
        match selector {
            GlyphSelector::Index(index) => (*index as u32) < self.glyph_count as u32,
            _ => self.glyph_id_for_selector(selector) != GlyphId(0),
        }
    }

    pub fn glyph_width_id(&self, glyph: GlyphId) -> f32 {
        self.font.as_scaled(self.px_scale).h_advance(glyph)
    }

    pub fn rasterize(&self, c: char) -> Option<(u32, u32, Vec<u8>)> {
        let glyph = self.font.glyph_id(c);

        let glyph = glyph.with_scale_and_position(self.px_scale, Point { x: 0.0, y: 0.0 });

        if let Some(outline) = self.font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            let width = bounds.width().ceil() as u32;
            let height = bounds.height().ceil() as u32;

            if width == 0 || height == 0 {
                return None;
            }

            let mut pixels = vec![0u8; (width * height) as usize];

            outline.draw(|x, y, coverage| {
                let idx = (y * width + x) as usize;
                if idx < pixels.len() {
                    pixels[idx] = (coverage * 255.0) as u8;
                }
            });

            Some((width, height, pixels))
        } else {
            None
        }
    }

    /// Draw with a single coverage mask. `color` is already precomposed over
    /// the background, and maximum coverage prevents overlapping contours from
    /// multiplying alpha.
    pub fn render_to_image(
        &self,
        c: char,
        img: &mut RgbaImage,
        x: i32,
        y: i32,
        color: (u8, u8, u8),
    ) {
        let glyph = self.font.glyph_id(c);

        self.render_glyph_id_to_image(glyph, img, x, y, color);
    }

    pub fn render_glyph_id_to_image(
        &self,
        glyph: GlyphId,
        img: &mut RgbaImage,
        x: i32,
        y: i32,
        color: (u8, u8, u8),
    ) {
        let glyph = glyph.with_scale_and_position(self.px_scale, Point { x: 0.0, y: 0.0 });

        if let Some(outline) = self.font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            let width = bounds.width().ceil() as usize;
            let height = bounds.height().ceil() as usize;

            if width == 0 || height == 0 {
                return;
            }

            // Accumulate maximum coverage before compositing once.
            let mut coverage_buf = vec![0.0f32; width * height];
            let min_x = bounds.min.x as i32;
            let min_y = bounds.min.y as i32;

            outline.draw(|px, py, coverage| {
                let idx = (py as usize) * width + px as usize;
                if idx < coverage_buf.len() {
                    coverage_buf[idx] = coverage_buf[idx].max(coverage);
                }
            });

            let offset_x = x + min_x;
            let offset_y = y + min_y;

            for py in 0..height {
                for px in 0..width {
                    let coverage = coverage_buf[py * width + px];
                    if coverage > 0.0 {
                        let img_x = offset_x + px as i32;
                        let img_y = offset_y + py as i32;

                        if img_x >= 0
                            && img_x < img.width() as i32
                            && img_y >= 0
                            && img_y < img.height() as i32
                        {
                            let pixel = img.get_pixel_mut(img_x as u32, img_y as u32);
                            let cov = coverage.clamp(0.0, 1.0);
                            pixel[0] = (color.0 as f32 * cov + pixel[0] as f32 * (1.0 - cov))
                                .round() as u8;
                            pixel[1] = (color.1 as f32 * cov + pixel[1] as f32 * (1.0 - cov))
                                .round() as u8;
                            pixel[2] = (color.2 as f32 * cov + pixel[2] as f32 * (1.0 - cov))
                                .round() as u8;
                            pixel[3] = 255;
                        }
                    }
                }
            }
        }
    }

    /// Render through usvg/resvg when the font contains color or variable tables.
    /// Returns `true` when the advanced backend handled the glyph.
    pub fn render_advanced_to_image(
        &self,
        c: char,
        img: &mut RgbaImage,
        x: i32,
        baseline_y: i32,
        color: (u8, u8, u8, u8),
    ) -> Result<bool> {
        let Some(renderer) = &self.advanced_renderer else {
            return Ok(false);
        };
        let mut layer = renderer.render_layer(
            &c.to_string(),
            img.width(),
            img.height(),
            x,
            baseline_y,
            if renderer.is_color_font {
                (255, 255, 255, 255)
            } else {
                color
            },
        )?;
        // A color-capable font can still contain ordinary monochrome glyphs.
        // Re-render neutral layers with the requested text color so a mixed
        // color/black-and-white run does not turn the monochrome glyph white.
        if renderer.is_color_font && !has_chromatic_pixels(&layer) {
            layer = renderer.render_layer(
                &c.to_string(),
                img.width(),
                img.height(),
                x,
                baseline_y,
                color,
            )?;
        }
        if alpha_bounds(&layer).is_none() {
            return Ok(false);
        }
        self.composite_aligned_color_glyph(c, img, x, baseline_y, layer)?;
        Ok(true)
    }

    /// Render a glyph selected by glyph ID through the same OpenType-aware
    /// backend used for Unicode code points.
    ///
    /// The temporary font view maps a private-use code point to `glyph`, so
    /// rustybuzz starts shaping from the explicitly selected glyph while still
    /// applying configured GSUB/GPOS features and variable-font coordinates.
    pub fn render_advanced_glyph_to_image(
        &self,
        glyph: GlyphId,
        img: &mut RgbaImage,
        x: i32,
        baseline_y: i32,
        color: (u8, u8, u8, u8),
    ) -> Result<bool> {
        let Some(cache) = &self.advanced_glyph_renderers else {
            return Ok(false);
        };
        let renderer = cache.renderer_for(glyph)?;
        let mut layer = renderer.render_layer(
            SYNTHETIC_GLYPH_TEXT,
            img.width(),
            img.height(),
            x,
            baseline_y,
            if renderer.is_color_font {
                (255, 255, 255, 255)
            } else {
                color
            },
        )?;
        if renderer.is_color_font && !has_chromatic_pixels(&layer) {
            layer = renderer.render_layer(
                SYNTHETIC_GLYPH_TEXT,
                img.width(),
                img.height(),
                x,
                baseline_y,
                color,
            )?;
        }
        if alpha_bounds(&layer).is_none() {
            return Ok(false);
        }
        self.composite_aligned_color_glyph_id(glyph, img, x, baseline_y, layer)?;
        Ok(true)
    }

    /// Render a complete shaped text run through the advanced backend.
    pub fn render_advanced_text_to_image(
        &self,
        text: &str,
        img: &mut RgbaImage,
        x: i32,
        baseline_y: i32,
        color: (u8, u8, u8, u8),
    ) -> Result<bool> {
        let Some(renderer) = &self.advanced_renderer else {
            return Ok(false);
        };
        let paint = if renderer.is_color_font {
            (255, 255, 255, 255)
        } else {
            color
        };
        let layer = renderer.render_layer(text, img.width(), img.height(), x, baseline_y, paint)?;
        image::imageops::overlay(img, &layer, 0, 0);
        Ok(true)
    }

    fn composite_aligned_color_glyph(
        &self,
        c: char,
        target: &mut RgbaImage,
        x: i32,
        baseline_y: i32,
        layer: RgbaImage,
    ) -> Result<()> {
        self.composite_aligned_color_glyph_id(self.font.glyph_id(c), target, x, baseline_y, layer)
    }

    fn composite_aligned_color_glyph_id(
        &self,
        glyph: GlyphId,
        target: &mut RgbaImage,
        x: i32,
        baseline_y: i32,
        layer: RgbaImage,
    ) -> Result<()> {
        let Some((min_x, min_y, max_x, max_y)) = alpha_bounds(&layer) else {
            return Ok(());
        };
        let crop =
            image::imageops::crop_imm(&layer, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
                .to_image();

        // Use the same explicitly selected glyph's monochrome outline as the
        // size/position reference. This keeps /name and #index selectors on
        // the same baseline and centering path as Unicode selectors.
        let mut outline_layer = RgbaImage::new(target.width(), target.height());
        self.render_glyph_id_to_image(glyph, &mut outline_layer, x, baseline_y, (255, 255, 255));
        let normal_bounds = alpha_bounds(&outline_layer);
        let (dest_x, dest_y, render_w, render_h) =
            if let Some((nx, ny, nmax_x, nmax_y)) = normal_bounds {
                (nx as i64, ny as i64, nmax_x - nx + 1, nmax_y - ny + 1)
            } else {
                let center_x = x as f32 + self.glyph_width_id(glyph) / 2.0;
                let center_y = baseline_y as f32 - (self.ascent + self.descent) / 2.0;
                (
                    (center_x - crop.width() as f32 / 2.0).round() as i64,
                    (center_y - crop.height() as f32 / 2.0).round() as i64,
                    crop.width(),
                    crop.height(),
                )
            };
        let resized = image::imageops::resize(
            &crop,
            render_w,
            render_h,
            image::imageops::FilterType::Lanczos3,
        );
        image::imageops::overlay(target, &resized, dest_x, dest_y);
        Ok(())
    }

    pub fn glyph_width(&self, c: char) -> f32 {
        let glyph = self.font.glyph_id(c);
        self.font.as_scaled(self.px_scale).h_advance(glyph)
    }
}

pub struct FontCache {
    cache: HashMap<String, FontLoader>,
    default_font_size: f32,
}

impl FontCache {
    pub fn new(default_font_size: f32) -> Self {
        Self {
            cache: HashMap::new(),
            default_font_size,
        }
    }

    pub fn get_or_load(&mut self, path: &Path) -> Result<&FontLoader> {
        let key = path.to_string_lossy().into_owned();
        match self.cache.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let loader = FontLoader::from_path(path, self.default_font_size)?;
                Ok(entry.insert(loader))
            }
        }
    }

    pub fn get(&self, path: &Path) -> Option<&FontLoader> {
        let key = path.to_string_lossy().to_string();
        self.cache.get(&key)
    }
}

impl SvgFontRenderer {
    fn new(data: &[u8], config: &FontConfig, is_color_font: bool) -> Result<Self> {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_font_data(data.to_vec());
        let family = fontdb
            .faces()
            .next()
            .and_then(|face| face.families.first())
            .map(|(name, _)| name.clone())
            .ok_or_else(|| anyhow::anyhow!("No recognizable face/family in font"))?;

        Ok(Self {
            fontdb: Arc::new(fontdb),
            family,
            settings: AdvancedFontSettings {
                features: config.font_feature_settings.clone(),
                variations: config.font_variation_settings.clone(),
                optical_sizing: config.font_optical_sizing,
            },
            is_color_font,
            font_size: config.size,
        })
    }

    fn render_layer(
        &self,
        text: &str,
        width: u32,
        height: u32,
        x: i32,
        baseline_y: i32,
        color: (u8, u8, u8, u8),
    ) -> Result<RgbaImage> {
        let variations = css_settings(&self.settings.variations);
        let features = css_settings(&self.settings.features);
        let optical_sizing = if self.settings.optical_sizing {
            "auto"
        } else {
            "none"
        };
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><text x="{x}" y="{y}" font-family="{family}" font-size="{font_size}" fill="rgb({r},{g},{b})" opacity="{opacity}" font-feature-settings="{features}" font-variation-settings="{variations}" font-optical-sizing="{optical_sizing}">{text}</text></svg>"#,
            width = width,
            height = height,
            x = x,
            y = baseline_y,
            family = escape_xml(&self.family),
            font_size = self.font_size,
            r = color.0,
            g = color.1,
            b = color.2,
            opacity = color.3 as f32 / 255.0,
            features = features,
            variations = variations,
            text = escape_xml(text),
        );

        let options = usvg::Options {
            font_family: self.family.clone(),
            fontdb: self.fontdb.clone(),
            ..Default::default()
        };
        let tree = usvg::Tree::from_str(&svg, &options)
            .context("Failed to build color font SVG render tree")?;
        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or_else(|| anyhow::anyhow!("Failed to allocate color font render buffer"))?;
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );

        RgbaImage::from_raw(width, height, pixmap.take_demultiplied())
            .ok_or_else(|| anyhow::anyhow!("Color font render buffer has invalid dimensions"))
    }
}

fn remap_cmap_to_glyph(data: &[u8], glyph: GlyphId) -> Result<Vec<u8>> {
    if data.len() < 12 {
        anyhow::bail!("Font is too short to contain an sfnt header");
    }

    let num_tables = read_be_u16(data, 4)? as usize;
    let directory_len = 12usize
        .checked_add(
            num_tables
                .checked_mul(16)
                .context("Font table directory overflow")?,
        )
        .context("Font table directory overflow")?;
    if directory_len > data.len() {
        anyhow::bail!("Font table directory is truncated");
    }

    let mut cmap_record = None;
    let mut head_offset = None;
    for index in 0..num_tables {
        let record = 12 + index * 16;
        let tag = &data[record..record + 4];
        if tag == b"cmap" {
            cmap_record = Some(record);
        } else if tag == b"head" {
            let offset = read_be_u32(data, record + 8)? as usize;
            let length = read_be_u32(data, record + 12)? as usize;
            if length >= 12 {
                if let Some(end) = offset.checked_add(length) {
                    if end <= data.len() {
                        head_offset = Some(offset);
                    }
                }
            }
        }
    }
    let cmap_record = cmap_record.context("Font has no cmap table to remap")?;

    let cmap = synthetic_cmap(glyph);
    let mut remapped = data.to_vec();
    while !remapped.len().is_multiple_of(4) {
        remapped.push(0);
    }
    let cmap_offset = remapped.len();
    remapped.extend_from_slice(&cmap);
    while !remapped.len().is_multiple_of(4) {
        remapped.push(0);
    }

    write_be_u32(&mut remapped, cmap_record + 4, table_checksum(&cmap))?;
    write_be_u32(
        &mut remapped,
        cmap_record + 8,
        u32::try_from(cmap_offset).context("Remapped cmap offset exceeds u32")?,
    )?;
    write_be_u32(
        &mut remapped,
        cmap_record + 12,
        u32::try_from(cmap.len()).context("Remapped cmap length exceeds u32")?,
    )?;

    if let Some(head_offset) = head_offset {
        write_be_u32(&mut remapped, head_offset + 8, 0)?;
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(table_checksum(&remapped));
        write_be_u32(&mut remapped, head_offset + 8, adjustment)?;
    }

    Ok(remapped)
}

fn synthetic_cmap(glyph: GlyphId) -> Vec<u8> {
    let mut cmap = Vec::with_capacity(48);
    push_be_u16(&mut cmap, 0);
    push_be_u16(&mut cmap, 2);

    // Unicode full repertoire and Windows Unicode full repertoire records.
    push_be_u16(&mut cmap, 0);
    push_be_u16(&mut cmap, 4);
    push_be_u32(&mut cmap, 20);
    push_be_u16(&mut cmap, 3);
    push_be_u16(&mut cmap, 10);
    push_be_u32(&mut cmap, 20);

    // Format 12, one group mapping the private-use scalar to the requested ID.
    push_be_u16(&mut cmap, 12);
    push_be_u16(&mut cmap, 0);
    push_be_u32(&mut cmap, 28);
    push_be_u32(&mut cmap, 0);
    push_be_u32(&mut cmap, 1);
    push_be_u32(&mut cmap, SYNTHETIC_GLYPH_CODEPOINT);
    push_be_u32(&mut cmap, SYNTHETIC_GLYPH_CODEPOINT);
    push_be_u32(&mut cmap, glyph.0 as u32);
    cmap
}

fn read_be_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .context("Unexpected end of font data")?
        .try_into()
        .context("Invalid 16-bit font value")?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .context("Unexpected end of font data")?
        .try_into()
        .context("Invalid 32-bit font value")?;
    Ok(u32::from_be_bytes(bytes))
}

fn write_be_u32(data: &mut [u8], offset: usize, value: u32) -> Result<()> {
    data.get_mut(offset..offset + 4)
        .context("Unexpected end of font data")?
        .copy_from_slice(&value.to_be_bytes());
    Ok(())
}

fn push_be_u16(data: &mut Vec<u8>, value: u16) {
    data.extend_from_slice(&value.to_be_bytes());
}

fn push_be_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_be_bytes());
}

fn table_checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        bounds = Some(match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    }
    bounds
}

fn has_chromatic_pixels(image: &RgbaImage) -> bool {
    image
        .pixels()
        .any(|pixel| pixel[3] > 0 && (pixel[0] != pixel[1] || pixel[1] != pixel[2]))
}

fn validate_settings(config: &FontConfig) -> Result<()> {
    if !config.size.is_finite() || config.size <= 0.0 {
        anyhow::bail!("Font size must be a positive finite number");
    }
    for tag in config
        .font_feature_settings
        .keys()
        .chain(config.font_variation_settings.keys())
    {
        if tag.len() != 4 || !tag.is_ascii() {
            anyhow::bail!("OpenType tag must be 4 ASCII characters: {tag:?}");
        }
    }
    for (tag, value) in &config.font_variation_settings {
        if !value.is_finite() {
            anyhow::bail!("Variable font axis {tag:?} value must be a finite number");
        }
    }
    Ok(())
}

fn css_settings<T: std::fmt::Display>(settings: &std::collections::BTreeMap<String, T>) -> String {
    if settings.is_empty() {
        return "normal".to_string();
    }
    settings
        .iter()
        .map(|(tag, value)| format!("&quot;{tag}&quot; {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        read_be_u16, read_be_u32, remap_cmap_to_glyph, synthetic_cmap, table_checksum,
        validate_settings, write_be_u32, SYNTHETIC_GLYPH_CODEPOINT,
    };
    use crate::json_config::FontConfig;
    use ab_glyph::GlyphId;

    #[test]
    fn rejects_invalid_opentype_tags() {
        let mut config = FontConfig::default();
        config.font_feature_settings.insert("liga".into(), 0);
        assert!(validate_settings(&config).is_ok());

        config
            .font_variation_settings
            .insert("weight".into(), 700.0);
        assert!(validate_settings(&config).is_err());
    }

    #[test]
    fn rejects_non_finite_axis_values() {
        let mut config = FontConfig::default();
        config
            .font_variation_settings
            .insert("wght".into(), f32::NAN);
        assert!(validate_settings(&config).is_err());
    }

    #[test]
    fn synthetic_cmap_maps_private_use_codepoint_to_selected_glyph() {
        let cmap = synthetic_cmap(GlyphId(37));
        assert_eq!(cmap.len(), 48);
        assert_eq!(read_be_u16(&cmap, 0).unwrap(), 0);
        assert_eq!(read_be_u16(&cmap, 2).unwrap(), 2);
        assert_eq!(read_be_u16(&cmap, 20).unwrap(), 12);
        assert_eq!(read_be_u32(&cmap, 36).unwrap(), SYNTHETIC_GLYPH_CODEPOINT);
        assert_eq!(read_be_u32(&cmap, 40).unwrap(), SYNTHETIC_GLYPH_CODEPOINT);
        assert_eq!(read_be_u32(&cmap, 44).unwrap(), 37);
    }

    #[test]
    fn remapped_sfnt_has_valid_checksum_and_cmap_record() {
        // A minimal synthetic sfnt directory is sufficient here because the
        // remapper only replaces table-directory metadata and appends cmap.
        let mut font = vec![0u8; 60];
        font[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        font[4..6].copy_from_slice(&2u16.to_be_bytes());

        font[12..16].copy_from_slice(b"head");
        write_be_u32(&mut font, 20, 44).unwrap();
        write_be_u32(&mut font, 24, 12).unwrap();

        font[28..32].copy_from_slice(b"cmap");
        write_be_u32(&mut font, 36, 56).unwrap();
        write_be_u32(&mut font, 40, 4).unwrap();

        let remapped = remap_cmap_to_glyph(&font, GlyphId(91)).unwrap();
        let cmap_offset = read_be_u32(&remapped, 36).unwrap() as usize;
        assert_eq!(read_be_u32(&remapped, 40).unwrap(), 48);
        assert_eq!(read_be_u32(&remapped, cmap_offset + 44).unwrap(), 91);
        assert_eq!(table_checksum(&remapped), 0xB1B0_AFBA);
    }
}
