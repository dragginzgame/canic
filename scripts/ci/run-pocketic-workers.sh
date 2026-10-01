#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BINARY="${1:?compiled internal test binary required}"
shift
[[ "$#" -eq 2 && -x "$BINARY" ]] || exit 2
command -v setsid >/dev/null
[[ ! -L "$ROOT/.tmp" ]] || exit 2
mkdir -p "$ROOT/.tmp"
PIDS=()
SCRATCHES=()

# Each worker is a distinct session: cancellation reaches its test, server and
# CLI descendants without signalling the caller or another validation process.
# shellcheck disable=SC2329 # Invoked by the EXIT trap.
finish() {
    local status=$? pid scratch
    trap - EXIT INT TERM
    for pid in "${PIDS[@]}"; do kill -TERM -- "-$pid" 2>/dev/null || true; done
    for pid in "${PIDS[@]}"; do
        kill -KILL -- "-$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
    for scratch in "${SCRATCHES[@]}"; do
        CANIC_TEST_SCRATCH="$scratch" bash "$ROOT/scripts/ci/cleanup-release-artifacts.sh" --scratch-only || status=1
    done
    exit "$status"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

worker=0
for selection in "$@"; do
    worker=$((worker + 1))
    [[ -s "$selection" ]] || exit 2
    scratch="$(mktemp -d "$ROOT/.tmp/test-runtime.XXXXXX")"
    SCRATCHES+=("$scratch")
    setsid env CANIC_TEST_SCRATCH="$scratch" TMPDIR="$scratch" \
        CANIC_POCKETIC_WORKER="$worker" \
        bash "$ROOT/scripts/ci/run-pocketic-worker.sh" "$BINARY" "$selection" &
    PIDS+=("$!")
done
status=0
for pid in "${PIDS[@]}"; do
    wait "$pid" || status=1
done
if [[ "$status" -ne 0 ]]; then
    echo '==> retained worker failures' >&2
    for scratch in "${SCRATCHES[@]}"; do
        for report in "$scratch"/attempt-*/outcomes.failure.txt; do
            [[ ! -f "$report" ]] || { cat "$report" >&2; echo >&2; }
        done
    done
fi
exit "$status"
