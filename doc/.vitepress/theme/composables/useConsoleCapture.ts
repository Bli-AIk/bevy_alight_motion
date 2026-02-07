/**
 * useConsoleCapture - 捕获 WASM console 日志并解析 ValidationReport
 *
 * WASM 端输出格式：
 * - JSON 报告: "[AM_VALIDATION_JSON]{...}"
 * - 人类可读日志: "[AM Validation] ..."
 */
import { ref, onMounted, onUnmounted } from 'vue'

export interface SceneStats {
  total_layers: number
  shape_count: number
  text_count: number
  image_count: number
  null_count: number
  embed_count: number
  audio_count: number
  video_count: number
  camera_count: number
  bookmark_count: number
}

export interface EffectUsage {
  id: string
  label: string
  display_name: string
  support_level: 'Full' | 'Partial' | 'Ignored'
  usage_count: number
}

export interface UnsupportedEffect {
  id: string
  label: string
  layer_label: string
  layer_id: number
}

export interface UnsupportedLayer {
  layer_type: string
  label: string
  id: number
}

export interface ValidationReport {
  stats: SceneStats
  supported_effects_used: EffectUsage[]
  unsupported_effects: UnsupportedEffect[]
  unsupported_layers: UnsupportedLayer[]
}

export function useConsoleCapture() {
  const validationReport = ref<ValidationReport | null>(null)
  const logs = ref<string[]>([])
  const rawLogs = ref<string[]>([])

  let originalLog: typeof console.log
  let originalWarn: typeof console.warn

  const captureLog = (...args: any[]) => {
    const message = args.map(a => String(a)).join(' ')
    rawLogs.value.push(message)

    // 检查是否是 JSON 格式的验证报告
    if (message.startsWith('[AM_VALIDATION_JSON]')) {
      const json = message.slice('[AM_VALIDATION_JSON]'.length)
      try {
        validationReport.value = JSON.parse(json)
      } catch (e) {
        console.error('Failed to parse validation report:', e)
      }
    }

    // 捕获 AM Validation 相关日志用于显示
    if (message.includes('[AM Validation]') || message.includes('========') || message.includes('--------')) {
      logs.value.push(message)
    }

    originalLog.apply(console, args)
  }

  onMounted(() => {
    originalLog = console.log
    originalWarn = console.warn

    console.log = captureLog
    console.warn = captureLog
  })

  onUnmounted(() => {
    if (originalLog) console.log = originalLog
    if (originalWarn) console.warn = originalWarn
  })

  const clearLogs = () => {
    logs.value = []
    rawLogs.value = []
    validationReport.value = null
  }

  return {
    validationReport,
    logs,
    rawLogs,
    clearLogs
  }
}
