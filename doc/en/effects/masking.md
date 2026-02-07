# Layer Masks

Masking allows one layer to define the visibility of another.

## Implementation

Currently, we support rectangular and elliptical masks via the `UnifiedEffectMaterial`. Complex path-based masks are not yet implemented.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_mask_square.amproj` | Rectangular inclusion masks. |
| `basic_mask_circle.amproj` | Elliptical inclusion masks. |
| `basic_child_mask.amproj` | Masking applied through parent-child hierarchy. |

## Status
- **Rectangle/Ellipse Mask**: ✅ Supported
- **Exclusion Mask**: ✅ Supported
- **Complex Path Mask**: ❌ Not Yet Implemented
