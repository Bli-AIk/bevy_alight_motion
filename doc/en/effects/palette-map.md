# Palette Map

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2025-02-08 14:00:00
> ⚠️ **Warning: Test data is stale (over 1 day(s) old). Please re-run tests.**

Maps image colors to specified palette colors. Supports up to 8 palette colors.

**Support Status**: ✅ Fully Supported

- **Color 1 (color1)**: ✅ Supported (Palette color 1)
- **Color 2 (color2)**: ✅ Supported (Palette color 2)
- **Color 3 (color3)**: ✅ Supported (Palette color 3 (optional))
- **Color 4 (color4)**: ✅ Supported (Palette color 4 (optional))
- **Color 5 (color5)**: ✅ Supported (Palette color 5 (optional))
- **Color 6 (color6)**: ✅ Supported (Palette color 6 (optional))
- **Color 7 (color7)**: ✅ Supported (Palette color 7 (optional))
- **Color 8 (color8)**: ✅ Supported (Palette color 8 (optional))
- **Color Count (count)**: ✅ Supported (Number of colors to use)
- **Shades Mode (shades)**: ⚠️ Basic support (Enable shade gradients (basic support, color transition differs slightly from AM))
- **Alpha (alpha)**: ✅ Supported (Effect blend strength)

**Related Test Files:**
- `fx_5_palette.amproj` ✅

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
