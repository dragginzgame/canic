.PHONY: help version tags patch patch-fast minor major \
        release-patch release-patch-fast release-minor release-major \
        release-stage release-candidate release-commit release-push release-clean release-cadence package publish \
        test-packaged-downstream-wasm-store \
        test-packaged-downstream-cli test-installed-canic-cli \
        test test-wasm validate build check clippy fmt fmt-check clean clean-wasm \
        audit-method-catalog-gate check-invariants ci-checks ci-preflight ci-security \
        control-plane-feature-gate \
        current-document-semantics-gate dependency-risk-inventory-test layering-gate \
        lint-workflows recovery-runbooks-gate release-integrity-contract-gate \
        release-validation-matrix-gate validation-runner-gate \
        wasm-capability-size-report-gate wasm-crypto-closure-gate \
        dependency-risk-gate shellcheck \
        install install-dev install-hooks update-dev \
        ensure-clean test-unit test-unit-fast test-ordinary test-pocketic test-pocketic-case workspace-test-inventory-gate \
        test-auth test-auth-chain-key test-cli test-runtime-fast \
        cloc

.DEFAULT_GOAL := help

CARGO_INSTALL_BIN_DIR ?= $(if $(CARGO_HOME),$(CARGO_HOME),$(HOME)/.cargo)/bin
include tool-versions.env
include ci/tool-versions.env
override SHARED_TOOLING_ROOT := $(CURDIR)
IC_TOOLS := ic-wasm icp wasm-opt
CLOC_REPORT := $(CURDIR)/scripts/dev/report-cloc.sh
LOCAL_TOOL_INSTALL_TARGETS += install-ic-tools install-candid-extractor install-testkit-server
LOCAL_TOOL_CHECK_TARGETS += ic-tools-check candid-extractor-check testkit-server-check
LOCAL_TOOL_INSTALL_TARGETS += install-release-metadata-editor
LOCAL_TOOL_CHECK_TARGETS += release-metadata-editor-check
FORMAT_RUST_ARGS = --sort-derives \
    $(foreach workspace,$(CANIC_BLOB_WORKSPACES),--extra-workspace $(workspace)) \
    --extra-manifest-order workspace,package,lib,dependencies,dev-dependencies,build-dependencies,profile
include make/consumer.mk

.PHONY: install-candid-extractor candid-extractor-check install-testkit-server testkit-server-check
install-candid-extractor:
	@bash scripts/dev/candid-extractor.sh

.PHONY: install-release-metadata-editor release-metadata-editor-check
install-release-metadata-editor:
	@bash scripts/dev/release-metadata-editor.sh

release-metadata-editor-check:
	@bash scripts/dev/release-metadata-editor.sh --check

candid-extractor-check:
	@bash scripts/dev/candid-extractor.sh --check

install-testkit-server:
	@runner="$$(bash scripts/ci/testkit-server.sh)" && "$$runner" setup

testkit-server-check:
	@runner="$$(bash scripts/ci/testkit-server.sh --check)" && "$$runner" check
ACTIONLINT_INSTALL_DIR ?= $(HOME)/.local/bin
SHELLCHECK_INSTALL_DIR ?= $(HOME)/.local/bin
ACTIONLINT_BIN ?= $(ACTIONLINT_INSTALL_DIR)/actionlint
SHELLCHECK_BIN ?= $(SHELLCHECK_INSTALL_DIR)/shellcheck

ICP_ENVIRONMENT ?= local
export ICP_ENVIRONMENT
CARGO_ENV := ICP_ENVIRONMENT=$(ICP_ENVIRONMENT)
CANIC_BLOB_WORKSPACES := integrations/blob-service/consumer \
                         integrations/blob-service/embedded-consumer
