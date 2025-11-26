import { resolve } from 'path';
import { defineConfig } from 'vitest/config.js';
import dotenv from 'dotenv';

dotenv.config({ path: '../../../../.env' });
let config = dotenv.config({ path: '../../../../.env.local', override: true }).parsed;
export default defineConfig({
    test: {
        // Test environment
        environment: 'node',

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
        env: config
    },

    // Resolve configuration for Node.js environment
    resolve: {
        alias: {
            // Alias for the WASM module
            '@auki/domain-http': resolve(__dirname, '../pkg'),
        },
    },

    // Define global variables available in tests
    define: {
        // Add any global constants needed for testing
        __TEST__: true,
    },

    // Optimize dependencies
    optimizeDeps: {
        exclude: ['@auki/domain-http'],
    },
});
