use super::{clamped_border_width, render_progress_bar};
use crate::scene::{
    Color, Position, ProgressBarBorder, ProgressBarComponent, ProgressDirection, Size,
};
use image::{Rgba, RgbaImage};

fn component(direction: ProgressDirection) -> ProgressBarComponent {
    ProgressBarComponent {
        id: "progress".to_string(),
        enabled: true,
        position: Position { x: 0.5, y: 0.5 },
        size: Size {
            width: 1.0,
            height: 1.0,
        },
        progress: 0.5,
        background_color: Color {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        },
        fill_color: Color {
            r: 200,
            g: 100,
            b: 50,
            a: 255,
        },
        direction,
        border: None,
    }
}

#[test]
fn left_to_right_progress_fills_first_half() {
    let mut image = RgbaImage::from_pixel(4, 2, Rgba([0, 0, 0, 255]));
    render_progress_bar(
        &component(ProgressDirection::LeftToRight),
        &Position { x: 0.5, y: 0.5 },
        &mut image,
    );
    assert_eq!(image.get_pixel(0, 0), &Rgba([200, 100, 50, 255]));
    assert_eq!(image.get_pixel(1, 0), &Rgba([200, 100, 50, 255]));
    assert_eq!(image.get_pixel(2, 0), &Rgba([10, 20, 30, 255]));
}

#[test]
fn reverse_and_vertical_directions_fill_from_the_requested_edge() {
    let mut horizontal = RgbaImage::from_pixel(4, 2, Rgba([0, 0, 0, 255]));
    render_progress_bar(
        &component(ProgressDirection::RightToLeft),
        &Position { x: 0.5, y: 0.5 },
        &mut horizontal,
    );
    assert_eq!(horizontal.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
    assert_eq!(horizontal.get_pixel(3, 0), &Rgba([200, 100, 50, 255]));

    let mut vertical = RgbaImage::from_pixel(2, 4, Rgba([0, 0, 0, 255]));
    render_progress_bar(
        &component(ProgressDirection::TopToBottom),
        &Position { x: 0.5, y: 0.5 },
        &mut vertical,
    );
    assert_eq!(vertical.get_pixel(0, 0), &Rgba([200, 100, 50, 255]));
    assert_eq!(vertical.get_pixel(0, 3), &Rgba([10, 20, 30, 255]));
}

#[test]
fn bottom_to_top_progress_fills_lower_half() {
    let mut image = RgbaImage::from_pixel(2, 4, Rgba([0, 0, 0, 255]));
    render_progress_bar(
        &component(ProgressDirection::BottomToTop),
        &Position { x: 0.5, y: 0.5 },
        &mut image,
    );
    assert_eq!(image.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
    assert_eq!(image.get_pixel(0, 3), &Rgba([200, 100, 50, 255]));
}

#[test]
fn border_renders_inside_and_on_top_of_fill() {
    let mut component = component(ProgressDirection::LeftToRight);
    component.progress = 1.0;
    component.border = Some(ProgressBarBorder {
        color: Color {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        },
        width: 1,
    });
    let mut image = RgbaImage::from_pixel(6, 4, Rgba([0, 0, 0, 255]));
    render_progress_bar(&component, &Position { x: 0.5, y: 0.5 }, &mut image);
    assert_eq!(image.get_pixel(0, 0), &Rgba([1, 2, 3, 255]));
    assert_eq!(image.get_pixel(2, 2), &Rgba([200, 100, 50, 255]));
}

#[test]
fn border_width_is_clamped_to_the_component_interior() {
    assert_eq!(clamped_border_width(20, 6, 99), 3);
    assert_eq!(clamped_border_width(20, 1, 99), 1);
    assert_eq!(clamped_border_width(20, 0, 99), 0);
}

#[test]
fn transparent_border_does_not_change_existing_progress_pixels() {
    let mut without_border = component(ProgressDirection::LeftToRight);
    without_border.progress = 1.0;
    let mut with_border = without_border.clone();
    with_border.border = Some(ProgressBarBorder {
        color: Color {
            r: 255,
            g: 0,
            b: 0,
            a: 0,
        },
        width: 2,
    });
    let mut expected = RgbaImage::from_pixel(6, 4, Rgba([0, 0, 0, 255]));
    let mut actual = expected.clone();
    let position = Position { x: 0.5, y: 0.5 };
    render_progress_bar(&without_border, &position, &mut expected);
    render_progress_bar(&with_border, &position, &mut actual);
    assert_eq!(actual, expected);
}

#[test]
fn semi_transparent_border_corner_is_blended_once() {
    let mut component = component(ProgressDirection::LeftToRight);
    component.progress = 1.0;
    component.border = Some(ProgressBarBorder {
        color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 128,
        },
        width: 1,
    });
    let mut image = RgbaImage::from_pixel(6, 4, Rgba([0, 0, 0, 255]));
    render_progress_bar(&component, &Position { x: 0.5, y: 0.5 }, &mut image);
    assert_eq!(image.get_pixel(0, 0), &Rgba([100, 50, 25, 255]));
}
