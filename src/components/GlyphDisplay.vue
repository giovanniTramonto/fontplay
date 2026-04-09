<script setup lang="ts">
import { computed } from 'vue'
import type { GlyphSvg } from "@/composables/useFontWasm";

const props = defineProps<{
  glyphs: Array<{ char: string; svg: GlyphSvg }>;
}>()

const glyphHeight = computed(() => {
  const n = props.glyphs.length
  return `${Math.max(48, Math.round(220 - n * 3))}px`
})
</script>

<template>
  <div
    class="display"
    role="img"
    :aria-label="glyphs.map((g) => g.char).join('') || 'No glyphs rendered yet'"
  >
    <svg
      v-for="{ char, svg } in glyphs"
      :key="char + svg.viewBox"
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="svg.viewBox"
      preserveAspectRatio="xMidYMid meet"
      class="glyph"
      aria-hidden="true"
    >
      <g>
        <path v-if="svg.d" :d="svg.d" fill="currentColor" />
        <!-- Placeholder for missing glyphs -->
        <rect
          v-else
          x="10%"
          y="10%"
          width="80%"
          height="80%"
          fill="none"
          stroke="currentColor"
          stroke-width="20"
          stroke-dasharray="40 20"
        />
      </g>
    </svg>
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
  padding: 2rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.glyph {
  display: block;
  height: v-bind(glyphHeight);
  width: auto;
  overflow: visible;
  color: var(--color-accent);
  flex-shrink: 0;
}
</style>
