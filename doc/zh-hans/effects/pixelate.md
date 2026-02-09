# 像素化 (Pixelate)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-09 23:01:34

降低图像分辨率，产生像素化效果。

**支持状态**: ❌ 不支持

- **大小 (size)**: ❌ 未实现 (像素大小 (1-100))
- **拉伸 (stretch)**: ❌ 未实现 (像素拉伸比例)
- **角度 (angle)**: ❌ 未实现 (像素网格旋转角度)
- **晕影 (vignette)**: ❌ 未实现 (晕影强度 (尚未完全支持))
- **屏幕空间 (screenSpace)**: ❌ 未实现 (是否使用屏幕空间坐标 (尚未完全支持))

**关联测试文件：**
- `basic/bounce/box.amproj`
- `basic/mask/child.amproj`
- `basic/mask/circle.amproj`
- `basic/shape/ex.amproj`
- `effects/pixelate/basic.amproj` ⏭️

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.pixelate2">
    <property name="size" type="float" value="10.0" />
    <property name="stretch" type="vec2" value="1.0,1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="vignette" type="float" value="0.0" />
    <property name="screenSpace" type="boolean" value="false" />
    <property name="threshold" type="float" value="0.5" />
    <property name="saturation" type="float" value="1.0" />
</effect>
```
</details>
