import { resolve } from 'path';
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import { defineConfig } from 'vitest/config';
import path from 'path';
import dotenv from 'dotenv';

dotenv.config({ path: '../../.env' });
let config = dotenv.config({ path: '../../.env.local', override: true }).parsed;

export default defineConfig({
    server: {
      fs: {
        allow: [
          '.', // project root
          path.resolve(__dirname, '../pkg'), // external pkg folder
        ],
      },
    },
    test: {
        // Test environment
        environment: 'jsdom',

        // Glob patterns for test files
        include: ['**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],

        // Exclude patterns
        exclude: ['**/node_modules/**', '**/dist/**', '**/cypress/**', '**/.{idea,git,cache,output,temp}/**'],

        // Global test setup
        globals: true,

        // Test timeout (30 seconds)
        testTimeout: 30000,

        // Hook timeout (10 seconds)
        hookTimeout: 10000,

        // Browser-specific configuration
        browser: {
            enabled: true,
            name: 'chromium', // or 'firefox', 'safari', 'edge'
            provider: 'playwright', // or 'webdriver'
            headless: true,
            instances: [
                { browser: 'chromium' },
            ],
            // Disable screenshots
            screenshotFailures: false,
        },
        // Add browser-safe configuration for tests
        env: config
    },

    // Resolve configuration for browser environment
    resolve: {
        alias: {
            // Alias for the WASM module
            'posemesh-domain-http': resolve(__dirname, '../pkg'),
        },
    },

    // Define global variables available in tests
    define: {
        // Add any global constants needed for testing
        __TEST__: true,
        __BROWSER__: true,
    },

    // Optimize dependencies
    optimizeDeps: {
        exclude: ['posemesh-domain-http'],
    },

    // Plugins for WASM support
    plugins: [wasm(), topLevelAwait()],

    // Build configuration
    build: {
        target: 'es2020',
    },

    // Worker configuration
    worker: {
        format: 'es'
    },
});
