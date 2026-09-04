use crate::font_loader::FontLoader;
use crate::scene::{Color, FontConfig, TextAlign, TextComponent};
use anyhow::{Context, Result};
use image::RgbaImage;
use parley::fontique::{Blob, Collection, CollectionOptions, FontInfoOverride, SourceCache};
use parley::setting::Tag;
use parley::style::{
    FontFamily, FontFamilyName, FontFeature, FontFeatures, FontVariation, FontVariations,
    StyleProperty,
};
use parley::{
    Alignment, AlignmentOptions, FontContext, Layout, LayoutContext, PositionedLayoutItem,
    TextWrapMode,
};
use resvg::{tiny_skia, usvg};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use swash::scale::image::{Content, Image as SwashImage};
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Format, Vector};
use swash::{CacheKey, FontRef};

const TYPOGRAPHY_SUPERSAMPLE: i32 = 4;

const SWASH_SOURCES: [Source; 3] = [
    Source::ColorOutline(0),
    Source::ColorBitmap(StrikeWith::BestFit),
    Source::Outline,
];

#[derive(Clone)]
struct ComponentFontStack {
    families: Vec<FontFamilyName<'static>>,
}

#[derive(Clone, Copy)]
struct SwashFaceKey {
    offset: u32,
    key: CacheKey,
}

#[derive(Clone, Copy, Default)]
struct FontCapabilities {
    has_svg: bool,
    colr_version: Option<u16>,
}

#[derive(Clone, Copy)]
struct GlyphPaintRequest<'a> {
    data: &'a [u8],
    face_index: u32,
    glyph_id: u32,
    size: f32,
    x: f32,
    baseline_y: f32,
    color: [u8; 4],
}

struct MonochromeRaster {
    glyph: SwashImage,
    origin_x: i32,
    baseline_y: i32,
}

#[derive(Default)]
struct CoverageMask {
    glyphs: Vec<MonochromeRaster>,
}

impl CoverageMask {
    fn new() -> Self {
        Self::default()
    }

    fn accumulate(&mut self, glyph: SwashImage, origin_x: i32, baseline_y: i32) -> bool {
        if matches!(glyph.content, Content::Color) {
            return false;
        }
        self.glyphs.push(MonochromeRaster {
            glyph,
            origin_x,
            baseline_y,
        });
        true
    }

    /// Merge monochrome glyph geometry at supersampled resolution and only then
    /// downsample to the target frame. This avoids two target-resolution AA masks
    /// darkening each other at Arabic/Indic joins and produces smoother edges.
    fn flush(&mut self, foreground: [u8; 4], target: &mut RgbaImage) {
        if self.glyphs.is_empty() {
            return;
        }

        let scale = TYPOGRAPHY_SUPERSAMPLE;
        let mut high_left = i32::MAX;
        let mut high_top = i32::MAX;
        let mut high_right = i32::MIN;
        let mut high_bottom = i32::MIN;
        for raster in &self.glyphs {
            let left = raster.origin_x + raster.glyph.placement.left;
            let top = raster.baseline_y - raster.glyph.placement.top;
            high_left = high_left.min(left);
            high_top = high_top.min(top);
            high_right = high_right.max(left + raster.glyph.placement.width as i32);
            high_bottom = high_bottom.max(top + raster.glyph.placement.height as i32);
        }

        let base_left = floor_div(high_left, scale).max(0);
        let base_top = floor_div(high_top, scale).max(0);
        let base_right = ceil_div(high_right, scale).min(target.width() as i32);
        let base_bottom = ceil_div(high_bottom, scale).min(target.height() as i32);
        if base_left >= base_right || base_top >= base_bottom {
            self.glyphs.clear();
            return;
        }

        let aligned_high_left = base_left * scale;
        let aligned_high_top = base_top * scale;
        let high_width = (base_right - base_left) as usize * scale as usize;
        let high_height = (base_bottom - base_top) as usize * scale as usize;
        let Some(mask_len) = high_width.checked_mul(high_height) else {
            self.glyphs.clear();
            return;
        };
        let mut mask = vec![0u8; mask_len];

        for raster in self.glyphs.drain(..) {
            let glyph_left = raster.origin_x + raster.glyph.placement.left;
            let glyph_top = raster.baseline_y - raster.glyph.placement.top;
            let width = raster.glyph.placement.width as usize;
            let height = raster.glyph.placement.height as usize;
            for py in 0..height {
                let y = glyph_top + py as i32;
                if y < aligned_high_top || y >= aligned_high_top + high_height as i32 {
                    continue;
                }
                for px in 0..width {
                    let x = glyph_left + px as i32;
                    if x < aligned_high_left || x >= aligned_high_left + high_width as i32 {
                        continue;
                    }
                    let pixel_index = py * width + px;
                    let coverage = monochrome_coverage(&raster.glyph, pixel_index);
                    if coverage == 0 {
                        continue;
                    }
                    let local_x = (x - aligned_high_left) as usize;
                    let local_y = (y - aligned_high_top) as usize;
                    let index = local_y * high_width + local_x;
                    mask[index] = merge_coverage(mask[index], coverage);
                }
            }
        }

        let samples = (scale * scale) as u32;
        for base_y in base_top..base_bottom {
            let local_base_y = (base_y - base_top) as usize;
            for base_x in base_left..base_right {
                let local_base_x = (base_x - base_left) as usize;
                let mut sum = 0u32;
                for sy in 0..scale as usize {
                    let row = (local_base_y * scale as usize + sy) * high_width;
                    let col = local_base_x * scale as usize;
                    for sx in 0..scale as usize {
                        sum += u32::from(mask[row + col + sx]);
                    }
                }
                let alpha = ((sum + samples / 2) / samples) as u8;
                if alpha == 0 {
                    continue;
                }
                blend_pixel(
                    target,
                    base_x,
                    base_y,
                    [foreground[0], foreground[1], foreground[2], alpha],
                    foreground[3],
                );
            }
        }
    }
}

