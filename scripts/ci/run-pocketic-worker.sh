#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BINARY="${1:?compiled internal test binary required}"
selection="${2:?registered case selection required}"
scratch="${CANIC_TEST_SCRATCH:?private worker scratch required}"
attempt=0
failed=0

# The foreground Testkit owner receives the worker-group signal and must finish
# its separate command/server groups before this shell releases the attempt.
trap : INT TERM

# Success keeps the warm process-local fixture. A case panic ends that process;
# only its unexecuted suffix may start on a fresh server and private scratch.
# A crash, invalid report or infrastructure failure never authorizes continuation.
while [[ -s "$selection" ]]; do
    attempt=$((attempt + 1))
    run="$scratch/attempt-$attempt"
    mkdir "$run"
    report="$run/outcomes.tsv"
    status=0
    env -u IC_TESTKIT_POCKET_IC_URL CANIC_TEST_SCRATCH="$run" TMPDIR="$run" \
        CANIC_GOVERNED_CASE_FILE="$selection" CANIC_GOVERNED_REPORT_FILE="$report" \
        bash "$ROOT/scripts/ci/run-workspace-tests.sh" native-pocketic "$BINARY" || status=$?

    next="$run/remaining.cases"
    if [[ ! -s "$report" ]] || ! awk -F '\t' -v status="$status" '
        NR == FNR { expected[++total] = $0; next }
        {
            if (NF != 2 || stopped || $2 != expected[++completed]) exit 1
            if ($1 == "FAIL") stopped = 1
            else if ($1 != "PASS") exit 1
        }
        END {
            if (completed == 0 || completed > total) exit 1
            if (status == 0) {
                if (stopped || completed != total) exit 1
            } else if (status != 101 || !stopped) exit 1
            for (remaining = completed + 1; remaining <= total; remaining++) print expected[remaining]
        }
    ' "$selection" "$report" > "$next"; then
        echo "worker ${CANIC_POCKETIC_WORKER:-?}: invalid or incomplete case outcomes; stopping this worker (exit $status)" >&2
        exit 1
    fi
    if [[ "$status" -ne 0 ]]; then
        failed=1
        if [[ -s "$next" ]]; then
            echo "worker ${CANIC_POCKETIC_WORKER:-?}: continuing unexecuted cases in a fresh process and server" >&2
        fi
    fi
    selection="$next"
done
exit "$failed"
