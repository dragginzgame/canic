#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-cli.XXXXXX")"
HOST_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
HOST_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TOOL_ROOT/downstream-root"
PROOF_HOME="$TMP_ROOT/home"
PROOF_TARGET_DIR="$TMP_ROOT/cargo-target"
PROOF_TMPDIR="$TMP_ROOT/tmp"
FAKE_ICP="$TMP_ROOT/fake-icp"
FAKE_ICP_STATE="$TMP_ROOT/fake-icp-state"
VERSION="$(
    cargo metadata --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
        jq -r '.packages[] | select(.name == "canic") | .version'
)"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

. "$ROOT/scripts/ci/blob-storage-cli-proof-lib.sh"

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
        canic-cli)
            cargo package -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-backup.path=\"$ROOT/crates/canic-backup\"" \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" \
                --config "patch.crates-io.canic-host.path=\"$ROOT/crates/canic-host\"" >/dev/null
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
        "$PACKAGE_STAGING_ROOT/canic-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-host-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-cli-$VERSION.crate"
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
canic-control-plane = { path = "package-root/canic-control-plane-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
EOF
}

assert_packaged_tool_root() {
    if grep -R -Fq "$ROOT/crates" "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream CLI proof must not use repository crate paths" >&2
        exit 1
    fi

    if grep -R -Fq 'target/debug/canic' "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream CLI proof must not use target/debug/canic" >&2
        exit 1
    fi
}

prepare_downstream_root() {
    mkdir -p \
        "$DOWNSTREAM_ROOT/.icp/local/canisters/app" \
        "$DOWNSTREAM_ROOT/.icp/local/canisters/root" \
        "$DOWNSTREAM_ROOT/fleets/downstream/app" \
        "$DOWNSTREAM_ROOT/fleets/downstream/root"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<'EOF'
[workspace]
members = []
resolver = "2"

[workspace.package]
version = "0.0.0"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/downstream/root/Cargo.toml" <<'EOF'
[package]
name = "downstream-root"
version = { workspace = true }
edition = "2024"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/downstream/app/Cargo.toml" <<'EOF'
[package]
name = "downstream-app"
version = { workspace = true }
edition = "2024"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/downstream/canic.toml" <<'EOF'
controllers = []
app_index = ["app"]

[fleet]
name = "downstream"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
EOF

    printf '\x00asm\x01\x00\x00\x00' | gzip -n > "$DOWNSTREAM_ROOT/.icp/local/canisters/app/app.wasm.gz"
}

run_packaged_canic() {
    (
        cd "$TOOL_ROOT"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            FAKE_ICP_STATE="$FAKE_ICP_STATE" \
            CANIC_WORKSPACE_ROOT="$DOWNSTREAM_ROOT" \
            cargo run --offline -q -p canic-cli --bin canic -- "$@"
    )
}

run_probe() {
    mkdir -p "$PROOF_HOME" "$PROOF_TARGET_DIR" "$PROOF_TMPDIR"
    assert_packaged_tool_root

    run_packaged_canic fleet list > "$TMP_ROOT/fleet-list.out"
    run_packaged_canic fleet role list downstream > "$TMP_ROOT/role-list.out"
    run_packaged_canic fleet role inspect downstream app > "$TMP_ROOT/app-inspect.out"
    run_packaged_canic deploy inspect catalog list --format json --output "$TMP_ROOT/catalog.json"
    run_packaged_canic blob-storage help > "$TMP_ROOT/blob-storage-help.out"
    if run_packaged_canic blob-storage status downstream app --json \
        > "$TMP_ROOT/blob-storage-status-json.out" \
        2> "$TMP_ROOT/blob-storage-status-json.err"
    then
        echo "expected packaged blob-storage JSON status without installed state to fail" >&2
        exit 1
    fi
    prepare_blob_storage_cli_fixture "$DOWNSTREAM_ROOT"
    run_blob_storage_cli_probe_commands run_packaged_canic "$TMP_ROOT" "$FAKE_ICP"
}

assert_probe_outputs() {
    grep -q 'downstream' "$TMP_ROOT/fleet-list.out" || {
        echo "expected packaged canic CLI to list downstream fleet" >&2
        sed -n '1,120p' "$TMP_ROOT/fleet-list.out" >&2
        exit 1
    }
    grep -q '2 (root, app)' "$TMP_ROOT/fleet-list.out" || {
        echo "expected packaged canic CLI to summarize root and app canisters" >&2
        sed -n '1,120p' "$TMP_ROOT/fleet-list.out" >&2
        exit 1
    }
    grep -q 'downstream.root' "$TMP_ROOT/role-list.out" || {
        echo "expected packaged canic CLI to list downstream.root" >&2
        sed -n '1,160p' "$TMP_ROOT/role-list.out" >&2
        exit 1
    }
    grep -q 'downstream.app' "$TMP_ROOT/role-list.out" || {
        echo "expected packaged canic CLI to list downstream.app" >&2
        sed -n '1,160p' "$TMP_ROOT/role-list.out" >&2
        exit 1
    }
    grep -q 'state: attached' "$TMP_ROOT/app-inspect.out" || {
        echo "expected packaged canic CLI to inspect app as attached" >&2
        sed -n '1,160p' "$TMP_ROOT/app-inspect.out" >&2
        exit 1
    }
    grep -q 'deploy artifact: eligible' "$TMP_ROOT/app-inspect.out" || {
        echo "expected packaged canic CLI to inspect app as build-eligible" >&2
        sed -n '1,160p' "$TMP_ROOT/app-inspect.out" >&2
        exit 1
    }
    grep -q '"entries": \[\]' "$TMP_ROOT/catalog.json" || {
        echo "expected packaged canic CLI catalog to stay empty without deployment state" >&2
        sed -n '1,160p' "$TMP_ROOT/catalog.json" >&2
        exit 1
    }
    grep -q 'catalog.no_deployment_state' "$TMP_ROOT/catalog.json" || {
        echo "expected packaged canic CLI catalog to report no deployment state" >&2
        sed -n '1,160p' "$TMP_ROOT/catalog.json" >&2
        exit 1
    }
    assert_blob_storage_cli_probe_outputs "packaged" "$TMP_ROOT"
}

main() {
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic-cli
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    prepare_fake_blob_storage_icp "$FAKE_ICP" "$FAKE_ICP_STATE"
    run_probe
    assert_probe_outputs

    echo "packaged downstream CLI probe passed"
}

main "$@"
