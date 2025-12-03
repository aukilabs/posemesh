# Posemesh Domain HTTP Tests

This directory contains comprehensive test configurations for the `@auki/domain-client` package, supporting both Node.js and headless browser environments.

## Test Configurations

### Node.js Tests (`vitest.config.js`)
- **Environment**: Node.js
- **Purpose**: Unit tests, API testing, server-side functionality
- **Test Files**: `*.test.js` (excluding browser-specific tests)

### Browser Tests (`vitest.browser.config.js`)
- **Environment**: jsdom (default) + Playwright (headless browsers)
- **Purpose**: Browser integration, WASM loading, DOM manipulation, HTTP client testing
- **Test Files**: `browser.test.js` and other browser-specific tests

