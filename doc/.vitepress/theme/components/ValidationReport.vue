<template>
  <div class="validation-report" v-if="report">
    <!-- 标题分隔线 -->
    <div class="header-bar">========================================</div>

    <!-- 场景统计 -->
    <div class="section stats-section">
      <div class="line">
        <span class="label">[AM Validation]</span>
        <span class="value">{{ report.stats.total_layers }} layers total</span>
      </div>
      <div class="line indent">
        <span class="bullet">·</span>
        <span>
          Shape: {{ report.stats.shape_count }},
          Text: {{ report.stats.text_count }},
          Image: {{ report.stats.image_count }},
          Null: {{ report.stats.null_count }},
          Embed: {{ report.stats.embed_count }}
        </span>
      </div>
      <div class="line indent" v-if="hasUnsupportedLayerCounts">
        <span class="bullet warning">·</span>
        <span class="warning">
          Audio: {{ report.stats.audio_count }},
          Video: {{ report.stats.video_count }},
          Camera: {{ report.stats.camera_count }}
        </span>
        <span class="note">(unsupported)</span>
      </div>
    </div>

    <!-- 支持的效果 -->
    <div class="section effects-section" v-if="report.supported_effects_used?.length">
      <div class="line">
        <span class="label">[AM Validation]</span>
        <span class="value">
          Effects in use:
          <span class="count-full">{{ fullSupportCount }} full</span>,
          <span class="count-partial">{{ partialSupportCount }} partial</span>
        </span>
      </div>
      <div
        class="line indent"
        v-for="effect in report.supported_effects_used"
        :key="effect.id"
      >
        <span :class="['icon', effect.support_level.toLowerCase()]">
          {{ effect.support_level === 'Full' ? '✓' : '⚠' }}
        </span>
        <span class="effect-name">{{ effect.display_name }}</span>
        <span class="usage">- {{ effect.usage_count }} usage(s)</span>
        <span class="partial-note" v-if="effect.support_level === 'Partial'">
          (partial support)
        </span>
      </div>
    </div>

    <!-- 不支持的效果 -->
    <div class="section unsupported-section" v-if="groupedUnsupportedEffects.length">
      <div class="line">
        <span class="label">[AM Validation]</span>
        <span class="value error">
          Unsupported effects ({{ groupedUnsupportedEffects.length }} unique types):
        </span>
      </div>
      <div
        class="line indent"
        v-for="effect in groupedUnsupportedEffects"
        :key="effect.id"
      >
        <span class="icon error">✗</span>
        <span class="error">'{{ effect.label }}'</span>
        <span class="id">({{ effect.id }})</span>
        <span class="usage">- {{ effect.count }} usage(s)</span>
      </div>
    </div>

    <!-- 不支持的图层 -->
    <div class="section unsupported-section" v-if="report.unsupported_layers?.length">
      <div class="line">
        <span class="label">[AM Validation]</span>
        <span class="value error">
          Unsupported layer types ({{ report.unsupported_layers.length }}):
        </span>
      </div>
      <div
        class="line indent"
        v-for="layer in report.unsupported_layers"
        :key="layer.id"
      >
        <span class="icon error">✗</span>
        <span class="layer-type error">{{ layer.layer_type }}</span>
        <span>'{{ layer.label }}'</span>
        <span class="id">(id={{ layer.id }})</span>
        <span class="note">- will be skipped</span>
      </div>
    </div>

    <!-- 分隔线 -->
    <div class="divider">----------------------------------------</div>

    <!-- 摘要 -->
    <div class="section summary">
      <div class="line" :class="hasIssues ? 'warning-line' : 'success-line'">
        <span class="label">[AM Validation]</span>
        <span v-if="hasIssues" class="value warning">
          ⚠ {{ totalIssues }} unsupported feature(s) will be skipped
        </span>
        <span v-else class="value success">
          ✓ All features in this project are fully supported
        </span>
      </div>
    </div>

    <!-- 结束分隔线 -->
    <div class="header-bar">========================================</div>

    <!-- 提交 Issue 链接 -->
    <div class="actions" v-if="hasIssues">
      <a :href="issueUrl" target="_blank" class="submit-issue-btn">
        📝 Request Feature Support
      </a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ValidationReport } from '../composables/useConsoleCapture'

const props = defineProps<{
  report: ValidationReport | null
}>()

