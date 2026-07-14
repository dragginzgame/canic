#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: scripts/ci/audit-product-tree-hash.sh <commit>" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

commit="$(git rev-parse "$1^{commit}")"
git ls-tree -r --full-tree --format='%(objectmode) %(objectname) %(path)' "$commit" \
    | awk '
        {
            path = $0
            sub(/^[^ ]+ [^ ]+ /, "", path)
        }
        path == "CHANGELOG.md" { next }
        path == "docs/status/current.md" { next }
        index(path, "docs/audits/") == 1 { next }
        index(path, "docs/design/0.92-holistic-audit-and-audit-system-validation/") == 1 { next }
        path == "scripts/ci/audit-product-tree-hash.sh" { next }
        path == "scripts/ci/check-audit-method-catalog.sh" { next }
        path == "scripts/ci/instruction-audit-report.sh" { next }
        path == "scripts/ci/run-layering-guards.sh" { next }
        path == "scripts/ci/wasm-audit-report.sh" { next }
        path == "crates/canic-tests/tests/instruction_audit.rs" { next }
        index(path, "crates/canic-tests/tests/instruction_audit_support/") == 1 { next }
        { print }
    ' \
    | sha256sum \
    | awk '{print $1}'
