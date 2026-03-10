<template>
  <div class="am-playground">
    <!-- WASM 不可用提示 -->
    <div class="wasm-unavailable" v-if="wasmError">
      <div class="warning-box">
        <h4>{{ i18n.wasmNotBuiltTitle }}</h4>
        <p>{{ i18n.wasmNotBuiltDesc }}</p>
        <pre class="build-cmd">cd wasm && ./build.sh</pre>
        <p class="hint">{{ i18n.wasmNotBuiltHint }}</p>
      </div>
    </div>

    <!-- 文件上传区（WASM 未加载时显示） -->
    <div class="upload-section" v-if="!isLoaded && !wasmError">
      <FileUploader
        :label="i18n.uploadLabel"
        accept=".amproj"
        @file-selected="loadProject"
      />
      <p class="upload-hint">{{ i18n.uploadHint }}</p>
    </div>

    <!-- 加载中 -->
    <div class="loading-section" v-if="isLoading">
      <div class="spinner"></div>
      <span>{{ i18n.loading }}</span>
    </div>

    <!-- 播放区域（加载后显示） -->
    <div class="player-section" v-show="isLoaded && !isLoading">
      <div class="canvas-container" ref="canvasContainer">
        <canvas id="bevy-canvas" ref="canvas"></canvas>
        <!-- 全屏按钮 -->
        <button class="fullscreen-btn" @click="toggleFullscreen" :title="i18n.fullscreen">
          ⛶
        </button>
        <!-- 关闭按钮 -->
        <button class="close-btn" @click="closePlayer" :title="i18n.close">
          ✕
        </button>
      </div>

      <!-- 快捷键提示 -->
      <div class="shortcuts-bar">
        <span class="shortcut">[Space] {{ i18n.playPause }}</span>
        <span class="shortcut">[R] {{ i18n.reset }}</span>
        <span class="shortcut">[←/→] {{ i18n.frameStep }}</span>
        <span class="shortcut">[↑/↓] {{ i18n.speed }}</span>
        <span class="shortcut">[L] {{ i18n.loop }}</span>
      </div>
    </div>

    <!-- 验证报告区域 -->
    <div class="validation-section" v-if="validationReport">
      <h3>📋 Validation Report</h3>
      <ValidationReport :report="validationReport" />
    </div>

    <!-- 调试日志（可折叠） -->
    <details class="debug-logs" v-if="logs.length > 0">
      <summary>🔍 {{ i18n.consoleLogs }} ({{ logs.length }})</summary>
      <pre class="log-content">{{ logs.join('\n') }}</pre>
    </details>

    <!-- WASM 构建信息 -->
    <div class="build-info" v-if="buildInfo">
      <span>WASM Build: {{ buildInfo.build_time }} ({{ buildInfo.git_branch }}@{{ buildInfo.git_hash }}) · {{ formatSize(buildInfo.wasm_size_bytes) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useData, withBase } from 'vitepress'
import FileUploader from './FileUploader.vue'
import ValidationReport from './ValidationReport.vue'
import { useConsoleCapture } from '../composables/useConsoleCapture'

// 获取当前语言
const { lang } = useData()
const isZhHans = computed(() => lang.value === 'zh-Hans' || lang.value.startsWith('zh'))

// 本地化文本
const i18n = computed(() => isZhHans.value ? {
  uploadLabel: '上传 .amproj 文件',
  uploadHint: '上传文件后将自动加载 WASM 播放器',
  wasmNotBuiltTitle: '⚠️ WASM 模块未构建',
  wasmNotBuiltDesc: 'Playground 需要 WASM 模块才能运行。请先构建：',
  wasmNotBuiltHint: '构建完成后刷新页面即可使用。',
  loading: '正在加载 WASM 和项目...',
  fullscreen: '全屏',
  close: '关闭 (刷新页面)',
  playPause: '播放/暂停',
  reset: '重置',
  frameStep: '帧步进',
  speed: '速度',
  loop: '循环',
  consoleLogs: 'Console Logs'
} : {
  uploadLabel: 'Upload .amproj file',
  uploadHint: 'WASM player will load automatically when you upload a file',
  wasmNotBuiltTitle: '⚠️ WASM Module Not Built',
  wasmNotBuiltDesc: 'Playground requires the WASM module to run. Please build it first:',
  wasmNotBuiltHint: 'Refresh the page after the build completes.',
  loading: 'Loading WASM and project...',
  fullscreen: 'Fullscreen',
  close: 'Close (refresh page)',
  playPause: 'Play/Pause',
  reset: 'Reset',
  frameStep: 'Frame Step',
  speed: 'Speed',
  loop: 'Loop',
  consoleLogs: 'Console Logs'
})

// 响应式状态
const canvas = ref<HTMLCanvasElement | null>(null)
const canvasContainer = ref<HTMLElement | null>(null)
const wasmError = ref(false)
const isLoading = ref(false)
const isLoaded = ref(false)

// 存储上传的文件数据，以便在 WASM 加载后使用
let pendingFileBytes: Uint8Array | null = null

// WASM 构建信息
const buildInfo = ref<{ build_time: string; git_hash: string; git_branch: string; wasm_size_bytes: number } | null>(null)

const formatSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const mb = bytes / (1024 * 1024)
  return `${mb.toFixed(1)} MB`
}

