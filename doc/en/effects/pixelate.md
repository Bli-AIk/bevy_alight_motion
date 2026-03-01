# Pixelate

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Reduces image resolution to create a pixelated effect.

**Support Status**: ⚠️ Partially Supported

- **Size (size)**: ✅ Implemented (Pixel size (1-100))
- **Stretch (stretch)**: ✅ Implemented (Pixel stretch ratio)
- **Angle (angle)**: ✅ Implemented (Pixel grid rotation angle)
- **Vignette (vignette)**: ⚠️ Partial (Vignette strength (partial support))
- **Screen Space (screenSpace)**: ⚠️ Partial (Whether to use screen space coordinates (partial support))

**Related Test Files:**
- `basic/bounce/box.amproj` ✅
- `basic/mask/child.amproj` ✅
- `basic/mask/circle.amproj` ✅
- `basic/mask/shape_ex.amproj` ✅
- `basic/shape/ex.amproj` ✅
- `effects/pixelate/basic.amproj` ✅
- `effects/pixelate/bone.amproj` ✅
- `effects/pixelate/bone_single.amproj` ✅
- `effects/pixelate/sprite.amproj` ✅

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
