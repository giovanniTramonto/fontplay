export type Transform =
  | { type: 'scaleX'; factor: number }
  | { type: 'scaleY'; factor: number }
  | { type: 'shear'; angle: number }
  | { type: 'jitter'; amplitude: number }
  | { type: 'wave'; amplitude: number; frequency: number }
