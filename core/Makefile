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
	./scripts/Build-Library.ps1 $(OS) $(ARCH) $(CONFIG) domain -InstallNecessaryRustToolchainsAndTargets
endif

build-domain-wasm:
	wasm-pack build --target bundler --release domain
	sed -i '' 's/"name": "posemesh-domain"/"name": "@aukilabs\/posemesh-domain"/' domain/pkg/package.json

build-domain-http-wasm:
	rm -rf domain-http/pkg
	wasm-pack build --target bundler --release domain-http
	cd domain-http/pkg && npm link

unit-tests:
	WASM_BINDGEN_USE_BROWSER=1 wasm-pack test --chrome --headless domain-http
	wasm-pack test --node domain-http
	cargo test --all-features
	cargo test --release --all-features
