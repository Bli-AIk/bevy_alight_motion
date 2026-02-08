# Media Fill

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2025-02-08 14:00:00
> ⚠️ **Warning: Test data is stale (over 1 day(s) old). Please re-run tests.**

Fills the shape with an image texture. Supports JPEG and PNG formats.

**Support Status**: ✅ Fully Supported

- **Fill Image (fillImage)**: ✅ Supported (Image resource URI (amproj:filename.png))

**Related Test Files:**
- `basic_shape.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<shape fillType="media" fillImage="amproj:image.png">
    <property name="size" type="vec2" value="100.0,100.0" />
</shape>
```
</details>
