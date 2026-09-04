# Rust test layout

Tests live beside the Rust crate rather than in production `src/` bodies.

- `unit/`: module-scoped unit tests. Production modules include these files only under `#[cfg(test)]`, so tests can exercise private implementation details without turning them into public API.
- `fixtures/configs/`: runnable schema-v4 fixtures named by the behavior they exercise.
- `fixtures/assets/`: assets used only by tests and runnable fixtures; these are not packaged as runtime application resources.

## Functional config fixtures

- `typography_features.json`: shaping, OpenType features, combining sequences, exact selectors, and font pinning.
- `scene_components.json`: Image, ProgressBar (including border), Glyph, and Text composition.
- `group_transform.json`: nested Group transform and inherited opacity.
- `component_animation.json`: typed component-property animations, including color, progress, opacity, scale, and rotation.

From the repository root, for example:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- validate --config src-tauri/tests/fixtures/configs/component_animation.json --preview
```

## Boundary rule

`src-tauri/tests/` contains only active automated tests and runnable fixtures. Historical investigation notes and one-off diagnostics are kept outside the source and test tree.
