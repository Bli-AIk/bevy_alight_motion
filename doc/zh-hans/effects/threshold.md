# 阈值 (Threshold)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-10 00:07:53

将图像转换为只有黑色和白色的高对比度图像。

**支持状态**: ✅ 完全支持

- **阈值 (threshold)**: ✅ 已实现 (亮度截止值)
- **羽化 (feather)**: ✅ 已实现 (边缘柔和度)
- **反转 (invert)**: ✅ 已实现 (反转效果)
- **混合模式 (blendMode)**: ✅ 已实现 (混合模式)

**关联测试文件：**
- `effects/threshold/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.threshold">
    <property name="threshold" type="float" value="0.5" />
    <property name="feather" type="float" value="0.0" />
    <property name="invert" type="bool" value="false" />
    <property name="blendMode" type="int" value="0" />
</effect>
```
</details>
