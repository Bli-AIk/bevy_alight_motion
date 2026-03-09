# 回声关键帧 (Echo Keyframes)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

创建元素的时移回声副本，支持关键帧控制时间间隔、数量和透明度。

**支持状态**: ✅ 完全支持

- **时间间隔 (seconds)**: ✅ 已实现 (每个回声副本的时间间隔（秒）)
- **数量 (count)**: ✅ 已实现 (回声副本数量)
- **透明度 (alpha)**: ✅ 已实现 (回声衰减透明度曲线)
- **合成模式 (mode)**: ✅ 已实现 (合成模式（0=上方, 1=下方）)

**关联测试文件：**
- `effects/echo-keyframes/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.repeat.echokf">
    <property name="seconds" type="float" value="0.5" />
    <property name="count" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="mode" type="int" value="1" />
</effect>
```
</details>
