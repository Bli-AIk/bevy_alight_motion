# 图层遮罩

通过一个图层的形状来控制另一个图层可见性的功能。

- **包含遮罩 (Inclusion)**: ✅ 已支持 (内部显示)
- **排除遮罩 (Exclusion)**: ✅ 已支持 (外部显示)
- **多重遮罩**: ✅ 已支持 (最多支持两个叠加遮罩)

**关联测试文件：**
- `basic_mask_square.amproj`
- `basic_mask_circle.amproj`
- `basic_child_mask.amproj`

---

<details>
<summary>技术细节与实现</summary>

### 片段裁剪
遮罩逻辑在 `UnifiedEffectMaterial` 中执行。着色器会检查当前像素是否位于遮罩图层的形状边界内。

### 空间坐标
遮罩计算在目标图层的局部空间中进行，以确保能够正确处理父级变换。
</details>