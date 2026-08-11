#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODE="${1:-}"

cd "$ROOT" || exit 1

case "$MODE" in
    patch | minor)
        gate_targets=(test-bump)
        ;;
    major)
        gate_targets=(control-plane-feature-gate clippy test)
        ;;
    *)
        echo "usage: $0 <patch|minor|major>" >&2
        exit 2
        ;;
esac

gate_status=0

# shellcheck disable=SC2329 # Invoked by the EXIT trap.
finish() {
    local command_status="$?"
    local cleanup_status

    trap - EXIT INT TERM
    if [[ "$gate_status" -eq 0 && "$command_status" -eq 0 ]]; then
        bash scripts/ci/cleanup-release-artifacts.sh
        cleanup_status=$?
    else
        echo "==> retaining Cargo build artifacts after unsuccessful release gates"
        cleanup_status=0
    fi

    if [[ "$gate_status" -ne 0 ]]; then
        exit "$gate_status"
    fi

    if [[ "$command_status" -ne 0 ]]; then
        exit "$command_status"
    fi

    exit "$cleanup_status"
}

# shellcheck disable=SC2329 # Invoked by the signal traps.
interrupt() {
    gate_status="$1"
    exit "$gate_status"
}

trap finish EXIT
trap 'interrupt 130' INT
trap 'interrupt 143' TERM

unset CANIC_TEST_SCRATCH
bash scripts/ci/run-with-test-scratch.sh make "${gate_targets[@]}"
gate_status=$?
exit "$gate_status"
