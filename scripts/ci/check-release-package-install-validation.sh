#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release package/install validation"
CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"

require_files "$GUARD_LABEL" "$CHECKLIST" "$OPERATIONS_INDEX" "$MATRIX"

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
    "Automated agents must not change release versions" \
    "Package validation must not leave committed package artifacts" \
    "Release blockers: none found in this checklist."

echo "release package/install validation guard passed"
