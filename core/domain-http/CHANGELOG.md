## v1.4.0

### Features
- Formatted errors
- Supported domain creation and deletion
- Renamed JS package to @auki/domain-http

## v1.2.0

### Features
- Submit jobs to reconstruction servers

## v1.1.0

### Features
- List domains and their domain servers in the given organization

## v1.0.0

### Features
- Added comprehensive tests for the JavaScript SDK.
- Introduced support for using OIDC access tokens to access domain servers.
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
