#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release validation"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
CI_GOVERNANCE="$ROOT/docs/governance/ci-deployment.md"
OLD_MATRIX_NAME="0.62-release-validation"
OLD_MATRIX_NAME="$OLD_MATRIX_NAME-matrix.md"

require_files "$GUARD_LABEL" "$MATRIX" "$OPERATIONS_INDEX" "$CI_GOVERNANCE"

forbid_operations_file "$OLD_MATRIX_NAME" "release validation matrix must use the non-versioned operations path"
forbid_git_reference "$OLD_MATRIX_NAME" "release validation docs must not point at the old versioned matrix path" docs CHANGELOG.md .github

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "release-validation-matrix.md"
require_texts "$CI_GOVERNANCE" "$GUARD_LABEL" "release-validation-matrix.md"

require_texts "$MATRIX" "$GUARD_LABEL" \
    "## Required Slice Gates" \
    "## Required CI Gates" \
    "## Focused Replay, Auth, And Cost Gates" \
    "## Package And Install Gates" \
    "## Reporting Format" \
    "cargo fmt --all -- --check" \
    "cargo test --locked -p canic --test changelog_governance -- --nocapture" \
    "git diff --check" \
    "bash scripts/ci/check-release-validation-matrix.sh" \
    "make fmt-check" \
    "make clippy" \
    "make test" \
    "make package"

echo "release validation matrix guard passed"
