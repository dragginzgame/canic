.PHONY: help version tags patch patch-quick minor major package publish \
        test-packaged-downstream-wasm-store \
        test-packaged-downstream-installer test-installed-canic-installer \
        test test-wasm test-bump build check clippy fmt fmt-check clean \
        install-dev update-dev demo-install \
        ensure-clean ensure-hooks test-unit test-unit-fast \
        test-canisters fmt-core cloc

# in case we need to use this
CARGO_ENV :=
TEST_TMPDIR ?= $(CURDIR)/.tmp/test-runtime

# Network defaults and mapping for build-time DFX_NETWORK
NETWORK ?= local
DFX_NETWORK ?=

ifeq ($(DFX_NETWORK),)
  ifeq ($(NETWORK),local)
    DFX_NETWORK := local
  else ifeq ($(NETWORK),ic)
    DFX_NETWORK := ic
  else ifeq ($(NETWORK),mainnet)
    DFX_NETWORK := ic
  else ifeq ($(NETWORK),staging)
    DFX_NETWORK := ic
  endif
endif

ifeq ($(DFX_NETWORK),)
  $(error DFX_NETWORK must be set to 'local' or 'ic')
endif
ifneq ($(DFX_NETWORK),local)
  ifneq ($(DFX_NETWORK),ic)
    $(error DFX_NETWORK must be set to 'local' or 'ic' (got $(DFX_NETWORK)))
  endif
endif

export DFX_NETWORK
CARGO_ENV := DFX_NETWORK=$(DFX_NETWORK) $(CARGO_ENV)
ifneq ($(CANIC_WASM_PROFILE),)
export CANIC_WASM_PROFILE
CARGO_ENV := CANIC_WASM_PROFILE=$(CANIC_WASM_PROFILE) $(CARGO_ENV)
endif

# Check for clean git state
ensure-clean:
	@if ! git diff-index --quiet HEAD --; then \
		echo "🚨 Working directory not clean! Please commit or stash your changes."; \
		exit 1; \
	fi

# Default target
help:
	@echo "Available commands:"
	@echo ""
	@echo "Setup / Installation:"
	@echo "  install-dev      Install the shared Rust/Cargo/Python/Canic toolchain"
	@echo "  update-dev       Update the local Rust/Cargo/Python/DFX development environment"
	@echo "  ensure-hooks     Configure git hooks"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Bump patch version (0.0.x)"
	@echo "  patch-quick      Bump patch version with cargo check instead of test-bump"
	@echo "  minor            Bump minor version (0.x.0)"
	@echo "  major            Bump major version (x.0.0)"
	@echo "  package          Build a publishable crate tarball"
	@echo "  publish          Publish workspace crates to registry in dependency order"
	@echo "  test-packaged-downstream-wasm-store  Verify the hidden packaged-downstream wasm_store build path"
	@echo "  test-packaged-downstream-installer  Verify the packaged-downstream installer manifest path"
	@echo "  test-installed-canic-installer  Verify the installed-binary canic-installer path"
	@echo ""
	@echo "Development:"
	@echo "  demo-install    Install the full local reference topology with fast wasm by default (fails if dfx is not already running)"
	@echo "  test             Run clippy + workspace tests (PocketIC/Cargo only)"
	@echo "  test-wasm        Run clippy + fast non-PocketIC tests for wasm iteration"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  clippy           Run clippy checks"
	@echo "  fmt              Format code"
	@echo "  fmt-check        Check formatting"
	@echo "  clean            Clean build artifacts"
	@echo ""
	@echo "Utilities:"
	@echo "  cloc             Show runtime vs test Rust LOC across canic crates"
	@echo ""
	@echo "Examples:"
	@echo "  make patch       # Bump patch version"
	@echo "  make patch-quick # Fast patch bump using cargo check"
	@echo "  make demo-install # Fast local install using fast wasm (override with CANIC_WASM_PROFILE=debug|fast|release)"
	@echo "  make test        # Run clippy and workspace tests"
	@echo "  make test-wasm   # Fast wasm iteration path without PocketIC/e2e"
	@echo "  make build       # Build project"

#
# Installing
#

# Install the shared Rust/Cargo/Canic toolchain
install-dev:
	bash scripts/dev/install_dev.sh

