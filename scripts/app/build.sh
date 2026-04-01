#!/bin/bash

# don't allow errors
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SELF="$SCRIPT_DIR/$(basename "$0")"

# Anchor the build helper to its own checkout/copy instead of any inherited
# shell state so downstream and dfx builds are not poisoned by stale env vars.
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCRIPTS="$ROOT/scripts"
export ROOT SCRIPTS

# Set up environment
source "$SCRIPT_DIR/../env.sh"
cd "$ROOT"

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
    if [ "$canister" = "wasm_store" ]; then
        printf '%s\n' "$(canonical_wasm_store_source_root)/wasm_store.did"
        return
    fi

    printf '%s\n' "$ROOT/canisters/$canister/$canister.did"
}

artifact_did_path() {
    local canister="$1"
    printf '%s\n' "$ROOT/.dfx/local/canisters/$canister/$canister.did"
}

canonical_wasm_store_manifest_path() {
    local workspace_manifest="$ROOT/crates/canic-wasm-store/Cargo.toml"
    if [ -f "$workspace_manifest" ]; then
        printf '%s\n' "$workspace_manifest"
        return
    fi

    local resolved_manifest
    if resolved_manifest="$(
        cargo metadata --format-version=1 --manifest-path "$ROOT/Cargo.toml" | python3 -c '
import json, sys
from pathlib import Path

data = json.load(sys.stdin)
packages = data.get("packages", [])

for package in packages:
    if package.get("name") == "canic-wasm-store":
        print(package["manifest_path"])
        raise SystemExit(0)

for package in packages:
    if package.get("name") != "canic":
        continue

    manifest_path = Path(package["manifest_path"]).resolve()
    version = package.get("version")

    # Local path/git checkouts of the Canic repo keep `canic` under `crates/canic`.
    sibling_manifest = manifest_path.parent.parent / "canic-wasm-store" / "Cargo.toml"
    if sibling_manifest.is_file():
        print(sibling_manifest)
        raise SystemExit(0)

    # Published crates land side-by-side in the cargo registry source tree.
    if version:
        registry_sibling = manifest_path.parent.parent / f"canic-wasm-store-{version}" / "Cargo.toml"
        if registry_sibling.is_file():
            print(registry_sibling)
            raise SystemExit(0)
'
    )" && [ -n "$resolved_manifest" ]; then
        printf '%s\n' "$resolved_manifest"
        return
    fi

    ensure_generated_wasm_store_wrapper
    printf '%s\n' "$(generated_wasm_store_wrapper_root)/Cargo.toml"
}

resolved_canic_manifest_path() {
    cargo metadata --format-version=1 --manifest-path "$ROOT/Cargo.toml" | python3 -c '
import json, sys

data = json.load(sys.stdin)
packages = data.get("packages", [])

for package in packages:
    if package.get("name") == "canic":
        print(package["manifest_path"])
        raise SystemExit(0)

raise SystemExit(
    "unable to locate resolved '\''canic'\'' package in cargo metadata; downstreams that build the implicit wasm_store must depend on '\''canic'\''."
)
'
}

generated_wasm_store_wrapper_root() {
    printf '%s\n' "$ROOT/.dfx/local/generated/canic-wasm-store"
}

generated_wasm_store_wrapper_patch_table() {
    local canic_manifest="$1"
    local canic_root
    local sibling_root
    local crate_name
    local manifest_path
    local crate_root
    local rendered=""

    canic_root="$(dirname "$canic_manifest")"
    sibling_root="$(dirname "$canic_root")"

    for crate_name in \
        canic-cdk \
        canic-control-plane \
        canic-core \
        canic-dsl-macros \
        canic-memory \
        canic-sharding-runtime \
        canic-types
    do
        manifest_path=""

        if [ -f "$sibling_root/$crate_name/Cargo.toml" ]; then
            manifest_path="$sibling_root/$crate_name/Cargo.toml"
        else
            for candidate in "$sibling_root/$crate_name-"*/Cargo.toml; do
                if [ -f "$candidate" ]; then
                    manifest_path="$candidate"
                    break
                fi
            done
        fi

        if [ -z "$manifest_path" ]; then
            continue
        fi

        crate_root="$(dirname "$manifest_path")"
        rendered="${rendered}${crate_name} = { path = \"$crate_root\" }\n"
    done

    if [ -z "$rendered" ]; then
        return
    fi

    printf '[patch.crates-io]\n%b' "$rendered"
}

