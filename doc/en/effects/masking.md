# Layer Masks

Visibility control where one layer's shape acts as a stencil for others.

- **Inclusion Mask**: ✅ Supported (Show inside)
- **Exclusion Mask**: ✅ Supported (Show outside)
- **Multi-Mask**: ✅ Supported (Up to two masks)

**Associated Test Files:**
- `basic_mask_square.amproj`
- `basic_mask_circle.amproj`
- `basic_child_mask.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Fragment Clipping
Masking is performed in the `UnifiedEffectMaterial`. The shader checks if the pixel is inside the mask layer's bounds.

### Coordinate Space
Masks are calculated in the target layer's local space to handle parent transformations correctly.
</details>