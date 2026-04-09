import type { Transform } from '#shared/types'

/**
 * Extracts a transform array from an LLM response.
 * Expects: {"transforms": [{type, ...}, ...]}
 */
export function extractTransforms(raw: string): Transform[] | null {
  // Try complete JSON first
  const match = raw.match(/\{[\s\S]*\}/)
  if (match) {
    try {
      const parsed: unknown = JSON.parse(match[0])
      if (
        parsed !== null &&
        typeof parsed === 'object' &&
        Array.isArray((parsed as Record<string, unknown>).transforms)
      ) {
        return (parsed as { transforms: Transform[] }).transforms
      }
    } catch {
      // fall through to partial recovery
    }
  }

  // Recover complete transform objects from a truncated response
  const items = [...raw.matchAll(/\{[^{}]*"type"\s*:\s*"[^""]+"[^{}]*\}/g)]
  if (items.length > 0) {
    try {
      return items.map((m) => JSON.parse(m[0]) as Transform)
    } catch {
      // ignore
    }
  }

  return null
}
