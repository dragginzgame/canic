#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-validation-runner-test.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/failure-logs"
cp "$ROOT/scripts/ci/run-validation-targets.sh" "$FIXTURE/scripts/ci/"
printf '%s\n' \
    '.PHONY: pass fail-one fail-two' \
    'pass:' \
    $'\t@echo pass-marker' \
    'fail-one:' \
    $'\t@echo first-failure-marker' \
    $'\t@exit 7' \
    'fail-two:' \
    $'\t@echo second-failure-marker' \
    $'\t@exit 9' >"$FIXTURE/Makefile"

status=0
CANIC_VALIDATION_FAILURE_LOG_DIR="$FIXTURE/failure-logs" \
    bash "$FIXTURE/scripts/ci/run-validation-targets.sh" \
    pass fail-one fail-two >"$FIXTURE/output.log" 2>&1 || status=$?

[[ "$status" -eq 1 ]] || {
    echo "validation target runner test failed: expected status 1, got $status" >&2
    exit 1
}
for expected in \
    'pass-marker' \
    'first-failure-marker' \
    'second-failure-marker' \
    'PASS' \
    'FAIL' \
    'VALIDATION FAILED: fail-one fail-two'; do
    rg -F "$expected" "$FIXTURE/output.log" >/dev/null || {
        echo "validation target runner test failed: missing output: $expected" >&2
        exit 1
    }
done
[[ -s "$FIXTURE/failure-logs/latest.log" ]] || {
    echo "validation target runner test failed: latest failure log was not retained" >&2
    exit 1
}

echo "validation target runner test passed"
