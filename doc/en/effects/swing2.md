# Swing

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-28 00:52:13

Makes the layer swing back and forth with specified frequency and amplitude.

**Support Status**: ✅ Fully Supported

- **Frequency (freq)**: ✅ Implemented (Oscillation frequency in Hz)
- **Min Angle (a1)**: ✅ Implemented (Minimum oscillation angle (degrees))
- **Max Angle (a2)**: ✅ Implemented (Maximum oscillation angle (degrees))
- **Phase (phase)**: ✅ Implemented (Oscillation phase offset (degrees))
- **Type (type)**: ✅ Implemented (Oscillation waveform type (0=sine, 1=triangle))

**Related Test Files:**
- `effects/swing/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.swing2">
    <property name="freq" type="float" value="1.0" />
    <property name="a1" type="float" value="-30.0" />
    <property name="a2" type="float" value="30.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="type" type="int" value="0" />
</effect>
```
</details>
