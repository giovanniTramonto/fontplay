<script setup lang="ts">
import FontBar from '@/components/FontBar.vue'
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
      <FontBar :name="blendFontName ?? ''" :fontInfo="blendFontInfo" @clear="emit('removeBlendFont')" />
      <div>
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
</style>
