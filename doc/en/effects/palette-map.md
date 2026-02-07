# Palette Map

Maps image grayscale values to a custom color palette.

- **Colors (1-8)**: ✅ Supported
- **Count**: ✅ Supported
- **Shades**: ⚠️ Basic support (Gradient interpolation)
- **Alpha**: ✅ Supported (Effect intensity)

**Associated Test Files:**
- `fx_5_palette.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Remapping Logic
Grayscale intensity (0.0 to 1.0) is used as an index into the provided color array.

### Shading
If `Shades` is enabled, we perform linear interpolation between the colors. Currently, there may be slight differences in the gradient ramp compared to AM.
</details>