export type ColrEffect = '3d-blocks' | 'shadow' | 'outline' | 'fill' | 'gradient'

export interface ColrConfig {
  effects: Set<ColrEffect>
  fillColor?: string
  gradientColors?: [string, string]
  outlineColor?: string
  blockColor?: string
}

export type Transform =
  | { type: 'scaleX'; factor: number }
  | { type: 'scaleY'; factor: number }
  | { type: 'shear'; angle: number }
  | { type: 'jitter'; amplitude: number }
  | { type: 'wave'; amplitude: number; frequency: number }
