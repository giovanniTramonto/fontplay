import type { ColrConfig, ColrEffect, Transform } from '../types'

export interface LLMResult {
  transforms: Transform[]
  colr: ColrConfig
}

export interface BlendLLMResult {
  blendFactor: number
}

export function extractResult(raw: string): LLMResult | null {
  const match = raw.match(/\{[\s\S]*\}/)
  if (!match) return null

  try {
    const parsed = JSON.parse(match[0]) as Record<string, unknown>

    const knownTransforms = new Set(['scaleX', 'shear', 'jitter', 'wave', 'waveY', 'rotate', 'perspective', 'arch'])
    const transforms = Array.isArray(parsed.transforms)
      ? (parsed.transforms as Transform[]).filter((t) => t && knownTransforms.has((t as { type: string }).type))
      : []

    const colr = parseColr(parsed.effects)
    return { transforms, colr }
  } catch {
    // fall through to partial recovery
  }

  // Recover transforms from truncated response
  const items = [...raw.matchAll(/\{[^{}]*"type"\s*:\s*"[^""]+"[^{}]*\}/g)]
  if (items.length > 0) {
    try {
      return {
        transforms: items.map((m) => JSON.parse(m[0]) as Transform),
        colr: { effects: new Set() },
      }
    } catch {
      // ignore
    }
  }

  return null
}

export function extractBlendResult(raw: string): BlendLLMResult | null {
  const match = raw.match(/\{[\s\S]*\}/)
  if (!match) return { blendFactor: 0.5 }
  try {
    const parsed = JSON.parse(match[0]) as Record<string, unknown>
    const blendFactor =
      typeof parsed.blendFactor === 'number'
        ? Math.max(0, Math.min(1, parsed.blendFactor))
        : 0.5
    return { blendFactor }
  } catch {
    return { blendFactor: 0.5 }
  }
}

function parseColr(raw: unknown): ColrConfig {
  if (!raw || typeof raw !== 'object') return { effects: new Set() }
  const e = raw as Record<string, unknown>

  const active = Array.isArray(e.active) ? (e.active as ColrEffect[]) : []
  // never allow fill + gradient simultaneously
  const filtered = active.includes('gradient') ? active.filter((x) => x !== 'fill') : active

  return {
    effects: new Set(filtered),
    fillColor: typeof e.fillColor === 'string' ? e.fillColor : undefined,
    gradientColors: isGradientColors(e.gradientColors) ? e.gradientColors : undefined,
    outlineColor: typeof e.outlineColor === 'string' ? e.outlineColor : undefined,
    blockColor: typeof e.blockColor === 'string' ? e.blockColor : undefined,
  }
}

export interface SpliceCharParams {
  cut1: number
  cut2: number
  zones: ['font1' | 'font2', 'font1' | 'font2', 'font1' | 'font2']
}

export interface SpliceLLMResult {
  default: SpliceCharParams
  perChar: Record<string, SpliceCharParams>
}

const FALLBACK_SPLICE: SpliceLLMResult = {
  default: { cut1: 333, cut2: 667, zones: ['font1', 'font2', 'font1'] },
  perChar: {},
}

function isZone(v: unknown): v is 'font1' | 'font2' {
  return v === 'font1' || v === 'font2'
}

function parseSpliceParams(raw: unknown): SpliceCharParams | null {
  if (!raw || typeof raw !== 'object') return null
  const r = raw as Record<string, unknown>
  const cut1 = typeof r.cut1 === 'number' ? Math.round(r.cut1) : null
  const cut2 = typeof r.cut2 === 'number' ? Math.round(r.cut2) : null
  const zones = Array.isArray(r.zones) && r.zones.length === 3 && r.zones.every(isZone)
    ? (r.zones as ['font1' | 'font2', 'font1' | 'font2', 'font1' | 'font2'])
    : null
  if (cut1 === null || cut2 === null || !zones) return null
  return { cut1: Math.max(0, Math.min(1000, cut1)), cut2: Math.max(0, Math.min(1000, cut2)), zones }
}

export function extractSpliceResult(raw: string): SpliceLLMResult {
  const match = raw.match(/\{[\s\S]*\}/)
  if (!match) return FALLBACK_SPLICE
  try {
    const parsed = JSON.parse(match[0]) as Record<string, unknown>
    const defaultParams = parseSpliceParams(parsed.default) ?? FALLBACK_SPLICE.default
    const perChar: Record<string, SpliceCharParams> = {}
    if (parsed.perChar && typeof parsed.perChar === 'object') {
      for (const [ch, val] of Object.entries(parsed.perChar as Record<string, unknown>)) {
        const p = parseSpliceParams(val)
        if (p && ch.length === 1) perChar[ch] = p
      }
    }
    return { default: defaultParams, perChar }
  } catch {
    return FALLBACK_SPLICE
  }
}

function isGradientColors(v: unknown): v is [string, string] | [string, string, string] {
  return Array.isArray(v) && (v.length === 2 || v.length === 3) && v.every((c) => typeof c === 'string')
}
