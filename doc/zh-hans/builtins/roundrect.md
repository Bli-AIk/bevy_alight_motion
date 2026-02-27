# 圆角矩形

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-28 00:52:13

圆角矩形形状，使用 SDF 渲染。

**支持状态**: ✅ 完全支持

- **尺寸 (size)**: ✅ 已支持 (形状的宽度和高度)
- **圆角度 (roundness)**: ✅ 已支持 (圆角程度 (0.0-1.0))

**关联测试文件：**
- `basic/shape/shape.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<shape s=".roundrect">
    <property name="size" type="vec2" value="200.0,100.0" />
    <property name="roundness" type="float" value="0.3" />
</shape>
```
</details>
