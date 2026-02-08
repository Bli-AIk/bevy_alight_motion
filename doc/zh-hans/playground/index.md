---
title: 试玩场
---

# 🎮 试玩场

<script setup>
import { onMounted } from 'vue'

onMounted(() => {
  // 确保在客户端渲染
})
</script>

在这里，你可以上传 Alight Motion 项目文件（`.amproj`），直接在浏览器中预览渲染效果。

<ClientOnly>
  <AmPlayground />
</ClientOnly>

## 使用说明

1. **上传文件**：点击上传区域或拖放 `.amproj` 文件
2. **查看报告**：加载后会显示 Validation Report，告诉你哪些特性被支持
3. **控制播放**：使用播放控制按钮和时间轴

## 注意事项

- 此 Playground 运行在 WebGL2 单线程模式下，性能可能低于原生版本
- 某些特性（如音频、视频图层）目前不支持
- 大型项目加载可能需要较长时间

## 支持的特性

详见 [已实现特性列表](/zh-hans/implemented-features)