CANIC_CARGO_TARGET_DIR ?= $(CURDIR)/target/cargo
CARGO_TARGET_DIR ?= $(CANIC_CARGO_TARGET_DIR)
export CARGO_TARGET_DIR
# Linux uses the checksum-pinned archive; other hosts use the existing Rust owner.
ifeq ($(shell uname -s):$(shell uname -m),Linux:x86_64)
CANIC_MANAGED_SCCACHE_BIN := $(CURDIR)/.tools/canic-sccache/bin/sccache
else
CANIC_MANAGED_SCCACHE_BIN := $(shell bash scripts/dev/install-rust-tools.sh --consumer "$(CURDIR)" --package sccache --version "$(SHARED_TOOLING_SCCACHE_VERSION)" --bin sccache --profile release --check 2>/dev/null)
endif
SCCACHE_BIN ?= $(CANIC_MANAGED_SCCACHE_BIN)
CANIC_SCCACHE_WRAPPER := $(CURDIR)/scripts/ci/run-canic-sccache.sh
CANIC_SCCACHE_BIN ?= $(SCCACHE_BIN)
RUSTC_WRAPPER ?= $(CANIC_SCCACHE_WRAPPER)
SCCACHE_RUNTIME_DIR ?= $(CURDIR)/.tmp/sccache-$(SHARED_TOOLING_SCCACHE_VERSION)-runtime
export SCCACHE_RUNTIME_DIR
ifneq ($(filter sccache run-canic-sccache.sh,$(notdir $(RUSTC_WRAPPER))),)
CARGO_INCREMENTAL ?= 0
SCCACHE_CACHE_SIZE ?= 40G
SCCACHE_IDLE_TIMEOUT ?= 7200
export CARGO_INCREMENTAL
export SCCACHE_CACHE_SIZE
export SCCACHE_IDLE_TIMEOUT
endif
ifneq ($(strip $(CANIC_SCCACHE_BIN)),)
export CANIC_SCCACHE_BIN
else
# An unprepared Rust-owner selection must be resolved anew by subsequent Make.
unexport CANIC_SCCACHE_BIN
endif
export RUSTC_WRAPPER
VALIDATION_RUNNER := bash scripts/ci/run-canic-validation-targets.sh
RELEASE_VALIDATION_LANE := bash scripts/ci/run-release-validation-lane.sh

.PHONY: prepare-build-environment check-build-environment
prepare-build-environment:
	@bash scripts/ci/prepare-validation.sh environment

check-build-environment:
	@bash scripts/ci/prepare-validation.sh check-environment

# Check for clean git state
ensure-clean:
	@state="$$(git status --porcelain)" || exit $$?; \
	if [ -n "$$state" ]; then \
		echo "🚨 Working directory not clean! Please commit or stash your changes."; \
		exit 1; \
	fi

