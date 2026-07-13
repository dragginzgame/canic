#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/ci/require_icp.sh"

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
require_icp_tools

if [ "$#" -ne 2 ]; then
    echo "usage: $0 <debug|fast|release> <config-path>" >&2
    exit 2
fi
BUILD_WASM_PROFILE="$1"
BUILD_CONFIG="$2"
case "$BUILD_WASM_PROFILE" in
    debug|fast|release) ;;
    *)
        echo "invalid wasm profile '$BUILD_WASM_PROFILE'; use debug, fast, or release" >&2
        exit 2
        ;;
esac

# Keep PocketIC-oriented CI artifacts small.
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:-} -C debuginfo=0"

if [ -n "${CANIC_REFERENCE_CANISTERS:-}" ]; then
    # Allow focused harnesses to build only the canisters they actually stage.
    read -r -a BUILD_CANISTERS <<<"$CANIC_REFERENCE_CANISTERS"
else
    DEFAULT_BUILD_CANISTERS="$(bash scripts/ci/list-config-canisters.sh --config "$BUILD_CONFIG" --ci-order)"
    mapfile -t BUILD_CANISTERS <<<"$DEFAULT_BUILD_CANISTERS"
fi

# Build the ordinary reference artifacts first so the thin-root manifest path
# can emit once the full root-subnet ordinary artifact set exists. Root itself builds the
# implicit bootstrap `wasm_store` artifact internally.
for canister in "${BUILD_CANISTERS[@]}"; do
    cargo run -q --profile fast -p canic-host --example build_artifact --locked -- \
        "$canister" "$BUILD_WASM_PROFILE" "$ROOT_DIR" "$ROOT_DIR" "$BUILD_CONFIG"

    if [ "$canister" = "root" ]; then
        ROOT_WASM_GZ_PATH=".icp/local/canisters/root/root.wasm.gz"
        ROOT_WASM_GZ_BYTES="$(stat -c%s "$ROOT_WASM_GZ_PATH")"
        if [ "$ROOT_WASM_GZ_BYTES" -ge 100000000 ]; then
            echo "root.wasm.gz too large for PocketIC chunk store: ${ROOT_WASM_GZ_BYTES} bytes" >&2
            exit 1
        fi
    fi
done
