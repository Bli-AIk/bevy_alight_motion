# 变换

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-08 18:36:46

图层的基础变换属性，包括位置、旋转、缩放、透明度和锚点。坐标系统：AM 使用左上角原点，Bevy 使用中心原点，库自动转换。

**支持状态**: ✅ 完全支持

- **位置 (location)**: ✅ 已支持 (图层位置 (x, y, z))
- **旋转 (rotation)**: ✅ 已支持 (Z 轴旋转角度（度）)
- **缩放 (scale)**: ✅ 已支持 (缩放比例 (x, y))
- **透明度 (opacity)**: ✅ 已支持 (透明度 (0.0-1.0))
- **锚点 (pivot)**: ✅ 已支持 (旋转和缩放的锚点位置)
- **锁定宽高比 (lockAspectRatio)**: ✅ 已支持 (是否锁定宽高比)

**关联测试文件：**
- `basic_shape.amproj` ✅
- `basic_pivot.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<transform lockAspectRatio="false">
    <location value="640.0,480.0,0.0" />
    <pivot value="0.0,0.0" />
    <rotation value="45.0" />
    <scale value="1.5,1.5" />
    <opacity value="0.8" />
</transform>
```
</details>
