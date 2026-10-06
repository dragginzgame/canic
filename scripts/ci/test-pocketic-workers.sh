#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-pocketic-workers.XXXXXX")"
finish() {
    local status=$?
    if [[ "$status" -ne 0 ]]; then
        for log in "$fixture/"*.log; do [[ ! -f "$log" ]] || tail -n 30 "$log" >&2; done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap finish EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/.tmp" "$fixture/bin"
for name in run-pocketic-workers run-pocketic-worker run-workspace-tests workspace-scope cleanup-release-artifacts stop-owned-pocketic-servers; do
    cp "$ROOT/scripts/ci/$name.sh" "$fixture/scripts/ci/"
done
cp "$ROOT/tool-versions.env" "$fixture/"
mkdir -p "$fixture/ci" "$fixture/scripts/ci"
cp "$ROOT/ci/ic-tools.tsv" "$fixture/ci/"
cp "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
printf 'use_native_test_icp() { :; }\n' > "$fixture/scripts/ci/native-icp-lib.sh"
cat > "$fixture/bin/pocket-ic" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$$" >> "$WORKER_FIXTURE_PIDS"
while [[ "$1" != --port-file ]]; do shift; done
printf '%s\n' "$((10000 + $$ % 50000))" > "$2"
exec sleep 60
FAKE
cat > "$fixture/bin/tests" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$$" >> "$WORKER_FIXTURE_PIDS"
printf '%s %s\n' "$CANIC_TEST_SCRATCH" "$CANIC_POCKET_IC_SERVER_URL" >> "$WORKER_FIXTURE_ENV"
printf '[FLEET-MEASURE] worker evidence\n'
printf 'worker progress\n'
while IFS= read -r scenario; do
    printf '%s %s\n' "$CANIC_POCKETIC_WORKER" "$scenario" >> "$WORKER_FIXTURE_EXECUTED"
    case "$scenario" in
        pass*) sleep 0.1 ;;
        slow) sleep 1 ;;
        fail*)
            printf 'FAIL\t%s\n' "$scenario" >> "$CANIC_GOVERNED_REPORT_FILE"
            printf 'assertion failed in %s\nRerun: %s\n' "$scenario" "$scenario" > "${CANIC_GOVERNED_REPORT_FILE%.tsv}.failure.txt"
            echo '[CANIC-TEST:E001] worker fixture failed'
            echo "error: assertion failed in $scenario"
            exit 101 ;;
        crash) exit 101 ;;
        wrong) printf 'FAIL\tunknown\n' > "$CANIC_GOVERNED_REPORT_FILE"; exit 101 ;;
        incomplete) printf 'PASS\t%s\n' "$scenario" > "$CANIC_GOVERNED_REPORT_FILE"; exit 101 ;;
        false-success) printf 'FAIL\t%s\n' "$scenario" > "$CANIC_GOVERNED_REPORT_FILE"; exit 0 ;;
        after-failure) printf 'FAIL\t%s\nPASS\tpass-last\n' "$scenario" > "$CANIC_GOVERNED_REPORT_FILE"; exit 101 ;;
        wait) exec sleep 60 ;;
        *) exit 2 ;;
    esac
    printf 'PASS\t%s\n' "$scenario" >> "$CANIC_GOVERNED_REPORT_FILE"
