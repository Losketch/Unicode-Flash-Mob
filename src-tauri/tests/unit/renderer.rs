use super::{
    add_rawvideo_input_args, add_stream_mapping_args, concat_file_entry, glyph_sequence_segments,
    initial_render_state, next_frame_job, process_events, render_frame_png,
    resolve_component_frame_states, typographic_glyph_origin, unicode_shaping_sequence,
    update_animations, validate_render_config, GlyphSequenceSegment,
};
use crate::json_config::{AnimationCurve, CharEntry, Event, EventType, GlyphSpec, RenderConfig};
use crate::scene::{
    AnimatableProperty, Color, ComponentPropertyValue, GlyphComponent, GlyphSelector,
    GroupComponent, ImageComponent, Position, ProgressBarBorder, ProgressBarComponent,
    ProgressDirection, Scale2D, SceneComponent, Size, Transform2D,
};
use std::path::Path;
use std::process::Command;

#[test]
fn unicode_sequence_is_kept_together_for_shaping() {
    let specs = vec![
        GlyphSpec {
            selector: GlyphSelector::CodePoint('f' as u32),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::CodePoint('f' as u32),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::CodePoint('i' as u32),
            font_index: None,
        },
    ];
    assert_eq!(unicode_shaping_sequence(&specs).as_deref(), Some("ffi"));
}

#[test]
fn single_unicode_selector_still_uses_shaping() {
    let specs = vec![GlyphSpec {
        selector: GlyphSelector::CodePoint('A' as u32),
        font_index: None,
    }];
    assert_eq!(unicode_shaping_sequence(&specs).as_deref(), Some("A"));
}

#[test]
fn mixed_selector_expression_keeps_unicode_runs_shapeable() {
    let specs = CharEntry::glyph_specs_from("#83/zeroU+30");
    assert_eq!(specs.len(), 3);
    assert_eq!(
        glyph_sequence_segments(&specs),
        vec![
            GlyphSequenceSegment::Exact(specs[0].clone()),
            GlyphSequenceSegment::Exact(specs[1].clone()),
            GlyphSequenceSegment::Unicode {
                text: "0".to_string(),
                font_index: None,
            },
        ]
    );
}

#[test]
fn adjacent_unicode_after_exact_selector_stays_one_shaping_run() {
    let specs = vec![
        GlyphSpec {
            selector: GlyphSelector::CodePoint('A' as u32),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::Index(3),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::CodePoint('B' as u32),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::CodePoint('C' as u32),
            font_index: None,
        },
    ];
    assert_eq!(
        glyph_sequence_segments(&specs),
        vec![
            GlyphSequenceSegment::Unicode {
                text: "A".to_string(),
                font_index: None,
            },
            GlyphSequenceSegment::Exact(specs[1].clone()),
            GlyphSequenceSegment::Unicode {
                text: "BC".to_string(),
                font_index: None,
            },
        ]
    );
}

#[test]
fn explicit_glyph_selectors_bypass_unicode_shaping() {
    let explicit = vec![
        GlyphSpec {
            selector: GlyphSelector::CodePoint('A' as u32),
            font_index: None,
        },
        GlyphSpec {
            selector: GlyphSelector::Index(3),
            font_index: None,
        },
    ];
    assert!(unicode_shaping_sequence(&explicit).is_none());
}

#[test]
fn adjacent_unicode_with_same_font_index_stays_one_shaping_run() {
    let specs = CharEntry::glyph_specs_from("U+41{2}U+30A{2}");
    assert_eq!(
        glyph_sequence_segments(&specs),
        vec![GlyphSequenceSegment::Unicode {
            text: "A\u{030A}".to_string(),
            font_index: Some(2),
        }]
    );
}

#[test]
fn pinned_emoji_modifier_sequence_stays_one_shaping_run() {
    let specs = CharEntry::glyph_specs_from("U+1F44D{3}U+1F3FD{3}");
    assert_eq!(
        glyph_sequence_segments(&specs),
        vec![GlyphSequenceSegment::Unicode {
            text: "👍🏽".to_string(),
            font_index: Some(3),
        }]
    );
}

