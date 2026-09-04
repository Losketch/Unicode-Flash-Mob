use super::{
    feature_settings, merge_coverage, svg_selected_node_render_transform, tiny_skia, usvg,
    variation_settings_for_size, CenteredUnicodeRequest, PositionedLayoutItem, TypographyRenderer,
};
use crate::font_loader::FontLoader;
use crate::scene::{Color, FontConfig, GlyphSelector};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn ceil_div_handles_positive_and_negative_supersampled_bounds() {
    assert_eq!(super::ceil_div(5, 4), 2);
    assert_eq!(super::ceil_div(4, 4), 1);
    assert_eq!(super::ceil_div(0, 4), 0);
    assert_eq!(super::ceil_div(-1, 4), 0);
    assert_eq!(super::ceil_div(-5, 4), -1);
}

#[test]
fn overlapping_monochrome_coverage_does_not_accumulate_opacity() {
    assert_eq!(merge_coverage(128, 128), 128);
    assert_eq!(merge_coverage(64, 192), 192);
}

#[test]
fn opentype_svg_can_force_em_square_viewport_without_replacing_viewbox() {
    let em_square = usvg::Size::from_wh(1000.0, 1000.0).unwrap();
    let options = usvg::Options {
        forced_size: Some(em_square),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 1000 500 500">
            <rect x="0" y="1000" width="500" height="500" fill="red"/>
        </svg>"#,
        &options,
    )
    .unwrap();
    assert_eq!(tree.size().width(), 1000.0);
    assert_eq!(tree.size().height(), 1000.0);
    let bbox = tree.root().abs_layer_bounding_box();
    assert_eq!(bbox.width(), 1000.0);
    assert_eq!(bbox.height(), 1000.0);

    let no_viewbox = usvg::Tree::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg">
            <rect x="0" y="0" width="250" height="500" fill="red"/>
        </svg>"#,
        &options,
    )
    .unwrap();
    assert_eq!(no_viewbox.size().width(), 1000.0);
    assert_eq!(no_viewbox.size().height(), 1000.0);

    // VF-Canto-style documents explicitly author their viewport. Those
    // dimensions must not be replaced by the host em square, otherwise the
    // default xMidYMid/meet transform horizontally centers a narrow SVG and
    // moves its ink outside the glyph's hmtx advance.
    let explicit_viewport = usvg::Tree::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg"
                width="421" height="1179.1" viewBox="0 1179 421 1179">
            <rect x="22" y="1179" width="307" height="100" fill="red"/>
        </svg>"#,
        &options,
    )
    .unwrap();
    assert_eq!(explicit_viewport.size().width(), 421.0);
    assert!((explicit_viewport.size().height() - 1179.1).abs() < 0.01);
    let bbox = explicit_viewport.root().abs_layer_bounding_box();
    assert!((bbox.x() - 22.0).abs() < 0.01, "bbox.x={}", bbox.x());
}

#[test]
fn opentype_svg_explicit_numeric_viewport_is_not_centered_in_em_square() {
    let em_square = usvg::Size::from_wh(1000.0, 1000.0).unwrap();
    let options = usvg::Options {
        forced_size: Some(em_square),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg"
                width="421" height="1179.1" viewBox="0 1179 421 1179">
            <rect x="35" y="1179" width="343" height="100" fill="red"/>
        </svg>"#,
        &options,
    )
    .unwrap();
    let bbox = tree.root().abs_layer_bounding_box();
    assert!((bbox.x() - 35.0).abs() < 0.01, "bbox.x={}", bbox.x());
    assert!(bbox.right() < 421.0, "bbox.right={}", bbox.right());
}

