# Stretch Segment

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-07 20:20:21

UV domain distortion effect that stretches the image along a dividing line. Uses AM's native scene-normalized coordinate formula with multi-effect stacking support.

**Support Status**: ✅ Fully Supported

- **Stretch (stretch)**: ✅ Implemented (Stretch amount (pixels))
- **Angle (angle)**: ✅ Implemented (Dividing line angle (degrees), uses scene-normalized coordinate system)
- **Offset (offset)**: ✅ Implemented (Dividing line position offset (scene-normalized: offset/1000))
- **Smooth (smooth)**: ✅ Implemented (Edge smoothness (implemented via smin_cubic))

**Related Test Files:**
- `effects/stretch-segment/basic.amproj` ✅
- `effects/stretch-segment/ex.amproj` ✅
- `effects/stretch-segment/ex2.amproj` ✅
- `effects/stretch-segment/ex3.amproj` ✅
- `effects/stretch-segment/ex4.amproj` ✅
- `effects/stretch-segment/ex5.amproj` ✅
- `effects/stretch-segment/muti/basic.amproj` ✅

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
