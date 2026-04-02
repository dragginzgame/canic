#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/app/reference_canisters.sh"

# Build debug artifacts by default so PocketIC/test harnesses can keep using the
# matching `.did` extraction path unless a caller explicitly overrides it.
BUILD_RELEASE="${RELEASE:-0}"

# Keep PocketIC-oriented CI artifacts small without changing the debug-profile
# semantics that the test harness currently expects.
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

# Build the ordinary reference artifacts first so the thin-root manifest path
# can emit once the full root-subnet release set exists. Root itself builds the
# hidden bootstrap `wasm_store` artifact internally.
for canister in "${REFERENCE_CANISTERS[@]}"; do
    RELEASE="$BUILD_RELEASE" scripts/app/canic_installer.sh canic-build-canister-artifact "$canister"
done

ROOT_WASM_GZ_PATH=".dfx/local/canisters/root/root.wasm.gz"
ROOT_WASM_GZ_BYTES="$(stat -c%s "$ROOT_WASM_GZ_PATH")"
if [ "$ROOT_WASM_GZ_BYTES" -ge 100000000 ]; then
    echo "root.wasm.gz too large for PocketIC chunk store: ${ROOT_WASM_GZ_BYTES} bytes" >&2
    exit 1
fi
