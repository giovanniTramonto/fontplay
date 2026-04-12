<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'

const emit = defineEmits<{
  write: [letters: string]
}>()

const letters = ref(DEFAULT_TEXT)
const isCustom = computed(() => letters.value.trim() !== DEFAULT_TEXT)

watch(letters, (val) => {
  const trimmed = val.trim()
  if (trimmed) emit('write', trimmed)
})

function onReset() {
  letters.value = DEFAULT_TEXT
}
</script>

<template>
  <div class="letter-input">
    <label for="letter-field" class="sr-only">Letters to display</label>
    <input
      id="letter-field"
      v-model="letters"
      type="text"
      placeholder="Type letters…"
      maxlength="100"
      class="field"
    />
    <button v-if="isCustom" type="button" class="btn" @click="onReset">Reset</button>
  </div>
</template>

<style scoped>
.letter-input {
  display: flex;
  gap: 0.5rem;
}

.field {
  flex: 1;
  padding: 0.6rem 0.8rem;
  font-size: 1.2rem;
  letter-spacing: 0.05em;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  outline: none;
  font-family: monospace;
}

.field:focus {
  border-color: var(--color-accent);
}
</style>
