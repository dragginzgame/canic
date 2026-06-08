#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
AUDIT="$ROOT/docs/operations/upgrade-state-compatibility-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
OLD_AUDIT_NAME="0.62-upgrade"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-state-compatibility-audit.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required upgrade/state audit file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required upgrade/state audit text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$AUDIT"
require_file "$OPERATIONS_INDEX"
require_file "$MATRIX"

if [ -e "$ROOT/docs/operations/$OLD_AUDIT_NAME" ]; then
    echo "upgrade/state audit must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_AUDIT_NAME" -- docs CHANGELOG.md .github scripts; then
    echo "upgrade/state docs must not point at an old versioned audit path" >&2
    exit 1
fi

require_text "$OPERATIONS_INDEX" "upgrade-state-compatibility-audit.md"
require_text "$MATRIX" "upgrade-state-compatibility-audit.md"

require_text "$AUDIT" "## Scope"
require_text "$AUDIT" "## Compatibility Boundaries"
require_text "$AUDIT" "## State Area Matrix"
require_text "$AUDIT" "## State-Invariant Checklist"
require_text "$AUDIT" "## Required RC Gates"
require_text "$AUDIT" "## Outcome Summary"

require_text "$AUDIT" "Replay receipt persistence and shape stability"
require_text "$AUDIT" "Project-local pending operation log"
require_text "$AUDIT" "Delegated-auth hard cut"
require_text "$AUDIT" "ICP refill and value-transfer replay state"
require_text "$AUDIT" "Lifecycle post-upgrade ordering"
require_text "$AUDIT" "Durable-publication and wasm-store state"

require_text "$AUDIT" "bash scripts/ci/check-upgrade-state-audit.sh"
require_text "$AUDIT" "cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture"
require_text "$AUDIT" "cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture"
require_text "$AUDIT" "cargo test --locked -p canic-tests --test lifecycle_boundary -- --test-threads=1 --nocapture"
require_text "$AUDIT" "Release blockers: none found in this audit."

echo "upgrade/state compatibility audit guard passed"
