#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

run_check() {
    local label="$1"
    shift
    local output=""

    echo "==> $label"
    if ! output="$(cargo check "$@" 2>&1)"; then
        printf '%s\n' "$output" >&2
        return 1
    fi
}

run_check \
    "control-plane minimal feature build" \
    --locked -p canic-control-plane --no-default-features
run_check \
    "control-plane wasm-store feature build" \
    --locked -p canic-control-plane --no-default-features --features wasm-store-canister
run_check \
    "host control-plane consumer build" \
    --locked -p canic-host
