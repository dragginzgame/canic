#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
DIAGNOSTIC_AUDIT="$ROOT/docs/operations/diagnostic-consistency-audit.md"
OLD_CHECKLIST_NAME="0.62-release-package"
OLD_CHECKLIST_NAME="$OLD_CHECKLIST_NAME-install-validation.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required release package/install validation file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required release package/install validation text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$CHECKLIST"
require_file "$OPERATIONS_INDEX"
require_file "$MATRIX"
require_file "$DIAGNOSTIC_AUDIT"

if [ -e "$ROOT/docs/operations/$OLD_CHECKLIST_NAME" ]; then
    echo "release package/install validation must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_CHECKLIST_NAME" -- docs CHANGELOG.md .github scripts; then
    echo "release package/install docs must not point at an old versioned checklist path" >&2
    exit 1
fi

require_text "$OPERATIONS_INDEX" "release-package-install-validation.md"
require_text "$MATRIX" "release-package-install-validation.md"
require_text "$DIAGNOSTIC_AUDIT" "release-package-install-validation.md"

require_text "$CHECKLIST" "## Scope"
require_text "$CHECKLIST" "## Existing Package and Install Gates"
require_text "$CHECKLIST" "## Artifact Verification Expectations"
require_text "$CHECKLIST" "## Environment and Ownership"
require_text "$CHECKLIST" "## Release Flow Boundary"
require_text "$CHECKLIST" "## Required RC Gates"
require_text "$CHECKLIST" "## Outcome Summary"

require_text "$CHECKLIST" "make package"
require_text "$CHECKLIST" "make test-installed-canic-cli"
require_text "$CHECKLIST" "make test-packaged-downstream-cli"
require_text "$CHECKLIST" "make test-packaged-downstream-wasm-store"
require_text "$CHECKLIST" "cargo build --release --workspace --locked"
require_text "$CHECKLIST" "make test-fleet-install"
require_text "$CHECKLIST" "make test-canisters"
require_text "$CHECKLIST" "bash scripts/ci/check-release-package-install-validation.sh"
require_text "$CHECKLIST" "Automated agents must not change release versions"
require_text "$CHECKLIST" "Package validation must not leave committed package artifacts"
require_text "$CHECKLIST" "Release blockers: none found in this checklist."

echo "release package/install validation guard passed"
