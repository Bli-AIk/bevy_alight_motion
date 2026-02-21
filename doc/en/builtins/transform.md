# Transform

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-21 16:56:05

Basic transform properties for layers including position, rotation, scale, opacity and anchor. Coordinate system: AM uses top-left origin, Bevy uses center origin, the library converts automatically.

**Support Status**: ✅ Fully Supported

- **Location (location)**: ✅ Supported (Layer position (x, y, z))
- **Rotation (rotation)**: ✅ Supported (Z-axis rotation angle (degrees))
- **Scale (scale)**: ✅ Supported (Scale factor (x, y))
- **Opacity (opacity)**: ✅ Supported (Opacity (0.0-1.0))
- **Pivot (pivot)**: ✅ Supported (Anchor point for rotation and scaling)
- **Lock Aspect Ratio (lockAspectRatio)**: ✅ Supported (Whether to lock aspect ratio)

**Related Test Files:**
- `basic/shape/shape.amproj` ✅
- `basic/pivot/pivot.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<transform lockAspectRatio="false">
    <location value="640.0,480.0,0.0" />
    <pivot value="0.0,0.0" />
    <rotation value="45.0" />
    <scale value="1.5,1.5" />
    <opacity value="0.8" />
</transform>
```
</details>
