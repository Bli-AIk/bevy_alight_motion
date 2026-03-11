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
    <div class="upload-section" v-if="!isLoaded && !isLoading && !wasmError">
      <FileUploader
        :label="i18n.uploadLabel"
        accept=".amproj"
        @file-selected="loadProject"
      />
      <p class="upload-hint">{{ i18n.uploadHint }}</p>
    </div>

    <!-- 加载中（canvas 区域已可见，叠加 loading overlay） -->
    <div class="player-section" v-show="isLoading || isLoaded">
      <div class="canvas-container" ref="canvasContainer">
        <canvas id="bevy-canvas" ref="canvas"></canvas>
        <!-- Loading overlay (on top of canvas) -->
        <div class="loading-overlay" v-if="isLoading">
          <div class="spinner"></div>
          <span>{{ i18n.loading }}</span>
        </div>
        <!-- 全屏按钮 -->
        <button v-if="isLoaded" class="fullscreen-btn" @click="toggleFullscreen" :title="i18n.fullscreen">
          ⛶
        </button>
        <!-- 关闭按钮 -->
        <button v-if="isLoaded" class="close-btn" @click="closePlayer" :title="i18n.close">
          ✕
        </button>
      </div>

      <!-- 快捷键提示 -->
      <div class="shortcuts-bar" v-if="isLoaded">
        <span class="shortcut">[Space] {{ i18n.playPause }}</span>
        <span class="shortcut">[R] {{ i18n.reset }}</span>
        <span class="shortcut">[←/→] {{ i18n.frameStep }}</span>
        <span class="shortcut">[↑/↓] {{ i18n.speed }}</span>
        <span class="shortcut">[L] {{ i18n.loop }}</span>
        <button class="download-logs-btn" @click="downloadLogs" :title="i18n.downloadLogs">
          📥 {{ i18n.downloadLogs }}
        </button>
      </div>
    </div>

    <!-- 验证报告区域 -->
    <div class="validation-section" v-if="validationReport">
      <h3>📋 Validation Report</h3>
      <ValidationReport :report="validationReport" />
    </div>

    <!-- 运行时日志（默认折叠，仅显示最后20行） -->
    <details class="runtime-logs" :open="showLogs">
      <summary>📜 {{ i18n.runtimeLogs }} ({{ totalLogLines }})</summary>
      <div class="log-controls">
        <button @click="toggleLogDisplay">{{ showAllLogs ? i18n.showLast20 : i18n.showAll }}</button>
        <button @click="downloadLogs">📥 {{ i18n.downloadLogs }}</button>
      </div>
      <pre class="log-content">{{ displayLogs }}</pre>
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
  runtimeLogs: '运行日志',
  showLast20: '显示最近20行',
  showAll: '显示全部',
  downloadLogs: '下载日志'
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
  runtimeLogs: 'Runtime Logs',
  showLast20: 'Show Last 20',
  showAll: 'Show All',
  downloadLogs: 'Download Logs'
})

// 响应式状态
const canvas = ref<HTMLCanvasElement | null>(null)
const canvasContainer = ref<HTMLElement | null>(null)
const wasmError = ref(false)
const isLoading = ref(false)
const isLoaded = ref(false)

// 运行时日志状态
const runtimeLogs = ref<string[]>([])
const showLogs = ref(false)
const showAllLogs = ref(false)

// 计算显示的日志
const totalLogLines = computed(() => runtimeLogs.value.length)
const displayLogs = computed(() => {
  const logs = runtimeLogs.value
  if (showAllLogs.value || logs.length <= 20) {
    return logs.join('\n')
  }
  return logs.slice(-20).join('\n')
})

const toggleLogDisplay = () => {
  showAllLogs.value = !showAllLogs.value
}

