# 纯色 (Solid Color)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

在内容上叠加一层纯色，支持混合模式和透明度控制。

**支持状态**: ⚠️ 部分支持

- **颜色 (color)**: ✅ 已实现 (叠加颜色（RGBA）)
- **透明度 (alpha)**: ✅ 已实现 (效果强度（0.0-1.0）)
- **混合模式 (blendMode)**: ⚠️ 部分实现 (混合模式（0=正常, 1=正片叠底, 2=滤色）)

**关联测试文件：**
- `effects/solid-color/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.solidcolor">
    <property name="color" type="color" value="#2D1EF6FF" />
    <property name="alpha" type="float" value="1.0" />
    <property name="blendMode" type="int" value="0" />
</effect>
```
</details>
