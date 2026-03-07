# Grid

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-07 20:20:21

Overlays a grid pattern on the layer or punches it out.

**Support Status**: ✅ Fully Supported

- **Position (position)**: ✅ Implemented (Grid offset position)
- **Spacing (spacing)**: ✅ Implemented (Space between grid lines)
- **Width (width)**: ✅ Implemented (Grid line width)
- **Color (color)**: ✅ Implemented (Grid line color)
- **Punch Out (punchout)**: ✅ Implemented (Whether to punch out grid lines from image)
- **Smoothing (smoothing)**: ✅ Implemented (Edge smoothing)
- **Screen Space (screenSpace)**: ✅ Implemented (Use screen coordinates instead of layer coordinates)

**Related Test Files:**
- `effects/grid/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.grid2">
    <property name="position" type="vec2" value="0.0,0.0" />
    <property name="spacing" type="float" value="0.1" />
    <property name="width" type="float" value="0.01" />
    <property name="color" type="color" value="#ff000000" />
    <property name="punchout" type="bool" value="false" />
    <property name="smoothing" type="float" value="0.05" />
    <property name="screenSpace" type="bool" value="false" />
</effect>
```
</details>
