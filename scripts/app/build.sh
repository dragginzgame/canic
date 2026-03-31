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

IS_RELEASE_BUILD=1
if [ "$PROFILE_DIR" = "debug" ]; then
    IS_RELEASE_BUILD=0
fi

artifact_profile_path() {
    local canister="$1"
    printf '%s\n' "$ROOT/.dfx/local/canisters/$canister/.build-profile"
}

source_did_path() {
    local canister="$1"
    printf '%s\n' "$ROOT/canisters/$canister/$canister.did"
}

artifact_did_path() {
    local canister="$1"
    printf '%s\n' "$ROOT/.dfx/local/canisters/$canister/$canister.did"
}

NONROOT_CANISTERS=(
    app
    minimal
    scale
    scale_hub
    test
    user_hub
    user_shard
)

workspace_wasm_build_stamp() {
    local profile_dir="$1"
    local scope="$2"
    printf '%s\n' "$ROOT/.dfx/local/canisters/.wasm-build-$scope-$profile_dir.stamp"
}

workspace_wasm_build_lock() {
    printf '%s\n' "$ROOT/.dfx/local/canisters/.wasm-build.lock"
}

newest_workspace_input_epoch() {
    find \
        "$ROOT/Cargo.toml" \
        "$ROOT/Cargo.lock" \
        "$ROOT/dfx.json" \
        "$ROOT/scripts/app/build.sh" \
        "$ROOT/crates" \
        "$ROOT/canisters" \
        -type f \
        ! -name '*.did' \
        -printf '%T@\n' 2>/dev/null | sort -nr | head -1
}

newest_canister_interface_input_epoch() {
    local canister="$1"
    find \
        "$ROOT/Cargo.toml" \
        "$ROOT/Cargo.lock" \
        "$ROOT/scripts/app/build.sh" \
        "$ROOT/canisters/$canister" \
        "$ROOT/crates/canic" \
        "$ROOT/crates/canic-core" \
        "$ROOT/crates/canic-cdk" \
        "$ROOT/crates/canic-memory" \
        "$ROOT/crates/canic-internal" \
        -type f \
        ! -name '*.did' \
        -printf '%T@\n' 2>/dev/null | sort -nr | head -1
}

source_did_is_current() {
    local canister="$1"
    local source_did
    source_did="$(source_did_path "$canister")"

    [ -f "$source_did" ] || return 1

    local newest_input
    newest_input="$(newest_canister_interface_input_epoch "$canister")"
    [ -n "$newest_input" ] || return 1

    local did_epoch
    did_epoch="$(stat -c '%Y' "$source_did")"

    awk "BEGIN { exit !($did_epoch >= $newest_input) }"
}

workspace_wasm_target_path() {
    local canister="$1"
    local profile_dir="$2"
    local target_root="${CARGO_TARGET_DIR:-$ROOT/target}"
    printf '%s\n' "$target_root/wasm32-unknown-unknown/$profile_dir/canister_$canister.wasm"
}

maybe_shrink_wasm_artifact() {
    local wasm_path="$1"

    if ! command -v ic-wasm >/dev/null 2>&1; then
        return
    fi

    local shrunk_path="${wasm_path}.shrunk"

    if ic-wasm "$wasm_path" -o "$shrunk_path" shrink >/dev/null 2>&1; then
        mv -f "$shrunk_path" "$wasm_path"
    else
        rm -f "$shrunk_path"
    fi
}

workspace_wasm_build_is_current() {
    local profile_dir="$1"
    local scope="$2"
    shift 2
    local stamp
    stamp="$(workspace_wasm_build_stamp "$profile_dir" "$scope")"

    [ -f "$stamp" ] || return 1

    local canister
    for canister in "$@"; do
        [ -f "$(workspace_wasm_target_path "$canister" "$profile_dir")" ] || return 1
    done

    local newest_input
    newest_input="$(newest_workspace_input_epoch)"
    [ -n "$newest_input" ] || return 1

    local stamp_epoch
    stamp_epoch="$(stat -c '%Y' "$stamp")"

    awk "BEGIN { exit !($stamp_epoch >= $newest_input) }"
}

build_requested_canisters() {
    local profile_dir="$1"
    shift

    local cargo_args=(
        build
        --target wasm32-unknown-unknown
    )

    if [ "$profile_dir" = "release" ]; then
        cargo_args+=(--release)
    fi

    local canister
    for canister in "$@"; do
        cargo_args+=(-p "canister_$canister")
    done

    cargo "${cargo_args[@]}"
}

ensure_workspace_wasm_build() {
    local profile_dir="$1"
    local scope="$2"
    shift 2

    mkdir -p "$ROOT/.dfx/local/canisters"

    local lock_file
    lock_file="$(workspace_wasm_build_lock)"

    exec 9>"$lock_file"
    flock 9

    if workspace_wasm_build_is_current "$profile_dir" "$scope" "$@"; then
        flock -u 9
        exec 9>&-
        return
    fi

    build_requested_canisters "$profile_dir" "$@"
    touch "$(workspace_wasm_build_stamp "$profile_dir" "$scope")"

    flock -u 9
    exec 9>&-
}

extract_and_cache_did_from_debug_artifact() {
    local canister="$1"
    local source_did
    local artifact_did

    source_did="$(source_did_path "$canister")"
    artifact_did="$(artifact_did_path "$canister")"

    build_requested_canisters "debug" "$canister"
    candid-extractor "$(workspace_wasm_target_path "$canister" "debug")" > "$source_did"
    cp -f "$source_did" "$artifact_did"
}

##
## Build Wasm
##

mkdir -p "$ROOT/.dfx/local/canisters/$CAN"
WASM_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm"
WASM_GZ_TARGET="$ROOT/.dfx/local/canisters/$CAN/$CAN.wasm.gz"
PROFILE_FILE="$(artifact_profile_path "$CAN")"
SOURCE_DID="$(source_did_path "$CAN")"
ARTIFACT_DID="$(artifact_did_path "$CAN")"

if [ "$CAN" = "root" ]; then
    build_requested_canisters "$PROFILE_DIR" root
else
    ensure_workspace_wasm_build "$PROFILE_DIR" "nonroot" "${NONROOT_CANISTERS[@]}"
fi
cp -f "$(workspace_wasm_target_path "$CAN" "$PROFILE_DIR")" "$WASM_TARGET"
maybe_shrink_wasm_artifact "$WASM_TARGET"
gzip -n -9 -c "$WASM_TARGET" > "$WASM_GZ_TARGET"
printf '%s\n' "$PROFILE_DIR" > "$PROFILE_FILE"

if [ "$IS_RELEASE_BUILD" = "1" ]; then
    echo "Building release (no candid extraction)"
    if source_did_is_current "$CAN"; then
        cp -f "$SOURCE_DID" "$ARTIFACT_DID"
    else
        echo "Source .did missing or stale: $SOURCE_DID; regenerating and caching it into $ARTIFACT_DID from a debug fallback"
        extract_and_cache_did_from_debug_artifact "$CAN"
    fi
else
    echo "Building debug (with candid extraction)"
    echo "Running candid extraction on same artifact"
    extract_and_cache_did_from_debug_artifact "$CAN"
fi
