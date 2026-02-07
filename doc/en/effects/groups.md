# Groups & Resolution

Nested scenes with independent dimensions and timelines.

- **Nesting**: ✅ Supported (Multi-level)
- **Clipping**: ✅ Supported (Resolution-based)
- **Time Offset**: ✅ Supported
- **Transform**: ✅ Supported (Group-level movement)

**Associated Test Files:**
- `basic_resolution_group.amproj`
- `basic_multi_level_group.amproj`

---

<details>
<summary>Technical Details & Implementation</summary>

### RTT Architecture
Groups are rendered into a texture using a separate camera and `RenderLayer`.
`EmbedClipMaterial` handles the rectangular clipping.

### The AM Rotation Bug
In AM, rotated groups have incorrect clipping bounds (expanding into a square). `bevy_alight_motion` fixes this by maintaining correct rectangular clipping regardless of rotation.
</details>