import { ref, shallowRef } from 'vue'

// Lazily loaded WASM module — built by wasm-pack into src/wasm/
// biome-ignore lint/suspicious/noExplicitAny: WASM module types are generated at build time
let wasmModule: any = null

async function loadWasm() {
  if (wasmModule) return wasmModule
  const mod = await import('@/wasm/fontplay.js')
  await mod.default() // run wasm-bindgen init
  wasmModule = mod
  return mod
}

export interface FontInfo {
  unitsPerEm: number
  ascender: number
  descender: number
  glyphCount: number
}

export interface GlyphSvg {
  d: string
  viewBox: string
  advanceWidth: number
  height: number
  found: boolean
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
      const buffer = await file.arrayBuffer()
      const bytes = new Uint8Array(buffer)
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

  async function getGlyphSvg(char: string): Promise<GlyphSvg | null> {
    if (!fontData.value) return null
    const codePoint = char.codePointAt(0)
    if (codePoint === undefined) return null
    try {
      const wasm = await loadWasm()
      const result: GlyphSvg = wasm.char_to_svg(fontData.value, codePoint)
      return result
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      return null
    }
  }

  function resetFont() {
    fontData.value = null
    fontInfo.value = null
    error.value = null
  }

  return { fontData, fontInfo, isLoading, error, loadFont, getGlyphSvg, resetFont }
}
