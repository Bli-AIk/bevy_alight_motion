# Wipe2

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2025-02-08 14:00:00
> ⚠️ **Warning: Test data is stale (over 1 day(s) old). Please re-run tests.**

Covers rectangular segments from opposite sides of the layer. Use keyframe animation to create wipe transitions.

- **Start (start)**: ✅ Supported (Visible range start point (0.0-1.0))
- **End (end)**: ✅ Supported (Visible range end point (0.0-1.0))
- **Angle (angle)**: ✅ Supported (Wipe direction angle)
- **Feather (feather)**: ⚠️ Basic support (Edge softness (basic support, not yet calibrated))

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

**Computed Support Status**: ❌ Not Supported
</details>