fn monochrome_coverage(glyph: &SwashImage, pixel_index: usize) -> u8 {
    match glyph.content {
        Content::Mask => glyph.data.get(pixel_index).copied().unwrap_or(0),
        Content::SubpixelMask => {
            let offset = pixel_index * 4;
            glyph
                .data
                .get(offset..offset + 4)
                .map(|channels| channels[0].max(channels[1]).max(channels[2]))
                .unwrap_or(0)
        }
        Content::Color => 0,
    }
}

fn floor_div(value: i32, divisor: i32) -> i32 {
    value.div_euclid(divisor)
}

fn ceil_div(value: i32, divisor: i32) -> i32 {
    debug_assert!(divisor > 0);
    let quotient = value.div_euclid(divisor);
    if value.rem_euclid(divisor) == 0 {
        quotient
    } else {
        quotient + 1
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TextPlacement {
    pub origin_x: f32,
    pub last_baseline_y: f32,
    pub canvas_width: u32,
}

#[derive(Clone)]
pub(crate) struct CenteredUnicodeRequest<'a> {
    pub component_id: &'a str,
    pub config: &'a FontConfig,
    pub font_size: f32,
    pub text: &'a str,
    pub font_index: Option<usize>,
    pub color: Color,
    pub center: (f32, f32),
}

pub(crate) struct PreparedUnicodeLayout {
    layout: Layout<[u8; 4]>,
    config: FontConfig,
}

impl PreparedUnicodeLayout {
    pub(crate) fn width(&self) -> f32 {
        self.layout.width()
    }
}

#[derive(Clone)]
pub(crate) struct ExplicitGlyphRequest<'a> {
    pub font: &'a FontLoader,
    pub glyph_id: u16,
    pub config: &'a FontConfig,
    pub position: (f32, f32),
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ExplicitGlyphLayoutMetrics {
    pub advance: f32,
    pub outline_center_x: Option<f32>,
    pub ascent: f32,
    pub descent: f32,
}

/// Per-render-worker typography state.
///
/// Parley and Swash both keep useful scratch caches. Keeping one instance per
/// frame worker preserves parallelism without a global typography mutex.
pub(crate) struct TypographyRenderer {
    font_context: FontContext,
    layout_context: LayoutContext<[u8; 4]>,
    scale_context: ScaleContext,
    font_stacks: HashMap<String, ComponentFontStack>,
    capabilities: HashMap<(usize, usize, u32), FontCapabilities>,
    swash_faces: HashMap<(usize, usize, u32), SwashFaceKey>,
}

impl TypographyRenderer {
    pub(crate) fn new(component_fonts: &HashMap<String, Vec<Arc<FontLoader>>>) -> Result<Self> {
        let collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: false,
        });
        let mut font_context = FontContext {
            collection,
            source_cache: SourceCache::default(),
        };
        let mut font_stacks = HashMap::new();
        let mut capabilities = HashMap::new();

        for (stack_index, (component_id, fonts)) in component_fonts.iter().enumerate() {
            let mut families = Vec::with_capacity(fonts.len());
            for (font_index, loader) in fonts.iter().enumerate() {
                // Override family names so the component's explicit font order
                // is the only fallback list Parley can select from.
                let family = format!("__ufm_stack_{stack_index}_{font_index}");
                // BlobStorage supports Arc<[u8]>, so workers share the immutable
                // font bytes retained by FontLoader instead of cloning large fonts.
                let blob = Blob::new(Arc::new(loader.source_data_arc()));
                let registered = font_context.collection.register_fonts(
                    blob,
                    Some(FontInfoOverride {
                        family_name: Some(&family),
                        ..FontInfoOverride::default()
                    }),
                );
                if registered.is_empty() {
                    anyhow::bail!(
                        "Parley could not register configured font {} for component {:?}",
                        font_index,
                        component_id
                    );
                }
                capabilities.insert(
                    font_identity(loader.source_data(), loader.face_index()),
                    inspect_font_capabilities(loader.source_data(), loader.face_index()),
                );
                families.push(FontFamilyName::named(&family).into_owned());
            }
            font_stacks.insert(component_id.clone(), ComponentFontStack { families });
        }

        Ok(Self {
            font_context,
            layout_context: LayoutContext::new(),
            scale_context: ScaleContext::new(),
            font_stacks,
            capabilities,
            swash_faces: HashMap::new(),
        })
    }

    pub(crate) fn render_text(
        &mut self,
        component: &TextComponent,
        text: &str,
        color: Color,
        placement: TextPlacement,
        image: &mut RgbaImage,
    ) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let stack = self.font_stacks.get(&component.id).with_context(|| {
            format!(
                "No registered font stack for text component {:?}",
                component.id
            )
        })?;
        if stack.families.is_empty() {
            return Ok(());
        }

        let font_family = FontFamily::List(Cow::Owned(stack.families.clone()));
        let features = feature_settings(&component.font)?;
        let variations = variation_settings_for_size(&component.font, component.font.size)?;
        let brush = [color.r, color.g, color.b, color.a];

        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(font_family));
        builder.push_default(StyleProperty::FontSize(component.font.size));
        builder.push_default(StyleProperty::Brush(brush));
        if !features.is_empty() {
            builder.push_default(StyleProperty::FontFeatures(FontFeatures::List(Cow::Owned(
                features,
            ))));
        }
        if !variations.is_empty() {
            builder.push_default(StyleProperty::FontVariations(FontVariations::List(
                Cow::Owned(variations),
            )));
        }
        if !component.wrap {
            builder.push_default(StyleProperty::TextWrapMode(TextWrapMode::NoWrap));
        }

        let mut layout: Layout<[u8; 4]> = builder.build(text);
        let max_width = component_max_width(component, placement.canvas_width, placement.origin_x);
        layout.break_all_lines(Some(max_width));
        layout.align(
            match component.align {
                TextAlign::Left => Alignment::Left,
                TextAlign::Center => Alignment::Center,
                TextAlign::Right => Alignment::Right,
            },
            AlignmentOptions {
                align_when_overflowing: true,
            },
        );

        let Some(last_line) = layout.lines().last() else {
            return Ok(());
        };
        let translation_y = placement.last_baseline_y - last_line.metrics().baseline;

        let mut coverage = CoverageMask::new();
        let mut coverage_color: Option<[u8; 4]> = None;
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let run = glyph_run.run();
                let font = run.font();
                let coords = run.normalized_coords();
                let size = run.font_size();
                let variations = effective_variations_for_size(&component.font, size);
                let run_color = glyph_run.style().brush;
                if coverage_color.is_some_and(|previous| previous != run_color) {
                    coverage.flush(coverage_color.unwrap_or(run_color), image);
                }
                coverage_color = Some(run_color);
                for glyph in glyph_run.positioned_glyphs() {
                    let x = placement.origin_x + glyph.x;
                    let y = translation_y + glyph.y;
                    self.paint_shaped_glyph(
                        GlyphPaintRequest {
                            data: font.data.data(),
                            face_index: font.index,
                            glyph_id: glyph.id,
                            size,
                            x,
                            baseline_y: y,
                            color: run_color,
                        },
                        coords,
                        &variations,
                        &mut coverage,
                        image,
                    )?;
                }
            }
        }
        if let Some(color) = coverage_color {
            coverage.flush(color, image);
        }
        Ok(())
    }

    /// Shape and paint a Unicode-only GlyphComponent sequence around a visual
    /// center. This is used for ligatures, combining sequences, variation
    /// selectors and emoji ZWJ sequences while explicit /name and #index
    /// selectors continue to bypass shaping.
    fn build_centered_unicode_layout(
        &mut self,
        request: &CenteredUnicodeRequest<'_>,
    ) -> Result<Option<Layout<[u8; 4]>>> {
        let stack = self
            .font_stacks
            .get(request.component_id)
            .with_context(|| {
                format!(
                    "No registered font stack for glyph component {:?}",
                    request.component_id
                )
            })?;
        if stack.families.is_empty() {
            return Ok(None);
        }

        let families = if let Some(font_index) = request.font_index {
            vec![stack.families[font_index.min(stack.families.len() - 1)].clone()]
        } else {
            stack.families.clone()
        };
        let font_family = FontFamily::List(Cow::Owned(families));
        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, request.text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(font_family));
        builder.push_default(StyleProperty::FontSize(request.font_size));
        builder.push_default(StyleProperty::Brush([
            request.color.r,
            request.color.g,
            request.color.b,
            request.color.a,
        ]));
        let features = feature_settings(request.config)?;
        if !features.is_empty() {
            builder.push_default(StyleProperty::FontFeatures(FontFeatures::List(Cow::Owned(
                features,
            ))));
        }
        let variations = variation_settings_for_size(request.config, request.config.size)?;
        if !variations.is_empty() {
            builder.push_default(StyleProperty::FontVariations(FontVariations::List(
                Cow::Owned(variations),
            )));
        }
        builder.push_default(StyleProperty::TextWrapMode(TextWrapMode::NoWrap));

        let mut layout: Layout<[u8; 4]> = builder.build(request.text);
        layout.break_all_lines(None);
        layout.align(Alignment::Left, AlignmentOptions::default());
        Ok(Some(layout))
    }

    pub(crate) fn prepare_centered_unicode_sequence(
        &mut self,
        request: &CenteredUnicodeRequest<'_>,
    ) -> Result<Option<PreparedUnicodeLayout>> {
        let Some(layout) = self.build_centered_unicode_layout(request)? else {
            return Ok(None);
        };
        Ok(Some(PreparedUnicodeLayout {
            layout,
            config: request.config.clone(),
        }))
    }

    pub(crate) fn render_prepared_unicode_sequence(
        &mut self,
        prepared: &PreparedUnicodeLayout,
        center: (f32, f32),
        image: &mut RgbaImage,
    ) -> Result<()> {
        let layout = &prepared.layout;
        if layout.is_empty() {
            return Ok(());
        }

        let first = layout
            .lines()
            .next()
            .context("Parley layout has no first line")?;
        let last = layout
            .lines()
            .last()
            .context("Parley layout has no last line")?;
        let top = first.metrics().block_min_coord;
        let bottom = last.metrics().block_max_coord;
        let translate_x = center.0 - layout.width() / 2.0;
        let translate_y = center.1 - (top + bottom) / 2.0;
        let mut coverage = CoverageMask::new();
        let mut coverage_color: Option<[u8; 4]> = None;

        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let run = glyph_run.run();
                let font = run.font();
                let coords = run.normalized_coords();
                let variations =
                    effective_variations_for_size(&prepared.config, prepared.config.size);
                let run_color = glyph_run.style().brush;
                if coverage_color.is_some_and(|previous| previous != run_color) {
                    coverage.flush(coverage_color.unwrap_or(run_color), image);
                }
                coverage_color = Some(run_color);
                for glyph in glyph_run.positioned_glyphs() {
                    self.paint_shaped_glyph(
                        GlyphPaintRequest {
                            data: font.data.data(),
                            face_index: font.index,
                            glyph_id: glyph.id,
                            size: run.font_size(),
                            x: translate_x + glyph.x,
                            baseline_y: translate_y + glyph.y,
                            color: run_color,
                        },
                        coords,
                        &variations,
                        &mut coverage,
                        image,
                    )?;
                }
            }
        }
        if let Some(color) = coverage_color {
            coverage.flush(color, image);
        }
        Ok(())
    }

    pub(crate) fn render_centered_unicode_sequence(
        &mut self,
        request: CenteredUnicodeRequest<'_>,
        image: &mut RgbaImage,
    ) -> Result<()> {
        if request.text.is_empty() {
            return Ok(());
        }
        let Some(prepared) = self.prepare_centered_unicode_sequence(&request)? else {
            return Ok(());
        };
        self.render_prepared_unicode_sequence(&prepared, request.center, image)
    }

    /// Return layout metrics for an exact glyph selection at the configured
    /// variable-font instance. These metrics intentionally bypass GSUB/GPOS but
    /// still honor HVAR/MVAR/gvar deltas so exact-glyph painting and placement
    /// describe the same font instance.
    pub(crate) fn explicit_glyph_layout_metrics(
        &self,
        font: &FontLoader,
        glyph_id: u16,
        config: &FontConfig,
    ) -> Result<ExplicitGlyphLayoutMetrics> {
        let variations = effective_variations_for_size(config, config.size);
        let face = configured_ttf_face(font.source_data(), font.face_index(), &variations)?;
        let units_per_em = f32::from(face.units_per_em());
        let scale = font.render_size() / units_per_em;
        let glyph_id = ttf_parser::GlyphId(glyph_id);
        let advance = face
            .glyph_hor_advance(glyph_id)
            .map(f32::from)
            .unwrap_or(0.0)
            * scale;
        let outline_center_x = face
            .glyph_bounding_box(glyph_id)
            .map(|bbox| (f32::from(bbox.x_min) + f32::from(bbox.x_max)) * 0.5 * scale);
        Ok(ExplicitGlyphLayoutMetrics {
            advance,
            outline_center_x,
            ascent: f32::from(face.ascender()) * scale,
            descent: f32::from(face.descender()) * scale,
        })
    }

    /// Paint an explicitly selected glyph ID without any Unicode remapping or
    /// GSUB substitution. Variable axes are still honored by the rasterizer.
    pub(crate) fn render_explicit_glyph(
        &mut self,
        explicit: ExplicitGlyphRequest<'_>,
        image: &mut RgbaImage,
    ) -> Result<bool> {
        let request = GlyphPaintRequest {
            data: explicit.font.source_data(),
            face_index: explicit.font.face_index(),
            glyph_id: u32::from(explicit.glyph_id),
            size: explicit.font.render_size(),
            x: explicit.position.0,
            baseline_y: explicit.position.1,
            color: [
                explicit.color.r,
                explicit.color.g,
                explicit.color.b,
                explicit.color.a,
            ],
        };
        let variations = effective_variations_for_size(explicit.config, explicit.config.size);
        let capabilities = self.font_capabilities(request.data, request.face_index);
        if capabilities.has_svg && self.paint_svg_glyph(request, image)? {
            return Ok(true);
        }
        if capabilities
            .colr_version
            .is_some_and(|version| version >= 1)
            && self.paint_colr_glyph(request, &variations, image)?
        {
            return Ok(true);
        }
        self.paint_swash_with_variations(request, &variations, image)
    }

    fn paint_shaped_glyph(
        &mut self,
        request: GlyphPaintRequest<'_>,
        normalized_coords: &[i16],
        variations: &BTreeMap<String, f32>,
        coverage: &mut CoverageMask,
        image: &mut RgbaImage,
    ) -> Result<bool> {
        let capabilities = self.font_capabilities(request.data, request.face_index);
        if capabilities.has_svg {
            coverage.flush(request.color, image);
            if self.paint_svg_glyph(request, image)? {
                return Ok(true);
            }
        }
        if capabilities
            .colr_version
            .is_some_and(|version| version >= 1)
        {
            coverage.flush(request.color, image);
        }
        if capabilities
            .colr_version
            .is_some_and(|version| version >= 1)
            && self.paint_colr_glyph(request, variations, image)?
        {
            return Ok(true);
        }

        let Ok(glyph_id) = u16::try_from(request.glyph_id) else {
            return Ok(false);
        };
        let font = self.swash_font_ref(request.data, request.face_index)?;
        let scale = TYPOGRAPHY_SUPERSAMPLE as f32;
        let high_x = request.x * scale;
        let high_baseline_y = request.baseline_y * scale;
        let mut scaler = self
            .scale_context
            .builder(font)
            .size(request.size * scale)
            .hint(false)
            .normalized_coords(normalized_coords.iter().copied())
            .build();
        let mut render = Render::new(&SWASH_SOURCES);
        render
            .format(Format::Alpha)
            .offset(Vector::new(high_x.fract(), high_baseline_y.fract()))
            .default_color([request.color[0], request.color[1], request.color[2], 255]);
        let Some(rendered) = render.render(&mut scaler, glyph_id) else {
            return Ok(false);
        };
        let origin_x = high_x.floor() as i32;
        let baseline_y = high_baseline_y.floor() as i32;
        if !matches!(rendered.content, Content::Color) {
            coverage.accumulate(rendered, origin_x, baseline_y);
            return Ok(true);
        }

        coverage.flush(request.color, image);
        composite_supersampled_swash_image(rendered, origin_x, baseline_y, request.color, image);
        Ok(true)
    }

    fn paint_swash_with_variations(
        &mut self,
        request: GlyphPaintRequest<'_>,
        variations: &BTreeMap<String, f32>,
        image: &mut RgbaImage,
    ) -> Result<bool> {
        let Ok(glyph_id) = u16::try_from(request.glyph_id) else {
            return Ok(false);
        };
        let font = self.swash_font_ref(request.data, request.face_index)?;
        let variation_pairs: Vec<(&str, f32)> = variations
            .iter()
            .map(|(tag, value)| (tag.as_str(), *value))
            .collect();
        let scale = TYPOGRAPHY_SUPERSAMPLE as f32;
        let high_x = request.x * scale;
        let high_baseline_y = request.baseline_y * scale;
        let mut builder = self
            .scale_context
            .builder(font)
            .size(request.size * scale)
            .hint(false);
        if !variation_pairs.is_empty() {
            builder = builder.variations(variation_pairs.iter().copied());
        }
        let mut scaler = builder.build();
        let mut render = Render::new(&SWASH_SOURCES);
        render
            .format(Format::Alpha)
            .offset(Vector::new(high_x.fract(), high_baseline_y.fract()))
            .default_color([request.color[0], request.color[1], request.color[2], 255]);
        let Some(rendered) = render.render(&mut scaler, glyph_id) else {
            return Ok(false);
        };
        composite_supersampled_swash_image(
            rendered,
            high_x.floor() as i32,
            high_baseline_y.floor() as i32,
            request.color,
            image,
        );
        Ok(true)
    }

    fn font_capabilities(&mut self, data: &[u8], face_index: u32) -> FontCapabilities {
        let identity = font_identity(data, face_index);
        if let Some(capabilities) = self.capabilities.get(&identity).copied() {
            return capabilities;
        }
        let capabilities = inspect_font_capabilities(data, face_index);
        self.capabilities.insert(identity, capabilities);
        capabilities
    }

    /// Direct COLR/CPAL path. Glyph selection and shaping have already happened;
    /// vendored usvg only translates the selected COLR paint graph to a tree,
    /// and resvg paints that tree. This covers COLR v1 constructs that Swash's
    /// layered ColorOutline backend does not currently model.
    fn paint_colr_glyph(
        &self,
        request: GlyphPaintRequest<'_>,
        variations: &BTreeMap<String, f32>,
        target: &mut RgbaImage,
    ) -> Result<bool> {
        let Ok(glyph_id) = u16::try_from(request.glyph_id) else {
            return Ok(false);
        };
        let variation_pairs: Vec<(&str, f32)> = variations
            .iter()
            .map(|(tag, value)| (tag.as_str(), *value))
            .collect();
        let Some(tree) = usvg::colr_glyph_to_tree(
            request.data,
            request.face_index,
            glyph_id,
            0,
            [request.color[0], request.color[1], request.color[2], 255],
            &variation_pairs,
        ) else {
            return Ok(false);
        };

        let face = match ttf_parser::Face::parse(request.data, request.face_index) {
            Ok(face) => face,
            Err(_) => return Ok(false),
        };
        let units_per_em = f32::from(face.units_per_em());
        if units_per_em <= 0.0 || !request.size.is_finite() || request.size <= 0.0 {
            return Ok(false);
        }

        let bbox = tree.root().abs_layer_bounding_box();
        let supersample = TYPOGRAPHY_SUPERSAMPLE as f32;
        let scale = request.size * supersample / units_per_em;
        let high_x = request.x * supersample;
        let high_baseline_y = request.baseline_y * supersample;
        let min_x = bbox.x() * scale + high_x.fract();
        let max_x = (bbox.x() + bbox.width()) * scale + high_x.fract();
        // COLR outlines use font coordinates (Y up), while the target image is Y down.
        let min_y = -(bbox.y() + bbox.height()) * scale + high_baseline_y.fract();
        let max_y = -bbox.y() * scale + high_baseline_y.fract();
        let left = min_x.floor();
        let top = min_y.floor();
        let right = max_x.ceil();
        let bottom = max_y.ceil();
        let pixel_width = (right - left).max(1.0) as u32;
        let pixel_height = (bottom - top).max(1.0) as u32;
        let Some(mut pixmap) = tiny_skia::Pixmap::new(pixel_width, pixel_height) else {
            return Ok(false);
        };

        let transform = tiny_skia::Transform::from_row(scale, 0.0, 0.0, -scale, -left, -top);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let Some(layer) =
            RgbaImage::from_raw(pixel_width, pixel_height, pixmap.take_demultiplied())
        else {
            return Ok(false);
        };
        composite_supersampled_rgba_layer(
            &layer,
            high_x.floor() as i32 + left as i32,
            high_baseline_y.floor() as i32 + top as i32,
            request.color[3],
            target,
        );
        Ok(true)
    }

    fn swash_font_ref<'a>(&mut self, data: &'a [u8], face_index: u32) -> Result<FontRef<'a>> {
        let identity = font_identity(data, face_index);
        let cached = if let Some(cached) = self.swash_faces.get(&identity).copied() {
            cached
        } else {
            let parsed = FontRef::from_index(data, face_index as usize)
                .context("Swash could not parse selected font face")?;
            let cached = SwashFaceKey {
                offset: parsed.offset,
                key: parsed.key,
            };
            self.swash_faces.insert(identity, cached);
            cached
        };
        Ok(FontRef {
            data,
            offset: cached.offset,
            key: cached.key,
        })
    }

    /// Direct SVG-in-OpenType path. The glyph ID comes from Parley (or an
    /// explicit glyph selector); ttf-parser retrieves that glyph's SVG document
    /// and usvg/resvg only paints the selected SVG node. No cmap fabrication and
    /// no text shaping occurs here.
    fn paint_svg_glyph(
        &self,
        request: GlyphPaintRequest<'_>,
        target: &mut RgbaImage,
    ) -> Result<bool> {
        let Ok(glyph_id_u16) = u16::try_from(request.glyph_id) else {
            return Ok(false);
        };
        let face = match ttf_parser::Face::parse(request.data, request.face_index) {
            Ok(face) => face,
            Err(_) => return Ok(false),
        };
        let Some(document) = face.glyph_svg_image(ttf_parser::GlyphId(glyph_id_u16)) else {
            return Ok(false);
        };

        let units_per_em = f32::from(face.units_per_em());
        if units_per_em <= 0.0 || !request.size.is_finite() || request.size <= 0.0 {
            return Ok(false);
        }
        let Some(em_square) = usvg::Size::from_wh(units_per_em, units_per_em) else {
            return Ok(false);
        };

        // OpenType SVG defines an initial viewport equal to the em square, with
        // SVG's y-down coordinate system and baseline at y=0. The initial
        // viewport is not clipped. Supplying currentColor through the CSS
        // cascade preserves authored colors while honoring the text foreground.
        let options = usvg::Options {
            default_size: em_square,
            forced_size: Some(em_square),
            style_sheet: Some(format!(
                "svg {{ color: rgb({}, {}, {}); overflow: visible !important; }}",
                request.color[0], request.color[1], request.color[2]
            )),
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(document.data, &options)
            .context("Failed to parse SVG-in-OpenType glyph document")?;
        let owned_root;
        let node = if document.start_glyph_id == document.end_glyph_id {
            owned_root = usvg::Node::Group(Box::new(tree.root().clone()));
            &owned_root
        } else {
            let node_id = format!("glyph{}", request.glyph_id);
            let Some(node) = tree.node_by_id(&node_id) else {
                // A shared SVG document can describe multiple glyphs. Rendering
                // the whole record would accidentally paint neighboring glyphs.
                return Ok(false);
            };
            node
        };
        let Some(bbox) = node.abs_layer_bounding_box() else {
            return Ok(false);
        };

        let supersample = TYPOGRAPHY_SUPERSAMPLE as f32;
        let scale = request.size * supersample / units_per_em;
        let high_x = request.x * supersample;
        let high_baseline_y = request.baseline_y * supersample;
        let min_x = bbox.x() * scale + high_x.fract();
        let max_x = (bbox.x() + bbox.width()) * scale + high_x.fract();
        let min_y = bbox.y() * scale + high_baseline_y.fract();
        let max_y = (bbox.y() + bbox.height()) * scale + high_baseline_y.fract();
        let left = min_x.floor();
        let top = min_y.floor();
        let right = max_x.ceil();
        let bottom = max_y.ceil();
        let pixel_width = (right - left).max(1.0) as u32;
        let pixel_height = (bottom - top).max(1.0) as u32;
        let Some(mut pixmap) = tiny_skia::Pixmap::new(pixel_width, pixel_height) else {
            return Ok(false);
        };
        // `abs_layer_bounding_box()` is expressed in the parsed SVG tree's
        // absolute canvas coordinates. `resvg::render_node()`, however, renders
        // a selected node as a new root and therefore does not replay transforms
        // from ancestors that are no longer part of that root. That distinction
        // matters for shared OpenType-SVG documents with an authored viewBox or
        // transformed ancestor groups: metrics/ink bounds are absolute, while a
        // raw `node_by_id()` render would otherwise use only the selected node's
        // local transform and visibly drift away from its OpenType glyph origin.
        //
        // resvg also pre-translates the selected node by `-bbox.origin` inside
        // `render_node()`. Build the caller transform as
        //
        //   pixel_map * missing_ancestors * +bbox.origin
        //
        // so resvg's internal `-bbox.origin` cancels the last term and the node
        // is finally painted in the same absolute SVG coordinates used by bbox.
        let transform = svg_selected_node_render_transform(
            node,
            bbox,
            scale,
            (high_x.fract(), high_baseline_y.fract()),
            (left, top),
        );
        if resvg::render_node(node, transform, &mut pixmap.as_mut()).is_none() {
            return Ok(false);
        }
        let Some(layer) =
            RgbaImage::from_raw(pixel_width, pixel_height, pixmap.take_demultiplied())
        else {
            return Ok(false);
        };
        composite_supersampled_rgba_layer(
            &layer,
            high_x.floor() as i32 + left as i32,
            high_baseline_y.floor() as i32 + top as i32,
            request.color[3],
            target,
        );
        Ok(true)
    }
}

