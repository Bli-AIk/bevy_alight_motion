# 圆形

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-28 00:52:13

基础圆形形状，使用 SDF 渲染。支持非均匀缩放形成椭圆。

**支持状态**: ✅ 完全支持

- **尺寸 (size)**: ✅ 已支持 (圆形的宽度和高度（非均匀值形成椭圆）)

**关联测试文件：**
- `basic/shape/shape.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<shape id="1" label="圆形 1" startTime="0" endTime="1000" fillType="color" s=".circle">
    <transform>
        <location value="640.0,480.0,0.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ff00ff00" />
</shape>
```
</details>