# Update the local Rust/Cargo/Python/DFX development environment.
update-dev:
	bash scripts/dev/install_dev.sh --update-python
	rustup update
	cargo install \
		cargo-audit cargo-bloat cargo-deny cargo-expand cargo-machete \
		cargo-llvm-lines cargo-sort cargo-tarpaulin cargo-sort-derives \
		ripgrep \
		candid-extractor ic-wasm
	cargo audit
	cargo update --verbose
	dfxvm self update


# Optional explicit install target (idempotent)
ensure-hooks:
	@if [ -d .git ]; then \
		git config --local core.hooksPath .githooks || true; \
		chmod +x .githooks/* 2>/dev/null || true; \
		echo "✅ Git hooks configured (core.hooksPath -> .githooks)"; \
	else \
		echo "⚠️  Not a git repo, skipping hooks setup"; \
	fi


#
# Version management (always format first)
#

version:
	@cargo get workspace.package.version

tags:
	@git tag --sort=-version:refname | head -10

patch: ensure-clean fmt test-bump
	@scripts/ci/bump-version.sh patch

patch-quick: ensure-clean fmt
	$(CARGO_ENV) cargo check --workspace
	@scripts/ci/bump-version.sh patch

minor: ensure-clean fmt test-bump
	@scripts/ci/bump-version.sh minor

major: ensure-clean fmt test
	@scripts/ci/bump-version.sh major

package: ensure-clean
	$(CARGO_ENV) cargo package

publish: ensure-clean
	$(CARGO_ENV) scripts/ci/publish-workspace.sh

test-packaged-downstream-wasm-store:
	$(CARGO_ENV) scripts/ci/verify-packaged-downstream-wasm-store.sh

test-packaged-downstream-installer:
	$(CARGO_ENV) scripts/ci/verify-packaged-downstream-installer.sh

test-installed-canic-installer:
	$(CARGO_ENV) scripts/ci/verify-installed-canic-installer.sh

#
# Tests
#

demo-install:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" CANIC_WASM_PROFILE="$(if $(CANIC_WASM_PROFILE),$(CANIC_WASM_PROFILE),fast)" $(CARGO_ENV) cargo run -q -p canic-installer --bin canic-install-root -- root

test: clippy test-unit

# Fast iteration path for wasm work.
# Skips integration tests under `tests/`, which is where the PocketIC-heavy
# suites live today.
test-wasm: clippy test-unit-fast

# Version-bump gate.
# Keeps clippy plus the fast unit/lib/bin workspace run, while leaving the
# local `dfx` fast path as an explicit manual target.
test-bump: clippy test-unit-fast

# Keep rust test execution single-threaded inside each test binary for PocketIC
# stability and deterministic fixture reuse.
# Integration test binaries are run explicitly in sequence so they do not queue
# behind the shared PocketIC runtime lock and look hung at startup.
# Parallel test threads can still trigger PocketIC panics like:
# `KeyAlreadyExists { key: "nns_subnet_id", version: 2 }` and incomplete HTTP messages.
test-unit:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) bash scripts/ci/run-workspace-tests.sh full

test-unit-fast:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) bash scripts/ci/run-workspace-tests.sh fast

test-canisters: demo-install
	test_pid="$$(TMPDIR="$(TEST_TMPDIR)" dfx canister call root canic_subnet_registry --output json | jq -er '.Ok[] | select(.role == "test") | .pid' | head -n1)"; \
	TMPDIR="$(TEST_TMPDIR)" dfx canister call "$$test_pid" test

#
# Development commands
#

build:
	$(CARGO_ENV) cargo build --workspace --release

check: ensure-hooks fmt
	$(CARGO_ENV) cargo check --workspace

clippy:
	$(CARGO_ENV) cargo clippy --workspace --all-targets --all-features -- -D warnings

fmt: ensure-hooks fmt-core

fmt-core:
	cargo sort --workspace
	cargo sort-derives
	cargo fmt --all

fmt-check: ensure-hooks fmt-check-core

fmt-check-core:
	cargo sort --workspace --check
	cargo sort-derives --check
	cargo fmt --all -- --check

clean:
	cargo clean

cloc:
	bash scripts/dev/cloc.sh
