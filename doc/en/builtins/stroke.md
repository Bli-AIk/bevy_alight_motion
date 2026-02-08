# Stroke

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2025-02-08 14:00:00
> ⚠️ **Warning: Test data is stale (over 1 day(s) old). Please re-run tests.**

Shape border stroke. Uses SDF rendering, stroke width stays constant during scale animation.

**Support Status**: ⚠️ Partially Supported

- **Direction (direction)**: ✅ Supported (Stroke direction (centered, inside, outside))
- **Cap Style (cap)**: ✅ Supported (Line cap style)
- **Join Style (join)**: ✅ Supported (Line join style (miter, round, bevel))
- **Color (color)**: ✅ Supported (Stroke color)
- **Width (size)**: ✅ Supported (Stroke width (pixels))

**Related Test Files:**
- `basic_shape.amproj` ✅
- `basic_shape_ex.amproj`

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<shape s=".rect">
    <path-stroke direction="centered" cap="round" join="round">
        <color value="#ff000000" />
        <size value="2.0" />
    </path-stroke>
</shape>
```
</details>
