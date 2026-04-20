<script setup lang="ts">
import ClearButton from '@/components/ClearButton.vue'
import type { FontInfo } from '@/composables/useFontWasm'

defineProps<{
  name: string
  fontInfo: FontInfo
}>()

defineEmits<{ clear: [] }>()
</script>

<template>
  <div class="font-bar">
    <ClearButton label="Remove font" @click="$emit('clear')" />
    <p class="font-bar-info text-size-m" aria-live="polite">
      {{ name }} — {{ fontInfo.glyphCount }} glyphs · {{ fontInfo.unitsPerEm }} UPM
    </p>
  </div>
</template>

<style scoped>
.font-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.font-bar-info {
  margin: 0;
}
</style>
