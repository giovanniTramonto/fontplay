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

function isGradientColors(v: unknown): v is [string, string] | [string, string, string] {
  return Array.isArray(v) && (v.length === 2 || v.length === 3) && v.every((c) => typeof c === 'string')
}
