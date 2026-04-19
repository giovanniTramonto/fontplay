<script setup lang="ts">
import ClearButton from '@/components/ClearButton.vue'
import FontUpload from '@/components/FontUpload.vue'
import type { FontInfo } from '@/composables/useFontWasm'

defineProps<{
  isLoading?: boolean
  blendFontName?: string | null
  blendFontInfo?: FontInfo | null
}>()

const blendFactor = defineModel<number>('blendFactor', { default: 0.5 })

const emit = defineEmits<{
  upload: [file: File]
  blend: []
  removeBlendFont: []
}>()
</script>

<template>
  <div class="blend-buttons">
    <template v-if="!blendFontInfo">
      <FontUpload @upload="(file) => emit('upload', file)" />
    </template>
    <template v-else>
      <div class="blend-font-bar">
        <p class="blend-font-info text-size-m">
          {{ blendFontName }} — {{ blendFontInfo.glyphCount }} glyphs · {{ blendFontInfo.unitsPerEm }} UPM
        </p>
        <ClearButton label="Remove blend font" @click="emit('removeBlendFont')" />
      </div>
      <div class="slider-row">
        <span class="text-size-m">A</span>
        <input
          v-model.number="blendFactor"
          type="range"
          min="0"
          max="1"
          step="0.01"
          class="blend-slider"
          aria-label="Blend factor"
        />
        <span class="text-size-m">B</span>
      </div>
      <div class="blend-action">
        <button class="btn" :disabled="isLoading" @click="emit('blend')">Play</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.blend-buttons {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.blend-font-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.blend-font-info {
  margin: 0;
}

.slider-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.blend-slider {
  flex: 1;
}

.blend-action {
  display: flex;
  justify-content: flex-end;
}
</style>
