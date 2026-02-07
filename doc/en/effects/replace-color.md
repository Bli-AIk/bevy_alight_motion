# Replace Color

This effect swaps a specific source color with a target color.

## Implementation

Implemented in the shader, it compares the current pixel's color with the `oldcolor` parameter. If it's within the `threshold`, it blends towards the `newcolor`.

## Associated Test Files

| File | Description |
|------|-------------|
| `fx_8_replace_color.amproj` | Comprehensive test for replacement, threshold, and feathering. |

## Status
- **Basic Replacement**: ✅ Supported
- **Threshold/Feather**: ✅ Supported
- **Lock Luminance**: ✅ Supported
