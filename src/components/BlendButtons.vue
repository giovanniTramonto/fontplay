<script setup lang="ts">
import type { FontInfo } from '@/composables/useFontWasm'

defineProps<{
  secondFontInfo?: FontInfo | null
}>()

const blendFactor = defineModel<number>('blendFactor', { default: 0.5 })
</script>

<template>
  <div v-if="secondFontInfo" class="slider-row">
    <span class="text-size-m">A</span>
    <input v-model.number="blendFactor" type="range" min="0" max="1" step="0.01" class="blend-slider"
      aria-label="Blend factor" />
    <span class="text-size-m">B</span>
  </div>
</template>

<style scoped>
.slider-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.blend-slider {
  flex: 1;
}
</style>
