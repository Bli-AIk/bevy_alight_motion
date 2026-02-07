# Transform & Movement

Basic layer transformations mapping Alight Motion values to Bevy.

- **Position**: ✅ Supported (Linear interpolation)
- **Rotation**: ✅ Supported (Z-axis rotation)
- **Scale**: ✅ Supported (Uniform and Non-uniform)
- **Pivot**: ✅ Supported (Anchor point with position compensation)

**Associated Test Files:**
- `basic_pivot.amproj`
- `basic_frame.amproj`
- `basic_bounce_box.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Coordinate System
AM uses a top-left origin with Y increasing downwards. Bevy uses a center origin with Y increasing upwards.
Formula: `bevy_x = am_x - width/2`, `bevy_y = height/2 - am_y`.

### Pivot Compensation
In AM, changing the pivot point doesn't move the layer visually. We replicate this by applying a translation offset calculated from the pivot shift to maintain visual consistency.

### Hierarchy
Layer parenting is implemented using Bevy's native parent-child system, ensuring transformation propagation.
</details>