use crate::content_template::ContentTemplate;
use crate::font_loader::validate_settings;
use crate::json_config::{EventType, RenderConfig, CURRENT_SCHEMA_VERSION};
use crate::scene::{AnimatableProperty, ComponentPropertyValue, Position, SceneComponent, Size};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

fn validate_position(position: &Position, label: &str) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
        anyhow::bail!("{label} must contain finite coordinates");
    }
    Ok(())
}

fn is_path_like_command(path: &Path) -> bool {
    path.is_absolute() || path.components().count() > 1
}

fn validate_executable_reference(path: &Path, label: &str) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    if is_path_like_command(path) {
        let resolved = crate::json_config::resolve_asset_reference(path);
        if !resolved.is_file() {
            anyhow::bail!("{label} not found: {}", resolved.display());
        }
    }
    Ok(())
}

fn validate_size(size: &Size, label: &str) -> Result<()> {
    if !size.width.is_finite() || !size.height.is_finite() {
        anyhow::bail!("{label} must contain finite values");
    }
    if size.width <= 0.0 || size.height <= 0.0 {
        anyhow::bail!("{label} must be greater than zero");
    }
    Ok(())
}

fn find_component<'a>(components: &'a [SceneComponent], id: &str) -> Option<&'a SceneComponent> {
    for component in components {
        if component.id() == id {
            return Some(component);
        }
        if let Some(found) = find_component(component.children(), id) {
            return Some(found);
        }
    }
    None
}

fn validate_component_property_value(
    component: &SceneComponent,
    property: AnimatableProperty,
    value: &ComponentPropertyValue,
    label: &str,
) -> Result<()> {
    if !component.supports_animatable_property(property) {
        if property == AnimatableProperty::Color {
            anyhow::bail!(
                "{label} targets component {}, which has no single color property",
                component.id()
            );
        }
        anyhow::bail!(
            "{label} targets component {}, which does not support property {:?}",
            component.id(),
            property
        );
    }

    match (property, value) {
        (AnimatableProperty::Color, ComponentPropertyValue::Color(_)) => Ok(()),
        (AnimatableProperty::Color, _) => {
            anyhow::bail!("{label} color property requires a color value")
        }
        (_, ComponentPropertyValue::Color(_)) => {
            anyhow::bail!("{label} property {:?} requires a numeric value", property)
        }
        (property, ComponentPropertyValue::Number(value)) => {
            if !value.is_finite() {
                anyhow::bail!("{label} property {:?} must be finite", property);
            }
            match property {
                AnimatableProperty::ScaleX | AnimatableProperty::ScaleY if *value == 0.0 => {
                    anyhow::bail!("{label} scale property must be non-zero")
                }
                AnimatableProperty::Opacity | AnimatableProperty::Progress
                    if !(0.0..=1.0).contains(value) =>
                {
                    anyhow::bail!("{label} property {:?} must be between 0 and 1", property)
                }
                _ => Ok(()),
            }
        }
    }
}

