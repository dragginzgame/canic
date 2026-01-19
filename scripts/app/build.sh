#!/bin/bash

# don't allow errors
set -e

# Set up environment
source "$(dirname "$0")/../env.sh"
cd "$SCRIPTS"

# Check if an argument was provided
if [ $# -eq 0 ]; then
    echo "usage: build.sh [canister_name]"
    exit 1
fi
CAN=$1

##
## Build Wasm
##

mkdir -p "$ROOT/.dfx/local/canisters/$CAN"
WASM_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm"

# Support release builds via env RELEASE=1 (defaults to debug)
PROFILE_FLAG=""
PROFILE_DIR="debug"
if [ "${RELEASE:-0}" = "1" ]; then
    PROFILE_FLAG="--release"
    PROFILE_DIR="release"
fi

FEATURES_FLAG=""
if [ "${CANIC_UNCERTIFIED_TESTING:-0}" = "1" ]; then
    FEATURES_FLAG="--features canic/uncertified-testing"
fi

cargo build --target wasm32-unknown-unknown -p "canister_$CAN" $PROFILE_FLAG $FEATURES_FLAG
cp -f "$ROOT/target/wasm32-unknown-unknown/$PROFILE_DIR/canister_$CAN.wasm" "$WASM_TARGET"

# extract candid

candid-extractor "$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm" \
    > "$ROOT/.dfx/local/canisters/$CAN/$CAN.did"
