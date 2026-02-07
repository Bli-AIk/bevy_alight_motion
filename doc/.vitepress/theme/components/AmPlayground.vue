<template>
  <div class="am-playground">
    <!-- 状态提示 -->
    <div class="status-bar" :class="statusClass">
      <span class="status-icon">{{ statusIcon }}</span>
      <span class="status-text">{{ statusText }}</span>
    </div>

    <!-- WASM 不可用提示 -->
    <div class="wasm-unavailable" v-if="wasmError">
      <div class="warning-box">
        <h4>⚠️ WASM 模块未构建</h4>
        <p>Playground 需要 WASM 模块才能运行。请先构建：</p>
        <pre class="build-cmd">cd wasm && ./build.sh</pre>
        <p class="hint">构建完成后刷新页面即可使用。</p>
      </div>
    </div>

    <!-- 文件上传区 -->
    <div class="upload-section" v-if="!wasmError">
      <FileUploader
        label="上传 .amproj 文件"
        accept=".amproj"
        @file-selected="loadProject"
        @file-cleared="clearProject"
      />
    </div>

    <!-- 播放区域 -->
    <div class="player-section" v-show="isLoaded">
      <div class="canvas-container">
        <canvas id="bevy-canvas" ref="canvas"></canvas>
        <div class="loading-overlay" v-if="isLoading">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>
      </div>

      <!-- 控制面板 -->
      <div class="controls">
        <div class="playback-controls">
          <button @click="reset" :disabled="!isLoaded" title="重置">⏮</button>
          <button @click="togglePlay" :disabled="!isLoaded" :title="isPlaying ? '暂停' : '播放'">
            {{ isPlaying ? '⏸' : '▶' }}
          </button>
          <button @click="stepForward" :disabled="!isLoaded" title="下一帧">⏭</button>
        </div>

        <div class="time-display" v-if="isLoaded">
          <span>{{ formatFrame(currentFrame) }}</span>
          <span>/</span>
          <span>{{ formatFrame(totalFrames) }}</span>
        </div>

        <div class="speed-control" v-if="isLoaded">
          <label>速度:</label>
          <select v-model="playbackSpeed" @change="updateSpeed">
            <option value="0.25">0.25x</option>
            <option value="0.5">0.5x</option>
            <option value="1">1x</option>
            <option value="2">2x</option>
          </select>
        </div>
      </div>

      <!-- 时间轴 -->
      <div class="timeline" v-if="isLoaded">
        <input
          type="range"
          v-model.number="currentFrame"
          :max="totalFrames"
          @input="seek"
        />
      </div>
    </div>

    <!-- 验证报告区域 -->
    <div class="validation-section" v-if="validationReport">
      <h3>📋 Validation Report</h3>
      <ValidationReport :report="validationReport" />
    </div>

    <!-- 调试日志（可折叠） -->
    <details class="debug-logs" v-if="logs.length > 0">
      <summary>🔍 Console Logs ({{ logs.length }})</summary>
      <pre class="log-content">{{ logs.join('\n') }}</pre>
    </details>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import FileUploader from './FileUploader.vue'
import ValidationReport from './ValidationReport.vue'
import { useConsoleCapture } from '../composables/useConsoleCapture'

// WASM 模块引用
let wasmModule: any = null

// 响应式状态
const canvas = ref<HTMLCanvasElement | null>(null)
const isWasmLoaded = ref(false)
const wasmError = ref(false)
const isLoading = ref(false)
const isLoaded = ref(false)
const isPlaying = ref(false)
const currentFrame = ref(0)
const totalFrames = ref(0)
const playbackSpeed = ref(1)

// Console 捕获
const { validationReport, logs, clearLogs } = useConsoleCapture()

// 计算属性
const statusClass = computed(() => {
  if (isLoading.value) return 'loading'
  if (isLoaded.value) return 'loaded'
  if (!isWasmLoaded.value) return 'init'
  return 'ready'
})

const statusIcon = computed(() => {
  if (isLoading.value) return '⏳'
  if (isLoaded.value) return '✅'
  if (!isWasmLoaded.value) return '🔄'
  return '📂'
})

const statusText = computed(() => {
  if (isLoading.value) return '正在加载项目...'
  if (isLoaded.value) return '项目已加载'
  if (!isWasmLoaded.value) return '正在初始化 WASM...'
  return '准备就绪，请上传 .amproj 文件'
})

