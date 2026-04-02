#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

# Keep PocketIC-oriented CI artifacts small without changing the debug-profile
# semantics that the test harness currently expects.
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

# Build the ordinary reference artifacts first so the thin-root manifest path
# can emit once the full root-subnet release set exists. Root itself builds the
# hidden bootstrap `wasm_store` artifact internally.
REFERENCE_CANISTERS=(
    app
    minimal
    scale
    scale_hub
    test
    user_hub
    user_shard
    root
)

for canister in "${REFERENCE_CANISTERS[@]}"; do
    RELEASE=0 bash scripts/app/build.sh "$canister"
done

ROOT_WASM_GZ_PATH=".dfx/local/canisters/root/root.wasm.gz"
ROOT_WASM_GZ_BYTES="$(stat -c%s "$ROOT_WASM_GZ_PATH")"
if [ "$ROOT_WASM_GZ_BYTES" -ge 100000000 ]; then
    echo "root.wasm.gz too large for PocketIC chunk store: ${ROOT_WASM_GZ_BYTES} bytes" >&2
    exit 1
fi
