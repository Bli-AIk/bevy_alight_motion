# 高斯模糊 (Gaussian Blur)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

使用多 pass 模糊实现平滑的高斯模糊效果，支持超出原始边界的发光扩散。

**支持状态**: ❌ 不支持

- **模糊强度 (strength)**: ✅ 已支持 (模糊强度像素值)

**关联测试文件：**
- `fx_2_gaussian_blur.amproj` ⏭️

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.gaussianblur">
    <property name="strength" type="float" value="0.0" />
</effect>
```
</details>
