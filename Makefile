.PHONY: help version tags patch minor major \
        release-patch release-minor release-major \
        release-stage release-commit release-push package publish \
        test-packaged-downstream-wasm-store \
        test-packaged-downstream-cli test-installed-canic-cli \
        test test-wasm test-bump build check clippy fmt fmt-check clean \
        install install-dev update-dev test-fleet-install \
        ensure-clean ensure-hooks test-unit test-unit-fast \
        test-canisters fmt-core cloc

# in case we need to use this
CARGO_ENV :=
TEST_TMPDIR ?= $(CURDIR)/.tmp/test-runtime

ICP_ENVIRONMENT ?= local
ICP_CLI_VERSION ?= 0.2.5
ICP_WASM_VERSION ?= 0.9.10
export ICP_ENVIRONMENT
CARGO_ENV := ICP_ENVIRONMENT=$(ICP_ENVIRONMENT) $(CARGO_ENV)
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
	@echo "  install          Install only the local canic CLI binary"
	@echo "  install-dev      Install the shared Rust/Cargo/actionlint/Canic toolchain"
	@echo "  update-dev       Update the local Rust/Cargo/actionlint/ICP CLI development environment"
	@echo "  ensure-hooks     Configure git hooks"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Bump patch version (0.0.x)"
	@echo "  minor            Bump minor version (0.x.0)"
	@echo "  major            Bump major version (x.0.0)"
	@echo "  release-patch    Bump, stage, commit, tag, and push a patch release"
	@echo "  release-minor    Bump, stage, commit, tag, and push a minor release"
	@echo "  release-major    Bump, stage, commit, tag, and push a major release"
	@echo "  release-stage    Stage release version files after review"
	@echo "  release-commit   Commit and tag the staged release"
	@echo "  release-push     Push the release commit and tags"
	@echo "  package          Build a publishable crate tarball"
	@echo "  publish          Publish workspace crates to registry in dependency order"
	@echo "  test-packaged-downstream-wasm-store  Verify the hidden packaged-downstream wasm_store build path"
	@echo "  test-packaged-downstream-cli  Verify the packaged-downstream canic CLI manifest path"
	@echo "  test-installed-canic-cli  Verify the installed-binary canic CLI path"
	@echo ""
	@echo "Development:"
	@echo "  test-fleet-install  Install the full local test/reference topology with fast wasm by default"
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
	@echo "  make release-patch # Bump, stage, commit, tag, and push patch release"
	@echo "  make release-stage release-commit release-push # Finish reviewed manual bump"
	@echo "  make test-fleet-install # Fast local install using fast wasm (override with CANIC_WASM_PROFILE=debug|fast|release)"
	@echo "  make test        # Run clippy and workspace tests"
	@echo "  make test-wasm   # Fast wasm iteration path without PocketIC/e2e"
	@echo "  make build       # Build project"

#
# Installing
#

# Install only the local canic CLI binary.
install:
	cargo install --locked --path crates/canic-cli

# Install the shared Rust/Cargo/actionlint/Canic toolchain
install-dev:
	bash scripts/dev/install_dev.sh

# Update the local Rust/Cargo/actionlint/ICP CLI development environment.
update-dev:
	CANIC_AUTO_BUMP_ICP_TOOLS=1 bash scripts/dev/install_dev.sh --update-prereqs
	rustup update
	cargo install --quiet \
		cargo-audit cargo-bloat cargo-deny cargo-expand cargo-machete \
		cargo-llvm-lines cargo-sort cargo-tarpaulin cargo-sort-derives \
		ripgrep \
		candid-extractor
	icp --version
	ic-wasm --version
	cargo audit
	cargo update --quiet


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

minor: ensure-clean fmt test-bump
	@scripts/ci/bump-version.sh minor

major: ensure-clean fmt test
	@scripts/ci/bump-version.sh major

release-patch: patch release-stage release-commit release-push

release-minor: minor release-stage release-commit release-push

release-major: major release-stage release-commit release-push

release-stage:
	git add Cargo.toml Cargo.lock README.md crates/canic-host/README.md scripts/dev/install_dev.sh \
		scripts/ci/sync-release-surface-version.sh $$(git ls-files -m -- '*/Cargo.toml' || true)

release-commit:
	@version="$$(cargo get workspace.package.version)"; \
	if git rev-parse "v$$version" >/dev/null 2>&1; then \
		echo "❌ Tag v$$version already exists. Aborting." >&2; \
		exit 1; \
	fi; \
	git commit -m "Release $$version"; \
	git tag -a "v$$version" -m "Release $$version"

release-push:
	git push --follow-tags

package: ensure-clean
	$(CARGO_ENV) cargo package

publish: ensure-clean
	$(CARGO_ENV) scripts/ci/publish-workspace.sh

test-packaged-downstream-wasm-store:
	$(CARGO_ENV) scripts/ci/verify-packaged-downstream-wasm-store.sh

test-packaged-downstream-cli:
	$(CARGO_ENV) scripts/ci/verify-packaged-downstream-cli.sh

test-installed-canic-cli:
	$(CARGO_ENV) scripts/ci/verify-installed-canic-cli.sh

#
# Tests
#

test-fleet-install:
	@mkdir -p "$(TEST_TMPDIR)"
	TMPDIR="$(TEST_TMPDIR)" $(CARGO_ENV) cargo run -q -p canic-cli --bin canic -- install --profile "$(if $(CANIC_WASM_PROFILE),$(CANIC_WASM_PROFILE),fast)" test

test: clippy test-unit

# Fast iteration path for wasm work.
# Skips integration tests under `tests/`, which is where the PocketIC-heavy
# suites live today.
test-wasm: clippy test-unit-fast

# Version-bump gate.
# Keeps clippy plus the fast unit/lib/bin workspace run, while leaving the
# local ICP CLI fast path as an explicit manual target.
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

test-canisters: test-fleet-install
	test_pid="$$(TMPDIR="$(TEST_TMPDIR)" icp canister -n "$(ICP_ENVIRONMENT)" call root canic_subnet_registry --output json | jq -er '.Ok[] | select(.role == "test") | .pid' | head -n1)"; \
	TMPDIR="$(TEST_TMPDIR)" icp canister -n "$(ICP_ENVIRONMENT)" call "$$test_pid" test

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
