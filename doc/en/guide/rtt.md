# RTT & Effects System

Render-to-Texture (RTT) is used for complex scene composition and group-level effects.

## The RTT Pipeline

1. **Isolation**: Layers within a group are assigned to a specific `RenderLayer`.
2. **Dedicated Camera**: A secondary camera renders only that `RenderLayer` into a `Texture`.
3. **Projection**: The resulting texture is then drawn back into the main scene.

## Unified Effect Shader

For individual layers, we use the `UnifiedEffectMaterial`. This shader is designed to apply multiple effects in a single pass to maximize performance:
- **Masking**
- **Wipe**
- **Stretch Segment**

By combining these, we avoid multiple render passes and minimize GPU state changes.