ensure_generated_wasm_store_wrapper() {
    local wrapper_root
    local canic_manifest
    local canic_root
    local patch_table

    wrapper_root="$(generated_wasm_store_wrapper_root)"
    canic_manifest="$(resolved_canic_manifest_path)"
    canic_root="$(dirname "$canic_manifest")"
    patch_table="$(generated_wasm_store_wrapper_patch_table "$canic_manifest")"

    mkdir -p "$wrapper_root/src"

    cat > "$wrapper_root/Cargo.toml" <<EOF
[package]
name = "canic-generated-wasm-store"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
name = "canister_wasm_store"
crate-type = ["cdylib", "rlib"]

[dependencies]
canic = { path = "$canic_root", features = ["control-plane"] }
ic-cdk = "0.20.0"
candid = { version = "0.10", default-features = false }

[build-dependencies]
canic = { path = "$canic_root" }
EOF

    if [ -n "$patch_table" ]; then
        printf '\n%s\n' "$patch_table" >> "$wrapper_root/Cargo.toml"
    fi

    cat > "$wrapper_root/build.rs" <<'EOF'
fn main() {
    let config_path = std::env::var("CANIC_CONFIG_PATH")
        .expect("CANIC_CONFIG_PATH must be set for generated wasm_store wrapper");

    canic::build!(config_path);
}
EOF

    cat > "$wrapper_root/src/lib.rs" <<'EOF'
#![allow(clippy::unused_async)]

canic::start_wasm_store!();
canic::cdk::export_candid_debug!();
EOF
}

canonical_wasm_store_source_root() {
    dirname "$(canonical_wasm_store_manifest_path)"
}

canonical_wasm_store_config_path() {
    if [ -n "${CANIC_CONFIG_PATH:-}" ]; then
        printf '%s\n' "$CANIC_CONFIG_PATH"
        return
    fi

    printf '%s\n' "$ROOT/canisters/canic.toml"
}

newest_canister_interface_input_epoch() {
    local canister="$1"
    local canister_source_root
    canister_source_root="$ROOT/canisters/$canister"
    if [ "$canister" = "wasm_store" ]; then
        canister_source_root="$(canonical_wasm_store_source_root)"
    fi

    find \
        "$ROOT/Cargo.toml" \
        "$ROOT/Cargo.lock" \
        "$ROOT/scripts/app/build.sh" \
        "$canister_source_root" \
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

build_requested_canisters() {
    local profile_dir="$1"
    shift

    if [ "$#" -eq 1 ] && [ "$1" = "wasm_store" ]; then
        local wasm_store_manifest
        wasm_store_manifest="$(canonical_wasm_store_manifest_path)"
        local config_path
        config_path="$(canonical_wasm_store_config_path)"

        local cargo_args=(
            build
            --manifest-path "$wasm_store_manifest"
            --target wasm32-unknown-unknown
        )

        if [ "$profile_dir" = "release" ]; then
            cargo_args+=(--release)
        fi

        CANIC_CONFIG_PATH="$config_path" \
        CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" \
        cargo "${cargo_args[@]}"
        return
    fi

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

emit_root_release_set_manifest_command() {
    if command -v canic-emit-root-release-set-manifest >/dev/null 2>&1; then
        printf '%s\n' "canic-emit-root-release-set-manifest"
        return
    fi

    if [ -f "$ROOT/crates/canic-installer/Cargo.toml" ]; then
        printf '%s\n' "cargo run -q -p canic-installer --bin canic-emit-root-release-set-manifest --"
        return
    fi

    printf '%s\n' ""
}

maybe_emit_root_release_set_manifest() {
    local manifest_command
    manifest_command="$(emit_root_release_set_manifest_command)"

    [ -n "$manifest_command" ] || return 0

    local manifest_path=""
    manifest_path="$(
        CANIC_WORKSPACE_ROOT="$ROOT" \
        DFX_NETWORK="${DFX_NETWORK:-local}" \
        bash -lc "$manifest_command --if-ready"
    )"

    if [ -n "$manifest_path" ]; then
        echo "Emitted root release-set manifest: $manifest_path"
    fi
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
    echo "Refreshing embedded wasm_store bootstrap artifact via normal custom build path"
    "$SELF" wasm_store
    export CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS=1
    build_requested_canisters "$PROFILE_DIR" root
elif [ "$CAN" = "wasm_store" ]; then
    build_requested_canisters "$PROFILE_DIR" wasm_store
else
    build_requested_canisters "$PROFILE_DIR" "$CAN"
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

if [ "$CAN" != "wasm_store" ]; then
    maybe_emit_root_release_set_manifest
fi
