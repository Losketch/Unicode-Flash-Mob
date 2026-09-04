use crate::scene::{ImageComponent, ImageFit, Position};
use anyhow::{Context, Result};
use image::imageops::{crop_imm, resize, FilterType};
use image::RgbaImage;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn load_image(path: &Path) -> Result<Arc<RgbaImage>> {
    let image = image::open(path)
        .with_context(|| format!("Failed to decode image: {}", path.display()))?
        .into_rgba8();
    Ok(Arc::new(image))
}

pub(crate) fn render_image_component(
    component: &ImageComponent,
    position: &Position,
    source: &RgbaImage,
    target: &mut RgbaImage,
) {
    let box_width = normalized_extent(component.size.width, target.width());
    let box_height = normalized_extent(component.size.height, target.height());
    if box_width == 0 || box_height == 0 || source.width() == 0 || source.height() == 0 {
        return;
    }

    let center_x = position.x * f64::from(target.width());
    let center_y = position.y * f64::from(target.height());
    let box_left = (center_x - f64::from(box_width) / 2.0).round() as i32;
    let box_top = (center_y - f64::from(box_height) / 2.0).round() as i32;
    let fitted = fit_image(source, box_width, box_height, component.fit);
    composite_rgba(
        &fitted.image,
        target,
        box_left + fitted.offset_x,
        box_top + fitted.offset_y,
        component.opacity,
    );
}

struct FittedImage {
    image: RgbaImage,
    offset_x: i32,
    offset_y: i32,
}

fn fit_image(source: &RgbaImage, box_width: u32, box_height: u32, fit: ImageFit) -> FittedImage {
    match fit {
        ImageFit::Stretch => FittedImage {
            image: resize(source, box_width, box_height, FilterType::Lanczos3),
            offset_x: 0,
            offset_y: 0,
        },
        ImageFit::Contain => {
            let scale = (box_width as f64 / source.width() as f64)
                .min(box_height as f64 / source.height() as f64);
            let width = (source.width() as f64 * scale)
                .round()
                .clamp(1.0, box_width as f64) as u32;
            let height = (source.height() as f64 * scale)
                .round()
                .clamp(1.0, box_height as f64) as u32;
            FittedImage {
                image: resize(source, width, height, FilterType::Lanczos3),
                offset_x: ((box_width - width) / 2) as i32,
                offset_y: ((box_height - height) / 2) as i32,
            }
        }
        ImageFit::Cover => {
            let scale = (box_width as f64 / source.width() as f64)
                .max(box_height as f64 / source.height() as f64);
            let width = (source.width() as f64 * scale).ceil().max(1.0) as u32;
            let height = (source.height() as f64 * scale).ceil().max(1.0) as u32;
            let resized = resize(source, width, height, FilterType::Lanczos3);
            let crop_x = width.saturating_sub(box_width) / 2;
            let crop_y = height.saturating_sub(box_height) / 2;
            FittedImage {
                image: crop_imm(&resized, crop_x, crop_y, box_width, box_height).to_image(),
                offset_x: 0,
                offset_y: 0,
            }
        }
    }
}

fn normalized_extent(value: f64, canvas_extent: u32) -> u32 {
    (value * f64::from(canvas_extent)).round().max(0.0) as u32
}

fn composite_rgba(
    source: &RgbaImage,
    target: &mut RgbaImage,
    dest_x: i32,
    dest_y: i32,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return;
    }

    for (source_x, source_y, source_pixel) in source.enumerate_pixels() {
        let target_x = dest_x + source_x as i32;
        let target_y = dest_y + source_y as i32;
        if target_x < 0
            || target_y < 0
            || target_x >= target.width() as i32
            || target_y >= target.height() as i32
        {
            continue;
        }
        let source_alpha = (f32::from(source_pixel[3]) / 255.0) * opacity;
        if source_alpha <= 0.0 {
            continue;
        }
        let destination = target.get_pixel_mut(target_x as u32, target_y as u32);
        let destination_alpha = f32::from(destination[3]) / 255.0;
        let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
        if output_alpha <= f32::EPSILON {
            *destination = image::Rgba([0, 0, 0, 0]);
            continue;
        }
        let blend_channel = |source: u8, destination: u8| -> u8 {
            let source_value = f32::from(source) / 255.0;
            let destination_value = f32::from(destination) / 255.0;
            let output = (source_value * source_alpha
                + destination_value * destination_alpha * (1.0 - source_alpha))
                / output_alpha;
            (output * 255.0).round().clamp(0.0, 255.0) as u8
        };
        destination[0] = blend_channel(source_pixel[0], destination[0]);
        destination[1] = blend_channel(source_pixel[1], destination[1]);
        destination[2] = blend_channel(source_pixel[2], destination[2]);
        destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
#[path = "../tests/unit/image_renderer.rs"]
mod tests;