// 生命周期
onMounted(async () => {
  try {
    // 动态导入 WASM 模块
    // 注意：WASM 文件需要先通过 wasm/build.sh 构建到 doc/public/wasm/
    const wasmUrl = '/wasm/bevy_alight_motion.js'

    // 检查 WASM 文件是否存在
    const response = await fetch(wasmUrl, { method: 'HEAD' })
    if (!response.ok) {
      console.warn('[Playground] WASM module not found. Run wasm/build.sh to build it.')
      wasmError.value = true
      return
    }

    const wasm = await import(/* @vite-ignore */ wasmUrl)
    await wasm.default()
    wasmModule = wasm
    isWasmLoaded.value = true
    console.log('[Playground] WASM module loaded')
  } catch (error) {
    console.warn('[Playground] Failed to load WASM:', error)
    console.warn('[Playground] This is expected in dev mode. Run wasm/build.sh to build the WASM module.')
    wasmError.value = true
  }
})

onUnmounted(() => {
  // 清理资源
  wasmModule = null
})

// 方法
const loadProject = async (file: File) => {
  if (!wasmModule) {
    alert('WASM 模块尚未加载，请稍候')
    return
  }

  clearLogs()
  isLoading.value = true

  try {
    const arrayBuffer = await file.arrayBuffer()
    const bytes = new Uint8Array(arrayBuffer)

    console.log(`[Playground] Loading project: ${file.name} (${bytes.length} bytes)`)

    const success = wasmModule.load_project_from_bytes(bytes)

    if (success) {
      isLoaded.value = true

      // 获取项目信息
      const state = wasmModule.get_state()
      if (state) {
        totalFrames.value = state.total_frames || 0
        currentFrame.value = 0
      }
    } else {
      alert('项目加载失败')
    }
  } catch (error) {
    console.error('[Playground] Load error:', error)
    alert('项目加载出错: ' + error)
  } finally {
    isLoading.value = false
  }
}

const clearProject = () => {
  isLoaded.value = false
  isPlaying.value = false
  currentFrame.value = 0
  totalFrames.value = 0
  clearLogs()
}

const togglePlay = () => {
  if (!wasmModule) return
  if (isPlaying.value) {
    wasmModule.pause()
  } else {
    wasmModule.play()
  }
  isPlaying.value = !isPlaying.value
}

const reset = () => {
  if (!wasmModule) return
  wasmModule.seek(0)
  currentFrame.value = 0
  isPlaying.value = false
}

const stepForward = () => {
  if (!wasmModule) return
  currentFrame.value = Math.min(currentFrame.value + 1, totalFrames.value)
  wasmModule.seek(currentFrame.value)
}

const seek = () => {
  if (!wasmModule) return
  wasmModule.seek(currentFrame.value)
}

const updateSpeed = () => {
  // TODO: 实现速度控制
  console.log(`[Playground] Speed changed to ${playbackSpeed.value}x`)
}

const formatFrame = (frame: number): string => {
  return frame.toString().padStart(4, '0')
}
</script>

<style scoped>
.am-playground {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1rem;
}

/* 状态栏 */
.status-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  margin-bottom: 1rem;
  font-weight: 500;
}

.status-bar.init {
  background: var(--vp-c-yellow-soft);
  color: var(--vp-c-yellow-1);
}

.status-bar.loading {
  background: var(--vp-c-blue-soft);
  color: var(--vp-c-blue-1);
}

.status-bar.ready {
  background: var(--vp-c-gray-soft);
  color: var(--vp-c-text-2);
}

.status-bar.loaded {
  background: var(--vp-c-green-soft);
  color: var(--vp-c-green-1);
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

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.7);
  color: white;
  gap: 0.5rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* 控制面板 */
.controls {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-top: 0.75rem;
  padding: 0.5rem;
}

.playback-controls {
  display: flex;
  gap: 0.25rem;
}

.playback-controls button {
  padding: 0.5rem 0.75rem;
  font-size: 1.2rem;
  background: var(--vp-c-brand);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.2s;
}

.playback-controls button:hover:not(:disabled) {
  background: var(--vp-c-brand-dark);
}

.playback-controls button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.time-display {
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
}

.speed-control {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-left: auto;
}

.speed-control label {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
}

.speed-control select {
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
}

/* 时间轴 */
.timeline {
  margin-top: 0.5rem;
}

.timeline input[type='range'] {
  width: 100%;
  cursor: pointer;
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
  .controls {
    flex-wrap: wrap;
  }

  .speed-control {
    margin-left: 0;
    width: 100%;
  }
}
</style>
