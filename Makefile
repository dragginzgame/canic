.PHONY: help version current tags patch minor major release \
        test test-unit test-canisters build check clippy fmt fmt-check clean plan install-dev \
        test-watch all ensure-clean examples install-canister-deps

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
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Bump patch version (0.0.x)"
	@echo "  minor            Bump minor version (0.x.0)"
	@echo "  major            Bump major version (x.0.0)"
	@echo "  release          CI-driven release (local target is no-op)"
	@echo ""
	@echo "Development:"
	@echo "  test             Build canister tests (dfx) then run cargo tests"
	@echo "  test-unit        Run Rust unit/integration tests (cargo only)"
	@echo "  test-canisters   Build/install test canisters via scripts/app/test.sh (requires dfx)"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  clippy           Run clippy checks"
	@echo "  fmt              Format code"
	@echo "  fmt-check        Check formatting"
	@echo "  clean            Clean build artifacts"
	@echo "  examples         Build crate examples"
	@echo "  plan             Show the current project plan"
	@echo ""
	@echo "Utilities:"
	@echo "  install-dev      Install development dependencies"
	@echo "  install-canister-deps  Install Wasm target and candid tools"
	@echo "  test-watch       Run tests in watch mode"
	@echo "  all              Run all checks, tests, and build"
	@echo ""
	@echo "Examples:"
	@echo "  make patch       # Bump patch version"
	@echo "  make test        # Run tests"
	@echo "  make build       # Build project"

# Version management (always format first)
version:
	cargo get workspace.package.version

tags:
	@git tag --sort=-version:refname | head -10

patch: ensure-clean fmt
	@scripts/ci/bump-version.sh patch

minor: ensure-clean fmt
	@scripts/ci/bump-version.sh minor

major: ensure-clean fmt
	@scripts/ci/bump-version.sh major

release: ensure-clean
	@echo "Release handled by CI on tag push"

#
# Installing
#

# Install Rust development tooling
install-dev:
	cargo install cargo-watch --locked || true
	cargo install cargo-edit --locked || true
	cargo install cargo-get cargo-sort cargo-sort-derives --locked || true

# Install wasm target + candid tools
install-canister-deps:
	rustup toolchain install 1.90.0 || true
	rustup target add wasm32-unknown-unknown
	cargo install candid-extractor --locked || true

# Install everything (dev + canister deps)
install-all: install-dev install-canister-deps
	@echo "✅ All development and canister dependencies installed"

#
# Development commands
#

# Build canister tests (via dfx) first, then run cargo tests. This ensures any
# wasm/canister artifacts used by tests are built before Rust tests execute.
test: test-canisters test-unit

test-unit:
	cargo test --workspace

test-canisters:
	@if command -v dfx >/dev/null 2>&1; then \
		dfx canister create --all -qq; \
		dfx build --all; \
		dfx ledger fabricate-cycles --canister root --cycles 9000000000000000 || true; \
		dfx canister install root --mode=reinstall -y; \
	else \
		echo "Skipping canister tests (dfx not installed)"; \
	fi

build: ensure-clean
	cargo build --release --workspace

check: fmt-check test-canisters
	cargo check --workspace

clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo clippy --workspace -- -D warnings

clean:
	cargo clean

# Security and versioning checks
security-check:
	@echo "Security checks are enforced via GitHub settings:"
	@echo "- Enable Protected Tags for pattern 'v*' (Settings → Tags)"
	@echo "- Restrict who can create tags and disable force pushes"
	@echo "- Require PR + CI on 'main' via branch protection"
	@echo "This target is informational only; no local script runs."

check-versioning: security-check
	bash scripts/ci/security-check.sh

# Planning summary
plan:
	@echo "=== PLAN.md ==="
	@{ [ -f PLAN.md ] && sed -n '1,200p' PLAN.md; } || echo "No PLAN.md found."
	@echo
	@echo "=== .codex/plan.json ==="
	@{ [ -f .codex/plan.json ] && sed -n '1,200p' .codex/plan.json; } || echo "No .codex/plan.json found."

# Run tests in watch mode
test-watch:
	cargo watch -x test

# Build and test everything
all: ensure-clean fmt-check clippy check test build

# Build examples
examples:
	cargo build --workspace --examples

