.PHONY: help version tags patch minor major \
        release-patch release-minor release-major \
        release-stage release-commit release-push release-cadence package publish \
        test-packaged-downstream-wasm-store \
        test-packaged-downstream-cli test-installed-canic-cli \
        test test-wasm validate build check clippy fmt fmt-check clean clean-wasm \
        blob-storage-inventory-gate blob-storage-cashier-inventory-gate \
        check-invariants control-plane-feature-gate \
        dependency-risk-gate gitleaks-scan \
        install install-dev install-hooks update-dev test-fleet-install \
        ensure-clean test-unit test-unit-fast workspace-test-inventory-gate \
        test-auth test-auth-chain-key test-cli test-runtime-fast \
        test-canisters cloc

CARGO_INSTALL_BIN_DIR ?= $(if $(CARGO_HOME),$(CARGO_HOME),$(HOME)/.cargo)/bin
include tool-versions.env
ACTIONLINT_INSTALL_DIR ?= $(HOME)/.local/bin
SHELLCHECK_INSTALL_DIR ?= $(HOME)/.local/bin
GITLEAKS_INSTALL_DIR ?= $(HOME)/.local/bin

ICP_ENVIRONMENT ?= local
export ICP_ENVIRONMENT
CARGO_ENV := ICP_ENVIRONMENT=$(ICP_ENVIRONMENT)

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
	@echo "  install-dev      Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/Gitleaks/ICP CLI/Canic toolchain"
	@echo "  install-hooks    Configure the repository formatting-only pre-commit hook"
	@echo "  update-dev       Pin the latest stable ICP CLI and synchronize development tools"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Validate, then bump patch version files (0.0.x)"
	@echo "  minor            Confirm, validate, then bump minor version files (0.x.0)"
	@echo "  major            Confirm, validate, then bump major version files (x.0.0)"
	@echo "  release-patch    Bump, stage, commit, tag, and push a patch release"
	@echo "  release-minor    Confirm, bump, stage, commit, tag, and push a minor release"
	@echo "  release-major    Confirm, bump, stage, commit, tag, and push a major release"
	@echo "  release-stage    Stage release version files after review"
	@echo "  release-commit   Commit and tag the staged release"
	@echo "  release-push     Atomically push the verified release commit and tag"
	@echo "  release-cadence  Report the current minor's advisory release-batch count"
	@echo "  package          Build a publishable crate tarball"
	@echo "  publish          Publish workspace crates to registry in dependency order"
	@echo "  test-packaged-downstream-wasm-store  Verify packaged Canister macros and wasm_store bootstrap paths"
	@echo "  test-packaged-downstream-cli  Verify packaged CLI/support crates from an isolated downstream root"
	@echo "  test-installed-canic-cli  Verify the installed canic binary with the v1 readiness smoke"
	@echo ""
	@echo "Development:"
	@echo "  test-fleet-install  Install the full local test/reference topology with fast wasm by default"
	@echo "  test             Run workspace tests (PocketIC/Cargo only)"
	@echo "  test-wasm        Run fast non-PocketIC tests for wasm iteration"
	@echo "  test-auth        Run focused delegated-auth, role-attestation, and protocol auth gates"
	@echo "  test-auth-chain-key  Run focused chain-key batch renewal gates"
	@echo "  test-cli         Run focused CLI and public surface tests"
	@echo "  test-runtime-fast  Run the fast deterministic runtime test lane"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  check-invariants Run repository structure and governance invariants"
	@echo "  clippy           Run clippy checks"
	@echo "  validate         Run formatting, invariant, feature, check, clippy, and test gates"
	@echo "  fmt              Format code"
	@echo "  fmt-check        Check formatting"
	@echo "  clean            Clean Cargo artifacts; each test invocation cleans its own scratch"
	@echo "  clean-wasm       Clean only transient Canic/PocketIC Wasm build caches"
	@echo "  gitleaks-scan     Scan complete repository history with pinned Gitleaks"
	@echo "  dependency-risk-gate  Reject vulnerability or transitive advisory drift"
	@echo ""
	@echo "Utilities:"
	@echo "  cloc             Show runtime vs test Rust LOC across canic crates"
	@echo ""
	@echo "Examples:"
	@echo "  make validate    # Run the complete local validation workflow"
	@echo "  make test        # Run workspace tests"
	@echo "  make release-patch # Validate, bump, commit, tag, and push a patch release"

#
# Installing
#

# Install only the local canic CLI binary.
install:
	cargo install --locked --path crates/canic-cli

# Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/Gitleaks/ICP CLI/Canic toolchain.
install-dev:
	ACTIONLINT_INSTALL_DIR="$(ACTIONLINT_INSTALL_DIR)" SHELLCHECK_INSTALL_DIR="$(SHELLCHECK_INSTALL_DIR)" GITLEAKS_INSTALL_DIR="$(GITLEAKS_INSTALL_DIR)" bash scripts/dev/install_dev.sh

# Configure the one repository-owned hook without installing the full toolchain.
install-hooks:
	bash scripts/dev/install-git-hooks.sh

# Pin the latest stable ICP CLI, then synchronize local development tools.
update-dev:
	bash scripts/dev/update-icp-cli-pin.sh
	ACTIONLINT_INSTALL_DIR="$(ACTIONLINT_INSTALL_DIR)" SHELLCHECK_INSTALL_DIR="$(SHELLCHECK_INSTALL_DIR)" GITLEAKS_INSTALL_DIR="$(GITLEAKS_INSTALL_DIR)" bash scripts/dev/install_dev.sh --update-prereqs
	cargo install --quiet \
		"cargo-audit@$(CANIC_CARGO_AUDIT_VERSION)" \
		"cargo-bloat@$(CANIC_CARGO_BLOAT_VERSION)" \
		"cargo-deny@$(CANIC_CARGO_DENY_VERSION)" \
		"cargo-expand@$(CANIC_CARGO_EXPAND_VERSION)" \
		"cargo-machete@$(CANIC_CARGO_MACHETE_VERSION)" \
		"cargo-llvm-lines@$(CANIC_CARGO_LLVM_LINES_VERSION)" \
		"cargo-sort@$(CANIC_CARGO_SORT_VERSION)" \
		"cargo-tarpaulin@$(CANIC_CARGO_TARPAULIN_VERSION)" \
		"cargo-sort-derives@$(CANIC_CARGO_SORT_DERIVES_VERSION)" \
		"candid-extractor@$(CANIC_CANDID_EXTRACTOR_VERSION)" \
		--locked
	bash scripts/dev/install_dev.sh --ensure-ripgrep
	"$(CARGO_INSTALL_BIN_DIR)/rg" --version
	"$(CARGO_INSTALL_BIN_DIR)/rg" --pcre2-version
	icp --version
	ic-wasm --version
	"$(GITLEAKS_INSTALL_DIR)/gitleaks" version
	bash scripts/ci/check-dependency-risk-inventory.sh

#
# Version management (validate the source candidate before mutation)
#

version:
	@cargo get workspace.package.version

tags:
	@git tag --sort=-version:refname | head -10

patch:
	@$(MAKE) --no-print-directory release-cadence
	@$(MAKE) ensure-clean
	+@$(MAKE) --no-print-directory validate
	@$(MAKE) ensure-clean
	@CANIC_RELEASE_VALIDATED=1 scripts/ci/bump-version.sh patch

minor:
	@scripts/ci/confirm-version-bump.sh minor
	@$(MAKE) ensure-clean
	+@$(MAKE) --no-print-directory validate
	@$(MAKE) ensure-clean
	@CANIC_RELEASE_VALIDATED=1 scripts/ci/bump-version.sh minor

major:
	@scripts/ci/confirm-version-bump.sh major
	@$(MAKE) ensure-clean
	+@$(MAKE) --no-print-directory validate
	@$(MAKE) ensure-clean
	@CANIC_RELEASE_VALIDATED=1 scripts/ci/bump-version.sh major

release-patch:
	@$(MAKE) patch
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push

release-minor:
	@$(MAKE) minor
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push

release-major:
	@$(MAKE) major
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push

release-stage:
	git add Cargo.toml Cargo.lock scripts/dev/install_dev.sh \
		scripts/ci/sync-release-surface-version.sh $$(git ls-files -m -- '*/Cargo.toml' || true)

release-commit:
	@scripts/ci/check-release-index.sh
	@version="$$(cargo get workspace.package.version)"; \
	if git rev-parse "v$$version" >/dev/null 2>&1; then \
		echo "❌ Tag v$$version already exists. Aborting." >&2; \
		exit 1; \
	fi; \
	git commit -m "Release $$version"; \
	git tag -a "v$$version" -m "Release $$version"

release-push:
	@bash scripts/ci/check-release-push-ready.sh
	@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release.sh

release-cadence:
	@bash scripts/dev/report-release-cadence.sh

package: ensure-clean
	$(CARGO_ENV) cargo package --locked

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
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/test-fleet-install.sh

test: test-unit

# Fast iteration path for wasm work.
# Runs only the three classified fast integration targets and skips every
# PocketIC suite.
test-wasm: test-unit-fast

