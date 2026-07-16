#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GATE="$ROOT/scripts/ci/check-dependency-risk-inventory.sh"

fail() {
    echo "dependency risk gate test failed: $1" >&2
    exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is unavailable"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
base="$tmp_dir/base.json"

(
    cd "$ROOT"
    cargo audit --no-fetch --json
) >"$base"

bash "$GATE" --audit-json "$base" >/dev/null

vulnerability="$tmp_dir/vulnerability.json"
jq '.vulnerabilities.found = true | .vulnerabilities.count = 1 | .vulnerabilities.list = [{}]' \
    "$base" >"$vulnerability"
if bash "$GATE" --audit-json "$vulnerability" >/dev/null 2>&1; then
    fail "known vulnerability fixture was accepted"
fi

new_warning="$tmp_dir/new-warning.json"
jq '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0001"
    | .package.name = "unexpected-package"
    | .package.version = "1.0.0"
    | .package.checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")]' \
    "$base" >"$new_warning"
if bash "$GATE" --audit-json "$new_warning" >/dev/null 2>&1; then
    fail "new informational advisory fixture was accepted"
fi

missing_warning="$tmp_dir/missing-warning.json"
jq '.warnings.unmaintained |= .[1:]' "$base" >"$missing_warning"
if bash "$GATE" --audit-json "$missing_warning" >/dev/null 2>&1; then
    fail "stale inventory fixture was accepted"
fi

identity_drift="$tmp_dir/identity-drift.json"
jq '.warnings.unmaintained[0].package.version = "9.9.9"' "$base" >"$identity_drift"
if bash "$GATE" --audit-json "$identity_drift" >/dev/null 2>&1; then
    fail "package identity drift fixture was accepted"
fi

echo "dependency risk gate tests passed"
