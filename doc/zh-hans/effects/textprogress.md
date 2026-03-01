# 文字进度 (Text Progress)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-01 17:45:43

显示文字的部分内容，实现打字机效果。

**支持状态**: ❌ 不支持

- **起始 (start)**: ✅ 已实现 (文本显示起始位置（0-1）)
- **结束 (end)**: ✅ 已实现 (文本显示结束位置（0-1）)
- **光标 (cursor)**: ⚠️ 部分实现 (光标样式（0=无，1-8=不同样式）)
- **闪烁 (blink)**: ⚠️ 部分实现 (光标闪烁)

**关联测试文件：**
- `effects/text-progress/basic.amproj` ❌

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.textprogress">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="cursor" type="int" value="0" />
    <property name="blink" type="bool" value="false" />
</effect>
```
</details>
