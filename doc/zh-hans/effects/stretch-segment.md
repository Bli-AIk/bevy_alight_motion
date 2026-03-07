# 拉伸片段 (Stretch Segment)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

UV 域变形效果，沿分割线拉伸图像。使用 AM 原始场景归一化坐标公式，支持多重拉伸效果叠加。

**支持状态**: ✅ 完全支持

- **拉伸 (stretch)**: ✅ 已实现 (拉伸量（像素）)
- **角度 (angle)**: ✅ 已实现 (分割线角度（度），使用场景归一化坐标系)
- **偏移 (offset)**: ✅ 已实现 (分割线位置偏移（场景归一化坐标 offset/1000）)
- **平滑 (smooth)**: ✅ 已实现 (边缘平滑度（使用 smin_cubic 实现）)

**关联测试文件：**
- `effects/stretch-segment/basic.amproj` ✅
- `effects/stretch-segment/ex.amproj` ✅
- `effects/stretch-segment/ex2.amproj` ✅
- `effects/stretch-segment/ex3.amproj` ✅
- `effects/stretch-segment/ex4.amproj` ✅
- `effects/stretch-segment/ex5.amproj` ✅
- `effects/stretch-segment/muti/basic.amproj` ✅

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
