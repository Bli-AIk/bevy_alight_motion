# Replace Color

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-21 13:21:45

Replaces a source color with a target color within a given tolerance. Supports sRGB to linear color space conversion and animation keyframes.

**Support Status**: ✅ Fully Supported

- **Old Color (oldcolor)**: ✅ Implemented (The source color to replace)
- **New Color (newcolor)**: ✅ Implemented (The target color to replace with)
- **Threshold (threshold)**: ✅ Implemented (Color matching tolerance)
- **Feather (feather)**: ✅ Implemented (Edge transition softness)
- **Alpha (alpha)**: ✅ Implemented (Effect strength)
- **Lock Luminance (lockluminance)**: ✅ Implemented (Preserve original pixel luminance)

**Related Test Files:**
- `effects/replace-color/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.replacecolor">
    <property name="oldcolor" type="color" value="#ffff0000" />
    <property name="newcolor" type="color" value="#ff00ff00" />
    <property name="threshold" type="float" value="0.1" />
    <property name="feather" type="float" value="0.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="lockluminance" type="bool" value="false" />
</effect>
```
</details>
