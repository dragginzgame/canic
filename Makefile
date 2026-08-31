.PHONY: help version tags patch patch-fast minor major \
        release-patch release-patch-fast release-minor release-major \
        release-stage release-candidate release-commit release-push release-clean release-cadence package publish \
        test-packaged-downstream-wasm-store \
        test-packaged-downstream-cli test-installed-canic-cli \
        test test-wasm validate build check clippy fmt fmt-check clean clean-wasm \
        blob-storage-inventory-gate blob-storage-cashier-inventory-gate \
        audit-method-catalog-gate check-invariants ci-checks ci-preflight ci-security \
        control-plane-feature-gate \
        current-document-semantics-gate dependency-risk-inventory-test layering-gate \
        lint-workflows recovery-runbooks-gate release-integrity-contract-gate \
        release-validation-matrix-gate validation-runner-gate \
        wasm-capability-size-report-gate \
        dependency-risk-gate gitleaks-scan shellcheck \
        install install-dev install-hooks update-dev \
        ensure-clean test-unit test-unit-fast test-ordinary test-pocketic workspace-test-inventory-gate \
        test-auth test-auth-chain-key test-cli test-runtime-fast \
        cloc

CARGO_INSTALL_BIN_DIR ?= $(if $(CARGO_HOME),$(CARGO_HOME),$(HOME)/.cargo)/bin
include tool-versions.env
ACTIONLINT_INSTALL_DIR ?= $(HOME)/.local/bin
SHELLCHECK_INSTALL_DIR ?= $(HOME)/.local/bin
GITLEAKS_INSTALL_DIR ?= $(HOME)/.local/bin
ACTIONLINT_BIN ?= $(ACTIONLINT_INSTALL_DIR)/actionlint
SHELLCHECK_BIN ?= $(SHELLCHECK_INSTALL_DIR)/shellcheck
GITLEAKS_BIN ?= $(GITLEAKS_INSTALL_DIR)/gitleaks

ICP_ENVIRONMENT ?= local
export ICP_ENVIRONMENT
CARGO_ENV := ICP_ENVIRONMENT=$(ICP_ENVIRONMENT)
CANIC_CARGO_TARGET_DIR ?= $(CURDIR)/target
CARGO_TARGET_DIR ?= $(CANIC_CARGO_TARGET_DIR)
export CARGO_TARGET_DIR
SCCACHE_BIN ?= $(shell command -v sccache 2>/dev/null)
CANIC_SCCACHE_WRAPPER := $(CURDIR)/scripts/ci/run-sccache.sh
ifeq ($(origin RUSTC_WRAPPER), undefined)
ifneq ($(strip $(SCCACHE_BIN)),)
CANIC_SCCACHE_BIN ?= $(SCCACHE_BIN)
RUSTC_WRAPPER ?= $(CANIC_SCCACHE_WRAPPER)
endif
endif
ifneq ($(filter sccache run-sccache.sh,$(notdir $(RUSTC_WRAPPER))),)
CARGO_INCREMENTAL ?= 0
SCCACHE_CACHE_SIZE ?= 40G
SCCACHE_IDLE_TIMEOUT ?= 7200
export CARGO_INCREMENTAL
export SCCACHE_CACHE_SIZE
export SCCACHE_IDLE_TIMEOUT
endif
export CANIC_SCCACHE_BIN
export RUSTC_WRAPPER
VALIDATION_RUNNER := bash scripts/ci/run-validation-targets.sh
RELEASE_VALIDATION_LANE := bash scripts/ci/run-release-validation-lane.sh

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
	@echo "  install-dev      Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/Gitleaks/ICP CLI/Binaryen/Canic toolchain"
	@echo "  install-hooks    Configure the repository formatting-only pre-commit hook"
	@echo "  update-dev       Pin the latest stable ICP CLI, report Binaryen updates, and synchronize development tools"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Completely validate, then bump patch version files (0.0.x)"
	@echo "  patch-fast       Target-check an eligible non-runtime patch, then bump (0.0.x)"
	@echo "  minor            Validate, then bump minor version files (0.x.0)"
	@echo "  major            Validate, then bump major version files (x.0.0)"
	@echo "  release-patch    Validate, publish a patch release, then clean Cargo artifacts"
	@echo "  release-patch-fast  Target-check, publish a non-runtime patch, then clean Cargo artifacts"
	@echo "  release-minor    Validate, publish a minor release, then clean Cargo artifacts"
	@echo "  release-major    Validate, publish a major release, then clean Cargo artifacts"
	@echo "  release-stage    Stage release version files after review"
	@echo "  release-candidate Verify post-bump package-version and lock consistency"
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
	@echo "  validate         Run staged complete-feedback formatting, policy, compile, and test gates"
	@echo "  fmt              Format code"
	@echo "  fmt-check        Check formatting"
	@echo "  clean            Clean Cargo artifacts; each test invocation cleans its own scratch"
	@echo "  clean-wasm       Clean only transient Canic/PocketIC Wasm build caches"
	@echo "  gitleaks-scan     Scan complete repository history with pinned Gitleaks"
	@echo "  shellcheck        Lint repository shell automation with pinned ShellCheck"
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

# Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/Gitleaks/ICP CLI/Binaryen/Canic toolchain.
install-dev:
	ACTIONLINT_INSTALL_DIR="$(ACTIONLINT_INSTALL_DIR)" SHELLCHECK_INSTALL_DIR="$(SHELLCHECK_INSTALL_DIR)" GITLEAKS_INSTALL_DIR="$(GITLEAKS_INSTALL_DIR)" bash scripts/dev/install_dev.sh

# Configure the one repository-owned hook without installing the full toolchain.
install-hooks:
	bash scripts/dev/install-git-hooks.sh

# Pin the latest stable ICP CLI, report Binaryen drift, then synchronize tools.
update-dev:
	bash scripts/dev/update-icp-cli-pin.sh
	bash scripts/dev/check-binaryen-update.sh
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
		"sccache@$(CANIC_SCCACHE_VERSION)" \
		--locked
	bash scripts/dev/install_dev.sh --ensure-ripgrep
	"$(CARGO_INSTALL_BIN_DIR)/rg" --version
	"$(CARGO_INSTALL_BIN_DIR)/rg" --pcre2-version
	"$(CARGO_INSTALL_BIN_DIR)/sccache" --version
	icp --version
	ic-wasm --version
	wasm-opt --version
	"$(GITLEAKS_INSTALL_DIR)/gitleaks" version
	bash scripts/ci/check-dependency-risk-inventory.sh

#
# Version management (validate the source candidate before mutation)
#

version:
	@bash scripts/ci/read-workspace-version.sh

tags:
	@git tag --sort=-version:refname | head -10

patch:
	@$(MAKE) --no-print-directory release-cadence
	+@$(RELEASE_VALIDATION_LANE) complete patch

patch-fast:
	@$(MAKE) --no-print-directory release-cadence
	+@$(RELEASE_VALIDATION_LANE) fast patch

minor:
	+@$(RELEASE_VALIDATION_LANE) complete minor

major:
	+@$(RELEASE_VALIDATION_LANE) complete major

release-patch:
	@$(MAKE) patch
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push
	@$(MAKE) release-clean

release-patch-fast:
	@$(MAKE) patch-fast
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push
	@$(MAKE) release-clean

release-minor:
	@$(MAKE) minor
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push
	@$(MAKE) release-clean

release-major:
	@$(MAKE) major
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push
	@$(MAKE) release-clean

release-stage:
	@version="$$(bash scripts/ci/read-workspace-version.sh)"; \
		minor_line="$${version%.*}"; \
		git add Cargo.toml Cargo.lock scripts/dev/install_dev.sh \
			scripts/ci/sync-release-surface-version.sh docs/status/current.md \
			"docs/changelog/$$minor_line.md" \
			$$(git ls-files -m -- '*/Cargo.toml' || true)

release-candidate:
	bash scripts/ci/check-release-candidate.sh

release-commit:
	@scripts/ci/check-release-index.sh
	@$(MAKE) --no-print-directory release-candidate
	@version="$$(bash scripts/ci/read-workspace-version.sh)"; \
	if git rev-parse "v$$version" >/dev/null 2>&1; then \
		echo "❌ Tag v$$version already exists. Aborting." >&2; \
		exit 1; \
	fi; \
	git commit -m "Release $$version"; \
	git tag -a "v$$version" -m "Release $$version"

release-push:
	@bash scripts/ci/check-release-push-ready.sh
	@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release.sh

