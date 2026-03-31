#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NETWORK="${DFX_NETWORK:-local}"
READY_TIMEOUT_SECONDS="${READY_TIMEOUT_SECONDS:-120}"

require_dfx_running() {
    if ! command -v dfx >/dev/null 2>&1; then
        echo "dfx is required for reference topology install" >&2
        exit 1
    fi

    if ! dfx ping "${NETWORK}" >/dev/null 2>&1; then
        echo "dfx replica is not running for network '${NETWORK}'" >&2
        echo "Start it in another terminal with scripts/app/dfx_start.sh and rerun." >&2
        exit 1
    fi
}

wait_for_root_ready() {
    local deadline=$((SECONDS + READY_TIMEOUT_SECONDS))
    local start_time=$SECONDS
    local next_report=$SECONDS

    echo "Waiting for ${ROOT_CANISTER} to report canic_ready (timeout ${READY_TIMEOUT_SECONDS}s)"
    while [ "$SECONDS" -lt "$deadline" ]; do
        if dfx canister call "${ROOT_CANISTER}" canic_ready 2>/dev/null | grep -q "true"; then
            echo "${ROOT_CANISTER} reported canic_ready after $((SECONDS - start_time))s"
            return 0
        fi

        if [ "$SECONDS" -ge "$next_report" ]; then
            echo "Still waiting for ${ROOT_CANISTER} canic_ready ($((SECONDS - start_time))s elapsed)"

            if registry_json="$(dfx canister call "${ROOT_CANISTER}" canic_subnet_registry --output json 2>/dev/null)"; then
                echo "Current subnet registry roles:"
                printf '%s\n' "${registry_json}" | python3 -c 'import json,sys; data=json.load(sys.stdin); roles=[entry.get("role","?") for entry in data.get("Ok", [])]; print("  " + (", ".join(roles) if roles else "<empty>"))'
            fi

            echo "Recent root logs:"
            dfx canister logs "${ROOT_CANISTER}" 2>/dev/null | tail -n 8 || true
            next_report=$((SECONDS + 5))
        fi

        sleep 1
    done

    echo "root did not report canic_ready within ${READY_TIMEOUT_SECONDS}s" >&2
    echo "Diagnostic: dfx canister call ${ROOT_CANISTER} canic_subnet_registry" >&2
    dfx canister call "${ROOT_CANISTER}" canic_subnet_registry >&2 || true
    echo "Diagnostic: dfx canister call ${ROOT_CANISTER} canic_wasm_store_bootstrap_debug" >&2
    dfx canister call "${ROOT_CANISTER}" canic_wasm_store_bootstrap_debug >&2 || true
    echo "Diagnostic: dfx canister call ${ROOT_CANISTER} canic_wasm_store_overview" >&2
    dfx canister call "${ROOT_CANISTER}" canic_wasm_store_overview >&2 || true
    echo "Diagnostic: dfx canister logs ${ROOT_CANISTER}" >&2
    dfx canister logs "${ROOT_CANISTER}" >&2 || true
    return 1
}

echo "Installing reference topology against DFX_NETWORK=${NETWORK}"
require_dfx_running

dfx canister create --all -qq
RELEASE=1 dfx build --all
dfx ledger fabricate-cycles --canister "${ROOT_CANISTER}" --cycles 9000000000000000 || true
dfx canister install "${ROOT_CANISTER}" --mode=reinstall -y --argument '(variant { Prime })'
wait_for_root_ready

echo "Reference topology installed successfully"
echo "Smoke check: dfx canister call ${ROOT_CANISTER} canic_ready"
