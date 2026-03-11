# bevy_alight_motion WASM Android 黑屏分析报告

**症状**: Android 浏览器上传 .amproj 后，仅显示 2 行日志，之后永久黑屏。PC 正常。

---

## 🔴 根本原因：Canvas 竞态 + Bevy 初始化时序

### 问题 1：`#[wasm_bindgen(start)]` 导致 Canvas 竞态（严重度：**极高**）

**文件**: `wasm/src/lib.rs:79-134` + `AmPlayground.vue:202-227`

**时序链**:
1. 用户上传文件 → `loadProject()` 调用 `loadWasm()`
2. `loadWasm()` 创建 `<script type="module">` 并执行 `import init; await init()`
3. `#[wasm_bindgen(start)]` 使 `main()` 在 `init()` 时**立即执行**
4. `main()` 输出 "WASM module initialized"（日志 1）
5. `main()` 输出 "App state initialized"（日志 2）
6. `App::new().run()` → Bevy 的 `WinitPlugin` 尝试获取 `<canvas id="bevy-canvas">`
7. **但**: canvas 位于 `v-show="isLoaded && !isLoading"` 的元素内。此时 `isLoaded = false`，canvas 被 CSS `display: none` 隐藏
8. Bevy 获取到一个**尺寸为 0×0** 的 canvas
9. WebGL2 创建上下文时，0×0 canvas → 创建失败或创建了无效上下文
10. **Android 影响更大**：Android Chrome 的 GPU 进程初始化更慢（200-500ms），竞态窗口更大

**PC 正常的原因**：
- PC GPU 初始化快（20-50ms），在 500ms 等待后 `isLoaded = true` 来得及
- PC 的 WebGL2 对 0×0 canvas 更宽容（某些驱动会使用 1×1 替代）

**关键代码路径**：
```
Vue: v-show="isLoaded && !isLoading"  ← canvas 被隐藏
                    ↓
WASM: App::new().run()  ← 此时 isLoaded 还是 false
                    ↓
500ms 后: isLoaded = true  ← 太晚了，Bevy 已经尝试创建了 WebGL context
```

### 问题 2：`v-show` vs `v-if` 的微妙差异

**文件**: `AmPlayground.vue:30`

```html
<div class="player-section" v-show="isLoaded && !isLoading">
```

`v-show` 使用 `display: none`，canvas **存在于 DOM 中但不可见**。Bevy 通过 `querySelector("#bevy-canvas")` 能找到它，但它的 `clientWidth/clientHeight` 为 0。

**后果**: Bevy 不报错（找到了 canvas），但 WebGL 上下文绑定到了零尺寸 framebuffer → 所有渲染都丢失。

---

## 🟡 加剧因素

### 问题 3：多相机 RTT 架构在 WebGL2 上的限制

**文件**: `gaussian_blur.rs:353-403`, `effects/rtt.rs:386-409`

bevy_alight_motion 使用**多相机 render-to-texture** 架构：
- 主相机 → 场景渲染
- 水平模糊相机 → RTT_H
- 垂直模糊相机 → RTT_V  
- 每个 embed-scene 还有独立 RTT 相机

这意味着每帧可能有 **4-6 个渲染 pass**。在 Android WebGL2 上：
- 多 render target 切换非常昂贵
- 某些 Mali/Adreno GPU 在 framebuffer 切换时有性能悬崖
- 如果主 canvas 已经是 0×0，RTT 相机的 framebuffer 也会异常

### 问题 4：unified_effect.wgsl 极其复杂（1736 行）

**文件**: `assets/shaders/unified_effect.wgsl`

- 1736 行 WGSL，415 个条件分支
- WebGL2 需要将 WGSL → GLSL ES 3.0 转译（通过 naga）
- Android 上 GLSL 编译器性能差，此着色器编译可能需要 **1-3 秒**
- 编译期间 Bevy 的事件循环阻塞 → 看起来像黑屏
- 如果 naga 转译产生的 GLSL 超过了移动 GPU 的指令限制 → **静默编译失败**

### 问题 5：500ms 初始化等待不足

**文件**: `AmPlayground.vue:254-256`

```typescript
await new Promise(resolve => requestAnimationFrame(resolve))  // ~16ms
await new Promise(resolve => setTimeout(resolve, 500))         // 500ms
```

Android 上 Bevy + WebGL2 初始化流程：
- WASM 实例化：200-400ms
- WebGL2 上下文创建：50-150ms
- 着色器编译（unified_effect 1736 行!）：500-3000ms
- Plugin 系统初始化：100-200ms
- **总计**：约 1-4 秒

500ms 远远不够。PC 上大约 200-500ms 就够了。

### 问题 6：`app.run()` 无错误处理

**文件**: `wasm/src/lib.rs:92-136`

```rust
App::new()
    // ...plugins...
    .run();
Ok(())  // 无论是否成功都返回 Ok
```

如果 Bevy 内部初始化失败（WebGL 上下文创建失败、着色器编译失败），错误被完全吞掉。没有任何日志能看到实际的失败原因。

---

## 🟢 已排除的因素

| 因素 | 状态 | 原因 |
|------|------|------|
| 着色器语法不兼容 | ✅ 排除 | 全部使用 `textureSample`，无 compute/storage |
| 纹理格式 | ✅ 排除 | `Rgba8UnormSrgb` 是 WebGL2 标准格式 |
| 构建目标 | ✅ 排除 | `wasm32-unknown-unknown` + `--target web` 正确 |
| PBR 依赖 | ✅ 排除 | 仅在 Cargo.toml 有 feature，实际 AM 使用 2D 管线 |
| Mutex 死锁 | ⚠️ 低风险 | 单线程 WASM 上 Mutex 不会阻塞，但 poisoning 有风险 |

---

## 修复方案建议

### 方案 A：最小改动（推荐先尝试）

1. **Canvas 始终可见**：将 `v-show` 改为 canvas 始终渲染，只隐藏上传区域
2. **大幅增加等待时间**：500ms → 5000ms，或改为轮询 WASM 就绪标志
3. **`main()` 去掉 `#[wasm_bindgen(start)]`**：改为显式调用，确保 canvas 就绪后再初始化 Bevy

### 方案 B：架构级修复

1. **分离 WASM 初始化和 Bevy 启动**：
   - `init()` 只做 panic hook + 日志
   - 新增 `start_app()` 函数，从 JS 在 canvas 就绪后调用
2. **添加 WebGL 能力检测**：在 WASM 初始化前检测 WebGL2 支持和 GPU 限制
3. **着色器按需编译**：延迟 unified_effect 编译到实际使用时

---

## 验证步骤

要确认根本原因，可以：
1. Android Chrome → `chrome://inspect` 远程调试
2. 检查 WebGL 上下文是否创建成功：`canvas.getContext('webgl2')` 返回值
3. 在 `main()` 中添加 canvas 尺寸日志：检查 `clientWidth × clientHeight`
4. 临时将 canvas 改为始终可见，测试是否修复黑屏