# Default target
help:
	@echo "Available commands:"
	@echo ""
	@echo "Setup / Installation:"
	@echo "  install          Install only the local canic CLI binary"
	@echo "  install-dev      Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/ICP CLI/Binaryen/Canic toolchain"
	@echo "  install-testkit-server  Prepare the published Testkit CLI and PocketIC"
	@echo "  testkit-server-check    Verify the prepared Testkit CLI and PocketIC offline"
	@echo "  install-tools    Install pinned repository host, IC and Rust tools"
	@echo "  prepare-validation SCOPE=release|preflight|security|checks  Prepare locked graphs and selected tools"
	@echo "  tools-check      Verify repository tool bytes and versions offline"
	@echo "  install-hooks    Configure the repository formatting-only pre-commit hook"
	@echo "  update-dev       Synchronize reviewed development tools and report Binaryen updates"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Completely validate, then bump patch version files (0.0.x)"
	@echo "  patch-fast       Target-check an eligible non-runtime patch, then bump (0.0.x)"
	@echo "  minor            Validate, then bump minor version files (0.x.0)"
	@echo "  major            Validate, then bump major version files (x.0.0)"
	@echo "  release-patch    Validate and publish a patch release; retain Cargo artifacts"
	@echo "  release-patch-fast  Target-check and publish a non-runtime patch; retain Cargo artifacts"
	@echo "  release-minor    Validate and publish a minor release; retain Cargo artifacts"
	@echo "  release-major    Validate and publish a major release; retain Cargo artifacts"
	@echo "  release-stage    Stage release version files after review"
	@echo "  release-candidate Verify post-bump package-version and lock consistency"
	@echo "  release-commit   Commit and tag the staged release"
	@echo "  release-push     Atomically push the verified release commit and tag"
	@echo "  release-clean    Explicitly clean Cargo artifacts after release and package publication"
	@echo "  release-cadence  Report the current minor's advisory release-batch count"
	@echo "  package          Build a publishable crate tarball"
	@echo "  publish          Publish workspace crates to registry in dependency order"
	@echo "  test-packaged-downstream-wasm-store  Verify packaged Canister, testing facade and wasm_store paths"
	@echo "  test-packaged-downstream-cli  Verify packaged CLI/support crates from an isolated downstream root"
	@echo "  test-installed-canic-cli  Verify the installed canic binary with the v1 readiness smoke"
	@echo ""
	@echo "Development:"
	@echo "  test             Run workspace tests (PocketIC/Cargo only)"
	@echo "  test-wasm        Run only the classified fast release-surface integrations"
	@echo "  test-pocketic-case CASE=...  Run one exact PocketIC test or integration target"
	@echo "  test-auth        Run focused delegated-auth, role-attestation, and protocol auth gates"
	@echo "  test-auth-chain-key  Run focused chain-key batch renewal gates"
	@echo "  test-cli         Run focused CLI and public surface tests"
	@echo "  test-runtime-fast  Run the same fast release-surface integration lane"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  check-invariants Run repository structure and governance invariants"
	@echo "  clippy           Run clippy checks"
	@echo "  validate         Run staged complete-feedback formatting, policy, compile, and test gates"
	@echo "  fmt              Format code"
	@echo "  fmt-check        Check formatting"
	@echo "  clean            Clean Cargo artifacts; each test invocation cleans its own scratch"
	@echo "  clean-wasm       Clean only transient Canic/PocketIC Wasm build caches"
	@echo "  shellcheck        Lint repository shell automation with pinned ShellCheck"
	@echo "  dependency-risk-gate  Reject vulnerability or transitive advisory drift"
	@echo ""
	@echo "Utilities:"
	@echo "  cloc             Show runtime vs test Rust LOC across Cargo workspace members"
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

# Install the shared Rust/Cargo/ripgrep/ShellCheck/actionlint/ICP CLI/Binaryen/Canic toolchain.
install-dev:
	ACTIONLINT_INSTALL_DIR="$(ACTIONLINT_INSTALL_DIR)" SHELLCHECK_INSTALL_DIR="$(SHELLCHECK_INSTALL_DIR)" \
		bash scripts/dev/install_dev.sh

.PHONY: dependency-pins-gate
dependency-pins-gate:
	bash scripts/ci/check-dependency-pins.sh --cargo-inheritance

# Synchronize reviewed pins and report Binaryen drift without rewriting the snapshot.
update-dev:
	bash scripts/dev/check-binaryen-update.sh
	ACTIONLINT_INSTALL_DIR="$(ACTIONLINT_INSTALL_DIR)" SHELLCHECK_INSTALL_DIR="$(SHELLCHECK_INSTALL_DIR)" \
		bash scripts/dev/install_dev.sh --update-prereqs
	cargo install --quiet \
		"cargo-audit@$(CANIC_CARGO_AUDIT_VERSION)" \
		"cargo-bloat@$(CANIC_CARGO_BLOAT_VERSION)" \
		"cargo-deny@$(CANIC_CARGO_DENY_VERSION)" \
		"cargo-expand@$(CANIC_CARGO_EXPAND_VERSION)" \
		"cargo-machete@$(CANIC_CARGO_MACHETE_VERSION)" \
		"cargo-llvm-lines@$(CANIC_CARGO_LLVM_LINES_VERSION)" \
		"cargo-tarpaulin@$(CANIC_CARGO_TARPAULIN_VERSION)" \
		"sccache@$(SHARED_TOOLING_SCCACHE_VERSION)" \
		--locked
	+$(MAKE) --no-print-directory install-rust-tools
	"$(CURDIR)/.tools/host/bin/rg" --version
	"$(CURDIR)/.tools/host/bin/rg" --pcre2-version
	"$(CARGO_INSTALL_BIN_DIR)/sccache" --version
	"$(CURDIR)/.tools/ic/bin/icp" --version
	"$(CURDIR)/.tools/ic/bin/ic-wasm" --version
	"$(CURDIR)/.tools/ic/bin/wasm-opt" --version
	+$(MAKE) --no-print-directory prepare-locked-dependencies
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