#[test]
fn changing_font_index_breaks_unicode_shaping_runs() {
    let specs = CharEntry::glyph_specs_from("U+41{1}U+42{2}U+43{2}");
    assert_eq!(
        glyph_sequence_segments(&specs),
        vec![
            GlyphSequenceSegment::Unicode {
                text: "A".to_string(),
                font_index: Some(1),
            },
            GlyphSequenceSegment::Unicode {
                text: "BC".to_string(),
                font_index: Some(2),
            },
        ]
    );
}

#[test]
fn invalid_axis_tag_is_reported_as_component_configuration_error() {
    let mut config = RenderConfig::default();
    let font = config.components[0].font_mut().unwrap();
    font.font_variation_settings.insert("Italic".into(), 20.0);
    let error = validate_render_config(&config, false).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("Invalid font settings for component"));
    assert!(message.contains("variable-font axis"));
    assert!(message.contains("Italic"));
}

#[test]
fn ffmpeg_stream_maps_follow_all_inputs() {
    let mut command = Command::new("ffmpeg");
    add_rawvideo_input_args(&mut command, 1920, 1080, 30.0);
    command.arg("-i").arg("music.m4a");
    add_stream_mapping_args(&mut command, true);

    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let music_input = args
        .windows(2)
        .position(|pair| pair[0] == "-i" && pair[1] == "music.m4a")
        .expect("music input should be present");
    let first_map = args
        .iter()
        .position(|arg| arg == "-map")
        .expect("stream mapping should be present");

    assert!(first_map > music_input + 1);
    let mappings: Vec<&str> = args[first_map..first_map + 4]
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(mappings, ["-map", "0:v:0", "-map", "1:a:0?"]);
}

#[test]
fn typographic_center_uses_advance_and_baseline() {
    assert_eq!(
        typographic_glyph_origin((960, 540), 201.0, None, 512.0, -128.0),
        (859, 732),
    );
}

#[test]
fn zero_advance_mark_is_visually_centered_horizontally() {
    assert_eq!(
        typographic_glyph_origin((960, 540), 0.0, Some(-35.0), 512.0, -128.0),
        (995, 732),
    );
}

#[test]
fn concat_list_uses_ffmpeg_friendly_windows_path() {
    assert_eq!(
        concat_file_entry(Path::new(r"C:\render parts\segment-0001.mp4")),
        "file 'C:/render parts/segment-0001.mp4'\n",
    );
}

#[test]
fn pause_event_adds_frames_without_advancing_the_character() {
    let mut config = RenderConfig {
        fps: 1.0,
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(3),
        }],
        events: vec![Event {
            frame: 1,
            event_type: EventType::Pause { duration: 2.0 },
        }],
        ..Default::default()
    };

    for component in &mut config.components {
        match component {
            crate::scene::SceneComponent::Glyph(component) => component.enabled = false,
            crate::scene::SceneComponent::Text(component) => component.enabled = false,
            crate::scene::SceneComponent::Image(component) => component.enabled = false,
            crate::scene::SceneComponent::ProgressBar(component) => component.enabled = false,
            crate::scene::SceneComponent::Group(component) => component.enabled = false,
        }
    }

    let mut state = initial_render_state(&config);
    let mut color_index = 0;
    let mut entry_index = 0;
    let mut frame_in_char = 0;
    let mut frame_index = 0;
    let mut jobs = Vec::new();

    while let Some(job) = next_frame_job(
        &config,
        &mut state,
        &mut color_index,
        &mut entry_index,
        &mut frame_in_char,
        &mut frame_index,
    ) {
        jobs.push(job);
    }

    assert_eq!(jobs.len() as u64, config.total_frames());
    assert_eq!(jobs.len(), 5);
}

