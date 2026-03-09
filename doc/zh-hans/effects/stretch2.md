# 拉伸 (Stretch)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

沿指定角度方向在UV空间拉伸图层。

**支持状态**: ⚠️ 部分支持

- **缩放 (scale)**: ✅ 已实现 (拉伸轴方向缩放因子（1.0=无拉伸）)
- **角度 (angle)**: ✅ 已实现 (拉伸方向角度（度）)
- **仅内容 (contentOnly)**: ✅ 已实现 (是否将拉伸结果裁剪到原始图层 Alpha)

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.stretch2">
    <property name="scale" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="contentOnly" type="bool" value="false" />
</effect>
```
</details>