#[test]
fn ibm_plex_zero_feature_shapes_to_alternate_glyph() {
    let mut config = FontConfig {
        size: 200.0,
        ..FontConfig::default()
    };
    config.font_feature_settings.insert("zero".into(), 1);
    // Match the mixed-selector fixture configuration: unsupported axes
    // on this static IBM Plex face must not suppress an otherwise valid GSUB feature.
    config.font_variation_settings.insert("wght".into(), 750.0);
    let font_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/IBMPlexSans-Bold.ttf");
    let loader = Arc::new(FontLoader::from_path_with_config(&font_path, &config).unwrap());
    let expected = u32::from(
        loader
            .glyph_id_for_selector(&GlyphSelector::Name("zero.alt02".to_string()))
            .0,
    );
    let base = u32::from(
        loader
            .glyph_id_for_selector(&GlyphSelector::CodePoint('0' as u32))
            .0,
    );
    assert_ne!(expected, base);

    let mut component_fonts = HashMap::new();
    component_fonts.insert("main".to_string(), vec![loader]);
    let mut renderer = TypographyRenderer::new(&component_fonts).unwrap();
    let request = CenteredUnicodeRequest {
        component_id: "main",
        config: &config,
        font_size: 200.0,
        text: "0",
        font_index: None,
        color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        center: (400.0, 300.0),
    };
    let layout = renderer
        .build_centered_unicode_layout(&request)
        .unwrap()
        .unwrap();
    let mut ids = Vec::new();
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(run) = item {
                ids.extend(run.positioned_glyphs().map(|glyph| glyph.id));
            }
        }
    }
    assert_eq!(ids, vec![expected]);
}

#[test]
fn shared_svg_node_render_keeps_viewbox_and_ancestor_transform() {
    let em_square = usvg::Size::from_wh(100.0, 100.0).unwrap();
    let options = usvg::Options {
        forced_size: Some(em_square),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="100 200 100 100">
             <g id="glyph7" transform="translate(5 7)" opacity="0.99">
               <rect x="120" y="220" width="10" height="10" fill="#ff0000"/>
             </g>
           </svg>"##,
        &options,
    )
    .unwrap();
    let node = tree.node_by_id("glyph7").unwrap();
    let bbox = node.abs_layer_bounding_box().unwrap();
    // Root viewBox translates (-100, -200) and glyph7 adds (+5, +7).
    assert!((bbox.x() - 25.0).abs() < 0.01, "bbox.x={}", bbox.x());
    assert!((bbox.y() - 27.0).abs() < 0.01, "bbox.y={}", bbox.y());

    let left = bbox.x().floor();
    let top = bbox.y().floor();
    let right = (bbox.x() + bbox.width()).ceil();
    let bottom = (bbox.y() + bbox.height()).ceil();
    let mut pixmap = tiny_skia::Pixmap::new(
        (right - left).max(1.0) as u32,
        (bottom - top).max(1.0) as u32,
    )
    .unwrap();
    let transform = svg_selected_node_render_transform(node, bbox, 1.0, (0.0, 0.0), (left, top));
    assert!(resvg::render_node(node, transform, &mut pixmap.as_mut()).is_some());
    assert!(
        pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] != 0),
        "selected glyph node was rendered outside its absolute bbox"
    );
    let center = ((pixmap.height() / 2 * pixmap.width() + pixmap.width() / 2) * 4 + 3) as usize;
    assert_ne!(
        pixmap.data()[center],
        0,
        "selected glyph local transform was lost or applied twice"
    );
}

#[test]
fn opentype_settings_are_forwarded_as_typed_values() {
    let mut config = FontConfig::default();
    assert!(feature_settings(&config).unwrap().is_empty());
    config.font_optical_sizing = false;
    assert!(variation_settings_for_size(&config, config.size)
        .unwrap()
        .is_empty());
    config.font_feature_settings.insert("liga".into(), 0);
    config.font_variation_settings.insert("wght".into(), 650.0);
    let features = feature_settings(&config).unwrap();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].value, 0);
    let variations = variation_settings_for_size(&config, config.size).unwrap();
    assert!(variations.iter().any(|setting| setting.value == 650.0));
}

#[test]
fn optical_sizing_adds_opsz_only_when_not_explicit() {
    let mut config = FontConfig {
        size: 42.0,
        font_optical_sizing: true,
        ..FontConfig::default()
    };
    let settings = variation_settings_for_size(&config, config.size).unwrap();
    assert!(settings.iter().any(|setting| setting.value == 42.0));
    config.font_variation_settings.insert("opsz".into(), 12.0);
    let settings = variation_settings_for_size(&config, config.size).unwrap();
    assert!(settings.iter().any(|setting| setting.value == 12.0));
    assert!(!settings.iter().any(|setting| setting.value == 42.0));
}
