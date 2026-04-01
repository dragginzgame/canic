.PHONY: help version tags patch patch-quick minor major package publish \
        test-packaged-downstream \
        test test-wasm test-bump build check clippy fmt fmt-check clean install-dev \
        demo-install test-watch all ensure-clean \
        ensure-hooks install-hooks

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
	@echo "  install-all      Install both dev and canister dependencies"
	@echo "  install-dev      Install Rust development dependencies"
	@echo "  install-canister-deps  Install Wasm target + candid tools"
	@echo "  install-hooks    Configure git hooks"
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
	@echo "  test-packaged-downstream  Verify the hidden packaged-downstream wasm_store build path"
	@echo ""
	@echo "Development:"
	@echo "  demo-install    Install the full local reference topology (fails if dfx is not already running)"
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
	@echo "  test-watch       Run tests in watch mode"
	@echo "  all              Run all checks, tests, and build"
	@echo ""
	@echo "Examples:"
	@echo "  make patch       # Bump patch version"
	@echo "  make patch-quick # Fast patch bump using cargo check"
	@echo "  make demo-install # Build + install the local reference topology"
	@echo "  make test        # Run clippy and workspace tests"
	@echo "  make test-wasm   # Fast wasm iteration path without PocketIC/e2e"
	@echo "  make build       # Build project"

#
# Installing
#

# Install everything (dev + canister deps)
install-all: install-dev install-canister-deps install-hooks
	@echo "✅ All development and canister dependencies installed"

# Install Rust development tooling
install-dev:
	cargo install cargo-watch --locked || true
	cargo install cargo-edit --locked || true
	cargo install cargo-get cargo-sort cargo-sort-derives --locked || true

# Install wasm target + candid tools
install-canister-deps:
	rustup toolchain install 1.94.1 || true
	rustup target add wasm32-unknown-unknown
	cargo install candid-extractor ic-wasm --locked || true


# Optional explicit install target (idempotent)
install-hooks ensure-hooks:
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

patch-quick: ensure-clean fmt quick-bump
	@scripts/ci/bump-version.sh patch

minor: ensure-clean fmt test-bump
	@scripts/ci/bump-version.sh minor

major: ensure-clean fmt test
	@scripts/ci/bump-version.sh major

package: ensure-clean
	$(CARGO_ENV) cargo package

publish: ensure-clean
	$(CARGO_ENV) scripts/ci/publish-workspace.sh

test-packaged-downstream:
	$(CARGO_ENV) scripts/ci/verify-packaged-downstream-wasm-store.sh

#
# Tests
#

demo-install:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) cargo run -q -p canic-internal --bin install_reference_topology -- root

test: clippy test-unit

# Fast iteration path for wasm work.
# Skips integration tests under `tests/`, which is where the PocketIC-heavy
# suites live today.
test-wasm: clippy test-unit-fast

# Version-bump gate.
# Keeps clippy plus the fast unit/lib/bin workspace run, while leaving the
# local `dfx` smoke path as an explicit manual target.
test-bump: clippy test-unit-fast

quick-bump:
	$(CARGO_ENV) cargo check --workspace

# Keep rust test execution single-threaded for PocketIC stability.
# Parallel test threads can trigger PocketIC panics like:
# `KeyAlreadyExists { key: "nns_subnet_id", version: 2 }` and incomplete HTTP messages.
test-unit:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) cargo test --workspace -- --test-threads=1

test-unit-fast:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) cargo test --workspace --lib --bins -- --test-threads=1

test-canisters: demo-install
	test_pid="$$(TMPDIR="$(TEST_TMPDIR)" dfx canister call root canic_subnet_registry --output json | python3 -c 'import json,sys; data=json.load(sys.stdin); matches=[entry["pid"] for entry in data.get("Ok", []) if entry.get("role")=="test"]; print(matches[0]) if matches else sys.exit("root canic_subnet_registry did not contain role '\''test'\''")')"; \
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

# Run tests in watch mode
test-watch:
	cargo watch -x test

# Build and test everything
all: ensure-clean ensure-hooks clean fmt-check check test build
