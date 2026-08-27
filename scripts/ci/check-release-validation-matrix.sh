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
PACKAGED_CANISTER="$ROOT/scripts/ci/verify-packaged-downstream-wasm-store.sh"
MAKEFILE="$ROOT/Makefile"

require_files "$GUARD_LABEL" \
    "$MATRIX" \
    "$PACKAGE_CHECKLIST" \
    "$OPERATIONS_INDEX" \
    "$CI_GOVERNANCE" \
    "$PACKAGED_CANISTER" \
    "$MAKEFILE"

for document in "$MATRIX" "$PACKAGE_CHECKLIST" "$OPERATIONS_INDEX" "$CI_GOVERNANCE"; do
    rg -q '^# ' "$document" || {
        echo "release validation document lacks a Markdown title: $(guard_path "$document")" >&2
        exit 1
    }
done

for linked_document in release-validation-matrix.md release-package-install-validation.md; do
    rg -q "\\([^)]*$linked_document\\)" "$OPERATIONS_INDEX" || {
        echo "operations index does not link $linked_document" >&2
        exit 1
    }
done
rg -q '\([^)]*release-validation-matrix\.md\)' "$CI_GOVERNANCE" || {
    echo "CI governance does not link the release validation matrix" >&2
    exit 1
}

require_texts "$PACKAGED_CANISTER" "$GUARD_LABEL" \
    'cargo +1.91.0 build --offline --locked' \
    'cargo run --manifest-path "$tool_root/Cargo.toml" --offline --locked' \
    'cargo package --locked'
require_text "$MAKEFILE" 'cargo package --locked' "$GUARD_LABEL"

echo "release validation matrix guard passed"
