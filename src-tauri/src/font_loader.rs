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
    // Index within `source_data`. Collection faces are materialized as standalone
    // sfnt data, so their backend-facing index is always zero.
    face_index: u32,
    px_scale: PxScale,
    glyph_names: HashMap<String, GlyphId>,
    glyph_count: u16,
}

impl FontLoader {
    pub fn from_path_with_face_index(
        path: &Path,
        face_index: u32,
        config: &FontConfig,
    ) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read font file: {}", path.display()))?;
        Self::from_bytes_with_config(&data, face_index, config).with_context(|| {
            format!(
                "Failed to load font face {face_index} from {}",
                path.display()
            )
        })
    }

    fn from_bytes_with_config(data: &[u8], face_index: u32, config: &FontConfig) -> Result<Self> {
        validate_settings(config)?;
        let selected_data = select_font_face_data(data, face_index)?;
        let font_size = config.size;
        let font = FontArc::try_from_vec(selected_data.clone())
            .map_err(|_| anyhow::anyhow!("Failed to initialize font backend from selected face"))?;

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
        let face = ttf_parser::Face::parse(&selected_data, 0)
            .map_err(|_| anyhow::anyhow!("Failed to parse font OpenType tables"))?;
        let mut glyph_names = HashMap::new();
        for index in 0..face.number_of_glyphs() {
            let glyph_id = ttf_parser::GlyphId(index);
            if let Some(name) = face.glyph_name(glyph_id) {
                glyph_names.insert(name.to_string(), GlyphId(index));
            }
        }
        let glyph_count = face.number_of_glyphs();
        Ok(Self {
            font,
            source_data: Arc::from(selected_data),
            face_index: 0,
            px_scale,
            glyph_names,
            glyph_count,
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

fn select_font_face_data(data: &[u8], face_index: u32) -> Result<Vec<u8>> {
    match ttf_parser::fonts_in_collection(data) {
        Some(count) => {
            if face_index >= count {
                anyhow::bail!(
                    "Font collection face index {face_index} is out of range (collection has {count} faces)"
                );
            }
            extract_collection_face(data, face_index)
        }
        None if face_index == 0 => Ok(data.to_vec()),
        None => anyhow::bail!(
            "Font face index {face_index} was requested, but the file is not a font collection"
        ),
    }
}

fn read_be_u16(data: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .context("Font collection offset overflow")?;
    let bytes: [u8; 2] = data
        .get(offset..end)
        .context("Font collection is truncated")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Font collection is truncated"))?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("Font collection offset overflow")?;
    let bytes: [u8; 4] = data
        .get(offset..end)
        .context("Font collection is truncated")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Font collection is truncated"))?;
    Ok(u32::from_be_bytes(bytes))
}

/// Materialize one TTC/OTC face as a standalone sfnt. A configured font source
/// represents exactly one face, so every typography backend receives identical
/// single-face bytes instead of independently choosing from the collection.
fn extract_collection_face(data: &[u8], face_index: u32) -> Result<Vec<u8>> {
    let face_index_offset = (face_index as usize)
        .checked_mul(4)
        .context("Font collection face index overflow")?;
    let offset_position = 12usize
        .checked_add(face_index_offset)
        .context("Font collection face index overflow")?;
    let face_offset = read_be_u32(data, offset_position)? as usize;
    let num_tables_offset = face_offset
        .checked_add(4)
        .context("Font collection face offset overflow")?;
    let num_tables = read_be_u16(data, num_tables_offset)? as usize;
    let directory_len = 12usize
        .checked_add(
            num_tables
                .checked_mul(16)
                .context("Font table directory overflow")?,
        )
        .context("Font table directory overflow")?;
    let directory_end = face_offset
        .checked_add(directory_len)
        .context("Font table directory overflow")?;
    data.get(face_offset..directory_end)
        .context("Font collection face directory is truncated")?;

    #[derive(Clone, Copy)]
    struct TableRecord {
        tag: [u8; 4],
        checksum: u32,
        source_offset: usize,
        length: usize,
    }

    let mut records = Vec::with_capacity(num_tables);
    for index in 0..num_tables {
        let record_offset = face_offset
            .checked_add(12)
            .and_then(|offset| offset.checked_add(index * 16))
            .context("Font table record offset overflow")?;
        let tag: [u8; 4] = data[record_offset..record_offset + 4]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid font table tag"))?;
        let checksum = read_be_u32(data, record_offset + 4)?;
        let source_offset = read_be_u32(data, record_offset + 8)? as usize;
        let length = read_be_u32(data, record_offset + 12)? as usize;
        let end = source_offset
            .checked_add(length)
            .context("Font table length overflow")?;
        data.get(source_offset..end).with_context(|| {
            format!(
                "Font table {:?} is truncated",
                String::from_utf8_lossy(&tag)
            )
        })?;
        records.push(TableRecord {
            tag,
            checksum,
            source_offset,
            length,
        });
    }

    let mut target_offset = directory_len;
    let mut output_len = directory_len;
    for record in &records {
        target_offset = (target_offset + 3) & !3;
        output_len = target_offset
            .checked_add(record.length)
            .context("Standalone font size overflow")?;
        target_offset = output_len;
    }
    output_len = (output_len + 3) & !3;

    let mut output = vec![0u8; output_len];
    output[..12].copy_from_slice(&data[face_offset..face_offset + 12]);
    target_offset = directory_len;
    let mut head_offset = None;
    for (index, record) in records.iter().enumerate() {
        target_offset = (target_offset + 3) & !3;
        let directory_offset = 12 + index * 16;
        output[directory_offset..directory_offset + 4].copy_from_slice(&record.tag);
        output[directory_offset + 4..directory_offset + 8]
            .copy_from_slice(&record.checksum.to_be_bytes());
        let table_offset = u32::try_from(target_offset)
            .context("Standalone font table offset exceeds the OpenType u32 range")?;
        let table_length = u32::try_from(record.length)
            .context("Standalone font table length exceeds the OpenType u32 range")?;
        output[directory_offset + 8..directory_offset + 12]
            .copy_from_slice(&table_offset.to_be_bytes());
        output[directory_offset + 12..directory_offset + 16]
            .copy_from_slice(&table_length.to_be_bytes());
        output[target_offset..target_offset + record.length]
            .copy_from_slice(&data[record.source_offset..record.source_offset + record.length]);
        if &record.tag == b"head" {
            head_offset = Some(target_offset);
        }
        target_offset += record.length;
    }

    if let Some(head_offset) = head_offset {
        let adjustment_offset = head_offset
            .checked_add(8)
            .context("head checksum-adjustment offset overflow")?;
        let adjustment_end = adjustment_offset
            .checked_add(4)
            .context("head checksum-adjustment offset overflow")?;
        output
            .get_mut(adjustment_offset..adjustment_end)
            .context("head table is too short for checksumAdjustment")?
            .fill(0);

        let mut sum = 0u32;
        for chunk in output.chunks(4) {
            let mut bytes = [0u8; 4];
            bytes[..chunk.len()].copy_from_slice(chunk);
            sum = sum.wrapping_add(u32::from_be_bytes(bytes));
        }
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(sum);
        output[adjustment_offset..adjustment_end].copy_from_slice(&adjustment.to_be_bytes());
    }

    ttf_parser::Face::parse(&output, 0)
        .map_err(|_| anyhow::anyhow!("Failed to materialize selected font collection face"))?;
    Ok(output)
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

pub(crate) fn validate_opentype_tag(tag: &str, kind: &str, example: &str) -> Result<()> {
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
