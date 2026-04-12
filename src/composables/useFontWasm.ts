import { ref, shallowRef } from 'vue'
import type { ColrConfig, Transform } from '#shared/types'

// biome-ignore lint/suspicious/noExplicitAny: WASM module types are generated at build time
let wasmModule: any = null

async function loadWasm() {
  if (wasmModule) return wasmModule
  const mod = await import('@/wasm/fontplay.js')
  await mod.default()
  wasmModule = mod
  return mod
}

export interface FontInfo {
  unitsPerEm: number
  ascender: number
  descender: number
  glyphCount: number
}

export function useFontWasm() {
  const fontData = shallowRef<Uint8Array | null>(null)
  const fontInfo = ref<FontInfo | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function loadFont(file: File): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      const wasm = await loadWasm()
      const bytes = new Uint8Array(await file.arrayBuffer())
      const info: FontInfo = wasm.get_font_info(bytes)
      fontData.value = bytes
      fontInfo.value = info
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      fontData.value = null
      fontInfo.value = null
    } finally {
      isLoading.value = false
    }
  }

  async function styleFont(transforms: Transform[], colr: ColrConfig, mood?: string): Promise<Uint8Array | null> {
    if (!fontData.value) return null
    const wasm = await loadWasm()
    const request = JSON.stringify({
      transforms,
      colr: {
        effects: [...colr.effects],
        fillColor: colr.fillColor ?? null,
        gradientColors: colr.gradientColors ?? null,
        blockColor: colr.blockColor ?? null,
      },
      mood: mood ?? null,
    })
    const bytes: Uint8Array = wasm.style_font(fontData.value, request)
    return bytes
  }

  function resetFont() {
    fontData.value = null
    fontInfo.value = null
    error.value = null
  }

  return { fontData, fontInfo, isLoading, error, loadFont, styleFont, resetFont }
}
