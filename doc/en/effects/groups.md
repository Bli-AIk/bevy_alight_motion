# Groups & Resolution

In Alight Motion, Groups (represented by `<embedScene>`) are nested scenes with their own dimensions and internal timelines.

## Embedded Scenes

An `embedScene` acts as a container. It has a `width` and `height` that define its internal canvas size. Any content inside the group that extends beyond these dimensions should be clipped.

### Time Offset
Groups can have a `timeOffset` attribute, which shifts the playback of the internal scene relative to the main timeline.

## Implementation: RTT (Render-to-Texture)

To handle group clipping and effects applied to entire groups, we use a Render-to-Texture (RTT) approach:
1. Each group is rendered by a dedicated camera into a texture.
2. The texture is then displayed in the parent scene using a `Sprite` or a custom material.
3. Clipping is enforced by the size of the render target and the `EmbedClipMaterial`.

## The "Rotation Bug" in AM

During development, we discovered a bug in Alight Motion regarding group clipping:
- **Expected Behavior**: Content outside the group's rectangular bounds should always be clipped.
- **AM Bug**: When a group is rotated near multiples of 45 degrees, the clipping area incorrectly expands into a larger square region.

**Our Policy**: We consider this a bug in AM and do not replicate it. In `bevy_alight_motion`, group clipping is always correctly enforced according to the defined resolution, regardless of rotation.

## Associated Test Files

| File | Description |
|------|-------------|
| `basic_resolution_group.amproj` | Tests basic group resolution and clipping. |
| `basic_resolution_group_ex.amproj` | Extended tests for group scaling and rotation. |
| `basic_multi_level_group.amproj` | Tests deeply nested groups. |

## Implementation Status
- **Group Rendering**: ✅ Supported via RTT
- **Resolution Clipping**: ✅ Supported (Corrected AM bug)
- **Time Offset**: ✅ Supported
- **Nested Groups**: ✅ Supported
