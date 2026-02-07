# 变换与移动

将 Alight Motion 的基础图层变换映射到 Bevy 引擎。

- **位置**: ✅ 已支持 (线性插值)
- **旋转**: ✅ 已支持 (Z 轴旋转)
- **缩放**: ✅ 已支持 (均匀与非均匀缩放)
- **锚点**: ✅ 已支持 (带位置补偿的锚点系统)

**关联测试文件：**
- `basic_pivot.amproj`
- `basic_frame.amproj`
- `basic_bounce_box.amproj`

---

<details>
<summary>技术细节与实现</summary>

### 坐标系转换
AM 使用左上角原点，Y 轴向下。Bevy 使用中心原点，Y 轴向上。
转换公式：`bevy_x = am_x - width/2`, `bevy_y = height/2 - am_y`。

### 锚点补偿
在 AM 中，修改锚点不会导致图层视觉位置发生位移。我们通过计算锚点移动带来的位移增量并应用反向补偿来实现这一行为。

### 层级系统
利用 Bevy 原生的父子实体系统实现图层嵌套，确保变换能够正确传递。
</details>