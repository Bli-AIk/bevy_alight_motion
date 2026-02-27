# 重复 (Repeat)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-28 00:52:13

创建图层的多个副本，每个副本应用累积的偏移、旋转、缩放和透明度变换。

**支持状态**: ⚠️ 部分支持

- **数量 (count)**: ✅ 已实现 (创建的副本数量)
- **时间偏移 (time)**: ❌ 未实现 (副本之间的时间偏移（尚未实现）)
- **偏移 (offset)**: ✅ 已实现 (每个副本的 X,Y 偏移（像素）)
- **角度 (angle)**: ✅ 已实现 (每个副本的旋转角度（度）)
- **缩放 (scale)**: ✅ 已实现 (每个副本的缩放乘数)
- **透明度 (alpha)**: ✅ 已实现 (每个副本的透明度乘数)

**关联测试文件：**
- `effects/repeat/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.repeat">
    <property name="count" type="float" value="3.0" />
    <property name="time" type="float" value="0.0" />
    <property name="offset" type="vec2" value="50.0,50.0" />
    <property name="angle" type="float" value="15.0" />
    <property name="scale" type="float" value="0.9" />
    <property name="alpha" type="float" value="0.8" />
</effect>
```
</details>