fn validate_component_tree(
    components: &[SceneComponent],
    parent_enabled: bool,
    component_ids: &mut HashSet<String>,
) -> Result<()> {
    for (index, component) in components.iter().enumerate() {
        let id = component.id().trim();
        if id.is_empty() {
            anyhow::bail!("Component {index} ID must not be empty");
        }
        if !component_ids.insert(id.to_string()) {
            anyhow::bail!("Duplicate component ID: {id}");
        }
        validate_position(component.position(), &format!("component {id} position"))?;
        let effective_enabled = parent_enabled && component.enabled();
        if effective_enabled {
            if let Some(font) = component.font() {
                validate_settings(font)
                    .with_context(|| format!("Invalid font settings for component {id:?}"))?;
            }
            if component.fonts().is_some_and(|fonts| fonts.is_empty()) {
                anyhow::bail!("Component {id} has no font configured");
            }
        }
        match component {
            SceneComponent::Text(text) => {
                if !text.max_width.is_finite() || text.max_width < 0.0 {
                    anyhow::bail!("Component {id} max_width must be finite and non-negative");
                }
            }
            SceneComponent::Image(image) => {
                validate_size(&image.size, &format!("component {id} size"))?;
                if !image.opacity.is_finite() || !(0.0..=1.0).contains(&image.opacity) {
                    anyhow::bail!("Component {id} opacity must be between 0 and 1");
                }
                if effective_enabled {
                    if image.source.as_os_str().is_empty() {
                        anyhow::bail!("Component {id} has no image source configured");
                    }
                    let resolved = crate::json_config::resolve_asset_reference(&image.source);
                    if !resolved.is_file() {
                        anyhow::bail!("Component {id} image not found: {}", resolved.display());
                    }
                }
            }
            SceneComponent::ProgressBar(progress) => {
                validate_size(&progress.size, &format!("component {id} size"))?;
                if !progress.progress.is_finite() || !(0.0..=1.0).contains(&progress.progress) {
                    anyhow::bail!("Component {id} progress must be between 0 and 1");
                }
                if progress
                    .border
                    .as_ref()
                    .is_some_and(|border| border.width == 0)
                {
                    anyhow::bail!(
                        "Component {id} border width must be a positive integer pixel value"
                    );
                }
            }
            SceneComponent::Group(group) => {
                let transform = &group.transform;
                validate_position(
                    &transform.translation,
                    &format!("component {id} translation"),
                )?;
                validate_position(&transform.anchor, &format!("component {id} anchor"))?;
                if !transform.scale.x.is_finite()
                    || !transform.scale.y.is_finite()
                    || transform.scale.x == 0.0
                    || transform.scale.y == 0.0
                {
                    anyhow::bail!("Component {id} scale must contain finite non-zero values");
                }
                if !transform.rotation.is_finite() {
                    anyhow::bail!("Component {id} rotation must be finite");
                }
                if !group.opacity.is_finite() || !(0.0..=1.0).contains(&group.opacity) {
                    anyhow::bail!("Component {id} opacity must be between 0 and 1");
                }
                validate_component_tree(&group.children, effective_enabled, component_ids)?;
            }
            SceneComponent::Glyph(_) => {}
        }
    }
    Ok(())
}

