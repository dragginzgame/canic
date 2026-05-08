#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-downstream-wasm-store.XXXXXX")"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TMP_ROOT/downstream-root"
VERSION="$(
    cargo metadata --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
        jq -r '.packages[] | select(.name == "canic") | .version'
)"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

ensure_packaged_crate() {
    local crate_name="$1"
    local crate_archive="$PACKAGE_STAGING_ROOT/$crate_name-$VERSION.crate"
    rm -f "$crate_archive"
    if [ "$crate_name" = "canic-cli" ]; then
        cargo package -p "$crate_name" --allow-dirty --no-verify \
            --config "patch.crates-io.canic-host.path=\"$ROOT/crates/canic-host\"" >/dev/null
    else
        cargo package -p "$crate_name" --allow-dirty --no-verify >/dev/null
    fi
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_archive=""
    for crate_archive in \
        "$PACKAGE_STAGING_ROOT/canic-cdk-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-backup-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-cli-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-control-plane-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-macros-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-host-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-memory-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-$VERSION.crate"
    do
        [ -f "$crate_archive" ] || {
            echo "expected packaged crate archive at $crate_archive" >&2
            exit 1
        }
        tar -xzf "$crate_archive" -C "$PACKAGE_ROOT"
    done
}

prepare_tool_root() {
    mkdir -p "$TOOL_ROOT"

    cat > "$TOOL_ROOT/Cargo.toml" <<EOF
[workspace]
members = ["package-root/canic-cli-$VERSION"]
resolver = "2"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-backup = { path = "package-root/canic-backup-$VERSION" }
canic-cdk = { path = "package-root/canic-cdk-$VERSION" }
canic-cli = { path = "package-root/canic-cli-$VERSION" }
canic-control-plane = { path = "package-root/canic-control-plane-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
canic-memory = { path = "package-root/canic-memory-$VERSION" }
EOF
}

prepare_downstream_root() {
    mkdir -p "$DOWNSTREAM_ROOT/src" "$DOWNSTREAM_ROOT/fleets"

    cp "$ROOT/fleets/test/canic.toml" "$DOWNSTREAM_ROOT/fleets/canic.toml"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<EOF
[package]
name = "canic-packaged-downstream-probe"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
canic = { path = "$PACKAGE_ROOT/canic-$VERSION" }
EOF

    cat > "$DOWNSTREAM_ROOT/src/lib.rs" <<'EOF'
pub fn packaged_downstream_probe() {}
EOF
}

run_probe() {
    (
        cd "$TOOL_ROOT"
        CANIC_WORKSPACE_ROOT="$DOWNSTREAM_ROOT" \
            CANIC_WASM_PROFILE=fast \
            cargo run --offline -q -p canic-cli --bin canic -- build wasm_store >/dev/null
    )
}

assert_probe_outputs() {
    local wrapper_manifest="$DOWNSTREAM_ROOT/.icp/local/generated/canic-wasm-store/Cargo.toml"
    local wasm_path="$DOWNSTREAM_ROOT/.icp/local/canisters/wasm_store/wasm_store.wasm"
    local wasm_gz_path="$DOWNSTREAM_ROOT/.icp/local/canisters/wasm_store/wasm_store.wasm.gz"
    local did_path="$DOWNSTREAM_ROOT/.icp/local/canisters/wasm_store/wasm_store.did"

    [ ! -d "$PACKAGE_ROOT/canic-wasm-store-$VERSION" ] || {
        echo "expected isolated package root to exclude canic-wasm-store so the generated wrapper path is exercised" >&2
        exit 1
    }
    [ -f "$wrapper_manifest" ] || {
        echo "expected generated wasm_store wrapper at $wrapper_manifest" >&2
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
    [ -s "$did_path" ] || {
        echo "expected built wasm_store candid artifact at $did_path" >&2
        exit 1
    }

    grep -q '\[patch.crates-io\]' "$wrapper_manifest" || {
        echo "expected generated wrapper to patch sibling packaged Canic crates" >&2
        exit 1
    }
    grep -q '\[profile.fast\]' "$wrapper_manifest" || {
        echo "expected generated wrapper to define the Canic fast profile" >&2
        exit 1
    }
    grep -q '\[profile.release\]' "$wrapper_manifest" || {
        echo "expected generated wrapper to define the Canic release profile" >&2
        exit 1
    }
}

main() {
    ensure_packaged_crate canic-cdk
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-cli
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic-memory
    ensure_packaged_crate canic
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    run_probe
    assert_probe_outputs

    echo "packaged downstream wasm_store probe passed"
}

main "$@"