release-clean:
	@if ! bash scripts/ci/cleanup-release-artifacts.sh; then \
		echo "warning: release push succeeded, but Cargo cleanup failed; run 'make clean' without rerunning the release" >&2; \
	fi

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

test: test-unit

# Fast iteration path for wasm work.
# Runs only the three classified fast integration targets and skips every
# PocketIC suite.
test-wasm: test-unit-fast

# Complete local validation has three sequential barriers. Each barrier collects
# every independent target failure before returning. Compile/lint work starts
# only after cheap source, policy, and security checks pass, and the complete
# test graph starts only after compile and warning-denied Clippy pass.
# Primitive development targets retain only the operation named by that target.
validate:
	+@$(VALIDATION_RUNNER) \
		fmt-check \
		check-invariants \
		dependency-risk-gate \
		gitleaks-scan \
		shellcheck \
		control-plane-feature-gate
	+@$(VALIDATION_RUNNER) \
		check \
		clippy
	+@$(VALIDATION_RUNNER) \
		test

check-invariants:
	+@$(VALIDATION_RUNNER) \
		layering-gate \
		current-document-semantics-gate \
		blob-storage-inventory-gate \
		blob-storage-cashier-inventory-gate \
		dependency-risk-inventory-test \
		release-validation-matrix-gate \
		release-integrity-contract-gate \
		audit-method-catalog-gate \
		recovery-runbooks-gate \
		validation-runner-gate \
		wasm-capability-size-report-gate

# CI keeps tool installation as an immediate prerequisite, then collects every
# independent preflight or security failure before gating expensive jobs.
ci-preflight:
	+@$(VALIDATION_RUNNER) \
		lint-workflows \
		shellcheck \
		layering-gate \
		current-document-semantics-gate \
		blob-storage-inventory-gate \
		blob-storage-cashier-inventory-gate \
		release-validation-matrix-gate \
		release-integrity-contract-gate \
		audit-method-catalog-gate \
		recovery-runbooks-gate \
		workspace-test-inventory-gate \
		wasm-capability-size-report-gate

ci-checks:
	+@$(VALIDATION_RUNNER) \
		control-plane-feature-gate \
		fmt-check \
		clippy

ci-security:
	+@$(VALIDATION_RUNNER) \
		gitleaks-scan \
		dependency-risk-gate \
		dependency-risk-inventory-test

audit-method-catalog-gate:
	bash scripts/ci/check-audit-method-catalog.sh

current-document-semantics-gate:
	bash scripts/ci/check-current-document-semantics.sh

dependency-risk-inventory-test:
	bash scripts/ci/test-dependency-risk-inventory.sh

layering-gate:
	bash scripts/ci/run-layering-guards.sh
	bash scripts/ci/check-pre-1-0-hard-cut.sh

lint-workflows:
	"$(ACTIONLINT_BIN)"

recovery-runbooks-gate:
	bash scripts/ci/check-recovery-runbooks.sh

release-integrity-contract-gate:
	bash scripts/ci/check-release-integrity-contract.sh

release-validation-matrix-gate:
	bash scripts/ci/check-release-validation-matrix.sh

validation-runner-gate:
	bash scripts/ci/test-validation-target-runner.sh

wasm-capability-size-report-gate:
	bash scripts/ci/test-wasm-capability-size-report.sh

dependency-risk-gate:
	bash scripts/ci/check-dependency-risk-inventory.sh

gitleaks-scan:
	GITLEAKS_BIN="$(GITLEAKS_BIN)" bash scripts/ci/run-secret-scan.sh

shellcheck:
	"$(SHELLCHECK_BIN)" --exclude=SC2001,SC2016 \
		scripts/ci/*.sh scripts/dev/*.sh .githooks/pre-commit

control-plane-feature-gate:
	bash scripts/ci/check-control-plane-feature-matrix.sh

# Temporary product guards: remove with a promoted standalone blob-service hard cut.
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
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh full

test-ordinary:
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh ordinary

test-pocketic:
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh pocketic

test-unit-fast:
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
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

#
# Development commands
#

build:
	$(CARGO_ENV) cargo build --locked --workspace --release --keep-going

check:
	$(CARGO_ENV) cargo check --locked --workspace --keep-going

clippy:
	$(CARGO_ENV) cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

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
