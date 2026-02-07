<template>
  <div class="file-uploader">
    <div
      class="drop-zone"
      :class="{ 'drag-over': isDragOver, 'has-file': selectedFile }"
      @dragover.prevent="isDragOver = true"
      @dragleave="isDragOver = false"
      @drop.prevent="onDrop"
      @click="openFilePicker"
    >
      <input
        ref="fileInput"
        type="file"
        :accept="accept"
        @change="onFileChange"
        hidden
      />

      <div class="content" v-if="!selectedFile">
        <span class="icon">📁</span>
        <span class="text">{{ label }}</span>
        <span class="hint">拖放文件或点击选择</span>
      </div>

      <div class="file-info" v-else>
        <span class="icon">✓</span>
        <span class="name">{{ selectedFile.name }}</span>
        <span class="size">({{ formatSize(selectedFile.size) }})</span>
        <button class="clear-btn" @click.stop="clearFile">✕</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  label: string
  accept: string
}>()

const emit = defineEmits<{
  (e: 'file-selected', file: File): void
  (e: 'file-cleared'): void
}>()

const fileInput = ref<HTMLInputElement | null>(null)
const isDragOver = ref(false)
const selectedFile = ref<File | null>(null)

const openFilePicker = () => {
  fileInput.value?.click()
}

const onFileChange = (event: Event) => {
  const input = event.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    selectFile(input.files[0])
  }
}

const onDrop = (event: DragEvent) => {
  isDragOver.value = false
  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    const file = event.dataTransfer.files[0]
    // 检查文件类型
    if (props.accept && !matchAccept(file, props.accept)) {
      alert(`请选择 ${props.accept} 类型的文件`)
      return
    }
    selectFile(file)
  }
}

const selectFile = (file: File) => {
  selectedFile.value = file
  emit('file-selected', file)
}

const clearFile = () => {
  selectedFile.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
  emit('file-cleared')
}

const matchAccept = (file: File, accept: string): boolean => {
  const acceptTypes = accept.split(',').map(t => t.trim())
  for (const type of acceptTypes) {
    if (type.startsWith('.')) {
      // 扩展名匹配
      if (file.name.toLowerCase().endsWith(type.toLowerCase())) {
        return true
      }
    } else if (type.includes('*')) {
      // MIME 类型通配符
      const [mainType] = type.split('/')
      if (file.type.startsWith(mainType + '/')) {
        return true
      }
    } else {
      // 完整 MIME 类型
      if (file.type === type) {
        return true
      }
    }
  }
  return false
}

const formatSize = (bytes: number): string => {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}
</script>

<style scoped>
.file-uploader {
  width: 100%;
}

.drop-zone {
  border: 2px dashed #4a4a6a;
  border-radius: 8px;
  padding: 1.5rem;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease;
  background: var(--vp-c-bg-soft);
}

.drop-zone:hover {
  border-color: #00d4ff;
  background: var(--vp-c-bg-soft);
}

.drop-zone.drag-over {
  border-color: #00ff88;
  background: rgba(0, 255, 136, 0.1);
}

.drop-zone.has-file {
  border-color: #00ff88;
  border-style: solid;
}

.content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.content .icon {
  font-size: 2rem;
}

.content .text {
  font-weight: 500;
  color: var(--vp-c-text-1);
}

.content .hint {
  font-size: 0.875rem;
  color: var(--vp-c-text-3);
}

.file-info {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
}

.file-info .icon {
  color: #00ff88;
  font-size: 1.2rem;
}

.file-info .name {
  font-weight: 500;
  color: var(--vp-c-text-1);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-info .size {
  color: var(--vp-c-text-3);
  font-size: 0.875rem;
}

.clear-btn {
  background: #ff4444;
  color: white;
  border: none;
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
  cursor: pointer;
  font-size: 0.75rem;
  margin-left: 0.5rem;
}

.clear-btn:hover {
  background: #ff6666;
}
</style>
