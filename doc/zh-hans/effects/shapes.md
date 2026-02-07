# 形状与填充

使用有向距离场 (SDF) 渲染的核心视觉原语。

- **矩形**: ✅ 已支持 (支持圆角控制)
- **圆形/椭圆**: ✅ 已支持
- **颜色填充**: ✅ 已支持 (线性 RGBA)
- **媒体填充**: ✅ 已支持 (图像纹理)
- **描边**: ✅ 已支持 (支持圆角、斜接、斜切连接样式)

**关联测试文件：**
- `basic_shape.amproj`
- `basic_shape_ex.amproj`

---

<details>
<summary>技术细节与实现</summary>

### SDF 渲染
通过 `SdfMaterial` 渲染。这使得形状在无限放大时也不会出现像素化，并能高效渲染描边。

### 描边连接
- **圆角 (Round)**: SDF 默认行为。
- **斜接/斜切 (Miter/Bevel)**: 由专门的着色器处理 (`stroked_fill_box_miter.wgsl` / `bevel.wgsl`)。

### 媒体映射
对于媒体填充，UV 坐标是相对于形状边界实时计算的。
</details>