#[test]
fn character_overrides_and_component_events_are_resolved_per_frame() {
    let mut config = RenderConfig::default();
    let primary_id = config
        .primary_glyph_component()
        .expect("default scene has a glyph component")
        .id
        .clone();
    let bottom_id = config
        .components
        .iter()
        .find_map(|component| match component {
            SceneComponent::Text(text) => Some(text.id.clone()),
            _ => None,
        })
        .expect("default scene has a text component");
    config.characters = vec![CharEntry {
        code_point: "U+0041".to_string(),
        description: String::new(),
        background_color: None,
        text_color: Some(Color {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        }),
        position: Some(Position { x: 0.2, y: 0.3 }),
        duration_frames: Some(1),
    }];
    config.events = vec![
        Event {
            frame: 0,
            event_type: EventType::SetComponentColor {
                element_id: bottom_id.clone(),
                color: Color {
                    r: 5,
                    g: 6,
                    b: 7,
                    a: 8,
                },
            },
        },
        Event {
            frame: 0,
            event_type: EventType::SetComponentPosition {
                element_id: bottom_id.clone(),
                position: Position { x: 0.7, y: 0.8 },
            },
        },
    ];

    let mut state = initial_render_state(&config);
    let mut color_index = 0;
    let mut entry_index = 0;
    let mut frame_in_char = 0;
    let mut frame_index = 0;
    let job = next_frame_job(
        &config,
        &mut state,
        &mut color_index,
        &mut entry_index,
        &mut frame_in_char,
        &mut frame_index,
    )
    .unwrap();

    let primary = job.component_states.get(&primary_id).unwrap();
    let bottom = job.component_states.get(&bottom_id).unwrap();
    assert_eq!(primary.color.as_ref().unwrap().r, 1);
    assert_eq!(primary.position.x, 0.2);
    assert_eq!(bottom.color.as_ref().unwrap().r, 5);
    assert_eq!(bottom.position.x, 0.7);
}

#[test]
fn enabled_image_requires_a_source() {
    let config = RenderConfig {
        components: vec![SceneComponent::Image(ImageComponent {
            enabled: true,
            ..ImageComponent::default()
        })],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("has no image source configured"));
}

#[test]
fn progress_bar_rejects_out_of_range_progress() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            progress: 1.5,
            ..ProgressBarComponent::default()
        })],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("progress must be between 0 and 1"));
}

#[test]
fn progress_bar_rejects_zero_border_width() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            border: Some(ProgressBarBorder {
                width: 0,
                ..ProgressBarBorder::default()
            }),
            ..ProgressBarComponent::default()
        })],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("Component progress border width must be a positive integer pixel value")
    );
}

#[test]
fn color_event_rejects_component_without_single_color_property() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent::default())],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentColor {
                element_id: "progress".to_string(),
                color: Color::default(),
            },
        }],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("has no single color property"));
}

#[test]
fn non_typography_components_respect_scene_paint_order() {
    let solid_bar = |id: &str, color: Color| {
        SceneComponent::ProgressBar(ProgressBarComponent {
            id: id.to_string(),
            enabled: true,
            position: Position { x: 0.5, y: 0.5 },
            size: Size {
                width: 1.0,
                height: 1.0,
            },
            progress: 1.0,
            background_color: color.clone(),
            fill_color: color,
            direction: ProgressDirection::LeftToRight,
            border: None,
        })
    };
    let mut config = RenderConfig {
        resolution: (4, 4),
        dynamic_background: false,
        fixed_background: true,
        background_color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        components: vec![
            solid_bar(
                "lower",
                Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
            ),
            solid_bar(
                "upper",
                Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            ),
        ],
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(1),
        }],
        ..Default::default()
    };
    config.normalize_scene();
    let png = render_frame_png(&config, 0, 4).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert_eq!(image.get_pixel(2, 2), &image::Rgba([255, 0, 0, 255]));
}

#[test]
fn progress_bar_border_survives_json_and_full_frame_render() {
    let base = RenderConfig {
        resolution: (6, 4),
        dynamic_background: false,
        fixed_background: true,
        background_color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            id: "progress".to_string(),
            enabled: true,
            position: Position { x: 0.5, y: 0.5 },
            size: Size {
                width: 1.0,
                height: 1.0,
            },
            progress: 1.0,
            background_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            fill_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            direction: ProgressDirection::LeftToRight,
            border: None,
        })],
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(1),
        }],
        ..RenderConfig::default()
    };
    let mut value = serde_json::to_value(base).unwrap();
    value["components"][0]["border"] = serde_json::json!({
        "color": { "r": 255, "g": 0, "b": 0, "a": 255 },
        "width": 4000
    });
    let config: RenderConfig = serde_json::from_value(value).unwrap();
    let SceneComponent::ProgressBar(progress) = &config.components[0] else {
        panic!("expected progress bar");
    };
    assert_eq!(
        progress.border.as_ref().map(|border| border.width),
        Some(4000)
    );

    let png = render_frame_png(&config, 0, 6).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert_eq!(image.get_pixel(0, 0), &image::Rgba([255, 0, 0, 255]));
    assert_eq!(image.get_pixel(3, 2), &image::Rgba([255, 0, 0, 255]));
}

