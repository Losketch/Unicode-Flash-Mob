use super::{portable_asset_reference, AnimationCurve, CharEntry, Event, EventType, RenderConfig};
use crate::content_template::ContentTemplate;
use crate::scene::{
    AnimatableProperty, Color, ComponentPropertyValue, FontConfig, GlyphComponent, GlyphSelector,
    GroupComponent, ImageComponent, ImageFit, Position, ProgressBarBorder, ProgressBarComponent,
    ProgressDirection, Scale2D, SceneComponent, Size, TextAlign, Transform2D,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("ufm-{name}-{nonce}"))
}

#[test]
fn bundled_asset_paths_are_serialized_portably() {
    let development_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts")
        .join("Example.ttf");
    assert_eq!(
        portable_asset_reference(&development_path),
        PathBuf::from("assets/fonts/Example.ttf")
    );
}

#[test]
fn from_file_resolves_project_relative_paths_against_config_directory() {
    let root = unique_temp_dir("relative-load");
    std::fs::create_dir_all(&root).unwrap();
    let config_path = root.join("project.json");
    let mut config = RenderConfig {
        output_path: PathBuf::from("output/video.mp4"),
        music_path: Some(PathBuf::from("media/music.wav")),
        components: vec![
            SceneComponent::Image(ImageComponent {
                source: PathBuf::from("media/image.png"),
                ..ImageComponent::default()
            }),
            SceneComponent::Glyph(GlyphComponent {
                fonts: vec![PathBuf::from("fonts/project.ttf")],
                ..GlyphComponent::default()
            }),
        ],
        ..RenderConfig::default()
    };
    config.ffmpeg.path = PathBuf::from("tools/ffmpeg-custom");
    config.content_templates.insert(
        "external".to_string(),
        ContentTemplate::External {
            executable: PathBuf::from("tools/template-runner"),
            args: Vec::new(),
        },
    );
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let loaded = RenderConfig::from_file(&config_path).unwrap();
    assert_eq!(loaded.output_path, root.join("output/video.mp4"));
    assert_eq!(loaded.music_path, Some(root.join("media/music.wav")));
    assert_eq!(loaded.ffmpeg.path, root.join("tools/ffmpeg-custom"));
    let SceneComponent::Image(image) = &loaded.components[0] else {
        panic!("expected image");
    };
    assert_eq!(image.source, root.join("media/image.png"));
    let SceneComponent::Glyph(glyph) = &loaded.components[1] else {
        panic!("expected glyph");
    };
    assert_eq!(glyph.fonts, vec![root.join("fonts/project.ttf")]);
    let ContentTemplate::External { executable, .. } =
        loaded.content_templates.get("external").unwrap()
    else {
        panic!("expected external template");
    };
    assert_eq!(executable, &root.join("tools/template-runner"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn from_file_preserves_bundled_assets_and_bare_commands() {
    let root = unique_temp_dir("portable-load");
    std::fs::create_dir_all(&root).unwrap();
    let config_path = root.join("project.json");
    let mut config = RenderConfig {
        components: vec![SceneComponent::Glyph(GlyphComponent {
            fonts: vec![PathBuf::from("assets/fonts/IBMPlexSans-Bold.ttf")],
            ..GlyphComponent::default()
        })],
        ..RenderConfig::default()
    };
    config.ffmpeg.path = PathBuf::from("ffmpeg");
    config.content_templates.insert(
        "external".to_string(),
        ContentTemplate::External {
            executable: PathBuf::from("python"),
            args: Vec::new(),
        },
    );
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let loaded = RenderConfig::from_file(&config_path).unwrap();
    let SceneComponent::Glyph(glyph) = &loaded.components[0] else {
        panic!("expected glyph");
    };
    assert_eq!(
        glyph.fonts,
        vec![PathBuf::from("assets/fonts/IBMPlexSans-Bold.ttf")]
    );
    assert_eq!(loaded.ffmpeg.path, PathBuf::from("ffmpeg"));
    let ContentTemplate::External { executable, .. } =
        loaded.content_templates.get("external").unwrap()
    else {
        panic!("expected external template");
    };
    assert_eq!(executable, &PathBuf::from("python"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn to_file_relativizes_paths_inside_the_config_directory() {
    let root = unique_temp_dir("portable-save");
    std::fs::create_dir_all(&root).unwrap();
    let config_path = root.join("project.json");
    let mut config = RenderConfig {
        output_path: root.join("output/video.mp4"),
        music_path: Some(root.join("media/music.wav")),
        components: vec![SceneComponent::Image(ImageComponent {
            source: root.join("media/image.png"),
            ..ImageComponent::default()
        })],
        ..RenderConfig::default()
    };
    config.ffmpeg.path = root.join("tools/ffmpeg-custom");
    config.content_templates.insert(
        "external".to_string(),
        ContentTemplate::External {
            executable: root.join("tools/template-runner"),
            args: Vec::new(),
        },
    );

    config.to_file(&config_path).unwrap();
    let saved: RenderConfig =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(saved.output_path, PathBuf::from("output/video.mp4"));
    assert_eq!(saved.music_path, Some(PathBuf::from("media/music.wav")));
    assert_eq!(saved.ffmpeg.path, PathBuf::from("tools/ffmpeg-custom"));
    let SceneComponent::Image(image) = &saved.components[0] else {
        panic!("expected image");
    };
    assert_eq!(image.source, PathBuf::from("media/image.png"));
    let ContentTemplate::External { executable, .. } =
        saved.content_templates.get("external").unwrap()
    else {
        panic!("expected external template");
    };
    assert_eq!(executable, &PathBuf::from("tools/template-runner"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_size_only_config_gets_advanced_defaults() {
    let config: FontConfig = serde_json::from_str(r#"{"size":128.0}"#).unwrap();
    assert_eq!(config.size, 128.0);
    assert!(config.font_feature_settings.is_empty());
    assert!(config.font_variation_settings.is_empty());
    assert!(config.font_optical_sizing);
}

#[test]
fn default_config_serializes_scene_schema_without_legacy_slots() {
    let config = RenderConfig::default();
    let value = serde_json::to_value(&config).unwrap();

    assert_eq!(
        value["schema_version"].as_u64(),
        Some(super::CURRENT_SCHEMA_VERSION as u64)
    );
    assert_eq!(value["components"].as_array().map(Vec::len), Some(2));
    assert_eq!(value["components"][0]["type"].as_str(), Some("glyph"));
    assert_eq!(value["components"][1]["type"].as_str(), Some("text"));
    assert!(value.get("main_text").is_none());
    assert!(value.get("bottom_text").is_none());
    assert!(value.get("main_font").is_none());
    assert!(value.get("bottom_font").is_none());
}

#[test]
fn scene_components_round_trip_in_scene_schema_v4() {
    let components = vec![
        SceneComponent::Image(ImageComponent {
            id: "logo".to_string(),
            enabled: true,
            source: PathBuf::from("assets/images/logo.webp"),
            position: Position { x: 0.2, y: 0.3 },
            size: Size {
                width: 0.4,
                height: 0.25,
            },
            opacity: 0.75,
            fit: ImageFit::Cover,
        }),
        SceneComponent::ProgressBar(ProgressBarComponent {
            id: "progress".to_string(),
            enabled: true,
            position: Position { x: 0.5, y: 0.9 },
            size: Size {
                width: 0.8,
                height: 0.03,
            },
            progress: 0.25,
            background_color: Color::default(),
            fill_color: Color::default(),
            direction: ProgressDirection::RightToLeft,
            border: Some(ProgressBarBorder {
                color: Color {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 128,
                },
                width: 3,
            }),
        }),
    ];
    let config = RenderConfig {
        components,
        ..RenderConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    let decoded: RenderConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert_eq!(decoded.components.len(), 2);
    assert!(matches!(decoded.components[0], SceneComponent::Image(_)));
    assert!(matches!(
        decoded.components[1],
        SceneComponent::ProgressBar(_)
    ));
    let SceneComponent::ProgressBar(progress) = &decoded.components[1] else {
        unreachable!();
    };
    let border = progress.border.as_ref().unwrap();
    assert_eq!(border.width, 3);
    assert_eq!(border.color.a, 128);
}

#[test]
fn progress_bar_border_is_optional_in_current_v4() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            border: None,
            ..ProgressBarComponent::default()
        })],
        ..RenderConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    let decoded: RenderConfig = serde_json::from_str(&json).unwrap();
    let SceneComponent::ProgressBar(progress) = &decoded.components[0] else {
        unreachable!();
    };
    assert!(progress.border.is_none());
    assert_eq!(decoded.schema_version, super::CURRENT_SCHEMA_VERSION);
}

#[test]
fn progress_bar_border_width_rejects_fractional_json() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            border: Some(ProgressBarBorder::default()),
            ..ProgressBarComponent::default()
        })],
        ..RenderConfig::default()
    };
    let mut value = serde_json::to_value(config).unwrap();
    value["components"][0]["border"]["width"] = serde_json::json!(0.1);
    let error = serde_json::from_value::<RenderConfig>(value).unwrap_err();
    assert!(error.to_string().contains("expected u32"));
}

#[test]
fn progress_bar_border_width_rejects_negative_json() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            border: Some(ProgressBarBorder::default()),
            ..ProgressBarComponent::default()
        })],
        ..RenderConfig::default()
    };
    let mut value = serde_json::to_value(config).unwrap();
    value["components"][0]["border"]["width"] = serde_json::json!(-1);
    let error = serde_json::from_value::<RenderConfig>(value).unwrap_err();
    assert!(error.to_string().contains("expected u32"));
}

