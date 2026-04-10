import type { Transform } from '#shared/types'

function seeded(seed: number) {
  let s = seed
  return () => {
    s = (Math.imul(1664525, s) + 1013904223) | 0
    return ((s >>> 0) / 0xffffffff) * 2 - 1
  }
}

function applyToPoint(
  x: number,
  y: number,
  transforms: Transform[],
  rand: () => number,
): [number, number] {
  let nx = x
  let ny = y
  for (const t of transforms) {
    switch (t.type) {
      case 'scaleX':
        nx *= t.factor
        break
      case 'scaleY':
        ny *= t.factor
        break
      case 'shear':
        nx += ny * Math.tan((t.angle * Math.PI) / 180)
        break
      case 'jitter':
        nx += rand() * t.amplitude
        ny += rand() * t.amplitude
        break
      case 'wave':
        nx += Math.sin(ny * t.frequency) * t.amplitude
        break
    }
  }
  return [nx, ny]
}

function r(n: number): number {
  return Math.round(n * 10) / 10
}

interface PathToken {
  cmd: string
  args: number[]
}

function tokenize(d: string): PathToken[] {
  const parts = d.match(/[MLCQZHVmlcqzhv][^MLCQZHVmlcqzhv]*/g) ?? []
  return parts.map((part) => ({
    cmd: part[0],
    args:
      part
        .slice(1)
        .match(/-?[\d.]+(?:[eE][+-]?\d+)?/g)
        ?.map(Number) ?? [],
  }))
}

export function getPathBounds(
  d: string,
): { minX: number; minY: number; maxX: number; maxY: number } | null {
  let minX = Number.POSITIVE_INFINITY
  let minY = Number.POSITIVE_INFINITY
  let maxX = Number.NEGATIVE_INFINITY
  let maxY = Number.NEGATIVE_INFINITY
  let found = false

  for (const { cmd, args } of tokenize(d)) {
    const u = cmd.toUpperCase()
    if (u === 'Z') continue

    // Extract only x,y coordinate pairs based on command type
    const pairs: [number, number][] = []
    if (u === 'M' || u === 'L') {
      for (let i = 0; i + 1 < args.length; i += 2) pairs.push([args[i], args[i + 1]])
    } else if (u === 'C') {
      for (let i = 0; i + 5 < args.length; i += 6) {
        pairs.push([args[i], args[i + 1]], [args[i + 2], args[i + 3]], [args[i + 4], args[i + 5]])
      }
    } else if (u === 'Q') {
      for (let i = 0; i + 3 < args.length; i += 4) {
        pairs.push([args[i], args[i + 1]], [args[i + 2], args[i + 3]])
      }
    }

    for (const [x, y] of pairs) {
      found = true
      if (x < minX) minX = x
      if (x > maxX) maxX = x
      if (y < minY) minY = y
      if (y > maxY) maxY = y
    }
  }

  return found ? { minX, minY, maxX, maxY } : null
}

export function applyTransformsToPath(d: string, transforms: Transform[], seed = 0): string {
  if (!d || !transforms.length) return d

  const rand = seeded(seed)
  const out: string[] = []

  for (const { cmd, args } of tokenize(d)) {
    const u = cmd.toUpperCase()

    if (u === 'Z') {
      out.push('Z')
      continue
    }

    if (u === 'M' || u === 'L') {
      const a: number[] = []
      for (let i = 0; i < args.length; i += 2) {
        const [nx, ny] = applyToPoint(args[i], args[i + 1], transforms, rand)
        a.push(r(nx), r(ny))
      }
      out.push(cmd + a.join(' '))
      continue
    }

    if (u === 'C') {
      const a: number[] = []
      for (let i = 0; i < args.length; i += 6) {
        const [x1, y1] = applyToPoint(args[i], args[i + 1], transforms, rand)
        const [x2, y2] = applyToPoint(args[i + 2], args[i + 3], transforms, rand)
        const [x, y] = applyToPoint(args[i + 4], args[i + 5], transforms, rand)
        a.push(r(x1), r(y1), r(x2), r(y2), r(x), r(y))
      }
      out.push(cmd + a.join(' '))
      continue
    }

    if (u === 'Q') {
      const a: number[] = []
      for (let i = 0; i < args.length; i += 4) {
        const [x1, y1] = applyToPoint(args[i], args[i + 1], transforms, rand)
        const [x, y] = applyToPoint(args[i + 2], args[i + 3], transforms, rand)
        a.push(r(x1), r(y1), r(x), r(y))
      }
      out.push(cmd + a.join(' '))
      continue
    }

    out.push(cmd + args.join(' '))
  }

  return out.join('')
}
