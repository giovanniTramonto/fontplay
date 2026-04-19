<script setup lang="ts">
import { computed, ref } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'
import BlendButtons from '@/components/BlendButtons.vue'
import ClearButton from '@/components/ClearButton.vue'
import FontUpload from '@/components/FontUpload.vue'
import RecombineButtons from '@/components/RecombineButtons.vue'
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
  blendFontsSdfCanvas,
  recombineFonts,
  resetFont,
} = useFontWasm()

const {
  fontData: blendFontData,
  fontInfo: blendFontInfo,
  loadFont: loadBlendFont,
  resetFont: resetBlendFont,
} = useFontWasm()

const text = ref(DEFAULT_TEXT)
const activeProperty = ref<string | null>(null)
const isAiLoading = ref(false)
const aiError = ref<string | null>(null)
const fontName = ref<string | null>(null)

// Styled font bytes — null means use original font
const styledFontBytes = ref<Uint8Array | null>(null)
const isColrEnabled = ref(true)
const activeTab = ref<'mood' | 'blend' | 'recombine'>('mood')

const blendFontName = ref<string | null>(null)
const blendFactor = ref(0.5)
const blendStyledFontBytes = ref<Uint8Array | null>(null)
const blendStyledFontFamily = ref<string | null>(null)
const blendBaseFontFamily = ref<string | null>(null)

const recombineChar = computed(() => [...text.value.trim()][0] ?? 'A')
const recombineStyledFontBytes = ref<Uint8Array | null>(null)
const recombineStyledFontFamily = ref<string | null>(null)

// Unique CSS font-family name, incremented on each font upload to avoid cache issues
let fontCounter = 0
const baseFontFamily = ref<string | null>(null)
const styledFontFamily = ref<string | null>(null)

// The generated result font-family (null when no result exists yet)
const resultFontFamily = computed(
  () => styledFontFamily.value ?? blendStyledFontFamily.value ?? recombineStyledFontFamily.value ?? null,
)

const activeDownloadBytes = computed(() => styledFontBytes.value ?? blendStyledFontBytes.value ?? recombineStyledFontBytes.value)
const activeDownloadName = computed(() => {
  if (styledFontBytes.value)
    return `${fontName.value ?? 'fontplay'}-${activeProperty.value ?? 'styled'}`
  if (blendStyledFontBytes.value)
    return `${fontName.value ?? 'font1'}-x-${blendFontName.value ?? 'font2'}-blend`
  if (recombineStyledFontBytes.value)
    return `${fontName.value ?? 'font1'}-x-${blendFontName.value ?? 'font2'}-recombine-${recombineChar.value}`
  return null
})

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
  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
  aiError.value = null
}

function onWrite(letters: string) {
  text.value = letters
}

async function onRecombine() {
  if (!baseFontFamily.value || !blendBaseFontFamily.value) return
  isAiLoading.value = true
  aiError.value = null
  const char = recombineChar.value
  try {
    const bytes = await recombineFonts(char, baseFontFamily.value, blendBaseFontFamily.value)
    if (bytes) {
      recombineStyledFontBytes.value = bytes
      fontCounter++
      const family = `fontplay-recombine-${fontCounter}`
      recombineStyledFontFamily.value = family
      injectFontFace(family, bytes)
    }
  } catch (e) {
    aiError.value = e instanceof Error ? e.message : String(e)
  } finally {
    isAiLoading.value = false
  }
}

function onRemoveBlendFont() {
  resetBlendFont()
  blendFontName.value = null
  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
  blendBaseFontFamily.value = null
}

async function onBlendUpload(file: File) {
  aiError.value = null
  blendFontName.value = file.name.replace(/\.[^.]+$/, '')
  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
  blendBaseFontFamily.value = null
  styledFontBytes.value = null
  styledFontFamily.value = null
  activeProperty.value = null
  await loadBlendFont(file)
  if (blendFontData.value) {
    fontCounter++
    const family = `fontplay-blend-base-${fontCounter}`
    blendBaseFontFamily.value = family
    injectFontFace(family, blendFontData.value)
  }
}

