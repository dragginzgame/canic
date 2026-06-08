#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
AUDIT="$ROOT/docs/operations/diagnostic-consistency-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
RUNBOOK="$ROOT/docs/operations/recovery-retry-runbooks.md"
OLD_AUDIT_NAME="0.62-diagnostic"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-consistency-audit.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required diagnostic consistency audit file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required diagnostic consistency audit text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$AUDIT"
require_file "$OPERATIONS_INDEX"
require_file "$MATRIX"
require_file "$RUNBOOK"

if [ -e "$ROOT/docs/operations/$OLD_AUDIT_NAME" ]; then
    echo "diagnostic consistency audit must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_AUDIT_NAME" -- docs CHANGELOG.md .github scripts; then
    echo "diagnostic consistency docs must not point at an old versioned audit path" >&2
    exit 1
fi

require_text "$OPERATIONS_INDEX" "diagnostic-consistency-audit.md"
require_text "$MATRIX" "diagnostic-consistency-audit.md"
require_text "$RUNBOOK" "diagnostic-consistency-audit.md"

require_text "$AUDIT" "## Scope"
require_text "$AUDIT" "## Public-Output Boundary"
require_text "$AUDIT" "## Outcome Labels"
require_text "$AUDIT" "## Diagnostic Matrix"
require_text "$AUDIT" "## Required RC Gates"
require_text "$AUDIT" "## Diagnostic Change Rules"
require_text "$AUDIT" "## Outcome Summary"

require_text "$AUDIT" "Duplicate or committed replay"
require_text "$AUDIT" "Missing operation ID"
require_text "$AUDIT" "Invalid operation ID"
require_text "$AUDIT" "Expired replay metadata or receipt"
require_text "$AUDIT" "Wrong caller or actor mismatch"
require_text "$AUDIT" "Wrong shard or delegated-auth shard mismatch"
require_text "$AUDIT" "Delegation-proof replay"
require_text "$AUDIT" "Delegated-token mint or issue replay"
require_text "$AUDIT" "Pending operation already exists"
require_text "$AUDIT" "Recovery-required operation state"
require_text "$AUDIT" "Value-transfer cost refusal"
require_text "$AUDIT" "Upgrade or permit-boundary refusal"
require_text "$AUDIT" "Durable-publication conflict or ambiguity"

require_text "$AUDIT" "Every future diagnostic change must declare its output class"
require_text "$AUDIT" "Public output changes are not included in this audit."
require_text "$AUDIT" "bash scripts/ci/check-diagnostic-consistency-audit.sh"
require_text "$AUDIT" "cargo test --locked -p canic-core replay_policy --lib -- --nocapture"
require_text "$AUDIT" "cargo test --locked -p canic-core workflow::rpc::request::handler --lib -- --nocapture"
require_text "$AUDIT" "Release blockers: none found in this audit."

echo "diagnostic consistency audit guard passed"
