# 图层遮罩

遮罩允许一个图层定义另一个图层的可见性。

## 实现方式

目前，我们通过 `UnifiedEffectMaterial` 支持矩形和椭圆遮罩。复杂的基于路径的遮罩尚未实现。

## 关联测试文件

| 文件 | 说明 |
|------|-------------|
| `basic_mask_square.amproj` | 矩形包含遮罩。 |
| `basic_mask_circle.amproj` | 椭圆包含遮罩。 |
| `basic_child_mask.amproj` | 通过父子层级应用的遮罩。 |

## 状态
- **矩形/椭圆遮罩**：✅ 已支持
- **排除遮罩 (Exclusion)**：✅ 已支持
- **复杂路径遮罩**：❌ 暂未实现
