# 擦拭 (Wipe / Cutoff)

基于指定角度从边缘隐藏图层的过渡效果。

- **起始/结束**: ✅ 已支持 (0.0 到 1.0 范围)
- **角度**: ✅ 已支持 (线性方向)
- **羽化**: ⚠️ 已支持 (需要进一步视觉校准)

**关联测试文件：**
- `basic_cutoff.amproj`
- `showcase.amproj`

---

<details>
<summary>技术细节与实现</summary>

### 着色器逻辑
作为 `UnifiedEffectMaterial` 的一部分。它将像素 UV 投影到由角度定义的垂直向量上。
`val = dot(uv, vec2(cos(angle), sin(angle)))`
超出 `[start, end]` 范围的像素将被丢弃。

### 校准说明
AM 的羽化行为是非线性的。目前的实现使用线性渐变，在边缘柔和度上可能与原版存在细微差异。
</details>