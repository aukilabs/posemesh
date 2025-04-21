# Makefile to build, lint and run integration tests
# make - build domain packages with all dependencies
# make run - run all integration tests

RUNNERS = \
	client-example \
	domain-manager \
	data-node \
	relay \
	test-concurrent

OS ?= "macOS"
ARCH ?= "ARM64"
CONFIG ?= "Release"

RUN_TARGETS = $(addprefix run-,$(RUNNERS))

.PHONY: all install fix run $(FIX_TARGETS) $(RUN_TARGETS)

all: install

install:
	./scripts/Build-Library.ps1 $(OS) $(ARCH) $(CONFIG) domain -InstallNecessaryRustToolchainsAndTargets

# TODO: Some runners don't stop on their own. Cant be run in parallel as use the same port.
run: $(RUN_TARGETS)

$(RUN_TARGETS):
	cargo run --package $(patsubst run-%,%,$@)
