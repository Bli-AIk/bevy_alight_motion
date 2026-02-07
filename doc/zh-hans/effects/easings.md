# 缓动曲线 (Easings)

用于实现参数平滑过渡的关键帧插值方法。

- **线性 (Linear)**: ✅ 已支持
- **步进 (Step)**: ✅ 已支持 (离散跳变)
- **三次贝塞尔 (Cubic Bezier)**: ✅ 已支持 (自定义曲线)

**关联测试文件：**
- `basic_bezier.amproj`
- `basic_bezier_ex.amproj`

---

<details>
<summary>技术细节与实现</summary>

### 贝塞尔求解器
使用牛顿迭代法根据归一化时间 `x` 求解贝塞尔方程中的 `y`。
具体实现位于 `src/schema/easing.rs`。

### 归一化
在进行插值之前，所有时间值都会在每个关键帧段内归一化到 `0.0` 至 `1.0` 范围内。
</details>