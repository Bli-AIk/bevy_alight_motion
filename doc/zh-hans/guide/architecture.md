# 架构总览

`bevy_alight_motion` 基于数据驱动架构，将 Alight Motion 的 XML 模式映射到 Bevy 的 ECS 系统。

## 核心流程

1. **加载**：解压 `.amproj`（一个 ZIP 文件），解析 `scene.xml` 为 Rust 数据结构。
2. **处理**：将媒体资源（图像、字体）注册到 Bevy 的 `AssetServer`。
3. **生成**：将图层转换为 Bevy 实体。使用 `set_parent()` 建立父子关系。
4. **动画**：专用系统每帧对关键帧进行插值，并更新 `Transform` 和材质属性。
5. **渲染**：自定义着色器处理擦拭、拉伸和 SDF 形状等特有的 AM 效果。
