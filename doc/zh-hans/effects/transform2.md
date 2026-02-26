# 变换 (Transform2)

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-02-23 18:55:44

可以复制类似变换、角度、缩放的设置，提供额外的位移控制。

**支持状态**: ⚠️ 部分支持

- **X 偏移 (posx)**: ✅ 已实现 (额外的水平位移)
- **Y 偏移 (posy)**: ✅ 已实现 (额外的垂直位移)
- **Z 偏移 (posz)**: ✅ 已实现 (缩放倍数（Z 轴位移模拟）)
- **角度 (angle)**: ✅ 已实现 (额外的旋转角度（度）)
- **X 反转 (xinv)**: ❌ 未实现 (水平翻转)
- **Y 反转 (yinv)**: ❌ 未实现 (垂直翻转)
- **Z 反转 (zinv)**: ❌ 未实现 (缩放反转)
- **角度反转 (ainv)**: ❌ 未实现 (角度反转)

**关联测试文件：**
- `effects/transform/complex1.amproj` ✅
- `effects/transform/complex2.amproj` ✅

---

<details>
<summary>技术细节与实现</summary>

### XML 示例

```xml
<effect id="com.alightcreative.effects.transform2" locallyApplied="true">
    <property name="posx" type="float" value="0.0" />
    <property name="posy" type="float" value="0.0" />
    <property name="posz" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="xinv" type="bool" value="false" />
    <property name="yinv" type="bool" value="false" />
    <property name="zinv" type="bool" value="false" />
    <property name="ainv" type="bool" value="false" />
</effect>
```
</details>
