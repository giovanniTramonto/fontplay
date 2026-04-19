<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  text: string
  fontFamily: string | null
}>()

const fontSize = computed(() => {
  const n = props.text.length
  if (n <= 1) return 'clamp(6rem, 30vw, 20rem)'
  if (n <= 3) return 'clamp(5rem, 22vw, 16rem)'
  if (n <= 6) return 'clamp(4rem, 16vw, 12rem)'
  if (n <= 10) return 'clamp(3rem, 11vw, 8rem)'
  return 'clamp(2rem, 7vw, 5rem)'
})
</script>

<template>
  <div class="display" role="img" :aria-label="text || 'No text entered'">
    <p v-if="text && fontFamily" class="display-text" :style="{ fontFamily: `'${fontFamily}', sans-serif`, fontSize }">
      {{ text }}
    </p>
    <p v-else-if="text" class="display-text display-text--plain" :style="{ fontSize }">
      {{ text }}
    </p>
    <p v-else class="placeholder">Upload a font and type some text</p>
  </div>
</template>

<style scoped>
.display {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  min-height: 300px;
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
