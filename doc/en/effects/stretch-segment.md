# Stretch Segment

UV-space deformation that stretches an image along a specified axis.

- **Stretch**: ✅ Supported (Pixel-based stretch)
- **Angle**: ⚠️ Supported (Minor visual differences)
- **Offset**: ⚠️ Supported (Split line positioning)
- **Smooth**: ❌ Not implemented

**Associated Test Files:**
- `fx_1_stretch_segment.amproj`
- `fx_1_ex2_stretch_segment.amproj`
- `fx_1_ex4_stretch_segment.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Stretching Formula
To match AM's behavior, we use a specific divisor for the stretch factor:
`base_divisor = original_width / 5.76`
`stretch_factor = 1.0 + stretch_pixels / base_divisor`

### Shader Implementation
Implemented in `unified_effect.wgsl`. The vertex shader expands the bounding box, and the fragment shader performs the UV mapping.

### Bounding Box
Precise AABB calculation is performed on the CPU to ensure the expanded layer isn't culled prematurely.
</details>