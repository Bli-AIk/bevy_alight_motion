# Architecture Overview

`bevy_alight_motion` is built on a data-driven architecture that maps Alight Motion's XML schema to Bevy's ECS.

## Core Pipeline

1. **Loading**: The `.amproj` (a ZIP file) is decompressed. The `scene.xml` is parsed into a Rust schema.
2. **Processing**: Media assets (images, fonts) are registered with Bevy's `AssetServer`.
3. **Spawning**: Layers are converted into Bevy entities. Parent-child relationships are established using `commands.spawn().set_parent()`.
4. **Animation**: A dedicated system interpolates keyframes every frame and updates `Transform` and material properties.
5. **Rendering**: Custom shaders handle specialized AM effects like Wipe, Stretch, and SDF shapes.
