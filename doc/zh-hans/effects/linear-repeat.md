# 线性重复 (Linear Repeat)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

创建沿直线排列的图层副本，支持位置、偏移、旋转、缩放、透明度和颜色混合等高级控制。

**支持状态**: ⚠️ 部分支持

- **数量 (count)**: ✅ 已实现 (创建的副本数量)
- **位置 (position)**: ✅ 已实现 (从第一个副本到最后一个副本的总位移)
- **偏移 (offset)**: ✅ 已实现 (应用于所有副本的恒定偏移)
- **角度 (angle)**: ✅ 已实现 (每个副本的旋转角度（度）)
- **缩放 (scale)**: ✅ 已实现 (每个副本的缩放乘数)
- **透明度 (alpha)**: ✅ 已实现 (每个副本的透明度乘数)
- **填充颜色 (fillColor)**: ✅ 已实现 (用于颜色混合的填充颜色)
- **混合 (blend)**: ✅ 已实现 (填充颜色的混合量)
- **交替颜色 (colorAltCopies)**: ✅ 已实现 (是否交替应用颜色)
- **开始 (start)**: ⚠️ 部分实现 (分布的起始点（0-1）)
- **结束 (end)**: ⚠️ 部分实现 (分布的结束点（0-1）)
- **相位 (phase)**: ⚠️ 部分实现 (分布的相位偏移)
- **缓入 (easeIn)**: ⚠️ 部分实现 (分布的缓入因子)
- **缓出 (easeOut)**: ⚠️ 部分实现 (分布的缓出因子)
- **重叠 (overlap)**: ❌ 未实现 (副本之间的重叠因子)
- **形状 (shape)**: ⚠️ 部分实现 (分布形状（0=线性）)
- **反转 (invert)**: ❌ 未实现 (是否反转效果)
- **随机顺序 (randomOrder)**: ❌ 未实现 (是否随机化副本顺序)
- **种子 (seed)**: ❌ 未实现 (随机种子)

**关联测试文件：**
- `effects/linear-repeat/basic.amproj` ✅
- `effects/linear-repeat/dual-16-9.amproj` ✅
- `effects/linear-repeat/dual.amproj` ✅
- `effects/linear-repeat/random.amproj` ❌
- `effects/linear-repeat/random_generated1/1.amproj` ❌
- `effects/linear-repeat/random_generated1/2.amproj` ❌
- `effects/linear-repeat/random_generated1/3.amproj` ❌
- `effects/linear-repeat/random_generated2/1.amproj` ✅
- `effects/linear-repeat/random_generated2/2.amproj` ✅
- `effects/linear-repeat/random_generated2/3.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.repeat.line">
    <property name="count" type="float" value="5.0" />
    <property name="position" type="vec2" value="200.0,0.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffff0000" />
    <property name="blend" type="float" value="0.0" />
    <property name="colorAltCopies" type="bool" value="false" />
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="easeIn" type="float" value="0.0" />
    <property name="easeOut" type="float" value="0.0" />
    <property name="overlap" type="float" value="0.0" />
    <property name="shape" type="int" value="0" />
    <property name="invert" type="bool" value="false" />
    <property name="randomOrder" type="bool" value="false" />
    <property name="seed" type="float" value="0.0" />
</effect>
```
</details>
