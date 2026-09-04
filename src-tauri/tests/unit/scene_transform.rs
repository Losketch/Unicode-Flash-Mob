use super::{transform_matrix, Affine2D, SceneRenderContext};
use crate::scene::{Position, Scale2D, Transform2D};

#[test]
fn affine_parent_times_child_composes_in_scene_order() {
    let parent = Affine2D::translation(10.0, 0.0).then(Affine2D::rotation_degrees(90.0));
    let child = Affine2D::translation(5.0, 0.0);
    let (x, y) = parent.then(child).transform_point(0.0, 0.0);
    assert!((x - 10.0).abs() < 1e-8);
    assert!((y - 5.0).abs() < 1e-8);
}

#[test]
fn transform_rotates_around_normalized_anchor() {
    let transform = Transform2D {
        translation: Position { x: 0.0, y: 0.0 },
        scale: Scale2D { x: 1.0, y: 1.0 },
        rotation: 90.0,
        anchor: Position { x: 0.5, y: 0.5 },
    };
    let matrix = transform_matrix(&transform, &transform.translation, 100, 100);
    let (x, y) = matrix.transform_point(100.0, 50.0);
    assert!((x - 50.0).abs() < 1e-8);
    assert!((y - 100.0).abs() < 1e-8);
}

#[test]
fn nested_opacity_multiplies() {
    let root = SceneRenderContext::default();
    let nested = root
        .child(Affine2D::identity(), 0.5)
        .child(Affine2D::identity(), 0.8);
    assert!((nested.opacity - 0.4).abs() < f32::EPSILON);
}
