import { resolve } from 'node:path'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  plugins: [vue(), wasm()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '#shared': resolve(__dirname, 'shared'),
    },
  },
})
