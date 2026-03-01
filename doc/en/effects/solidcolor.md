# Solid Color

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Overlays a solid color on the content with blend mode and alpha control.

**Support Status**: ⚠️ Partially Supported

- **Color (color)**: ✅ Implemented (Overlay color (RGBA))
- **Alpha (alpha)**: ✅ Implemented (Effect strength (0.0-1.0))
- **Blend Mode (blendMode)**: ⚠️ Partial (Blend mode (0=normal, 1=multiply, 2=screen))

**Related Test Files:**
- `effects/solid-color/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.solidcolor">
    <property name="color" type="color" value="#2D1EF6FF" />
    <property name="alpha" type="float" value="1.0" />
    <property name="blendMode" type="int" value="0" />
</effect>
```
</details>
