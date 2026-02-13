# 矩形

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-13 18:30:49

基础矩形形状，支持 SDF 渲染和精灵渲染。

**支持状态**: ✅ 完全支持

- **尺寸 (size)**: ✅ 已支持 (形状的宽度和高度)

**关联测试文件：**
- `basic/shape/shape.amproj` ✅
- `basic/shape/ex.amproj` ✅

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
</details>
