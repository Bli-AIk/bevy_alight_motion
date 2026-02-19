# 媒体填充

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-19 08:18:36

使用图像纹理填充形状。支持 JPEG 和 PNG 格式。

**支持状态**: ✅ 完全支持

- **填充图像 (fillImage)**: ✅ 已支持 (图像资源 URI (amproj:filename.png))

**关联测试文件：**
- `basic/shape/shape.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<shape fillType="media" fillImage="amproj:image.png">
    <property name="size" type="vec2" value="100.0,100.0" />
</shape>
```
</details>
