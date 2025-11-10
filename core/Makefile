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

build-domain-http-wasm:
	@rm -rf domain-http/pkg
	@CRATE_NAME=posemesh-domain-http CRATE_DIR=domain-http ./build-wasm.sh

unit-tests:
	@make build-domain-http-wasm
	@cd domain-http/js-tests && npm install && npx playwright install chromium && npm run test:all
	@cargo test --release --all-features -- --test-threads=1

publish-domain-http:
	@VERSION=`grep '^version =' domain-http/Cargo.toml | head -n 1 | sed -E 's/version = "(.*)"/\1/'`; \
	echo "posemesh-domain-http version: $$VERSION, will be published to npm and cargo"; \
	IS_STABLE=`echo "$$VERSION" | grep -E '.*-rc|.*-beta|.*-alpha'` ; \
	if [ -z "$$IS_STABLE" ]; then \
		if ! grep -q "## v$${VERSION}" domain-http/README.md; then \
			echo "ERROR: No changelog for v$${VERSION} found in domain-http/README.md"; \
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
	fi
	@cargo publish -p posemesh-domain-http
	@cd domain-http/pkg && \
	if [ -z "$$IS_STABLE" ]; then \
		npm publish --tag "v$$VERSION"; \
	else \
		npm publish; \
	fi

publish-utils:
	@VERSION=`grep '^version =' utils/Cargo.toml | head -n 1 | sed -E 's/version = "(.*)"/\1/'`; \
	if [ -z "$$CI" ]; then \
		read -p "Continue with publish? [y/N]: " CONFIRM; \
		if [ "$$CONFIRM" != "y" ] && [ "$$CONFIRM" != "Y" ]; then \
			echo "Publish cancelled."; \
			exit 1; \
		fi; \
	fi
	@cargo publish -p posemesh-utils