const fullSupportCount = computed(() =>
  props.report?.supported_effects_used?.filter(e => e.support_level === 'Full').length ?? 0
)

const partialSupportCount = computed(() =>
  props.report?.supported_effects_used?.filter(e => e.support_level === 'Partial').length ?? 0
)

const hasUnsupportedLayerCounts = computed(() => {
  if (!props.report) return false
  const s = props.report.stats
  return s.audio_count > 0 || s.video_count > 0 || s.camera_count > 0
})

const groupedUnsupportedEffects = computed(() => {
  if (!props.report?.unsupported_effects) return []
  const groups = new Map<string, { id: string; label: string; count: number }>()
  for (const effect of props.report.unsupported_effects) {
    const existing = groups.get(effect.id)
    if (existing) {
      existing.count++
    } else {
      groups.set(effect.id, { id: effect.id, label: effect.label, count: 1 })
    }
  }
  return Array.from(groups.values())
})

const totalIssues = computed(() =>
  (props.report?.unsupported_effects?.length ?? 0) +
  (props.report?.unsupported_layers?.length ?? 0)
)

const hasIssues = computed(() => totalIssues.value > 0)

const issueUrl = computed(() => {
  const effects = groupedUnsupportedEffects.value.map(e => `- ${e.label} (${e.id})`).join('\n')
  const layers = props.report?.unsupported_layers?.map(l => `- ${l.layer_type}`).join('\n') ?? ''
  const body = encodeURIComponent(
    `## Unsupported Effects\n${effects || 'None'}\n\n## Unsupported Layers\n${layers || 'None'}`
  )
  return `https://github.com/Bli-AIk/bevy_alight_motion/issues/new?title=Feature%20Request&body=${body}`
})
</script>

<style scoped>
.validation-report {
  font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
  font-size: 0.875rem;
  background: #1a1a2e;
  border-radius: 8px;
  padding: 1rem;
  color: #e0e0e0;
  overflow-x: auto;
  line-height: 1.6;
}

.header-bar,
.divider {
  color: #00d4ff;
}

.section {
  margin: 0.5rem 0;
}

.line {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.indent {
  padding-left: 1.5rem;
}

.label {
  color: #00d4ff;
  font-weight: bold;
}

.bullet {
  color: #666;
}

.bullet.warning {
  color: #ffcc00;
}

.icon.full {
  color: #00ff88;
}

.icon.partial {
  color: #ffcc00;
}

.icon.error {
  color: #ff4444;
}

.effect-name {
  color: #e0e0e0;
}

.usage {
  color: #888;
}

.partial-note {
  color: #ffcc00;
  font-style: italic;
}

.count-full {
  color: #00ff88;
}

.count-partial {
  color: #ffcc00;
}

.error {
  color: #ff4444;
}

.warning {
  color: #ffcc00;
}

.success {
  color: #00ff88;
  font-weight: bold;
}

.layer-type {
  font-weight: bold;
}

.id {
  color: #888;
}

.note {
  color: #666;
  font-style: italic;
}

.warning-line .value {
  color: #ff4444;
  font-weight: bold;
}

.success-line .value {
  color: #00ff88;
  font-weight: bold;
}

.actions {
  margin-top: 1rem;
}

.submit-issue-btn {
  display: inline-block;
  padding: 0.5rem 1rem;
  background: #4a4a6a;
  color: #fff;
  border-radius: 4px;
  text-decoration: none;
  transition: background 0.2s;
}

.submit-issue-btn:hover {
  background: #5a5a7a;
}

/* 暗色主题适配 */
.dark .validation-report {
  background: #0d1117;
}

/* 亮色主题适配 */
.light .validation-report {
  background: #f6f8fa;
  color: #24292f;
}

.light .label {
  color: #0969da;
}

.light .header-bar,
.light .divider {
  color: #0969da;
}

.light .count-full,
.light .icon.full,
.light .success {
  color: #1a7f37;
}

.light .count-partial,
.light .icon.partial,
.light .partial-note,
.light .warning,
.light .bullet.warning {
  color: #9a6700;
}

.light .icon.error,
.light .error {
  color: #cf222e;
}

.light .id,
.light .usage,
.light .note {
  color: #57606a;
}

.light .submit-issue-btn {
  background: #0969da;
}

.light .submit-issue-btn:hover {
  background: #0860ca;
}
</style>
