# 矩形

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

基础矩形形状，支持 SDF 渲染和精灵渲染。

- **尺寸 (size)**: ✅ 已支持 (形状的宽度和高度)

**关联测试文件：**
- `basic_shape.amproj` ✅
- `basic_shape_ex.amproj`

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<shape id="1" label="矩形 1" startTime="0" endTime="1000" fillType="color" s=".rect">
    <transform>
        <location value="640.0,480.0,0.0" />
        <rotation value="0.0" />
        <scale value="1.0,1.0" />
        <opacity value="1.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ffff0000" />
</shape>
```

**计算的支持状态**: ⚠️ 部分支持
</details>
