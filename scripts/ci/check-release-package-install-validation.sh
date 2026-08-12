#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/doc-guard-lib.sh
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release package/install validation"
CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
FLEET_INSTALL="$ROOT/scripts/ci/test-fleet-install.sh"
MAKEFILE="$ROOT/Makefile"

require_files "$GUARD_LABEL" \
    "$CHECKLIST" \
    "$OPERATIONS_INDEX" \
    "$MATRIX" \
    "$FLEET_INSTALL" \
    "$MAKEFILE"

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "release-package-install-validation.md"
require_texts "$MATRIX" "$GUARD_LABEL" "release-package-install-validation.md"
require_texts "$CHECKLIST" "$GUARD_LABEL" \
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
    "shipped operator command" \
    "structured JSON error" \
    "live sync, live fund" \
    "cargo build --release --workspace --locked" \
    "make test-fleet-install" \
    "make test-canisters" \
    "bash scripts/ci/check-release-package-install-validation.sh" \
    "Automated agents must never change release versions" \
    "Package validation must not leave committed package artifacts" \
    "single-root local installation/application evidence" \
    "This checklist does not issue a release verdict."

forbid_texts "$CHECKLIST" "$GUARD_LABEL" \
    "current stale recipe" \
    "remain blocked until" \
    "must not be run or credited" \
    "Release blockers: none found"

# shellcheck disable=SC2016 # Match literal variables in the inspected script.
require_texts "$FLEET_INSTALL" "$GUARD_LABEL" \
    'if [[ "$application_subnet_count" -ne 1 ]]; then' \
    '--fleet-input "$input_path"'
require_text "$MAKEFILE" "test-canisters: test-fleet-install" "$GUARD_LABEL"

echo "release package/install validation guard passed"
