# 缩放辅助 (Scale Assist)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2025-02-08 14:00:00
> ⚠️ **注意：测试数据已过期（超过 1 天），建议重新运行测试。**

根据选择的轴向自动调整图层尺寸以适应画布。

**支持状态**: ⚠️ 部分支持

- **轴向 (scaleassistaxis)**: ✅ 已支持 (缩放基准轴 (1=宽度, 2=高度))

**关联测试文件：**
- `fx_6_scaleassist.amproj` ✅
- `fx_6_ex_scaleassist.amproj`

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.scaleassist">
    <property name="scaleassistaxis" type="float" value="1.0" />
</effect>
```
</details>
