# Radial Repeat

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Creates multiple copies of the layer along a circular path with configurable radius, sweep angle, scale, and more.

**Support Status**: ✅ Fully Supported

- **Count (count)**: ✅ Implemented (Number of copies)
- **Radius (radius)**: ✅ Implemented (Radius of the circular path)
- **Orientation (orientation)**: ✅ Implemented (Orientation angle of copies)
- **Start Angle (startAngle)**: ✅ Implemented (Start angle (degrees))
- **Sweep (sweep)**: ✅ Implemented (Sweep angle (degrees))
- **Base Scale (baseScale)**: ✅ Implemented (Base scale for all copies)
- **Offset (offset)**: ✅ Implemented (Offset per copy)
- **Angle (angle)**: ✅ Implemented (Rotation angle per copy)
- **Scale (scale)**: ✅ Implemented (Scale per copy)
- **Alpha (alpha)**: ✅ Implemented (Alpha per copy)
- **Fill Color (fillColor)**: ✅ Implemented (Fill color for copies)
- **Blend (blend)**: ✅ Implemented (Color blend amount)
- **Alternate Color (colorAltCopies)**: ✅ Implemented (Alternate copy coloring)
- **Start (start)**: ✅ Implemented (Visibility range start)
- **End (end)**: ✅ Implemented (Visibility range end)
- **Phase (phase)**: ✅ Implemented (Animation phase offset)
- **Ease In (easeIn)**: ✅ Implemented (Ease in amount)
- **Ease Out (easeOut)**: ✅ Implemented (Ease out amount)
- **Overlap (overlap)**: ✅ Implemented (Copy overlap amount)
- **Shape (shape)**: ✅ Implemented (Arrangement shape type)
- **Invert (invert)**: ✅ Implemented (Invert arrangement order)
- **Random Order (randomOrder)**: ✅ Implemented (Randomize arrangement order)
- **Seed (seed)**: ✅ Implemented (Random seed)

**Related Test Files:**
- `effects/radial-repeat/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.repeat.radial">
    <property name="count" type="float" value="5.0" />
    <property name="radius" type="float" value="100.0" />
    <property name="orientation" type="float" value="0.0" />
    <property name="startAngle" type="float" value="0.0" />
    <property name="sweep" type="float" value="360.0" />
    <property name="baseScale" type="float" value="1.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffffffff" />
    <property name="blend" type="float" value="0.0" />
    <property name="colorAltCopies" type="bool" value="false" />
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="easeIn" type="float" value="0.0" />
    <property name="easeOut" type="float" value="0.0" />
    <property name="overlap" type="float" value="0.0" />
    <property name="shape" type="int" value="0" />
    <property name="invert" type="bool" value="false" />
    <property name="randomOrder" type="bool" value="false" />
    <property name="seed" type="float" value="0.0" />
</effect>
```
</details>
