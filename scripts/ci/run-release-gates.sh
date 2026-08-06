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
    bash scripts/ci/cleanup-release-artifacts.sh
    cleanup_status=$?

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

mkdir -p "$ROOT/.tmp/test-runtime"
export TMPDIR="$ROOT/.tmp/test-runtime"

make "${gate_targets[@]}"
gate_status=$?
exit "$gate_status"
