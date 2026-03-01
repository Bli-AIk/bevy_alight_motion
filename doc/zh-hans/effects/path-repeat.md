# 路径重复 (Path Repeat)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-01 17:45:43

沿路径分布图层的多个副本，支持切线对齐、缩放、透明度等参数。

**支持状态**: ✅ 完全支持

- **数量 (count)**: ✅ 已实现 (路径上分布的副本数量)
- **起始位置 (startPos)**: ✅ 已实现 (路径起始位置（0.0-1.0）)
- **结束位置 (endPos)**: ✅ 已实现 (路径结束位置（0.0-1.0）)
- **路径相位 (pathPhase)**: ✅ 已实现 (路径上的相位偏移)
- **切线对齐 (tangent)**: ✅ 已实现 (副本是否沿切线方向旋转)
- **偏移 (offset)**: ✅ 已实现 (每个副本的 X,Y 偏移（像素）)
- **角度 (angle)**: ✅ 已实现 (每个副本的旋转角度（度）)
- **缩放 (scale)**: ✅ 已实现 (每个副本的缩放乘数)
- **透明度 (alpha)**: ✅ 已实现 (每个副本的透明度乘数)
- **填充颜色 (fillColor)**: ✅ 已实现 (交替副本的填充颜色)
- **混合 (blend)**: ✅ 已实现 (填充颜色混合量)

**关联测试文件：**
- `effects/path-repeat/animation.amproj` ✅
- `effects/path-repeat/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.repeat.path">
    <property name="count" type="float" value="3.0" />
    <property name="startPos" type="float" value="0.0" />
    <property name="endPos" type="float" value="1.0" />
    <property name="pathPhase" type="float" value="0.0" />
    <property name="tangent" type="bool" value="false" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#FFFFFFFF" />
    <property name="blend" type="float" value="0.0" />
</effect>
```
</details>