fn one_character() -> CharEntry {
    CharEntry {
        code_point: "U+0041".to_string(),
        description: String::new(),
        background_color: None,
        text_color: None,
        position: None,
        duration_frames: Some(1),
    }
}

fn solid_progress(id: &str, position: Position, color: Color) -> SceneComponent {
    SceneComponent::ProgressBar(ProgressBarComponent {
        id: id.to_string(),
        enabled: true,
        position,
        size: Size {
            width: 0.25,
            height: 0.25,
        },
        progress: 1.0,
        background_color: color.clone(),
        fill_color: color,
        direction: ProgressDirection::LeftToRight,
        border: None,
    })
}

#[test]
fn identity_group_preserves_leaf_pixels() {
    let leaf = solid_progress(
        "leaf",
        Position { x: 0.5, y: 0.5 },
        Color {
            r: 250,
            g: 10,
            b: 20,
            a: 255,
        },
    );
    let plain = RenderConfig {
        resolution: (16, 16),
        components: vec![leaf.clone()],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let grouped = RenderConfig {
        resolution: (16, 16),
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            transform: Transform2D::default(),
            opacity: 1.0,
            children: vec![leaf],
        })],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    assert_eq!(
        render_frame_png(&plain, 0, 0).unwrap(),
        render_frame_png(&grouped, 0, 0).unwrap()
    );
}

#[test]
fn group_translation_moves_child_as_one_scene_subtree() {
    let config = RenderConfig {
        resolution: (20, 20),
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            transform: Transform2D {
                translation: Position { x: 0.25, y: 0.0 },
                ..Transform2D::default()
            },
            opacity: 1.0,
            children: vec![solid_progress(
                "leaf",
                Position { x: 0.25, y: 0.5 },
                Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            )],
        })],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let png = render_frame_png(&config, 0, 0).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert_eq!(image.get_pixel(10, 10)[0], 255);
    assert_eq!(image.get_pixel(5, 10)[0], 0);
}

#[test]
fn nested_group_transforms_compose_before_leaf_paint() {
    let inner = GroupComponent {
        id: "inner".to_string(),
        enabled: true,
        transform: Transform2D {
            translation: Position { x: 0.25, y: 0.0 },
            ..Transform2D::default()
        },
        opacity: 1.0,
        children: vec![solid_progress(
            "leaf",
            Position { x: 0.0, y: 0.5 },
            Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        )],
    };
    let config = RenderConfig {
        resolution: (20, 20),
        components: vec![SceneComponent::Group(GroupComponent {
            id: "outer".to_string(),
            enabled: true,
            transform: Transform2D {
                translation: Position { x: 0.25, y: 0.0 },
                ..Transform2D::default()
            },
            opacity: 1.0,
            children: vec![SceneComponent::Group(inner)],
        })],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let png = render_frame_png(&config, 0, 0).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert_eq!(image.get_pixel(10, 10)[0], 255);
}

#[test]
fn group_opacity_multiplies_child_layer() {
    let config = RenderConfig {
        resolution: (8, 8),
        background_color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            transform: Transform2D::default(),
            opacity: 0.5,
            children: vec![solid_progress(
                "leaf",
                Position { x: 0.5, y: 0.5 },
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            )],
        })],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };
    let png = render_frame_png(&config, 0, 0).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert!((127..=128).contains(&image.get_pixel(4, 4)[0]));
}

