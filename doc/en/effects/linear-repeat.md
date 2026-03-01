# Linear Repeat

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Creates copies of the layer arranged in a line with advanced controls for position, offset, rotation, scale, alpha, and color blending.

**Support Status**: ⚠️ Partially Supported

- **Count (count)**: ✅ Implemented (Number of copies to create)
- **Position (position)**: ✅ Implemented (Total displacement from first to last copy)
- **Offset (offset)**: ✅ Implemented (Constant offset applied to all copies)
- **Angle (angle)**: ✅ Implemented (Rotation angle per copy (degrees))
- **Scale (scale)**: ✅ Implemented (Scale multiplier per copy)
- **Alpha (alpha)**: ✅ Implemented (Alpha multiplier per copy)
- **Fill Color (fillColor)**: ✅ Implemented (Fill color for color blending)
- **Blend (blend)**: ✅ Implemented (Amount of fill color blending)
- **Alternate Colors (colorAltCopies)**: ✅ Implemented (Whether to alternate color application)
- **Start (start)**: ⚠️ Partial (Start point of distribution (0-1))
- **End (end)**: ⚠️ Partial (End point of distribution (0-1))
- **Phase (phase)**: ⚠️ Partial (Phase shift for distribution)
- **Ease In (easeIn)**: ⚠️ Partial (Ease-in factor for distribution)
- **Ease Out (easeOut)**: ⚠️ Partial (Ease-out factor for distribution)
- **Overlap (overlap)**: ❌ Not implemented (Overlap factor between copies)
- **Shape (shape)**: ⚠️ Partial (Distribution shape (0=linear))
- **Invert (invert)**: ❌ Not implemented (Whether to invert the effect)
- **Random Order (randomOrder)**: ❌ Not implemented (Whether to randomize copy order)
- **Seed (seed)**: ❌ Not implemented (Random seed)

**Related Test Files:**
- `effects/linear-repeat/basic.amproj` ✅
- `effects/linear-repeat/dual-16-9.amproj` ✅
- `effects/linear-repeat/dual.amproj` ✅
- `effects/linear-repeat/random.amproj` ❌
- `effects/linear-repeat/random_generated1/1.amproj` ❌
- `effects/linear-repeat/random_generated1/2.amproj` ❌
- `effects/linear-repeat/random_generated1/3.amproj` ❌
- `effects/linear-repeat/random_generated2/1.amproj` ✅
- `effects/linear-repeat/random_generated2/2.amproj` ✅
- `effects/linear-repeat/random_generated2/3.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.repeat.line">
    <property name="count" type="float" value="5.0" />
    <property name="position" type="vec2" value="200.0,0.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffff0000" />
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
