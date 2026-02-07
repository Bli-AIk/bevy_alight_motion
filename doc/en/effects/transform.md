# Transform & Movement

Transformations are the foundation of any animation. `bevy_alight_motion` handles position, rotation, scale, and pivot changes, mapping them from AM's coordinate system to Bevy's.

## Coordinate System Mapping

AM and Bevy use different coordinate systems:
- **AM**: Origin at top-left, Y increases downwards.
- **Bevy**: Origin at center (by default), Y increases upwards.

We convert AM coordinates to Bevy coordinates using the following formula:
```rust
bevy_x = am_x - canvas_width / 2.0
bevy_y = canvas_height / 2.0 - am_y
```

## Key Components

### Position (`location`)
Mapped directly to Bevy's `Transform.translation`. The Z-coordinate is used for layer ordering (depth).

### Rotation (`rotation`)
Mapped to `Transform.rotation`. AM typically rotates around the Z-axis in degrees.

### Scale (`scale`)
Mapped to `Transform.scale`. Note that for SDF shapes, non-uniform scaling might be handled within the shader to maintain stroke consistency.

### Pivot (`pivot`)
The pivot point (anchor) determines the center of rotation and scaling. In AM, changing the pivot often involves a position compensation to keep the object appearing in the same place.

## Implementation Details

The transformation logic is primarily handled in `src/animation/systems.rs`, where keyframe values are interpolated and applied to the entity's `Transform`.

### Parent-Child Hierarchy
We leverage Bevy's built-in hierarchy. When an AM layer has a `parent` attribute, the corresponding Bevy entity is made a child of the parent entity, allowing transformations to propagate naturally.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_pivot.amproj` | Tests pivot point offset and position compensation. |
| `basic_frame.amproj` | Tests basic keyframed position and rotation. |
| `basic_bounce_box.amproj` | Tests complex scale and position animation combinations. |

## Implementation Status
- **Position/Rotation/Scale**: ✅ Fully Supported
- **Pivot Compensation**: ✅ Fully Supported
- **3D Rotation**: ❌ Not Yet Implemented
