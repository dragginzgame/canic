#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-downstream-wasm-store.XXXXXX")"
HOST_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
HOST_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
GENERATED_TOOL_ROOT="$TMP_ROOT/tool-root-generated"
GENERATED_PACKAGE_ROOT="$GENERATED_TOOL_ROOT/package-root"
GENERATED_DOWNSTREAM_ROOT="$TMP_ROOT/downstream-generated"
GENERATED_TARGET_DIR="$TMP_ROOT/cargo-target-generated"
CANONICAL_TOOL_ROOT="$TMP_ROOT/tool-root-canonical"
CANONICAL_PACKAGE_ROOT="$CANONICAL_TOOL_ROOT/package-root"
CANONICAL_DOWNSTREAM_ROOT="$TMP_ROOT/downstream-canonical"
CANONICAL_TARGET_DIR="$TMP_ROOT/cargo-target-canonical"
PROOF_HOME="$TMP_ROOT/home"
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
    local package_root="$1"
    local include_wasm_store="$2"

    mkdir -p "$package_root"

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
        tar -xzf "$crate_archive" -C "$package_root"
    done

    if [ "$include_wasm_store" = "yes" ]; then
        crate_archive="$PACKAGE_STAGING_ROOT/canic-wasm-store-$VERSION.crate"
        [ -f "$crate_archive" ] || {
            echo "expected packaged crate archive at $crate_archive" >&2
            exit 1
        }
        tar -xzf "$crate_archive" -C "$package_root"
    fi
}

prepare_tool_root() {
    local tool_root="$1"

    mkdir -p "$tool_root"

    cat > "$tool_root/Cargo.toml" <<EOF
[workspace]
members = ["package-root/canic-host-$VERSION"]
exclude = ["package-root/canic-wasm-store-$VERSION"]
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
    local tool_root="$1"
    local package_root="$2"

    if grep -R -Fq "$ROOT/crates" "$tool_root/Cargo.toml" "$package_root"; then
        echo "packaged downstream wasm_store proof must not use repository crate paths" >&2
        exit 1
    fi

    if grep -R -Fq 'target/debug/canic' "$tool_root/Cargo.toml" "$package_root"; then
        echo "packaged downstream wasm_store proof must not use target/debug/canic" >&2
        exit 1
    fi
}

prepare_downstream_root() {
    local downstream_root="$1"
    local package_root="$2"

    mkdir -p "$downstream_root/src" "$downstream_root/fleets"

    cp "$ROOT/fleets/test/canic.toml" "$downstream_root/fleets/canic.toml"

    cat > "$downstream_root/Cargo.toml" <<EOF
[package]
name = "canic-packaged-downstream-probe"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
canic = { path = "$package_root/canic-$VERSION" }

[patch.crates-io]
canic-control-plane = { path = "$package_root/canic-control-plane-$VERSION" }
canic-core = { path = "$package_root/canic-core-$VERSION" }
canic-macros = { path = "$package_root/canic-macros-$VERSION" }
EOF

    cat > "$downstream_root/src/lib.rs" <<'EOF'
pub fn packaged_downstream_probe() {}
EOF
}

run_probe() {
    local tool_root="$1"
    local package_root="$2"
    local downstream_root="$3"
    local target_dir="$4"

    mkdir -p "$PROOF_HOME" "$target_dir" "$PROOF_TMPDIR"
    assert_packaged_tool_root "$tool_root" "$package_root"

    (
        cd "$tool_root"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            CANIC_WORKSPACE_ROOT="$downstream_root" \
            CANIC_WASM_PROFILE=fast \
            cargo run --offline -q -p canic-host --example build_artifact -- wasm_store >/dev/null
    )
}

assert_wasm_store_artifacts() {
    local scenario="$1"
    local downstream_root="$2"
    local wasm_path="$downstream_root/.icp/local/canisters/wasm_store/wasm_store.wasm"
    local wasm_gz_path="$downstream_root/.icp/local/canisters/wasm_store/wasm_store.wasm.gz"
    local did_path="$downstream_root/.icp/local/canisters/wasm_store/wasm_store.did"
    local profile_path="$downstream_root/.icp/local/canisters/wasm_store/.build-profile"

    [ -s "$wasm_path" ] || {
        echo "expected built $scenario wasm_store artifact at $wasm_path" >&2
        exit 1
    }
    [ -s "$wasm_gz_path" ] || {
        echo "expected built $scenario wasm_store gzip artifact at $wasm_gz_path" >&2
        exit 1
    }
    [ -s "$did_path" ] || {
        echo "expected built $scenario wasm_store candid artifact at $did_path" >&2
        exit 1
    }
    grep -qx 'fast' "$profile_path" || {
        echo "expected $scenario wasm_store probe to build the fast profile" >&2
        exit 1
    }
}

