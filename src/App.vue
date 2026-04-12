<script setup lang="ts">
import { computed, ref } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'
import FontUpload from '@/components/FontUpload.vue'
import GlyphDisplay from '@/components/GlyphDisplay.vue'
import LetterInput from '@/components/LetterInput.vue'
import MoodButtons from '@/components/MoodButtons.vue'
import { useFontWasm } from '@/composables/useFontWasm'
import { askLLM } from '@/composables/useLLM'

const {
  fontData,
  fontInfo,
  isLoading: isFontLoading,
  error: fontError,
  loadFont,
  styleFont,
  resetFont,
} = useFontWasm()

const text = ref(DEFAULT_TEXT)
const activeProperty = ref<string | null>(null)
const isAiLoading = ref(false)
const aiError = ref<string | null>(null)
const fontName = ref<string | null>(null)

// Styled font bytes — null means use original font
const styledFontBytes = ref<Uint8Array | null>(null)
const isColrEnabled = ref(true)

// Unique CSS font-family name, incremented on each font upload to avoid cache issues
let fontCounter = 0
const baseFontFamily = ref<string | null>(null)
const styledFontFamily = ref<string | null>(null)

// The font-family shown in GlyphDisplay: styled if available, else original
const displayFontFamily = computed(() => styledFontFamily.value ?? baseFontFamily.value)

// Inject @font-face into document head
function injectFontFace(family: string, bytes: Uint8Array) {
  const id = `fontplay-face-${family}`
  document.getElementById(id)?.remove()
  const blob = new Blob([new Uint8Array(bytes)], { type: 'font/ttf' })
  const url = URL.createObjectURL(blob)
  const style = document.createElement('style')
  style.id = id
  style.textContent = `@font-face { font-family: '${family}'; src: url('${url}') format('truetype'); font-display: block; }`
  document.head.appendChild(style)
}

async function onFontUpload(file: File) {
  aiError.value = null
  styledFontBytes.value = null
  styledFontFamily.value = null
  activeProperty.value = null
  fontName.value = file.name.replace(/\.[^.]+$/, '')

  await loadFont(file)

  if (fontData.value) {
    fontCounter++
    const family = `fontplay-base-${fontCounter}`
    baseFontFamily.value = family
    injectFontFace(family, fontData.value)
  }

  text.value = DEFAULT_TEXT
}

function onRemoveFont() {
  resetFont()
  fontName.value = null
  baseFontFamily.value = null
  styledFontFamily.value = null
  styledFontBytes.value = null
  activeProperty.value = null
  aiError.value = null
}

function onWrite(letters: string) {
  text.value = letters
}

async function onStyle(property: string | null) {
  if (property === null) {
    styledFontBytes.value = null
    styledFontFamily.value = null
    activeProperty.value = null
    return
  }
  if (!fontInfo.value) return

  isAiLoading.value = true
  aiError.value = null
  try {
    const result = await askLLM(property)
    const colr = isColrEnabled.value
      ? result.colr
      : { effects: new Set<import('#shared/types').ColrEffect>() }
    const bytes = await styleFont(result.transforms, colr, property)
    if (bytes) {
      styledFontBytes.value = bytes
      fontCounter++
      const family = `fontplay-styled-${fontCounter}`
      styledFontFamily.value = family
      injectFontFace(family, bytes)
    }
    activeProperty.value = property
  } catch (e) {
    aiError.value = e instanceof Error ? e.message : String(e)
  } finally {
    isAiLoading.value = false
  }
}

function downloadFont() {
  const bytes = styledFontBytes.value
  if (!bytes) return
  const blob = new Blob([new Uint8Array(bytes)], { type: 'font/ttf' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${fontName.value ?? 'fontplay'}-${activeProperty.value ?? 'styled'}.ttf`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
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
      <GlyphDisplay :text="text" :fontFamily="displayFontFamily" />
      <div class="display-options">
        <label class="colrv1-toggle text-size-m">
          <input v-model="isColrEnabled" type="checkbox" />
          Enable COLRv1
        </label>
      </div>
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Style">
      <MoodButtons :isLoading="isAiLoading" :activeProperty="activeProperty" @style="onStyle" />
      <p v-if="isAiLoading" aria-live="polite" class="loading">Generating style…</p>
      <p v-else-if="aiError" role="alert" class="error">{{ aiError }}</p>
    </section>

    <section v-if="styledFontBytes" aria-label="Export">
      <button class="btn" @click="downloadFont">
        Download {{ fontName }}-{{ activeProperty }}.ttf
      </button>
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

.font-bar-info {
  margin: 0;
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
</style>
