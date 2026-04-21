<script setup lang="ts">
import { computed, ref } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'
import BlendButtons from '@/components/BlendButtons.vue'
import FontBar from '@/components/FontBar.vue'
import FontUpload from '@/components/FontUpload.vue'
import GlyphDisplay from '@/components/GlyphDisplay.vue'
import SpliceButtons from '@/components/SpliceButtons.vue'
import StyleButtons from '@/components/StyleButtons.vue'
import { useFontWasm } from '@/composables/useFontWasm'
import { askLLM, askSpliceLLM } from '@/composables/useLLM'

const {
  fontData,
  fontInfo,
  isLoading: isFontLoading,
  error: fontError,
  loadFont,
  styleFont,
  blendFontsSdfCanvas,
  spliceFontsAtCuts,
  resetFont,
} = useFontWasm()

const {
  fontData: blendFontData,
  fontInfo: secondFontInfo,
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
const activeTab = ref<'style' | 'blend' | 'splice'>('style')

const blendFontName = ref<string | null>(null)
const blendFactor = ref(0.5)
const blendStyledFontBytes = ref<Uint8Array | null>(null)
const blendStyledFontFamily = ref<string | null>(null)
const blendBaseFontFamily = ref<string | null>(null)

const spliceActiveIntensity = ref<'low' | 'medium' | 'high' | null>(null)
const spliceStyledFontBytes = ref<Uint8Array | null>(null)
const spliceStyledFontFamily = ref<string | null>(null)

// Unique CSS font-family name, incremented on each font upload to avoid cache issues
let fontCounter = 0
const baseFontFamily = ref<string | null>(null)
const styledFontFamily = ref<string | null>(null)

const resultFontFamily = computed(
  () => styledFontFamily.value ?? blendStyledFontFamily.value ?? spliceStyledFontFamily.value ?? null,
)
const resultText = computed(() => text.value)

const activeDownloadBytes = computed(() => styledFontBytes.value ?? blendStyledFontBytes.value ?? spliceStyledFontBytes.value)
const activeDownloadName = computed(() => {
  if (styledFontBytes.value)
    return `${fontName.value ?? 'fontplay'}-${activeProperty.value ?? 'styled'}`
  if (blendStyledFontBytes.value)
    return `${fontName.value ?? 'font1'}-x-${blendFontName.value ?? 'font2'}-blend`
  if (spliceStyledFontBytes.value)
    return `${fontName.value ?? 'font1'}-x-${blendFontName.value ?? 'font2'}-splice`
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
  spliceStyledFontBytes.value = null
  spliceStyledFontFamily.value = null
  spliceActiveIntensity.value = null
  aiError.value = null
}

async function onSplice(intensity: 'low' | 'medium' | 'high') {
  if (!blendFontData.value) return
  styledFontBytes.value = null
  styledFontFamily.value = null
  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
  isAiLoading.value = true
  aiError.value = null
  spliceActiveIntensity.value = intensity
  try {
    const spliceResult = await askSpliceLLM(intensity)
    const bytes = await spliceFontsAtCuts(blendFontData.value, spliceResult)
    if (bytes) {
      spliceStyledFontBytes.value = bytes
      fontCounter++
      const family = `fontplay-splice-${fontCounter}`
      spliceStyledFontFamily.value = family
      injectFontFace(family, bytes)
    }
  } catch (e) {
    aiError.value = e instanceof Error ? e.message : String(e)
    spliceActiveIntensity.value = null
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
  spliceStyledFontBytes.value = null
  spliceStyledFontFamily.value = null
}

async function onBlendUpload(file: File) {
  aiError.value = null
  blendFontName.value = file.name.replace(/\.[^.]+$/, '')
  blendStyledFontBytes.value = null
  blendStyledFontFamily.value = null
  blendBaseFontFamily.value = null
  spliceStyledFontBytes.value = null
  spliceStyledFontFamily.value = null
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
  styledFontBytes.value = null
  styledFontFamily.value = null
  spliceStyledFontBytes.value = null
  spliceStyledFontFamily.value = null
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
  <main class="main">
    <header>
      <h1>fontplay</h1>
    </header>

    <section aria-label="Font upload">
      <template v-if="!fontInfo">
        <FontUpload @upload="onFontUpload" />
      </template>
      <template v-else>
        <FontBar :name="fontName ?? ''" :fontInfo="fontInfo" @clear="onRemoveFont" />
      </template>
      <p v-if="isFontLoading" aria-live="polite" class="loading">Reading font…</p>
      <p v-else-if="fontError" role="alert" class="error">{{ fontError }}</p>
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Glyph display">
      <GlyphDisplay v-model="text" editable :fontFamily="baseFontFamily" />
    </section>

    <section v-if="fontInfo && !isFontLoading" aria-label="Style">
      <div class="tabs" role="tablist">
        <button role="tab" :aria-selected="activeTab === 'style'" :class="['btn', { active: activeTab === 'style' }]"
          @click="activeTab = 'style'">Style</button>
        <button role="tab" :aria-selected="activeTab === 'blend'" :class="['btn', { active: activeTab === 'blend' }]"
          @click="activeTab = 'blend'">Blend</button>
        <button role="tab" :aria-selected="activeTab === 'splice'"
          :class="['btn', { active: activeTab === 'splice' }]" @click="activeTab = 'splice'">Splice</button>
      </div>
      <div class="container">
        <StyleButtons v-if="activeTab === 'style'" :isLoading="isAiLoading" :activeProperty="activeProperty"
          v-model:isColrEnabled="isColrEnabled" @style="onStyle" />
        <BlendButtons v-else-if="activeTab === 'blend'" :secondFontInfo="secondFontInfo"
          v-model:blendFactor="blendFactor" />
        <SpliceButtons v-else-if="activeTab === 'splice'" :secondFontInfo="secondFontInfo"
          :isLoading="isAiLoading" :activeIntensity="spliceActiveIntensity" @splice="onSplice" />
      </div>
      <template v-if="activeTab !== 'style'">
        <template v-if="!secondFontInfo">
          <div class="container">
            <FontUpload @upload="onBlendUpload" />
          </div>
        </template>
        <template v-else>
          <div class="container">
            <FontBar :name="blendFontName ?? ''" :fontInfo="secondFontInfo" @clear="onRemoveBlendFont" />
          </div>
          <div class="container">
            <GlyphDisplay v-model="text" :fontFamily="blendBaseFontFamily" />
          </div>
        </template>
        <div v-if="activeTab === 'blend'" class="container">
          <button class="btn" :disabled="!secondFontInfo || isAiLoading" @click="onBlend()">Play</button>
        </div>
      </template>
      <p v-if="isAiLoading" aria-live="polite" class="loading">{{ activeTab === 'style' ? 'Generating…' : activeTab === 'blend' ? 'Blending…' : 'Splicing…' }}</p>
      <p v-else-if="aiError" role="alert" class="error">{{ aiError }}</p>
    </section>

    <section v-if="resultFontFamily" aria-label="Result">
      <GlyphDisplay :modelValue="resultText" :fontFamily="resultFontFamily" />
    </section>

    <section class="export-section" v-if="activeDownloadBytes" aria-label="Export">
      <button class="btn" @click="downloadFont" :title="`${activeDownloadName}.ttf`">
        Download TTF
      </button>
    </section>
  </main>
</template>

<style scoped>
.main {
  max-width: 1200px;
  margin-inline: auto;
  padding: 2rem 1rem;
  display: flex;
  flex-direction: column;
  gap: var(--gap);
}


.tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.export-section {
  display: flex;
  justify-content: flex-end;
}
</style>
