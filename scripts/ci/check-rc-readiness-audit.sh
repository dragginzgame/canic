#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
AUDIT="$ROOT/docs/operations/rc-readiness-audit.md"
OPERATIONS_INDEX="$ROOT/docs/operations/README.md"
MATRIX="$ROOT/docs/operations/release-validation-matrix.md"
PACKAGE_CHECKLIST="$ROOT/docs/operations/release-package-install-validation.md"
OLD_AUDIT_NAME="0.62-rc"
OLD_AUDIT_NAME="$OLD_AUDIT_NAME-readiness-audit.md"

require_file() {
    local path="$1"
    if [ ! -f "$path" ]; then
        echo "missing required RC readiness audit file: ${path#$ROOT/}" >&2
        exit 1
    fi
}

require_text() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "missing required RC readiness audit text in ${path#$ROOT/}: $needle" >&2
        exit 1
    fi
}

require_file "$AUDIT"
require_file "$OPERATIONS_INDEX"
require_file "$MATRIX"
require_file "$PACKAGE_CHECKLIST"

if [ -e "$ROOT/docs/operations/$OLD_AUDIT_NAME" ]; then
    echo "RC readiness audit must use the non-versioned operations path" >&2
    exit 1
fi

if git -C "$ROOT" grep -n "$OLD_AUDIT_NAME" -- docs CHANGELOG.md .github scripts; then
    echo "RC readiness docs must not point at an old versioned audit path" >&2
    exit 1
fi

require_text "$OPERATIONS_INDEX" "rc-readiness-audit.md"
require_text "$MATRIX" "rc-readiness-audit.md"
require_text "$PACKAGE_CHECKLIST" "rc-readiness-audit.md"

require_text "$AUDIT" "## A. Verdict"
require_text "$AUDIT" "READY TO CLOSE 0.62 IMPLEMENTATION WORK"
require_text "$AUDIT" "## B. Scope Confirmation"
require_text "$AUDIT" "## C. 0.62 Completion Summary"
require_text "$AUDIT" "## D. Blockers"
require_text "$AUDIT" "None found in this audit."
require_text "$AUDIT" "## E. Non-Blocking Release-Readiness Work"
require_text "$AUDIT" "## F. Validation Results"
require_text "$AUDIT" "## G. Recommendation"

require_text "$AUDIT" "implementation close-out from RC promotion and final release"
require_text "$AUDIT" "This verdict closes implementation slicing only"
require_text "$AUDIT" "Move to RC/full validation flow."
require_text "$AUDIT" "Avoid starting a 0.62.7 implementation slice"
require_text "$AUDIT" "bash scripts/ci/check-rc-readiness-audit.sh"
require_text "$AUDIT" "cargo test --locked -p canic-core replay_policy --lib -- --nocapture"
require_text "$AUDIT" "make package"
require_text "$AUDIT" "make test-canisters"
require_text "$AUDIT" "Do not change runtime behavior, CLI output, Candid, JSON/output formats"

echo "RC readiness audit guard passed"
