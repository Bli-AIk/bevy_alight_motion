# Gaussian Blur

The Gaussian Blur effect creates a smooth blurring of the layer.

## Implementation

We use a dual-pass (horizontal and vertical) Gaussian blur approach. This is efficient and allows for large blur radii without significant performance impact.

## Parameters
- **Strength**: The radius of the blur in pixels.

## Associated Test Files

| File | Description |
|------|-------------|
| `fx_2_gaussian_blur.amproj` | Basic blur strength and animation. |

## Status
- **Blur Strength**: ✅ Supported