// 加载构建信息
onMounted(async () => {
  try {
    const resp = await fetch(withBase('/wasm/build_info.json'))
    if (resp.ok) buildInfo.value = await resp.json()
  } catch { /* ignore */ }
})

// Console 捕获
const { validationReport, logs, clearLogs } = useConsoleCapture()

// 检查 WASM 文件是否存在
const checkWasmExists = async (): Promise<boolean> => {
  try {
    const wasmPath = withBase('/wasm/bevy_alight_motion.js')
    const response = await fetch(wasmPath, { method: 'HEAD' })
    return response.ok
  } catch {
    return false
  }
}

// 加载 WASM 模块
const loadWasm = async (): Promise<boolean> => {
  return new Promise((resolve) => {
    const wasmUrl = withBase('/wasm/bevy_alight_motion.js')
    
    const script = document.createElement('script')
    script.type = 'module'
    script.textContent = `
      import init, * as wasm from '${wasmUrl}';
      await init();
      window.__bevy_wasm = wasm;
      window.dispatchEvent(new CustomEvent('bevy-wasm-loaded'));
    `
    document.head.appendChild(script)

    const timeout = setTimeout(() => {
      console.warn('[Playground] WASM load timeout')
      resolve(false)
    }, 60000) // 60秒超时

    window.addEventListener('bevy-wasm-loaded', () => {
      clearTimeout(timeout)
      console.log('[Playground] WASM module loaded')
      resolve(true)
    }, { once: true })
  })
}

// 上传文件并加载
const loadProject = async (file: File) => {
  // 检查 WASM 是否存在
  const wasmExists = await checkWasmExists()
  if (!wasmExists) {
    console.warn('[Playground] WASM module not found. Run wasm/build.sh to build it.')
    wasmError.value = true
    return
  }

  clearLogs()
  isLoading.value = true

  try {
    // 读取文件
    const arrayBuffer = await file.arrayBuffer()
    pendingFileBytes = new Uint8Array(arrayBuffer)
    console.log(`[Playground] File loaded: ${file.name} (${pendingFileBytes.length} bytes)`)

    // 加载 WASM
    const wasmLoaded = await loadWasm()
    if (!wasmLoaded) {
      throw new Error('WASM load failed')
    }

    // 等待一帧让 Bevy 初始化
    await new Promise(resolve => requestAnimationFrame(resolve))
    await new Promise(resolve => setTimeout(resolve, 500)) // 额外等待 500ms

    // 加载项目
    const wasmModule = (window as any).__bevy_wasm
    if (wasmModule && pendingFileBytes) {
      console.log('[Playground] Loading project into WASM...')
      const success = wasmModule.load_project_from_bytes(pendingFileBytes)
      if (success) {
        isLoaded.value = true
        console.log('[Playground] Project loaded successfully')
      } else {
        throw new Error('Project load failed')
      }
    }
  } catch (error) {
    console.error('[Playground] Error:', error)
    wasmError.value = true
  } finally {
    isLoading.value = false
  }
}

