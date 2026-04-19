<script setup lang="ts">
defineProps<{ label: string }>()
defineEmits<{ click: [] }>()
</script>

<template>
  <button class="btn" :aria-label="label" @click="$emit('click')">✕</button>
</template>