assert_generated_probe_outputs() {
    local package_root="$1"
    local downstream_root="$2"
    local wrapper_manifest="$downstream_root/.icp/local/generated/canic-wasm-store/Cargo.toml"

    [ ! -d "$package_root/canic-wasm-store-$VERSION" ] || {
        echo "expected isolated package root to exclude canic-wasm-store so the generated wrapper path is exercised" >&2
        exit 1
    }
    [ -f "$wrapper_manifest" ] || {
        echo "expected generated wasm_store wrapper at $wrapper_manifest" >&2
        exit 1
    }
    assert_wasm_store_artifacts "generated wrapper" "$downstream_root"

    grep -Fq '[patch.crates-io]' "$wrapper_manifest" || {
        echo "expected generated wrapper to patch sibling packaged Canic crates" >&2
        exit 1
    }
    grep -Fq 'features = ["wasm-store-canister"]' "$wrapper_manifest" || {
        echo "expected generated wrapper to use the wasm-store-canister feature" >&2
        exit 1
    }
    if grep -Fq 'features = ["control-plane"]' "$wrapper_manifest"; then
        echo "generated wrapper must not use the root control-plane feature" >&2
        exit 1
    fi
    grep -Fq '[profile.fast]' "$wrapper_manifest" || {
        echo "expected generated wrapper to define the Canic fast profile" >&2
        exit 1
    }
    grep -Fq '[profile.release]' "$wrapper_manifest" || {
        echo "expected generated wrapper to define the Canic release profile" >&2
        exit 1
    }
    grep -Fq "$package_root/canic-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to depend on packaged canic source" >&2
        exit 1
    }
    grep -Fq "$package_root/canic-control-plane-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-control-plane source" >&2
        exit 1
    }
    grep -Fq "$package_root/canic-core-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-core source" >&2
        exit 1
    }
    grep -Fq "$package_root/canic-macros-$VERSION" "$wrapper_manifest" || {
        echo "expected generated wrapper to patch packaged canic-macros source" >&2
        exit 1
    }
    if grep -Fq "$ROOT/crates" "$wrapper_manifest"; then
        echo "generated wasm_store wrapper must not use repository crate paths" >&2
        exit 1
    fi
}

assert_canonical_probe_outputs() {
    local package_root="$1"
    local downstream_root="$2"
    local wrapper_manifest="$downstream_root/.icp/local/generated/canic-wasm-store/Cargo.toml"

    [ -d "$package_root/canic-wasm-store-$VERSION" ] || {
        echo "expected isolated package root to include canonical canic-wasm-store" >&2
        exit 1
    }
    [ ! -f "$wrapper_manifest" ] || {
        echo "expected canonical canic-wasm-store source instead of generated wrapper" >&2
        exit 1
    }
    assert_wasm_store_artifacts "canonical canic-wasm-store" "$downstream_root"
}

main() {
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic
    ensure_packaged_crate canic-wasm-store

    populate_isolated_package_root "$GENERATED_PACKAGE_ROOT" no
    prepare_tool_root "$GENERATED_TOOL_ROOT"
    prepare_downstream_root "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_PACKAGE_ROOT"
    run_probe "$GENERATED_TOOL_ROOT" "$GENERATED_PACKAGE_ROOT" "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_TARGET_DIR"
    assert_generated_probe_outputs "$GENERATED_PACKAGE_ROOT" "$GENERATED_DOWNSTREAM_ROOT"

    populate_isolated_package_root "$CANONICAL_PACKAGE_ROOT" yes
    prepare_tool_root "$CANONICAL_TOOL_ROOT"
    prepare_downstream_root "$CANONICAL_DOWNSTREAM_ROOT" "$CANONICAL_PACKAGE_ROOT"
    run_probe "$CANONICAL_TOOL_ROOT" "$CANONICAL_PACKAGE_ROOT" "$CANONICAL_DOWNSTREAM_ROOT" "$CANONICAL_TARGET_DIR"
    assert_canonical_probe_outputs "$CANONICAL_PACKAGE_ROOT" "$CANONICAL_DOWNSTREAM_ROOT"

    echo "packaged downstream wasm_store probe passed"
}

main "$@"
