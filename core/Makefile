# Makefile to build, test domain
# make build-domain - build domain packages with all dependencies

OS ?= "macOS"
ARCH ?= "ARM64"
CONFIG ?= "Release"

.PHONY: build-domain build-domain-wasm unit-tests build-domain-http-wasm

build-domain:
ifeq ($(ARCH),WASM)
	$(MAKE) build-domain-wasm
else
	./scripts/Build-Library.ps1 $(OS) $(ARCH) $(CONFIG) posemesh-domain -InstallNecessaryRustToolchainsAndTargets
endif

build-domain-wasm:
	wasm-pack build --target bundler --release domain
	sed -i '' 's/"name": "posemesh-domain"/"name": "@aukilabs\/posemesh-domain"/' domain/pkg/package.json

build-domain-http-wasm:
	@rm -rf domain-http/pkg
	@CRATE_NAME=posemesh-domain-http CRATE_DIR=domain-http ./build-wasm.sh

unit-tests:
	WASM_BINDGEN_USE_BROWSER=1 wasm-pack test --chrome --headless domain-http
	wasm-pack test --node domain-http
	cargo test --release --all-features

fmt-compute-node:
	cargo fmt -p posemesh-compute-node-runner-api -p posemesh-compute-node -- --check

clippy-compute-node:
	cargo clippy -p posemesh-compute-node-runner-api -p posemesh-compute-node --all-targets -- -D warnings

test-compute-node:
	cargo test -p posemesh-compute-node-runner-api
	cargo test -p posemesh-compute-node

ci-compute-node: fmt-compute-node clippy-compute-node test-compute-node
