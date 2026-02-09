# 网格 (Grid)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-09 23:01:34

在图层上叠加网格图案或将其挖空。

**支持状态**: ✅ 完全支持

- **位置 (position)**: ✅ 已实现 (网格偏移位置)
- **间距 (spacing)**: ✅ 已实现 (网格线之间的间距)
- **宽度 (width)**: ✅ 已实现 (网格线宽度)
- **颜色 (color)**: ✅ 已实现 (网格线颜色)
- **挖空 (punchout)**: ✅ 已实现 (是否从图像中挖空网格线)
- **平滑 (smoothing)**: ✅ 已实现 (边缘平滑度)
- **屏幕空间 (screenSpace)**: ✅ 已实现 (使用屏幕坐标而非图层坐标)

**关联测试文件：**
- `effects/grid/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.grid2">
    <property name="position" type="vec2" value="0.0,0.0" />
    <property name="spacing" type="float" value="0.1" />
    <property name="width" type="float" value="0.01" />
    <property name="color" type="color" value="#ff000000" />
    <property name="punchout" type="bool" value="false" />
    <property name="smoothing" type="float" value="0.05" />
    <property name="screenSpace" type="bool" value="false" />
</effect>
```
</details>
