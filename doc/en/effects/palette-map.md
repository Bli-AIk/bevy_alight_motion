# Palette Map

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-09 14:03:39

Maps image colors to specified palette colors. Supports up to 8 palette colors.

**Support Status**: ❌ Not Supported

- **Color 1 (color1)**: ✅ Implemented (Palette color 1)
- **Color 2 (color2)**: ✅ Implemented (Palette color 2)
- **Color 3 (color3)**: ✅ Implemented (Palette color 3 (optional))
- **Color 4 (color4)**: ✅ Implemented (Palette color 4 (optional))
- **Color 5 (color5)**: ✅ Implemented (Palette color 5 (optional))
- **Color 6 (color6)**: ✅ Implemented (Palette color 6 (optional))
- **Color 7 (color7)**: ✅ Implemented (Palette color 7 (optional))
- **Color 8 (color8)**: ✅ Implemented (Palette color 8 (optional))
- **Color Count (count)**: ❌ Not implemented (Number of colors to use)
- **Shades Mode (shades)**: ✅ Implemented (Enable shade gradients (basic support, color transition differs slightly from AM))
- **Alpha (alpha)**: ✅ Implemented (Effect blend strength)

**Related Test Files:**
- `effects/palette/basic.amproj`

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.palettemap">
    <property name="color1" type="color" value="#ff000000" />
    <property name="color2" type="color" value="#ffffffff" />
    <property name="count" type="float" value="2.0" />
    <property name="shades" type="bool" value="false" />
    <property name="alpha" type="float" value="1.0" />
</effect>
```
</details>
