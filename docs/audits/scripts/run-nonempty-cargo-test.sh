#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "usage: bash docs/audits/scripts/run-nonempty-cargo-test.sh <cargo-test-args...>" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUTPUT="$(mktemp)"
trap 'rm -f "$OUTPUT"' EXIT

set +e
(
    cd "$ROOT"
    cargo test "$@"
) 2>&1 | tee "$OUTPUT"
cargo_status="${PIPESTATUS[0]}"
set -e

if [[ "$cargo_status" -ne 0 ]]; then
    exit "$cargo_status"
fi

passed="$({
    sed -nE 's/^test result: ok\. ([0-9]+) passed;.*/\1/p' "$OUTPUT"
} | awk '{ total += $1 } END { print total + 0 }')"

if [[ "$passed" -eq 0 ]]; then
    echo "audit test selection executed zero passing tests" >&2
    exit 3
fi

echo "audit test selection executed $passed passing test(s)"
