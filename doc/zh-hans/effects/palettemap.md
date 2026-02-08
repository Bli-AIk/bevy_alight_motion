# 调色板映射 (Palette Map)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

将图像颜色映射到指定的调色板颜色。支持最多 8 个调色板颜色。

- **颜色 1 (color1)**: ✅ 已支持 (调色板颜色 1)
- **颜色 2 (color2)**: ✅ 已支持 (调色板颜色 2)
- **颜色 3 (color3)**: ✅ 已支持 (调色板颜色 3（可选）)
- **颜色 4 (color4)**: ✅ 已支持 (调色板颜色 4（可选）)
- **颜色 5 (color5)**: ✅ 已支持 (调色板颜色 5（可选）)
- **颜色 6 (color6)**: ✅ 已支持 (调色板颜色 6（可选）)
- **颜色 7 (color7)**: ✅ 已支持 (调色板颜色 7（可选）)
- **颜色 8 (color8)**: ✅ 已支持 (调色板颜色 8（可选）)
- **颜色数量 (count)**: ✅ 已支持 (使用的颜色数量)
- **阴影模式 (shades)**: ⚠️ 基础支持 (是否启用阴影渐变（基础支持，颜色过渡算法与 AM 存在细微差异）)
- **混合强度 (alpha)**: ✅ 已支持 (效果混合强度)

**关联测试文件：**
- `fx_5_palette.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.palettemap">
    <property name="color1" type="color" value="#ff000000" />
    <property name="color2" type="color" value="#ffffffff" />
    <property name="count" type="float" value="2.0" />
    <property name="shades" type="bool" value="false" />
    <property name="alpha" type="float" value="1.0" />
</effect>
```

**计算的支持状态**: ✅ 完全支持
</details>
