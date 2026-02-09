---
layout: home

hero:
  name: "bevy_alight_motion"
  text: "A Bevy plugin for importing assets directly from Alight Motion project files."
  tagline: "The high-performance bridge between professional animation design and the Rust ecosystem."
  actions:
    - theme: brand
      text: Quick Start
      link: /en/guide/introduction
    - theme: alt
      text: View Examples
      link: /en/examples/
---

<script setup>
import { onMounted } from 'vue'

onMounted(() => {
  // 检测用户语言并重定向到相应的语言版本
  const lang = navigator.language || navigator.userLanguage
  const isZhHans = lang.startsWith('zh')
  const basePath = '/bevy_alight_motion'
  const targetPath = isZhHans ? `${basePath}/zh-hans/` : `${basePath}/en/`
  
  // 只在根路径时重定向，避免循环
  if (window.location.pathname === basePath + '/' || window.location.pathname === basePath) {
    window.location.replace(targetPath)
  }
})
</script>

<!-- Single, centered install box -->
<div class="install-widget">
  <span class="install-cmd">cargo add bevy_alight_motion</span>
  <svg class="install-copy" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
</div>
