# Wipe2

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-19 08:18:36

Covers rectangular segments from opposite sides of the layer. Use keyframe animation to create wipe transitions.

**Support Status**: ✅ Fully Supported

- **Start (start)**: ✅ Implemented (Visible range start point (0.0-1.0))
- **End (end)**: ✅ Implemented (Visible range end point (0.0-1.0))
- **Angle (angle)**: ✅ Implemented (Wipe direction angle)
- **Feather (feather)**: ⚠️ Partial (Edge softness (basic support, not yet calibrated))

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.wipe2">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="feather" type="float" value="0.0" />
</effect>
```
</details>
