use crate::scene::{Color, Position, ProgressBarComponent, ProgressDirection};
use image::{Rgba, RgbaImage};

pub(crate) fn render_progress_bar(
    component: &ProgressBarComponent,
    position: &Position,
    target: &mut RgbaImage,
) {
    let width = normalized_extent(component.size.width, target.width());
    let height = normalized_extent(component.size.height, target.height());
    if width == 0 || height == 0 {
        return;
    }

    let center_x = position.x * f64::from(target.width());
    let center_y = position.y * f64::from(target.height());
    let left = (center_x - f64::from(width) / 2.0).round() as i32;
    let top = (center_y - f64::from(height) / 2.0).round() as i32;
    fill_rect(
        target,
        left,
        top,
        width,
        height,
        &component.background_color,
    );

    let progress = component.progress.clamp(0.0, 1.0);
    let (fill_left, fill_top, fill_width, fill_height) = match component.direction {
        ProgressDirection::LeftToRight => (left, top, scaled_extent(width, progress), height),
        ProgressDirection::RightToLeft => {
            let fill_width = scaled_extent(width, progress);
            (
                left + width as i32 - fill_width as i32,
                top,
                fill_width,
                height,
            )
        }
        ProgressDirection::TopToBottom => (left, top, width, scaled_extent(height, progress)),
        ProgressDirection::BottomToTop => {
            let fill_height = scaled_extent(height, progress);
            (
                left,
                top + height as i32 - fill_height as i32,
                width,
                fill_height,
            )
        }
    };
    if fill_width > 0 && fill_height > 0 {
        fill_rect(
            target,
            fill_left,
            fill_top,
            fill_width,
            fill_height,
            &component.fill_color,
        );
    }

    if let Some(border) = &component.border {
        stroke_rect_inside(
            target,
            left,
            top,
            width,
            height,
            border.width,
            &border.color,
        );
    }
}

fn normalized_extent(value: f64, canvas_extent: u32) -> u32 {
    (value * f64::from(canvas_extent)).round().max(0.0) as u32
}

fn scaled_extent(extent: u32, progress: f64) -> u32 {
    (f64::from(extent) * progress)
        .round()
        .clamp(0.0, f64::from(extent)) as u32
}

fn fill_rect(target: &mut RgbaImage, left: i32, top: i32, width: u32, height: u32, color: &Color) {
    if color.a == 0 {
        return;
    }
    let right = left.saturating_add(width as i32);
    let bottom = top.saturating_add(height as i32);
    let clipped_left = left.max(0) as u32;
    let clipped_top = top.max(0) as u32;
    let clipped_right = right.min(target.width() as i32).max(0) as u32;
    let clipped_bottom = bottom.min(target.height() as i32).max(0) as u32;
    if clipped_left >= clipped_right || clipped_top >= clipped_bottom {
        return;
    }

    for y in clipped_top..clipped_bottom {
        for x in clipped_left..clipped_right {
            blend_pixel(target.get_pixel_mut(x, y), color);
        }
    }
}

fn clamped_border_width(width: u32, height: u32, requested: u32) -> u32 {
    let min_extent = width.min(height);
    if min_extent == 0 {
        return 0;
    }
    requested.min((min_extent / 2).max(1))
}

fn stroke_rect_inside(
    target: &mut RgbaImage,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    requested_width: u32,
    color: &Color,
) {
    if color.a == 0 {
        return;
    }
    let border_width = clamped_border_width(width, height, requested_width);
    if border_width == 0 {
        return;
    }

    let right = left.saturating_add(width as i32);
    let bottom = top.saturating_add(height as i32);
    let clipped_left = left.max(0) as u32;
    let clipped_top = top.max(0) as u32;
    let clipped_right = right.min(target.width() as i32).max(0) as u32;
    let clipped_bottom = bottom.min(target.height() as i32).max(0) as u32;
    if clipped_left >= clipped_right || clipped_top >= clipped_bottom {
        return;
    }

    for y in clipped_top..clipped_bottom {
        let local_y = (y as i32 - top) as u32;
        for x in clipped_left..clipped_right {
            let local_x = (x as i32 - left) as u32;
            let is_border = local_x < border_width
                || local_x >= width.saturating_sub(border_width)
                || local_y < border_width
                || local_y >= height.saturating_sub(border_width);
            if is_border {
                blend_pixel(target.get_pixel_mut(x, y), color);
            }
        }
    }
}

fn blend_pixel(destination: &mut Rgba<u8>, source: &Color) {
    let source_alpha = f32::from(source.a) / 255.0;
    let destination_alpha = f32::from(destination[3]) / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= f32::EPSILON {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }
    let blend_channel = |source: u8, destination: u8| -> u8 {
        let source_value = f32::from(source) / 255.0;
        let destination_value = f32::from(destination) / 255.0;
        let output = (source_value * source_alpha
            + destination_value * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
        (output * 255.0).round().clamp(0.0, 255.0) as u8
    };
    destination[0] = blend_channel(source.r, destination[0]);
    destination[1] = blend_channel(source.g, destination[1]);
    destination[2] = blend_channel(source.b, destination[2]);
    destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
#[path = "../tests/unit/primitive_renderer.rs"]
mod tests;
