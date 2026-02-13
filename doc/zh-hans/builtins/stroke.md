# 描边

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-13 18:30:49

形状边框描边。使用 SDF 渲染，描边宽度在缩放动画中保持不变。

**支持状态**: ✅ 完全支持

- **方向 (direction)**: ✅ 已支持 (描边方向（居中、内部、外部）)
- **端点样式 (cap)**: ✅ 已支持 (线条端点样式)
- **连接样式 (join)**: ✅ 已支持 (线条连接样式（斜接、圆角、斜切）)
- **颜色 (color)**: ✅ 已支持 (描边颜色)
- **宽度 (size)**: ✅ 已支持 (描边宽度（像素）)

**关联测试文件：**
- `basic/shape/shape.amproj` ✅
- `basic/shape/ex.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<shape s=".rect">
    <path-stroke direction="centered" cap="round" join="round">
        <color value="#ff000000" />
        <size value="2.0" />
    </path-stroke>
</shape>
```
</details>
