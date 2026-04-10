import type { ColrConfig, ColrEffect, Transform } from '#shared/types'

export interface LLMResult {
  transforms: Transform[]
  colr: ColrConfig
}

export function extractResult(raw: string): LLMResult | null {
  const match = raw.match(/\{[\s\S]*\}/)
  if (!match) return null

  try {
    const parsed = JSON.parse(match[0]) as Record<string, unknown>

    const transforms = Array.isArray(parsed.transforms) ? (parsed.transforms as Transform[]) : []

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

function parseColr(raw: unknown): ColrConfig {
  if (!raw || typeof raw !== 'object') return { effects: new Set() }
  const e = raw as Record<string, unknown>

  const active = Array.isArray(e.active) ? (e.active as ColrEffect[]) : []
  // never allow fill + gradient simultaneously
  const filtered = active.includes('gradient') ? active.filter((x) => x !== 'fill') : active

  return {
    effects: new Set(filtered),
    fillColor: typeof e.fillColor === 'string' ? e.fillColor : undefined,
    gradientColors: isColorPair(e.gradientColors) ? e.gradientColors : undefined,
    outlineColor: typeof e.outlineColor === 'string' ? e.outlineColor : undefined,
    blockColor: typeof e.blockColor === 'string' ? e.blockColor : undefined,
  }
}

function isColorPair(v: unknown): v is [string, string] {
  return Array.isArray(v) && v.length === 2 && typeof v[0] === 'string' && typeof v[1] === 'string'
}
