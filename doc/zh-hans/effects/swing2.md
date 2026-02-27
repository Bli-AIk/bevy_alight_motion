# 摇摆 (Swing)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-28 00:21:36

使图层以指定频率和幅度来回摇摆旋转。

**支持状态**: ✅ 完全支持

- **频率 (freq)**: ✅ 已实现 (振荡频率（Hz）)
- **最小角度 (a1)**: ✅ 已实现 (振荡最小角度（度）)
- **最大角度 (a2)**: ✅ 已实现 (振荡最大角度（度）)
- **相位 (phase)**: ✅ 已实现 (振荡相位偏移（度）)
- **类型 (type)**: ✅ 已实现 (振荡波形类型（0=正弦，1=三角）)

**关联测试文件：**
- `effects/swing/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.swing2">
    <property name="freq" type="float" value="1.0" />
    <property name="a1" type="float" value="-30.0" />
    <property name="a2" type="float" value="30.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="type" type="int" value="0" />
</effect>
```
</details>
