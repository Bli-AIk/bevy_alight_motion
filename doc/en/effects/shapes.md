# Shapes & Fills

Shapes are the primary visual building blocks in Alight Motion. We support rectangles and circles with various fill and stroke options.

## SDF Rendering

For high-quality rendering, we use **Signed Distance Fields (SDF)**. This allows shapes to remain crisp at any scale and enables efficient rendering of strokes and rounded corners.

### Supported Shapes
- **Rectangles**: Mapped to `SdfShape::Rect`.
- **Circles**: Mapped to `SdfShape::Circle`.
- **Ellipses**: Rendered as circles with non-uniform scaling applied.

## Fills

We support two main fill types:
1. **Color Fill**: A solid linear RGBA color.
2. **Media Fill**: An image texture (PNG/JPG) mapped to the shape.

## Strokes (Borders)

Strokes are rendered using the SDF material. We support multiple join types:
- **Round**: Smooth rounded corners.
- **Miter**: Sharp pointed corners.
- **Bevel**: Flat clipped corners.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_shape.amproj` | Tests basic rectangles, circles, color fills, and media fills. |
| `basic_shape_ex.amproj` | Extended tests for different stroke widths and join types. |
| `basic_gradient.amproj` | (WIP) Placeholder for testing gradient fills. |

## Implementation Status
- **Rectangle/Circle**: ✅ Fully Supported
- **Color Fill**: ✅ Fully Supported
- **Media Fill**: ✅ Fully Supported
- **Strokes (Round/Miter/Bevel)**: ✅ Fully Supported
- **Gradients**: ❌ Not Yet Implemented
