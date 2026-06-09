#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="diagnostic consistency audit"
AUDIT="$ROOT/docs/operations/diagnostic-consistency-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
RUNBOOK="$ROOT/docs/operations/recovery-retry-runbooks.md"
OLD_AUDIT_NAME="0.62-diagnostic"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-consistency-audit.md"

require_files "$GUARD_LABEL" "$AUDIT" "$OPERATIONS_INDEX" "$MATRIX" "$RUNBOOK"

forbid_operations_file "$OLD_AUDIT_NAME" "diagnostic consistency audit must use the non-versioned operations path"
forbid_git_reference "$OLD_AUDIT_NAME" "diagnostic consistency docs must not point at an old versioned audit path" docs CHANGELOG.md .github scripts

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "diagnostic-consistency-audit.md"
require_texts "$MATRIX" "$GUARD_LABEL" "diagnostic-consistency-audit.md"
require_texts "$RUNBOOK" "$GUARD_LABEL" "diagnostic-consistency-audit.md"

require_texts "$AUDIT" "$GUARD_LABEL" \
    "## Scope" \
    "## Public-Output Boundary" \
    "## Outcome Labels" \
    "## Diagnostic Matrix" \
    "## Required RC Gates" \
    "## Diagnostic Change Rules" \
    "## Outcome Summary" \
    "Duplicate or committed replay" \
    "Missing operation ID" \
    "Invalid operation ID" \
    "Expired replay metadata or receipt" \
    "Wrong caller or actor mismatch" \
    "Wrong shard or delegated-auth shard mismatch" \
    "Delegation-proof replay" \
    "Delegated-token mint or issue replay" \
    "Pending operation already exists" \
    "Recovery-required operation state" \
    "Value-transfer cost refusal" \
    "Upgrade or permit-boundary refusal" \
    "Durable-publication conflict or ambiguity" \
    "Every future diagnostic change must declare its output class" \
    "Public output changes are not included in this audit." \
    "bash scripts/ci/check-diagnostic-consistency-audit.sh" \
    "cargo test --locked -p canic-core replay_policy --lib -- --nocapture" \
    "cargo test --locked -p canic-core workflow::rpc::request::handler --lib -- --nocapture" \
    "Release blockers: none found in this audit."

echo "diagnostic consistency audit guard passed"
