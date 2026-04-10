<script setup lang="ts">
import { computed, ref } from 'vue'
import FontUpload from '@/components/FontUpload.vue'
import GlyphDisplay from '@/components/GlyphDisplay.vue'
import LetterInput from '@/components/LetterInput.vue'
import MoodButtons from '@/components/MoodButtons.vue'
import { type GlyphSvg, useFontWasm } from '@/composables/useFontWasm'
import { askLLM } from '@/composables/useLLM'
import { applyTransformsToPath } from '@/utils/transformPath'
import type { Transform } from '#shared/types'
import { DEFAULT_TEXT } from '#shared/constants'

const { fontInfo, isLoading: isFontLoading, error: fontError, loadFont, getGlyphSvg, resetFont } =
  useFontWasm()

interface RenderedGlyph {
  char: string
  svg: GlyphSvg
}

const glyphs = ref<RenderedGlyph[]>([])
const appliedTransforms = ref<Transform[]>([])
const activeProperty = ref<string | null>(null)
const isColrV1Enabled = ref(false)
const isAiLoading = ref(false)
const aiError = ref<string | null>(null)
const fontName = ref<string | null>(null)

const displayGlyphs = computed(() => {
  if (!appliedTransforms.value.length) return glyphs.value
  return glyphs.value.map(({ char, svg }) => {
    const d = applyTransformsToPath(svg.d, appliedTransforms.value, char.codePointAt(0) ?? 0)
    return { char, svg: { ...svg, d } }
  })
})

async function onFontUpload(file: File) {
  aiError.value = null
  glyphs.value = []
  appliedTransforms.value = []
  fontName.value = file.name.replace(/\.[^.]+$/, '')
  await loadFont(file)
  await onWrite(DEFAULT_TEXT)
}

function onRemoveFont() {
  resetFont()
  fontName.value = null
  glyphs.value = []
  appliedTransforms.value = []
  activeProperty.value = null
  aiError.value = null
}

async function onWrite(letters: string) {
  aiError.value = null
  appliedTransforms.value = []
  activeProperty.value = null
  const unique = [...new Set([...letters])]
  const results = await Promise.all(
    unique.map(async (char) => {
      const svg = await getGlyphSvg(char)
      return svg ? { char, svg } : null
    })
  )
  glyphs.value = [...letters].flatMap((char) => {
    const found = results.find((r) => r?.char === char)
    return found ? [found] : []
  })
}

async function onStyle(property: string | null) {
  if (property === null) {
    appliedTransforms.value = []
    activeProperty.value = null
    return
  }
  if (!glyphs.value.length) return
  isAiLoading.value = true
  aiError.value = null
  try {
    appliedTransforms.value = await askLLM(property)
    activeProperty.value = property
  } catch (e) {
    aiError.value = e instanceof Error ? e.message : String(e)
  } finally {
    isAiLoading.value = false
  }
}
</script>

<template>
  <main>
    <header>
      <h1>fontplay</h1>
    </header>

    <section aria-label="Font upload">
      <template v-if="!fontInfo">
        <FontUpload @upload="onFontUpload" />
      </template>
      <template v-else>
        <div class="font-bar">
          <p aria-live="polite" class="font-bar-info text-size-m">
            {{ fontName }} — {{ fontInfo.glyphCount }} glyphs · {{ fontInfo.unitsPerEm }} UPM
          </p>
          <button class="btn" aria-label="Remove font" @click="onRemoveFont">✕</button>
        </div>
      </template>
      <p v-if="isFontLoading" aria-live="polite" class="loading">Reading font…</p>
      <p v-else-if="fontError" role="alert" class="error">{{ fontError }}</p>
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Letter input">
      <LetterInput @write="onWrite" />
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Glyph display">
      <GlyphDisplay :glyphs="displayGlyphs" />
      <div class="display-options">
        <label class="colrv1-toggle text-size-m">
          <input v-model="isColrV1Enabled" type="checkbox" />
          Enable COLRv1
        </label>
      </div>
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Style">
      <MoodButtons :is-loading="isAiLoading" :active-property="activeProperty" @style="onStyle" />
      <p v-if="isAiLoading" aria-live="polite" class="loading">Generating style…</p>
      <p v-else-if="aiError" role="alert" class="error">{{ aiError }}</p>
    </section>
  </main>
</template>

<style scoped>
.font-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.display-options {
  display: flex;
  justify-content: flex-end;
  margin-top: 0.5rem;
}

.colrv1-toggle {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  cursor: pointer;
  user-select: none;
}

.font-bar-info {
  margin: 0;
}
</style>

