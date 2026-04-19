<script setup lang="ts">
import ClearButton from '@/components/ClearButton.vue'
import FontUpload from '@/components/FontUpload.vue'
import type { FontInfo } from '@/composables/useFontWasm'

defineProps<{
  isLoading?: boolean
  blendFontName?: string | null
  blendFontInfo?: FontInfo | null
}>()

const emit = defineEmits<{
  upload: [file: File]
  recombine: []
  removeBlendFont: []
}>()
</script>

<template>
  <div class="recombine-buttons">
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
      <div class="recombine-action">
        <button class="btn" :disabled="isLoading" @click="emit('recombine')">Play</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.recombine-buttons {
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

.recombine-action {
  display: flex;
  justify-content: flex-end;
}

</style>
