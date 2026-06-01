#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-downstream-wasm-store.XXXXXX")"
HOST_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
HOST_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TMP_ROOT/downstream-root"
PROOF_HOME="$TMP_ROOT/home"
PROOF_TARGET_DIR="$TMP_ROOT/cargo-target"
PROOF_TMPDIR="$TMP_ROOT/tmp"
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
    case "$crate_name" in
        canic-control-plane)
            cargo package -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" >/dev/null
            ;;
        canic)
            cargo package -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-control-plane.path=\"$ROOT/crates/canic-control-plane\"" \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" \
                --config "patch.crates-io.canic-macros.path=\"$ROOT/crates/canic-macros\"" >/dev/null
            ;;
        canic-host)
            cargo package -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" >/dev/null
            ;;
        *)
            cargo package -p "$crate_name" --allow-dirty --no-verify >/dev/null
            ;;
    esac
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_archive=""
    for crate_archive in \
        "$PACKAGE_STAGING_ROOT/canic-backup-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-control-plane-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-macros-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-host-$VERSION.crate" \
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
members = ["package-root/canic-host-$VERSION"]
resolver = "2"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-backup = { path = "package-root/canic-backup-$VERSION" }
canic-control-plane = { path = "package-root/canic-control-plane-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
EOF
}

assert_packaged_tool_root() {
    if grep -R -Fq "$ROOT/crates" "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream wasm_store proof must not use repository crate paths" >&2
        exit 1
    fi

    if grep -R -Fq 'target/debug/canic' "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream wasm_store proof must not use target/debug/canic" >&2
        exit 1
    fi
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

[patch.crates-io]
canic-control-plane = { path = "$PACKAGE_ROOT/canic-control-plane-$VERSION" }
canic-core = { path = "$PACKAGE_ROOT/canic-core-$VERSION" }
canic-macros = { path = "$PACKAGE_ROOT/canic-macros-$VERSION" }
EOF

    cat > "$DOWNSTREAM_ROOT/src/lib.rs" <<'EOF'
pub fn packaged_downstream_probe() {}
EOF
}

run_probe() {
    mkdir -p "$PROOF_HOME" "$PROOF_TARGET_DIR" "$PROOF_TMPDIR"
    assert_packaged_tool_root

    (
        cd "$TOOL_ROOT"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            CANIC_WORKSPACE_ROOT="$DOWNSTREAM_ROOT" \
            CANIC_WASM_PROFILE=fast \
            cargo run --offline -q -p canic-host --example build_artifact -- wasm_store >/dev/null
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
    grep -q "$PACKAGE_ROOT/canic-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to depend on packaged canic source" >&2
        exit 1
    }
    grep -q "$PACKAGE_ROOT/canic-control-plane-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-control-plane source" >&2
        exit 1
    }
    grep -q "$PACKAGE_ROOT/canic-core-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-core source" >&2
        exit 1
    }
    grep -q "$PACKAGE_ROOT/canic-macros-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-macros source" >&2
        exit 1
    }
    if grep -Fq "$ROOT/crates" "$wrapper_manifest"; then
        echo "generated wasm_store wrapper must not use repository crate paths" >&2
        exit 1
    fi
}

main() {
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    run_probe
    assert_probe_outputs

    echo "packaged downstream wasm_store probe passed"
}

main "$@"
