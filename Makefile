# Makefile for Rust Workspace with Versioning

.PHONY: help version current tags patch minor major release test build check clippy fmt fmt-check clean check-versioning git-versions security-check all

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
	cargo fmt

clean:
	cargo clean
	rm -rf target/

# Check versioning system
check-versioning:
	@echo "Checking versioning system setup..."
	@test -f scripts/app/version.sh || (echo "❌ version.sh not found" && exit 1)
	@test -f scripts/app/security-check.sh || (echo "❌ security-check.sh not found" && exit 1)
	@test -f Makefile || (echo "❌ Makefile not found" && exit 1)
	@test -f Cargo.toml || (echo "❌ Cargo.toml not found" && exit 1)
	@echo "✅ Versioning system setup complete"

# Check available git versions
git-versions:
	@echo "Available versions for git dependencies:"
	@git tag --sort=-version:refname | head -10
	@echo ""
	@echo "Example usage:"
	@echo '  icu = { git = "https://github.com/dragginzgame/icu", tag = "v0.1.10", features = [] }'

# Security check for tag immutability
security-check:
	@./scripts/app/security-check.sh

# Build and test everything
all: clean check fmt-check clippy test build
