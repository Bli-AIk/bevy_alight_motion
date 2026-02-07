# SDF Rendering

Signed Distance Fields (SDF) are at the heart of our shape rendering system.

## Advantages

- **Resolution Independence**: Shapes remain perfectly sharp even when zoomed in infinitely.
- **Efficient Strokes**: Borders can be rendered with variable width and different join styles (Round, Miter, Bevel) without extra geometry.
- **Anti-aliasing**: Perfect edges through mathematical calculations in the fragment shader.

## Implementation

We use custom WGSL shaders (`sdf_shape.wgsl`, `stroked_fill_box.wgsl`) that calculate the distance from the current pixel to the shape's boundary.

### Supported Features
- Rectangles with independent corner rounding.
- Circles and ellipses.
- Dynamic stroke width.
