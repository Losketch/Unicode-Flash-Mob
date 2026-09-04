use crate::scene::{FontConfig, GlyphSelector};
use ab_glyph::{Font, FontArc, GlyphId, Point, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Parsed font metadata plus the original font bytes.
///
/// The original bytes are intentionally retained so shaping/raster backends can
/// share one immutable font resource instead of re-reading files or rebuilding
/// synthetic cmap tables.
pub struct FontLoader {
    font: FontArc,
    source_data: Arc<[u8]>,
    face_index: u32,
    px_scale: PxScale,
    glyph_names: HashMap<String, GlyphId>,
    glyph_count: u16,
}

impl FontLoader {
    pub fn from_path_with_config(path: &Path, config: &FontConfig) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read font file: {}", path.display()))?;
        Self::from_bytes_with_config(&data, config)
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

        // Preserve the historical main-glyph sizing behavior. TextComponent
        // layout uses standard CSS-like em sizing through Parley instead.
        let corrected_scale = if raw_ascent > 0.0 {
            font_size * units_per_em / raw_ascent
        } else {
            font_size
        };

        let px_scale = PxScale::from(corrected_scale);
        let face = ttf_parser::Face::parse(data, 0)
            .map_err(|_| anyhow::anyhow!("Failed to parse font OpenType tables"))?;
        let mut glyph_names = HashMap::new();
        for index in 0..face.number_of_glyphs() {
            let glyph_id = ttf_parser::GlyphId(index);
            if let Some(name) = face.glyph_name(glyph_id) {
                glyph_names.insert(name.to_string(), GlyphId(index));
            }
        }
        Ok(Self {
            font,
            source_data: Arc::from(data),
            face_index: 0,
            px_scale,
            glyph_names,
            glyph_count: face.number_of_glyphs(),
        })
    }

    pub fn source_data(&self) -> &[u8] {
        self.source_data.as_ref()
    }

    pub fn source_data_arc(&self) -> Arc<[u8]> {
        Arc::clone(&self.source_data)
    }

    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    pub fn render_size(&self) -> f32 {
        self.px_scale.x
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

    pub fn render_glyph_id_to_image(
        &self,
        glyph: GlyphId,
        img: &mut RgbaImage,
        x: i32,
        y: i32,
        color: (u8, u8, u8),
    ) {
        let glyph = glyph.with_scale_and_position(self.px_scale, Point { x: 0.0, y: 0.0 });
        let Some(outline) = self.font.outline_glyph(glyph) else {
            return;
        };
        let bounds = outline.px_bounds();
        let width = bounds.width().ceil() as usize;
        let height = bounds.height().ceil() as usize;
        if width == 0 || height == 0 {
            return;
        }

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
                if coverage <= 0.0 {
                    continue;
                }
                let img_x = offset_x + px as i32;
                let img_y = offset_y + py as i32;
                if img_x < 0
                    || img_x >= img.width() as i32
                    || img_y < 0
                    || img_y >= img.height() as i32
                {
                    continue;
                }
                let pixel = img.get_pixel_mut(img_x as u32, img_y as u32);
                let cov = coverage.clamp(0.0, 1.0);
                pixel[0] = (color.0 as f32 * cov + pixel[0] as f32 * (1.0 - cov)).round() as u8;
                pixel[1] = (color.1 as f32 * cov + pixel[1] as f32 * (1.0 - cov)).round() as u8;
                pixel[2] = (color.2 as f32 * cov + pixel[2] as f32 * (1.0 - cov)).round() as u8;
                pixel[3] = 255;
            }
        }
    }
}

pub(crate) fn validate_settings(config: &FontConfig) -> Result<()> {
    if !config.size.is_finite() || config.size <= 0.0 {
        anyhow::bail!("Font size must be a positive finite number");
    }
    for tag in config.font_feature_settings.keys() {
        validate_opentype_tag(tag, "OpenType feature", "liga")?;
    }
    for tag in config.font_variation_settings.keys() {
        validate_opentype_tag(tag, "variable-font axis", "wght")?;
    }
    for (tag, value) in &config.font_feature_settings {
        if u16::try_from(*value).is_err() {
            anyhow::bail!("OpenType feature {tag:?} value {value} exceeds the supported u16 range");
        }
    }
    for (tag, value) in &config.font_variation_settings {
        if !value.is_finite() {
            anyhow::bail!("Variable font axis {tag:?} value must be a finite number");
        }
    }
    Ok(())
}

fn validate_opentype_tag(tag: &str, kind: &str, example: &str) -> Result<()> {
    if tag.len() != 4
        || !tag
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
    {
        anyhow::bail!(
            "{kind} tag must be exactly 4 printable ASCII characters (for example {example:?}): {tag:?}"
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/font_loader.rs"]
mod tests;
