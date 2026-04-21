<script setup lang="ts">
defineProps<{
  secondFontInfo?: import('@/composables/useFontWasm').FontInfo | null
  isLoading?: boolean
  activeIntensity?: 'low' | 'medium' | 'high' | null
}>()

const emit = defineEmits<{
  splice: [intensity: 'low' | 'medium' | 'high']
}>()
</script>

<template>
  <div v-if="secondFontInfo" class="splice-presets">
    <button
      v-for="level in (['low', 'medium', 'high'] as const)"
      :key="level"
      type="button"
      :class="['btn', { active: activeIntensity === level }]"
      :disabled="isLoading"
      @click="emit('splice', level)"
    >
      {{ level.charAt(0).toUpperCase() + level.slice(1) }}
    </button>
  </div>
</template>

<style scoped>
.splice-presets {
  display: flex;
  gap: 0.5rem;
}

.splice-presets .btn {
  flex: 1;
}
</style>
