#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CATALOG="$ROOT/docs/audits/METHODS.md"
HOWTO="$ROOT/docs/audits/AUDIT-HOWTO.md"
META="$ROOT/docs/audits/META-AUDIT.md"
RETIRED="$ROOT/docs/audits/retired-methods.md"
FINGERPRINTS="$ROOT/docs/audits/method-fingerprints-v1.md"
TRACE_PROTOCOL="$ROOT/docs/audits/mandatory-trace-protocol.md"

for file in "$CATALOG" "$HOWTO" "$META" "$RETIRED" "$FINGERPRINTS" "$TRACE_PROTOCOL"; do
    if [[ ! -f "$file" ]]; then
        echo "audit method catalog missing required file: ${file#$ROOT/}" >&2
        exit 1
    fi
done

mapfile -t definitions < <(
    find \
        "$ROOT/docs/audits/recurring/system" \
        "$ROOT/docs/audits/recurring/invariants" \
        -maxdepth 1 -type f -name '*.md' ! -name README.md -print
    printf '%s\n' "$ROOT/docs/audits/modular/module-surface-hardening.md"
)

expected_count=22
if [[ "${#definitions[@]}" -ne "$expected_count" ]]; then
    echo "audit method catalog expected $expected_count active definitions, found ${#definitions[@]}" >&2
    exit 1
fi

required_fields=(
    '## Method Contract'
    '- Audit ID: `CANIC-'
    '- Method version:'
    '- Disposition:'
    '- Owner:'
    '- Kind/profile:'
    '- Trace mode:'
    '- Cost/runtime:'
    '- Prerequisites:'
    '- False-positive boundary:'
    '- Shared contract:'
)

for definition in "${definitions[@]}"; do
    for field in "${required_fields[@]}"; do
        if ! grep -Fq -- "$field" "$definition"; then
            echo "${definition#$ROOT/}: missing method field: $field" >&2
            exit 1
        fi
    done

    basename="$(basename "$definition")"
    if ! grep -Fq -- "$basename" "$CATALOG"; then
        echo "${definition#$ROOT/}: not listed in docs/audits/METHODS.md" >&2
        exit 1
    fi

    relative_path="${definition#$ROOT/}"
    audit_id="$(sed -n 's/^- Audit ID: `\([^`]*\)`.*/\1/p' "$definition")"
    method_version="$(sed -n 's/^- Method version: `\([^`]*\)`.*/\1/p' "$definition")"
    content_hash="$(sha256sum "$definition" | awk '{print $1}')"
    fingerprint_row="| \`$audit_id\` | \`$method_version\` | \`$content_hash\` | \`$relative_path\` |"
    if ! grep -Fqx -- "$fingerprint_row" "$FINGERPRINTS"; then
        echo "$relative_path: method fingerprint manifest is stale" >&2
        exit 1
    fi
done

fingerprinted_inputs=(
    docs/audits/AUDIT-HOWTO.md
    docs/audits/META-AUDIT.md
    docs/audits/METHODS.md
    docs/audits/mandatory-trace-protocol.md
    docs/audits/product-tree-scope-v1.md
    docs/audits/retired-methods.md
    docs/audits/fixtures/layering/allowed-import.txt
    docs/audits/fixtures/layering/forbidden-direct-import.txt
    docs/audits/fixtures/layering/forbidden-grouped-import.txt
    docs/audits/fixtures/layering/forbidden-nested-grouped-import.txt
    docs/audits/fixtures/change-friction-v2-sample.tsv
    docs/audits/scripts/measure-change-friction-v2.sh
    docs/audits/scripts/measure-complexity-v2.sh
    docs/audits/scripts/run-nonempty-cargo-test.sh
    scripts/ci/audit-product-tree-hash.sh
    scripts/ci/check-audit-method-catalog.sh
    scripts/ci/instruction-audit-report.sh
    scripts/ci/run-layering-guards.sh
    scripts/ci/wasm-audit-report.sh
)
for relative_path in "${fingerprinted_inputs[@]}"; do
    content_hash="$(sha256sum "$ROOT/$relative_path" | awk '{print $1}')"
    fingerprint_row="| \`$content_hash\` | \`$relative_path\` |"
    if ! grep -Fqx -- "$fingerprint_row" "$FINGERPRINTS"; then
        echo "$relative_path: executable/governance fingerprint manifest is stale" >&2
        exit 1
    fi
done

duplicate_ids="$(
    sed -n 's/^- Audit ID: `\([^`]*\)`.*/\1/p' "${definitions[@]}" \
        | sort \
        | uniq -d
)"
if [[ -n "$duplicate_ids" ]]; then
    echo "duplicate active audit method IDs:" >&2
    echo "$duplicate_ids" >&2
    exit 1
fi

grep -Fq -- '## Holistic Coverage Ownership' "$CATALOG"
grep -Fq -- 'CANIC-MANDATORY-TRACE-001/v1' "$CATALOG"
grep -Fq -- '- Audit ID: `CANIC-MANDATORY-TRACE-001`' "$TRACE_PROTOCOL"
grep -Fq -- '- Method version: `1`' "$TRACE_PROTOCOL"
for trace_id in \
    TRACE-DEPLOY-001 \
    TRACE-AUTH-001 \
    TRACE-CAPABILITY-001 \
    TRACE-CYCLES-001 \
    TRACE-INTENT-001 \
    TRACE-CONTROL-001 \
    TRACE-TOPOLOGY-001 \
    TRACE-BLOB-001 \
    TRACE-BACKUP-001 \
    TRACE-LIFECYCLE-001; do
    grep -Fq -- "$trace_id" "$TRACE_PROTOCOL"
done
grep -Fq -- 'snapshot_status:' "$FINGERPRINTS"
grep -Fq -- 'result_validity:' "$HOWTO"
grep -Fq -- '## Authority Precedence' "$META"
grep -Fq -- 'METHOD_VERSION="2"' "$ROOT/scripts/ci/instruction-audit-report.sh"
grep -Fq -- 'candidate_method_fingerprint' "$ROOT/scripts/ci/instruction-audit-report.sh"
grep -Fq -- '"$candidate_method_fingerprint" != "$METHOD_FINGERPRINT"' "$ROOT/scripts/ci/instruction-audit-report.sh"
grep -Fq -- 'BASELINE_REPORT="N/A"' "$ROOT/scripts/ci/instruction-audit-report.sh"
grep -Fq -- 'evidence-manifest.yml' "$ROOT/scripts/ci/instruction-audit-report.sh"
grep -Fq -- 'METHOD_VERSION="2"' "$ROOT/scripts/ci/wasm-audit-report.sh"
grep -Fq -- 'disposable linked Git worktree' "$ROOT/scripts/ci/wasm-audit-report.sh"
grep -Fq -- 'CARGO_NET_OFFLINE="true"' "$ROOT/scripts/ci/wasm-audit-report.sh"
grep -Fq -- 'root_independent_composite' "$ROOT/scripts/ci/wasm-audit-report.sh"
grep -Fq -- 'candidate_method_fingerprint' "$ROOT/scripts/ci/wasm-audit-report.sh"
grep -Fq -- 'build_artifact' "$ROOT/scripts/ci/wasm-audit-report.sh"
if grep -Fq -- 'cargo build --target wasm32-unknown-unknown' "$ROOT/scripts/ci/wasm-audit-report.sh"; then
    echo "Wasm audit runner must not restore direct Cargo Wasm compilation" >&2
    exit 1
fi
grep -Fq -- 'evidence-manifest.yml' "$ROOT/scripts/ci/wasm-audit-report.sh"

echo "audit method catalog guard passed"
