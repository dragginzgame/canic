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

# Build in release mode by default to keep wasm artifacts small.
# Set RELEASE=0 to force a debug build.
PROFILE_FLAG="--release"
PROFILE_DIR="release"
if [ "${RELEASE:-1}" = "0" ]; then
    PROFILE_FLAG=""
    PROFILE_DIR="debug"
fi

artifact_profile_path() {
    local canister="$1"
    printf '%s\n' "$ROOT/.dfx/local/canisters/$canister/.build-profile"
}

dependencies_for_canister() {
    local canister="$1"

    case "$canister" in
        user_hub)
            printf '%s\n' "user_shard"
            ;;
        scale_hub)
            printf '%s\n' "scale"
            ;;
        shard_hub)
            printf '%s\n' "shard"
            ;;
        wasm_store)
            printf '%s\n' "app" "minimal" "user_hub" "scale_hub" "shard_hub" "test"
            ;;
        root)
            printf '%s\n' "wasm_store"
            ;;
    esac
}

canister_artifact_is_current() {
    local canister="$1"
    local artifact="$ROOT/.dfx/local/canisters/$canister/$canister.wasm.gz"
    local profile_file
    profile_file="$(artifact_profile_path "$canister")"

    if [ ! -f "$artifact" ] || [ ! -f "$profile_file" ]; then
        return 1
    fi

    local built_profile
    built_profile="$(cat "$profile_file")"
    if [ "$built_profile" != "$PROFILE_DIR" ]; then
        return 1
    fi

    local dep
    while IFS= read -r dep; do
        [ -n "$dep" ] || continue
        canister_artifact_is_current "$dep" || return 1
    done < <(dependencies_for_canister "$canister")

    return 0
}

# Build dependent canisters first when this helper is invoked directly.
# DFX handles dependency ordering itself, but these guards keep standalone
# `scripts/app/build.sh <canister>` calls from failing on missing `.wasm.gz`
# artifacts consumed by bundle canisters.
ensure_canister_artifact() {
    local dep="$1"
    if canister_artifact_is_current "$dep"; then
        return
    fi

    RELEASE="${RELEASE:-1}" "$SELF" "$dep"
}

case "$CAN" in
    user_hub|scale_hub|shard_hub|wasm_store|root)
        while IFS= read -r dep; do
            [ -n "$dep" ] || continue
            ensure_canister_artifact "$dep"
        done < <(dependencies_for_canister "$CAN")
        ;;
esac

##
## Build Wasm
##

mkdir -p "$ROOT/.dfx/local/canisters/$CAN"
WASM_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm"
WASM_GZ_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm.gz"
PROFILE_FILE="$(artifact_profile_path "$CAN")"

CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS=1 \
cargo build --target wasm32-unknown-unknown -p "canister_$CAN" $PROFILE_FLAG
cp -f "$ROOT/target/wasm32-unknown-unknown/$PROFILE_DIR/canister_$CAN.wasm" "$WASM_TARGET"
gzip -n -c "$WASM_TARGET" > "$WASM_GZ_TARGET"
printf '%s\n' "$PROFILE_DIR" > "$PROFILE_FILE"

# Build a debug extractor-only Wasm with eager init disabled so
# `candid-extractor` can instantiate bundle canisters without executing
# runtime startup hooks. The debug profile keeps `get_candid_pointer`
# exported through `canic::export_candid!()`.
CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS=1 CANIC_SKIP_EAGER_INIT=1 \
cargo build --target wasm32-unknown-unknown -p "canister_$CAN"

# extract candid

candid-extractor "$ROOT/target/wasm32-unknown-unknown/debug/canister_$CAN.wasm" \
    > "$ROOT/.dfx/local/canisters/$CAN/$CAN.did"
