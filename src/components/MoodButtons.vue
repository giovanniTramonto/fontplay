<script setup lang="ts">
defineProps<{
  isLoading?: boolean
  activeProperty?: string | null
}>()

const isColrEnabled = defineModel<boolean>('isColrEnabled', { default: true })

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
  <div>
  <label class="colrv1-toggle text-size-m">
    <input v-model="isColrEnabled" type="checkbox" />
    Enable COLRv1
  </label>
  <div class="style-buttons">
    <button v-for="{ id, label } in properties" :key="id" :disabled="isLoading"
      :class="['btn btn--secondary', { active: activeProperty === id }]"
      @click="emit('style', activeProperty === id ? null : id)">
      {{ label }}
    </button>
  </div>
  </div>
</template>

<style scoped>
.colrv1-toggle {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  cursor: pointer;
  user-select: none;
  margin-bottom: 0.75rem;
}

.style-buttons {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
</style>
