use super::validate_settings;
use crate::scene::FontConfig;

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
