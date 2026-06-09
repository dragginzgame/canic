#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="upgrade/state audit"
AUDIT="$ROOT/docs/operations/upgrade-state-compatibility-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
OLD_AUDIT_NAME="0.62-upgrade"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-state-compatibility-audit.md"

require_files "$GUARD_LABEL" "$AUDIT" "$OPERATIONS_INDEX" "$MATRIX"

forbid_operations_file "$OLD_AUDIT_NAME" "upgrade/state audit must use the non-versioned operations path"
forbid_git_reference "$OLD_AUDIT_NAME" "upgrade/state docs must not point at an old versioned audit path" docs CHANGELOG.md .github scripts

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "upgrade-state-compatibility-audit.md"
require_texts "$MATRIX" "$GUARD_LABEL" "upgrade-state-compatibility-audit.md"

require_texts "$AUDIT" "$GUARD_LABEL" \
    "## Scope" \
    "## Compatibility Boundaries" \
    "## State Area Matrix" \
    "## State-Invariant Checklist" \
    "## Required RC Gates" \
    "## Outcome Summary" \
    "Replay receipt persistence and shape stability" \
    "Project-local pending operation log" \
    "Delegated-auth hard cut" \
    "ICP refill and value-transfer replay state" \
    "Lifecycle post-upgrade ordering" \
    "Durable-publication and wasm-store state" \
    "bash scripts/ci/check-upgrade-state-audit.sh" \
    "cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture" \
    "cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture" \
    "cargo test --locked -p canic-tests --test lifecycle_boundary -- --test-threads=1 --nocapture" \
    "Release blockers: none found in this audit."

echo "upgrade/state compatibility audit guard passed"
