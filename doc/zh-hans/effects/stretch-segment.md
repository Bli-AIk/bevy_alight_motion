# 拉伸片段 (Stretch Segment)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-09 02:47:39

UV 域变形效果，沿分割线拉伸图像。拉伸公式: new_width = orig_width * (1.0 + stretch_px / (orig_width / 5.76))

**支持状态**: ⚠️ 部分支持

- **拉伸 (stretch)**: ✅ 已实现 (拉伸量（像素）)
- **角度 (angle)**: ✅ 已实现 (分割线角度（基本支持，存在轻微视觉差异）)
- **偏移 (offset)**: ✅ 已实现 (分割线位置偏移（基本支持，存在轻微视觉差异）)
- **平滑 (smooth)**: ✅ 已实现 (边缘平滑度（尚未实现）)

**关联测试文件：**
- `effects/stretch-segment/basic.amproj` ❌
- `effects/stretch-segment/ex.amproj` ❌
- `effects/stretch-segment/ex2.amproj` ❌
- `effects/stretch-segment/ex3.amproj` ❌
- `effects/stretch-segment/ex4.amproj` ✅
- `effects/stretch-segment/ex5.amproj` ❌

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.stretchsegment">
    <property name="stretch" type="float" value="0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="offset" type="float" value="0.0" />
    <property name="smooth" type="float" value="0.0" />
</effect>
```
</details>