done < "$CANIC_GOVERNED_CASE_FILE"
echo 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s'
FAKE
cat > "$fixture/bin/cargo" <<'FAKE'
#!/usr/bin/env bash
exit 99
FAKE
chmod +x "$fixture/bin/"*
export POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH"
for scenario in pass left right multiple crash wrong incomplete false-success after-failure interrupt; do
    export WORKER_FIXTURE_PIDS="$fixture/$scenario.pids"
    export WORKER_FIXTURE_ENV="$fixture/$scenario.env"
    export WORKER_FIXTURE_EXECUTED="$fixture/$scenario.executed"
    left=pass right=pass
    case "$scenario" in
        left) left=$'pass-first\nfail-middle\npass-last'; right=slow ;;
        right) left=slow; right=$'fail-first\npass-last' ;;
        multiple) left=$'fail-first\npass-middle\nfail-next\npass-last'; right=fail-other ;;
        interrupt) left="wait"; right="wait" ;;
        pass) ;;
        *) left="$scenario"$'\npass-last'; right=slow ;;
    esac
    printf '%s\n' "$left" > "$fixture/left.cases"
    printf '%s\n' "$right" > "$fixture/right.cases"
    status=0
    if [[ "$scenario" == interrupt ]]; then
        timeout --preserve-status --kill-after=3s 2s bash "$fixture/scripts/ci/run-pocketic-workers.sh" \
            "$fixture/bin/tests" "$fixture/left.cases" "$fixture/right.cases" > "$fixture/$scenario.log" 2>&1 || status=$?
    else
        timeout 10s bash "$fixture/scripts/ci/run-pocketic-workers.sh" \
            "$fixture/bin/tests" "$fixture/left.cases" "$fixture/right.cases" > "$fixture/$scenario.log" 2>&1 || status=$?
    fi
    if [[ "$scenario" == pass ]]; then
        [[ "$status" -eq 0 ]]
        rg -q '^\[worker 1\] worker progress$' "$fixture/$scenario.log"
        rg -q '^\[worker 2\] worker progress$' "$fixture/$scenario.log"
        if rg -q '\[FLEET-MEASURE\]' "$fixture/$scenario.log"; then exit 1; fi
    else
        [[ "$status" -ne 0 && "$status" -ne 124 && "$status" -ne 137 ]]
    fi
    if [[ "$scenario" == left || "$scenario" == right || "$scenario" == multiple ]]; then
        rg -q '^\[CANIC-TEST:E001\] worker fixture failed$' "$fixture/$scenario.log"
        sed -n '/==> retained worker failures/,$p' "$fixture/$scenario.log" > "$fixture/failures"
        while read -r _ case_name; do
            [[ "$case_name" != fail* ]] || rg -q "^Rerun: $case_name$" "$fixture/failures"
        done < "$WORKER_FIXTURE_EXECUTED"
    fi
    if [[ "$scenario" == pass || "$scenario" == left || "$scenario" == right || "$scenario" == multiple ]]; then
        {
            sed 's/^/1 /' "$fixture/left.cases"
            sed 's/^/2 /' "$fixture/right.cases"
        } | sort > "$fixture/expected"
        sort "$WORKER_FIXTURE_EXECUTED" > "$fixture/actual"
        diff -u "$fixture/expected" "$fixture/actual"
    elif [[ "$scenario" != interrupt ]]; then
        rg -q 'invalid or incomplete case outcomes' "$fixture/$scenario.log"
        if rg -q 'pass-last' "$WORKER_FIXTURE_EXECUTED"; then exit 1; fi
        rg -q '^2 slow$' "$WORKER_FIXTURE_EXECUTED"
    fi
    # Every resumed process owns a different scratch and server, and no case is retried.
    [[ "$(sort -u "$WORKER_FIXTURE_ENV" | wc -l)" -eq "$(wc -l < "$WORKER_FIXTURE_ENV")" ]]
    [[ "$(sort -u "$WORKER_FIXTURE_EXECUTED" | wc -l)" -eq "$(wc -l < "$WORKER_FIXTURE_EXECUTED")" ]]
    while read -r pid; do
        if [[ -r "/proc/$pid/stat" ]]; then
            # A killed orphan may briefly await its OS reaper; it must not run.
            state="$(awk '{print $3}' "/proc/$pid/stat")"
            [[ "$state" == Z ]] || { echo "worker left process $pid ($state) alive" >&2; exit 1; }
        fi
    done < "$WORKER_FIXTURE_PIDS"
    if compgen -G "$fixture/.tmp/test-runtime.*" >/dev/null; then exit 1; fi
    rg -q '\[FLEET-MEASURE\] worker evidence' "$fixture/target/test-runs"
done
echo 'isolated workers: complete outcomes, fresh continuation, invalid reports, interruption and cleanup passed'
