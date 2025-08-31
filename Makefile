.PHONY: help version current tags patch minor major release test build check clippy fmt fmt-check clean check-versioning git-versions security-check all install-dev install-canister-deps test-watch

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
	@echo "  release          Create a release with current version"
	@echo ""
	@echo "Development:"
	@echo "  test             Run all tests"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  clippy           Run clippy checks"
	@echo "  fmt              Format code"
	@echo "  clean            Clean build artifacts"
	@echo "  examples         Build crate examples (with and without 'ic')"
	@echo "  install-canister-deps  Install Wasm target and tools"
	@echo "  examples         Build crate examples (with and without 'ic')"
	@echo ""
	@echo "Utilities:"
	@echo "  check-versioning Check versioning system setup"
	@echo "  git-versions     Check available git dependency versions"
	@echo "  security-check   Check tag immutability and version integrity"
	@echo ""
	@echo "Examples:"
	@echo "  make patch       # Bump patch version"
	@echo "  make test        # Run tests"
	@echo "  make build       # Build project"

# Version management
version:
	@./scripts/app/version.sh current

current:
	@./scripts/app/version.sh current

tags:
	@./scripts/app/version.sh tags

patch:
	@./scripts/app/version.sh patch

minor:
	@./scripts/app/version.sh minor

major:
	@./scripts/app/version.sh major

release:
	@./scripts/app/version.sh release

# Development commands
test:
	cargo test --workspace

build:
	cargo build --release --workspace



check:
	cargo check --workspace

clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean
	rm -rf target/

# Build examples
examples:
	cargo build -p icu --examples
	cargo build -p icu --examples --features ic

# Install Wasm target and candid tools
install-canister-deps:
	rustup toolchain install 1.89.0 || true
	rustup target add wasm32-unknown-unknown
	cargo install candid-extractor --locked || true

# Install development dependencies
install-dev:
	cargo install cargo-watch

# Run tests in watch mode
test-watch:
	cargo watch -x test

# Check versioning system
check-versioning:
	@./scripts/app/check-versioning.sh

# Check available git versions
git-versions:
	@./scripts/app/check-git-versions.sh

# Security check for tag immutability
security-check:
	@./scripts/app/security-check.sh

# Build and test everything
all: clean check fmt-check clippy test build 
