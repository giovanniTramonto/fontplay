<script setup lang="ts">
import { computed } from 'vue'
import type { ColrConfig } from '#shared/types'
import type { GlyphSvg } from '@/composables/useFontWasm'

const props = defineProps<{
  glyphs: Array<{ char: string; svg: GlyphSvg }>
  colrConfig?: ColrConfig
}>()

const fx = computed(() => props.colrConfig?.effects ?? new Set())

const glyphHeight = computed(() => {
  const n = props.glyphs.length
  return `${Math.max(48, Math.round(220 - n * 3))}px`
})

function gradientId(char: string) {
  return `grad-${char.codePointAt(0)}`
}
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
      <defs>
        <filter :id="`blur-${char.codePointAt(0)}`">
          <feGaussianBlur stdDeviation="40" />
        </filter>
        <linearGradient v-if="fx.has('gradient')" :id="gradientId(char)" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" :stop-color="colrConfig?.gradientColors?.[0] ?? '#f97316'" />
          <stop offset="100%" :stop-color="colrConfig?.gradientColors?.[1] ?? '#8b5cf6'" />
        </linearGradient>
      </defs>

      <template v-if="svg.d">
        <!-- COLRv1: shadow — always below everything -->
        <path v-if="fx.has('shadow')" :d="svg.d" fill="rgba(0,0,0,0.28)" :filter="`url(#blur-${char.codePointAt(0)})`" transform="translate(50 60)" />

        <!-- COLRv1: 3D block — extruded layers from back to front -->
        <template v-if="fx.has('3d-blocks')">
          <path v-for="i in 8" :key="i" :d="svg.d" :fill="colrConfig?.blockColor ?? '#111'" :transform="`translate(${(8 - i) * 5} ${(8 - i) * 5})`" />
        </template>

        <!-- COLRv1: PaintGlyph + PaintSolid (front/main layer) -->
        <path
          :d="svg.d"
          :fill="
            fx.has('gradient') ? `url(#${gradientId(char)})` :
            fx.has('fill')     ? (colrConfig?.fillColor ?? '#e63946') :
            'currentColor'
          "
          :stroke="fx.has('outline') ? (colrConfig?.outlineColor ?? '#2563eb') : 'none'"
          :stroke-width="fx.has('outline') ? 40 : 0"
          paint-order="stroke"
        />
      </template>

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
