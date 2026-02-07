# RTT 与效果系统

渲染到纹理 (RTT) 用于处理复杂的场景合成和编组级效果。

## RTT 流水线

1. **隔离**：编组内的图层被分配到特定的 `RenderLayer`。
2. **专用相机**：一个副相机仅将该 `RenderLayer` 渲染到一个 `Texture` 中。
3. **投射**：生成的纹理随后被绘制回主场景。

## 统一效果着色器 (Unified Effect Shader)

对于单个图层，我们使用 `UnifiedEffectMaterial`。该着色器旨在单次 pass 中应用多个效果，以实现性能最大化：
- **遮罩 (Masking)**
- **擦拭 (Wipe)**
- **拉伸片段 (Stretch Segment)**

通过组合这些效果，我们避免了多次渲染 pass，并减少了 GPU 状态切换。
