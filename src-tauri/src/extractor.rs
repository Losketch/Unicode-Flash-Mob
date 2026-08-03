//! Shared font extraction for the CLI and Tauri commands.

use crate::json_config::{CharEntry, RenderConfig};
use crate::unicode_data::{self, UnicodeDataManager};
use anyhow::{Context, Result};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

fn resolve_font_paths(font_files: &[PathBuf], cwd: &Path) -> Vec<PathBuf> {
    font_files
        .iter()
        .map(|font_path| {
            let asset_path = crate::json_config::resolve_asset_reference(font_path);
            if asset_path.as_path() != font_path.as_path() || font_path.is_absolute() {
                asset_path
            } else {
                cwd.join(font_path)
            }
        })
        .collect()
}

/// Return every mapped Unicode code point in one font, sorted and deduplicated.
pub fn collect_font_codepoints(font_path: &Path) -> Result<BTreeSet<u32>> {
    let data = std::fs::read(font_path)
        .with_context(|| format!("Failed to read font: {}", font_path.display()))?;
    let face = ttf_parser::Face::parse(&data, 0)
        .with_context(|| format!("Failed to parse font: {}", font_path.display()))?;

    let cmap = face
        .tables()
        .cmap
        .with_context(|| format!("Font has no cmap table: {}", font_path.display()))?;

    let mut codepoints = BTreeSet::new();
    for subtable in cmap.subtables {
        if !subtable.is_unicode() {
            continue;
        }
        subtable.codepoints(|codepoint| {
            if codepoint != 0xFFFD
                && char::from_u32(codepoint).is_some()
                && subtable.glyph_index(codepoint).is_some()
            {
                codepoints.insert(codepoint);
            }
        });
    }
    Ok(codepoints)
}

pub fn extract_to_config(
    font_files: &[PathBuf],
    output_path: &Path,
    video_output: Option<&Path>,
) -> Result<RenderConfig> {
    let cwd = std::env::current_dir()?;
    let mut config = RenderConfig::default();

    let absolute_fonts = resolve_font_paths(font_files, &cwd);
    if absolute_fonts.is_empty() {
        anyhow::bail!("At least one font file is required");
    }
    config.main_text.fonts = absolute_fonts.clone();

    config.output_path = match video_output {
        Some(video) if video.is_absolute() => video.to_path_buf(),
        Some(video) => cwd.join(video),
        None => cwd
            .join(crate::json_config::DEFAULT_OUTPUT_DIR)
            .join(crate::json_config::default_output_filename()),
    };

    config.bottom_text.fonts = crate::json_config::bundled_font_paths("fonts/IBMPlexSans-Bold.ttf");
    config.bottom_text.enabled = !config.bottom_text.fonts.is_empty();

    let (data_path, blocks_path) = unicode_data::find_unicode_data_files();
    let unicode_manager = UnicodeDataManager::load(&data_path, &blocks_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load Unicode data: {}", e);
        UnicodeDataManager::empty()
    });

    let mut seen = BTreeSet::new();
    for font_path in &absolute_fonts {
        seen.extend(collect_font_codepoints(font_path)?);
    }

    // Assign one background color per Unicode block.
    let mut color_mapping: HashMap<String, usize> = HashMap::new();
    let mut color_counter = 0usize;

    for &cp in &seen {
        let description = unicode_manager.get_description(cp);

        let background_color = if !description.is_empty() {
            let key = description.lines().next().unwrap_or("").trim().to_string();
            let color_idx = *color_mapping.entry(key).or_insert_with(|| {
                let idx = color_counter;
                color_counter = (color_counter + 1) % config.background_colors.len();
                idx
            });
            Some(config.background_colors[color_idx].clone())
        } else {
            None
        };

        config.characters.push(CharEntry {
            code_point: format!("U+{:04X}", cp),
            description,
            background_color,
            text_color: None,
            position: None,
            duration_frames: None,
        });
    }

    let output_path_buf = output_path.to_path_buf();
    config
        .to_file(&output_path_buf)
        .with_context(|| format!("Failed to save config file: {}", output_path.display()))?;

    Ok(config)
}