// 更新日志（定时从 WASM 获取）
let logUpdateInterval: number | null = null
const updateLogs = () => {
  const wasmModule = (window as any).__bevy_wasm
  if (wasmModule && wasmModule.get_logs) {
    const logs = wasmModule.get_logs()
    if (logs) {
      runtimeLogs.value = logs.split('\n').filter(l => l.length > 0)
    }
  }
}

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

  // 读取文件
  const arrayBuffer = await file.arrayBuffer()
  pendingFileBytes = new Uint8Array(arrayBuffer)
  console.log(`[Playground] File loaded: ${file.name} (${pendingFileBytes.length} bytes)`)

  // 显示 canvas 区域（loading overlay 覆盖在上面）
  isLoading.value = true

  // 等待两帧，确保 canvas 已渲染并有非零尺寸
  await new Promise(resolve => requestAnimationFrame(resolve))
  await new Promise(resolve => requestAnimationFrame(resolve))

  const canvasEl = document.getElementById('bevy-canvas')
  if (canvasEl) {
    console.log(`[Playground] Canvas ready: ${canvasEl.clientWidth}x${canvasEl.clientHeight}`)
  }

  try {
    // 加载 WASM（init 只做轻量初始化，不启动 Bevy）
    const wasmLoaded = await loadWasm()
    if (!wasmLoaded) {
      throw new Error('WASM load failed')
    }

    // 启动 Bevy（此时 canvas 已可见且有尺寸）
    const wasmModule = (window as any).__bevy_wasm
    if (wasmModule && wasmModule.start_app) {
      console.log('[Playground] Calling start_app()...')
      wasmModule.start_app()
    } else {
      throw new Error('start_app() not found in WASM module')
    }

    // 等待 Bevy 初始化（移动端需更长时间）
    const isMobile = /Android|iPhone|iPad/i.test(navigator.userAgent)
    const initDelay = isMobile ? 3000 : 1000
    console.log(`[Playground] Waiting ${initDelay}ms for Bevy init (mobile: ${isMobile})...`)
    await new Promise(resolve => setTimeout(resolve, initDelay))

    // 加载项目
    if (wasmModule && pendingFileBytes) {
      console.log('[Playground] Loading project into WASM...')
      const success = wasmModule.load_project_from_bytes(pendingFileBytes)
      if (success) {
        isLoaded.value = true
        console.log('[Playground] Project loaded successfully')
        
        // 启动日志更新定时器
        showLogs.value = true
        logUpdateInterval = window.setInterval(() => {
          updateLogs()
        }, 1000)
      } else {
        throw new Error('Project load failed')
      }
    }
  } catch (error) {
    console.error('[Playground] Error:', error)
    showLogs.value = true
    updateLogs()
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

// 下载运行时日志
const downloadLogs = () => {
  const wasmModule = (window as any).__bevy_wasm
  if (wasmModule && wasmModule.download_logs) {
    wasmModule.download_logs()
  } else {
    console.warn('[Playground] download_logs not available')
  }
}

// 组件卸载时提示用户刷新页面
onUnmounted(() => {
  if (logUpdateInterval) {
    clearInterval(logUpdateInterval)
  }
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

/* 加载中 overlay (覆盖在 canvas 上) */
.loading-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.7);
  z-index: 20;
  border-radius: 4px;
}

.loading-overlay span {
  margin-top: 1rem;
  color: #e0e0e0;
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
  touch-action: none;
}

#bevy-canvas {
  width: 100%;
  height: 100%;
  touch-action: none;
  -webkit-touch-callout: none;
  -webkit-user-select: none;
  user-select: none;
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

.download-logs-btn {
  background: var(--vp-c-bg);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid var(--vp-c-divider);
  cursor: pointer;
  font-size: 0.8rem;
  color: var(--vp-c-text-2);
  transition: background 0.2s, color 0.2s;
}

.download-logs-btn:hover {
  background: var(--vp-c-brand);
  color: var(--vp-c-bg);
}

/* 验证报告区域 */
.validation-section {
  margin-bottom: 1.5rem;
}

.validation-section h3 {
  margin-bottom: 0.75rem;
  font-size: 1.1rem;
}

/* 运行时日志 */
.runtime-logs {
  margin-top: 1rem;
}

.runtime-logs summary {
  cursor: pointer;
  padding: 0.5rem;
  background: var(--vp-c-bg-soft);
  border-radius: 4px;
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
}

.log-controls {
  display: flex;
  gap: 0.5rem;
  margin: 0.5rem 0;
}

.log-controls button {
  background: var(--vp-c-bg);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid var(--vp-c-divider);
  cursor: pointer;
  font-size: 0.75rem;
  color: var(--vp-c-text-2);
  transition: background 0.2s;
}

.log-controls button:hover {
  background: var(--vp-c-brand);
  color: var(--vp-c-bg);
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
