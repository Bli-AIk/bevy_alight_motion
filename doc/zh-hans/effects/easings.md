# 缓动曲线

Alight Motion 中的关键帧插值支持线性 (linear)、步进 (step) 以及自定义的三次贝塞尔 (Cubic Bezier) 曲线。

## 三次贝塞尔

Alight Motion 使用归一化的三次贝塞尔曲线来实现平滑过渡。我们使用牛顿迭代法在给定时间 `x` 的情况下求解 `y`。

### 实现
具体的数学实现请参见 `src/schema/easing.rs`。

## 关联测试文件

| 文件 | 说明 |
|------|-------------|
| `basic_bezier.amproj` | 标准三次贝塞尔曲线。 |
| `basic_bezier_ex.amproj` | 各种曲线形状的扩展测试。 |

## 状态
- **线性 (Linear)**：✅ 已支持
- **步进 (Step)**：✅ 已支持
- **三次贝塞尔 (Cubic Bezier)**：✅ 已支持
