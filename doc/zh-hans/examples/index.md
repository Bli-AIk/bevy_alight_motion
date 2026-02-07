# 示例展示

本页面列出了用于验证 `bevy_alight_motion` 功能的 Alight Motion 项目文件。

## 基础功能

| 示例文件 | 验证功能 | 状态 |
|--------------|-------------------|:------:|
| `basic_pivot.amproj` | 锚点偏移与位置补偿 | ✅ |
| `basic_frame.amproj` | 关键帧动画 | ✅ |
| `basic_shape.amproj` | SDF 矩形/圆形与填充 | ✅ |
| `basic_resolution_group.amproj` | 编组裁剪 | ✅ |
| `basic_bezier.amproj` | 三次贝塞尔缓动 | ✅ |
| `basic_mask_square.amproj` | 矩形遮罩 | ✅ |

## 高级效果

| 示例文件 | 验证功能 | 状态 |
|--------------|-------------------|:------:|
| `fx_1_stretch_segment.amproj` | 拉伸片段 (Stretch Segment) | ⚠️ |
| `fx_5_palette.amproj` | 调色板映射 (Palette Map) | ⚠️ |
| `fx_6_scaleassist.amproj` | 缩放辅助 (Scale Assist) | ✅ |
| `fx_8_replace_color.amproj` | 颜色替换 (Replace Color) | ✅ |
| `fx_2_gaussian_blur.amproj` | 高斯模糊 (Gaussian Blur) | ✅ |

## 复杂场景

| 示例文件 | 验证功能 | 状态 |
|--------------|-------------------|:------:|
| `showcase.amproj` | 综合功能演示 | ⚠️ |

---

**图例**:
- ✅ 通过 (相似度 ≥ 95%)
- ⚠️ 部分通过 (存在细微差异)
- ❌ 未实现
