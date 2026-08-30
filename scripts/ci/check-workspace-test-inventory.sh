#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INVENTORY="$ROOT/scripts/ci/workspace-test-inventory.tsv"

fail() {
    echo "workspace test inventory guard failed: $1" >&2
    exit 1
}

[ -f "$INVENTORY" ] || fail "missing inventory: $INVENTORY"

expected_header=$'package\ttarget\trelease_lane\texecution\tsuite'
IFS= read -r actual_header <"$INVENTORY"
[ "$actual_header" = "$expected_header" ] || fail "invalid inventory header"

declare -A seen=()
entry_count=0
fast_count=0
parallel_count=0
pocketic_count=0
line_number=1
while IFS=$'\t' read -r package target release_lane execution suite extra; do
    line_number=$((line_number + 1))
    [ -n "$package" ] && [ -n "$target" ] && [ -n "$release_lane" ] &&
        [ -n "$execution" ] && [ -n "$suite" ] && [ -z "${extra:-}" ] ||
        fail "line $line_number must contain exactly five nonempty tab-separated fields"

    key="$package/$target"
    [ -z "${seen[$key]:-}" ] || fail "duplicate target: $key"
    seen[$key]=1
    [ -f "$ROOT/crates/$package/tests/$target.rs" ] ||
        fail "inventoried target does not exist: crates/$package/tests/$target.rs"

    case "$release_lane" in
        fast)
            fast_count=$((fast_count + 1))
            ;;
        full) ;;
        *) fail "line $line_number has invalid release lane: $release_lane" ;;
    esac

    case "$execution/$suite" in
        parallel/ordinary)
            parallel_count=$((parallel_count + 1))
            ;;
        pocketic-serial/runtime | pocketic-serial/blob-storage | pocketic-serial/payload-limits | pocketic-serial/instruction-audit)
            [ "$package" = "canic-tests" ] ||
                fail "PocketIC target must belong to canic-tests: $key"
            pocketic_count=$((pocketic_count + 1))
            ;;
        *) fail "line $line_number has invalid execution/suite classification: $execution/$suite" ;;
    esac

    if [ "$release_lane" = "fast" ] && [ "$execution" != "parallel" ]; then
        fail "fast-lane target must be parallel-safe: $key"
    fi
    entry_count=$((entry_count + 1))
done < <(tail -n +2 "$INVENTORY")

[ "$entry_count" -gt 0 ] || fail "inventory is empty"
[ "$fast_count" -gt 0 ] || fail "inventory has no fast-lane targets"
[ "$parallel_count" -gt 0 ] || fail "inventory has no parallel-safe targets"
[ "$pocketic_count" -gt 0 ] || fail "inventory has no serial PocketIC targets"

discover_integration_targets() {
    local package target test_file tests_dir
    for tests_dir in "$ROOT"/crates/*/tests; do
        [ -d "$tests_dir" ] || continue
        package="$(basename "$(dirname "$tests_dir")")"
        for test_file in "$tests_dir"/*.rs; do
            [ -f "$test_file" ] || continue
            target="$(basename "$test_file" .rs)"
            printf '%s\t%s\n' "$package" "$target"
        done
    done | LC_ALL=C sort
}

discovered="$(discover_integration_targets)"
inventoried="$(awk -F '\t' 'NR > 1 { print $1 "\t" $2 }' "$INVENTORY" | LC_ALL=C sort)"
if [ "$inventoried" != "$discovered" ]; then
    diff -u <(printf '%s\n' "$discovered") <(printf '%s\n' "$inventoried") || true
    fail "every top-level integration test must appear exactly once"
fi

if rg -n -U \
    '(?s)PocketIcBuilder::new\(\).{0,250}\.build\(\)' \
    "$ROOT/crates/canic-host" \
    "$ROOT/crates/canic-testing-internal" \
    "$ROOT/crates/canic-tests" \
    --glob '*.rs'; then
    fail "PocketIC builders must use the bounded explicit startup helper"
fi

echo "workspace test inventory guard passed ($entry_count targets: $parallel_count parallel, $pocketic_count serial PocketIC)"
