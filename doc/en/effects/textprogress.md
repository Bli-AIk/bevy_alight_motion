# Text Progress

> ⚠️ **This documentation is auto-generated. Do not edit manually.**
> Last tested: 2026-03-01 17:45:43

Displays part of the text as a string, enabling typewriter effects.

**Support Status**: ❌ Not Supported

- **Start (start)**: ✅ Implemented (Text display start position (0-1))
- **End (end)**: ✅ Implemented (Text display end position (0-1))
- **Cursor (cursor)**: ⚠️ Partial (Cursor style (0=none, 1-8=different styles))
- **Blink (blink)**: ⚠️ Partial (Cursor blink)

**Related Test Files:**
- `effects/text-progress/basic.amproj` ❌

---

<details>
<summary>Technical Details</summary>

### XML Example

```xml
<effect id="com.alightcreative.effects.textprogress">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="cursor" type="int" value="0" />
    <property name="blink" type="bool" value="false" />
</effect>
```
</details>
