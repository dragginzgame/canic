#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
CI_GOVERNANCE="$ROOT/docs/governance/ci-deployment.md"
OLD_MATRIX_NAME="0.62-release-validation"
OLD_MATRIX_NAME="$OLD_MATRIX_NAME-matrix.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required release validation file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required release validation text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$MATRIX"
require_file "$OPERATIONS_INDEX"
require_file "$CI_GOVERNANCE"

if [ -e "$ROOT/docs/operations/$OLD_MATRIX_NAME" ]; then
    echo "release validation matrix must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_MATRIX_NAME" -- docs CHANGELOG.md .github >/tmp/canic-release-validation-old-paths.$$; then
    cat /tmp/canic-release-validation-old-paths.$$ >&2
    rm -f /tmp/canic-release-validation-old-paths.$$
    echo "release validation docs must not point at the old versioned matrix path" >&2
    exit 1
fi
rm -f /tmp/canic-release-validation-old-paths.$$

require_text "$OPERATIONS_INDEX" "release-validation-matrix.md"
require_text "$CI_GOVERNANCE" "release-validation-matrix.md"

require_text "$MATRIX" "## Required Slice Gates"
require_text "$MATRIX" "## Required CI Gates"
require_text "$MATRIX" "## Focused Replay, Auth, And Cost Gates"
require_text "$MATRIX" "## Package And Install Gates"
require_text "$MATRIX" "## Reporting Format"

require_text "$MATRIX" "cargo fmt --all -- --check"
require_text "$MATRIX" "cargo test --locked -p canic --test changelog_governance -- --nocapture"
require_text "$MATRIX" "git diff --check"
require_text "$MATRIX" "bash scripts/ci/check-release-validation-matrix.sh"
require_text "$MATRIX" "make fmt-check"
require_text "$MATRIX" "make clippy"
require_text "$MATRIX" "make test"
require_text "$MATRIX" "make package"

echo "release validation matrix guard passed"
