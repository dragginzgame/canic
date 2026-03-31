#!/usr/bin/env bash

set -euo pipefail

ROOT_CANISTER="${1:-${ROOT_CANISTER:-root}}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NETWORK="${DFX_NETWORK:-local}"

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

echo "Installing reference topology against DFX_NETWORK=${NETWORK}"
require_dfx_running

dfx canister create --all -qq
RELEASE=1 dfx build --all
dfx ledger fabricate-cycles --canister "${ROOT_CANISTER}" --cycles 9000000000000000 || true
dfx canister install "${ROOT_CANISTER}" --mode=reinstall -y --argument '(variant { Prime })'

echo "Reference topology installed successfully"
echo "Smoke check: dfx canister call ${ROOT_CANISTER} canic_ready"
