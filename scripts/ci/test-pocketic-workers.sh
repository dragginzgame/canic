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
for name in run-pocketic-workers run-workspace-tests cleanup-release-artifacts stop-owned-pocketic-servers; do
    cp "$ROOT/scripts/ci/$name.sh" "$fixture/scripts/ci/"
done
cp "$ROOT/tool-versions.env" "$fixture/"
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
read -r scenario < "$CANIC_GOVERNED_CASE_FILE"
printf '%s %s\n' "$CANIC_TEST_SCRATCH" "$CANIC_POCKET_IC_SERVER_URL" >> "$WORKER_FIXTURE_ENV"
printf '[FLEET-MEASURE] worker evidence\n'
printf 'worker progress\n'
case "$scenario" in
    pass) sleep 0.2 ;;
    fail) sleep 0.5; echo '[CANIC-TEST:E001] worker fixture failed'; echo 'error: worker fixture failed'; exit 1 ;;
    wait) exec sleep 60 ;;
    *) exit 2 ;;
esac
FAKE
cat > "$fixture/bin/cargo" <<'FAKE'
#!/usr/bin/env bash
exit 99
FAKE
chmod +x "$fixture/bin/"*
export POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH"
for scenario in pass left right interrupt; do
    export WORKER_FIXTURE_PIDS="$fixture/$scenario.pids"
    export WORKER_FIXTURE_ENV="$fixture/$scenario.env"
    left=pass right=pass
    case "$scenario" in
        left) left=fail; right="wait" ;;
        right) left="wait"; right=fail ;;
        interrupt) left="wait"; right="wait" ;;
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
    if [[ "$scenario" == left || "$scenario" == right ]]; then
        rg -q '^\[CANIC-TEST:E001\] worker fixture failed$' "$fixture/$scenario.log"
    fi
    [[ "$(sort -u "$WORKER_FIXTURE_ENV" | wc -l)" -eq 2 ]]
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
echo 'isolated worker success, failure cancellation, interruption, logs and cleanup passed'