// 关闭播放器（刷新页面以完全卸载 WASM）
const closePlayer = () => {
  if (confirm(isZhHans.value ? '关闭播放器将刷新页面。确定要继续吗？' : 'Closing the player will refresh the page. Are you sure?')) {
    window.location.reload()
  }
}

// 全屏切换
const toggleFullscreen = async () => {
  const container = canvasContainer.value
  if (!container) return

  try {
    if (!document.fullscreenElement) {
      await container.requestFullscreen()
    } else {
      await document.exitFullscreen()
    }
  } catch (err) {
    console.warn('[Playground] Fullscreen error:', err)
  }
}

// 组件卸载时提示用户刷新页面
onUnmounted(() => {
  if (isLoaded.value) {
    console.log('[Playground] Component unmounted. WASM may still be running.')
  }
})
</script>

<style scoped>
.am-playground {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1rem;
}

/* WASM 不可用提示 */
.wasm-unavailable {
  margin-bottom: 1.5rem;
}

.warning-box {
  background: var(--vp-c-yellow-soft);
  border: 1px solid var(--vp-c-yellow-2);
  border-radius: 8px;
  padding: 1.5rem;
}

.warning-box h4 {
  margin: 0 0 0.75rem 0;
  color: var(--vp-c-yellow-1);
}

.warning-box p {
  margin: 0.5rem 0;
  color: var(--vp-c-text-1);
}

.warning-box .hint {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
}

.build-cmd {
  background: var(--vp-c-bg-soft);
  padding: 0.75rem 1rem;
  border-radius: 4px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.875rem;
  overflow-x: auto;
}

/* 上传区域 */
.upload-section {
  margin-bottom: 1.5rem;
}

.upload-hint {
  margin-top: 0.5rem;
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
  text-align: center;
}

/* 加载中 */
.loading-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem;
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  margin-bottom: 1.5rem;
}

.loading-section span {
  margin-top: 1rem;
  color: var(--vp-c-text-2);
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--vp-c-divider);
  border-top-color: var(--vp-c-brand);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* 播放器区域 */
.player-section {
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
}

.canvas-container {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 9;
  background: #000;
  border-radius: 4px;
  overflow: hidden;
}

#bevy-canvas {
  width: 100%;
  height: 100%;
}

/* 全屏和关闭按钮 */
.fullscreen-btn,
.close-btn {
  position: absolute;
  width: 36px;
  height: 36px;
  background: rgba(0, 0, 0, 0.6);
  border: none;
  border-radius: 4px;
  color: white;
  font-size: 18px;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.2s, background 0.2s;
  z-index: 10;
}

.fullscreen-btn {
  bottom: 10px;
  right: 10px;
}

.close-btn {
  top: 10px;
  right: 10px;
}

.fullscreen-btn:hover,
.close-btn:hover {
  opacity: 1;
  background: rgba(0, 0, 0, 0.8);
}

/* 快捷键提示 */
.shortcuts-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-top: 0.75rem;
  padding: 0.5rem;
  font-size: 0.8rem;
  color: var(--vp-c-text-2);
}

.shortcut {
  background: var(--vp-c-bg);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid var(--vp-c-divider);
}

/* 验证报告区域 */
.validation-section {
  margin-bottom: 1.5rem;
}

.validation-section h3 {
  margin-bottom: 0.75rem;
  font-size: 1.1rem;
}

/* 调试日志 */
.debug-logs {
  margin-top: 1rem;
}

.debug-logs summary {
  cursor: pointer;
  padding: 0.5rem;
  background: var(--vp-c-bg-soft);
  border-radius: 4px;
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
}

.log-content {
  margin-top: 0.5rem;
  padding: 1rem;
  background: #1a1a2e;
  color: #e0e0e0;
  border-radius: 4px;
  font-size: 0.75rem;
  max-height: 300px;
  overflow: auto;
}

/* 响应式 */
@media (max-width: 640px) {
  .shortcuts-bar {
    font-size: 0.7rem;
  }
  
  .shortcut {
    padding: 0.2rem 0.4rem;
  }
}

/* 构建信息 */
.build-info {
  margin-top: 1rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  color: var(--vp-c-text-3);
  text-align: right;
}
</style>
