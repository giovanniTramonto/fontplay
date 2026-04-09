import { resolve } from 'node:path'
import vue from '@vitejs/plugin-vue'
import wasm from 'vite-plugin-wasm'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue(), wasm()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '#shared': resolve(__dirname, 'shared'),
    },
  },
})
