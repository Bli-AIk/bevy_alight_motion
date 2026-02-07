# Wipe (Cutoff)

Layer visibility transition from edges based on a specified angle.

- **Start/End**: ✅ Supported (0.0 to 1.0 range)
- **Angle**: ✅ Supported (Linear direction)
- **Feather**: ⚠️ Supported (Needs visual calibration)

**Associated Test Files:**
- `basic_cutoff.amproj`
- `showcase.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Shader Logic
Part of `UnifiedEffectMaterial`. It projects the pixel UV onto the direction vector defined by the angle.
`val = dot(uv, vec2(cos(angle), sin(angle)))`
Pixels outside the `[start, end]` range are discarded.

### Calibration
AM's feathering behavior is non-linear. Current implementation uses a linear ramp which might differ slightly in soft edge appearances.
</details>