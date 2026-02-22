# 径向重复 (Radial Repeat)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-22 12:25:09

沿圆形路径创建图层的多个副本，支持半径、扫掠角度、缩放等参数。

**支持状态**: ✅ 完全支持

- **数量 (count)**: ✅ 已实现 (副本数量)
- **半径 (radius)**: ✅ 已实现 (圆形路径的半径)
- **朝向 (orientation)**: ✅ 已实现 (副本朝向角度)
- **起始角度 (startAngle)**: ✅ 已实现 (起始角度（度）)
- **扫掠 (sweep)**: ✅ 已实现 (扫掠角度（度）)
- **基础缩放 (baseScale)**: ✅ 已实现 (所有副本的基础缩放)
- **偏移 (offset)**: ✅ 已实现 (每个副本的偏移)
- **角度 (angle)**: ✅ 已实现 (每个副本的旋转角度)
- **缩放 (scale)**: ✅ 已实现 (每个副本的缩放)
- **透明度 (alpha)**: ✅ 已实现 (每个副本的透明度)
- **填充颜色 (fillColor)**: ✅ 已实现 (副本填充颜色)
- **混合 (blend)**: ✅ 已实现 (颜色混合量)
- **交替着色 (colorAltCopies)**: ✅ 已实现 (交替副本着色)
- **开始 (start)**: ✅ 已实现 (可见范围开始)
- **结束 (end)**: ✅ 已实现 (可见范围结束)
- **相位 (phase)**: ✅ 已实现 (动画相位偏移)
- **缓入 (easeIn)**: ✅ 已实现 (缓入量)
- **缓出 (easeOut)**: ✅ 已实现 (缓出量)
- **重叠 (overlap)**: ✅ 已实现 (副本重叠量)
- **形状 (shape)**: ✅ 已实现 (排列形状类型)
- **反转 (invert)**: ✅ 已实现 (反转排列顺序)
- **随机顺序 (randomOrder)**: ✅ 已实现 (随机排列顺序)
- **种子 (seed)**: ✅ 已实现 (随机种子)

**关联测试文件：**
- `effects/radial-repeat/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.repeat.radial">
    <property name="count" type="float" value="5.0" />
    <property name="radius" type="float" value="100.0" />
    <property name="orientation" type="float" value="0.0" />
    <property name="startAngle" type="float" value="0.0" />
    <property name="sweep" type="float" value="360.0" />
    <property name="baseScale" type="float" value="1.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffffffff" />
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
