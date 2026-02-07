# Easings

Keyframe interpolation in Alight Motion supports linear, step, and custom Cubic Bezier curves.

## Cubic Bezier

Alight Motion uses normalized Cubic Bezier curves for smooth transitions. We implement this using Newton's method to solve for `y` given a time `x`.

### Implementation
See `src/schema/easing.rs` for the mathematical implementation.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_bezier.amproj` | Standard cubic bezier curves. |
| `basic_bezier_ex.amproj` | Extended tests for various curve shapes. |

## Status
- **Linear**: ✅ Supported
- **Step**: ✅ Supported
- **Cubic Bezier**: ✅ Supported
