# Jitter

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-07 20:20:21

Applies random position displacement using Simplex noise.

**Support Status**: ✅ Fully Supported

- **Angle (angle)**: ✅ Implemented (Movement direction angle (degrees))
- **Frequency (freq)**: ✅ Implemented (Noise frequency (steps per second))
- **Magnitude (mag)**: ✅ Implemented (Displacement magnitude (pixels))
- **Seed (seed)**: ✅ Implemented (Noise seed value)
- **Slack (slack)**: ✅ Implemented (Perpendicular slack amount (0.0-1.0))
- **Z Jitter (zjitter)**: ✅ Implemented (Z-axis jitter magnitude)

**Related Test Files:**
- `effects/jetter/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.jitter">
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="30.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="seed" type="float" value="0.0" />
    <property name="slack" type="float" value="0.0" />
    <property name="zjitter" type="float" value="0.0" />
</effect>
```
</details>
