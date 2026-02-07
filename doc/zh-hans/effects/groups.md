# 编组与分辨率

具有独立尺寸和时间线的嵌套场景。

- **嵌套**: ✅ 已支持 (支持多层嵌套)
- **裁剪**: ✅ 已支持 (基于分辨率的 AABB 裁剪)
- **时间偏移**: ✅ 已支持
- **变换**: ✅ 已支持 (编组级的整体位移/旋转)

**关联测试文件：**
- `basic_resolution_group.amproj`
- `basic_multi_level_group.amproj`

---

<details>
<summary>技术细节与实现</summary>

### RTT 架构
编组通过独立的相机和 `RenderLayer` 渲染到纹理中。
`EmbedClipMaterial` 负责执行矩形裁剪区域。

### AM 旋转 Bug 说明
在 AM 原版中，旋转的编组会出现错误的裁剪边界（扩大为正方形）。`bevy_alight_motion` 修正了这一行为，无论旋转角度如何，始终保持正确的矩形裁剪。
</details>