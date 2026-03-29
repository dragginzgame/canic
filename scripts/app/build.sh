#!/bin/bash

# don't allow errors
set -e

SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"

# Set up environment
source "$(dirname "$0")/../env.sh"
cd "$SCRIPTS"

# Check if an argument was provided
if [ $# -eq 0 ]; then
    echo "usage: build.sh [canister_name]"
    exit 1
fi
CAN=$1

# Build dependent canisters first when this helper is invoked directly.
# DFX handles dependency ordering itself, but these guards keep standalone
# `scripts/app/build.sh <canister>` calls from failing on missing `.wasm.gz`
# artifacts consumed by bundle canisters.
ensure_canister_artifact() {
    local dep="$1"
    local artifact="$ROOT/.dfx/local/canisters/$dep/$dep.wasm.gz"

    if [ -f "$artifact" ]; then
        return
    fi

    "$SELF" "$dep"
}

case "$CAN" in
    user_hub)
        ensure_canister_artifact "user_shard"
        ;;
    scale_hub)
        ensure_canister_artifact "scale"
        ;;
    shard_hub)
        ensure_canister_artifact "shard"
        ;;
    wasm_store)
        ensure_canister_artifact "app"
        ensure_canister_artifact "minimal"
        ensure_canister_artifact "user_hub"
        ensure_canister_artifact "scale_hub"
        ensure_canister_artifact "shard_hub"
        ensure_canister_artifact "test"
        ;;
    root)
        ensure_canister_artifact "wasm_store"
        ;;
esac

##
## Build Wasm
##

mkdir -p "$ROOT/.dfx/local/canisters/$CAN"
WASM_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm"
WASM_GZ_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm.gz"

# Build in release mode by default to keep wasm artifacts small.
# Set RELEASE=0 to force a debug build.
PROFILE_FLAG="--release"
PROFILE_DIR="release"
if [ "${RELEASE:-1}" = "0" ]; then
    PROFILE_FLAG=""
    PROFILE_DIR="debug"
fi

CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS=1 \
cargo build --target wasm32-unknown-unknown -p "canister_$CAN" $PROFILE_FLAG
cp -f "$ROOT/target/wasm32-unknown-unknown/$PROFILE_DIR/canister_$CAN.wasm" "$WASM_TARGET"
gzip -n -c "$WASM_TARGET" > "$WASM_GZ_TARGET"

# Build a debug extractor-only Wasm with eager init disabled so
# `candid-extractor` can instantiate bundle canisters without executing
# runtime startup hooks. The debug profile keeps `get_candid_pointer`
# exported through `canic::export_candid!()`.
CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS=1 CANIC_SKIP_EAGER_INIT=1 \
cargo build --target wasm32-unknown-unknown -p "canister_$CAN"

# extract candid

candid-extractor "$ROOT/target/wasm32-unknown-unknown/debug/canister_$CAN.wasm" \
    > "$ROOT/.dfx/local/canisters/$CAN/$CAN.did"