# Complete local validation is deliberately explicit. Primitive development
# targets retain only the operation named by that target.
validate:
	+@$(MAKE) --no-print-directory fmt-check
	+@$(MAKE) --no-print-directory check-invariants
	+@$(MAKE) --no-print-directory dependency-risk-gate
	+@$(MAKE) --no-print-directory gitleaks-scan
	+@$(MAKE) --no-print-directory control-plane-feature-gate
	+@$(MAKE) --no-print-directory check
	+@$(MAKE) --no-print-directory clippy
	+@$(MAKE) --no-print-directory test

check-invariants:
	bash scripts/ci/run-layering-guards.sh
	bash scripts/ci/check-current-document-semantics.sh
	# Temporary product guards: remove with a promoted standalone blob-service hard cut.
	+@$(MAKE) --no-print-directory blob-storage-inventory-gate
	+@$(MAKE) --no-print-directory blob-storage-cashier-inventory-gate
	bash scripts/ci/test-dependency-risk-inventory.sh
	bash scripts/ci/check-release-validation-matrix.sh
	bash scripts/ci/check-release-integrity-contract.sh
	bash scripts/ci/check-audit-method-catalog.sh
	bash scripts/ci/check-recovery-runbooks.sh

dependency-risk-gate:
	bash scripts/ci/check-dependency-risk-inventory.sh

gitleaks-scan:
	GITLEAKS_BIN="$(GITLEAKS_INSTALL_DIR)/gitleaks" bash scripts/ci/run-secret-scan.sh

control-plane-feature-gate:
	bash scripts/ci/check-control-plane-feature-matrix.sh

blob-storage-inventory-gate:
	bash scripts/ci/check-blob-storage-inventory-gate.sh

blob-storage-cashier-inventory-gate:
	bash scripts/ci/check-blob-storage-cashier-inventory-gate.sh

# Keep ordinary Rust tests parallel. The workspace runner classifies every
# integration target and serializes only PocketIC suites for deterministic
# fixture reuse. Parallel PocketIC tests can still trigger failures like:
# `KeyAlreadyExists { key: "nns_subnet_id", version: 2 }` and incomplete HTTP messages.
workspace-test-inventory-gate:
	bash scripts/ci/check-workspace-test-inventory.sh

test-unit:
	CARGO_INCREMENTAL=0 $(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh full

test-unit-fast:
	CARGO_INCREMENTAL=0 $(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh fast

test-auth:
	$(CARGO_ENV) cargo test --locked -p canic-core auth --lib
	$(CARGO_ENV) cargo test --locked -p canic-macros
	$(CARGO_ENV) cargo test --locked -p canic --test protocol_surface

test-auth-chain-key:
	$(CARGO_ENV) cargo test --locked -p canic-core chain_key_batch --lib
	$(CARGO_ENV) cargo test --locked -p canic-core chain_key_lazy_repair --lib
	$(CARGO_ENV) cargo test --locked -p canic-core delegated_token_lazy_repair --lib
	$(CARGO_ENV) cargo test --locked -p canic-core workflow::runtime::auth --lib

test-cli:
	$(CARGO_ENV) cargo test --locked -p canic-cli
	$(CARGO_ENV) cargo test --locked -p canic --test install_script_surface
	$(CARGO_ENV) cargo test --locked -p canic --test reference_surface

test-runtime-fast: test-unit-fast

test-canisters: test-fleet-install
	icp canister -e "$(ICP_ENVIRONMENT)" call test test

#
# Development commands
#

build:
	$(CARGO_ENV) cargo build --locked --workspace --release

check:
	$(CARGO_ENV) cargo check --locked --workspace

clippy:
	CARGO_INCREMENTAL=0 $(CARGO_ENV) cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

fmt:
	cargo sort --workspace
	cargo sort-derives
	cargo fmt --all

fmt-check:
	cargo sort --workspace --check
	cargo sort-derives --check
	cargo fmt --all -- --check

clean:
	@bash scripts/ci/cleanup-release-artifacts.sh

clean-wasm:
	rm -rf -- target/test-artifacts
	rm -rf -- target/canic-wasm
	rm -rf -- target/icp-build
	rm -rf -- target/pic-wasm
	rm -rf -- target/pic-runtime-wasm
	rm -rf -- target/pic-wasm-no-test-material
	rm -rf -- target/fleet-coordinator
	rm -rf -- target/fleet-registry-sync
	rm -rf -- target/standalone-blob_storage_cashier_mock
	rm -rf -- target/standalone-blob_storage_probe
	rm -rf -- target/standalone-leaf_probe
	rm -rf -- target/standalone-payload_limit_probe
	rm -rf -- target/standalone-root-probe
	rm -rf -- target/standalone-scaling_probe

cloc:
	bash scripts/dev/cloc.sh
