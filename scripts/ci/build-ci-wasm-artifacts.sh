#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/app/reference_canisters.sh"

require_cmd() {
    local cmd="$1"

    if command -v "$cmd" >/dev/null 2>&1; then
        return 0
    fi

    echo "missing required build tool '$cmd'" >&2
    echo "run: bash scripts/dev/install_dev.sh" >&2
    exit 1
}

require_cmd cargo
require_cmd candid-extractor
require_cmd ic-wasm

# Build the middle fast artifacts by default so PocketIC/test harnesses and
# local demo flows get smaller faster wasm without paying full release cost.
BUILD_WASM_PROFILE="${CANIC_WASM_PROFILE:-}"
if [ -z "$BUILD_WASM_PROFILE" ]; then
    BUILD_WASM_PROFILE="fast"
fi

# Keep PocketIC-oriented CI artifacts small.
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

BUILD_CANISTERS=("${REFERENCE_CANISTERS[@]}")
if [ -n "${CANIC_REFERENCE_CANISTERS:-}" ]; then
    # Allow focused harnesses to build only the canisters they actually stage.
    read -r -a BUILD_CANISTERS <<<"$CANIC_REFERENCE_CANISTERS"
fi

NON_ROOT_CANISTERS=()
BUILD_ROOT=0
for canister in "${BUILD_CANISTERS[@]}"; do
    if [ "$canister" = "root" ]; then
        BUILD_ROOT=1
    else
        NON_ROOT_CANISTERS+=("$canister")
    fi
done

# Build the ordinary reference artifacts first so the thin-root manifest path
# can emit once the full root-subnet release set exists. Root itself builds the
# hidden bootstrap `wasm_store` artifact internally.
for canister in "${NON_ROOT_CANISTERS[@]}"; do
    CANIC_WASM_PROFILE="$BUILD_WASM_PROFILE" scripts/app/canic_installer.sh canic-build-canister-artifact "$canister"
done

if [ "$BUILD_ROOT" -eq 1 ]; then
    CANIC_WASM_PROFILE="$BUILD_WASM_PROFILE" scripts/app/canic_installer.sh canic-build-canister-artifact root

    ROOT_WASM_GZ_PATH=".dfx/local/canisters/root/root.wasm.gz"
    ROOT_WASM_GZ_BYTES="$(stat -c%s "$ROOT_WASM_GZ_PATH")"
    if [ "$ROOT_WASM_GZ_BYTES" -ge 100000000 ]; then
        echo "root.wasm.gz too large for PocketIC chunk store: ${ROOT_WASM_GZ_BYTES} bytes" >&2
        exit 1
    fi
fi
