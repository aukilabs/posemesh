# Contributing

The developer path starts with [compute nodes and robots](README.md).
The Rust workspace is in `core/`; run Cargo and Make commands there.
Use Rust 1.89+ and Python 3 for the worker checks.

## Check the worker runtime

~~~sh
cd core
make ci-compute-node
~~~

This checks Rust formatting, Clippy, runner API/host/Echo tests, and the Python
submission helper's unit tests. These use local fixtures and mocks; running
the paired tutorial is a separate integration check that needs configured
DDS/DMS services and workers.

For docs changes, check relative paths and heading links, verify referenced
commands against the manifests and source, and run `git diff --check`.
Compile complete Rust snippets in a separate project as described in
[Write a runner](docs/how-to/write-a-runner.md). Keep the paired Echo example
as the executable reference and update its guide when behavior changes.

## Repository map

| Path | Purpose |
| --- | --- |
| [`core/compute-node/`](core/compute-node/) | Compute and robot hosts, DDS/DMS clients, leases and storage |
| [`core/compute-node-runner-api/`](core/compute-node-runner-api/) | Runner ports and the paired Echo example |
| [`core/domain-http/`](core/domain-http/) | Domain HTTP client, WASM and Python bindings |
| [`core/node-registration/`](core/node-registration/) | Wallet registration helpers used by the compute host |
| [`core/networking/`](core/networking/), [`core/domain/`](core/domain/), [`core/base/`](core/base/) | Earlier networking/domain stack and native base library |
| [`sdk/`](sdk/) | C++ spatial SDK and generated C/JavaScript/Objective-C/Swift interfaces |
| [`third-party/`](third-party/) | Submodules and build scripts for native SDK dependencies |

The earlier networking and C++ components have separate build paths. The
compute/robot quickstart does not exercise them. Keep source and generated
interfaces in their existing locations when changing those components.

## Domain HTTP and bindings

From `core/`, build the Rust client with:

~~~sh
cargo build --locked -p posemesh-domain-http
~~~

The existing [Makefile](core/Makefile) provides `make build-domain-http TARGET=wasm`
and `make build-domain-http TARGET=python`. WASM uses `wasm-pack` and the
`wasm32-unknown-unknown` target; Python uses a local virtual environment,
requirements file, and `maturin`. The Python build copies the client README
into its package directory.

`make unit-tests` also builds and tests the bindings and Domain HTTP client.
Those tests can contact configured services and mutate Domain data; read the
[JavaScript tests](core/domain-http/bindings/javascript/tests/basic.test.ts)
and [Python tests](core/domain-http/bindings/python/tests/test_basic.py) and
use a designated test account/Domain. They are not part of the offline worker
check. Build output and generated bindings should come from the scripts.

The [publish workflow](.github/workflows/build-and-publish.yml) packages
`posemesh-utils`, `posemesh-domain-http`, `@auki/domain-client`, and
`auki-domain-client`. Domain HTTP version changes belong in its Cargo manifest
and [CHANGELOG](core/domain-http/CHANGELOG.md); the publish target checks for
`## v<version>`. Use that workflow and its Make targets for release behavior.

## C++ SDK maintenance

The [native SDK workflow](.github/workflows/posemesh-sdk.yml) is the reference
for platform dependencies and build order. It initializes submodules,
generates interfaces, builds the Rust base library and third-party libraries,
then runs the SDK build script.

Interface definitions are in [`sdk/interface/`](sdk/interface/) and enums in
[`sdk/enum/`](sdk/enum/). Regenerate from `sdk/gentool/` with `npm run generate`.
Do not hand-edit generated interface code. Platform builds use
[`core/scripts/Build-Library.ps1`](core/scripts/Build-Library.ps1) and
[`sdk/scripts/Build-Library.ps1`](sdk/scripts/Build-Library.ps1); follow the
workflow's platform/architecture arguments and prerequisite installation.

## Submitting a change

For dependency updates, follow the [dependency maintenance reference](docs/reference/dependencies.md)
for SDK/Rust compatibility, lockfiles, offline checks and outstanding upstream
advisories.

Use the existing formatters (`cargo fmt` for Rust and the root `.clang-format`
for C++) and Conventional Commits. Keep the README and guides focused on
supported developer tasks. Put API
contracts beside the interfaces, and link to provider-owned authentication or
scheduling details instead of duplicating them. Keep design history in Git
and PRs. Describe behavior changes, affected hosts or bindings, exact checks
run, and any integration or platform checks you could not run.
