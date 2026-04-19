import { ref, shallowRef } from 'vue'
import type { ColrConfig, Transform } from '#shared/types'

const CANVAS_SIZE = 512
const BLEND_CHARS =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,!?-\'"():;@#%&'

function renderGlyphToCanvas(
  fontFamily: string,
  char: string,
  upem: number,
  ascender: number,
  descender: number,
): Uint8Array {
  const size = CANVAS_SIZE
  const canvas = new OffscreenCanvas(size, size)
  const ctx = canvas.getContext('2d')
  if (!ctx) return new Uint8Array(size * size).fill(255)
  ctx.fillStyle = 'white'
  ctx.fillRect(0, 0, size, size)
  const em = ascender - descender
  const scale = (size * 0.8) / em
  const cssFontSizePx = scale * upem
  const baselinePx = size * 0.5 + (ascender + descender) * 0.5 * scale
  ctx.fillStyle = 'black'
  ctx.font = `${cssFontSizePx}px '${fontFamily}'`
  ctx.textBaseline = 'alphabetic'
  ctx.textAlign = 'left'
  const textWidth = ctx.measureText(char).width
  const leftPadPx = Math.max(0, (size - textWidth) / 2)
  ctx.fillText(char, leftPadPx, baselinePx)
  const imageData = ctx.getImageData(0, 0, size, size)
  const result = new Uint8Array(size * size)
  for (let i = 0; i < size * size; i++) {
    result[i] = imageData.data[i * 4]
  }
  return result
}

function saveDebugBitmap(pixels: Uint8Array, size: number, filename: string) {
  const canvas = new OffscreenCanvas(size, size)
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  const imgData = ctx.createImageData(size, size)
  for (let i = 0; i < size * size; i++) {
    imgData.data[i * 4 + 0] = pixels[i]
    imgData.data[i * 4 + 1] = pixels[i]
    imgData.data[i * 4 + 2] = pixels[i]
    imgData.data[i * 4 + 3] = 255
  }
  ctx.putImageData(imgData, 0, 0)
  canvas.convertToBlob({ type: 'image/png' }).then((blob) =>
    fetch('/debug-bitmap', { method: 'POST', headers: { 'x-filename': filename }, body: blob }),
  )
}

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

  async function styleFont(
    transforms: Transform[],
    colr: ColrConfig,
    mood?: string,
  ): Promise<Uint8Array | null> {
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

  async function blendFontsSdfCanvas(
    secondFontData: Uint8Array,
    font1Family: string,
    font2Family: string,
    text: string,
    blendFactor: number,
  ): Promise<Uint8Array | null> {
    if (!fontData.value || !fontInfo.value) return null
    const wasm = await loadWasm()
    const { unitsPerEm: upem, ascender, descender } = fontInfo.value
    const size = CANVAS_SIZE

    const charSet = new Set([...BLEND_CHARS, ...text].filter((c) => c.trim() !== ''))
    const chars = [...charSet]
    const charCodes = chars.map((c) => c.codePointAt(0) ?? 0)

    const bitmaps1 = new Uint8Array(chars.length * size * size)
    const bitmaps2 = new Uint8Array(chars.length * size * size)
    for (let i = 0; i < chars.length; i++) {
      bitmaps1.set(renderGlyphToCanvas(font1Family, chars[i], upem, ascender, descender), i * size * size)
      bitmaps2.set(renderGlyphToCanvas(font2Family, chars[i], upem, ascender, descender), i * size * size)
    }

    saveDebugBitmap(bitmaps1.subarray(0, size * size), size, 'font1-bitmap.png')
    saveDebugBitmap(bitmaps2.subarray(0, size * size), size, 'font2-bitmap.png')
    const blendDebug: Uint8Array = wasm.blend_one_bitmap_debug(
      bitmaps1.subarray(0, size * size),
      bitmaps2.subarray(0, size * size),
      size,
      blendFactor,
    )
    saveDebugBitmap(blendDebug, size, 'blend-bitmap.png')

    const request = JSON.stringify({ blendFactor, charCodes, bitmapSize: size })
    const bytes: Uint8Array = wasm.blend_from_canvas_bitmaps(
      fontData.value,
      secondFontData,
      bitmaps1,
      bitmaps2,
      request,
    )
    return bytes
  }

  function resetFont() {
    fontData.value = null
    fontInfo.value = null
    error.value = null
  }

  return { fontData, fontInfo, isLoading, error, loadFont, styleFont, blendFontsSdfCanvas, resetFont }
}
