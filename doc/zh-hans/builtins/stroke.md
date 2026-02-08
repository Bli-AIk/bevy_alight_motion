# 描边

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

形状边框描边。使用 SDF 渲染，描边宽度在缩放动画中保持不变。

- **方向 (direction)**: ✅ 已支持 (描边方向（居中、内部、外部）)
- **端点样式 (cap)**: ✅ 已支持 (线条端点样式)
- **连接样式 (join)**: ✅ 已支持 (线条连接样式（斜接、圆角、斜切）)
- **颜色 (color)**: ✅ 已支持 (描边颜色)
- **宽度 (size)**: ✅ 已支持 (描边宽度（像素）)

**关联测试文件：**
- `basic_shape.amproj` ✅
- `basic_shape_ex.amproj`

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

**计算的支持状态**: ⚠️ 部分支持
</details>
