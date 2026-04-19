export function makeBlendPrompt(
  font1Name: string,
  font2Name: string,
  font1Paths: Record<string, string>,
  font2Paths: Record<string, string>,
): string {
  const formatPaths = (paths: Record<string, string>) =>
    Object.entries(paths)
      .map(([ch, path]) => `  '${ch}': ${path}`)
      .join('\n') || '  (no glyphs available)'

  return `You are blending two fonts together.

Font 1 "${font1Name}" — glyph outlines (coordinates normalized by UPM, Y axis up):
${formatPaths(font1Paths)}

Font 2 "${font2Name}" — glyph outlines:
${formatPaths(font2Paths)}

Analyze the structural differences: contour count, curve complexity, proportions, stroke style.
Choose a blendFactor (0.0 = pure Font 1, 1.0 = pure Font 2) that creates an interesting blend based on the actual shapes.

Return ONLY a JSON object — no markdown, no explanation:
{ "blendFactor": 0.5 }`
}
