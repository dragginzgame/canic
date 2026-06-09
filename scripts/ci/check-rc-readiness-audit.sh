#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="RC readiness audit"
AUDIT="$ROOT/docs/operations/rc-readiness-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
PACKAGE_CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OLD_AUDIT_NAME="0.62-rc"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-readiness-audit.md"

require_files "$GUARD_LABEL" "$AUDIT" "$OPERATIONS_INDEX" "$MATRIX" "$PACKAGE_CHECKLIST"

forbid_operations_file "$OLD_AUDIT_NAME" "RC readiness audit must use the non-versioned operations path"
forbid_git_reference "$OLD_AUDIT_NAME" "RC readiness docs must not point at an old versioned audit path" docs CHANGELOG.md .github scripts

require_texts "$OPERATIONS_INDEX" "$GUARD_LABEL" "rc-readiness-audit.md"
require_texts "$MATRIX" "$GUARD_LABEL" "rc-readiness-audit.md"
require_texts "$PACKAGE_CHECKLIST" "$GUARD_LABEL" "rc-readiness-audit.md"

require_texts "$AUDIT" "$GUARD_LABEL" \
    "## A. Verdict" \
    "READY TO CLOSE 0.62 IMPLEMENTATION WORK" \
    "## B. Scope Confirmation" \
    "## C. 0.62 Completion Summary" \
    "## D. Blockers" \
    "None found in this audit." \
    "## E. Non-Blocking Release-Readiness Work" \
    "## F. Validation Results" \
    "## G. Recommendation" \
    "implementation close-out from RC promotion and final release" \
    "This verdict closes implementation slicing only" \
    "Move to RC/full validation flow." \
    "Avoid starting a 0.62.7 implementation slice" \
    "bash scripts/ci/check-rc-readiness-audit.sh" \
    "cargo test --locked -p canic-core replay_policy --lib -- --nocapture" \
    "make package" \
    "make test-canisters" \
    "Do not change runtime behavior, CLI output, Candid, JSON/output formats"

echo "RC readiness audit guard passed"
