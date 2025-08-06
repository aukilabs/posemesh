# posemesh-domain-http

This package provides an HTTP client library for interacting with AukiLabs domain data services. It enables Rust applications to authenticate, upload, and download domain data using both native and WebAssembly (WASM) targets. The library is designed to be used as part of the Posemesh Core ecosystem, allowing seamless integration with AukiLabs' distributed data infrastructure.

Key features:
- Authenticate as an app or user to obtain access tokens for domain operations.
- Download domain data with streaming support for efficient handling of large datasets.
- Upload domain data, supporting both creation and update operations.
- Cross-platform support: works on native platforms (using `tokio` and `reqwest`) and in the browser via WASM.

This crate is intended for developers building applications or services that need to interact with AukiLabs' domain data APIs in a secure and efficient manner.
