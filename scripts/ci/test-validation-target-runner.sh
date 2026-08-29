#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-validation-runner-test.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/failure-logs"
cp "$ROOT/scripts/ci/run-validation-targets.sh" "$FIXTURE/scripts/ci/"
printf '%s\n' \
    '.PHONY: pass mutate-runner fail-one fail-two' \
    'pass:' \
    $'\t@echo pass-marker' \
    'mutate-runner:' \
    $'\t@printf "for broken do\\n" > scripts/ci/run-validation-targets.sh' \
    $'\t@echo mutation-marker' \
    'fail-one:' \
    $'\t@echo "error: first-live-error-marker"' \
    $'\t@echo first-failure-marker' \
    $'\t@exit 7' \
    'fail-two:' \
    $'\t@echo "error: second-live-error-marker"' \
    $'\t@echo second-failure-marker' \
    $'\t@exit 9' >"$FIXTURE/Makefile"

status=0
CANIC_VALIDATION_FAILURE_LOG_DIR="$FIXTURE/failure-logs" \
    bash "$FIXTURE/scripts/ci/run-validation-targets.sh" \
    pass mutate-runner fail-one fail-two >"$FIXTURE/output.log" 2>&1 || status=$?

[[ "$status" -eq 1 ]] || {
    echo "validation target runner test failed: expected status 1, got $status" >&2
    exit 1
}
for expected in \
    'pass-marker' \
    'mutation-marker' \
    'first-failure-marker' \
    'second-failure-marker' \
    '[ERR:fail-one] error: first-live-error-marker' \
    '[ERR:fail-two] error: second-live-error-marker' \
    '[ERR:fail-one] Target failed' \
    '[ERR:fail-one] first-failure-marker' \
    '[ERR:fail-two] Target failed' \
    '[ERR:fail-two] second-failure-marker' \
    '[ERR:summary] Latest highlighted errors:' \
    '[ERR:summary] VALIDATION FAILED: fail-one fail-two' \
    'PASS' \
    'FAIL' \
    'VALIDATION FAILED: fail-one fail-two'; do
    rg -F "$expected" "$FIXTURE/output.log" >/dev/null || {
        echo "validation target runner test failed: missing output: $expected" >&2
        exit 1
    }
done
for expected in \
    '[ERR:fail-one] error: first-live-error-marker' \
    '[ERR:fail-two] error: second-live-error-marker'; do
    [[ "$(rg -F -c "$expected" "$FIXTURE/output.log")" -ge 3 ]] || {
        echo "validation target runner test failed: live error was not highlighted before both summaries" >&2
        exit 1
    }
done
[[ -s "$FIXTURE/failure-logs/latest.log" ]] || {
    echo "validation target runner test failed: latest failure log was not retained" >&2
    exit 1
}
if rg -F '[ERR:' "$FIXTURE/failure-logs/latest.log" >/dev/null; then
    echo "validation target runner test failed: raw failure log was decorated" >&2
    exit 1
fi
[[ -s "$FIXTURE/failure-logs/latest-errors.log" ]] || {
    echo "validation target runner test failed: highlighted failure log was not retained" >&2
    exit 1
}
for expected in \
    '[ERR:fail-one] first-failure-marker' \
    '[ERR:fail-two] second-failure-marker'; do
    rg -F "$expected" "$FIXTURE/failure-logs/latest-errors.log" >/dev/null || {
        echo "validation target runner test failed: highlighted log omits: $expected" >&2
        exit 1
    }
done

echo "validation target runner test passed"
