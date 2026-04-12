# fontplay

Upload a font, pick a mood, download the result.

fontplay applies AI-generated geometric transforms and color effects to every glyph in a font — entirely in the browser. A Rust/WASM module rewrites the outlines and builds a new TTF file client-side. The styled font is injected as a `@font-face` and can be downloaded with the mood baked into the font name.

## How it works

1. **Upload** a `.ttf`, `.otf`, or `.woff2` file
2. **Type** any text — the display updates live using your font
3. **Pick a mood** — the AI chooses transforms (scale, shear, jitter, wave) and color effects (fill, gradient, shadow, 3D blocks)
4. **Download** the styled TTF — installable in Font Book, etc.

The original font is never modified. Each mood change re-applies transforms from the original bytes.

## Stack

- **Vue 3** · Composition API · TypeScript strict
- **Vite 8**
- **Rust + WASM** · [skrifa](https://docs.rs/skrifa) (reading) + [write-fonts](https://docs.rs/write-fonts) (writing) · compiled via `wasm-pack`
- **AI** · Ollama locally (`qwen2.5-coder`) · Anthropic Claude in production (via Netlify Function)
- **Biome** · formatter + linter

## Getting started

### Prerequisites

```bash
brew install rustup && rustup-init
rustup target add wasm32-unknown-unknown
brew install wasm-pack
```

### Install & run

```bash
npm install
npm run dev        # builds WASM + starts Vite dev server
```

For local AI, copy `.env.example` → `.env` and set `VITE_OLLAMA_URL`. Without it, the app calls the Netlify Function (requires deployment).

### Build

```bash
npm run build        # type-check + WASM build + Vite production build
npm run build:wasm   # compile Rust → WASM only
npm run format       # Biome format
npm run lint         # Biome lint
```

## Deployment (Netlify)

Set `ANTHROPIC_API_KEY` in the Netlify dashboard. Do **not** set `VITE_OLLAMA_URL` in production — its absence is what triggers the Netlify Function path.

## Font output

The exported TTF contains:
- Rewritten `glyf` outlines with transforms applied to every glyph
- **COLRv1** paint trees (Chrome, web browsers)
- **COLRv0** solid-color layers (macOS CoreText — Font Book, Pages, TextEdit)
- Updated name table with the mood appended to the full name and PostScript name
