# Building

Notes for building the Rust workspace in `core/`. Verified on Linux (Ubuntu, kernel 6.8) with Rust 1.96.

## Prerequisites

- Rust 1.89 or newer (the workspace sets `rust-version = "1.89.0"`)
- `cmake`, `gcc`, `g++`, `pkg-config`
- `protoc`

On Debian/Ubuntu:

```sh
sudo apt install build-essential cmake pkg-config protobuf-compiler
```

## Building core

```sh
cd core
cargo build
```

That is the whole thing. It pulls around 450 crates and takes roughly two minutes on a laptop.

## Submodules

`third-party/` carries OpenCV, protobuf, glm and ios-cmake as submodules. They belong to the SDK build, not to the Rust workspace. You do not need `git submodule update --init` to build `core/` — a fresh clone with empty submodule directories compiles fine.

If you are building the SDK, start from `sdk/` instead.

## Makefile targets

The targets in `core/Makefile` are written for macOS. `OS` and `ARCH` default to `macOS` and `ARM64`, and `build-domain` shells out to `scripts/Build-Library.ps1`, which requires PowerShell:

```
$ make build-domain
/usr/bin/env: 'pwsh': No such file or directory
make: *** [Makefile:14: build-domain] Error 127
```

To use it on Linux you need `pwsh` installed and the platform passed explicitly. If you only need the Rust library, `cargo build` is enough — the script additionally handles toolchain installation and packaging for Apple targets.

## WASM

```sh
make build-domain-wasm
```

Requires `wasm-pack`.