/// Validate a configuration before preview or full video rendering.
///
/// When `require_frames` is false, frame/output/FFmpeg-only requirements are
/// skipped so static preview remains useful while a project is still being edited.
pub(crate) fn validate_render_config(config: &RenderConfig, require_frames: bool) -> Result<()> {
    if config.schema_version > CURRENT_SCHEMA_VERSION {
        anyhow::bail!(
            "Unsupported configuration schema {}; this build supports up to {}",
            config.schema_version,
            CURRENT_SCHEMA_VERSION
        );
    }

    let (width, height) = config.resolution;
    if width == 0 || height == 0 {
        anyhow::bail!("Resolution must be greater than zero");
    }
    if !config.fps.is_finite() || config.fps <= 0.0 {
        anyhow::bail!("FPS must be a finite value greater than zero");
    }
    if require_frames && config.total_frames() == 0 {
        anyhow::bail!("The render contains no frames");
    }
    if require_frames && config.output_path.as_os_str().is_empty() {
        anyhow::bail!("Output path must not be empty");
    }
    if require_frames && config.output_path.is_dir() {
        anyhow::bail!(
            "Output path points to a directory instead of a file: {}",
            config.output_path.display()
        );
    }
    if require_frames && config.output_path.extension().is_none() {
        anyhow::bail!(
            "Output path must include a file extension so FFmpeg can determine the container: {}",
            config.output_path.display()
        );
    }
    if require_frames {
        if let Some(parent) = config
            .output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty() && parent.exists())
            .filter(|parent| !parent.is_dir())
        {
            anyhow::bail!("Output parent is not a directory: {}", parent.display());
        }
        if let Some(music) = config.music_path.as_ref() {
            let resolved = crate::json_config::resolve_asset_reference(music);
            if !resolved.is_file() {
                anyhow::bail!("Music file not found: {}", resolved.display());
            }
        }
    }
    if config.dynamic_background
        && !config.fixed_background
        && config.events.is_empty()
        && config.background_colors.is_empty()
    {
        anyhow::bail!("Dynamic background requires at least one color");
    }

    let mut component_ids = HashSet::new();
    validate_component_tree(&config.components, true, &mut component_ids)?;

    for (name, template) in &config.content_templates {
        if let ContentTemplate::External { executable, .. } = template {
            validate_executable_reference(
                executable,
                &format!("External content template {name:?} executable"),
            )?;
        }
    }

    for (index, entry) in config.characters.iter().enumerate() {
        if let Some(position) = &entry.position {
            validate_position(position, &format!("character {index} position"))?;
        }
    }
    for (index, event) in config.events.iter().enumerate() {
        match &event.event_type {
            EventType::SetComponentPosition {
                element_id,
                position,
            } => {
                if !component_ids.contains(element_id) {
                    anyhow::bail!("event {index} targets unknown component {element_id}");
                }
                validate_position(position, &format!("event {index} position"))?;
            }
            EventType::MoveComponentPosition {
                element_id,
                start_position,
                end_position,
                duration,
                ..
            } => {
                if !component_ids.contains(element_id) {
                    anyhow::bail!("event {index} targets unknown component {element_id}");
                }
                validate_position(start_position, &format!("event {index} start position"))?;
                validate_position(end_position, &format!("event {index} end position"))?;
                if !duration.is_finite() || *duration < 0.0 {
                    anyhow::bail!("event {index} duration must be finite and non-negative");
                }
            }
            EventType::SetComponentColor { element_id, color } => {
                let component =
                    find_component(&config.components, element_id).with_context(|| {
                        format!("event {index} targets unknown component {element_id}")
                    })?;
                validate_component_property_value(
                    component,
                    AnimatableProperty::Color,
                    &ComponentPropertyValue::Color(color.clone()),
                    &format!("event {index}"),
                )?;
            }
            EventType::SetComponentProperty {
                element_id,
                property,
                value,
            } => {
                let component =
                    find_component(&config.components, element_id).with_context(|| {
                        format!("event {index} targets unknown component {element_id}")
                    })?;
                validate_component_property_value(
                    component,
                    *property,
                    value,
                    &format!("event {index}"),
                )?;
            }
            EventType::AnimateComponentProperty {
                element_id,
                property,
                start_value,
                end_value,
                duration,
                ..
            } => {
                let component =
                    find_component(&config.components, element_id).with_context(|| {
                        format!("event {index} targets unknown component {element_id}")
                    })?;
                validate_component_property_value(
                    component,
                    *property,
                    start_value,
                    &format!("event {index} start value"),
                )?;
                validate_component_property_value(
                    component,
                    *property,
                    end_value,
                    &format!("event {index} end value"),
                )?;
                if !duration.is_finite() || *duration < 0.0 {
                    anyhow::bail!("event {index} duration must be finite and non-negative");
                }
            }
            EventType::ColorTransition { duration, .. } | EventType::Pause { duration } => {
                if !duration.is_finite() || *duration < 0.0 {
                    anyhow::bail!("event {index} duration must be finite and non-negative");
                }
            }
            EventType::SetBackgroundColor { .. } => {}
        }
    }

    if require_frames {
        validate_executable_reference(&config.ffmpeg.path, "FFmpeg executable")?;
        if config.ffmpeg.encoder.trim().is_empty() {
            anyhow::bail!("FFmpeg encoder must not be empty");
        }
        if config.ffmpeg.preset.trim().is_empty() {
            anyhow::bail!("FFmpeg preset must not be empty");
        }
        if config.ffmpeg.pixel_format.trim().is_empty() {
            anyhow::bail!("FFmpeg pixel format must not be empty");
        }
        if config.ffmpeg.crf > 51 {
            anyhow::bail!("FFmpeg CRF must be between 0 and 51");
        }
        if config.ffmpeg.max_inflight_frames == 0 {
            anyhow::bail!("FFmpeg max_inflight_frames must be at least 1");
        }
        if config.ffmpeg.encoding_processes == 0 {
            anyhow::bail!("FFmpeg encoding_processes must be at least 1");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/config_validation.rs"]
mod tests;
