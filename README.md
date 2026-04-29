# fontplay
Upload a font, play with it, download the result.

fontplay applies AI-generated transforms and color effects to fonts — entirely in the browser. A Rust/WASM module rewrites glyph outlines and builds a new TTF client-side.

### Demo
**[giovanni-fontplay.netlify.app](https://giovanni-fontplay.netlify.app/)**

## Features

### Style
Pick a style (Modern, Cyber, Playful, Edgy, Cool) — the AI chooses geometric transforms (scale, shear, rotate, wave, …) and COLRv1 color effects (fill, gradient, shadow, 3D blocks, highlight, outline). The result is a fully styled font you can download.

### Blend
Upload a second font. The glyphs of both fonts are rasterised to canvas bitmaps and blended via SDF morphing in WASM. Use the slider to control the mix ratio.

### Splice
Upload a second font. The AI slices each glyph horizontally into three zones and assigns each zone to one of the two fonts — creating a typographic hybrid. Choose an intensity (Low / Medium / High) to control how dramatically the fonts are mixed.

---

The original font is never modified. Each operation re-applies from the original bytes.

## Stack

- **Vue 3** · Composition API · TypeScript strict
- **Vite 8**
- **Rust + WASM** · [skrifa](https://docs.rs/skrifa) (reading) + [write-fonts](https://docs.rs/write-fonts) (writing) · compiled via `wasm-pack`
- **AI** · Ollama locally · Anthropic Claude in production (via Netlify Functions)
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

Copy `.env.example` → `.env` and set `VITE_OLLAMA_URL` + `VITE_OLLAMA_MODEL` for local AI. Without the env vars, the app calls the Netlify Functions (requires deployment).

### Build

```bash
npm run build        # type-check + WASM build + Vite production build
npm run build:wasm   # compile Rust → WASM only
npm run format       # Biome format
npm run lint         # Biome lint
```

## Deployment (Netlify)

Set `ANTHROPIC_API_KEY` in the Netlify dashboard. Do **not** set `VITE_OLLAMA_URL` in production — its absence triggers the Netlify Function path.

## Font output

The exported TTF contains:
- Rewritten `glyf` outlines with all transforms applied to every glyph
- **COLRv1** paint trees (Chrome, web browsers)
- **COLRv0** solid-color layers (macOS CoreText — Font Book, Pages, TextEdit)
- Updated name table with the mood/operation appended to the font name
