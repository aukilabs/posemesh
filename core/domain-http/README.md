# Domain HTTP

A cross-platform HTTP client library for interacting with posemesh domains on the Auki Network. Supports both native and WebAssembly (WASM) environments.

## Authentication Modes

posemesh-domain-http supports multiple authentication methods, each providing different levels of access to domain data:

### 1. Sign in with App Credentials

- **How:** Use your app's key and secret to authenticate.
- **Access:**  
  - **Read access** to **all domains**.
  - **No write access**—app credentials are intended for read-only operations.
- **Use case:** Suitable for backend services or applications that need to fetch domain data but do not need to modify it.

### 2. Sign in with Auki User Credentials

- **How:** Authenticate using a user's email and password.
- **Access:**  
  - **Write access** to domains **owned by the user** within the same organization.
  - **Read access** to **all domains**.
- **Use case:** Use this mode when you need to allow users to manage (create, update, or delete) their own domains, as well as view other domains in the organization.

### 3. Authenticate with OIDC Access Token

- **How:** Provide a valid OIDC (OpenID Connect) access token obtained from the authentication service.
- **Access:**  
  - Grants **read and write access** to domains, according to the roles assigned to the user.
- **Use case:** Enables single sign-on and fine-grained access control. Permissions are determined by the user’s assigned roles.

**Key Features:**
- Secure authentication and authorization with the Auki Network.
- Efficient streaming download of domain data, enabling seamless handling of large datasets.
- Flexible upload functionality for both creating and updating domain data.
- Universal compatibility: [JavaScript package](https://www.npmjs.com/package/posemesh-domain-http) works in browsers, Deno, and Node.js(v18+ with ReadableStream support).

# Changelog

## v1.4.0

### Features
- Formatted errors
- Added domain creation and deletion
- Package renaming to @auki/domain-http

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
