# Oscillate

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Makes the layer oscillate periodically along a specified direction using sine/triangle waves.

**Support Status**: ⚠️ Partially Supported

- **Direction Mode (direction)**: ⚠️ Partial (Movement direction mode (0=angle, 1=depth/Z, 2=orbit))
- **Angle (angle)**: ✅ Implemented (Movement direction angle (degrees))
- **Frequency (freq)**: ✅ Implemented (Oscillation frequency (Hz))
- **Magnitude (mag)**: ✅ Implemented (Movement magnitude (pixels))
- **Wave Type (type)**: ✅ Implemented (Wave type (0=sine, 1=triangle))
- **Phase (phase)**: ✅ Implemented (Phase offset)

**Related Test Files:**
- `effects/swing/animation.amproj` ✅
- `effects/swing/multi.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.oscillate3">
    <property name="direction" type="int" value="0" />
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="2.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="type" type="int" value="0" />
    <property name="phase" type="float" value="0.0" />
</effect>
```
</details>
