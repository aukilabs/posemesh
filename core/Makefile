# Makefile to build, test domain
# make build-domain - build domain packages with all dependencies

OS ?= "macOS"
ARCH ?= "ARM64"
CONFIG ?= "Release"

.PHONY: build-domain build-domain-wasm unit-tests build-domain-http-wasm publish-domain-http publish-utils

build-domain:
ifeq ($(ARCH),WASM)
	$(MAKE) build-domain-wasm
else
	./scripts/Build-Library.ps1 $(OS) $(ARCH) $(CONFIG) posemesh-domain -InstallNecessaryRustToolchainsAndTargets
endif

build-domain-wasm:
	wasm-pack build --target bundler --release domain
	sed -i '' 's/"name": "posemesh-domain"/"name": "@aukilabs\/posemesh-domain"/' domain/pkg/package.json

build-domain-http:
ifeq ($(TARGET),wasm)
	$(MAKE) build-domain-http-wasm
else
	$(MAKE) build-domain-http-uniffi LANG=$(TARGET)
endif

build-domain-http-wasm:
	@rm -rf domain-http/bindings/javascript/pkg
	@CRATE_NAME=posemesh-domain-http CRATE_DIR=domain-http OUT_DIR=bindings/javascript/pkg ./build-wasm.sh

build-domain-http-uniffi:	
	@rm -rf domain-http/bindings/python/pkg
	@if [ -z "$(LANG)" ]; then \
		echo "Please specify LANG. Usage: make build-domain-http-uniffi LANG=python"; \
		exit 1; \
	fi
	@if [ "$(LANG)" = "python" ]; then \
		cd domain-http/bindings/python && \
		cp ../../README.md . && \
		python3 -m venv .venv && \
		. .venv/bin/activate && \
		pip install -r requirements.txt && \
		maturin develop; \
	else \
		OUT_DIR=domain-http/bindings/$(LANG)/pkg; \
		mkdir -p $$OUT_DIR; \
		cargo run --features=uniffi/cli --bin uniffi-bindgen -p uniffi-bindgen generate domain-http/src/domain-client.udl --language $(LANG) --out-dir $$OUT_DIR; \
	fi

unit-tests:
	@make build-domain-http-wasm
	@cd domain-http/bindings/javascript/tests && npm install && npx playwright install chromium && npm run test:all
	@make build-domain-http-uniffi LANG=python
	@cd domain-http/bindings/python && \
		if [ -f "../../../.env" ]; then \
			set -o allexport; . ../../../.env; set +o allexport; \
		fi; \
		if [ -f "../../../.env.local" ]; then \
			set -o allexport; . ../../../.env.local; set +o allexport; \
		fi; \
		. .venv/bin/activate && pytest -v -s
	@cargo test --release --all-features -- --test-threads=1

publish-domain-http:
	@VERSION=`grep '^version =' domain-http/Cargo.toml | head -n 1 | sed -E 's/version = "(.*)"/\1/'`; \
	echo "posemesh-domain-http version: $$VERSION, will be published to cargo, npm, and PyPI"; \
	IS_UNSTABLE=`echo "$$VERSION" | grep -E '.*-rc|.*-beta|.*-alpha'` ; \
	if [ -z "$$IS_UNSTABLE" ]; then \
		if ! grep -q "## v$${VERSION}" domain-http/CHANGELOG.md; then \
			echo "ERROR: No changelog for v$${VERSION} found in domain-http/CHANGELOG.md"; \
			exit 1; \
		fi; \
		if [ -z "$$CI" ]; then \
			echo "Stable releases should only be published by CI. Aborting."; \
			exit 1; \
		fi; \
	fi; \
	if npm view posemesh-domain-http@$$VERSION > /dev/null 2>&1; then \
		echo "ERROR: Version $$VERSION is already published on npm."; \
		exit 1; \
	fi; \
	if [ -z "$$CI" ]; then \
		if make unit-tests; then \
			read -p "Continue with publish v$$VERSION? [y/N]: " CONFIRM; \
			if [ "$$CONFIRM" != "y" ] && [ "$$CONFIRM" != "Y" ]; then \
				echo "Publish cancelled."; \
				exit 1; \
			fi; \
		else \
			echo "Unit tests failed. Aborting publish."; \
			exit 1; \
		fi; \
	fi; \
	cargo publish -p posemesh-domain-http; \
	make build-domain-http-wasm; \
	cd domain-http/bindings/javascript/pkg && \
	if [ -z "$$IS_UNSTABLE" ]; then \
		npm publish --access public --tag stable; \
	else \
		npm publish; \
	fi;

publish-utils:
	@VERSION=`grep '^version =' utils/Cargo.toml | head -n 1 | sed -E 's/version = "(.*)"/\1/'`; \
	echo "posemesh-utils version: $$VERSION, will be published to crates.io"; \
	cargo publish -p posemesh-utils