async function onBlend() {
  if (!blendFontData.value || !baseFontFamily.value || !blendBaseFontFamily.value) return
  isAiLoading.value = true
  aiError.value = null
  await new Promise((r) => setTimeout(r, 0))
  try {
    const bytes = await blendFontsSdfCanvas(
      blendFontData.value,
      baseFontFamily.value,
      blendBaseFontFamily.value,
      text.value,
      blendFactor.value,
    )
    if (bytes) {
      blendStyledFontBytes.value = bytes
      fontCounter++
      const family = `fontplay-blend-${fontCounter}`
      blendStyledFontFamily.value = family
      injectFontFace(family, bytes)
    }
  } catch (e) {
    aiError.value = e instanceof Error ? e.message : String(e)
  } finally {
    isAiLoading.value = false
  }
}

async function onStyle(property: string | null) {
  if (property === null) {
    styledFontBytes.value = null
    styledFontFamily.value = null
    activeProperty.value = null
    return
  }
  if (!fontInfo.value) return

  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
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
  const bytes = activeDownloadBytes.value
  const name = activeDownloadName.value
  if (!bytes || !name) return
  const blob = new Blob([new Uint8Array(bytes)], { type: 'font/ttf' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${name}.ttf`
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
          <ClearButton label="Remove font" @click="onRemoveFont" />
        </div>
      </template>
      <p v-if="isFontLoading" aria-live="polite" class="loading">Reading font…</p>
      <p v-else-if="fontError" role="alert" class="error">{{ fontError }}</p>
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Glyph display">
      <GlyphDisplay :text="text" :fontFamily="baseFontFamily" :resultFontFamily="resultFontFamily" />
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Letter input">
      <LetterInput @write="onWrite" />
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Style">
      <div class="tabs" role="tablist">
        <button role="tab" :aria-selected="activeTab === 'mood'" :class="['btn', { active: activeTab === 'mood' }]"
          @click="activeTab = 'mood'">Mood</button>
        <button role="tab" :aria-selected="activeTab === 'blend'" :class="['btn', { active: activeTab === 'blend' }]"
          @click="activeTab = 'blend'">Blend</button>
        <button role="tab" :aria-selected="activeTab === 'recombine'" :class="['btn', { active: activeTab === 'recombine' }]"
          @click="activeTab = 'recombine'">Recombine</button>
      </div>
      <div class="container">
        <MoodButtons v-if="activeTab === 'mood'" :isLoading="isAiLoading" :activeProperty="activeProperty"
          v-model:isColrEnabled="isColrEnabled" @style="onStyle" />
        <BlendButtons v-else-if="activeTab === 'blend'" :isLoading="isAiLoading" :blendFontName="blendFontName"
          :blendFontInfo="blendFontInfo" v-model:blendFactor="blendFactor" @upload="onBlendUpload" @blend="onBlend"
          @removeBlendFont="onRemoveBlendFont" />
        <RecombineButtons v-else-if="activeTab === 'recombine'" :isLoading="isAiLoading"
          :blendFontName="blendFontName" :blendFontInfo="blendFontInfo"
          @upload="onBlendUpload" @recombine="onRecombine" @removeBlendFont="onRemoveBlendFont" />
      </div>
      <p v-if="isAiLoading" aria-live="polite" class="loading">{{ activeTab === 'blend' ? 'Blending…' : activeTab === 'recombine' ? 'Recombining…' : 'Generating…' }}</p>
      <p v-else-if="aiError" role="alert" class="error">{{ aiError }}</p>
    </section>

    <section v-if="activeDownloadBytes" aria-label="Export">
      <button class="btn" @click="downloadFont">
        Download {{ activeDownloadName }}.ttf
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

.tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

</style>
