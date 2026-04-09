<script setup lang="ts">
import { ref } from 'vue'

const emit = defineEmits<{
  upload: [file: File]
}>()

const isDragging = ref(false)

function onFileChange(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) emit('upload', file)
}

function onDrop(event: DragEvent) {
  isDragging.value = false
  const file = event.dataTransfer?.files[0]
  if (
    file &&
    (file.name.endsWith('.ttf') ||
      file.name.endsWith('.otf') ||
      file.name.endsWith('.woff2') ||
      file.name.endsWith('.woff'))
  ) {
    emit('upload', file)
  }
}

function onDragOver() {
  isDragging.value = true
}

function onDragLeave() {
  isDragging.value = false
}
</script>

<template>
  <label
    class="upload-area"
    :class="{ dragging: isDragging }"
    @dragover.prevent="onDragOver"
    @dragleave="onDragLeave"
    @drop.prevent="onDrop"
  >
    <input
      type="file"
      accept=".ttf,.otf,.woff,.woff2"
      class="sr-only"
      @change="onFileChange"
    />
    <span>Drop a font file here or <strong>click to upload</strong></span>
    <span class="hint">.ttf · .otf · .woff · .woff2</span>
  </label>
</template>

<style scoped>
.upload-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  padding: 2rem;
  background: var(--color-surface);
  border: 2px dashed var(--color-border);
  border-radius: var(--radius);
  cursor: pointer;
  text-align: center;
  color: var(--color-muted);
  transition:
    border-color 0.15s,
    background 0.15s;
}

.upload-area:hover,
.upload-area.dragging {
  border-color: var(--color-accent);
  background: #ececec;
}

.hint {
  font-size: 0.8rem;
  color: #999;
}
</style>
