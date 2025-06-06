import { defineConfig } from 'vite';
import { resolve } from 'path';
import tailwindcss from '@tailwindcss/vite'
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  optimizeDeps: {
    exclude: ['posemesh-domain']
  },
  server: {
    fs: {
      strict: false
    }
  },
  build: {
    target: 'es2020',
  },
  worker: {
    format: 'es'
  },
  plugins: [tailwindcss(), wasm(), topLevelAwait()]
}); 