#[test]
fn duplicate_ids_are_rejected_across_group_boundaries() {
    let config = RenderConfig {
        components: vec![
            solid_progress("same", Position::default(), Color::default()),
            SceneComponent::Group(GroupComponent {
                id: "group".to_string(),
                enabled: true,
                children: vec![solid_progress(
                    "same",
                    Position::default(),
                    Color::default(),
                )],
                ..GroupComponent::default()
            }),
        ],
        ..RenderConfig::default()
    };
    assert!(
        format!("{:#}", validate_render_config(&config, false).unwrap_err())
            .contains("Duplicate component ID: same")
    );
}

#[test]
fn group_rejects_zero_scale() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            transform: Transform2D {
                scale: Scale2D { x: 0.0, y: 1.0 },
                ..Transform2D::default()
            },
            ..GroupComponent::default()
        })],
        ..RenderConfig::default()
    };
    assert!(
        format!("{:#}", validate_render_config(&config, false).unwrap_err())
            .contains("scale must contain finite non-zero values")
    );
}

#[test]
fn events_can_target_nested_components() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: true,
            children: vec![solid_progress(
                "nested",
                Position::default(),
                Color::default(),
            )],
            ..GroupComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentPosition {
                element_id: "nested".to_string(),
                position: Position { x: 0.2, y: 0.3 },
            },
        }],
        ..RenderConfig::default()
    };
    validate_render_config(&config, false).unwrap();
}

#[test]
fn legacy_move_component_position_uses_generic_property_state() {
    let config = RenderConfig {
        fps: 10.0,
        components: vec![solid_progress(
            "progress",
            Position { x: 0.1, y: 0.2 },
            Color::default(),
        )],
        events: vec![Event {
            frame: 0,
            event_type: EventType::MoveComponentPosition {
                element_id: "progress".to_string(),
                start_position: Position { x: 0.1, y: 0.2 },
                end_position: Position { x: 0.9, y: 0.8 },
                duration: 1.0,
                curve: AnimationCurve::Linear,
            },
        }],
        ..RenderConfig::default()
    };
    let mut state = initial_render_state(&config);
    let mut color_index = 0;
    process_events(&config, 0, &mut state, &mut color_index, false);
    update_animations(&mut state, 5);
    let states = resolve_component_frame_states(&config, &one_character(), &state);
    let position = &states["progress"].position;
    assert!((position.x - 0.5).abs() < 1e-6);
    assert!((position.y - 0.5).abs() < 1e-6);
}

#[test]
fn generic_progress_property_event_changes_scheduled_frame_fill() {
    let config = RenderConfig {
        resolution: (8, 4),
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            id: "progress".to_string(),
            enabled: true,
            position: Position { x: 0.5, y: 0.5 },
            size: Size {
                width: 1.0,
                height: 1.0,
            },
            progress: 0.0,
            background_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            fill_color: Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            direction: ProgressDirection::LeftToRight,
            border: None,
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentProperty {
                element_id: "progress".to_string(),
                property: AnimatableProperty::Progress,
                value: ComponentPropertyValue::Number(0.5),
            },
        }],
        characters: vec![one_character()],
        ..RenderConfig::default()
    };

    let mut state = initial_render_state(&config);
    let mut color_index = 0;
    let mut entry_index = 0;
    let mut frame_in_char = 0;
    let mut frame_index = 0;
    let job = next_frame_job(
        &config,
        &mut state,
        &mut color_index,
        &mut entry_index,
        &mut frame_in_char,
        &mut frame_index,
    )
    .unwrap();
    let progress_state = job.component_states.get("progress").unwrap();
    assert_eq!(progress_state.progress, Some(0.5));

    let SceneComponent::ProgressBar(progress) = &config.components[0] else {
        panic!("expected progress bar");
    };
    let mut resolved = progress.clone();
    resolved.progress = progress_state.progress.unwrap();
    let mut image = image::RgbaImage::from_pixel(8, 4, image::Rgba([0, 0, 0, 255]));
    crate::primitive_renderer::render_progress_bar(&resolved, &progress_state.position, &mut image);
    assert_eq!(image.get_pixel(1, 2), &image::Rgba([255, 0, 0, 255]));
    assert_eq!(image.get_pixel(6, 2), &image::Rgba([0, 0, 0, 255]));
}

