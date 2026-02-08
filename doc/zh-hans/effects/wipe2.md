# 擦拭 (Wipe2)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-08 14:15:04

从图层的相对两侧遮盖矩形片段。使用关键帧动画创建擦拭过渡。

**支持状态**: ❌ 不支持

- **起始 (start)**: ✅ 已支持 (可见范围起点 (0.0-1.0))
- **结束 (end)**: ✅ 已支持 (可见范围终点 (0.0-1.0))
- **角度 (angle)**: ✅ 已支持 (擦除方向角度)
- **羽化 (feather)**: ⚠️ 基础支持 (边缘柔和度（基本支持，尚未校对）)

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.wipe2">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="feather" type="float" value="0.0" />
</effect>
```
</details>
