#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GATE="$ROOT/scripts/ci/check-dependency-risk-inventory.sh"

fail() {
    echo "dependency risk gate test failed: $1" >&2
    exit 1
}

command -v git >/dev/null 2>&1 || fail "git is unavailable"
command -v jq >/dev/null 2>&1 || fail "jq is unavailable"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
base="$tmp_dir/base.json"
audit_db="$tmp_dir/advisory-db"

# The gate validates dependency ownership offline. Populate the exact locked
# graph first so this test is independent of a runner's Cargo cache state.
(
    cd "$ROOT"
    cargo fetch --locked
)

if ! (
    cd "$ROOT"
    cargo audit --db "$audit_db" --json
) >"$base"; then
    [ -s "$base" ] || fail "cargo audit did not produce advisory JSON"
fi

bash "$GATE" --audit-json "$base" >/dev/null

# A shared cache may retain an untracked copy after an upstream advisory is
# renamed or moved. Offline isolation must copy only the selected tracked
# database revision so the stale file cannot create a duplicate advisory ID.
tracked_advisory="$(git -C "$audit_db" ls-files 'crates/*/RUSTSEC-*.md' | sed -n '1p')"
[ -n "$tracked_advisory" ] || fail "fresh advisory database has no tracked advisory"
mkdir -p "$audit_db/crates/canic-stale-duplicate"
cp "$audit_db/$tracked_advisory" \
    "$audit_db/crates/canic-stale-duplicate/${tracked_advisory##*/}"
CANIC_CARGO_AUDIT_NO_FETCH=1 CANIC_CARGO_AUDIT_DB="$audit_db" \
    bash "$GATE" >/dev/null

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
bash "$GATE" --audit-json "$new_warning" >/dev/null 2>&1 ||
    fail "new transitive informational advisory fixture was rejected"

missing_warning="$tmp_dir/missing-warning.json"
jq '.warnings.unmaintained |= .[1:]' "$base" >"$missing_warning"
bash "$GATE" --audit-json "$missing_warning" >/dev/null 2>&1 ||
    fail "removed transitive informational advisory fixture was rejected"

identity_drift="$tmp_dir/identity-drift.json"
jq '.warnings.unmaintained[0].package.version = "9.9.9"' "$base" >"$identity_drift"
bash "$GATE" --audit-json "$identity_drift" >/dev/null 2>&1 ||
    fail "transitive informational package identity drift fixture was rejected"

direct_warning="$tmp_dir/direct-warning.json"
jq '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0002"
    | .package.name = "serde"
    | .package.version = "1.0.0")]' \
    "$base" >"$direct_warning"
if bash "$GATE" --audit-json "$direct_warning" >/dev/null 2>&1; then
    fail "unmaintained direct dependency fixture was accepted"
fi

yanked_warning="$tmp_dir/yanked-warning.json"
jq '.warnings.yanked = [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0003"
    | .kind = "yanked"
    | .package.name = "transitive-yanked-package")]' \
    "$base" >"$yanked_warning"
if bash "$GATE" --audit-json "$yanked_warning" >/dev/null 2>&1; then
    fail "yanked dependency fixture was accepted"
fi

echo "dependency risk gate tests passed"
