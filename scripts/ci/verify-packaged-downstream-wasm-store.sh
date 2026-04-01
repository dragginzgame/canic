#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-downstream.XXXXXX")"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
PACKAGE_ROOT="$TMP_ROOT/package-root"
VERSION="$(
    cargo metadata --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
        python3 -c '
import json, sys

data = json.load(sys.stdin)
root = next(pkg for pkg in data["packages"] if pkg["name"] == "canic")
print(root["version"])
'
)"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

package_canister_crate() {
    local crate_name="$1"
    cargo package -p "$crate_name" --allow-dirty --no-verify >/dev/null
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_dir=""
    for crate_dir in \
        "$PACKAGE_STAGING_ROOT/canic-cdk-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-control-plane-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-dsl-macros-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-memory-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-sharding-runtime-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-types-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-$VERSION"
    do
        [ -d "$crate_dir" ] || {
            echo "expected packaged crate directory at $crate_dir" >&2
            exit 1
        }
        cp -R "$crate_dir" "$PACKAGE_ROOT/"
    done
}

prepare_probe_root() {
    mkdir -p "$TMP_ROOT/src" "$TMP_ROOT/scripts/app" "$TMP_ROOT/canisters"
    cp "$ROOT/scripts/app/build.sh" "$TMP_ROOT/scripts/app/build.sh"
    cp "$ROOT/scripts/env.sh" "$TMP_ROOT/scripts/env.sh"
    cp "$ROOT/canisters/canic.toml" "$TMP_ROOT/canisters/canic.toml"

    cat > "$TMP_ROOT/Cargo.toml" <<EOF
[package]
name = "canic-packaged-downstream-probe"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
canic = { path = "$PACKAGE_ROOT/canic-$VERSION" }
EOF

    cat > "$TMP_ROOT/src/lib.rs" <<'EOF'
pub fn packaged_downstream_probe() {}
EOF
}

assert_probe_outputs() {
    local wrapper_manifest="$TMP_ROOT/.dfx/local/generated/canic-wasm-store/Cargo.toml"
    local wasm_path="$TMP_ROOT/.dfx/local/canisters/wasm_store/wasm_store.wasm"
    local wasm_gz_path="$TMP_ROOT/.dfx/local/canisters/wasm_store/wasm_store.wasm.gz"

    [ ! -d "$PACKAGE_ROOT/canic-wasm-store-$VERSION" ] || {
        echo "expected isolated package root to exclude canic-wasm-store so the hidden wrapper path is exercised" >&2
        exit 1
    }
    [ -f "$wrapper_manifest" ] || {
        echo "expected generated hidden wasm_store wrapper at $wrapper_manifest" >&2
        exit 1
    }
    [ -s "$wasm_path" ] || {
        echo "expected built wasm_store artifact at $wasm_path" >&2
        exit 1
    }
    [ -s "$wasm_gz_path" ] || {
        echo "expected built wasm_store gzip artifact at $wasm_gz_path" >&2
        exit 1
    }

    grep -q '\[patch.crates-io\]' "$wrapper_manifest" || {
        echo "expected generated wrapper to patch sibling packaged Canic crates" >&2
        exit 1
    }
}

main() {
    package_canister_crate canic-cdk
    package_canister_crate canic-control-plane
    package_canister_crate canic-core
    package_canister_crate canic-dsl-macros
    package_canister_crate canic-memory
    package_canister_crate canic-sharding-runtime
    package_canister_crate canic-types
    package_canister_crate canic
    populate_isolated_package_root

    prepare_probe_root

    (
        cd "$TMP_ROOT"
        bash scripts/app/build.sh wasm_store
    )

    assert_probe_outputs

    echo "packaged downstream wasm_store probe passed"
}

main "$@"
