#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/doc-guard-lib.sh
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="release validation"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
CI_GOVERNANCE="$ROOT/docs/governance/ci-deployment.md"
FLEET_INSTALL="$ROOT/scripts/ci/test-fleet-install.sh"
MAKEFILE="$ROOT/Makefile"

require_files "$GUARD_LABEL" \
    "$MATRIX" \
    "$OPERATIONS_INDEX" \
    "$CI_GOVERNANCE" \
    "$FLEET_INSTALL" \
    "$MAKEFILE"

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "release-validation-matrix.md"
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

forbid_texts "$MATRIX" "$GUARD_LABEL" \
    "invoke install without the required Fleet input" \
    "query the removed single-root Registry surface" \
    "They are not valid 0.100 evidence yet"

# shellcheck disable=SC2016 # Match literal variables in the inspected script.
require_texts "$FLEET_INSTALL" "$GUARD_LABEL" \
    'if [[ "$application_subnet_count" -ne 1 ]]; then' \
    '--fleet-input "$input_path"'
require_text "$MAKEFILE" "test-canisters: test-fleet-install" "$GUARD_LABEL"

echo "release validation matrix guard passed"
