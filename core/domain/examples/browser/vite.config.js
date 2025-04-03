import { defineConfig } from 'vite';
import { resolve } from 'path';
import tailwindcss from '@tailwindcss/vite'

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
    rollupOptions: {
      external: ['posemesh-domain']
    }
  },
  resolve: {
    preserveSymlinks: true,
    alias: {
      'posemesh-domain': resolve(__dirname, './pkg/posemesh-domain.js')
    }
  },
  worker: {
    format: 'es'
  },
  plugins: [tailwindcss()]
}); 
