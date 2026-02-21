# Circle

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-02-21 16:56:05

Basic circle shape using SDF rendering. Supports non-uniform scaling for ellipses.

**Support Status**: ✅ Fully Supported

- **Size (size)**: ✅ Supported (Width and height of the circle (non-uniform values create ellipse))

**Related Test Files:**
- `basic/shape/shape.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<shape id="1" label="圆形 1" startTime="0" endTime="1000" fillType="color" s=".circle">
    <transform>
        <location value="640.0,480.0,0.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ff00ff00" />
</shape>
```
</details>
