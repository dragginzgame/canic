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
        CANIC_GOVERNED_CASE_FILE="$selection" CANIC_POCKETIC_WORKER="$worker" \
        bash "$ROOT/scripts/ci/run-workspace-tests.sh" native-pocketic "$BINARY" &
    PIDS+=("$!")
done
remaining=("${PIDS[@]}")
while [[ "${#remaining[@]}" -gt 0 ]]; do
    completed=''
    status=0
    wait -n -p completed "${remaining[@]}" || status=$?
    [[ "$status" -eq 0 ]] || exit "$status"
    next=()
    for pid in "${remaining[@]}"; do
        [[ "$pid" == "$completed" ]] || next+=("$pid")
    done
    remaining=("${next[@]}")
done
