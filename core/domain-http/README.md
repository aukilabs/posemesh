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
- Universal compatibility: [JavaScript package](https://www.npmjs.com/package/@auki/domain-http) works in browsers, Deno, and Node.js(v18+ with ReadableStream support).


## Changelog

See [CHANGELOG.md](./CHANGELOG.md) for a list of features, bug fixes, and breaking changes.

## Development

To build the `domain-http` crate locally for different platforms and for WebAssembly (WASM), follow these instructions:

### 1. Build for Native (Host) Platform

From the repository root, run:
```sh
cargo build -p posemesh-domain-http --release
```

### 2. Build for a Different Target Platform (Cross-Compile)

For cross-compiling, use [`cross`](https://github.com/cross-rs/cross). Install cross if you don't already have it:
```sh
cargo install cross
```

Then, build for your desired target. For example, to build for ARM64 Linux:
```sh
cross build -p posemesh-domain-http --release --target aarch64-unknown-linux-gnu
```
Replace `aarch64-unknown-linux-gnu` with the appropriate [Rust target triple](https://doc.rust-lang.org/nightly/rustc/platform-support.html) for your platform.

### 3. Build for WebAssembly (WASM)

To build the WASM package, use the provided Makefile task:
```sh
make build-domain-http-wasm
```
This will produce the WASM package in `domain-http/pkg`.

### 4. Additional Notes

- Make sure you have the required Rust target installed:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- For JavaScript or web integration, see the output in the `domain-http/pkg` directory after building for WASM.

For any further development, see each crate's README for more details.

## Test

To run all tests (including both Rust and JS/WASM tests), use:

```sh
make unit-tests
```

This will run all unit and integration tests for both the Rust crate and the JavaScript/WASM package.

## Publish

To publish a new version of this crate, follow these steps:

1. **Update the Version in `Cargo.toml`**
   - If you are publishing a development/unstable release (such as during pre-release testing or RCs), increment the version in `Cargo.toml` to an unstable version suffix, such as:
     - `1.5.0-alpha.1`
     - `1.5.0-beta.1`
     - `1.5.0-rc.1`
   - Only stable releases (e.g., `1.5.0`, with no `-` suffix) are published to crates.io by CI.
   - **Stable versions:**
     - When you are ready for a stable release, bump the version to a new stable version (e.g., `1.5.0`).
     - **Update the changelog** in `README.md` with the changes for the new release.

2. **Committing & PR**
   - Always commit and push your version bump (and changelog update if stable) in your PR.

3. **Publishing**
   - For stable versions: After merging your PR to the default branch, the CI will automatically build and publish the crate to crates.io and npm.js
   - For unstable/pre-release versions: You can publish manually if needed

## Contributing

Contributions are welcome! Please ensure that all tests pass before submitting a pull request.


