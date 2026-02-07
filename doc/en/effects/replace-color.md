# Replace Color

Swaps a specific source color with a target color within a given tolerance.

- **Old/New Color**: ✅ Supported
- **Threshold**: ✅ Supported (Tolerance)
- **Feather**: ✅ Supported (Edge softening)
- **Lock Luminance**: ✅ Supported

**Associated Test Files:**
- `fx_8_replace_color.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Comparison Logic
We calculate the distance between the pixel color and `oldcolor` in the RGB color space.

### Lock Luminance
When enabled, the target color's luminance is adjusted to match the original pixel's luminance, preserving textures and shading.
</details>