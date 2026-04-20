export type ColrEffect = '3d-blocks' | 'shadow' | 'outline' | 'double-outline' | 'fill' | 'gradient' | 'highlight'

export interface ColrConfig {
  effects: Set<ColrEffect>
  fillColor?: string
  gradientColors?: [string, string] | [string, string, string]
  outlineColor?: string
  blockColor?: string
}

export type Transform =
  | { type: 'scaleX'; factor: number }
  | { type: 'shear'; angle: number }
  | { type: 'jitter'; amplitude: number }
  | { type: 'wave'; amplitude: number; frequency: number }
  | { type: 'waveY'; amplitude: number; frequency: number }
  | { type: 'rotate'; angle: number }
  | { type: 'perspective'; depth: number }
  | { type: 'arch'; amplitude: number }
