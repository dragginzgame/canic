#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-validation-runner-test.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/failure-logs"
cp "$ROOT/scripts/ci/run-validation-targets.sh" "$FIXTURE/scripts/ci/"
printf '%s\n' \
    '.PHONY: pass mutate-runner fail-coded fail-one fail-two fail-after-caught-panic' \
    'pass:' \
    $'\t@echo pass-marker' \
    $'\t@echo "test api::error::tests::public_error_is_preserved_without_remap ... ok"' \
    $'\t@echo "test ops::auth::error::tests::optional_case ... ignored"' \
    'mutate-runner:' \
    $'\t@printf "for broken do\\n" > scripts/ci/run-validation-targets.sh' \
    $'\t@echo mutation-marker' \
    'fail-coded:' \
    $'\t@echo "[CANIC-TEST:E001] [SUITE] FAIL stable-failure-event"' \
    $'\t@exit 6' \
    'fail-one:' \
    $'\t@echo "error: first-live-error-marker"' \
    $'\t@echo first-failure-marker' \
    $'\t@exit 7' \
    'fail-two:' \
    $'\t@echo "error: second-live-error-marker"' \
    $'\t@echo "error[E0425]: coded-error-marker"' \
    $'\t@echo second-failure-marker' \
    $'\t@exit 9' \
    'fail-after-caught-panic:' \
    $'\t@echo "thread '\''caught-test'\'' panicked at fake.rs:1:1:"' \
    $'\t@echo "caught panic marker"' \
    $'\t@echo "test caught-test ... ok"' \
    $'\t@echo ordinary-context-one' \
    $'\t@echo ordinary-context-two' \
    $'\t@echo ordinary-context-three' \
    $'\t@echo ordinary-context-four' \
    $'\t@echo ordinary-context-five' \
    $'\t@echo "test actual-test ... FAILED"' \
    $'\t@exit 11' >"$FIXTURE/Makefile"

status=0
CANIC_VALIDATION_FAILURE_LOG_DIR="$FIXTURE/failure-logs" \
    CANIC_VALIDATION_LOG_DIR="$FIXTURE/target/validation-runs" \
    CANIC_VALIDATION_ROOT="$FIXTURE" \
    CANIC_VALIDATION_RUNNER_DEPTH=0 \
    CANIC_VALIDATION_RUNNER_SNAPSHOT_PATH='' \
    bash "$FIXTURE/scripts/ci/run-validation-targets.sh" \
    pass mutate-runner fail-coded fail-one fail-two fail-after-caught-panic \
    >"$FIXTURE/output.log" 2>&1 || status=$?

[[ "$status" -eq 1 ]] || {
    echo "validation target runner test failed: expected status 1, got $status" >&2
    exit 1
}
for expected in \
    'pass-marker' \
    'test api::error::tests::public_error_is_preserved_without_remap ... ok' \
    'test ops::auth::error::tests::optional_case ... ignored' \
    '[ERR:fail-two] error[E0425]: coded-error-marker' \
    'mutation-marker' \
    '[ERR:fail-coded] [CANIC-TEST:E001] [SUITE] FAIL stable-failure-event' \
    'first-failure-marker' \
    'second-failure-marker' \
    "thread 'caught-test' panicked at fake.rs:1:1:" \
    'test caught-test ... ok' \
    '[ERR:fail-one] error: first-live-error-marker' \
    '[ERR:fail-two] error: second-live-error-marker' \
    '[ERR:fail-after-caught-panic] test actual-test ... FAILED' \
    '[ERR:fail-one] Target failed' \
    '[ERR:fail-one] first-failure-marker' \
    '[ERR:fail-two] Target failed' \
    '[ERR:fail-two] second-failure-marker' \
    '[ERR:fail-after-caught-panic] Target failed' \
    '[ERR:summary] Latest highlighted errors:' \
    '[ERR:summary] VALIDATION FAILED: fail-coded fail-one fail-two fail-after-caught-panic' \
    'PASS' \
    'FAIL' \
    'VALIDATION FAILED: fail-coded fail-one fail-two fail-after-caught-panic'; do
    rg -F "$expected" "$FIXTURE/output.log" >/dev/null || {
        echo "validation target runner test failed: missing output: $expected" >&2
        exit 1
    }
done
if rg '^\[ERR:pass\]' "$FIXTURE/output.log" >/dev/null; then
    echo "validation target runner test failed: passing/ignored error-module tests were highlighted" >&2
    exit 1
fi
if rg -F "[ERR:fail-after-caught-panic] thread 'caught-test' panicked at" \
    "$FIXTURE/output.log" >/dev/null; then
    echo "validation target runner test failed: caught panic was highlighted" >&2
    exit 1
fi
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
    '[ERR:fail-coded] [CANIC-TEST:E001] [SUITE] FAIL stable-failure-event' \
    '[ERR:fail-one] first-failure-marker' \
    '[ERR:fail-two] second-failure-marker' \
    '[ERR:fail-after-caught-panic] test actual-test ... FAILED'; do
    rg -F "$expected" "$FIXTURE/failure-logs/latest-errors.log" >/dev/null || {
        echo "validation target runner test failed: highlighted log omits: $expected" >&2
        exit 1
    }
done
if rg -F "[ERR:fail-after-caught-panic] thread 'caught-test' panicked at" \
    "$FIXTURE/failure-logs/latest-errors.log" >/dev/null; then
    echo "validation target runner test failed: highlighted log includes caught panic" >&2
    exit 1
fi

timings="$(rg --files "$FIXTURE/target/validation-runs" | rg '/timings.tsv$')"
awk -F '\t' '
    NR == 1 { if ($0 != "target\tresult\tseconds\tlog") exit 1; next }
    $3 !~ /^[0-9]+$/ { exit 1 }
    $1 == "pass" && $2 == "PASS" { passed = 1 }
    $1 == "fail-two" && $2 == "FAIL" { failed = 1 }
    END { if (!passed || !failed) exit 1 }
' "$timings"
pass_log="$(awk -F '\t' '$1 == "pass" { print $4 }' "$timings")"
rg -F 'pass-marker' "$pass_log" >/dev/null

# Successful runs retain raw logs without replacing the latest failure evidence.
cp "$FIXTURE/failure-logs/latest.log" "$FIXTURE/prior-failure.log"
cp "$ROOT/scripts/ci/run-validation-targets.sh" "$FIXTURE/scripts/ci/"
CANIC_VALIDATION_FAILURE_LOG_DIR="$FIXTURE/failure-logs" \
    CANIC_VALIDATION_LOG_DIR="$FIXTURE/success-logs" \
    CANIC_VALIDATION_ROOT="$FIXTURE" \
    bash "$FIXTURE/scripts/ci/run-validation-targets.sh" pass >"$FIXTURE/success.log" 2>&1
success_timings="$(rg --files "$FIXTURE/success-logs" | rg '/timings.tsv$')"
success_log="$(awk -F '\t' '$1 == "pass" && $2 == "PASS" { print $4 }' "$success_timings")"
rg -F 'pass-marker' "$success_log" >/dev/null
cmp "$FIXTURE/prior-failure.log" "$FIXTURE/failure-logs/latest.log"

echo "validation target runner test passed"
