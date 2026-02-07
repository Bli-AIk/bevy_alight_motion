# 变换与移动

变换是所有动画的基础。`bevy_alight_motion` 处理位置、旋转、缩放和锚点（Pivot）的变化，并将它们从 AM 的坐标系映射到 Bevy 的坐标系。

## 坐标系统映射

AM 和 Bevy 使用不同的坐标系统：
- **AM**：原点在左上角，Y 轴向下增加。
- **Bevy**：原点在中心（默认），Y 轴向上增加。

我们使用以下公式将 AM 坐标转换为 Bevy 坐标：
```rust
bevy_x = am_x - canvas_width / 2.0
bevy_y = canvas_height / 2.0 - am_y
```

## 核心组件

### 位置 (`location`)
直接映射到 Bevy 的 `Transform.translation`。Z 坐标用于图层排序（深度）。

### 旋转 (`rotation`)
映射到 `Transform.rotation`。AM 通常绕 Z 轴以角度为单位进行旋转。

### 缩放 (`scale`)
映射到 `Transform.scale`。注意，对于 SDF 形状，非均匀缩放可能会在着色器内部处理，以保持描边的一致性。

### 锚点 (`pivot`)
锚点决定了旋转和缩放的中心。在 AM 中，修改锚点通常涉及位置补偿，以确保物体在视觉上保持在原位。

## 实现细节

变换逻辑主要在 `src/animation/systems.rs` 中处理，关键帧的值在此处进行插值并应用到实体的 `Transform` 组件。

### 父子层级
我们利用 Bevy 内置的层级系统。当一个 AM 图层具有 `parent` 属性时，对应的 Bevy 实体将成为父实体的子节点，从而使变换能够自然传递。

## 关联测试文件

| 文件 | 说明 |
|------|-------------|
| `basic_pivot.amproj` | 测试锚点偏移和位置补偿。 |
| `basic_frame.amproj` | 测试基础的关键帧位置和旋转动画。 |
| `basic_bounce_box.amproj` | 测试复杂的缩放和位置动画组合。 |

## 实现状态
- **位置/旋转/缩放**：✅ 完全支持
- **锚点补偿**：✅ 完全支持
- **3D 旋转**：❌ 暂未实现
