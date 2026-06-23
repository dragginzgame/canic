#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release package/install validation"
CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
DIAGNOSTIC_AUDIT="$ROOT/docs/operations/diagnostic-consistency-audit.md"
OLD_CHECKLIST_NAME="0.62-release-package"
OLD_CHECKLIST_NAME="$OLD_CHECKLIST_NAME-install-validation.md"

require_files "$GUARD_LABEL" "$CHECKLIST" "$OPERATIONS_INDEX" "$MATRIX" "$DIAGNOSTIC_AUDIT"

forbid_operations_file "$OLD_CHECKLIST_NAME" "release package/install validation must use the non-versioned operations path"
forbid_git_reference "$OLD_CHECKLIST_NAME" "release package/install docs must not point at an old versioned checklist path" docs CHANGELOG.md .github scripts

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "release-package-install-validation.md"
require_texts "$MATRIX" "$GUARD_LABEL" "release-package-install-validation.md"
require_texts "$DIAGNOSTIC_AUDIT" "$GUARD_LABEL" "release-package-install-validation.md"

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
