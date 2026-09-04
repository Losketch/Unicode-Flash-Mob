use image::{Rgba, RgbaImage};

use crate::scene::{Position, Transform2D};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Affine2D {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Default for Affine2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Affine2D {
    pub const fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub const fn translation(x: f64, y: f64) -> Self {
        Self {
            tx: x,
            ty: y,
            ..Self::identity()
        }
    }

    pub const fn scale(x: f64, y: f64) -> Self {
        Self {
            m11: x,
            m22: y,
            m12: 0.0,
            m21: 0.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn rotation_degrees(degrees: f64) -> Self {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        Self {
            m11: cos,
            m12: -sin,
            m21: sin,
            m22: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn then(self, rhs: Self) -> Self {
        // Column-vector convention: self * rhs, so rhs is applied first.
        Self {
            m11: self.m11 * rhs.m11 + self.m12 * rhs.m21,
            m12: self.m11 * rhs.m12 + self.m12 * rhs.m22,
            m21: self.m21 * rhs.m11 + self.m22 * rhs.m21,
            m22: self.m21 * rhs.m12 + self.m22 * rhs.m22,
            tx: self.m11 * rhs.tx + self.m12 * rhs.ty + self.tx,
            ty: self.m21 * rhs.tx + self.m22 * rhs.ty + self.ty,
        }
    }

    pub fn transform_point(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.m11 * x + self.m12 * y + self.tx,
            self.m21 * x + self.m22 * y + self.ty,
        )
    }

    pub fn inverse(self) -> Option<Self> {
        let determinant = self.m11 * self.m22 - self.m12 * self.m21;
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
            return None;
        }
        let inv = 1.0 / determinant;
        let m11 = self.m22 * inv;
        let m12 = -self.m12 * inv;
        let m21 = -self.m21 * inv;
        let m22 = self.m11 * inv;
        Some(Self {
            m11,
            m12,
            m21,
            m22,
            tx: -(m11 * self.tx + m12 * self.ty),
            ty: -(m21 * self.tx + m22 * self.ty),
        })
    }

    pub fn is_identity(self) -> bool {
        const EPS: f64 = 1e-10;
        (self.m11 - 1.0).abs() < EPS
            && self.m12.abs() < EPS
            && self.m21.abs() < EPS
            && (self.m22 - 1.0).abs() < EPS
            && self.tx.abs() < EPS
            && self.ty.abs() < EPS
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SceneRenderContext {
    pub transform: Affine2D,
    pub opacity: f32,
}

impl Default for SceneRenderContext {
    fn default() -> Self {
        Self {
            transform: Affine2D::identity(),
            opacity: 1.0,
        }
    }
}

impl SceneRenderContext {
    pub fn child(self, local: Affine2D, opacity: f32) -> Self {
        Self {
            transform: self.transform.then(local),
            opacity: (self.opacity * opacity).clamp(0.0, 1.0),
        }
    }
}

pub(crate) fn transform_matrix(
    transform: &Transform2D,
    translation: &Position,
    width: u32,
    height: u32,
) -> Affine2D {
    let tx = translation.x * f64::from(width);
    let ty = translation.y * f64::from(height);
    let anchor_x = transform.anchor.x * f64::from(width);
    let anchor_y = transform.anchor.y * f64::from(height);
    Affine2D::translation(tx, ty)
        .then(Affine2D::translation(anchor_x, anchor_y))
        .then(Affine2D::rotation_degrees(transform.rotation))
        .then(Affine2D::scale(transform.scale.x, transform.scale.y))
        .then(Affine2D::translation(-anchor_x, -anchor_y))
}

pub(crate) fn composite_affine(
    source: &RgbaImage,
    target: &mut RgbaImage,
    transform: Affine2D,
    opacity: f32,
) {
    if opacity <= 0.0 {
        return;
    }
    if transform.is_identity() {
        composite_identity(source, target, opacity);
        return;
    }
    let Some(inverse) = transform.inverse() else {
        return;
    };
    let width = target.width();
    let height = target.height();
    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = inverse.transform_point(f64::from(x) + 0.5, f64::from(y) + 0.5);
            let sx = sx - 0.5;
            let sy = sy - 0.5;
            if !(-1.0..=f64::from(source.width())).contains(&sx)
                || !(-1.0..=f64::from(source.height())).contains(&sy)
            {
                continue;
            }
            let sample = bilinear_sample(source, sx, sy);
            blend_pixel(target.get_pixel_mut(x, y), sample, opacity);
        }
    }
}

fn composite_identity(source: &RgbaImage, target: &mut RgbaImage, opacity: f32) {
    let width = source.width().min(target.width());
    let height = source.height().min(target.height());
    for y in 0..height {
        for x in 0..width {
            blend_pixel(target.get_pixel_mut(x, y), *source.get_pixel(x, y), opacity);
        }
    }
}

fn bilinear_sample(image: &RgbaImage, x: f64, y: f64) -> Rgba<u8> {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = (x - f64::from(x0)).clamp(0.0, 1.0) as f32;
    let fy = (y - f64::from(y0)).clamp(0.0, 1.0) as f32;
    let weights = [
        ((x0, y0), (1.0 - fx) * (1.0 - fy)),
        ((x0 + 1, y0), fx * (1.0 - fy)),
        ((x0, y0 + 1), (1.0 - fx) * fy),
        ((x0 + 1, y0 + 1), fx * fy),
    ];
    let mut a = 0.0f32;
    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;
    for ((px, py), weight) in weights {
        if !(0..image.width() as i32).contains(&px) || !(0..image.height() as i32).contains(&py) {
            continue;
        }
        let pixel = image.get_pixel(px as u32, py as u32);
        let alpha = f32::from(pixel[3]) / 255.0;
        a += alpha * weight;
        r += (f32::from(pixel[0]) / 255.0) * alpha * weight;
        g += (f32::from(pixel[1]) / 255.0) * alpha * weight;
        b += (f32::from(pixel[2]) / 255.0) * alpha * weight;
    }
    if a <= f32::EPSILON {
        return Rgba([0, 0, 0, 0]);
    }
    Rgba([
        ((r / a) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g / a) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b / a) * 255.0).round().clamp(0.0, 255.0) as u8,
        (a * 255.0).round().clamp(0.0, 255.0) as u8,
    ])
}

fn blend_pixel(destination: &mut Rgba<u8>, source: Rgba<u8>, opacity: f32) {
    let source_alpha = (f32::from(source[3]) / 255.0) * opacity;
    if source_alpha <= 0.0 {
        return;
    }
    let destination_alpha = f32::from(destination[3]) / 255.0;
    let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if out_alpha <= f32::EPSILON {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }
    for (destination_channel, source_channel) in
        destination.0[..3].iter_mut().zip(source.0[..3].iter())
    {
        let s = f32::from(*source_channel) / 255.0;
        let d = f32::from(*destination_channel) / 255.0;
        *destination_channel = (((s * source_alpha + d * destination_alpha * (1.0 - source_alpha))
            / out_alpha)
            * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    destination[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
#[path = "../tests/unit/scene_transform.rs"]
mod tests;
