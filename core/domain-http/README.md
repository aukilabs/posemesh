# posemesh-domain-http

A cross-platform HTTP client library for interacting with posemesh domains on the Auki Network. Supports both native and WebAssembly (WASM) environments.

**Key Features:**
- Secure authentication and authorization with the Auki Network.
- Efficient streaming download of domain data, enabling seamless handling of large datasets.
- Flexible upload functionality for both creating and updating domain data.
- Universal compatibility: [JavaScript package](https://www.npmjs.com/package/posemesh-domain-http) works in browsers, Deno, and Node.js(v18+ with ReadableStream support).

# Changelog

## v1.0.0

### Features
- Added comprehensive tests for the JavaScript SDK.
- Introduced support for using Zitadel tokens to access domain servers.
- Added a method to list available domains.
- Improved and polished JavaScript/TypeScript documentation.
- Added a method to delete domain data by ID.
- Enabled streaming download of domain data in the WASM build for efficient handling of large datasets.

### Bug Fixes
- Fixed an issue where streaming download of domain data failed if multiple domain data objects were present in a single chunk.

### Breaking Changes
- Replaced most JavaScript classes with plain objects using `serde_wasm_bindgen` for better interoperability.
- Removed the `callback` parameter from the JS `downloadDomainData` method to provide a more idiomatic JavaScript developer experience.
- Renamed the `logout` parameter to `remember_password` for clarity (`logout` is now equivalent to `!remember_password`).
