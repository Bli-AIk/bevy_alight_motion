# Gaussian Blur

Smooth pixel blurring effect to soften layer edges or create depth.

- **Strength**: ✅ Supported (Blur radius in pixels)
- **Animation**: ✅ Supported

**Associated Test Files:**
- `fx_2_gaussian_blur.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Dual-Pass Strategy
To maintain performance, we use a separable Gaussian filter (horizontal pass followed by a vertical pass). This reduces the complexity from O(N²) to O(N).

### Out-of-Bounds Rendering
The blur effect expands the effective rendering area of the layer to account for the "glow" or spread of pixels beyond the original container.
</details>