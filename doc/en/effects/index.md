# Effects Overview

`bevy_alight_motion` implements a wide range of Alight Motion effects, ranging from basic transformations to complex GPU-accelerated shaders.

## Feature Categories

### [Basic Features](./transform)
Core animation properties that apply to almost every layer.
- **Transform & Movement**: Position, Rotation, Scale, and Pivot.
- **Shapes & Fills**: SDF-based rendering for rectangles and circles.
- **Groups & Resolution**: Nested scenes and clipping.
- **Easings**: Linear, Step, and Cubic Bezier interpolation.

### [Advanced Effects](./wipe)
Specialized visual modifications implemented via custom shaders.
- **Wipe**: Linear transitions and cutoffs.
- **Gaussian Blur**: Efficient multi-pass blurring.
- **Stretch Segment**: Axis-aligned UV deformation.
- **Palette Map**: Grayscale to color mapping.
- **Replace Color**: Targeted color swapping.
- **Scale Assist**: Automatic dimension adjustment.

### [Masking](./masking)
Visibility control using layer shapes.
- **Layer Masks**: Inclusion and exclusion masking.

---

For a complete list of test files and their implementation status, check the [Examples Gallery](../examples/).