#[test]
fn generic_group_rotation_animation_is_valid() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            children: vec![solid_progress(
                "nested",
                Position::default(),
                Color::default(),
            )],
            ..GroupComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::AnimateComponentProperty {
                element_id: "group".to_string(),
                property: AnimatableProperty::Rotation,
                start_value: ComponentPropertyValue::Number(0.0),
                end_value: ComponentPropertyValue::Number(90.0),
                duration: 1.0,
                curve: AnimationCurve::Linear,
            },
        }],
        ..RenderConfig::default()
    };
    validate_render_config(&config, false).unwrap();
}

#[test]
fn generic_animation_updates_group_rotation_state() {
    let config = RenderConfig {
        fps: 10.0,
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            ..GroupComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::AnimateComponentProperty {
                element_id: "group".to_string(),
                property: AnimatableProperty::Rotation,
                start_value: ComponentPropertyValue::Number(0.0),
                end_value: ComponentPropertyValue::Number(90.0),
                duration: 1.0,
                curve: AnimationCurve::Linear,
            },
        }],
        ..RenderConfig::default()
    };
    let mut state = initial_render_state(&config);
    let mut color_index = 0;
    process_events(&config, 0, &mut state, &mut color_index, false);
    update_animations(&mut state, 5);
    let states = resolve_component_frame_states(&config, &one_character(), &state);
    assert!((states["group"].rotation.unwrap() - 45.0).abs() < 1e-6);
}

#[test]
fn unsupported_component_property_is_rejected() {
    let config = RenderConfig {
        components: vec![SceneComponent::ProgressBar(ProgressBarComponent {
            id: "progress".to_string(),
            ..ProgressBarComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentProperty {
                element_id: "progress".to_string(),
                property: AnimatableProperty::Rotation,
                value: ComponentPropertyValue::Number(45.0),
            },
        }],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("does not support property Rotation"));
}

#[test]
fn generic_property_value_type_is_validated() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            ..GroupComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentProperty {
                element_id: "group".to_string(),
                property: AnimatableProperty::Opacity,
                value: ComponentPropertyValue::Color(Color::default()),
            },
        }],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("requires a numeric value"));
}

#[test]
fn generic_opacity_property_is_range_checked() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            ..GroupComponent::default()
        })],
        events: vec![Event {
            frame: 0,
            event_type: EventType::SetComponentProperty {
                element_id: "group".to_string(),
                property: AnimatableProperty::Opacity,
                value: ComponentPropertyValue::Number(1.5),
            },
        }],
        ..RenderConfig::default()
    };
    let error = validate_render_config(&config, false).unwrap_err();
    assert!(format!("{error:#}").contains("must be between 0 and 1"));
}

#[test]
fn disabled_group_does_not_require_enabled_child_font() {
    let config = RenderConfig {
        components: vec![SceneComponent::Group(GroupComponent {
            id: "group".to_string(),
            enabled: false,
            children: vec![SceneComponent::Glyph(GlyphComponent {
                id: "nested_glyph".to_string(),
                enabled: true,
                fonts: Vec::new(),
                ..GlyphComponent::default()
            })],
            ..GroupComponent::default()
        })],
        ..RenderConfig::default()
    };
    validate_render_config(&config, false).unwrap();
}

#[test]
fn disabled_text_layers_do_not_require_fonts_for_preview() {
    let mut config = RenderConfig {
        resolution: (8, 8),
        characters: vec![CharEntry {
            code_point: "U+0041".to_string(),
            description: String::new(),
            background_color: None,
            text_color: None,
            position: None,
            duration_frames: Some(1),
        }],
        ..Default::default()
    };

    for component in &mut config.components {
        match component {
            crate::scene::SceneComponent::Glyph(component) => {
                component.enabled = false;
                component.fonts.clear();
            }
            crate::scene::SceneComponent::Text(component) => {
                component.enabled = false;
                component.fonts.clear();
            }
            crate::scene::SceneComponent::Image(component) => component.enabled = false,
            crate::scene::SceneComponent::ProgressBar(component) => component.enabled = false,
            crate::scene::SceneComponent::Group(component) => component.enabled = false,
        }
    }

    assert!(!render_frame_png(&config, 0, 8).unwrap().is_empty());
}
