<script setup lang="ts">
defineProps<{ label: string }>()
defineEmits<{ click: [] }>()
</script>

<template>
  <button class="btn btn--secondary clear-button" :aria-label="label" @click="$emit('click')"><span>✕</span></button>
</template>

<style scoped>
.clear-button {
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.clear-button>span {
  display: block;
  position: absolute;
}
</style>