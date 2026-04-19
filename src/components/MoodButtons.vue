<script setup lang="ts">
defineProps<{
  isLoading?: boolean
  activeProperty?: string | null
}>()

const emit = defineEmits<{ style: [property: string | null] }>()

const properties = [
  { id: 'modern', label: 'Modern' },
  { id: 'cyber', label: 'Cyber' },
  { id: 'playful', label: 'Playful' },
  { id: 'edgy', label: 'Edgy' },
  { id: 'cool', label: 'Cool' },
]
</script>

<template>
  <div class="style-buttons">
    <button v-for="{ id, label } in properties" :key="id" :disabled="isLoading"
      :class="['btn btn--secondary', { active: activeProperty === id }]"
      @click="emit('style', activeProperty === id ? null : id)">
      {{ label }}
    </button>
  </div>
</template>

<style scoped>
.style-buttons {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
</style>
