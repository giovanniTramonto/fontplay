# fontplay

Upload a font, see its glyphs rendered as SVG, and transform them with AI-powered moods.

## What it does

1. **Upload** a font file (.ttf / .otf / .woff / .woff2)
2. **Write** any letters — they render as SVG paths extracted directly from the font
3. **Choose a mood** (Modern, Futuristic, Playful, Edgy, Cool) — an LLM picks a creative combination of geometric transforms and applies them to the actual path coordinates

Unlike CSS transforms, fontplay modifies the real glyph path data — making results exportable as new fonts in the future.

## Stack

- **Vue 3** + TypeScript (Composition API, `<script setup>`)
- **Rust + WASM** via [skrifa](https://docs.rs/skrifa) — parses font files and extracts SVG paths in the browser
- **LLM** — Ollama locally, Claude (Anthropic) in production via Netlify Function
- **Vite 8**, Biome, @csstools/normalize.css

## Getting started

### Prerequisites

Install Rust via rustup (required for the WASM target):

```bash
brew install rustup
rustup-init
rustup target add wasm32-unknown-unknown
```

Install wasm-pack:

```bash
brew install wasm-pack
```

### Install & run

```bash
npm install
npm run dev
```

### Local AI (Ollama)

```bash
cp .env.example .env
# make sure ollama serve is running
```

The app calls Ollama directly from the browser when `VITE_OLLAMA_URL` is set. Without it, it calls the Netlify Function.

## How the AI works

Each mood button sends the mood name to the LLM. The LLM responds with a small JSON array of transforms:

```json
{"transforms": [{"type": "scaleX", "factor": 0.75}, {"type": "shear", "angle": -10}]}
```

Available transforms: `scaleX`, `scaleY`, `shear`, `jitter`, `wave`

JavaScript applies these to every coordinate in the SVG path data — the glyph shapes actually change, not just their CSS appearance.

## Deployment (Netlify)

- Set `ANTHROPIC_API_KEY` in the Netlify dashboard
- Do not set `VITE_OLLAMA_URL` in production
- The build command installs wasm-pack and compiles Rust automatically

## Commands

```bash
npm run dev          # build WASM + start dev server
npm run build        # build WASM + type-check + production build
npm run build:wasm   # compile Rust → WASM only
npm run format       # Biome format
npm run lint         # Biome lint
```
