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

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "recovery-retry-runbooks.md"
require_texts "$MATRIX" "$GUARD_LABEL" "recovery-retry-runbooks.md"
require_texts "$RUNBOOK" "$GUARD_LABEL" \
    "## Scope" \
    "## Operator Safety Rules" \
    "## Runbook Template" \
    "## Runbooks" \
    "### Safe Retry After Network Or Client Failure" \
    "### Duplicate Operation Or Committed Replay" \
    "### Operation Already In Progress" \
    "### Payload Or Caller Mismatch" \
    "### Expired Authorization Or Replay Metadata" \
    "### Delegation Caller Or Issuer Mismatch" \
    "### ICP Project Root Pending ICP Refill" \
    "### ICP Refill Recovery-Required State" \
    "### Cost-Boundary Refusal" \
    "### Durable-Publication Ambiguity" \
    "### Same-Release Restart Or Upgrade Near Replay-Sensitive Operation" \
    "### Receipt Mismatch Or Unexpected Receipt State" \
    "## Validation Gates" \
    "## Outcome Summary" \
    "same operation ID, same actor, same payload" \
    "Do not change payload, caller, issuer, or target while reusing an operation ID." \
    "These runbooks never authorize a cross-release upgrade or state transition." \
    "bash scripts/ci/check-recovery-runbooks.sh" \
    "cargo test --locked -p canic-core replay_policy --lib -- --nocapture" \
    "cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture" \
    "cargo test --locked -p canic-cli cycles::convert --lib -- --nocapture" \
    "The runbooks remain the operator procedure; they do not issue a release"

forbid_texts "$RUNBOOK" "$GUARD_LABEL" \
    "Project-Local" \
    "project-local" \
    "upgraded binary" \
    "Release blockers: none found"

echo "recovery/retry runbooks guard passed"
