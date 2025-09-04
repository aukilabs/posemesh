# posemesh-domain-http

This package provides an HTTP client library for interacting with posemesh domains on the Auki Network, supporting both native and WebAssembly targets.

Key features:
- Authenticate as an app or user to obtain access tokens for domain operations.
- Download domain data with streaming support for efficient handling of large datasets.
- Upload domain data, supporting both creation and update operations.
- Cross-platform support: works on native platforms (using `tokio` and `reqwest`) and in the browser via WASM.

## Auki Console
Create an account on https://console.auki.network, and create an app for your application if you only need read access.
