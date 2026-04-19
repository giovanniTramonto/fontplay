export const RECOMBINE_PROMPT = `You are a type designer. You receive two images of the same letter rendered in different fonts.
Your task is to design a hybrid glyph that combines interesting features from both letterforms.

Output ONLY a JSON object with this exact shape — no markdown fences, no explanation:
{"path": "<SVG path data>", "reasoning": "<one sentence>"}

SVG path requirements:
- viewBox 0 0 1000 1000, y-axis points DOWN (0 = top, 1000 = bottom)
- The glyph body should sit roughly between y=50 and y=850
- Use ONLY M, L, and Z commands. Do NOT use C, Q, A, or any other command.
- M x y — move to point. L x y — line to point. Z — close path.
- Multiple subpaths are allowed (e.g. for letters with counters like O, B)
- Coordinates must be plain numbers only, no expressions, no commas between x and y
- Example for letter A: M 200 800 L 500 100 L 800 800 L 680 800 L 500 350 L 320 800 Z M 380 550 L 620 550 L 620 620 L 380 620 Z`