#[test]
fn image_asset_paths_are_normalized_with_other_bundled_assets() {
    let development_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("images")
        .join("logo.png");
    let mut config = RenderConfig::default();
    config
        .components
        .push(SceneComponent::Image(ImageComponent {
            source: development_path,
            ..ImageComponent::default()
        }));
    config.normalize_bundled_asset_references();
    let source = config
        .components
        .last_mut()
        .unwrap()
        .image_source_mut()
        .unwrap();
    assert_eq!(source, &PathBuf::from("assets/images/logo.png"));
}

#[test]
fn current_v4_scene_normalization_is_idempotent() {
    let mut config = RenderConfig::default();
    let before = serde_json::to_value(&config.components).unwrap();
    config.normalize_scene();
    assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert_eq!(serde_json::to_value(&config.components).unwrap(), before);
}

#[test]
fn future_schema_is_not_downgraded_by_normalization() {
    let mut config = RenderConfig {
        schema_version: super::CURRENT_SCHEMA_VERSION + 1,
        ..RenderConfig::default()
    };
    config.normalize_scene();
    assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION + 1);
}

#[test]
fn missing_schema_version_is_treated_as_legacy_v3() {
    let mut value = serde_json::to_value(RenderConfig::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("schema_version");
    object.remove("components");

    let mut config: RenderConfig = serde_json::from_value(value).unwrap();
    assert_eq!(config.schema_version, super::LEGACY_SCHEMA_VERSION);
    config.normalize_scene();

    assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert_eq!(config.components.len(), 2);
}

#[test]
fn intentional_empty_v4_scene_stays_empty() {
    let mut config = RenderConfig {
        schema_version: 4,
        components: Vec::new(),
        ..RenderConfig::default()
    };
    config.normalize_scene();
    assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert!(config.components.is_empty());
}

#[test]
fn legacy_fixed_slots_migrate_to_ordered_components() {
    let mut config = RenderConfig {
        schema_version: super::LEGACY_SCHEMA_VERSION,
        components: Vec::new(),
        legacy_main_text: Some(super::LegacyTextElement {
            id: "legacy-main".to_string(),
            fonts: vec![PathBuf::from("main.ttf")],
            content: "ignored-by-v3-main-renderer".to_string(),
            position: Position { x: 0.25, y: 0.4 },
            color: Color {
                r: 1,
                g: 2,
                b: 3,
                a: 4,
            },
            enabled: true,
            align: TextAlign::Center,
            wrap: true,
            max_width: 0.5,
        }),
        legacy_main_font: Some(FontConfig {
            size: 321.0,
            ..FontConfig::default()
        }),
        legacy_bottom_text: Some(super::LegacyTextElement {
            id: "legacy-bottom".to_string(),
            fonts: vec![PathBuf::from("bottom.ttf")],
            content: "{code} :: {description}".to_string(),
            position: Position { x: 0.1, y: 0.9 },
            color: Color {
                r: 5,
                g: 6,
                b: 7,
                a: 8,
            },
            enabled: true,
            align: TextAlign::Right,
            wrap: false,
            max_width: 0.8,
        }),
        legacy_bottom_font: Some(FontConfig {
            size: 37.0,
            ..FontConfig::default()
        }),
        ..RenderConfig::default()
    };

    config.normalize_scene();

    assert_eq!(config.schema_version, super::CURRENT_SCHEMA_VERSION);
    assert_eq!(config.components.len(), 2);
    let glyph = config.primary_glyph_component().unwrap();
    assert_eq!(glyph.id, "legacy-main");
    assert_eq!(glyph.content, "{glyph}");
    assert_eq!(glyph.fonts, vec![PathBuf::from("main.ttf")]);
    assert_eq!(glyph.font.size, 321.0);
    let SceneComponent::Text(text) = &config.components[1] else {
        panic!("expected migrated text component");
    };
    assert_eq!(text.id, "legacy-bottom");
    assert_eq!(text.content, "{code} :: {description}");
    assert_eq!(text.font.size, 37.0);
    assert_eq!(text.align, TextAlign::Right);
}

#[test]
fn legacy_event_names_deserialize_to_component_events() {
    let event_type: EventType = serde_json::from_str(
        r#"{"set_text_position":{"element_id":"caption","position":{"x":0.2,"y":0.3}}}"#,
    )
    .unwrap();

    match event_type {
        EventType::SetComponentPosition {
            element_id,
            position,
        } => {
            assert_eq!(element_id, "caption");
            assert_eq!(position.x, 0.2);
            assert_eq!(position.y, 0.3);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn parses_concatenated_glyph_selectors_and_font_indices() {
    let entry = CharEntry {
        code_point: "U+0033{0}/at{0}#0{1}".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: None,
    };
    let specs = CharEntry::glyph_specs_from(&entry.code_point);
    assert_eq!(specs.len(), 3);
    assert_eq!(specs[0].selector, GlyphSelector::CodePoint(0x33));
    assert_eq!(specs[0].font_index, Some(0));
    assert_eq!(specs[1].selector, GlyphSelector::Name("at".to_string()));
    assert_eq!(specs[1].font_index, Some(0));
    assert_eq!(specs[2].selector, GlyphSelector::Index(0));
    assert_eq!(specs[2].font_index, Some(1));
}

#[test]
fn selected_unicode_text_ignores_explicit_glyph_selectors() {
    let entry = CharEntry {
        code_point: "U+0041/fooU+0042#3".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: None,
    };
    assert_eq!(entry.selected_unicode_text(), "AB");

    let literal = CharEntry {
        code_point: "not-a-selector".to_string(),
        ..entry
    };
    assert_eq!(literal.selected_unicode_text(), "not-a-selector");

    let hex_like_literal = CharEntry {
        code_point: "face-to-face".to_string(),
        ..literal
    };
    assert_eq!(hex_like_literal.selected_unicode_text(), "face-to-face");
}

#[test]
fn accepts_all_codepoint_prefix_forms() {
    let entry = CharEntry {
        code_point: "U+0041u+0042,0x43 0X44 45".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: None,
    };
    let values: Vec<_> = CharEntry::glyph_specs_from(&entry.code_point)
        .into_iter()
        .map(|spec| spec.selector)
        .collect();
    assert_eq!(
        values,
        vec![
            GlyphSelector::CodePoint(0x41),
            GlyphSelector::CodePoint(0x42),
            GlyphSelector::CodePoint(0x43),
            GlyphSelector::CodePoint(0x44),
            GlyphSelector::CodePoint(0x45),
        ]
    );
}

#[test]
fn separates_named_glyphs_from_delimited_codepoints() {
    let entry = CharEntry {
        code_point: "/A U+0042,/C".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: None,
    };
    let selectors: Vec<_> = CharEntry::glyph_specs_from(&entry.code_point)
        .into_iter()
        .map(|spec| spec.selector)
        .collect();
    assert_eq!(
        selectors,
        vec![
            GlyphSelector::Name("A".to_string()),
            GlyphSelector::CodePoint(0x42),
            GlyphSelector::Name("C".to_string()),
        ]
    );
}

#[test]
fn generic_component_property_events_round_trip() {
    let events = vec![
        Event {
            frame: 3,
            event_type: EventType::SetComponentProperty {
                element_id: "group".to_string(),
                property: AnimatableProperty::Rotation,
                value: ComponentPropertyValue::Number(15.0),
            },
        },
        Event {
            frame: 4,
            event_type: EventType::AnimateComponentProperty {
                element_id: "progress".to_string(),
                property: AnimatableProperty::Progress,
                start_value: ComponentPropertyValue::Number(0.25),
                end_value: ComponentPropertyValue::Number(0.75),
                duration: 0.5,
                curve: AnimationCurve::EaseInOut,
            },
        },
    ];
    let json = serde_json::to_string(&events).unwrap();
    let decoded: Vec<Event> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.len(), 2);
    let EventType::SetComponentProperty {
        property, value, ..
    } = &decoded[0].event_type
    else {
        panic!("expected set_component_property");
    };
    assert_eq!(*property, AnimatableProperty::Rotation);
    let ComponentPropertyValue::Number(value) = value else {
        panic!("expected numeric property value");
    };
    assert!((*value - 15.0).abs() < f64::EPSILON);
    assert!(matches!(
        &decoded[1].event_type,
        EventType::AnimateComponentProperty {
            property: AnimatableProperty::Progress,
            ..
        }
    ));
}

#[test]
fn reachable_pause_events_extend_total_frames() {
    let config = RenderConfig {
        fps: 10.0,
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(5),
        }],
        events: vec![Event {
            frame: 2,
            event_type: EventType::Pause { duration: 0.3 },
        }],
        ..Default::default()
    };
    assert_eq!(config.total_frames(), 8);
}

#[test]
fn unreachable_pause_events_do_not_extend_total_frames() {
    let config = RenderConfig {
        fps: 10.0,
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(5),
        }],
        events: vec![Event {
            frame: 5,
            event_type: EventType::Pause { duration: 1.0 },
        }],
        ..Default::default()
    };
    assert_eq!(config.total_frames(), 5);
}