fn svg_selected_node_render_transform(
    node: &usvg::Node,
    bbox: usvg::NonZeroRect,
    scale: f32,
    subpixel_origin: (f32, f32),
    pixel_origin: (f32, f32),
) -> tiny_skia::Transform {
    let pixel_map = tiny_skia::Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        subpixel_origin.0 - pixel_origin.0,
        subpixel_origin.1 - pixel_origin.1,
    );
    pixel_map
        .pre_concat(svg_selected_node_parent_transform(node))
        .pre_translate(bbox.x(), bbox.y())
}

fn svg_selected_node_parent_transform(node: &usvg::Node) -> tiny_skia::Transform {
    match node {
        // resvg::render_node() calls render_group(), which applies the selected
        // group's own relative transform. Strip that local transform from the
        // usvg absolute transform so only the missing ancestors are restored.
        usvg::Node::Group(group) => group
            .transform()
            .invert()
            .map(|inverse| group.abs_transform().pre_concat(inverse))
            .unwrap_or_else(|| group.abs_transform()),
        // Paths/images do not get their usvg `abs_transform()` replayed by
        // resvg::render_node(); it must therefore be supplied as the root.
        usvg::Node::Path(path) => path.abs_transform(),
        usvg::Node::Image(image) => image.abs_transform(),
        // OpenType-SVG forbids <text> for interoperable glyph rendering, but
        // keeping the absolute transform here makes the helper total and avoids
        // surprising geometry if usvg preserved one in a malformed font.
        usvg::Node::Text(text) => text.abs_transform(),
    }
}

