#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/app/reference_canisters.sh"

# Build the middle fast artifacts by default so PocketIC/test harnesses and
# local demo flows get smaller faster wasm without paying full release cost.
BUILD_WASM_PROFILE="${CANIC_WASM_PROFILE:-}"
if [ -z "$BUILD_WASM_PROFILE" ]; then
    BUILD_WASM_PROFILE="fast"
fi

# Keep PocketIC-oriented CI artifacts small.
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

# Build the ordinary reference artifacts first so the thin-root manifest path
# can emit once the full root-subnet release set exists. Root itself builds the
# hidden bootstrap `wasm_store` artifact internally.
for canister in "${REFERENCE_CANISTERS[@]}"; do
    CANIC_WASM_PROFILE="$BUILD_WASM_PROFILE" scripts/app/canic_installer.sh canic-build-canister-artifact "$canister"
done

ROOT_WASM_GZ_PATH=".dfx/local/canisters/root/root.wasm.gz"
ROOT_WASM_GZ_BYTES="$(stat -c%s "$ROOT_WASM_GZ_PATH")"
if [ "$ROOT_WASM_GZ_BYTES" -ge 100000000 ]; then
    echo "root.wasm.gz too large for PocketIC chunk store: ${ROOT_WASM_GZ_BYTES} bytes" >&2
    exit 1
fi
