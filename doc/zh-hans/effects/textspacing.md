# 文字间距 (Text Spacing)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

控制文本的字间距和行间距。

**支持状态**: ❌ 不支持

- **字间距 (letterspacing)**: ✅ 已实现 (字间距（em 单位，0.0=默认）)
- **行间距 (linespacing)**: ✅ 已实现 (行间距倍数（1.0=默认）)

**关联测试文件：**
- `effects/text-spacing/basic.amproj` ❌

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.textspacing">
    <property name="letterspacing" type="float" value="0.0" />
    <property name="linespacing" type="float" value="1.0" />
</effect>
```
</details>
