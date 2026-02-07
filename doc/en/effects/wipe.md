# Wipe Effect

The **Wipe** effect (also known as Cutoff) creates a transition by hiding parts of a layer from its sides.

## Parameters

- **Start**: The starting percentage of the visible area (0.0 to 1.0).
- **End**: The ending percentage of the visible area (0.0 to 1.0).
- **Angle**: The direction of the wipe transition.
- **Feather**: Softness of the edge.

## Implementation Details

The Wipe effect is part of our `UnifiedEffectMaterial`. It uses a linear gradient calculation in the fragment shader to determine the alpha value of each pixel.

### Shader Logic
In `unified_effect.wgsl`, we project the pixel's UV coordinates onto the axis defined by the **Angle**. If the projected value is outside the **[Start, End]** range, the pixel is discarded or its alpha is reduced.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_cutoff.amproj` | Tests basic vertical/horizontal wipes. |
| `showcase.amproj` | Uses wipes for complex text transitions. |

## Implementation Status
- **Start/End/Angle**: ✅ Fully Supported
- **Feather**: ⚠️ Basic support (calibration with AM needed)