release-patch-fast:
	@$(MAKE) patch-fast
	@$(MAKE) release-stage
	@$(MAKE) release-commit
	@$(MAKE) release-push

release-stage:
	@bash scripts/release/stage.sh

release-candidate:
	bash scripts/ci/check-release-candidate.sh

release-commit:
	@scripts/ci/check-release-index.sh
	@$(MAKE) --no-print-directory release-candidate
	@bash scripts/ci/commit-release.sh

release-push:
	@bash scripts/ci/check-release-push-ready.sh
	@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release.sh

release-clean:
	@if ! bash scripts/ci/cleanup-release-artifacts.sh; then \
		echo "warning: Cargo cleanup failed; retry with 'make clean' when convenient" >&2; \
	fi

release-cadence:
	@bash scripts/dev/report-release-cadence.sh

package: ensure-clean
	+@$(MAKE) --no-print-directory clippy
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

# One membership definition per qualification group. Local and CI scheduling
# differ deliberately: local Clippy first; hosted cheap jobs gate expensive jobs.
CANIC_QUALIFICATION_LINT := clippy
CANIC_QUALIFICATION_FORMAT := fmt-check
CANIC_QUALIFICATION_POLICY := check-invariants shellcheck
CANIC_QUALIFICATION_SECURITY := dependency-risk-gate dependency-risk-inventory-test
CANIC_QUALIFICATION_FEATURES := control-plane-feature-gate

.PHONY: prepare-locked-dependencies prepare-validation
prepare-locked-dependencies:
	@for manifest in Cargo.toml $(addsuffix /Cargo.toml,$(CANIC_BLOB_WORKSPACES)); do \
		cargo fetch --locked --manifest-path "$$manifest" || exit "$$?"; done

prepare-validation:
	+bash scripts/ci/prepare-validation.sh "$(SCOPE)"

# Preparation is explicit; qualification contains no dependency-setup recipe.
# Every barrier remains sequential even under parallel Make.
validate:
	+@$(MAKE) --no-print-directory tools-check
	+@$(VALIDATION_RUNNER) dependency-adoption-admission
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_LINT)
	+@$(VALIDATION_RUNNER) dependency-adoption-fixtures
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_FORMAT) $(CANIC_QUALIFICATION_POLICY) $(CANIC_QUALIFICATION_SECURITY)
	+@$(VALIDATION_RUNNER) check
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_FEATURES)
	+@$(VALIDATION_RUNNER) test

CANIC_INVARIANT_TARGETS := \
		cargo-dependency-boundaries-gate \
		runtime-dependency-graph-gate \
		dependency-pins-gate \
		layering-gate \
		current-document-semantics-gate \
		release-validation-matrix-gate \
		release-integrity-contract-gate \
		recovery-runbooks-gate \
		validation-runner-gate \
		wasm-capability-size-report-gate \
		wasm-crypto-closure-gate

check-invariants:
	+@$(VALIDATION_RUNNER) $(CANIC_INVARIANT_TARGETS)

# CI keeps tool installation as an immediate prerequisite, then collects every
# independent preflight or security failure before gating expensive jobs.
ci-preflight:
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_POLICY) lint-workflows workspace-test-inventory-gate

ci-checks:
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_FORMAT) $(CANIC_QUALIFICATION_LINT)
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_FEATURES)

