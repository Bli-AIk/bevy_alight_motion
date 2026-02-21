# Transform2

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-21 10:09:51

Provides additional transform controls similar to the base transform properties.

**Support Status**: ✅ Fully Supported

- **X Offset (posx)**: ✅ Implemented (Additional horizontal offset)
- **Y Offset (posy)**: ✅ Implemented (Additional vertical offset)
- **Z Offset (posz)**: ✅ Implemented (Scale multiplier (Z axis offset simulation))
- **Angle (angle)**: ✅ Implemented (Additional rotation angle (degrees))
- **X Invert (xinv)**: ❌ Not implemented (Horizontal flip)
- **Y Invert (yinv)**: ❌ Not implemented (Vertical flip)
- **Z Invert (zinv)**: ❌ Not implemented (Scale inversion)
- **Angle Invert (ainv)**: ❌ Not implemented (Angle inversion)

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.transform2" locallyApplied="true">
    <property name="posx" type="float" value="0.0" />
    <property name="posy" type="float" value="0.0" />
    <property name="posz" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="xinv" type="bool" value="false" />
    <property name="yinv" type="bool" value="false" />
    <property name="zinv" type="bool" value="false" />
    <property name="ainv" type="bool" value="false" />
</effect>
```
</details>