fn configured_ttf_face<'a>(
    data: &'a [u8],
    face_index: u32,
    variations: &BTreeMap<String, f32>,
) -> Result<ttf_parser::Face<'a>> {
    let mut face = ttf_parser::Face::parse(data, face_index)
        .context("ttf-parser could not parse selected font face")?;
    for (tag, value) in variations {
        let bytes = tag.as_bytes();
        if bytes.len() != 4 {
            continue;
        }
        let tag = ttf_parser::Tag::from_bytes(&[bytes[0], bytes[1], bytes[2], bytes[3]]);
        let _ = face.set_variation(tag, *value);
    }
    Ok(face)
}

fn inspect_font_capabilities(data: &[u8], face_index: u32) -> FontCapabilities {
    let Ok(face) = ttf_parser::Face::parse(data, face_index) else {
        return FontCapabilities::default();
    };
    let colr_version = face
        .raw_face()
        .table(ttf_parser::Tag::from_bytes(b"COLR"))
        .and_then(|table| table.get(..2))
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]));
    FontCapabilities {
        has_svg: face
            .raw_face()
            .table(ttf_parser::Tag::from_bytes(b"SVG "))
            .is_some(),
        colr_version,
    }
}

fn component_max_width(component: &TextComponent, canvas_width: u32, origin_x: f32) -> f32 {
    let width = if component.max_width > 0.0 {
        component.max_width as f32 * canvas_width as f32
    } else {
        canvas_width as f32 - origin_x
    };
    width.max(1.0)
}

