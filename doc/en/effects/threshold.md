# Threshold

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-10 00:19:13

Converts the image to a high-contrast image with only black and white.

**Support Status**: ✅ Fully Supported

- **Threshold (threshold)**: ✅ Implemented (Luminance cutoff)
- **Feather (feather)**: ✅ Implemented (Edge softness)
- **Invert (invert)**: ✅ Implemented (Invert the effect)
- **Blend Mode (blendMode)**: ✅ Implemented (Blend mode)

**Related Test Files:**
- `effects/threshold/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.threshold">
    <property name="threshold" type="float" value="0.5" />
    <property name="feather" type="float" value="0.0" />
    <property name="invert" type="bool" value="false" />
    <property name="blendMode" type="int" value="0" />
</effect>
```
</details>
