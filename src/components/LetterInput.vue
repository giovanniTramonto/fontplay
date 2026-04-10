<script setup lang="ts">
import { computed, ref } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'

const emit = defineEmits<{
  write: [letters: string]
}>()

const letters = ref(DEFAULT_TEXT)

const isCustom = computed(() => letters.value.trim() !== DEFAULT_TEXT)

function onSubmit() {
  const trimmed = letters.value.trim()
  if (trimmed) emit('write', trimmed)
}

function onReset() {
  letters.value = DEFAULT_TEXT
  emit('write', DEFAULT_TEXT)
}
</script>

<template>
  <form class="letter-input" @submit.prevent="onSubmit">
    <label for="letter-field" class="sr-only">Letters to display</label>
    <input
      id="letter-field"
      v-model="letters"
      type="text"
      placeholder="Type letters…"
      maxlength="100"
      class="field"
    />
    <button type="submit" :disabled="!letters.trim()" class="btn">Write</button>
    <button v-if="isCustom" type="button" class="btn" @click="onReset">Reset</button>
  </form>
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
