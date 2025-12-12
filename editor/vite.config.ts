import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  base: process.env.GITHUB_PAGES ? '/wasm_engine/' : '/',
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
  ],
  build: {
    target: 'esnext',
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
});