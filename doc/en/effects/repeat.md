# Repeat

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-22 12:25:09

Creates multiple copies of the layer with cumulative offset, rotation, scale, and alpha transforms.

**Support Status**: ⚠️ Partially Supported

- **Count (count)**: ✅ Implemented (Number of copies to create)
- **Time Offset (time)**: ❌ Not implemented (Time offset between copies (not yet implemented))
- **Offset (offset)**: ✅ Implemented (X,Y offset per copy (pixels))
- **Angle (angle)**: ✅ Implemented (Rotation angle per copy (degrees))
- **Scale (scale)**: ✅ Implemented (Scale multiplier per copy)
- **Alpha (alpha)**: ✅ Implemented (Alpha multiplier per copy)

**Related Test Files:**
- `effects/repeat/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.repeat">
    <property name="count" type="float" value="3.0" />
    <property name="time" type="float" value="0.0" />
    <property name="offset" type="vec2" value="50.0,50.0" />
    <property name="angle" type="float" value="15.0" />
    <property name="scale" type="float" value="0.9" />
    <property name="alpha" type="float" value="0.8" />
</effect>
```
</details>
