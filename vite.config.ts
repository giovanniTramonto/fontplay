import { writeFileSync } from 'node:fs'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { resolve } from 'node:path'
import vue from '@vitejs/plugin-vue'
import { defineConfig, type Plugin } from 'vite'
import wasm from 'vite-plugin-wasm'

const debugBitmapPlugin: Plugin = {
  name: 'debug-bitmap',
  configureServer(server) {
    server.middlewares.use('/debug-bitmap', (req: IncomingMessage, res: ServerResponse) => {
      if (req.method !== 'POST') { res.statusCode = 405; res.end(); return }
      const filename = (req.headers['x-filename'] as string) || 'debug.png'
      const chunks: Buffer[] = []
      req.on('data', (chunk: Buffer) => chunks.push(chunk))
      req.on('end', () => {
        writeFileSync(resolve(__dirname, 'public', 'debug', filename), Buffer.concat(chunks))
        res.statusCode = 200
        res.setHeader('Access-Control-Allow-Origin', '*')
        res.end('ok')
      })
    })
  },
}

export default defineConfig({
  plugins: [vue(), wasm(), debugBitmapPlugin],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '#shared': resolve(__dirname, 'shared'),
    },
  },
})
