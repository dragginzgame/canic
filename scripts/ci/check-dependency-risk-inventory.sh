#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INVENTORY="$ROOT/scripts/ci/dependency-risk-inventory.tsv"
TOOLS="$ROOT/tool-versions.env"

fail() {
    echo "dependency risk gate failed: $1" >&2
    exit 1
}

[ -f "$INVENTORY" ] || fail "missing inventory: $INVENTORY"
[ -f "$TOOLS" ] || fail "missing tool-version authority: $TOOLS"
command -v cargo >/dev/null 2>&1 || fail "cargo is unavailable"
command -v jq >/dev/null 2>&1 || fail "jq is unavailable"

# shellcheck source=/dev/null
source "$TOOLS"
audit_version="$(cargo audit --version 2>/dev/null | awk '{print $2}')" ||
    fail "cargo-audit is unavailable"
[ "$audit_version" = "$CANIC_CARGO_AUDIT_VERSION" ] ||
    fail "cargo-audit version mismatch: expected $CANIC_CARGO_AUDIT_VERSION, got ${audit_version:-unavailable}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
audit_json="$tmp_dir/audit.json"

case "$#" in
0)
    audit_args=(--json)
    if [ "${CANIC_CARGO_AUDIT_NO_FETCH:-0}" = "1" ]; then
        audit_args=(--no-fetch --json)
    fi
    (
        cd "$ROOT"
        cargo audit "${audit_args[@]}"
    ) >"$audit_json"
    ;;
2)
    [ "$1" = "--audit-json" ] || fail "usage: $0 [--audit-json <path>]"
    [ -f "$2" ] || fail "audit JSON does not exist: $2"
    cp "$2" "$audit_json"
    ;;
*) fail "usage: $0 [--audit-json <path>]" ;;
esac

jq -e 'type == "object"' "$audit_json" >/dev/null || fail "cargo-audit JSON is invalid"
vulnerability_count="$(jq -r '.vulnerabilities.count // (.vulnerabilities.list | length) // 0' "$audit_json")"
[[ "$vulnerability_count" =~ ^[0-9]+$ ]] || fail "cargo-audit vulnerability count is invalid"
[ "$vulnerability_count" -eq 0 ] || fail "$vulnerability_count known vulnerabilities found"

expected="$tmp_dir/expected.tsv"
actual="$tmp_dir/actual.tsv"
awk -F '\t' 'NF && $1 !~ /^#/ { print $1 "\t" $2 "\t" $3 "\t" $4 "\t" $5 }' "$INVENTORY" |
    LC_ALL=C sort >"$expected"
jq -r '
    (.warnings // {})
    | to_entries[]
    | .value[]
    | [
        .advisory.id,
        .kind,
        .package.name,
        .package.version,
        (.package.checksum // "")
      ]
    | @tsv
' "$audit_json" | LC_ALL=C sort >"$actual"

if ! diff -u "$expected" "$actual" >/dev/null; then
    diff -u "$expected" "$actual" >&2 || true
    fail "informational advisory inventory changed; review additions, removals, and package identities"
fi

metadata="$tmp_dir/metadata.json"
(
    cd "$ROOT"
    cargo metadata --locked --offline --format-version 1
) >"$metadata"

direct_dependencies="$tmp_dir/direct-dependencies.txt"
(
    cd "$ROOT"
    cargo metadata --locked --offline --format-version 1 --no-deps
) | jq -r '.packages[].dependencies[].name' | LC_ALL=C sort -u >"$direct_dependencies"

while IFS=$'\t' read -r advisory_id kind package version _checksum introducers; do
    case "$advisory_id" in
    '' | \#*) continue ;;
    esac
    [ "$kind" = "unmaintained" ] || fail "$advisory_id is not classified as unmaintained"
    if rg -x -F "$package" "$direct_dependencies" >/dev/null; then
        fail "$package is now a direct workspace dependency"
    fi

    package_id="$(jq -r --arg package "$package" --arg version "$version" '
        [.packages[] | select(.name == $package and .version == $version) | .id]
        | if length == 1 then .[0] else empty end
    ' "$metadata")"
    [ -n "$package_id" ] || fail "$advisory_id package identity is absent or ambiguous"

    expected_introducers_path="$tmp_dir/expected-introducers-$package.txt"
    actual_introducers_path="$tmp_dir/actual-introducers-$package.txt"
    IFS=',' read -r -a expected_introducers <<<"$introducers"
    [ "${#expected_introducers[@]}" -gt 0 ] || fail "$advisory_id has no expected introducer"
    for introducer in "${expected_introducers[@]}"; do
        [ -n "$introducer" ] || fail "$advisory_id has an empty introducer"
        printf '%s\n' "$introducer"
    done | LC_ALL=C sort -u >"$expected_introducers_path"
    jq -r --arg package_id "$package_id" '
        [.resolve.nodes[] | select(any(.deps[]?; .pkg == $package_id)) | .id] as $parent_ids
        | .packages[]
        | select(.id as $id | $parent_ids | index($id))
        | .name
    ' "$metadata" | LC_ALL=C sort -u >"$actual_introducers_path"
    if ! diff -u "$expected_introducers_path" "$actual_introducers_path" >/dev/null; then
        diff -u "$expected_introducers_path" "$actual_introducers_path" >&2 || true
        fail "$advisory_id immediate introducer set changed for $package"
    fi
done <"$INVENTORY"

echo "dependency risk gate passed: zero vulnerabilities and 4 exact transitive advisories"
