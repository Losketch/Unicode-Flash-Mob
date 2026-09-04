use super::{composite_rgba, fit_image};
use crate::scene::ImageFit;
use image::{Rgba, RgbaImage};

#[test]
fn contain_preserves_aspect_ratio_and_centers() {
    let source = RgbaImage::new(200, 100);
    let fitted = fit_image(&source, 100, 100, ImageFit::Contain);
    assert_eq!(fitted.image.dimensions(), (100, 50));
    assert_eq!((fitted.offset_x, fitted.offset_y), (0, 25));
}

#[test]
fn cover_fills_requested_box() {
    let source = RgbaImage::new(200, 100);
    let fitted = fit_image(&source, 100, 100, ImageFit::Cover);
    assert_eq!(fitted.image.dimensions(), (100, 100));
    assert_eq!((fitted.offset_x, fitted.offset_y), (0, 0));
}

#[test]
fn stretch_uses_exact_requested_box() {
    let source = RgbaImage::new(200, 100);
    let fitted = fit_image(&source, 80, 120, ImageFit::Stretch);
    assert_eq!(fitted.image.dimensions(), (80, 120));
    assert_eq!((fitted.offset_x, fitted.offset_y), (0, 0));
}

#[test]
fn image_opacity_is_source_over_composited() {
    let mut target = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
    let source = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
    composite_rgba(&source, &mut target, 0, 0, 0.5);
    let pixel = target.get_pixel(0, 0);
    assert!((127..=128).contains(&pixel[0]));
    assert_eq!(pixel[3], 255);
}