fn parley_tag(tag: &str) -> Result<Tag> {
    Tag::parse(tag).with_context(|| format!("Invalid OpenType tag {tag:?}"))
}

fn feature_settings(config: &FontConfig) -> Result<Vec<FontFeature>> {
    config
        .font_feature_settings
        .iter()
        .map(|(tag, value)| {
            let value = u16::try_from(*value).with_context(|| {
                format!("OpenType feature {tag:?} value {value} exceeds Parley's u16 range")
            })?;
            Ok(FontFeature::new(parley_tag(tag)?, value))
        })
        .collect()
}

fn effective_variations_for_size(config: &FontConfig, optical_size: f32) -> BTreeMap<String, f32> {
    let mut settings = config.font_variation_settings.clone();
    if config.font_optical_sizing && !settings.contains_key("opsz") {
        settings.insert("opsz".to_string(), optical_size);
    }
    settings
}

fn variation_settings_for_size(
    config: &FontConfig,
    optical_size: f32,
) -> Result<Vec<FontVariation>> {
    effective_variations_for_size(config, optical_size)
        .into_iter()
        .map(|(tag, value)| Ok(FontVariation::new(parley_tag(&tag)?, value)))
        .collect()
}

fn font_identity(data: &[u8], face_index: u32) -> (usize, usize, u32) {
    (data.as_ptr() as usize, data.len(), face_index)
}

