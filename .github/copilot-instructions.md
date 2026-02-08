# Copilot Instructions for bevy_alight_motion

Bevy plugin for loading and playing Alight Motion (`.amproj`) project files. Uses Bevy 0.17+ and Rust 2024 edition.

## Build, Test, Lint

```bash
# Build
cargo build

# Run tests
cargo test                           # all tests
cargo test test_name                 # single test
cargo nextest run                    # CI uses nextest

# Lint
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Run examples
cargo run --example player -- <project_name>
cargo run --example player --features debug -- <project_name>  # with inspector
```

## Architecture

Three-layer design:

1. **Data Layer** (`src/schema/`): Rust structs for XML deserialization via `quick-xml` + `serde`
   - `types.rs`: AM scene, layers, media definitions
   - `easing.rs`: Keyframe easing (cubic-bezier, step, linear)
   
2. **Resource Layer** (`src/loader.rs`): Asset loading and ZIP extraction
   - `AlightMotionLoader`: Bevy `AssetLoader` for `.amproj`/`.xml` files
   - `AmProject`: Loaded asset containing scene data, images, fonts
   
3. **Runtime Layer**: ECS components and systems
   - `src/plugin.rs`: `AlightMotionPlugin` entry point, system registration
   - `src/animation/`: Playback, interpolation, layer lifecycle
   - `src/scene/`: Entity spawning, coordinate conversion, layer collection
   - `src/effects/`: RTT (render-to-texture), Gaussian blur, masking
   - `src/sdf.rs`, `src/sdf_material.rs`: SDF shape rendering

## Key Conventions

### Module Organization
- Each major module has submodules with focused files (~200-400 lines)
- Public types re-exported in parent module via `pub use`
- Tests in module-level `#[cfg(test)] mod tests` blocks

### Component Patterns
- `Am` prefix for all plugin-specific types: `AmScene`, `AmLayer`, `AmPlayback`, `AmAnimated`
- Components derive: `#[derive(Component, Debug, Clone)]`
- Schema types derive: `#[derive(Debug, Clone, Deserialize)]` with `#[serde(rename = "@attr")]` for XML attributes

### Coordinate System
- Alight Motion uses top-left origin; Bevy uses center origin
- Use `am_to_bevy_coords(x, y, &config)` for conversion
- Y-axis is flipped (`flip_y: true` in `AmSceneConfig`)

### Animation System
- Keyframes use normalized time (0.0-1.0 within segment)
- Easing defined on target keyframe, not source
- `interpolate_float/vec2/vec3` functions for value interpolation
- Layer lifecycle managed by `manage_layer_lifecycle_system`

### Prelude
Import common types via:
```rust
use bevy_alight_motion::prelude::*;
```

### Bilingual Documentation
Code comments include both English and Chinese (简体中文) where present.
