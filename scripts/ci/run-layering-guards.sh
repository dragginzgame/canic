#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

ops_policy_pattern='crate\s*::\s*domain\s*::\s*policy|use\s+crate\s*::\s*\{[^;]*\bdomain\s*::\s*(?:policy|\{[^;]*\bpolicy\b)'

scan_ops_to_policy() {
    local root="$1"
    local name_pattern="$2"
    local file

    while IFS= read -r -d '' file; do
        if awk '
                /^#\[cfg\(test\)\]$/ { cfg_test = 1; next }
                cfg_test && /^mod tests;$/ { cfg_test = 0; next }
                cfg_test && /^mod tests[[:space:]]*\{/ { exit }
                cfg_test { cfg_test = 0 }
                { print }
            ' "$file" \
            | rg --pcre2 --multiline --quiet "$ops_policy_pattern"; then
            printf '%s\n' "$file"
        fi
    done < <(find "$root" -type f -name "$name_pattern" ! -name tests.rs -print0)
}

fixture_root="docs/audits/fixtures/layering"
expected_fixture_matches="$fixture_root/forbidden-direct-import.txt
$fixture_root/forbidden-grouped-import.txt
$fixture_root/forbidden-nested-grouped-import.txt"
actual_fixture_matches="$(scan_ops_to_policy "$fixture_root" '*.txt' | sort)"

if [[ "$actual_fixture_matches" != "$expected_fixture_matches" ]]; then
    echo "ops-to-policy detector fixture mismatch" >&2
    printf 'expected:\n%s\nactual:\n%s\n' \
        "$expected_fixture_matches" "$actual_fixture_matches" >&2
    exit 2
fi

if [[ "${1:-}" == "--self-test" ]]; then
    if [[ $# -ne 1 ]]; then
        echo "usage: scripts/ci/run-layering-guards.sh [--self-test]" >&2
        exit 2
    fi
    echo "layering guard detector fixtures passed"
    exit 0
fi

if [[ $# -ne 0 ]]; then
    echo "usage: scripts/ci/run-layering-guards.sh [--self-test]" >&2
    exit 2
fi

failed=0

ops_policy_matches="$(scan_ops_to_policy crates/canic-core/src/ops '*.rs' | sort)"
if [[ -n "$ops_policy_matches" ]]; then
    printf '%s\n' "$ops_policy_matches"
    echo "ops must not depend upward on the policy layer" >&2
    failed=1
fi

if rg "storage::.*Record|storage::stable" \
    crates/canic-core/src/workflow \
    crates/canic-control-plane/src/workflow \
    --glob '!**/tests.rs'; then
    echo "workflow must not touch stable storage or storage records" >&2
    failed=1
fi

if rg "(^|[^A-Za-z0-9_])api::|crate::api::" crates/canic-core/src/workflow --glob '*.rs'; then
    echo "workflow must not depend on the api layer" >&2
    failed=1
fi

if rg "ops::replay|ReplayReceipt|ReplayPayloadHasher|ReplayReceiptDecision|ReplayReceiptReserveInput|reserve_or_replay_receipt|commit_receipt_response|mark_recovery_required" crates/canic-core/src/api --glob '*.rs' --glob '!**/tests.rs'; then
    echo "api must delegate shared replay orchestration to workflow" >&2
    failed=1
fi

if rg "RootDelegatedRoleGrantPolicy|RootDelegationAudiencePolicy|\bRootIssuerPolicy\b|AuthStateOps::upsert_root_issuer_policy|fn root_issuer_policy_|fn validate_root_issuer_policy_upsert_request" crates/canic-core/src/api --glob '*.rs' --glob '!**/tests.rs'; then
    echo "api must delegate root issuer policy upsert handling to auth ops" >&2
    failed=1
fi

if rg "struct .*Policy|enum .*Policy|impl .*Policy" crates/canic-core/src/workflow --glob '*.rs' --glob '!**/tests.rs'; then
    echo "workflow must apply policy, not define policy types" >&2
    failed=1
fi

if rg "crate::dto::|use crate::dto|\bdto::" crates/canic-core/src/domain crates/canic-core/src/storage crates/canic-core/src/model --glob '*.rs'; then
    echo "domain, storage, and model layers must not depend on DTOs" >&2
    failed=1
fi

if find crates/canic-core/src/ops/auth -name '*.rs' ! -name 'tests.rs' -print0 \
    | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
    | rg "dto::error::Error|crate::dto::error|ErrorCode|InternalError::public\("; then
    echo "auth ops must use internal error constructors, not public error DTOs" >&2
    failed=1
fi

if find crates/canic-core/src/access -name '*.rs' -print0 \
    | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
    | rg "stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord"; then
    echo "access must use ops boundaries, not storage records or stable types" >&2
    failed=1
fi

if rg "pub use .*Record" crates/canic-core/src | rg -v "pub\\(crate\\)"; then
    echo "record types must not be publicly re-exported" >&2
    failed=1
fi

if rg "(to_view|from_view)" crates/canic-core/src | rg -v "record_to_view|view::"; then
    echo "misuse of 'view' detected in function names" >&2
    failed=1
fi

exit "$failed"
