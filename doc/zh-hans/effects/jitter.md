# 抖动 (Jitter)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-01 17:45:43

使用 Simplex 噪声对图层位置进行随机抖动。

**支持状态**: ✅ 完全支持

- **角度 (angle)**: ✅ 已实现 (运动方向角度（度）)
- **频率 (freq)**: ✅ 已实现 (噪声频率（步/秒）)
- **幅度 (mag)**: ✅ 已实现 (位移幅度（像素）)
- **种子 (seed)**: ✅ 已实现 (噪声种子值)
- **松弛 (slack)**: ✅ 已实现 (垂直方向松弛量（0.0-1.0）)
- **Z轴抖动 (zjitter)**: ✅ 已实现 (Z轴方向抖动幅度)

**关联测试文件：**
- `effects/jetter/basic.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.jitter">
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="30.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="seed" type="float" value="0.0" />
    <property name="slack" type="float" value="0.0" />
    <property name="zjitter" type="float" value="0.0" />
</effect>
```
</details>