fn composite_supersampled_swash_image(
    glyph: SwashImage,
    origin_x: i32,
    baseline_y: i32,
    foreground: [u8; 4],
    target: &mut RgbaImage,
) {
    if !matches!(glyph.content, Content::Color) {
        let mut coverage = CoverageMask::new();
        coverage.accumulate(glyph, origin_x, baseline_y);
        coverage.flush(foreground, target);
        return;
    }

    let left = origin_x + glyph.placement.left;
    let top = baseline_y - glyph.placement.top;
    let width = glyph.placement.width;
    let height = glyph.placement.height;
    let Some(layer) = RgbaImage::from_raw(width, height, glyph.data) else {
        return;
    };
    composite_supersampled_rgba_layer(&layer, left, top, foreground[3], target);
}

fn composite_supersampled_rgba_layer(
    layer: &RgbaImage,
    high_dest_x: i32,
    high_dest_y: i32,
    opacity: u8,
    target: &mut RgbaImage,
) {
    let scale = TYPOGRAPHY_SUPERSAMPLE;
    let high_right = high_dest_x + layer.width() as i32;
    let high_bottom = high_dest_y + layer.height() as i32;
    let base_left = floor_div(high_dest_x, scale).max(0);
    let base_top = floor_div(high_dest_y, scale).max(0);
    let base_right = ceil_div(high_right, scale).min(target.width() as i32);
    let base_bottom = ceil_div(high_bottom, scale).min(target.height() as i32);
    if base_left >= base_right || base_top >= base_bottom {
        return;
    }

    let samples = (scale * scale) as u64;
    for base_y in base_top..base_bottom {
        for base_x in base_left..base_right {
            let mut sum_alpha = 0u64;
            let mut sum_red_alpha = 0u64;
            let mut sum_green_alpha = 0u64;
            let mut sum_blue_alpha = 0u64;
            for sy in 0..scale {
                let high_y = base_y * scale + sy;
                let source_y = high_y - high_dest_y;
                if source_y < 0 || source_y >= layer.height() as i32 {
                    continue;
                }
                for sx in 0..scale {
                    let high_x = base_x * scale + sx;
                    let source_x = high_x - high_dest_x;
                    if source_x < 0 || source_x >= layer.width() as i32 {
                        continue;
                    }
                    let pixel = layer.get_pixel(source_x as u32, source_y as u32);
                    let alpha = u64::from(pixel[3]);
                    sum_alpha += alpha;
                    sum_red_alpha += u64::from(pixel[0]) * alpha;
                    sum_green_alpha += u64::from(pixel[1]) * alpha;
                    sum_blue_alpha += u64::from(pixel[2]) * alpha;
                }
            }
            if sum_alpha == 0 {
                continue;
            }
            let alpha = ((sum_alpha + samples / 2) / samples).min(255) as u8;
            let red = ((sum_red_alpha + sum_alpha / 2) / sum_alpha).min(255) as u8;
            let green = ((sum_green_alpha + sum_alpha / 2) / sum_alpha).min(255) as u8;
            let blue = ((sum_blue_alpha + sum_alpha / 2) / sum_alpha).min(255) as u8;
            blend_pixel(target, base_x, base_y, [red, green, blue, alpha], opacity);
        }
    }
}

fn merge_coverage(existing: u8, incoming: u8) -> u8 {
    existing.max(incoming)
}

fn blend_pixel(target: &mut RgbaImage, x: i32, y: i32, source: [u8; 4], opacity: u8) {
    if x < 0 || y < 0 || x >= target.width() as i32 || y >= target.height() as i32 {
        return;
    }
    let src_alpha = (u16::from(source[3]) * u16::from(opacity) + 127) / 255;
    if src_alpha == 0 {
        return;
    }
    let src_alpha_f = src_alpha as f32 / 255.0;
    let dst = target.get_pixel_mut(x as u32, y as u32);
    for channel in 0..3 {
        dst[channel] = (source[channel] as f32 * src_alpha_f
            + dst[channel] as f32 * (1.0 - src_alpha_f))
            .round() as u8;
    }
    dst[3] = 255;
}

#[cfg(test)]
#[path = "../tests/unit/typography_renderer.rs"]
mod tests;
