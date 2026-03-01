# Path Repeat

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Distributes copies of the layer along a path with tangent alignment, scale, alpha, and easing controls.

**Support Status**: ✅ Fully Supported

- **Count (count)**: ✅ Implemented (Number of copies along the path)
- **Start Position (startPos)**: ✅ Implemented (Path start position (0.0-1.0))
- **End Position (endPos)**: ✅ Implemented (Path end position (0.0-1.0))
- **Path Phase (pathPhase)**: ✅ Implemented (Phase offset along the path)
- **Tangent (tangent)**: ✅ Implemented (Whether copies rotate to follow path tangent)
- **Offset (offset)**: ✅ Implemented (X,Y offset per copy (pixels))
- **Angle (angle)**: ✅ Implemented (Rotation angle per copy (degrees))
- **Scale (scale)**: ✅ Implemented (Scale multiplier per copy)
- **Alpha (alpha)**: ✅ Implemented (Alpha multiplier per copy)
- **Fill Color (fillColor)**: ✅ Implemented (Fill color for alternate copies)
- **Blend (blend)**: ✅ Implemented (Fill color blend amount)

**Related Test Files:**
- `effects/path-repeat/animation.amproj` ✅
- `effects/path-repeat/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.repeat.path">
    <property name="count" type="float" value="3.0" />
    <property name="startPos" type="float" value="0.0" />
    <property name="endPos" type="float" value="1.0" />
    <property name="pathPhase" type="float" value="0.0" />
    <property name="tangent" type="bool" value="false" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#FFFFFFFF" />
    <property name="blend" type="float" value="0.0" />
</effect>
```
</details>
