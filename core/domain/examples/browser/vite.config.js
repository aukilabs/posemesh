import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 3000
  },
  optimizeDeps: {
    exclude: ['posemesh-domain']
  },
  build: {
    target: 'es2020',
    sourcemap: true
  },
  resolve: {
    preserveSymlinks: true
  },
  worker: {
    format: 'es'
  }
}); 
