# Pixelate

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-10 00:19:13

Reduces image resolution to create a pixelated effect.

**Support Status**: ⚠️ Partially Supported

- **Size (size)**: ❌ Not implemented (Pixel size (1-100))
- **Stretch (stretch)**: ❌ Not implemented (Pixel stretch ratio)
- **Angle (angle)**: ❌ Not implemented (Pixel grid rotation angle)
- **Vignette (vignette)**: ❌ Not implemented (Vignette strength (partial support))
- **Screen Space (screenSpace)**: ❌ Not implemented (Whether to use screen space coordinates (partial support))

**Related Test Files:**
- `basic/bounce/box.amproj` ✅
- `basic/mask/child.amproj` ✅
- `basic/mask/circle.amproj` ✅
- `basic/shape/ex.amproj` ✅
- `effects/pixelate/basic.amproj` ⏭️

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.pixelate2">
    <property name="size" type="float" value="10.0" />
    <property name="stretch" type="vec2" value="1.0,1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="vignette" type="float" value="0.0" />
    <property name="screenSpace" type="boolean" value="false" />
    <property name="threshold" type="float" value="0.5" />
    <property name="saturation" type="float" value="1.0" />
</effect>
```
</details>
