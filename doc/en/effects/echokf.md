# Echo Keyframes

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Creates time-shifted echo copies of an element with keyframe control over timing, count, and alpha.

**Support Status**: ✅ Fully Supported

- **Seconds (seconds)**: ✅ Implemented (Time spacing per echo copy (seconds))
- **Count (count)**: ✅ Implemented (Number of echo copies)
- **Alpha (alpha)**: ✅ Implemented (Echo fade alpha curve)
- **Composite Mode (mode)**: ✅ Implemented (Composite mode (0=atop, 1=behind))

**Related Test Files:**
- `effects/echo-keyframes/basic.amproj` ✅

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.repeat.echokf">
    <property name="seconds" type="float" value="0.5" />
    <property name="count" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="mode" type="int" value="1" />
</effect>
```
</details>
