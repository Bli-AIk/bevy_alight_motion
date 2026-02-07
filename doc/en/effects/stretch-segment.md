# Stretch Segment Effect

The **Stretch Segment** effect is a UV-space transformation that stretches an image along a specified axis.

## Mathematical Principle

The effect works by dividing the image into two parts along a split line and inserting a stretched segment in between.

### Stretch Formula
To match Alight Motion's behavior exactly, we use a specific divisor based on the original width:
```rust
base_divisor = original_width / 5.76
stretch_factor = 1.0 + stretch_pixels / base_divisor
new_width = original_width * stretch_factor
```

## Parameters

- **Stretch**: The amount of stretch in pixels.
- **Angle**: The angle of the split line.
- **Offset**: The position of the split line relative to the center.

## Implementation Details

This effect is implemented within the `UnifiedEffectMaterial` shader (`unified_effect.wgsl`). By calculating the transformation in the vertex shader and applying it to UV coordinates in the fragment shader, we achieve a high-performance deformation.

### Bounding Box Calculation
Because the stretch increases the visual size of the layer, we must calculate a precise **AABB (Axis-Aligned Bounding Box)** for the stretched and rotated layer to prevent premature clipping.

## Associated Test Files

| File | Description |
|------|-------------|
| `fx_1_stretch_segment.amproj` | Basic stretch segment test. |
| `fx_1_ex2_stretch_segment.amproj` | Tests different angles and offsets. |
| `fx_1_ex4_stretch_segment.amproj` | Comprehensive test for stretch animation. |

## Implementation Status
- **Stretch Amount**: ✅ Supported
- **Angle/Offset**: ⚠️ Basic support (minor visual differences compared to AM)
- **Smooth**: ❌ Not Yet Implemented