ci-security:
	+@$(VALIDATION_RUNNER) $(CANIC_QUALIFICATION_SECURITY)

audit-method-catalog-gate:
	bash scripts/ci/check-audit-method-catalog.sh
	bash scripts/ci/test-document-guards.sh

current-document-semantics-gate:
	bash scripts/ci/check-current-document-semantics.sh

dependency-risk-inventory-test:
	bash scripts/ci/test-dependency-risk-inventory.sh

layering-gate:
	bash scripts/ci/run-layering-guards.sh

lint-workflows:
	"$(ACTIONLINT_BIN)"

recovery-runbooks-gate:
	bash scripts/ci/check-recovery-runbooks.sh

release-integrity-contract-gate:
	bash scripts/ci/check-shared-governance.sh
	bash scripts/ci/check-release-integrity-contract.sh
	bash scripts/ci/test-require-jq.sh
	bash scripts/ci/test-dev-tool-recipes.sh
	bash scripts/ci/test-release-tools.sh
	bash scripts/ci/test-pinned-ci-tools.sh
	bash scripts/ci/test-build-environment.sh
	bash scripts/ci/test-shared-release-adoption.sh
	bash scripts/ci/test-canic-formatters.sh
	bash scripts/ci/test-dependency-adoption.sh
	bash scripts/ci/test-release-commit-view.sh
	bash scripts/ci/test-commit-release.sh
	bash scripts/ci/test-release-recipes.sh
	bash scripts/release/test-stage.sh
	MAKEFLAGS='-- RELEASE_PREVIOUS=9.9.9 RELEASE_KIND=minor RELEASE_VERSION=9.10.0' bash scripts/release/test-preflight.sh
	bash scripts/ci/test-release-validation-lane.sh
	bash scripts/ci/check-publish-manifest-boundary.sh
	bash scripts/ci/test-publish-manifest-boundary.sh
	bash scripts/ci/test-packaged-output-paths.sh
	bash scripts/ci/test-publish-workspace.sh

release-validation-matrix-gate:
	bash scripts/ci/check-release-validation-matrix.sh

validation-runner-gate:
	bash scripts/ci/test-sccache-wrapper.sh
	bash scripts/ci/test-canic-validation-targets.sh
	bash scripts/ci/test-workspace-cargo.sh
	bash scripts/ci/test-workspace-test-runner.sh
	bash scripts/ci/test-pocketic-workers.sh
	bash scripts/ci/test-native-icp.sh

wasm-capability-size-report-gate:
	bash scripts/ci/test-wasm-capability-size-report.sh

wasm-crypto-closure-gate:
	bash scripts/ci/test-wasm-crypto-closure.sh
	bash scripts/ci/check-wasm-crypto-closure.sh

dependency-risk-gate:
	bash scripts/ci/check-dependency-risk-inventory.sh

