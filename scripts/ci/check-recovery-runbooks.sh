#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUNBOOK="$ROOT/docs/operations/recovery-retry-runbooks.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
UPGRADE_AUDIT="$ROOT/docs/operations/upgrade-state-compatibility-audit.md"
OLD_RUNBOOK_NAME="0.62-recovery"
OLD_RUNBOOK_NAME="$OLD_RUNBOOK_NAME-retry-runbooks.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required recovery runbook file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required recovery runbook text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$RUNBOOK"
require_file "$OPERATIONS_INDEX"
require_file "$MATRIX"
require_file "$UPGRADE_AUDIT"

if [ -e "$ROOT/docs/operations/$OLD_RUNBOOK_NAME" ]; then
    echo "recovery runbooks must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_RUNBOOK_NAME" -- docs CHANGELOG.md .github scripts; then
    echo "recovery runbook docs must not point at an old versioned runbook path" >&2
    exit 1
fi

require_text "$OPERATIONS_INDEX" "recovery-retry-runbooks.md"
require_text "$MATRIX" "recovery-retry-runbooks.md"
require_text "$UPGRADE_AUDIT" "recovery-retry-runbooks.md"

require_text "$RUNBOOK" "## Scope"
require_text "$RUNBOOK" "## Operator Safety Rules"
require_text "$RUNBOOK" "## Runbook Template"
require_text "$RUNBOOK" "## Runbooks"
require_text "$RUNBOOK" "### Safe Retry After Network Or Client Failure"
require_text "$RUNBOOK" "### Duplicate Operation Or Committed Replay"
require_text "$RUNBOOK" "### Operation Already In Progress"
require_text "$RUNBOOK" "### Payload Or Caller Mismatch"
require_text "$RUNBOOK" "### Expired Authorization Or Replay Metadata"
require_text "$RUNBOOK" "### Delegation Caller Or Shard Mismatch"
require_text "$RUNBOOK" "### Project-Local Pending ICP Refill"
require_text "$RUNBOOK" "### ICP Refill Recovery-Required State"
require_text "$RUNBOOK" "### Cost-Boundary Refusal"
require_text "$RUNBOOK" "### Durable-Publication Ambiguity"
require_text "$RUNBOOK" "### Upgrade Interrupted Near Replay-Sensitive Operation"
require_text "$RUNBOOK" "### Receipt Mismatch Or Unexpected Receipt State"
require_text "$RUNBOOK" "## Validation Gates"
require_text "$RUNBOOK" "## Outcome Summary"

require_text "$RUNBOOK" "same operation ID, same actor, same payload"
require_text "$RUNBOOK" "Do not change payload, caller, shard, or target while reusing an operation ID."
require_text "$RUNBOOK" "bash scripts/ci/check-recovery-runbooks.sh"
require_text "$RUNBOOK" "cargo test --locked -p canic-core replay_policy --lib -- --nocapture"
require_text "$RUNBOOK" "cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture"
require_text "$RUNBOOK" "cargo test --locked -p canic-cli cycles::convert --lib -- --nocapture"
require_text "$RUNBOOK" "Release blockers: none found in these runbooks."

echo "recovery/retry runbooks guard passed"
