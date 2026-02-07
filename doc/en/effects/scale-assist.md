# Scale Assist

Automatically adjusts a layer's scale to match the canvas dimensions along a specific axis.

- **Axis**: ✅ Supported (Horizontal or Vertical)
- **Automatic Fit**: ✅ Supported

**Associated Test Files:**
- `fx_6_scaleassist.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Logic
Scale Assist calculates the ratio between the layer's source size and the project's target resolution. It then applies this ratio as a scale factor to the chosen axis.

### Use Case
Commonly used to ensure background images or UI frames cover the screen correctly regardless of the source asset resolution.
</details>