shellcheck:
	"$(SHELLCHECK_BIN)" --shell=bash --exclude=SC2001,SC2016 \
		scripts/ci/*.sh scripts/dev/*.sh .githooks/pre-commit tool-versions.env ci/tool-versions.env

control-plane-feature-gate:
	bash scripts/ci/check-control-plane-feature-matrix.sh

# Temporary product guards: remove with a promoted standalone blob-service hard cut.

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

# Export the selector as data rather than interpolating it into shell source.
test-pocketic-case: export CANIC_POCKETIC_CASE = $(CASE)
test-pocketic-case:
	@test -n "$$CANIC_POCKETIC_CASE" || { echo 'usage: make test-pocketic-case CASE=<exact-test-or-integration-target>' >&2; exit 2; }
	$(CARGO_ENV) bash scripts/ci/run-with-test-scratch.sh \
		bash scripts/ci/run-workspace-tests.sh targeted-pocketic "$$CANIC_POCKETIC_CASE"

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
	$(CARGO_ENV) bash scripts/ci/run-workspace-cargo.sh build --release --keep-going

check:
	$(CARGO_ENV) bash scripts/ci/run-workspace-cargo.sh check --keep-going

clippy:
	$(CARGO_ENV) bash scripts/ci/run-workspace-cargo.sh clippy -D warnings

clean:
	@bash scripts/ci/cleanup-release-artifacts.sh

clean-wasm:
	rm -rf -- target/test-artifacts
	rm -rf -- target/canic-wasm
	rm -rf -- target/canic-packages
	rm -rf -- target/icp-build
	rm -rf -- target/pic-wasm
	rm -rf -- target/pic-runtime-wasm
	rm -rf -- target/pic-wasm-no-test-material
	rm -rf -- target/fleet-coordinator
	rm -rf -- target/fleet-registry-sync
	rm -rf -- target/standalone-leaf_probe
	rm -rf -- target/standalone-payload_limit_probe
	rm -rf -- target/standalone-root-probe
	rm -rf -- target/standalone-scaling_probe

# Shared Tooling owns the standard release order and Git effects.
RELEASE_REMOTE ?= origin
RELEASE_BRANCH ?= main
ifneq ($(word 2,$(filter release-patch release-minor release-major release-resume,$(MAKECMDGOALS))),)
$(error Select exactly one release target)
endif
.PHONY: release-resume release-version release-preflight release-prepare-version release-prepared-check release-files release-commit-check release-committed-check release-tagged-check release-push-check

.PHONY: release-verify
release-version:
	@bash scripts/ci/read-workspace-version.sh
release-preflight:
	@bash scripts/release/adapter.sh preflight
release-verify:
	+CARGO_NET_OFFLINE=true $(RELEASE_VALIDATION_LANE) complete
release-prepare-version:
	@CANIC_RELEASE_VALIDATED=1 CANIC_RELEASE_VALIDATED_HEAD="$(RELEASE_SOURCE)" CANIC_RELEASE_VALIDATION_KIND=complete CANIC_RELEASE_DATE="$(RELEASE_DATE)" bash scripts/ci/bump-version.sh "$(RELEASE_KIND)"
release-prepared-check:
	@bash scripts/ci/check-release-candidate.sh
release-committed-check:
	@bash scripts/release/adapter.sh committed
release-tagged-check:
	@bash scripts/release/adapter.sh tagged
release-files:
	@bash scripts/release/adapter.sh files
release-commit-check:
	@bash scripts/ci/check-release-index.sh
	@bash scripts/ci/check-release-candidate.sh
release-push-check:
	@bash scripts/release/adapter.sh tagged

.PHONY: cargo-dependency-boundaries-gate
cargo-dependency-boundaries-gate:
	@bash scripts/ci/check-cargo-dependency-boundaries.sh --consumer "$(CURDIR)" --external-ic
	@bash scripts/ci/check-cargo-dependency-boundaries.sh --consumer "$(CURDIR)/integrations/blob-service/consumer" --external-ic
	@bash scripts/ci/check-cargo-dependency-boundaries.sh --consumer "$(CURDIR)/integrations/blob-service/embedded-consumer" --external-ic

# Canic owns the runtime roots and single-provider policy; Shared observes them.
.PHONY: runtime-dependency-graph-gate
runtime-dependency-graph-gate:
	@perl scripts/ci/check-cargo-graph.pl --consumer "$(CURDIR)" \
		--package canic --package canic-core --package canic-control-plane --package canic-macros \
		--target wasm32-unknown-unknown --all-features \
		--require ic-memory=1 --require ic-timers=1 --forbid ic-cdk-timers \
		--forbid canic-blob-service --forbid canic-cli --forbid canic-testing-internal \
		--forbid ic-testkit

# Release validation reuses adoption phases without repeating workspace Clippy.
.PHONY: dependency-adoption-admission dependency-adoption-fixtures
dependency-adoption-admission:
	$(CARGO_ENV) bash scripts/dev/check-dependency-adoption.sh --admission-only

dependency-adoption-fixtures:
	$(CARGO_ENV) bash scripts/dev/check-dependency-adoption.sh --fixtures-only
