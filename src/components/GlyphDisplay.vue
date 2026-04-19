<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  text: string
  fontFamily: string | null
  resultFontFamily?: string | null
}>()

const fontSize = computed(() => {
  const n = props.text.length
  if (n <= 1) return 'clamp(4rem, 28vh, 16rem)'
  if (n <= 3) return 'clamp(3rem, 20vh, 12rem)'
  if (n <= 6) return 'clamp(2rem, 14vh, 8rem)'
  if (n <= 10) return 'clamp(1.5rem, 10vh, 6rem)'
  return 'clamp(1rem, 6vh, 4rem)'
})
</script>

<template>
  <div class="display-row" :class="{ 'display-row--split': resultFontFamily }">
    <div class="panel">
      <div class="display" role="img" :aria-label="text || 'No text entered'">
        <p v-if="text && fontFamily" class="display-text" :style="{ fontFamily: `'${fontFamily}', sans-serif`, fontSize }">
          {{ text }}
        </p>
        <p v-else-if="text" class="display-text display-text--plain" :style="{ fontSize }">
          {{ text }}
        </p>
        <p v-else class="placeholder">Upload a font and type some text</p>
      </div>
    </div>

    <div v-if="resultFontFamily" class="panel">
      <div class="display" role="img" :aria-label="text || 'No text entered'">
        <p v-if="text" class="display-text" :style="{ fontFamily: `'${resultFontFamily}', sans-serif`, fontSize }">
          {{ text }}
        </p>
        <p v-else class="placeholder">Upload a font and type some text</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.display-row {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.display-row--split {
  grid-template-columns: 1fr 1fr;
}

.display-row--split {
  align-items: stretch;
}

.panel {
  display: flex;
  flex-direction: column;
}

.display {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  overflow: hidden;
}

.display-text {
  line-height: 1.1;
  margin: 0;
  color: var(--color-accent);
  word-break: break-all;
}

.display-text--plain {
  color: var(--color-text);
  opacity: 0.3;
}

.placeholder {
  margin: 0;
  color: var(--color-text);
  opacity: 0.4;
}
</style>
