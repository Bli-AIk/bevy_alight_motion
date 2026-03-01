# 振荡 (Oscillate)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-01 17:45:43

使图层沿指定方向以正弦/三角波进行周期性振荡运动。

**支持状态**: ⚠️ 部分支持

- **方向模式 (direction)**: ⚠️ 部分实现 (运动方向模式（0=角度, 1=深度/Z轴, 2=轨道）)
- **角度 (angle)**: ✅ 已实现 (运动方向角度（度）)
- **频率 (freq)**: ✅ 已实现 (振荡频率（Hz）)
- **幅度 (mag)**: ✅ 已实现 (运动幅度（像素）)
- **波形 (type)**: ✅ 已实现 (波形类型（0=正弦, 1=三角波）)
- **相位 (phase)**: ✅ 已实现 (相位偏移)

**关联测试文件：**
- `effects/swing/animation.amproj` ✅
- `effects/swing/multi.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.oscillate3">
    <property name="direction" type="int" value="0" />
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="2.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="type" type="int" value="0" />
    <property name="phase" type="float" value="0.0" />
</effect>
```
</details>
