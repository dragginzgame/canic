#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
FAILED_CHECKS=()

run_check() {
    local label="$1"
    shift
    local output=""

    echo "==> $label"
    if ! output="$(cargo check --keep-going "$@" 2>&1)"; then
        printf '%s\n' "$output" >&2
        FAILED_CHECKS+=("$label")
        echo "==> $label failed" >&2
        return
    fi
    echo "==> $label passed"
}

run_check \
    "control-plane minimal feature build" \
    --locked -p canic-control-plane --no-default-features
run_check \
    "control-plane Fleet Coordinator feature build" \
    --locked -p canic-control-plane --no-default-features --features fleet-coordinator-canister
run_check \
    "control-plane Fleet Subnet Root feature build" \
    --locked -p canic-control-plane --no-default-features --features root-control-plane
run_check \
    "control-plane wasm-store feature build" \
    --locked -p canic-control-plane --no-default-features --features wasm-store-canister
run_check \
    "host control-plane consumer build" \
    --locked -p canic-host

if [[ ${#FAILED_CHECKS[@]} -ne 0 ]]; then
    printf 'CONTROL-PLANE FEATURE MATRIX FAILED: %s\n' "${FAILED_CHECKS[*]}" >&2
    exit 1
fi

echo "CONTROL-PLANE FEATURE MATRIX PASSED: all requested profiles succeeded."
