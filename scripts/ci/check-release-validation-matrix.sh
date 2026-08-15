#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/doc-guard-lib.sh
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release validation"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
PACKAGE_CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
CI_GOVERNANCE="$ROOT/docs/governance/ci-deployment.md"
FLEET_INSTALL="$ROOT/scripts/ci/test-fleet-install.sh"
PACKAGED_CANISTER="$ROOT/scripts/ci/verify-packaged-downstream-wasm-store.sh"
MAKEFILE="$ROOT/Makefile"

require_files "$GUARD_LABEL" \
    "$MATRIX" \
    "$PACKAGE_CHECKLIST" \
    "$OPERATIONS_INDEX" \
    "$CI_GOVERNANCE" \
    "$FLEET_INSTALL" \
    "$PACKAGED_CANISTER" \
    "$MAKEFILE"

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" \
    "release-validation-matrix.md" \
    "release-package-install-validation.md"
require_texts "$CI_GOVERNANCE" "$GUARD_LABEL" "release-validation-matrix.md"

require_texts "$MATRIX" "$GUARD_LABEL" \
    "## Required Slice Gates" \
    "## Required CI Gates" \
    "## Focused Replay, Auth, And Cost Gates" \
    "## Package And Install Gates" \
    "## Reporting Format" \
    "cargo test --locked -p canic --test changelog_governance -- --nocapture" \
    "git diff --check" \
    "use only the guards that directly own" \
    "The active workflow is the source of truth." \
    "Do not reproduce its step-by-step" \
    "job counts or step adjacency." \
    "make validate" \
    "make package" \
    "requires exactly one Application Subnet" \
    "Neither target is multi-root qualification evidence."

require_texts "$PACKAGE_CHECKLIST" "$GUARD_LABEL" \
    "## Scope" \
    "## Existing Package and Install Gates" \
    "## Artifact Verification Expectations" \
    "## Environment and Ownership" \
    "## Release Flow Boundary" \
    "## Required RC Gates" \
    "## Outcome Summary" \
    "make package" \
    "make test-installed-canic-cli" \
    "make test-packaged-downstream-cli" \
    "make test-packaged-downstream-wasm-store" \
    'packaged `build!`, `start!` and `finish!`' \
    "MSRV/local/IC boundary" \
    "cargo build --release --workspace --locked" \
    "make test-fleet-install" \
    "make test-canisters" \
    "Automated agents must never change release versions" \
    "Package validation must not leave committed package artifacts" \
    "This checklist does not issue a release verdict."

# shellcheck disable=SC2016 # Match literal variables in the inspected script.
require_texts "$FLEET_INSTALL" "$GUARD_LABEL" \
    'if [[ "$application_subnet_count" -ne 1 ]]; then' \
    'cargo run --locked' \
    '--fleet-input "$input_path"'
require_texts "$PACKAGED_CANISTER" "$GUARD_LABEL" \
    'cargo +1.91.0 build --offline --locked' \
    'cargo run --manifest-path "$tool_root/Cargo.toml" --offline --locked' \
    'cargo package --locked'
require_text "$MAKEFILE" "test-canisters: test-fleet-install" "$GUARD_LABEL"
require_text "$MAKEFILE" 'cargo package --locked' "$GUARD_LABEL"

echo "release validation matrix guard passed"
