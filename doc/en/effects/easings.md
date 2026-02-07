# Easings

Keyframe interpolation methods for smooth parameter transitions.

- **Linear**: ✅ Supported
- **Step**: ✅ Supported (Discrete jumps)
- **Cubic Bezier**: ✅ Supported (Custom curves)

**Associated Test Files:**
- `basic_bezier.amproj`
- `basic_bezier_ex.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### Cubic Bezier Solver
Implemented using Newton's method to solve the Bezier equation for `y` given the normalized time `x`.
The implementation can be found in `src/schema/easing.rs`.

### Normalization
All timings are normalized to a `0.0` to `1.0` range within each keyframe segment before interpolation.
</details>