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
        echo "audit method catalog missing required file: ${file#"$ROOT"/}" >&2
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

if [[ "${#definitions[@]}" -eq 0 ]]; then
    echo "audit method catalog has no active definitions" >&2
    exit 1
fi

definition_paths="$({
    for definition in "${definitions[@]}"; do
        printf '%s\n' "${definition#"$ROOT"/}"
    done
} | sort)"
# Parse record columns; headings, wrapping and table padding are editorial.
table_records() {
    local columns="$1"
    local document="$2"
    awk -F '|' -v columns="$columns" '
        NF > 2 && (columns == 0 || NF == columns + 2) {
            for (i = 2; i <= NF - 1; i++) {
                gsub(/^[[:space:]]*`?|`?[[:space:]]*$/, "", $i)
                printf "%s%s", $i, (i == NF - 1 ? "\n" : "\t")
            }
        }
    ' "$document"
}

method_field() {
    local field="$1"
    local document="$2"
    awk -v field="$field" '
        index($0, "- " field ":") == 1 {
            value = substr($0, length(field) + 4)
            gsub(/^[[:space:]]*`?|`?[[:space:]]*$/, "", value)
            print value
            count++
        }
        END { if (count != 1) exit 1 }
    ' "$document"
}

fingerprinted_definition_paths="$(
    table_records 4 "$FINGERPRINTS" | awk -F '\t' '$1 ~ /^CANIC-/ { print $4 }' | sort
)"
if [[ "$definition_paths" != "$fingerprinted_definition_paths" ]]; then
    echo "audit method catalog active definitions differ from the fingerprint manifest" >&2
    diff -u \
        <(printf '%s\n' "$fingerprinted_definition_paths") \
        <(printf '%s\n' "$definition_paths") >&2 || true
    exit 1
fi

for definition in "${definitions[@]}"; do
    relative_path="${definition#"$ROOT"/}"
    audit_id="$(method_field 'Audit ID' "$definition")"
    method_version="$(method_field 'Method version' "$definition")"
    [[ "$audit_id" =~ ^CANIC-[A-Z0-9-]+$ && "$method_version" =~ ^[0-9]+([.][0-9]+)*$ ]] || {
        echo "$relative_path: invalid method identity" >&2
        exit 1
    }
    basename="$(basename "$definition")"
    catalog_matches="$(table_records 0 "$CATALOG" | awk -F '\t' \
        -v name="$basename" \
        '$1 == name { count++ } END { print count+0 }')"
    [[ "$catalog_matches" -eq 1 ]] || {
        echo "$relative_path: missing or duplicate catalog identity" >&2
        exit 1
    }
    content_hash="$(sha256sum "$definition" | awk '{print $1}')"
    fingerprint_matches="$(table_records 4 "$FINGERPRINTS" | awk -F '\t' \
        -v id="$audit_id" -v version="$method_version" -v hash="$content_hash" -v path="$relative_path" \
        '$1 == id && $2 == version && $3 == hash && $4 == path { count++ } END { print count+0 }')"
    if [[ "$fingerprint_matches" -ne 1 ]]; then
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
    if ! table_records 2 "$FINGERPRINTS" | awk -F '\t' \
        -v hash="$content_hash" -v path="$relative_path" \
        '$2 == path { count++; if ($1 != hash) invalid = 1 }
         END { exit !(count == 1 && !invalid) }'; then
        echo "$relative_path: executable/governance fingerprint must be unique and current" >&2
        exit 1
    fi
done

duplicate_ids="$(
    for definition in "${definitions[@]}"; do
        method_field 'Audit ID' "$definition"
    done | sort | uniq -d
)"
if [[ -n "$duplicate_ids" ]]; then
    echo "duplicate active audit method IDs:" >&2
    echo "$duplicate_ids" >&2
    exit 1
fi

# Fingerprints bind the reviewed definitions and executable inputs. Do not
# infer behavior from comments, shell spelling or explanatory document prose.
trace_id="$(method_field 'Audit ID' "$TRACE_PROTOCOL")"
trace_version="$(method_field 'Method version' "$TRACE_PROTOCOL")"
[[ "$trace_id" == CANIC-MANDATORY-TRACE-001 && "$trace_version" =~ ^[0-9]+([.][0-9]+)*$ ]] || {
    echo "invalid mandatory trace method identity" >&2
    exit 1
}
rg -q '\([^)]*mandatory-trace-protocol\.md\)' "$CATALOG" || {
    echo "audit catalog does not link the mandatory trace protocol" >&2
    exit 1
}
trace_ids="$(table_records 2 "$TRACE_PROTOCOL" | awk -F '\t' '$1 ~ /^TRACE-/ { print $1 }')"
[[ -n "$trace_ids" && -z "$(printf '%s\n' "$trace_ids" | sort | uniq -d)" ]] || {
    echo "mandatory trace identities must be nonempty and unique" >&2
    exit 1
}

echo "audit method catalog guard passed"
