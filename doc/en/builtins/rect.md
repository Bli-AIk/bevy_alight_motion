# Rectangle

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2025-02-08 14:00:00
> ⚠️ **Warning: Test data is stale (over 1 day(s) old). Please re-run tests.**

Basic rectangle shape, supports SDF and sprite rendering.

- **Size (size)**: ✅ Supported (Width and height of the shape)

**Related Test Files:**
- `basic_shape.amproj` ✅
- `basic_shape_ex.amproj`

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<shape id="1" label="矩形 1" startTime="0" endTime="1000" fillType="color" s=".rect">
    <transform>
        <location value="640.0,480.0,0.0" />
        <rotation value="0.0" />
        <scale value="1.0,1.0" />
        <opacity value="1.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ffff0000" />
</shape>
```

**Computed Support Status**: ⚠️ Partially Supported
</details>
