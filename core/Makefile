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
	@make build-domain-http-wasm
	@cd domain-http/js-tests && npm install && npm run test:all
	@cargo test --release --all-features -- --test-threads=1
