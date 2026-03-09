# 效果列表

> ⚠️ **此文档由代码自动生成，请勿手动编辑。**
> 最近测试时间：2026-03-07 20:20:21

| 效果 | 支持状态 | 说明 |
|------|---------|------|
| [变换 (Transform2)](./transform2.md) | ⚠️ | 可以复制类似变换、角度、缩放的设置，提供额外的位移控制。 |
| [擦拭 (Wipe2)](./wipe2.md) | ✅ | 从图层的相对两侧遮盖矩形片段。使用关键帧动画创建擦拭过渡。 |
| [拉伸片段 (Stretch Segment)](./stretch-segment.md) | ✅ | UV 域变形效果，沿分割线拉伸图像。使用 AM 原始场景归一化坐标公式，支持多重拉伸效果叠加。 |
| [拉伸 (Stretch)](./stretch2.md) | ⚠️ | 沿指定角度方向在UV空间拉伸图层。 |
| [高斯模糊 (Gaussian Blur)](./gaussian-blur.md) | ❌ | 使用多 pass 模糊实现平滑的高斯模糊效果，支持超出原始边界的发光扩散。 |
| [网格 (Grid)](./grid.md) | ✅ | 在图层上叠加网格图案或将其挖空。 |
| [阈值 (Threshold)](./threshold.md) | ⚠️ | 将图像转换为只有黑色和白色的高对比度图像。 |
| [调色板映射 (Palette Map)](./palette-map.md) | ⚠️ | 将图像颜色映射到指定的调色板颜色。支持最多 8 个调色板颜色。 |
| [颜色替换 (Replace Color)](./replace-color.md) | ✅ | 在给定的容差范围内，将指定的源颜色替换为目标颜色。支持 sRGB 到线性颜色空间转换和动画关键帧。 |
| [缩放辅助 (Scale Assist)](./scale-assist.md) | ✅ | 根据选择的轴向自动调整图层尺寸以适应画布。 |
| [像素化 (Pixelate)](./pixelate.md) | ⚠️ | 降低图像分辨率，产生像素化效果。 |
| [振荡 (Oscillate)](./oscillate3.md) | ⚠️ | 使图层沿指定方向以正弦/三角波进行周期性振荡运动。 |
| [抖动 (Jitter)](./jitter.md) | ✅ | 使用 Simplex 噪声对图层位置进行随机抖动。 |
| [重复 (Repeat)](./repeat.md) | ⚠️ | 创建图层的多个副本，每个副本应用累积的偏移、旋转、缩放和透明度变换。 |
| [线性重复 (Linear Repeat)](./linear-repeat.md) | ⚠️ | 创建沿直线排列的图层副本，支持位置、偏移、旋转、缩放、透明度和颜色混合等高级控制。 |
| [径向重复 (Radial Repeat)](./radial-repeat.md) | ✅ | 沿圆形路径创建图层的多个副本，支持半径、扫掠角度、缩放等参数。 |
| [路径重复 (Path Repeat)](./path-repeat.md) | ✅ | 沿路径分布图层的多个副本，支持切线对齐、缩放、透明度等参数。 |
| [回声关键帧 (Echo Keyframes)](./echokf.md) | ✅ | 创建元素的时移回声副本，支持关键帧控制时间间隔、数量和透明度。 |
| [纯色 (Solid Color)](./solidcolor.md) | ⚠️ | 在内容上叠加一层纯色，支持混合模式和透明度控制。 |
| [摇摆 (Swing)](./swing2.md) | ❌ | 使图层以指定频率和幅度来回摇摆旋转。 |
| [旋转 (Spin)](./spin.md) | ✅ | 使图层以指定速度持续旋转。 |
| [文字进度 (Text Progress)](./textprogress.md) | ❌ | 显示文字的部分内容，实现打字机效果。 |
| [文字间距 (Text Spacing)](./textspacing.md) | ❌ | 控制文本的字间距和行间距。 |
