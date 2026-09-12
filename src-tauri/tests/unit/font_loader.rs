use super::{select_font_face_data, validate_settings, FontLoader};
use crate::scene::{FontConfig, GlyphSelector};

#[test]
fn rejects_invalid_opentype_tags() {
    let mut config = FontConfig::default();
    config.font_feature_settings.insert("liga".into(), 0);
    assert!(validate_settings(&config).is_ok());

    config.font_variation_settings.insert("Italic".into(), 20.0);
    let error = validate_settings(&config).unwrap_err().to_string();
    assert!(error.contains("variable-font axis"));
    assert!(error.contains("exactly 4 printable ASCII"));
    assert!(error.contains("Italic"));
}

#[test]
fn rejects_feature_values_outside_parley_range() {
    let mut config = FontConfig::default();
    config
        .font_feature_settings
        .insert("salt".into(), u32::from(u16::MAX) + 1);
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

fn collection_with_faces(fonts: &[&[u8]]) -> Vec<u8> {
    fn read_u16(data: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
    }

    fn rewrite_offsets(font: &[u8], base: usize) -> Vec<u8> {
        let mut face = font.to_vec();
        let num_tables = read_u16(&face, 4) as usize;
        for index in 0..num_tables {
            let offset_position = 12 + index * 16 + 8;
            let old = u32::from_be_bytes(
                face[offset_position..offset_position + 4]
                    .try_into()
                    .unwrap(),
            );
            face[offset_position..offset_position + 4]
                .copy_from_slice(&(old + base as u32).to_be_bytes());
        }
        face
    }

    let header_len = 12 + fonts.len() * 4;
    let mut offsets = Vec::with_capacity(fonts.len());
    let mut faces = Vec::with_capacity(fonts.len());
    let mut cursor = header_len;
    for font in fonts {
        cursor = (cursor + 3) & !3;
        offsets.push(cursor);
        let face = rewrite_offsets(font, cursor);
        cursor += face.len();
        faces.push(face);
    }

    let mut collection = vec![0u8; cursor];
    collection[0..4].copy_from_slice(b"ttcf");
    collection[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    collection[8..12].copy_from_slice(&(fonts.len() as u32).to_be_bytes());
    for (index, offset) in offsets.iter().enumerate() {
        let pos = 12 + index * 4;
        collection[pos..pos + 4].copy_from_slice(&(*offset as u32).to_be_bytes());
        collection[*offset..*offset + faces[index].len()].copy_from_slice(&faces[index]);
    }
    collection
}

fn bundled_font(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/fonts")
            .join(name),
    )
    .unwrap()
}

#[test]
fn selects_and_materializes_requested_font_collection_face() {
    let first = bundled_font("IBMPlexSans-Bold.ttf");
    let second = bundled_font("NotoSansTest-Regular.ttf");
    let first_glyphs = ttf_parser::Face::parse(&first, 0)
        .unwrap()
        .number_of_glyphs();
    let second_glyphs = ttf_parser::Face::parse(&second, 0)
        .unwrap()
        .number_of_glyphs();
    assert_ne!(first_glyphs, second_glyphs);

    let collection = collection_with_faces(&[&first, &second]);
    assert_eq!(ttf_parser::fonts_in_collection(&collection), Some(2));

    let selected = select_font_face_data(&collection, 1).unwrap();
    assert_eq!(ttf_parser::fonts_in_collection(&selected), None);
    let face = ttf_parser::Face::parse(&selected, 0).unwrap();
    assert_eq!(face.number_of_glyphs(), second_glyphs);

    let loader =
        FontLoader::from_bytes_with_config(&collection, 1, &FontConfig::default()).unwrap();
    assert_eq!(loader.face_index(), 0);
    let loaded_face = ttf_parser::Face::parse(loader.source_data(), loader.face_index()).unwrap();
    assert_eq!(loaded_face.number_of_glyphs(), second_glyphs);
    assert!(loader.has_selector(&GlyphSelector::CodePoint(0x0E70)));
}

#[test]
fn rejects_nonzero_face_index_for_single_font() {
    let font = bundled_font("NotoSansTest-Regular.ttf");
    let error = select_font_face_data(&font, 1).unwrap_err().to_string();
    assert!(error.contains("not a font collection"));
}

#[test]
fn rejects_out_of_range_font_collection_face() {
    let font = bundled_font("NotoSansTest-Regular.ttf");
    let collection = collection_with_faces(&[&font, &font]);
    let error = select_font_face_data(&collection, 2)
        .unwrap_err()
        .to_string();
    assert!(error.contains("out of range"));
}
