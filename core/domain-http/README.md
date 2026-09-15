# Domain HTTP client

`posemesh-domain-http` provides authentication and Domain data operations for
Rust, JavaScript/WASM, and Python. The Posemesh worker host now uses the Auki
SDK for its task storage ports. Runner authors normally use `TaskCtx.input` and
`TaskCtx.output`; see the
[runner reference](https://github.com/aukilabs/posemesh/blob/main/docs/reference/runner.md).

## Use the client directly

| Language | Package | API and examples |
| --- | --- | --- |
| Rust | `posemesh-domain-http` | [Source](https://github.com/aukilabs/posemesh/tree/main/core/domain-http/src) |
| JavaScript/TypeScript | `@auki/domain-client` | [Binding API](https://github.com/aukilabs/posemesh/blob/main/core/domain-http/src/wasm.rs), [examples in tests](https://github.com/aukilabs/posemesh/blob/main/core/domain-http/bindings/javascript/tests/basic.test.ts) |
| Python | `auki-domain-client` (`auki_domain_client` import) | [Binding contract](https://github.com/aukilabs/posemesh/blob/main/core/domain-http/src/domain-client.udl), [examples in tests](https://github.com/aukilabs/posemesh/blob/main/core/domain-http/bindings/python/tests/test_basic.py) |

The client supports User and App credentials, Domain listing, and Domain data
uploads/downloads. Configure API and DDS endpoints for the same environment.
The linked integration tests require an account and Domain; inspect their
setup before running them. The JavaScript OIDC suite uses `AUTH_TEST_TOKEN` to
check authentication and reads; the shared account needs read access to the test
Domain. Local binding fixtures cover OIDC viewer and writer responses, including
denied authentication, denied Domain access and HTTP 403 on writes after a
successful read. Live App tests verify that creates and updates are denied;
User tests cover successful writes. Run the local fixtures in Node and Chromium
with `npm run test:bindings` from `core/domain-http/bindings/javascript/tests`.

## Build from source

Run from `posemesh/core`:

~~~sh
cargo build --locked -p posemesh-domain-http
~~~

For WASM, use `make build-domain-http TARGET=wasm`. For Python, use
`make build-domain-http TARGET=python`. Prerequisites, test behavior, and
release instructions are in
[Contributing](https://github.com/aukilabs/posemesh/blob/main/CONTRIBUTING.md#domain-http-and-bindings).

See the [changelog](https://github.com/aukilabs/posemesh/blob/main/core/domain-http/CHANGELOG.md)
for published changes. This README is also included in the Python package.
