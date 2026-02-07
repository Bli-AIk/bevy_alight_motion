# 拉伸片段 (Stretch Segment)

沿指定轴拉伸图像的 UV 空间变形效果。

- **拉伸量**: ✅ 已支持 (像素级拉伸)
- **角度**: ⚠️ 已支持 (存在轻微视觉差异)
- **偏移**: ⚠️ 已支持 (分割线位置控制)
- **平滑**: ❌ 暂未实现

**关联测试文件：**
- `fx_1_stretch_segment.amproj`
- `fx_1_ex2_stretch_segment.amproj`
- `fx_1_ex4_stretch_segment.amproj`

---

<details>
<summary>技术细节与实现</summary>

### 拉伸公式
为了匹配 AM 的行为，我们使用了基于原始宽度的除数计算拉伸因子：
`base_divisor = original_width / 5.76`
`stretch_factor = 1.0 + stretch_pixels / base_divisor`

### 着色器实现
在 `unified_effect.wgsl` 中实现。顶点着色器负责扩大包围盒，片段着色器执行 UV 映射变换。

### 包围盒计算
在 CPU 端进行精确的 AABB 计算，确保拉伸后的图层不会因为超出原始边界而被过早剔除。
</details>