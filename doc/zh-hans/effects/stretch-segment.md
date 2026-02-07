# 拉伸片段效果 (Stretch Segment)

**拉伸片段**效果是一种 UV 空间变换，它沿指定的轴拉伸图像。

## 数学原理

该效果的工作原理是将图像沿分割线分为两部分，并在中间插入一个拉伸片段。

### 拉伸公式
为了精确匹配 Alight Motion 的行为，我们使用基于原始宽度的特定除数：
```rust
base_divisor = original_width / 5.76
stretch_factor = 1.0 + stretch_pixels / base_divisor
new_width = original_width * stretch_factor
```

## 参数

- **拉伸 (Stretch)**：拉伸量，以像素为单位。
- **角度 (Angle)**：分割线的角度。
- **偏移 (Offset)**：分割线相对于中心的位置。

## 实现细节

该效果在 `UnifiedEffectMaterial` 着色器 (`unified_effect.wgsl`) 中实现。通过在顶点着色器中计算变换并在片段着色器中将其应用到 UV 坐标，我们实现了高性能的变形效果。

### 包围盒计算
由于拉伸会增加图层的视觉尺寸，我们必须为拉伸和旋转后的图层计算精确的 **AABB（轴对齐包围盒）**，以防止过早被裁剪。

## 关联测试文件

| 文件 | 说明 |
|------|-------------|
| `fx_1_stretch_segment.amproj` | 基础拉伸片段测试。 |
| `fx_1_ex2_stretch_segment.amproj` | 测试不同的角度和偏移。 |
| `fx_1_ex4_stretch_segment.amproj` | 拉伸动画的综合测试。 |

## 实现状态
- **拉伸量**：✅ 已支持
- **角度/偏移**：⚠️ 基础支持（与 AM 相比存在轻微视觉差异）
- **平滑 (Smooth)**：❌ 暂未实现
