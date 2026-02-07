# Shapes & Fills

Core visual primitives rendered using Signed Distance Fields (SDF).

- **Rectangle**: ✅ Supported (With corner rounding)
- **Circle/Ellipse**: ✅ Supported
- **Color Fill**: ✅ Supported (Linear RGBA)
- **Media Fill**: ✅ Supported (Image texture)
- **Strokes**: ✅ Supported (Round, Miter, Bevel joins)

**Associated Test Files:**
- `basic_shape.amproj`
- `basic_shape_ex.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### SDF Rendering
Shapes are rendered via `SdfMaterial`. This allows for infinite zoom without pixelation and efficient stroke rendering.

### Stroke Joins
- **Round**: Built-in SDF behavior.
- **Miter/Bevel**: Handled by specialized shaders (`stroked_fill_box_miter.wgsl` / `bevel.wgsl`).

### Media Mapping
For media fills, UVs are calculated relative to the shape bounds.
</details>