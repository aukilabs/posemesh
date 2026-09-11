# Dependency maintenance

The September 11, 2026 review uses PR #142 at `59ddade` as its baseline.
It covers the Cargo workspace, all npm manifests, and the Python binding's
build/test requirements. Native third-party submodules were not upgraded.

## Compatibility boundaries

- Keep the Auki SDK crates on the same Git revision. This update preserves
  `3ee1142529d5f64af87f6fc2276d3a6a78e728cc` and libp2p 0.56.
- Preserve Rust 1.89 support. Resolve Cargo updates with
  `--config 'resolver.incompatible-rust-versions="fallback"'` and check with
  Rust 1.89 before committing the lockfile.
- Use the committed npm lockfiles with `npm ci`. Generate the local WASM
  packages before installing their example/test consumers.
- Keep Vite on 6.4 and Babel on 7 for this update. Vitest moves to 4.1.11
  with its explicit Playwright provider; the browser tests do not use jsdom.

## Reviewed updates

| Dependency | Baseline | Reviewed lockfile |
| --- | --- | --- |
| bytes | 1.10.1 | 1.12.1 |
| time | 0.3.43 | 0.3.55 |
| keccak | 0.1.5 | 0.1.6 |
| quinn-proto | 0.11.13 | 0.11.17 |
| jsonwebtoken | 9.3.1 and 10.4.0 | 10.4.0 |
| h2 | 0.4.12 | 0.4.19 |
| rustls-webpki | 0.103.4 | 0.103.15 |
| postgres-protocol | 0.6.8 | 0.6.12 |
| tokio-postgres | 0.7.13 | 0.7.18 |
| yamux (default transport) | 0.13.6 | 0.13.10 |
| rand (existing release lines) | 0.8.5 and 0.9.2 | 0.8.8 and 0.9.5 |
| anyhow | 1.0.99 | 1.0.104 |
| event-listener (5.x) | 5.4.1 | 5.4.2 |
| Vite (browser example) | 6.2.3 | 6.4.3 |
| Rollup (browser example) | 4.37.0 | 4.63.1 |
| Vitest | ^3.2.4, no committed lockfile | 4.1.11 |

JWT consumers share the SDK's `rust_crypto` backend. The Domain crate now
explicitly enables ring's `std` and `wasm32_unknown_unknown_js` features,
which JWT v9 previously enabled transitively. The offline JWT tests exercise
Ed25519 key generation, signing, verification, wrong-key rejection and expiry
on native Rust and WASM.

The protobuf generator uses the `pb-rs` library with its CLI features disabled.
This removes obsolete clap 2, env_logger 0.7, ansi_term and atty dependencies
without changing the generator version.

Both Vite consumers override the top-level-await plugin's UUID dependency to
11.1.1 or later within v11. The plugin pins vulnerable UUID 10; the override
retains its v4/v5 APIs and is validated by the example build and browser test.
The browser example declares its local Domain WASM package so a clean install
does not depend on a prior manual npm link.

The Web SDK uses patched Babel 7 packages and depends directly on
`babel-preset-minify` 0.5.2. Its CMake build invokes Babel with that preset;
the unused `babel-minify` CLI introduced the vulnerable yargs-parser package.

## Outstanding upstream dependencies

These findings remain visible; no audit suppressions were added.

| Dependency | Finding and follow-up |
| --- | --- |
| hickory-proto 0.25.2 | [Encoding CPU exhaustion](https://rustsec.org/advisories/RUSTSEC-2026-0119.html) needs 0.26.1+. The pinned SDK, libp2p-dns 0.44 and libp2p-mdns 0.48 require 0.25. Coordinate the SDK/libp2p migration. |
| hickory-proto 0.25.2 | [NSEC3 validation loop](https://rustsec.org/advisories/RUSTSEC-2026-0118.html) also remains in the lockfile scan. DNSSEC is not enabled in the reviewed resolved feature graph. Recheck features when upgrading the SDK. |
| rsa 0.9.10 | [Marvin timing attack](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) has no patched release listed. JWT's RustCrypto backend includes RSA; the reviewed Domain/P2P paths use Ed25519. Removing RSA requires coordination with the SDK's crypto-backend selection. |
| yamux 0.12.1 | [Malformed-frame panic](https://github.com/advisories/GHSA-vxx9-2994-q338) remains flagged by GitHub's version range. libp2p-yamux 0.47 includes both 0.12 and 0.13 unconditionally. All reviewed Posemesh and pinned SDK call sites use `Config::default`, selecting patched 0.13.10. Upstream removal of the legacy implementation is still needed to remove this lockfile finding. |

RustSec additionally reports unmaintained `async-std`, `bincode` and `paste`
dependencies. Retiring these requires changes to their parent libraries and
is separate from the compatible security updates above.

## Validation and repeatable checks

The review reduced cargo-audit's vulnerability count from 15 to 3 and removed
all of its unsoundness warnings. GitHub advisory data was checked separately;
it includes JWT and Yamux findings absent from this RustSec scan.
All three npm lockfiles audit with zero known vulnerabilities. The Python
requirements, resolved with `uv pip compile` and checked with pip-audit, also
reported zero known vulnerabilities in the review environment. Python's
existing ranges were retained; they are not a committed lockfile.

From `core/`:

```sh
cargo +1.89.0 check --locked --workspace --all-targets
make ci-compute-node test-domain-auth
wasm-pack test --node domain --test auth_jwt --locked
wasm-pack build domain --target bundler --dev
make build-domain-http TARGET=wasm
```

On macOS, Domain WASM builds need a Clang with the WASM backend, such as
Homebrew LLVM. Set `CC_wasm32_unknown_unknown` and
`AR_wasm32_unknown_unknown` to that installation's `clang` and `llvm-ar`.

After generating the WASM packages:

```sh
cd core/domain/examples/browser
npm ci
npm run build
npm audit
```

```sh
cd core/domain-http/bindings/javascript/tests
npm ci
npx playwright install chromium
npm run test:bindings
npm audit
```

`test:bindings` runs offline in Node and Chromium. The complete `test:all`
suite and `make unit-tests` use live credentials and can mutate Domain data;
they were not run in this dependency review. Worker validation passed 110
Rust tests and 7 Python tests, plus the two JWT tests on native Rust and WASM.
The Web SDK's seven standalone JavaScript modules passed Babel transformation
and a minifier behavior check; full native SDK platform builds were not run.
