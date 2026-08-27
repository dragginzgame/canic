#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/doc-guard-lib.sh
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="recovery runbook"
RUNBOOK="$ROOT/docs/operations/recovery-retry-runbooks.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"

require_files "$GUARD_LABEL" "$RUNBOOK" "$OPERATIONS_INDEX" "$MATRIX"

for document in "$RUNBOOK" "$OPERATIONS_INDEX" "$MATRIX"; do
    rg -q '^# ' "$document" || {
        echo "recovery runbook document lacks a Markdown title: $(guard_path "$document")" >&2
        exit 1
    }
done

# Keep the operator procedure discoverable without freezing its headings,
# example commands, or explanatory prose.
rg -q '\([^)]*recovery-retry-runbooks\.md\)' "$OPERATIONS_INDEX" || {
    echo "operations index does not link the recovery runbook" >&2
    exit 1
}
rg -q '\([^)]*recovery-retry-runbooks\.md\)' "$MATRIX" || {
    echo "release matrix does not link the recovery runbook" >&2
    exit 1
}

if rg -n '^(<<<<<<<|=======|>>>>>>>)' "$RUNBOOK" >/dev/null; then
    echo "recovery runbook contains unresolved merge markers" >&2
    exit 1
fi

echo "recovery/retry runbooks guard passed"
