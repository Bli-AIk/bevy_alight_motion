# Stretch Segment

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-09 23:01:34

UV domain distortion effect that stretches the image along a dividing line. Formula: new_width = orig_width * (1.0 + stretch_px / (orig_width / 5.76))

**Support Status**: ✅ Fully Supported

- **Stretch (stretch)**: ✅ Implemented (Stretch amount (pixels))
- **Angle (angle)**: ✅ Implemented (Dividing line angle (basic support, minor visual differences))
- **Offset (offset)**: ✅ Implemented (Dividing line position offset (basic support, minor visual differences))
- **Smooth (smooth)**: ✅ Implemented (Edge smoothness (not yet implemented))

**Related Test Files:**
- `effects/stretch-segment/basic.amproj` ✅
- `effects/stretch-segment/ex.amproj` ✅
- `effects/stretch-segment/ex2.amproj` ✅
- `effects/stretch-segment/ex3.amproj` ✅
- `effects/stretch-segment/ex4.amproj` ✅
- `effects/stretch-segment/ex5.amproj` ✅

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
