# Posemesh Domain HTTP Tests

This directory contains comprehensive test configurations for the `posemesh-domain-http` package, supporting both Node.js and headless browser environments.

## Test Configurations

### Node.js Tests (`vitest.config.js`)
- **Environment**: Node.js
- **Purpose**: Unit tests, API testing, server-side functionality
- **Test Files**: `*.test.js` (excluding browser-specific tests)

### Browser Tests (`vitest.browser.config.js`)
- **Environment**: jsdom (default) + Playwright (headless browsers)
- **Purpose**: Browser integration, WASM loading, DOM manipulation, HTTP client testing
- **Test Files**: `browser.test.js` and other browser-specific tests

## Quick Start

### Setup
```bash
cd domain-http/js-tests
npm install
```

### Running Tests

#### Node.js Tests
```bash
# Run Node.js tests in watch mode
npm run test

# Run Node.js tests once
npm run test:run

# Run Node.js tests with coverage
npm run test:coverage

# Open Vitest UI for Node.js tests
npm run test:ui
```

#### Browser Tests
```bash
# Run browser tests in watch mode
npm run test:browser

# Run browser tests once
npm run test:browser:run

# Run browser tests with coverage
npm run test:browser:coverage

# Open Vitest UI for browser tests
npm run test:browser:ui

# Run browser tests in watch mode
npm run test:browser:watch
```

#### All Tests
```bash
# Run both Node.js and browser tests
npm run test:all
```