#[test]
fn group_component_round_trips_in_current_v4() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            transform: Transform2D {
                translation: Position { x: 0.1, y: -0.2 },
                scale: Scale2D { x: 1.2, y: -0.8 },
                rotation: 15.0,
                anchor: Position { x: 0.5, y: 0.5 },
            },
            opacity: 0.75,
            children: vec![SceneComponent::ProgressBar(ProgressBarComponent::default())],
        })],
        ..RenderConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    let decoded: RenderConfig = serde_json::from_str(&json).unwrap();
    let SceneComponent::Group(group) = &decoded.components[0] else {
        panic!("expected group");
    };
    assert_eq!(group.children.len(), 1);
    assert_eq!(group.transform.scale.y, -0.8);
    assert_eq!(group.opacity, 0.75);
}

#[test]
fn primary_glyph_can_live_inside_a_group() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            children: vec![SceneComponent::Glyph(GlyphComponent {
                id: "nested".to_string(),
                ..GlyphComponent::default()
            })],
            ..GroupComponent::default()
        })],
        ..RenderConfig::default()
    };
    assert_eq!(config.primary_glyph_component().unwrap().id, "nested");
}

#[test]
fn nested_group_assets_are_serialized_portably() {
    let development_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("images")
        .join("nested.png");
    let mut config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            children: vec![SceneComponent::Image(ImageComponent {
                id: "nested_image".to_string(),
                source: development_path,
                ..ImageComponent::default()
            })],
            ..GroupComponent::default()
        })],
        ..RenderConfig::default()
    };
    config.normalize_bundled_asset_references();
    let SceneComponent::Group(group) = &config.components[0] else {
        panic!("expected group");
    };
    let SceneComponent::Image(image) = &group.children[0] else {
        panic!("expected image");
    };
    assert_eq!(image.source, PathBuf::from("assets/images/nested.png"));
}
