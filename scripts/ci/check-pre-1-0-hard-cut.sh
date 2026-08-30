#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

product_version_pattern='(?:pub(?:\([^)]*\))?[[:space:]]+)?const[[:space:]]+[A-Z][A-Z0-9_]*(?:SCHEMA|PROTOCOL|MANIFEST|FORMAT|WIRE|CONFIG)[A-Z0-9_]*VERSION[A-Z0-9_]*[[:space:]]*:[^=;]+=[[:space:]]*(?:[2-9][0-9]*|1[0-9]+)[[:space:]]*;|\b(?:schema_version|protocol_version|manifest_version|format_version|wire_version|config_version)[[:space:]]*:[[:space:]]*(?:[2-9][0-9]*|1[0-9]+)\b|b?"canic(?:/|\.)[^"\n]*(?:/|\.)v(?:[2-9][0-9]*)"'
migration_surface_pattern='\b(?:enum|struct)[[:space:]]+(?:MigrationPolicy|StateMigrationManifest)\b|\b(?:min_supported_version|migration_policy)[[:space:]]*:|\bmigrations[[:space:]]*:[[:space:]]*Vec[[:space:]]*<[[:space:]]*StateMigrationManifest[[:space:]]*>|\bfn[[:space:]]+(?:read|load|decode|parse)_[A-Za-z0-9_]*(?:legacy|migration)[A-Za-z0-9_]*|\bfn[[:space:]]+[A-Za-z0-9_]*(?:legacy|migration)[A-Za-z0-9_]*(?:read|load|decode|parse)[A-Za-z0-9_]*|\b(?:struct|enum|type)[[:space:]]+[A-Za-z0-9_]*(?:Legacy|Migration)[A-Za-z0-9_]*(?:Reader|Loader|Decoder)\b'

scan_forbidden() {
    local root="$1"
    local glob="$2"

    {
        rg --pcre2 --files-with-matches --glob "$glob" "$product_version_pattern" "$root" || true
        rg --pcre2 --files-with-matches --glob "$glob" "$migration_surface_pattern" "$root" || true
    } | sort -u
}

fixture_root="docs/audits/fixtures/pre-1-0-hard-cut"
expected_fixture_matches="$fixture_root/forbidden-migration-reader.txt
$fixture_root/forbidden-product-version.txt"
actual_fixture_matches="$(scan_forbidden "$fixture_root" '*.txt')"

if [[ "$actual_fixture_matches" != "$expected_fixture_matches" ]]; then
    echo "pre-1.0 hard-cut detector fixture mismatch" >&2
    printf 'expected:\n%s\nactual:\n%s\n' \
        "$expected_fixture_matches" "$actual_fixture_matches" >&2
    exit 2
fi

if [[ "${1:-}" == "--self-test" ]]; then
    if [[ $# -ne 1 ]]; then
        echo "usage: scripts/ci/check-pre-1-0-hard-cut.sh [--self-test]" >&2
        exit 2
    fi
    echo "pre-1.0 hard-cut detector fixtures passed"
    exit 0
fi

if [[ $# -ne 0 ]]; then
    echo "usage: scripts/ci/check-pre-1-0-hard-cut.sh [--self-test]" >&2
    exit 2
fi

matches="$({
    scan_forbidden crates '*.rs'
    scan_forbidden canisters '*.rs'
} | sort -u)"

if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches"
    echo "pre-1.0 current product schemas must remain v1 and migration/legacy readers are forbidden" >&2
    exit 1
fi

echo "pre-1.0 hard-cut source guard passed"
