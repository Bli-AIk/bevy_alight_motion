# 颜色替换 (Replace Color)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

在给定的容差范围内，将指定的源颜色替换为目标颜色。支持 sRGB 到线性颜色空间转换和动画关键帧。

- **旧颜色 (oldcolor)**: ✅ 已支持 (要替换的源颜色)
- **新颜色 (newcolor)**: ✅ 已支持 (替换后的目标颜色)
- **阈值 (threshold)**: ✅ 已支持 (颜色匹配容差)
- **羽化 (feather)**: ✅ 已支持 (边缘过渡柔和度)
- **透明度 (alpha)**: ✅ 已支持 (效果强度)
- **锁定亮度 (lockluminance)**: ✅ 已支持 (保持原始像素的亮度)

**关联测试文件：**
- `fx_8_replace_color.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.replacecolor">
    <property name="oldcolor" type="color" value="#ffff0000" />
    <property name="newcolor" type="color" value="#ff00ff00" />
    <property name="threshold" type="float" value="0.1" />
    <property name="feather" type="float" value="0.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="lockluminance" type="bool" value="false" />
</effect>
```

**计算的支持状态**: ✅ 完全支持
</details>
