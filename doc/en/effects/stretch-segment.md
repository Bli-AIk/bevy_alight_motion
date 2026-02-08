# Stretch Segment

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-08 14:15:04

UV domain distortion effect that stretches the image along a dividing line. Formula: new_width = orig_width * (1.0 + stretch_px / (orig_width / 5.76))

**Support Status**: ⚠️ Partially Supported

- **Stretch (stretch)**: ✅ Supported (Stretch amount (pixels))
- **Angle (angle)**: ⚠️ Basic support (Dividing line angle (basic support, minor visual differences))
- **Offset (offset)**: ⚠️ Basic support (Dividing line position offset (basic support, minor visual differences))
- **Smooth (smooth)**: ❌ Not implemented (Edge smoothness (not yet implemented))

**Related Test Files:**
- `fx_1_stretch_segment.amproj` ❌
- `fx_1_ex_stretch_segment.amproj` ❌
- `fx_1_ex2_stretch_segment.amproj` ❌
- `fx_1_ex3_stretch_segment.amproj` ❌
- `fx_1_ex4_stretch_segment.amproj` ✅
- `fx_1_ex5_stretch_segment.amproj` ❌

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.stretchsegment">
    <property name="stretch" type="float" value="0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="offset" type="float" value="0.0" />
    <property name="smooth" type="float" value="0.0" />
</effect>
```
</details>
